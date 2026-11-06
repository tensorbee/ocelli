/**
 * Cursor and tool-state events.
 *
 * HLD section 10 keeps the tool state machine in the shell. F-068 stores
 * the modes. This module emits the F-103 shell events after a successful
 * change and writes one CSS cursor. It does not listen.
 *
 * The cursor strings and the emission order are the local decisions in
 * `.claude/plans/F-072-design.md`.
 */

import { STACK_COMMANDS, type StackCommand } from "./generated.js";
import { CURSOR_CONTROLLER_OPTION_KEYS, refuseUnknownOptions } from "./options.js";
import type {
  ModeChange,
  PointerBinding,
  ToolMode,
  ToolOutcome,
  ToolRegistry,
} from "./tools.js";

/** Cursor phase. The caller reports it. This module does not watch the pointer. */
export const CURSOR_PHASES = ["idle", "dragging"] as const;

/** One of `idle` or `dragging`. */
export type CursorPhase = (typeof CURSOR_PHASES)[number];

/** One CSS cursor pair. */
export interface CursorPair {
  readonly idle: string;
  readonly dragging: string;
}

/** Local cursors for the eleven stack commands. */
export const COMMAND_CURSORS: Readonly<Record<StackCommand, CursorPair>> = {
  Pan: { idle: "grab", dragging: "grabbing" },
  Zoom: { idle: "zoom-in", dragging: "zoom-in" },
  Rotate: { idle: "crosshair", dragging: "crosshair" },
  Scroll: { idle: "ns-resize", dragging: "ns-resize" },
  Fit: { idle: "default", dragging: "default" },
  FlipHorizontal: { idle: "default", dragging: "default" },
  FlipVertical: { idle: "default", dragging: "default" },
  SetWindow: { idle: "crosshair", dragging: "crosshair" },
  ApplyPreset: { idle: "default", dragging: "default" },
  SetInvert: { idle: "default", dragging: "default" },
  SetColormap: { idle: "default", dragging: "default" },
};

/** A style target. Tests pass a plain object. */
export interface CursorStyle {
  cursor: string;
}

/** `tool.activated` or `tool.modeChanged`. */
export interface ToolStateEvent {
  readonly name: "tool.activated" | "tool.modeChanged";
  readonly groupId: string;
  readonly tool: string;
  readonly command: StackCommand;
  readonly mode: ToolMode;
  readonly previous: ToolMode;
}

/** Wrapper that emits after the registry accepts a mode change. */
export interface ToolState {
  setMode(
    groupId: string,
    tool: string,
    mode: ToolMode,
    bindings: readonly PointerBinding[],
  ): ToolOutcome<ModeChange>;
  registerTool: ToolRegistry["registerTool"];
  removeTool: ToolRegistry["removeTool"];
}

/** Reads the registry and writes one cursor. */
export interface CursorController {
  cursor(phase: CursorPhase): string;
  sync(phase: CursorPhase): void;
}

/** Options for {@link createCursorController}. */
export interface CursorControllerOptions {
  readonly registry: ToolRegistry;
  readonly groupId: string;
  readonly style: CursorStyle;
  readonly cursors?: Partial<Readonly<Record<StackCommand, CursorPair>>>;
}

/**
 * Emit tool-state events for one registry.
 *
 * A direct `registry.setMode` does not emit. A refusal emits nothing.
 */
export function createToolState(
  registry: ToolRegistry,
  listener: (event: ToolStateEvent) => void,
): ToolState {
  return {
    registerTool(groupId, binding) {
      return registry.registerTool(groupId, binding);
    },
    removeTool(groupId, tool) {
      return registry.removeTool(groupId, tool);
    },
    setMode(groupId, tool, mode, bindings) {
      const before = registry.group(groupId);
      const result = registry.setMode(groupId, tool, mode, bindings);
      if (!result.ok || !before.ok) {
        return result;
      }
      const subject = before.value.tools.find((item) => item.tool === tool);
      if (subject === undefined) {
        return result;
      }
      if (subject.mode !== mode) {
        listener({
          name: "tool.modeChanged",
          groupId,
          tool,
          command: subject.command,
          mode,
          previous: subject.mode,
        });
        if (mode === "active") {
          listener({
            name: "tool.activated",
            groupId,
            tool,
            command: subject.command,
            mode,
            previous: subject.mode,
          });
        }
      }
      for (const name of result.value.demoted) {
        const demoted = before.value.tools.find((item) => item.tool === name);
        if (demoted === undefined) {
          continue;
        }
        listener({
          name: "tool.modeChanged",
          groupId,
          tool: name,
          command: demoted.command,
          mode: "passive",
          previous: demoted.mode,
        });
      }
      return result;
    },
  };
}

/**
 * Choose a cursor from the active tools and write it.
 *
 * Primary wins. Otherwise the first active tool in registration order wins.
 */
export function createCursorController(options: CursorControllerOptions): CursorController {
  refuseUnknownOptions("CursorControllerOptions", options, CURSOR_CONTROLLER_OPTION_KEYS);
  if (options.cursors !== undefined) {
    refuseUnknownOptions("CursorControllerOptions.cursors", options.cursors, STACK_COMMANDS);
  }
  const table: Record<StackCommand, CursorPair> = { ...COMMAND_CURSORS };
  if (options.cursors !== undefined) {
    for (const command of Object.keys(options.cursors) as StackCommand[]) {
      const pair = options.cursors[command];
      if (pair !== undefined) {
        table[command] = pair;
      }
    }
  }

  function cursor(phase: CursorPhase): string {
    const group = options.registry.group(options.groupId);
    if (!group.ok) {
      return "default";
    }
    const active = group.value.tools.filter((item) => item.mode === "active");
    const primary = active.find((item) =>
      item.bindings.some((binding) => binding.button === "primary"),
    );
    const chosen = primary ?? active[0];
    if (chosen === undefined) {
      return "default";
    }
    const pair = table[chosen.command];
    return phase === "dragging" ? pair.dragging : pair.idle;
  }

  return {
    cursor,
    sync(phase) {
      options.style.cursor = cursor(phase);
    },
  };
}
