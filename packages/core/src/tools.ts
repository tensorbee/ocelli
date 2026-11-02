/**
 * Tool lifecycle, groups, bindings and modes.
 *
 * HLD section 3 puts tool interaction state in the shell. This module stores
 * that state for one caller. It does not listen for events, pack a command,
 * or compute a hit or a length. Those numbers live in `ocelli-geom`.
 *
 * The mode names and the demotion rule are the local decisions in
 * `.claude/plans/F-068-design.md`. The HLD does not name them.
 */

import { STACK_COMMANDS } from "./generated.js";
import type { StackCommand } from "./generated.js";
import type { ToolBinding, ViewportId } from "./viewport.js";

/** Stored modes. `active` is the only mode that owns a pointer binding. */
export const TOOL_MODES = ["active", "passive", "off"] as const;

/** One of `active`, `passive` or `off`. */
export type ToolMode = (typeof TOOL_MODES)[number];

/**
 * Pointer buttons.
 *
 * `primary` is DOM button 0, `auxiliary` is button 1 and `secondary` is
 * button 2. The names are this registry's, not a copied binding enum.
 */
export const POINTER_BUTTONS = ["primary", "auxiliary", "secondary"] as const;

/** One of `primary`, `auxiliary` or `secondary`. */
export type PointerButton = (typeof POINTER_BUTTONS)[number];

/** Modifier keys, in the order a stored binding keeps them. */
export const POINTER_MODIFIERS = ["alt", "ctrl", "meta", "shift"] as const;

/** One of `alt`, `ctrl`, `meta` or `shift`. */
export type PointerModifier = (typeof POINTER_MODIFIERS)[number];

/** One button plus a set of modifiers. Order does not make a second binding. */
export interface PointerBinding {
  readonly button: PointerButton;
  readonly modifiers: readonly PointerModifier[];
}

/** Every refusal `ToolRegistry` can return. A bad call changes nothing. */
export const TOOL_REFUSALS = [
  "empty-group",
  "duplicate-group",
  "unknown-group",
  "empty-tool",
  "duplicate-tool",
  "unknown-tool",
  "unknown-command",
  "bad-mode",
  "bad-binding",
  "active-needs-binding",
  "bindings-only-when-active",
  "duplicate-binding",
  "bad-viewport",
  "duplicate-viewport",
  "missing-viewport",
  "viewport-taken",
] as const;

/** One refusal name. */
export type ToolRefusal = (typeof TOOL_REFUSALS)[number];

/** A successful value, or the refusal that left the registry unchanged. */
export type ToolOutcome<T> =
  | { readonly ok: true; readonly value: T }
  | { readonly ok: false; readonly refusal: ToolRefusal };

/** One stored tool. Reads return a copy of this shape. */
export interface RegisteredTool {
  readonly tool: string;
  readonly command: StackCommand;
  readonly mode: ToolMode;
  readonly bindings: readonly PointerBinding[];
}

/** Names moved from `active` to `passive` by a `setMode` that took their binding. */
export interface ModeChange {
  readonly demoted: readonly string[];
}

/** A copy of one group. Mutating the arrays does not change the registry. */
export interface ToolGroupView {
  readonly id: string;
  readonly tools: readonly RegisteredTool[];
  readonly viewports: readonly ViewportId[];
}

/**
 * Caller-owned tool state.
 *
 * A second `createToolRegistry` does not see this one's groups.
 */
export interface ToolRegistry {
  /** Insert an empty group. The id is stored as given. */
  createGroup(id: string): ToolOutcome<string>;
  /** Drop the group and release its viewports. */
  destroyGroup(id: string): ToolOutcome<string>;
  /** Group ids in creation order. */
  groups(): readonly string[];
  /** A copy of the group. */
  group(id: string): ToolOutcome<ToolGroupView>;
  /** Add a tool in mode `off` with no bindings. Does not run `command`. */
  registerTool(groupId: string, binding: ToolBinding): ToolOutcome<RegisteredTool>;
  /** Remove a tool. No other tool is promoted. */
  removeTool(groupId: string, tool: string): ToolOutcome<string>;
  /**
   * Set the mode. `active` requires a binding. Any other active tool that
   * owns one of those bindings becomes `passive` and loses every binding.
   */
  setMode(
    groupId: string,
    tool: string,
    mode: ToolMode,
    bindings: readonly PointerBinding[],
  ): ToolOutcome<ModeChange>;
  /** Attach a viewport. One viewport belongs to at most one group. */
  attachViewport(groupId: string, id: ViewportId): ToolOutcome<ViewportId>;
  /** Detach a viewport this group currently lists. */
  detachViewport(groupId: string, id: ViewportId): ToolOutcome<ViewportId>;
  /** The active tool for this binding, or null when nobody owns it. */
  bindingOwner(
    groupId: string,
    binding: PointerBinding,
  ): ToolOutcome<RegisteredTool | null>;
}

interface StoredTool {
  tool: string;
  command: StackCommand;
  mode: ToolMode;
  bindings: PointerBinding[];
}

interface StoredGroup {
  id: string;
  tools: StoredTool[];
  viewports: ViewportId[];
}

function refused<T>(refusal: ToolRefusal): ToolOutcome<T> {
  return { ok: false, refusal };
}

function accepted<T>(value: T): ToolOutcome<T> {
  return { ok: true, value };
}

function blank(value: unknown): boolean {
  return typeof value !== "string" || value.trim().length === 0;
}

function isMode(value: unknown): value is ToolMode {
  return typeof value === "string" && (TOOL_MODES as readonly string[]).includes(value);
}

function isButton(value: unknown): value is PointerButton {
  return (
    typeof value === "string" && (POINTER_BUTTONS as readonly string[]).includes(value)
  );
}

function isModifier(value: unknown): value is PointerModifier {
  return (
    typeof value === "string" &&
    (POINTER_MODIFIERS as readonly string[]).includes(value)
  );
}

function isCommand(value: unknown): value is StackCommand {
  return (
    typeof value === "string" && (STACK_COMMANDS as readonly string[]).includes(value)
  );
}

function normalizeBinding(binding: PointerBinding): PointerBinding | ToolRefusal {
  if (binding === null || typeof binding !== "object") {
    return "bad-binding";
  }
  if (!isButton(binding.button) || !Array.isArray(binding.modifiers)) {
    return "bad-binding";
  }
  const modifiers: PointerModifier[] = [];
  for (const modifier of binding.modifiers) {
    if (!isModifier(modifier)) {
      return "bad-binding";
    }
    if (!modifiers.includes(modifier)) {
      modifiers.push(modifier);
    }
  }
  modifiers.sort(
    (left, right) => POINTER_MODIFIERS.indexOf(left) - POINTER_MODIFIERS.indexOf(right),
  );
  return { button: binding.button, modifiers };
}

function bindingKey(binding: PointerBinding): string {
  return `${binding.button}\u0000${binding.modifiers.join("\u0000")}`;
}

function sameBindings(
  left: readonly PointerBinding[],
  right: readonly PointerBinding[],
): boolean {
  if (left.length !== right.length) {
    return false;
  }
  const keys = new Set(left.map(bindingKey));
  return right.every((binding) => keys.has(bindingKey(binding)));
}

function copyTool(tool: StoredTool): RegisteredTool {
  return {
    tool: tool.tool,
    command: tool.command,
    mode: tool.mode,
    bindings: tool.bindings.map((binding) => ({
      button: binding.button,
      modifiers: [...binding.modifiers],
    })),
  };
}

function usableViewport(id: ViewportId): boolean {
  return Number.isSafeInteger(id) && id >= 0;
}

/** A new registry. It shares nothing with any registry created earlier. */
export function createToolRegistry(): ToolRegistry {
  const groups: StoredGroup[] = [];

  function findGroup(id: string): StoredGroup | undefined {
    return groups.find((group) => group.id === id);
  }

  return {
    groups() {
      return groups.map((group) => group.id);
    },

    createGroup(id) {
      if (blank(id)) {
        return refused("empty-group");
      }
      if (findGroup(id) !== undefined) {
        return refused("duplicate-group");
      }
      groups.push({ id, tools: [], viewports: [] });
      return accepted(id);
    },

    destroyGroup(id) {
      const index = groups.findIndex((group) => group.id === id);
      if (index < 0) {
        return refused("unknown-group");
      }
      groups.splice(index, 1);
      return accepted(id);
    },

    group(id) {
      const found = findGroup(id);
      if (found === undefined) {
        return refused("unknown-group");
      }
      return accepted({
        id: found.id,
        tools: found.tools.map(copyTool),
        viewports: [...found.viewports],
      });
    },

    registerTool(groupId, binding) {
      const found = findGroup(groupId);
      if (found === undefined) {
        return refused("unknown-group");
      }
      if (blank(binding.tool)) {
        return refused("empty-tool");
      }
      if (!isCommand(binding.command)) {
        return refused("unknown-command");
      }
      if (found.tools.some((tool) => tool.tool === binding.tool)) {
        return refused("duplicate-tool");
      }
      const stored: StoredTool = {
        tool: binding.tool,
        command: binding.command,
        mode: "off",
        bindings: [],
      };
      found.tools.push(stored);
      return accepted(copyTool(stored));
    },

    removeTool(groupId, tool) {
      const found = findGroup(groupId);
      if (found === undefined) {
        return refused("unknown-group");
      }
      const index = found.tools.findIndex((row) => row.tool === tool);
      if (index < 0) {
        return refused("unknown-tool");
      }
      found.tools.splice(index, 1);
      return accepted(tool);
    },

    setMode(groupId, tool, mode, bindings) {
      const found = findGroup(groupId);
      if (found === undefined) {
        return refused("unknown-group");
      }
      const subject = found.tools.find((row) => row.tool === tool);
      if (subject === undefined) {
        return refused("unknown-tool");
      }
      if (!isMode(mode)) {
        return refused("bad-mode");
      }
      if (!Array.isArray(bindings)) {
        return refused("bad-binding");
      }
      if (mode === "active") {
        if (bindings.length === 0) {
          return refused("active-needs-binding");
        }
      } else if (bindings.length !== 0) {
        return refused("bindings-only-when-active");
      }

      const normalized: PointerBinding[] = [];
      const keys = new Set<string>();
      for (const binding of bindings) {
        const next = normalizeBinding(binding);
        if (typeof next === "string") {
          return refused(next);
        }
        const key = bindingKey(next);
        if (keys.has(key)) {
          return refused("duplicate-binding");
        }
        keys.add(key);
        normalized.push(next);
      }

      if (subject.mode === mode && sameBindings(subject.bindings, normalized)) {
        return accepted({ demoted: [] });
      }

      const demoted: string[] = [];
      if (mode === "active") {
        for (const row of found.tools) {
          if (row.tool === subject.tool || row.mode !== "active") {
            continue;
          }
          const owns = row.bindings.some((binding) => keys.has(bindingKey(binding)));
          if (owns) {
            demoted.push(row.tool);
          }
        }
      }

      for (const name of demoted) {
        const row = found.tools.find((item) => item.tool === name);
        if (row === undefined) {
          continue;
        }
        row.mode = "passive";
        row.bindings = [];
      }
      subject.mode = mode;
      subject.bindings = normalized;
      return accepted({ demoted: [...demoted] });
    },

    attachViewport(groupId, id) {
      const found = findGroup(groupId);
      if (found === undefined) {
        return refused("unknown-group");
      }
      if (!usableViewport(id)) {
        return refused("bad-viewport");
      }
      if (found.viewports.includes(id)) {
        return refused("duplicate-viewport");
      }
      const taken = groups.some(
        (group) => group.id !== found.id && group.viewports.includes(id),
      );
      if (taken) {
        return refused("viewport-taken");
      }
      found.viewports.push(id);
      return accepted(id);
    },

    detachViewport(groupId, id) {
      const found = findGroup(groupId);
      if (found === undefined) {
        return refused("unknown-group");
      }
      if (!usableViewport(id)) {
        return refused("bad-viewport");
      }
      const index = found.viewports.indexOf(id);
      if (index < 0) {
        return refused("missing-viewport");
      }
      found.viewports.splice(index, 1);
      return accepted(id);
    },

    bindingOwner(groupId, binding) {
      const found = findGroup(groupId);
      if (found === undefined) {
        return refused("unknown-group");
      }
      const normalized = normalizeBinding(binding);
      if (typeof normalized === "string") {
        return refused(normalized);
      }
      const key = bindingKey(normalized);
      const owner = found.tools.find(
        (row) =>
          row.mode === "active" &&
          row.bindings.some((item) => bindingKey(item) === key),
      );
      return accepted(owner === undefined ? null : copyTool(owner));
    },
  };
}
