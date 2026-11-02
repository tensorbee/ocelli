import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

import { describe, expect, it } from "vitest";

import { packCommands } from "./commands.js";
import { createPointerDispatch } from "./pointer.js";
import type { GestureBasis, PointerDomEvent, PointerTarget } from "./pointer.js";
import { createToolRegistry } from "./tools.js";
import type { PointerBinding, ToolRegistry } from "./tools.js";
import { viewportId } from "./viewport.js";
import type { StackCommand } from "./viewport.js";

const SOURCE = readFileSync(
  join(dirname(fileURLToPath(import.meta.url)), "pointer.ts"),
  "utf8",
);

const BASIS: GestureBasis = { panX: 10, panY: 20, zoom: 1, radians: 0 };
const PRIMARY: PointerBinding = { button: "primary", modifiers: [] };

function harness(command: StackCommand = "Pan") {
  const tools = createToolRegistry();
  expect(tools.createGroup("g")).toEqual({ ok: true, value: "g" });
  const viewport = viewportId(4);
  expect(tools.attachViewport("g", viewport)).toEqual({ ok: true, value: viewport });
  expect(tools.registerTool("g", { tool: "tool", command })).toEqual({
    ok: true,
    value: { tool: "tool", command, mode: "off", bindings: [] },
  });
  expect(tools.setMode("g", "tool", "active", [PRIMARY])).toMatchObject({ ok: true });
  let time = 0;
  const dispatch = createPointerDispatch({
    registry: tools,
    groupId: "g",
    viewport,
    basis: () => BASIS,
    now: () => time,
  });
  return {
    tools,
    viewport,
    dispatch,
    at(value: number) {
      time = value;
    },
  };
}

function down(partial: Partial<PointerDomEvent> = {}): PointerDomEvent {
  return {
    type: "pointerdown",
    pointerId: 1,
    pointerType: "mouse",
    button: 0,
    clientX: 0,
    clientY: 0,
    ...partial,
  };
}

function move(partial: Partial<PointerDomEvent> = {}): PointerDomEvent {
  return {
    type: "pointermove",
    pointerId: 1,
    pointerType: "mouse",
    clientX: 0,
    clientY: 0,
    ...partial,
  };
}

function up(partial: Partial<PointerDomEvent> = {}): PointerDomEvent {
  return {
    type: "pointerup",
    pointerId: 1,
    pointerType: "mouse",
    button: 0,
    clientX: 0,
    clientY: 0,
    ...partial,
  };
}

describe("pointer dispatch", () => {
  it("one frame packs the last pan", () => {
    const { dispatch, viewport } = harness();
    dispatch.ingest(down({ clientX: 100, clientY: 200 }));
    dispatch.ingest(move({ clientX: 104, clientY: 204 }));
    dispatch.ingest(move({ clientX: 110, clientY: 250 }));
    const frame = dispatch.takeFrame();
    expect(frame.commands).toEqual([{ kind: "Pan", x: 20, y: 70 }]);
    expect(frame.packed).toBe(true);
    expect(frame.reason).toBe("packed");
    expect(frame.bytes).toEqual(packCommands(viewport, frame.commands));
    expect(frame.events.filter((event) => event.name === "pointer.drag")).toHaveLength(2);
    const again = dispatch.takeFrame();
    expect(again.reason).toBe("empty");
    expect(again.commands).toEqual([]);
    expect(again.bytes).toEqual(new Uint8Array());
  });

  it("a missing basis does not pack a pan", () => {
    const tools = armed("Pan");
    const dispatch = createPointerDispatch({
      registry: tools,
      groupId: "g",
      viewport: viewportId(4),
      basis: () => null,
    });
    dispatch.ingest(down({ clientX: 0, clientY: 0 }));
    dispatch.ingest(move({ clientX: 10, clientY: 0 }));
    const frame = dispatch.takeFrame();
    expect(frame.commands).toEqual([]);
    expect(frame.reason).toBe("missing-basis");
    expect(frame.events.some((event) => event.name === "pointer.drag")).toBe(true);
  });

  it("zoom and rotate use the gesture origin", () => {
    const zoom = harness("Zoom");
    zoom.dispatch.ingest(down());
    zoom.dispatch.ingest(move({ clientX: 0, clientY: 256 }));
    const zoomed = zoom.dispatch.takeFrame();
    expect(zoomed.commands).toEqual([{ kind: "Zoom", factor: Math.exp(-1) }]);
    zoom.dispatch.ingest(move({ clientX: 0, clientY: 512 }));
    const zoomedAgain = zoom.dispatch.takeFrame();
    expect(zoomedAgain.commands).toEqual([{ kind: "Zoom", factor: Math.exp(-2) }]);

    const rotate = harness("Rotate");
    rotate.dispatch.ingest(down());
    rotate.dispatch.ingest(move({ clientX: 256, clientY: 0 }));
    const turned = rotate.dispatch.takeFrame();
    expect(turned.commands).toEqual([{ kind: "Rotate", radians: 1 }]);
    rotate.dispatch.ingest(move({ clientX: 512, clientY: 0 }));
    const turnedAgain = rotate.dispatch.takeFrame();
    expect(turnedAgain.commands).toEqual([{ kind: "Rotate", radians: 2 }]);
  });

  it("a non finite zoom is not packed", () => {
    const tools = armed("Zoom");
    const dispatch = createPointerDispatch({
      registry: tools,
      groupId: "g",
      viewport: viewportId(4),
      basis: () => ({ panX: 0, panY: 0, zoom: Number.MAX_VALUE, radians: 0 }),
    });
    dispatch.ingest(down());
    dispatch.ingest(move({ clientY: -1_000_000_000 }));
    const frame = dispatch.takeFrame();
    expect(frame.commands).toEqual([]);
    expect(frame.reason).toBe("non-finite");
    expect(frame.events.length).toBeGreaterThan(0);
  });

  it("a wheel frame scrolls once", () => {
    const { dispatch } = harness("Scroll");
    dispatch.ingest({ type: "wheel", deltaY: 40, deltaMode: 0 });
    dispatch.ingest({ type: "wheel", deltaY: 12, deltaMode: 0 });
    const frame = dispatch.takeFrame();
    expect(frame.commands).toEqual([{ kind: "Scroll", step: 1 }]);
    expect(frame.events.map((event) => event.name)).toEqual(["pointer.wheel", "pointer.wheel"]);
  });

  it("two scroll tools are ambiguous", () => {
    const tools = armed("Scroll", "primary-tool");
    expect(tools.registerTool("g", { tool: "other", command: "Scroll" })).toMatchObject({
      ok: true,
    });
    expect(
      tools.setMode("g", "other", "active", [{ button: "auxiliary", modifiers: [] }]),
    ).toMatchObject({ ok: true });
    const dispatch = createPointerDispatch({
      registry: tools,
      groupId: "g",
      viewport: viewportId(4),
    });
    dispatch.ingest({ type: "wheel", deltaY: 20 });
    const frame = dispatch.takeFrame();
    expect(frame.commands).toEqual([]);
    expect(frame.reason).toBe("ambiguous-scroll");
  });

  it("wheel mode changes the step", () => {
    const { dispatch } = harness("Scroll");
    dispatch.ingest({ type: "wheel", deltaY: 1.9, deltaMode: 1 });
    dispatch.ingest({ type: "wheel", deltaY: 1.9, deltaMode: 1 });
    expect(dispatch.takeFrame().commands).toEqual([{ kind: "Scroll", step: 2 }]);
    dispatch.ingest({ type: "wheel", deltaY: 8, deltaMode: 2 });
    dispatch.ingest({ type: "wheel", deltaY: -3, deltaMode: 2 });
    expect(dispatch.takeFrame().commands).toEqual([]);
    dispatch.ingest({ type: "wheel", deltaY: 3_000_000_000, deltaMode: 1 });
    expect(dispatch.takeFrame().commands).toEqual([{ kind: "Scroll", step: 2147483647 }]);
  });

  it("a scroll drag and a wheel share one step", () => {
    const { dispatch } = harness("Scroll");
    dispatch.ingest(down());
    dispatch.ingest(move({ clientY: 10 }));
    dispatch.ingest({ type: "wheel", deltaY: 2, deltaMode: 1 });
    expect(dispatch.takeFrame().commands).toEqual([{ kind: "Scroll", step: 3 }]);
  });

  it("a window tool does not invent a payload", () => {
    const window = harness("SetWindow");
    window.dispatch.ingest(down());
    window.dispatch.ingest(move({ clientX: 30, clientY: 40 }));
    const framed = window.dispatch.takeFrame();
    expect(framed.commands).toEqual([]);
    expect(framed.reason).toBe("payload-not-in-gesture");

    const fit = harness("Fit");
    fit.dispatch.ingest(down());
    fit.dispatch.ingest(up({ clientX: 1 }));
    expect(fit.dispatch.takeFrame().commands).toEqual([]);
  });

  it("a second contact does not pack", () => {
    const { dispatch } = harness();
    dispatch.ingest(down({ clientX: 0, clientY: 0 }));
    dispatch.ingest(move({ clientX: 20, clientY: 0 }));
    dispatch.ingest(down({ pointerId: 2, clientX: 5, clientY: 5 }));
    dispatch.ingest(move({ pointerId: 2, clientX: 40, clientY: 40 }));
    const frame = dispatch.takeFrame();
    expect(frame.commands).toEqual([{ kind: "Pan", x: 30, y: 20 }]);
    expect(frame.events.some((event) => event.pointerId === 2)).toBe(true);
  });

  it("pointer type selects the event family", () => {
    const mouse = harness();
    mouse.dispatch.ingest(down());
    mouse.dispatch.ingest(move({ clientX: 8 }));
    expect(mouse.dispatch.takeFrame().events.map((event) => event.name)).toEqual([
      "pointer.down",
      "pointer.drag",
    ]);

    const touch = harness();
    touch.dispatch.ingest(down({ pointerType: "touch" }));
    touch.dispatch.ingest(move({ pointerType: "touch", clientX: 8 }));
    expect(touch.dispatch.takeFrame().events.map((event) => event.name)).toEqual([
      "touch.start",
      "touch.drag",
    ]);
  });

  it("three pixels is not a click", () => {
    const { dispatch } = harness();
    dispatch.ingest(down());
    dispatch.ingest(up({ clientX: 3 }));
    const dragged = dispatch.takeFrame();
    expect(dragged.events.map((event) => event.name)).toEqual(["pointer.down", "pointer.up"]);
    expect(dragged.commands).toEqual([{ kind: "Pan", x: 13, y: 20 }]);

    const click = harness();
    click.dispatch.ingest(down());
    click.dispatch.ingest(up({ clientX: 2.9 }));
    const clicked = click.dispatch.takeFrame();
    expect(clicked.events.map((event) => event.name)).toEqual([
      "pointer.down",
      "pointer.up",
      "pointer.click",
    ]);
    expect(clicked.commands).toEqual([]);
    expect(clicked.reason).toBe("no-delta");
  });

  it("double click uses 400 milliseconds", () => {
    const { dispatch, at } = harness();
    dispatch.ingest(down());
    dispatch.ingest(up());
    expect(dispatch.takeFrame().events.map((event) => event.name)).toContain("pointer.click");
    at(400);
    dispatch.ingest(down());
    dispatch.ingest(up());
    const doubled = dispatch.takeFrame();
    expect(doubled.events.map((event) => event.name)).toContain("pointer.doubleClick");
    expect(doubled.events.map((event) => event.name)).not.toContain("pointer.click");

    const late = harness();
    late.dispatch.ingest(down());
    late.dispatch.ingest(up());
    late.dispatch.takeFrame();
    late.at(401);
    late.dispatch.ingest(down());
    late.dispatch.ingest(up());
    const names = late.dispatch.takeFrame().events.map((event) => event.name);
    expect(names).toContain("pointer.click");
    expect(names).not.toContain("pointer.doubleClick");
  });

  it("a swipe is not a tap", () => {
    const { dispatch, at } = harness();
    dispatch.ingest(down({ pointerType: "touch" }));
    at(300);
    dispatch.ingest(up({ pointerType: "touch", clientX: 24 }));
    const names = dispatch.takeFrame().events.map((event) => event.name);
    expect(names).toEqual(["touch.start", "touch.end", "touch.swipe"]);

    const tap = harness();
    tap.dispatch.ingest(down({ pointerType: "touch" }));
    tap.dispatch.ingest(up({ pointerType: "touch", clientX: 1 }));
    expect(tap.dispatch.takeFrame().events.map((event) => event.name)).toEqual([
      "touch.start",
      "touch.end",
      "touch.tap",
    ]);

    const slow = harness();
    slow.at(0);
    slow.dispatch.ingest(down({ pointerType: "touch" }));
    slow.at(301);
    slow.dispatch.ingest(up({ pointerType: "touch", clientX: 24 }));
    expect(slow.dispatch.takeFrame().events.map((event) => event.name)).toEqual([
      "touch.start",
      "touch.end",
    ]);
  });

  it("cancel closes the gesture", () => {
    const { dispatch } = harness();
    dispatch.ingest(down());
    dispatch.takeFrame();
    dispatch.ingest({ type: "pointercancel", pointerId: 1 });
    expect(dispatch.takeFrame().reason).toBe("empty");
    dispatch.ingest(move({ clientX: 12 }));
    const moved = dispatch.takeFrame();
    expect(moved.events.map((event) => event.name)).toEqual(["pointer.move"]);
    expect(moved.commands).toEqual([]);
    dispatch.ingest(up({ clientX: 12 }));
    expect(dispatch.takeFrame().events.map((event) => event.name)).not.toContain("pointer.click");
  });

  it("modifier order still finds the owner", () => {
    const tools = createToolRegistry();
    expect(tools.createGroup("g").ok).toBe(true);
    const viewport = viewportId(4);
    expect(tools.attachViewport("g", viewport).ok).toBe(true);
    expect(tools.registerTool("g", { tool: "tool", command: "Pan" }).ok).toBe(true);
    expect(
      tools.setMode("g", "tool", "active", [
        { button: "primary", modifiers: ["shift", "alt"] },
      ]).ok,
    ).toBe(true);
    const dispatch = createPointerDispatch({
      registry: tools,
      groupId: "g",
      viewport,
      basis: () => BASIS,
    });
    dispatch.ingest(down({ altKey: true, shiftKey: true }));
    dispatch.ingest(move({ clientX: 10, altKey: true, shiftKey: true }));
    expect(dispatch.takeFrame().commands).toEqual([{ kind: "Pan", x: 20, y: 20 }]);
  });

  it("a refused group packs nothing", () => {
    const passive = createToolRegistry();
    expect(passive.createGroup("g").ok).toBe(true);
    const viewport = viewportId(4);
    expect(passive.attachViewport("g", viewport).ok).toBe(true);
    expect(passive.registerTool("g", { tool: "tool", command: "Pan" }).ok).toBe(true);
    const idle = createPointerDispatch({
      registry: passive,
      groupId: "g",
      viewport,
      basis: () => BASIS,
    });
    idle.ingest(down());
    idle.ingest(move({ clientX: 10 }));
    expect(idle.takeFrame().reason).toBe("no-owner");

    const missing = createPointerDispatch({
      registry: passive,
      groupId: "other",
      viewport,
      basis: () => BASIS,
    });
    missing.ingest(down());
    expect(missing.takeFrame().reason).toBe("unknown-group");

    const detached = createToolRegistry();
    expect(detached.createGroup("g").ok).toBe(true);
    expect(detached.registerTool("g", { tool: "tool", command: "Pan" }).ok).toBe(true);
    expect(detached.setMode("g", "tool", "active", [PRIMARY]).ok).toBe(true);
    const unattached = createPointerDispatch({
      registry: detached,
      groupId: "g",
      viewport,
      basis: () => BASIS,
    });
    unattached.ingest(down());
    unattached.ingest(move({ clientX: 10 }));
    expect(unattached.takeFrame()).toMatchObject({
      reason: "missing-viewport",
      commands: [],
    });
  });

  it("an extra button is not a binding", () => {
    const { dispatch } = harness();
    dispatch.ingest(down({ button: 3 }));
    dispatch.ingest(move({ clientX: 20 }));
    const frame = dispatch.takeFrame();
    expect(frame.commands).toEqual([]);
    expect(frame.reason).toBe("no-gesture");
    expect(frame.events.map((event) => event.name)).toEqual(["pointer.move"]);
  });

  it("a non finite point is dropped", () => {
    const { dispatch } = harness();
    dispatch.ingest(down({ clientX: Number.NaN }));
    dispatch.ingest({ type: "wheel", deltaY: Number.NaN });
    expect(dispatch.takeFrame().reason).toBe("empty");
  });

  it("the dispatcher does not cross the boundary", () => {
    expect(SOURCE.includes("submitCommands")).toBe(false);
    expect(SOURCE.includes("writeCommands")).toBe(false);
    expect(SOURCE.includes("packPointerFrame")).toBe(false);
  });

  it("detach removes the listener", () => {
    const listeners = new Map<string, ((event: PointerDomEvent) => void)[]>();
    let captured = 0;
    const target: PointerTarget = {
      addEventListener(type, listener) {
        const found = listeners.get(type) ?? [];
        found.push(listener);
        listeners.set(type, found);
      },
      removeEventListener(type, listener) {
        const found = listeners.get(type) ?? [];
        listeners.set(
          type,
          found.filter((item) => item !== listener),
        );
      },
      setPointerCapture() {
        throw new Error("capture refused");
      },
      releasePointerCapture() {
        captured += 1;
      },
    };
    const { dispatch } = harness();
    const attached = createPointerDispatch({
      registry: armed("Pan"),
      groupId: "g",
      viewport: viewportId(4),
      target,
      basis: () => BASIS,
    });
    attached.attach();
    attached.attach();
    expect(listeners.get("pointerdown")).toHaveLength(1);
    const listener = listeners.get("pointerdown")?.[0];
    expect(listener).toBeTypeOf("function");
    listener?.(down());
    listener?.(move({ clientX: 12 }));
    expect(attached.takeFrame().commands).toEqual([{ kind: "Pan", x: 22, y: 20 }]);
    attached.detach();
    expect(listeners.get("pointerdown")).toEqual([]);
    dispatch.ingest(down());
    expect(dispatch.takeFrame().events).toHaveLength(1);
    void captured;
  });

  it("a second press in the same frame starts a gesture", () => {
    const { dispatch, at } = harness();
    dispatch.ingest(down({ clientX: 0, clientY: 0 }));
    dispatch.ingest(up({ clientX: 1, clientY: 0 }));
    at(100);
    dispatch.ingest(down({ clientX: 1, clientY: 0 }));
    dispatch.ingest(move({ clientX: 40, clientY: 0 }));
    const frame = dispatch.takeFrame();
    expect(frame.events.filter((event) => event.name === "pointer.down")).toHaveLength(2);
    expect(frame.commands).toEqual([{ kind: "Pan", x: 49, y: 20 }]);
    dispatch.ingest(move({ clientX: 80, clientY: 0 }));
    expect(dispatch.takeFrame().commands).toEqual([{ kind: "Pan", x: 89, y: 20 }]);
  });

  it("two clicks in one frame can double click", () => {
    const { dispatch, at } = harness();
    dispatch.ingest(down());
    dispatch.ingest(up());
    at(100);
    dispatch.ingest(down());
    dispatch.ingest(up());
    const names = dispatch.takeFrame().events.map((event) => event.name);
    expect(names).toContain("pointer.click");
    expect(names).toContain("pointer.doubleClick");
  });

  it("capture keeps the target as this", () => {
    const calls: Array<{ name: string; self: PointerTarget; id: number }> = [];
    const target: PointerTarget = {
      addEventListener() {
        return undefined;
      },
      removeEventListener() {
        return undefined;
      },
      setPointerCapture(pointerId: number) {
        calls.push({ name: "set", self: this, id: pointerId });
      },
      releasePointerCapture(pointerId: number) {
        calls.push({ name: "release", self: this, id: pointerId });
      },
    };
    const dispatch = createPointerDispatch({
      registry: armed("Pan"),
      groupId: "g",
      viewport: viewportId(4),
      target,
      basis: () => BASIS,
    });
    dispatch.ingest(down({ pointerId: 7 }));
    dispatch.ingest(up({ pointerId: 7 }));
    expect(calls).toEqual([
      { name: "set", self: target, id: 7 },
      { name: "release", self: target, id: 7 },
    ]);
  });

  it("a return inside the click distance still replaces pan", () => {
    const { dispatch } = harness();
    dispatch.ingest(down());
    dispatch.ingest(move({ clientX: 100 }));
    expect(dispatch.takeFrame().commands).toEqual([{ kind: "Pan", x: 110, y: 20 }]);
    dispatch.ingest(move({ clientX: 2 }));
    expect(dispatch.takeFrame().commands).toEqual([{ kind: "Pan", x: 12, y: 20 }]);
    dispatch.ingest(up({ clientX: 2 }));
    const ended = dispatch.takeFrame();
    expect(ended.events.map((event) => event.name)).toContain("pointer.click");
    expect(ended.commands).toEqual([{ kind: "Pan", x: 12, y: 20 }]);
  });

  it("a scroll drag keeps the return step", () => {
    const { dispatch } = harness("Scroll");
    dispatch.ingest(down());
    dispatch.ingest(move({ clientY: 10 }));
    expect(dispatch.takeFrame().commands).toEqual([{ kind: "Scroll", step: 1 }]);
    dispatch.ingest(move({ clientY: 2 }));
    expect(dispatch.takeFrame().commands).toEqual([{ kind: "Scroll", step: -1 }]);
  });

  it("an ambiguous wheel does not drop a scroll drag", () => {
    const tools = armed("Scroll", "primary-tool");
    expect(tools.registerTool("g", { tool: "other", command: "Scroll" })).toMatchObject({
      ok: true,
    });
    expect(
      tools.setMode("g", "other", "active", [{ button: "auxiliary", modifiers: [] }]),
    ).toMatchObject({ ok: true });
    const dispatch = createPointerDispatch({
      registry: tools,
      groupId: "g",
      viewport: viewportId(4),
    });
    dispatch.ingest(down());
    dispatch.ingest(move({ clientY: 10 }));
    dispatch.ingest({ type: "wheel", deltaY: 4, deltaMode: 1 });
    const frame = dispatch.takeFrame();
    expect(frame.commands).toEqual([{ kind: "Scroll", step: 1 }]);
    expect(frame.reason).toBe("packed");
    expect(frame.tool).toBe("primary-tool");
  });

  it("a hover is not a delta", () => {
    const { dispatch } = harness();
    dispatch.ingest(move({ clientX: 20 }));
    const frame = dispatch.takeFrame();
    expect(frame.reason).toBe("no-gesture");
    expect(frame.commands).toEqual([]);
  });

  it("a stray up is not an extra contact", () => {
    const { dispatch } = harness();
    dispatch.ingest(up({ clientX: 4 }));
    const stray = dispatch.takeFrame();
    expect(stray.reason).toBe("no-gesture");
    expect(stray.commands).toEqual([]);
    expect(stray.events.map((event) => event.name)).toEqual(["pointer.up"]);

    const touch = harness();
    touch.dispatch.ingest(up({ pointerType: "touch", clientX: 4 }));
    expect(touch.dispatch.takeFrame().reason).toBe("no-gesture");

    dispatch.ingest(move({ clientX: 8 }));
    dispatch.ingest(up({ clientX: 8 }));
    expect(dispatch.takeFrame().reason).toBe("no-gesture");

    const { dispatch: held } = harness();
    held.ingest(down());
    held.takeFrame();
    held.ingest(up({ pointerId: 2, clientX: 4 }));
    expect(held.takeFrame().reason).toBe("ignored-contact");
    held.ingest(move({ clientX: 20 }));
    expect(held.takeFrame().commands).toEqual([{ kind: "Pan", x: 30, y: 20 }]);
  });

  it("wheel contributions sum before the clamp", () => {
    const { dispatch } = harness("Scroll");
    dispatch.ingest({ type: "wheel", deltaY: 3_000_000_000, deltaMode: 1 });
    dispatch.ingest({ type: "wheel", deltaY: -3_000_000_000, deltaMode: 1 });
    expect(dispatch.takeFrame().commands).toEqual([]);

    dispatch.ingest({ type: "wheel", deltaY: -1_000_000_000, deltaMode: 1 });
    dispatch.ingest({ type: "wheel", deltaY: 3_000_000_000, deltaMode: 1 });
    expect(dispatch.takeFrame().commands).toEqual([{ kind: "Scroll", step: 2_000_000_000 }]);

    dispatch.ingest({ type: "wheel", deltaY: 3_000_000_000, deltaMode: 1 });
    dispatch.ingest({ type: "wheel", deltaY: -1_000_000_000, deltaMode: 1 });
    expect(dispatch.takeFrame().commands).toEqual([{ kind: "Scroll", step: 2_000_000_000 }]);

    dispatch.ingest({ type: "wheel", deltaY: 3_000_000_000, deltaMode: 1 });
    dispatch.ingest({ type: "wheel", deltaY: -100, deltaMode: 0 });
    expect(dispatch.takeFrame().commands).toEqual([{ kind: "Scroll", step: 2147483647 }]);

    dispatch.ingest({ type: "wheel", deltaY: -3_000_000_000, deltaMode: 1 });
    expect(dispatch.takeFrame().commands).toEqual([{ kind: "Scroll", step: -2147483648 }]);

    dispatch.ingest(down());
    dispatch.ingest(move({ clientY: -10 }));
    dispatch.ingest({ type: "wheel", deltaY: 3_000_000_000, deltaMode: 1 });
    expect(dispatch.takeFrame().commands).toEqual([{ kind: "Scroll", step: 2147483647 }]);
  });

  it("cancel in the same frame is not a delta", () => {
    const { dispatch } = harness();
    dispatch.ingest(down());
    dispatch.ingest(move({ clientX: 30 }));
    dispatch.ingest({ type: "pointercancel", pointerId: 1 });
    const frame = dispatch.takeFrame();
    expect(frame.events.map((event) => event.name)).toContain("pointer.drag");
    expect(frame.events.map((event) => event.name)).not.toContain("pointer.click");
    expect(frame.commands).toEqual([]);
    expect(frame.reason).toBe("no-gesture");
  });
});

function armed(command: StackCommand, name = "tool"): ToolRegistry {
  const tools = createToolRegistry();
  expect(tools.createGroup("g").ok).toBe(true);
  expect(tools.attachViewport("g", viewportId(4)).ok).toBe(true);
  expect(tools.registerTool("g", { tool: name, command }).ok).toBe(true);
  expect(tools.setMode("g", name, "active", [PRIMARY]).ok).toBe(true);
  return tools;
}
