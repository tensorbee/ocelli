import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

import { describe, expect, it } from "vitest";

import { createCursorController, createToolState } from "./cursor.js";
import { COMMAND_CURSORS } from "./index.js";
import type {
  CursorController,
  CursorControllerOptions,
  CursorPair,
  CursorPhase,
  CursorStyle,
  ToolState,
  ToolStateEvent,
} from "./index.js";
import { createToolRegistry } from "./tools.js";
import type { PointerBinding, ToolRegistry } from "./tools.js";
import type { StackCommand } from "./viewport.js";

const SOURCE = readFileSync(
  join(dirname(fileURLToPath(import.meta.url)), "cursor.ts"),
  "utf8",
);

const PRIMARY: PointerBinding = { button: "primary", modifiers: [] };
const SECONDARY: PointerBinding = { button: "secondary", modifiers: [] };

function registry(): ToolRegistry {
  const tools = createToolRegistry();
  expect(tools.createGroup("g").ok).toBe(true);
  return tools;
}

function add(tools: ToolRegistry, name: string, command: StackCommand): void {
  expect(tools.registerTool("g", { tool: name, command }).ok).toBe(true);
}

function publishedTypes(
  state: ToolState,
  controller: CursorController,
  phase: CursorPhase,
  pair: CursorPair,
  style: CursorStyle,
  options: CursorControllerOptions,
): void {
  void state;
  void controller;
  void phase;
  void pair;
  void style;
  void options;
}

function listen(tools: ToolRegistry): { events: ToolStateEvent[]; state: ReturnType<typeof createToolState> } {
  const events: ToolStateEvent[] = [];
  const state = createToolState(tools, (event) => {
    events.push(event);
  });
  return { events, state };
}

describe("tool state", () => {
  it("activation follows the mode change", () => {
    const tools = registry();
    add(tools, "pan", "Pan");
    const { events, state } = listen(tools);
    const result = state.setMode("g", "pan", "active", [PRIMARY]);
    expect(result).toMatchObject({ ok: true, value: { demoted: [] } });
    expect(events.map((event) => event.name)).toEqual(["tool.modeChanged", "tool.activated"]);
    expect(events[0]).toMatchObject({ tool: "pan", mode: "active", previous: "off", command: "Pan" });
    expect(events[1]).toMatchObject({ tool: "pan", previous: "off" });
  });

  it("demotion follows the subject", () => {
    const tools = registry();
    add(tools, "Pan", "Pan");
    add(tools, "Zoom", "Zoom");
    const { events, state } = listen(tools);
    expect(state.setMode("g", "Pan", "active", [PRIMARY]).ok).toBe(true);
    events.splice(0, events.length);
    const result = state.setMode("g", "Zoom", "active", [PRIMARY]);
    expect(result).toMatchObject({ ok: true, value: { demoted: ["Pan"] } });
    expect(events.map((event) => `${event.name}:${event.groupId}:${event.tool}:${event.command}:${event.previous}:${event.mode}`)).toEqual([
      "tool.modeChanged:g:Zoom:Zoom:off:active",
      "tool.activated:g:Zoom:Zoom:off:active",
      "tool.modeChanged:g:Pan:Pan:active:passive",
    ]);
  });

  it("demotion keeps registration order", () => {
    const tools = registry();
    add(tools, "n", "Pan");
    add(tools, "a", "Zoom");
    add(tools, "winner", "Rotate");
    const { events, state } = listen(tools);
    expect(state.setMode("g", "n", "active", [SECONDARY]).ok).toBe(true);
    expect(state.setMode("g", "a", "active", [{ button: "auxiliary", modifiers: [] }]).ok).toBe(true);
    events.splice(0, events.length);
    const result = state.setMode("g", "winner", "active", [
      SECONDARY,
      { button: "auxiliary", modifiers: [] },
    ]);
    expect(result).toMatchObject({ ok: true, value: { demoted: ["n", "a"] } });
    expect(events.map((event) => `${event.name}:${event.tool}:${event.command}`)).toEqual([
      "tool.modeChanged:winner:Rotate",
      "tool.activated:winner:Rotate",
      "tool.modeChanged:n:Pan",
      "tool.modeChanged:a:Zoom",
    ]);
  });

  it("demotion does not follow command order", () => {
    const tools = registry();
    add(tools, "a", "Zoom");
    add(tools, "n", "Pan");
    add(tools, "winner", "Rotate");
    const { events, state } = listen(tools);
    expect(state.setMode("g", "a", "active", [{ button: "auxiliary", modifiers: [] }]).ok).toBe(true);
    expect(state.setMode("g", "n", "active", [SECONDARY]).ok).toBe(true);
    events.splice(0, events.length);
    const result = state.setMode("g", "winner", "active", [
      { button: "auxiliary", modifiers: [] },
      SECONDARY,
    ]);
    expect(result).toMatchObject({ ok: true, value: { demoted: ["a", "n"] } });
    expect(events.map((event) => `${event.name}:${event.tool}:${event.command}`)).toEqual([
      "tool.modeChanged:winner:Rotate",
      "tool.activated:winner:Rotate",
      "tool.modeChanged:a:Zoom",
      "tool.modeChanged:n:Pan",
    ]);
  });

  it("leaving active does not activate", () => {
    const tools = registry();
    add(tools, "pan", "Pan");
    const { events, state } = listen(tools);
    expect(state.setMode("g", "pan", "active", [PRIMARY]).ok).toBe(true);
    events.splice(0, events.length);
    expect(state.setMode("g", "pan", "passive", []).ok).toBe(true);
    expect(events.map((event) => event.name)).toEqual(["tool.modeChanged"]);
    expect(events[0]).toMatchObject({
      tool: "pan",
      command: "Pan",
      mode: "passive",
      previous: "active",
      groupId: "g",
    });
    events.splice(0, events.length);
    expect(state.setMode("g", "pan", "active", [PRIMARY]).ok).toBe(true);
    expect(events.map((event) => event.name)).toEqual(["tool.modeChanged", "tool.activated"]);
    expect(events[0]).toMatchObject({
      tool: "pan",
      command: "Pan",
      mode: "active",
      previous: "passive",
      groupId: "g",
    });
    expect(events[1]).toMatchObject({
      name: "tool.activated",
      tool: "pan",
      command: "Pan",
      mode: "active",
      previous: "passive",
      groupId: "g",
    });
    events.splice(0, events.length);
    expect(state.setMode("g", "pan", "off", []).ok).toBe(true);
    expect(events.map((event) => event.name)).toEqual(["tool.modeChanged"]);
    expect(events[0]).toMatchObject({
      tool: "pan",
      command: "Pan",
      mode: "off",
      previous: "active",
      groupId: "g",
    });
  });

  it("a mode change that is not active still emits", () => {
    const tools = registry();
    add(tools, "pan", "Pan");
    const { events, state } = listen(tools);
    expect(state.setMode("g", "pan", "passive", []).ok).toBe(true);
    expect(events.map((event) => `${event.name}:${event.groupId}:${event.tool}:${event.command}:${event.previous}:${event.mode}`)).toEqual([
      "tool.modeChanged:g:pan:Pan:off:passive",
    ]);
    events.splice(0, events.length);
    expect(state.setMode("g", "pan", "off", []).ok).toBe(true);
    expect(events.map((event) => `${event.name}:${event.groupId}:${event.tool}:${event.command}:${event.previous}:${event.mode}`)).toEqual([
      "tool.modeChanged:g:pan:Pan:passive:off",
    ]);
  });

  it("events use the group that was written", () => {
    const tools = createToolRegistry();
    expect(tools.createGroup("stack").ok).toBe(true);
    expect(tools.registerTool("stack", { tool: "pan", command: "Pan" }).ok).toBe(true);
    expect(tools.registerTool("stack", { tool: "zoom", command: "Zoom" }).ok).toBe(true);
    const { events, state } = listen(tools);
    expect(state.setMode("stack", "pan", "active", [PRIMARY]).ok).toBe(true);
    events.splice(0, events.length);
    expect(state.setMode("stack", "zoom", "active", [PRIMARY]).ok).toBe(true);
    expect(events.map((event) => event.groupId)).toEqual(["stack", "stack", "stack"]);
  });

  it("a listener throw leaves the registry written", () => {
    const tools = registry();
    add(tools, "pan", "Pan");
    const state = createToolState(tools, () => {
      throw new Error("listener");
    });
    expect(() => state.setMode("g", "pan", "active", [PRIMARY])).toThrow("listener");
    expect(tools.group("g")).toMatchObject({
      ok: true,
      value: { tools: [{ tool: "pan", mode: "active", bindings: [PRIMARY] }] },
    });
  });

  it("demotion is not activation", () => {
    const tools = registry();
    add(tools, "Z", "Pan");
    add(tools, "A", "Zoom");
    const { events, state } = listen(tools);
    expect(state.setMode("g", "Z", "active", [PRIMARY]).ok).toBe(true);
    expect(state.setMode("g", "A", "active", [SECONDARY]).ok).toBe(true);
    events.splice(0, events.length);
    const result = state.setMode("g", "A", "active", [SECONDARY, PRIMARY]);
    expect(result).toMatchObject({ ok: true, value: { demoted: ["Z"] } });
    expect(events.map((event) => `${event.name}:${event.tool}:${event.mode}`)).toEqual([
      "tool.modeChanged:Z:passive",
    ]);
    expect(events.some((event) => event.name === "tool.activated")).toBe(false);
  });

  it("a no op emits nothing", () => {
    const tools = registry();
    add(tools, "pan", "Pan");
    const { events, state } = listen(tools);
    expect(state.setMode("g", "pan", "active", [PRIMARY]).ok).toBe(true);
    events.splice(0, events.length);
    expect(state.setMode("g", "pan", "active", [PRIMARY])).toMatchObject({
      ok: true,
      value: { demoted: [] },
    });
    expect(events).toEqual([]);
  });

  it("a refusal is silent", () => {
    const tools = registry();
    add(tools, "pan", "Pan");
    const { events, state } = listen(tools);
    const refused = state.setMode("g", "pan", "active", []);
    expect(refused).toEqual({ ok: false, refusal: "active-needs-binding" });
    expect(events).toEqual([]);
    expect(tools.group("g")).toMatchObject({
      ok: true,
      value: { tools: [{ tool: "pan", mode: "off", bindings: [] }] },
    });
  });

  it("a direct setMode does not emit", () => {
    const tools = registry();
    add(tools, "pan", "Pan");
    const { events } = listen(tools);
    expect(tools.setMode("g", "pan", "active", [PRIMARY]).ok).toBe(true);
    expect(events).toEqual([]);
  });

  it("registration is not activation", () => {
    const tools = registry();
    const { events, state } = listen(tools);
    expect(state.registerTool("g", { tool: "pan", command: "Pan" }).ok).toBe(true);
    expect(state.removeTool("g", "pan").ok).toBe(true);
    expect(events).toEqual([]);
  });

  it("the wrapper does not change demotion", () => {
    const tools = registry();
    add(tools, "Z", "Pan");
    add(tools, "A", "Zoom");
    const direct = createToolRegistry();
    expect(direct.createGroup("g").ok).toBe(true);
    expect(direct.registerTool("g", { tool: "Z", command: "Pan" }).ok).toBe(true);
    expect(direct.registerTool("g", { tool: "A", command: "Zoom" }).ok).toBe(true);
    expect(tools.setMode("g", "Z", "active", [PRIMARY]).ok).toBe(true);
    expect(direct.setMode("g", "Z", "active", [PRIMARY]).ok).toBe(true);
    const { state } = listen(tools);
    const wrapped = state.setMode("g", "A", "active", [PRIMARY]);
    const plain = direct.setMode("g", "A", "active", [PRIMARY]);
    expect(wrapped).toEqual(plain);
  });
});

describe("cursor controller", () => {
  it("sync writes the style", () => {
    const tools = registry();
    add(tools, "pan", "Pan");
    expect(tools.setMode("g", "pan", "active", [PRIMARY]).ok).toBe(true);
    const style: CursorStyle = { cursor: "grab" };
    const cursor = createCursorController({ registry: tools, groupId: "g", style });
    style.cursor = "stale";
    cursor.sync("idle");
    expect(style.cursor).toBe("grab");
    style.cursor = "stale";
    cursor.sync("dragging");
    expect(style.cursor).toBe("grabbing");

    let writes = 0;
    let current = "grab";
    const counted: CursorStyle = {
      get cursor() {
        return current;
      },
      set cursor(value: string) {
        writes += 1;
        current = value;
      },
    };
    const again = createCursorController({ registry: tools, groupId: "g", style: counted });
    again.sync("idle");
    again.sync("idle");
    expect(writes).toBe(2);
    expect(current).toBe("grab");
  });

  it("each command has its cursor", () => {
    const rows: ReadonlyArray<readonly [StackCommand, string, string]> = [
      ["Pan", "grab", "grabbing"],
      ["Zoom", "zoom-in", "zoom-in"],
      ["Rotate", "crosshair", "crosshair"],
      ["Scroll", "ns-resize", "ns-resize"],
      ["Fit", "default", "default"],
      ["FlipHorizontal", "default", "default"],
      ["FlipVertical", "default", "default"],
      ["SetWindow", "crosshair", "crosshair"],
      ["ApplyPreset", "default", "default"],
      ["SetInvert", "default", "default"],
      ["SetColormap", "default", "default"],
    ];
    for (const [command, idle, dragging] of rows) {
      const tools = registry();
      add(tools, "tool", command);
      expect(tools.setMode("g", "tool", "active", [PRIMARY]).ok).toBe(true);
      const cursor = createCursorController({
        registry: tools,
        groupId: "g",
        style: { cursor: "" },
      });
      expect(cursor.cursor("idle")).toBe(idle);
      expect(cursor.cursor("dragging")).toBe(dragging);
    }
  });

  it("pan uses grab and grabbing", () => {
    const tools = registry();
    add(tools, "pan", "Pan");
    expect(tools.setMode("g", "pan", "active", [PRIMARY]).ok).toBe(true);
    const style: CursorStyle = { cursor: "" };
    const cursor = createCursorController({ registry: tools, groupId: "g", style });
    expect(cursor.cursor("idle")).toBe("grab");
    expect(cursor.cursor("dragging")).toBe("grabbing");
    cursor.sync("idle");
    expect(style.cursor).toBe("grab");
    cursor.sync("dragging");
    expect(style.cursor).toBe("grabbing");
  });

  it("primary owns the cursor", () => {
    const tools = registry();
    add(tools, "Z", "Pan");
    add(tools, "A", "Zoom");
    expect(tools.setMode("g", "Z", "active", [SECONDARY]).ok).toBe(true);
    expect(tools.setMode("g", "A", "active", [PRIMARY]).ok).toBe(true);
    const style: CursorStyle = { cursor: "" };
    const cursor = createCursorController({ registry: tools, groupId: "g", style });
    expect(cursor.cursor("idle")).toBe("zoom-in");
    expect(cursor.cursor("dragging")).toBe("zoom-in");
    cursor.sync("idle");
    expect(style.cursor).toBe("zoom-in");
    cursor.sync("dragging");
    expect(style.cursor).toBe("zoom-in");
  });

  it("the first active tool owns the cursor", () => {
    const tools = registry();
    add(tools, "Z", "Pan");
    add(tools, "A", "Zoom");
    expect(tools.setMode("g", "Z", "active", [SECONDARY]).ok).toBe(true);
    expect(tools.setMode("g", "A", "active", [{ button: "auxiliary", modifiers: [] }]).ok).toBe(true);
    const style: CursorStyle = { cursor: "" };
    const cursor = createCursorController({ registry: tools, groupId: "g", style });
    expect(cursor.cursor("idle")).toBe("grab");
    expect(cursor.cursor("dragging")).toBe("grabbing");
    cursor.sync("dragging");
    expect(style.cursor).toBe("grabbing");
  });

  it("the first active tool does not follow command order", () => {
    const tools = registry();
    add(tools, "zoom", "Zoom");
    add(tools, "pan", "Pan");
    expect(tools.setMode("g", "zoom", "active", [{ button: "auxiliary", modifiers: [] }]).ok).toBe(true);
    expect(tools.setMode("g", "pan", "active", [SECONDARY]).ok).toBe(true);
    const style: CursorStyle = { cursor: "" };
    const cursor = createCursorController({ registry: tools, groupId: "g", style });
    expect(cursor.cursor("idle")).toBe("zoom-in");
    expect(cursor.cursor("dragging")).toBe("zoom-in");
    cursor.sync("idle");
    expect(style.cursor).toBe("zoom-in");
    cursor.sync("dragging");
    expect(style.cursor).toBe("zoom-in");
  });

  it("a default cursor does not follow command order", () => {
    const tools = registry();
    add(tools, "invert", "SetInvert");
    add(tools, "pan", "Pan");
    expect(tools.setMode("g", "invert", "active", [{ button: "auxiliary", modifiers: [] }]).ok).toBe(true);
    expect(tools.setMode("g", "pan", "active", [SECONDARY]).ok).toBe(true);
    const style: CursorStyle = { cursor: "grab" };
    const cursor = createCursorController({ registry: tools, groupId: "g", style });
    expect(cursor.cursor("idle")).toBe("default");
    expect(cursor.cursor("dragging")).toBe("default");
    cursor.sync("idle");
    expect(style.cursor).toBe("default");
    cursor.sync("dragging");
    expect(style.cursor).toBe("default");
  });

  it("a default cursor is not skipped", () => {
    const tools = registry();
    add(tools, "fit", "Fit");
    add(tools, "pan", "Pan");
    expect(tools.setMode("g", "fit", "active", [PRIMARY]).ok).toBe(true);
    expect(tools.setMode("g", "pan", "active", [SECONDARY]).ok).toBe(true);
    const style: CursorStyle = { cursor: "grab" };
    const cursor = createCursorController({ registry: tools, groupId: "g", style });
    expect(cursor.cursor("idle")).toBe("default");
    expect(cursor.cursor("dragging")).toBe("default");
    cursor.sync("idle");
    expect(style.cursor).toBe("default");
    cursor.sync("dragging");
    expect(style.cursor).toBe("default");
  });

  it("the first active default cursor is kept", () => {
    const tools = registry();
    add(tools, "fit", "Fit");
    add(tools, "pan", "Pan");
    expect(tools.setMode("g", "fit", "active", [SECONDARY]).ok).toBe(true);
    expect(tools.setMode("g", "pan", "active", [{ button: "auxiliary", modifiers: [] }]).ok).toBe(true);
    const style: CursorStyle = { cursor: "grab" };
    const cursor = createCursorController({ registry: tools, groupId: "g", style });
    expect(cursor.cursor("idle")).toBe("default");
    expect(cursor.cursor("dragging")).toBe("default");
    cursor.sync("idle");
    expect(style.cursor).toBe("default");
    cursor.sync("dragging");
    expect(style.cursor).toBe("default");
  });

  it("primary can sit after another binding", () => {
    const tools = registry();
    add(tools, "pan", "Pan");
    add(tools, "zoom", "Zoom");
    expect(tools.setMode("g", "pan", "active", [SECONDARY]).ok).toBe(true);
    expect(tools.setMode("g", "zoom", "active", [
      { button: "auxiliary", modifiers: [] },
      PRIMARY,
    ]).ok).toBe(true);
    const style: CursorStyle = { cursor: "" };
    const cursor = createCursorController({ registry: tools, groupId: "g", style });
    expect(cursor.cursor("idle")).toBe("zoom-in");
    expect(cursor.cursor("dragging")).toBe("zoom-in");
    cursor.sync("idle");
    expect(style.cursor).toBe("zoom-in");
    cursor.sync("dragging");
    expect(style.cursor).toBe("zoom-in");
  });

  it("primary with a modifier still owns the cursor", () => {
    const tools = registry();
    add(tools, "pan", "Pan");
    add(tools, "zoom", "Zoom");
    expect(tools.setMode("g", "pan", "active", [SECONDARY]).ok).toBe(true);
    expect(tools.setMode("g", "zoom", "active", [{ button: "primary", modifiers: ["shift"] }]).ok).toBe(true);
    const style: CursorStyle = { cursor: "" };
    const cursor = createCursorController({ registry: tools, groupId: "g", style });
    expect(cursor.cursor("idle")).toBe("zoom-in");
    expect(cursor.cursor("dragging")).toBe("zoom-in");
    cursor.sync("idle");
    expect(style.cursor).toBe("zoom-in");
    cursor.sync("dragging");
    expect(style.cursor).toBe("zoom-in");
  });

  it("a later primary default cursor is kept", () => {
    const tools = registry();
    add(tools, "pan", "Pan");
    add(tools, "fit", "Fit");
    expect(tools.setMode("g", "pan", "active", [SECONDARY]).ok).toBe(true);
    expect(tools.setMode("g", "fit", "active", [PRIMARY]).ok).toBe(true);
    const style: CursorStyle = { cursor: "grab" };
    const cursor = createCursorController({ registry: tools, groupId: "g", style });
    expect(cursor.cursor("idle")).toBe("default");
    expect(cursor.cursor("dragging")).toBe("default");
    cursor.sync("idle");
    expect(style.cursor).toBe("default");
    cursor.sync("dragging");
    expect(style.cursor).toBe("default");
  });

  it("a later modified primary still owns the cursor", () => {
    const tools = registry();
    add(tools, "pan", "Pan");
    add(tools, "zoom", "Zoom");
    expect(tools.setMode("g", "pan", "active", [SECONDARY]).ok).toBe(true);
    expect(tools.setMode("g", "zoom", "active", [
      { button: "auxiliary", modifiers: [] },
      { button: "primary", modifiers: ["shift"] },
    ]).ok).toBe(true);
    const style: CursorStyle = { cursor: "" };
    const cursor = createCursorController({ registry: tools, groupId: "g", style });
    expect(cursor.cursor("idle")).toBe("zoom-in");
    expect(cursor.cursor("dragging")).toBe("zoom-in");
    cursor.sync("idle");
    expect(style.cursor).toBe("zoom-in");
    cursor.sync("dragging");
    expect(style.cursor).toBe("zoom-in");
  });

  it("a later modified primary keeps a default cursor", () => {
    const tools = registry();
    add(tools, "pan", "Pan");
    add(tools, "fit", "Fit");
    expect(tools.setMode("g", "pan", "active", [SECONDARY]).ok).toBe(true);
    expect(tools.setMode("g", "fit", "active", [
      { button: "auxiliary", modifiers: [] },
      { button: "primary", modifiers: ["shift"] },
    ]).ok).toBe(true);
    const style: CursorStyle = { cursor: "grab" };
    const cursor = createCursorController({ registry: tools, groupId: "g", style });
    expect(cursor.cursor("idle")).toBe("default");
    expect(cursor.cursor("dragging")).toBe("default");
    cursor.sync("idle");
    expect(style.cursor).toBe("default");
    cursor.sync("dragging");
    expect(style.cursor).toBe("default");
  });

  it("a passive tool does not own the cursor", () => {
    const tools = registry();
    add(tools, "pan", "Pan");
    expect(tools.setMode("g", "pan", "active", [PRIMARY]).ok).toBe(true);
    expect(tools.setMode("g", "pan", "passive", []).ok).toBe(true);
    const cursor = createCursorController({
      registry: tools,
      groupId: "g",
      style: { cursor: "" },
    });
    expect(cursor.cursor("idle")).toBe("default");
    expect(cursor.cursor("dragging")).toBe("default");
  });

  it("an unknown group uses the default cursor", () => {
    const tools = registry();
    add(tools, "pan", "Pan");
    expect(tools.setMode("g", "pan", "active", [PRIMARY]).ok).toBe(true);
    const style: CursorStyle = { cursor: "grab" };
    const cursor = createCursorController({
      registry: tools,
      groupId: "missing",
      style,
    });
    expect(cursor.cursor("idle")).toBe("default");
    expect(cursor.cursor("dragging")).toBe("default");
    cursor.sync("idle");
    expect(style.cursor).toBe("default");
    cursor.sync("dragging");
    expect(style.cursor).toBe("default");
  });

  it("the controller reads its group", () => {
    const tools = registry();
    expect(tools.createGroup("other").ok).toBe(true);
    add(tools, "pan", "Pan");
    expect(tools.registerTool("other", { tool: "zoom", command: "Zoom" }).ok).toBe(true);
    expect(tools.setMode("g", "pan", "active", [PRIMARY]).ok).toBe(true);
    expect(tools.setMode("other", "zoom", "active", [PRIMARY]).ok).toBe(true);
    const here = createCursorController({
      registry: tools,
      groupId: "other",
      style: { cursor: "" },
    });
    expect(here.cursor("idle")).toBe("zoom-in");
    expect(here.cursor("dragging")).toBe("zoom-in");
  });

  it("an idle group uses the default cursor", () => {
    const tools = registry();
    add(tools, "pan", "Pan");
    const style: CursorStyle = { cursor: "grab" };
    const cursor = createCursorController({ registry: tools, groupId: "g", style });
    cursor.sync("idle");
    expect(style.cursor).toBe("default");
  });

  it("an override replaces one cursor", () => {
    const tools = registry();
    add(tools, "pan", "Pan");
    add(tools, "zoom", "Zoom");
    expect(tools.setMode("g", "pan", "active", [PRIMARY]).ok).toBe(true);
    const style: CursorStyle = { cursor: "" };
    const cursor = createCursorController({
      registry: tools,
      groupId: "g",
      style,
      cursors: { Pan: { idle: "move", dragging: "move" } },
    });
    expect(cursor.cursor("idle")).toBe("move");
    expect(cursor.cursor("dragging")).toBe("move");
    const plain = createCursorController({
      registry: tools,
      groupId: "g",
      style: { cursor: "" },
    });
    expect(plain.cursor("idle")).toBe("grab");
    expect(plain.cursor("dragging")).toBe("grabbing");
    expect(COMMAND_CURSORS.Pan).toEqual({ idle: "grab", dragging: "grabbing" });
    expect(tools.setMode("g", "pan", "off", []).ok).toBe(true);
    expect(tools.setMode("g", "zoom", "active", [PRIMARY]).ok).toBe(true);
    expect(cursor.cursor("idle")).toBe("zoom-in");
    const phase: CursorPhase = "idle";
    const pair: CursorPair = { idle: "move", dragging: "move" };
    const options: CursorControllerOptions = {
      registry: tools,
      groupId: "g",
      style,
    };
    publishedTypes(createToolState(tools, () => {}), cursor, phase, pair, style, options);
  });

  it("the cursor module does not listen", () => {
    expect(SOURCE.includes("addEventListener")).toBe(false);
    expect(SOURCE.includes("pointer.ts")).toBe(false);
    expect(SOURCE.includes("pointer.js")).toBe(false);
  });
});
