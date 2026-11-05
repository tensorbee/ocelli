/**
 * Pointer, touch and wheel normalisation.
 *
 * HLD section 5.1 batches a pointer stream into one command buffer per
 * frame. This module returns that buffer. It does not submit it.
 *
 * The payload rules are the local decisions in
 * `.claude/plans/F-069-design.md`. `set_pan`, `set_zoom` and
 * `set_rotation` replace, so a client position is not a pan.
 */

import { packCommands } from "./commands.js";
import type { PackedCommand } from "./commands.js";
import type { StackCommand } from "./generated.js";
import { POINTER_DISPATCH_OPTION_KEYS, refuseUnknownOptions } from "./options.js";
import type {
  PointerBinding,
  PointerButton,
  PointerModifier,
  ToolRegistry,
} from "./tools.js";
import type { ViewportId } from "./viewport.js";

/** Local CSS-pixel distance that separates a click from a drag. */
const CLICK_DISTANCE = 3;

/** Local CSS-pixel distance that qualifies a touch swipe. */
const SWIPE_DISTANCE = 24;

/** Local double-click window, in milliseconds. */
const DOUBLE_CLICK_MS = 400;

/** Local swipe window, in milliseconds. */
const SWIPE_MS = 300;

/** Local scale for zoom and rotation. Not a DICOM number. */
const GESTURE_SCALE = 256;

const I32_MIN = -2147483648;
const I32_MAX = 2147483647;

const CAMERA_COMMANDS: readonly StackCommand[] = ["Pan", "Zoom", "Rotate"];

/** Why a frame did or did not pack a command. */
export const DISPATCH_REASONS = [
  "packed",
  "empty",
  "no-delta",
  "no-gesture",
  "no-owner",
  "payload-not-in-gesture",
  "missing-basis",
  "non-finite",
  "ambiguous-scroll",
  "ignored-contact",
  "unknown-group",
  "missing-viewport",
] as const;

/** One reason from {@link DISPATCH_REASONS}. */
export type DispatchReason = (typeof DISPATCH_REASONS)[number];

/** Caller-supplied camera inputs captured at pointer down. */
export interface GestureBasis {
  readonly panX: number;
  readonly panY: number;
  readonly zoom: number;
  readonly radians: number;
}

/**
 * The DOM fields this dispatcher reads.
 *
 * A test can pass a plain object. The values are not a browser event.
 */
export interface PointerDomEvent {
  readonly type: string;
  readonly pointerId?: number;
  readonly pointerType?: string;
  readonly button?: number;
  readonly clientX?: number;
  readonly clientY?: number;
  readonly altKey?: boolean;
  readonly ctrlKey?: boolean;
  readonly metaKey?: boolean;
  readonly shiftKey?: boolean;
  readonly deltaX?: number;
  readonly deltaY?: number;
  readonly deltaMode?: number;
}

/** A target that can subscribe. Capture methods are optional. */
export interface PointerTarget {
  addEventListener(type: string, listener: (event: PointerDomEvent) => void): void;
  removeEventListener(type: string, listener: (event: PointerDomEvent) => void): void;
  setPointerCapture?(pointerId: number): void;
  releasePointerCapture?(pointerId: number): void;
}

/** One normalised shell event. Names match the F-103 catalogue. */
export interface ShellGestureEvent {
  readonly name:
    | "pointer.down"
    | "pointer.up"
    | "pointer.move"
    | "pointer.drag"
    | "pointer.wheel"
    | "pointer.click"
    | "pointer.doubleClick"
    | "touch.start"
    | "touch.drag"
    | "touch.end"
    | "touch.tap"
    | "touch.swipe";
  readonly viewport: number;
  readonly x: number;
  readonly y: number;
  readonly button: PointerButton | null;
  readonly modifiers: readonly PointerModifier[];
  readonly pointerType: "mouse" | "pen" | "touch";
  readonly pointerId: number;
  readonly deltaX: number;
  readonly deltaY: number;
}

/** One animation frame. `bytes` is empty when `commands` is empty. */
export interface DispatchFrame {
  readonly viewport: number;
  readonly events: readonly ShellGestureEvent[];
  readonly commands: readonly PackedCommand[];
  readonly bytes: Uint8Array;
  readonly tool: string | null;
  readonly command: StackCommand | null;
  readonly packed: boolean;
  readonly reason: DispatchReason;
}

/** Options for {@link createPointerDispatch}. */
export interface PointerDispatchOptions {
  readonly registry: ToolRegistry;
  readonly groupId: string;
  readonly viewport: ViewportId;
  readonly target?: PointerTarget;
  readonly basis?: () => GestureBasis | null;
  readonly now?: () => number;
}

/** The caller-owned dispatcher. A second call does not share its queue. */
export interface PointerDispatch {
  ingest(event: PointerDomEvent): void;
  takeFrame(): DispatchFrame;
  attach(): void;
  detach(): void;
}

interface Gesture {
  pointerId: number;
  pointerType: "mouse" | "pen" | "touch";
  button: PointerButton;
  originX: number;
  originY: number;
  lastX: number;
  lastY: number;
  segmentY: number;
  downTime: number;
  tool: string | null;
  command: StackCommand | null;
  basis: GestureBasis | null;
  blocked: DispatchReason | null;
  doubleCandidate: boolean;
  /** True after any sample is at least {@link CLICK_DISTANCE} from the origin. */
  engaged: boolean;
}

interface ClickMark {
  x: number;
  y: number;
  time: number;
}

interface WheelSample {
  mode: number;
  deltaY: number;
}

const LISTENER_TYPES = [
  "pointerdown",
  "pointermove",
  "pointerup",
  "pointercancel",
  "wheel",
] as const;

/**
 * Create a dispatcher for one viewport in one tool group.
 *
 * `takeFrame` packs at most one buffer. It does not call wasm.
 */
export function createPointerDispatch(options: PointerDispatchOptions): PointerDispatch {
  refuseUnknownOptions("PointerDispatchOptions", options, POINTER_DISPATCH_OPTION_KEYS);
  const viewport = options.viewport;
  const now = options.now ?? Date.now;
  const basisOf = options.basis ?? (() => null);
  const pending: ShellGestureEvent[] = [];
  const wheels: WheelSample[] = [];
  let gesture: Gesture | null = null;
  const closed: Gesture[] = [];
  let lastClick: ClickMark | null = null;
  let sawExtra = false;
  let attached = false;

  function onDom(event: PointerDomEvent): void {
    ingest(event);
  }

  function ingest(event: PointerDomEvent): void {
    if (event.type === "wheel") {
      ingestWheel(event);
      return;
    }
    if (event.type === "pointerdown") {
      ingestDown(event);
      return;
    }
    if (event.type === "pointermove") {
      ingestMove(event);
      return;
    }
    if (event.type === "pointerup") {
      ingestUp(event);
      return;
    }
    if (event.type === "pointercancel") {
      ingestCancel(event);
    }
  }

  function ingestWheel(event: PointerDomEvent): void {
    const deltaY = event.deltaY ?? 0;
    const deltaX = event.deltaX ?? 0;
    if (!Number.isFinite(deltaY) || !Number.isFinite(deltaX)) {
      return;
    }
    const point = pointOf(event);
    if (point === null) {
      return;
    }
    wheels.push({ mode: event.deltaMode ?? 0, deltaY });
    pending.push(
      shell("pointer.wheel", point.x, point.y, null, modifiersOf(event), "mouse", event.pointerId ?? 1, deltaX, deltaY),
    );
  }

  function ingestDown(event: PointerDomEvent): void {
    const point = pointOf(event);
    const button = buttonOf(event.button);
    if (point === null || button === null) {
      return;
    }
    const pointerType = pointerTypeOf(event);
    const pointerId = event.pointerId ?? 1;
    if (gesture !== null) {
      sawExtra = true;
      pending.push(
        shell(
          pointerType === "touch" ? "touch.start" : "pointer.down",
          point.x,
          point.y,
          button,
          modifiersOf(event),
          pointerType,
          pointerId,
          0,
          0,
        ),
      );
      return;
    }
    const captured = captureTool(button, modifiersOf(event));
    const time = now();
    const doubleCandidate =
      pointerType !== "touch" &&
      lastClick !== null &&
      time - lastClick.time <= DOUBLE_CLICK_MS &&
      distance(point.x, point.y, lastClick.x, lastClick.y) < CLICK_DISTANCE;
    gesture = {
      pointerId,
      pointerType,
      button,
      originX: point.x,
      originY: point.y,
      lastX: point.x,
      lastY: point.y,
      segmentY: point.y,
      downTime: time,
      tool: captured.tool,
      command: captured.command,
      basis: captured.basis,
      blocked: captured.blocked,
      doubleCandidate,
      engaged: false,
    };
    pending.push(
      shell(
        pointerType === "touch" ? "touch.start" : "pointer.down",
        point.x,
        point.y,
        button,
        modifiersOf(event),
        pointerType,
        pointerId,
        0,
        0,
      ),
    );
    holdCapture(pointerId);
  }

  function ingestMove(event: PointerDomEvent): void {
    const point = pointOf(event);
    if (point === null) {
      return;
    }
    const pointerId = event.pointerId ?? 1;
    const pointerType = pointerTypeOf(event);
    if (gesture === null || gesture.pointerId !== pointerId) {
      if (gesture !== null) {
        sawExtra = true;
        pending.push(
          shell(
            pointerType === "touch" ? "touch.drag" : "pointer.move",
            point.x,
            point.y,
            null,
            modifiersOf(event),
            pointerType,
            pointerId,
            0,
            0,
          ),
        );
      } else if (pointerType !== "touch") {
        pending.push(
          shell(
            "pointer.move",
            point.x,
            point.y,
            null,
            modifiersOf(event),
            pointerType,
            pointerId,
            0,
            0,
          ),
        );
      }
      return;
    }
    const deltaX = point.x - gesture.lastX;
    const deltaY = point.y - gesture.lastY;
    gesture.lastX = point.x;
    gesture.lastY = point.y;
    noteTravel(gesture);
    const name =
      gesture.pointerType === "touch"
        ? "touch.drag"
        : gesture.engaged
          ? "pointer.drag"
          : "pointer.move";
    pending.push(
      shell(name, point.x, point.y, gesture.button, modifiersOf(event), gesture.pointerType, pointerId, deltaX, deltaY),
    );
  }

  function ingestUp(event: PointerDomEvent): void {
    const point = pointOf(event);
    if (point === null) {
      return;
    }
    const pointerId = event.pointerId ?? 1;
    const pointerType = pointerTypeOf(event);
    if (gesture === null || gesture.pointerId !== pointerId) {
      if (gesture !== null) {
        sawExtra = true;
      }
      pending.push(
        shell(
          pointerType === "touch" ? "touch.end" : "pointer.up",
          point.x,
          point.y,
          null,
          modifiersOf(event),
          pointerType,
          pointerId,
          0,
          0,
        ),
      );
      return;
    }
    const deltaX = point.x - gesture.lastX;
    const deltaY = point.y - gesture.lastY;
    gesture.lastX = point.x;
    gesture.lastY = point.y;
    noteTravel(gesture);
    const moved = distance(point.x, point.y, gesture.originX, gesture.originY);
    const elapsed = now() - gesture.downTime;
    pending.push(
      shell(
        gesture.pointerType === "touch" ? "touch.end" : "pointer.up",
        point.x,
        point.y,
        gesture.button,
        modifiersOf(event),
        gesture.pointerType,
        pointerId,
        deltaX,
        deltaY,
      ),
    );
    if (gesture.pointerType === "touch") {
      if (moved >= SWIPE_DISTANCE && elapsed <= SWIPE_MS) {
        pending.push(
          shell("touch.swipe", point.x, point.y, gesture.button, modifiersOf(event), "touch", pointerId, deltaX, deltaY),
        );
      } else if (moved < CLICK_DISTANCE) {
        pending.push(
          shell("touch.tap", point.x, point.y, gesture.button, modifiersOf(event), "touch", pointerId, deltaX, deltaY),
        );
      }
    } else if (moved < CLICK_DISTANCE) {
      const name = gesture.doubleCandidate ? "pointer.doubleClick" : "pointer.click";
      pending.push(
        shell(name, point.x, point.y, gesture.button, modifiersOf(event), gesture.pointerType, pointerId, deltaX, deltaY),
      );
      if (gesture.doubleCandidate) {
        lastClick = null;
      } else {
        lastClick = { x: point.x, y: point.y, time: now() };
      }
    }
    releaseCapture(pointerId);
    closed.push(gesture);
    gesture = null;
  }

  function ingestCancel(event: PointerDomEvent): void {
    const pointerId = event.pointerId ?? 1;
    if (gesture === null || gesture.pointerId !== pointerId) {
      return;
    }
    releaseCapture(pointerId);
    gesture = null;
  }

  function captureTool(
    button: PointerButton,
    modifiers: readonly PointerModifier[],
  ): {
    tool: string | null;
    command: StackCommand | null;
    basis: GestureBasis | null;
    blocked: DispatchReason | null;
  } {
    const group = options.registry.group(options.groupId);
    if (!group.ok) {
      return { tool: null, command: null, basis: null, blocked: "unknown-group" };
    }
    if (!group.value.viewports.includes(viewport)) {
      return { tool: null, command: null, basis: null, blocked: "missing-viewport" };
    }
    const binding: PointerBinding = { button, modifiers };
    const owner = options.registry.bindingOwner(options.groupId, binding);
    if (!owner.ok) {
      return { tool: null, command: null, basis: null, blocked: "unknown-group" };
    }
    if (owner.value === null) {
      return { tool: null, command: null, basis: null, blocked: "no-owner" };
    }
    const command = owner.value.command;
    if (command === "Scroll") {
      return { tool: owner.value.tool, command, basis: null, blocked: null };
    }
    if (!CAMERA_COMMANDS.includes(command)) {
      return {
        tool: owner.value.tool,
        command,
        basis: null,
        blocked: "payload-not-in-gesture",
      };
    }
    const basis = basisOf();
    if (basis === null || !finiteBasis(basis)) {
      return { tool: owner.value.tool, command, basis: null, blocked: "missing-basis" };
    }
    return { tool: owner.value.tool, command, basis, blocked: null };
  }

  function takeFrame(): DispatchFrame {
    const events = pending.splice(0, pending.length);
    const samples = wheels.splice(0, wheels.length);
    const extra = sawExtra;
    sawExtra = false;
    const finished = closed.splice(0, closed.length);
    if (events.length === 0) {
      return blank("empty");
    }
    const group = options.registry.group(options.groupId);
    if (!group.ok) {
      return finish(events, [], null, null, "unknown-group");
    }
    if (!group.value.viewports.includes(viewport)) {
      return finish(events, [], null, null, "missing-viewport");
    }

    const sources = finished;
    if (gesture !== null) {
      sources.push(gesture);
    }
    let commands: PackedCommand[] = [];
    let reason: DispatchReason = "no-gesture";
    let tool: string | null = null;
    let command: StackCommand | null = null;
    let dragStep = 0;
    let sawGesture = false;

    for (const source of sources) {
      const matches = events.some(
        (item) => item.pointerId === source.pointerId && item.name !== "pointer.wheel",
      );
      if (!matches) {
        continue;
      }
      sawGesture = true;
      tool = source.tool;
      command = source.command;
      if (source.blocked !== null) {
        if (commands.length === 0 && dragStep === 0) {
          reason = source.blocked;
        }
      } else {
        const produced = produce(source);
        dragStep = addI32(dragStep, produced.dragStep);
        for (const item of produced.commands) {
          if (item.kind === "Scroll") {
            continue;
          }
          commands = commands.filter((current) => current.kind !== item.kind);
          commands.push(item);
        }
        const contributed = produced.dragStep !== 0 || commands.length > 0;
        if (contributed && produced.reason === "packed") {
          reason = "packed";
        } else if (commands.length === 0 && dragStep === 0) {
          reason = produced.reason;
        }
      }
      source.segmentY = source.lastY;
    }

    const wheelStep = wheelStepOf(samples);
    const sawWheel = samples.length > 0;
    const scrollTools = group.value.tools.filter(
      (item) => item.mode === "active" && item.command === "Scroll",
    );
    let scrollStep = dragStep;
    if (sawWheel) {
      if (scrollTools.length > 1) {
        if (scrollStep === 0 && commands.length === 0) {
          reason = "ambiguous-scroll";
          if (command === "Scroll") {
            tool = null;
            command = null;
          }
        }
      } else if (scrollTools.length === 1) {
        if (command === "Scroll") {
          const step = addI32(dragStep, wheelStep);
          scrollStep = step;
          if (step === 0 && commands.length === 0) {
            reason = "no-delta";
          }
        } else if (wheelStep !== 0) {
          scrollStep = addI32(dragStep, wheelStep);
          if (tool === null) {
            const owner = scrollTools[0];
            tool = owner === undefined ? null : owner.tool;
            command = "Scroll";
          }
        } else if (!sawGesture) {
          reason = "no-delta";
          scrollStep = 0;
        }
      } else if (!sawGesture && commands.length === 0) {
        reason = "no-owner";
        scrollStep = 0;
      }
    }

    if (scrollStep !== 0) {
      commands.push({ kind: "Scroll", step: scrollStep });
    }
    if (!sawGesture && !sawWheel && extra) {
      reason = "ignored-contact";
    }
    if (commands.length > 0) {
      reason = "packed";
    }
    return finish(events, commands, tool, command, reason);
  }

  function produce(source: Gesture): {
    commands: PackedCommand[];
    reason: DispatchReason;
    dragStep: number;
  } {
    const travelled =
      source.engaged ||
      distance(source.lastX, source.lastY, source.originX, source.originY) >= CLICK_DISTANCE;
    if (source.command === "Scroll") {
      if (!travelled) {
        return { commands: [], reason: "no-delta", dragStep: 0 };
      }
      const dragStep = sign(source.lastY - source.segmentY);
      if (dragStep === 0) {
        return { commands: [], reason: "no-delta", dragStep: 0 };
      }
      return {
        commands: [{ kind: "Scroll", step: dragStep }],
        reason: "packed",
        dragStep,
      };
    }
    if (source.command === "Pan" && source.basis !== null) {
      if (!travelled) {
        return { commands: [], reason: "no-delta", dragStep: 0 };
      }
      return {
        commands: [
          {
            kind: "Pan",
            x: source.basis.panX + (source.lastX - source.originX),
            y: source.basis.panY + (source.lastY - source.originY),
          },
        ],
        reason: "packed",
        dragStep: 0,
      };
    }
    if (source.command === "Zoom" && source.basis !== null) {
      if (!travelled) {
        return { commands: [], reason: "no-delta", dragStep: 0 };
      }
      const factor =
        source.basis.zoom * Math.exp(-(source.lastY - source.originY) / GESTURE_SCALE);
      if (!Number.isFinite(factor)) {
        return { commands: [], reason: "non-finite", dragStep: 0 };
      }
      return { commands: [{ kind: "Zoom", factor }], reason: "packed", dragStep: 0 };
    }
    if (source.command === "Rotate" && source.basis !== null) {
      if (!travelled) {
        return { commands: [], reason: "no-delta", dragStep: 0 };
      }
      const radians = source.basis.radians + (source.lastX - source.originX) / GESTURE_SCALE;
      if (!Number.isFinite(radians)) {
        return { commands: [], reason: "non-finite", dragStep: 0 };
      }
      return { commands: [{ kind: "Rotate", radians }], reason: "packed", dragStep: 0 };
    }
    return { commands: [], reason: "no-delta", dragStep: 0 };
  }

  function finish(
    events: readonly ShellGestureEvent[],
    commands: readonly PackedCommand[],
    tool: string | null,
    command: StackCommand | null,
    reason: DispatchReason,
  ): DispatchFrame {
    const bytes =
      commands.length === 0 ? new Uint8Array() : packCommands(viewport, commands);
    return {
      viewport,
      events,
      commands,
      bytes,
      tool,
      command,
      packed: commands.length > 0,
      reason,
    };
  }

  function blank(reason: DispatchReason): DispatchFrame {
    return finish([], [], null, null, reason);
  }

  function shell(
    name: ShellGestureEvent["name"],
    x: number,
    y: number,
    button: PointerButton | null,
    modifiers: readonly PointerModifier[],
    pointerType: "mouse" | "pen" | "touch",
    pointerId: number,
    deltaX: number,
    deltaY: number,
  ): ShellGestureEvent {
    return { name, viewport, x, y, button, modifiers, pointerType, pointerId, deltaX, deltaY };
  }

  function holdCapture(pointerId: number): void {
    const target = options.target;
    if (target === undefined || target.setPointerCapture === undefined) {
      return;
    }
    try {
      target.setPointerCapture(pointerId);
    } catch (error) {
      void error;
    }
  }

  function releaseCapture(pointerId: number): void {
    const target = options.target;
    if (target === undefined || target.releasePointerCapture === undefined) {
      return;
    }
    try {
      target.releasePointerCapture(pointerId);
    } catch (error) {
      void error;
    }
  }

  return {
    ingest,
    takeFrame,
    attach() {
      if (attached || options.target === undefined) {
        return;
      }
      attached = true;
      for (const type of LISTENER_TYPES) {
        options.target.addEventListener(type, onDom);
      }
    },
    detach() {
      if (!attached || options.target === undefined) {
        return;
      }
      attached = false;
      for (const type of LISTENER_TYPES) {
        options.target.removeEventListener(type, onDom);
      }
    },
  };
}

function wheelStepOf(samples: readonly WheelSample[]): number {
  let pixel = 0;
  let line = 0;
  let page = 0;
  for (const sample of samples) {
    if (sample.mode === 1) {
      line += Math.trunc(sample.deltaY);
    } else if (sample.mode === 2) {
      page += sign(sample.deltaY);
    } else {
      pixel += sample.deltaY;
    }
  }
  return sign(pixel) + line + page;
}

function finiteBasis(basis: GestureBasis): boolean {
  return (
    Number.isFinite(basis.panX) &&
    Number.isFinite(basis.panY) &&
    Number.isFinite(basis.zoom) &&
    Number.isFinite(basis.radians)
  );
}

function pointOf(event: PointerDomEvent): { x: number; y: number } | null {
  const x = event.clientX ?? 0;
  const y = event.clientY ?? 0;
  if (!Number.isFinite(x) || !Number.isFinite(y)) {
    return null;
  }
  return { x, y };
}

function buttonOf(button: number | undefined): PointerButton | null {
  if (button === 0) {
    return "primary";
  }
  if (button === 1) {
    return "auxiliary";
  }
  if (button === 2) {
    return "secondary";
  }
  return null;
}

function pointerTypeOf(event: PointerDomEvent): "mouse" | "pen" | "touch" {
  if (event.pointerType === "touch") {
    return "touch";
  }
  if (event.pointerType === "pen") {
    return "pen";
  }
  return "mouse";
}

function modifiersOf(event: PointerDomEvent): PointerModifier[] {
  const modifiers: PointerModifier[] = [];
  if (event.altKey === true) {
    modifiers.push("alt");
  }
  if (event.ctrlKey === true) {
    modifiers.push("ctrl");
  }
  if (event.metaKey === true) {
    modifiers.push("meta");
  }
  if (event.shiftKey === true) {
    modifiers.push("shift");
  }
  return modifiers;
}

function noteTravel(active: Gesture): void {
  if (distance(active.lastX, active.lastY, active.originX, active.originY) >= CLICK_DISTANCE) {
    active.engaged = true;
  }
}

function distance(x: number, y: number, originX: number, originY: number): number {
  return Math.hypot(x - originX, y - originY);
}

function sign(value: number): number {
  if (value > 0) {
    return 1;
  }
  if (value < 0) {
    return -1;
  }
  return 0;
}

function addI32(left: number, right: number): number {
  const sum = left + right;
  if (sum > I32_MAX) {
    return I32_MAX;
  }
  if (sum < I32_MIN) {
    return I32_MIN;
  }
  return sum;
}
