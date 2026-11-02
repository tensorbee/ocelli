import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

import { describe, expect, it } from "vitest";

import { createToolRegistry } from "./tools.js";
import type { PointerBinding, ToolRegistry } from "./tools.js";
import { viewportId } from "./viewport.js";
import type { StackCommand, ToolBinding } from "./viewport.js";

const SOURCE = readFileSync(
  join(dirname(fileURLToPath(import.meta.url)), "tools.ts"),
  "utf8",
);

const PRIMARY: PointerBinding = { button: "primary", modifiers: [] };
const PRIMARY_SHIFT: PointerBinding = { button: "primary", modifiers: ["shift"] };
const AUXILIARY: PointerBinding = { button: "auxiliary", modifiers: [] };

function tool(name: string, command: StackCommand): ToolBinding {
  return { tool: name, command };
}

function registry(): ToolRegistry {
  const tools = createToolRegistry();
  const created = tools.createGroup("g");
  expect(created).toEqual({ ok: true, value: "g" });
  return tools;
}

function snapshot(tools: ToolRegistry): unknown {
  const view = tools.group("g");
  expect(view.ok).toBe(true);
  return view.ok ? view.value : null;
}

describe("tool registry", () => {
  it("a second active tool takes the binding", () => {
    const tools = registry();
    expect(tools.registerTool("g", tool("Pan", "Pan")).ok).toBe(true);
    expect(tools.registerTool("g", tool("Zoom", "Zoom")).ok).toBe(true);
    expect(tools.setMode("g", "Pan", "active", [PRIMARY])).toEqual({
      ok: true,
      value: { demoted: [] },
    });
    expect(tools.setMode("g", "Zoom", "active", [PRIMARY])).toEqual({
      ok: true,
      value: { demoted: ["Pan"] },
    });
    const view = tools.group("g");
    expect(view.ok).toBe(true);
    if (!view.ok) {
      return;
    }
    expect(view.value.tools.map((row) => [row.tool, row.mode, row.bindings.length])).toEqual([
      ["Pan", "passive", 0],
      ["Zoom", "active", 1],
    ]);
    expect(tools.bindingOwner("g", PRIMARY)).toEqual({
      ok: true,
      value: expect.objectContaining({ tool: "Zoom", command: "Zoom", mode: "active" }),
    });
  });

  it("taking one binding demotes the whole owner", () => {
    const tools = registry();
    expect(tools.registerTool("g", tool("Pan", "Pan")).ok).toBe(true);
    expect(tools.registerTool("g", tool("Zoom", "Zoom")).ok).toBe(true);
    expect(
      tools.setMode("g", "Pan", "active", [PRIMARY, AUXILIARY]),
    ).toEqual({ ok: true, value: { demoted: [] } });
    expect(tools.setMode("g", "Zoom", "active", [PRIMARY])).toEqual({
      ok: true,
      value: { demoted: ["Pan"] },
    });
    expect(tools.bindingOwner("g", AUXILIARY)).toEqual({ ok: true, value: null });
    expect(tools.bindingOwner("g", PRIMARY)).toEqual({
      ok: true,
      value: expect.objectContaining({ tool: "Zoom" }),
    });
    const view = tools.group("g");
    expect(view.ok && view.value.tools[0]).toEqual({
      tool: "Pan",
      command: "Pan",
      mode: "passive",
      bindings: [],
    });
  });

  it("modifiers split a button into two bindings", () => {
    const tools = registry();
    expect(tools.registerTool("g", tool("Pan", "Pan")).ok).toBe(true);
    expect(tools.registerTool("g", tool("Zoom", "Zoom")).ok).toBe(true);
    expect(tools.setMode("g", "Pan", "active", [PRIMARY]).ok).toBe(true);
    expect(tools.setMode("g", "Zoom", "active", [PRIMARY_SHIFT])).toEqual({
      ok: true,
      value: { demoted: [] },
    });
    expect(tools.bindingOwner("g", PRIMARY)).toEqual({
      ok: true,
      value: expect.objectContaining({ tool: "Pan" }),
    });
    expect(tools.bindingOwner("g", { button: "primary", modifiers: ["shift"] })).toEqual({
      ok: true,
      value: expect.objectContaining({ tool: "Zoom" }),
    });
  });

  it("an active tool needs a binding", () => {
    const tools = registry();
    expect(tools.registerTool("g", tool("Pan", "Pan")).ok).toBe(true);
    expect(tools.setMode("g", "Pan", "active", [PRIMARY]).ok).toBe(true);
    const before = snapshot(tools);
    expect(tools.setMode("g", "Pan", "active", [])).toEqual({
      ok: false,
      refusal: "active-needs-binding",
    });
    expect(snapshot(tools)).toEqual(before);
  });

  it("passive and off store no bindings", () => {
    const tools = registry();
    expect(tools.registerTool("g", tool("Pan", "Pan")).ok).toBe(true);
    expect(tools.setMode("g", "Pan", "active", [PRIMARY]).ok).toBe(true);
    const before = snapshot(tools);
    expect(tools.setMode("g", "Pan", "passive", [PRIMARY])).toEqual({
      ok: false,
      refusal: "bindings-only-when-active",
    });
    expect(snapshot(tools)).toEqual(before);
    expect(tools.setMode("g", "Pan", "passive", [])).toEqual({
      ok: true,
      value: { demoted: [] },
    });
    expect(tools.bindingOwner("g", PRIMARY)).toEqual({ ok: true, value: null });
    const passive = tools.group("g");
    expect(passive.ok && passive.value.tools[0]).toEqual({
      tool: "Pan",
      command: "Pan",
      mode: "passive",
      bindings: [],
    });
    expect(tools.setMode("g", "Pan", "off", [AUXILIARY])).toEqual({
      ok: false,
      refusal: "bindings-only-when-active",
    });
    expect(snapshot(tools)).toEqual(passive.ok ? passive.value : null);
    expect(tools.setMode("g", "Pan", "active", [PRIMARY]).ok).toBe(true);
    expect(tools.setMode("g", "Pan", "off", [])).toEqual({
      ok: true,
      value: { demoted: [] },
    });
    const off = tools.group("g");
    expect(off.ok && off.value.tools[0]).toEqual({
      tool: "Pan",
      command: "Pan",
      mode: "off",
      bindings: [],
    });
    expect(tools.bindingOwner("g", PRIMARY)).toEqual({ ok: true, value: null });
  });

  it("a second registration does not replace the command", () => {
    const tools = registry();
    expect(tools.registerTool("g", tool("Pan", "Pan")).ok).toBe(true);
    expect(tools.registerTool("g", tool("Pan", "Zoom"))).toEqual({
      ok: false,
      refusal: "duplicate-tool",
    });
    const view = tools.group("g");
    expect(view.ok && view.value.tools[0]?.command).toBe("Pan");
  });

  it("the tool name is not the command", () => {
    const tools = registry();
    const registered = tools.registerTool("g", tool("Pan", "Zoom"));
    expect(registered).toEqual({
      ok: true,
      value: {
        tool: "Pan",
        command: "Zoom",
        mode: "off",
        bindings: [],
      },
    });
  });

  it("refusals leave the registry unchanged", () => {
    const tools = registry();
    expect(tools.registerTool("g", tool("Pan", "Pan")).ok).toBe(true);
    expect(tools.setMode("g", "Pan", "active", [PRIMARY]).ok).toBe(true);
    const before = snapshot(tools);
    expect(tools.createGroup(" ")).toEqual({ ok: false, refusal: "empty-group" });
    expect(tools.createGroup("g")).toEqual({ ok: false, refusal: "duplicate-group" });
    expect(tools.registerTool("missing", tool("Pan", "Pan"))).toEqual({
      ok: false,
      refusal: "unknown-group",
    });
    expect(tools.registerTool("g", tool("  ", "Pan"))).toEqual({
      ok: false,
      refusal: "empty-tool",
    });
    expect(tools.registerTool("g", tool("Nope", "NotACommand" as StackCommand))).toEqual({
      ok: false,
      refusal: "unknown-command",
    });
    expect(tools.setMode("g", "Pan", "enabled" as "active", [PRIMARY])).toEqual({
      ok: false,
      refusal: "bad-mode",
    });
    expect(
      tools.setMode("g", "Pan", "active", [{ button: "middle" as "primary", modifiers: [] }]),
    ).toEqual({ ok: false, refusal: "bad-binding" });
    expect(tools.setMode("g", "Missing", "active", [PRIMARY])).toEqual({
      ok: false,
      refusal: "unknown-tool",
    });
    expect(tools.setMode("g", "Pan", "active", [PRIMARY, PRIMARY])).toEqual({
      ok: false,
      refusal: "duplicate-binding",
    });
    expect(snapshot(tools)).toEqual(before);
    expect(tools.groups()).toEqual(["g"]);
  });

  it("the same mode is a no-op", () => {
    const tools = registry();
    expect(tools.registerTool("g", tool("Pan", "Pan")).ok).toBe(true);
    expect(tools.registerTool("g", tool("Zoom", "Zoom")).ok).toBe(true);
    expect(tools.setMode("g", "Pan", "active", [PRIMARY]).ok).toBe(true);
    expect(tools.setMode("g", "Zoom", "active", [PRIMARY_SHIFT]).ok).toBe(true);
    const before = snapshot(tools);
    expect(
      tools.setMode("g", "Pan", "active", [{ button: "primary", modifiers: [] }]),
    ).toEqual({ ok: true, value: { demoted: [] } });
    expect(snapshot(tools)).toEqual(before);
  });

  it("repeated modifiers collapse", () => {
    const tools = registry();
    expect(tools.registerTool("g", tool("Pan", "Pan")).ok).toBe(true);
    expect(
      tools.setMode("g", "Pan", "active", [
        { button: "primary", modifiers: ["shift", "shift"] },
      ]),
    ).toEqual({ ok: true, value: { demoted: [] } });
    const view = tools.group("g");
    expect(view.ok).toBe(true);
    if (view.ok) {
      expect(view.value.tools[0]?.bindings).toEqual([
        { button: "primary", modifiers: ["shift"] },
      ]);
    }
    expect(tools.bindingOwner("g", PRIMARY_SHIFT)).toEqual({
      ok: true,
      value: expect.objectContaining({ tool: "Pan" }),
    });
  });

  it("a viewport belongs to one group", () => {
    const tools = createToolRegistry();
    expect(tools.createGroup("a").ok).toBe(true);
    expect(tools.createGroup("b").ok).toBe(true);
    const id = viewportId(4);
    expect(tools.attachViewport("a", id)).toEqual({ ok: true, value: id });
    expect(tools.attachViewport("b", id)).toEqual({ ok: false, refusal: "viewport-taken" });
    expect(tools.attachViewport("a", id)).toEqual({
      ok: false,
      refusal: "duplicate-viewport",
    });
    const groupA = tools.group("a");
    const groupB = tools.group("b");
    expect(groupA.ok && groupA.value.viewports).toEqual([id]);
    expect(groupB.ok && groupB.value.viewports).toEqual([]);
    expect(tools.destroyGroup("a")).toEqual({ ok: true, value: "a" });
    expect(tools.attachViewport("b", id)).toEqual({ ok: true, value: id });
  });

  it("detach of an absent viewport is refused", () => {
    const tools = registry();
    const id = viewportId(4);
    expect(tools.attachViewport("g", id).ok).toBe(true);
    expect(tools.detachViewport("g", viewportId(5))).toEqual({
      ok: false,
      refusal: "missing-viewport",
    });
    expect(tools.detachViewport("g", viewportId(-1))).toEqual({
      ok: false,
      refusal: "bad-viewport",
    });
    const view = tools.group("g");
    expect(view.ok && view.value.viewports).toEqual([id]);
    expect(tools.detachViewport("g", id)).toEqual({ ok: true, value: id });
    const after = tools.group("g");
    expect(after.ok && after.value.viewports).toEqual([]);
  });

  it("viewport ids are non-negative integers", () => {
    const tools = registry();
    expect(tools.attachViewport("g", viewportId(0))).toEqual({
      ok: true,
      value: viewportId(0),
    });
    expect(tools.attachViewport("g", viewportId(-1))).toEqual({
      ok: false,
      refusal: "bad-viewport",
    });
    expect(tools.attachViewport("g", viewportId(1.5))).toEqual({
      ok: false,
      refusal: "bad-viewport",
    });
    expect(tools.attachViewport("g", viewportId(Number.NaN))).toEqual({
      ok: false,
      refusal: "bad-viewport",
    });
    expect(tools.attachViewport("g", viewportId(Number.POSITIVE_INFINITY))).toEqual({
      ok: false,
      refusal: "bad-viewport",
    });
    const view = tools.group("g");
    expect(view.ok && view.value.viewports).toEqual([viewportId(0)]);
  });

  it("reads return copies", () => {
    const tools = registry();
    const registered = tools.registerTool("g", tool("Pan", "Pan"));
    expect(registered.ok).toBe(true);
    if (!registered.ok) {
      return;
    }
    expect(tools.setMode("g", "Pan", "active", [PRIMARY_SHIFT]).ok).toBe(true);
    const id = viewportId(4);
    expect(tools.attachViewport("g", id).ok).toBe(true);
    const view = tools.group("g");
    expect(view.ok).toBe(true);
    if (!view.ok) {
      return;
    }
    const listed = view.value.tools as unknown as {
      tool: string;
      command: StackCommand;
      mode: string;
      bindings: { button: string; modifiers: string[] }[];
    }[];
    listed.push({
      tool: "Zoom",
      command: "Zoom",
      mode: "off",
      bindings: [],
    });
    const pan = listed[0];
    if (pan === undefined) {
      return;
    }
    pan.mode = "off";
    pan.bindings.push({ button: "auxiliary", modifiers: [] });
    pan.bindings[0]?.modifiers.push("ctrl");
    (view.value.viewports as unknown as number[]).push(9);
    (tools.groups() as unknown as string[]).push("other");
    const registeredMutable = registered.value as unknown as {
      mode: string;
      bindings: PointerBinding[];
    };
    registeredMutable.mode = "off";
    registeredMutable.bindings.push(PRIMARY);
    const owner = tools.bindingOwner("g", PRIMARY_SHIFT);
    expect(owner.ok).toBe(true);
    if (owner.ok && owner.value !== null) {
      const ownerMutable = owner.value as unknown as {
        mode: string;
        bindings: { modifiers: string[] }[];
      };
      ownerMutable.mode = "passive";
      ownerMutable.bindings[0]?.modifiers.push("alt");
    }
    const again = tools.group("g");
    expect(again.ok && again.value).toEqual({
      id: "g",
      tools: [
        {
          tool: "Pan",
          command: "Pan",
          mode: "active",
          bindings: [{ button: "primary", modifiers: ["shift"] }],
        },
      ],
      viewports: [id],
    });
    expect(tools.groups()).toEqual(["g"]);
    expect(tools.bindingOwner("g", PRIMARY_SHIFT)).toEqual({
      ok: true,
      value: {
        tool: "Pan",
        command: "Pan",
        mode: "active",
        bindings: [{ button: "primary", modifiers: ["shift"] }],
      },
    });
  });

  it("registries are independent", () => {
    const first = createToolRegistry();
    const second = createToolRegistry();
    expect(first.createGroup("g").ok).toBe(true);
    expect(second.group("g")).toEqual({ ok: false, refusal: "unknown-group" });
    expect(second.groups()).toEqual([]);
  });

  it("the registry does not dispatch", () => {
    expect(SOURCE.includes("packCommands")).toBe(false);
    expect(SOURCE.includes("writeCommands")).toBe(false);
    expect(SOURCE.includes("addEventListener")).toBe(false);
  });

  it("removing the active tool does not promote another", () => {
    const tools = registry();
    expect(tools.registerTool("g", tool("Pan", "Pan")).ok).toBe(true);
    expect(tools.registerTool("g", tool("Zoom", "Zoom")).ok).toBe(true);
    expect(tools.setMode("g", "Pan", "active", [PRIMARY]).ok).toBe(true);
    expect(tools.setMode("g", "Zoom", "off", []).ok).toBe(true);
    expect(tools.removeTool("g", "Pan")).toEqual({ ok: true, value: "Pan" });
    expect(tools.bindingOwner("g", PRIMARY)).toEqual({ ok: true, value: null });
    const view = tools.group("g");
    expect(view.ok && view.value.tools).toEqual([
      { tool: "Zoom", command: "Zoom", mode: "off", bindings: [] },
    ]);
  });

  it("modifier order does not make a second binding", () => {
    const tools = registry();
    expect(tools.registerTool("g", tool("Pan", "Pan")).ok).toBe(true);
    expect(
      tools.setMode("g", "Pan", "active", [
        { button: "primary", modifiers: ["shift", "alt"] },
      ]).ok,
    ).toBe(true);
    expect(
      tools.bindingOwner("g", { button: "primary", modifiers: ["shift", "alt"] }),
    ).toEqual({
      ok: true,
      value: expect.objectContaining({ tool: "Pan" }),
    });
    expect(
      tools.bindingOwner("g", { button: "primary", modifiers: ["shift", "shift", "alt"] }),
    ).toEqual({
      ok: true,
      value: expect.objectContaining({ tool: "Pan" }),
    });
    const view = tools.group("g");
    expect(view.ok && view.value.tools[0]?.bindings[0]?.modifiers).toEqual(["alt", "shift"]);
  });

  it("a blank id with a visible character is stored as given", () => {
    const tools = createToolRegistry();
    expect(tools.createGroup(" pan")).toEqual({ ok: true, value: " pan" });
    expect(tools.groups()).toEqual([" pan"]);
    expect(tools.createGroup("pan")).toEqual({ ok: true, value: "pan" });
    expect(tools.registerTool(" pan", tool(" pan", "Pan")).ok).toBe(true);
    expect(tools.registerTool(" pan", tool("pan", "Zoom")).ok).toBe(true);
    const view = tools.group(" pan");
    expect(view.ok && view.value.tools.map((row) => row.tool)).toEqual([" pan", "pan"]);
  });

  it("demotion names follow registration order", () => {
    const tools = registry();
    expect(tools.registerTool("g", tool("Z", "Pan")).ok).toBe(true);
    expect(tools.registerTool("g", tool("A", "Zoom")).ok).toBe(true);
    expect(tools.registerTool("g", tool("M", "Rotate")).ok).toBe(true);
    expect(tools.setMode("g", "Z", "active", [PRIMARY]).ok).toBe(true);
    expect(tools.setMode("g", "A", "active", [AUXILIARY]).ok).toBe(true);
    expect(tools.setMode("g", "M", "active", [AUXILIARY, PRIMARY])).toEqual({
      ok: true,
      value: { demoted: ["Z", "A"] },
    });
    const view = tools.group("g");
    expect(
      view.ok && view.value.tools.map((row) => [row.tool, row.mode, row.bindings.length]),
    ).toEqual([
      ["Z", "passive", 0],
      ["A", "passive", 0],
      ["M", "active", 2],
    ]);
  });

  it("shrinking a binding set does not demote the subject", () => {
    const tools = registry();
    expect(tools.registerTool("g", tool("Pan", "Pan")).ok).toBe(true);
    expect(tools.registerTool("g", tool("Zoom", "Zoom")).ok).toBe(true);
    expect(tools.setMode("g", "Pan", "active", [PRIMARY, AUXILIARY]).ok).toBe(true);
    expect(tools.setMode("g", "Zoom", "active", [PRIMARY_SHIFT]).ok).toBe(true);
    const beforeZoom = tools.bindingOwner("g", PRIMARY_SHIFT);
    expect(tools.setMode("g", "Pan", "active", [AUXILIARY])).toEqual({
      ok: true,
      value: { demoted: [] },
    });
    const view = tools.group("g");
    expect(
      view.ok && view.value.tools.map((row) => [row.tool, row.mode, row.bindings.length]),
    ).toEqual([
      ["Pan", "active", 1],
      ["Zoom", "active", 1],
    ]);
    expect(tools.bindingOwner("g", PRIMARY_SHIFT)).toEqual(beforeZoom);
    expect(tools.bindingOwner("g", PRIMARY)).toEqual({ ok: true, value: null });
  });

  it("a refused binding does not demote another tool", () => {
    const tools = registry();
    expect(tools.registerTool("g", tool("Pan", "Pan")).ok).toBe(true);
    expect(tools.registerTool("g", tool("Zoom", "Zoom")).ok).toBe(true);
    expect(tools.setMode("g", "Pan", "active", [PRIMARY]).ok).toBe(true);
    const before = snapshot(tools);
    expect(tools.setMode("g", "Zoom", "active", [PRIMARY, PRIMARY])).toEqual({
      ok: false,
      refusal: "duplicate-binding",
    });
    expect(snapshot(tools)).toEqual(before);
    expect(
      tools.setMode("g", "Zoom", "active", [
        PRIMARY,
        { button: "middle" as "primary", modifiers: [] },
      ]),
    ).toEqual({ ok: false, refusal: "bad-binding" });
    expect(snapshot(tools)).toEqual(before);
    expect(tools.setMode("g", "Zoom", "active", [])).toEqual({
      ok: false,
      refusal: "active-needs-binding",
    });
    expect(tools.setMode("g", "Zoom", "passive", [PRIMARY])).toEqual({
      ok: false,
      refusal: "bindings-only-when-active",
    });
    expect(snapshot(tools)).toEqual(before);
    expect(tools.bindingOwner("g", PRIMARY)).toEqual({
      ok: true,
      value: expect.objectContaining({ tool: "Pan", mode: "active" }),
    });
  });

  it("group ids stay in creation order", () => {
    const tools = createToolRegistry();
    expect(tools.createGroup("c").ok).toBe(true);
    expect(tools.createGroup("a").ok).toBe(true);
    expect(tools.createGroup("b").ok).toBe(true);
    expect(tools.groups()).toEqual(["c", "a", "b"]);
    expect(tools.destroyGroup("a")).toEqual({ ok: true, value: "a" });
    expect(tools.groups()).toEqual(["c", "b"]);
    expect(tools.group("a")).toEqual({ ok: false, refusal: "unknown-group" });
  });

  it("bindings that normalise to one key are refused", () => {
    const tools = registry();
    expect(tools.registerTool("g", tool("Pan", "Pan")).ok).toBe(true);
    expect(tools.setMode("g", "Pan", "active", [PRIMARY]).ok).toBe(true);
    const before = snapshot(tools);
    expect(
      tools.setMode("g", "Pan", "active", [
        { button: "primary", modifiers: ["shift", "alt"] },
        { button: "primary", modifiers: ["alt", "shift"] },
      ]),
    ).toEqual({ ok: false, refusal: "duplicate-binding" });
    expect(snapshot(tools)).toEqual(before);
  });

  it("two groups can own the same button", () => {
    const tools = createToolRegistry();
    expect(tools.createGroup("a").ok).toBe(true);
    expect(tools.createGroup("b").ok).toBe(true);
    expect(tools.registerTool("a", tool("Pan", "Pan")).ok).toBe(true);
    expect(tools.registerTool("b", tool("Zoom", "Zoom")).ok).toBe(true);
    expect(tools.setMode("a", "Pan", "active", [PRIMARY])).toEqual({
      ok: true,
      value: { demoted: [] },
    });
    expect(tools.setMode("b", "Zoom", "active", [PRIMARY])).toEqual({
      ok: true,
      value: { demoted: [] },
    });
    expect(tools.bindingOwner("a", PRIMARY)).toEqual({
      ok: true,
      value: expect.objectContaining({ tool: "Pan" }),
    });
    expect(tools.bindingOwner("b", PRIMARY)).toEqual({
      ok: true,
      value: expect.objectContaining({ tool: "Zoom" }),
    });
  });

  it("detach keeps the remaining viewport order", () => {
    const tools = registry();
    const first = viewportId(3);
    const second = viewportId(1);
    const third = viewportId(2);
    expect(tools.attachViewport("g", first).ok).toBe(true);
    expect(tools.attachViewport("g", second).ok).toBe(true);
    expect(tools.attachViewport("g", third).ok).toBe(true);
    expect(tools.detachViewport("g", second)).toEqual({ ok: true, value: second });
    const view = tools.group("g");
    expect(view.ok && view.value.viewports).toEqual([first, third]);
  });

  it("viewport limits are the safe integer range", () => {
    const tools = registry();
    expect(tools.attachViewport("g", viewportId(Number.MAX_SAFE_INTEGER))).toEqual({
      ok: true,
      value: viewportId(Number.MAX_SAFE_INTEGER),
    });
    expect(tools.attachViewport("g", viewportId(2 ** 53))).toEqual({
      ok: false,
      refusal: "bad-viewport",
    });
    const view = tools.group("g");
    expect(view.ok && view.value.viewports).toEqual([viewportId(Number.MAX_SAFE_INTEGER)]);
  });
});
