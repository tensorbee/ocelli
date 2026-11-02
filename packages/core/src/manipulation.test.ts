import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

import { describe, expect, it } from "vitest";

import {
  MANIPULATION_TOOLS,
  stackDrag,
  windowDrag,
  windowRegion,
} from "./manipulation.js";
import { createToolRegistry } from "./tools.js";

const SOURCE = readFileSync(
  join(dirname(fileURLToPath(import.meta.url)), "manipulation.ts"),
  "utf8",
);

describe("manipulation tools", () => {
  it("a window drag of 256 right and 256 up moves centre and width by 1", () => {
    // Hand-computed. 256 / 256 is 1. Up is a smaller Y, so width gains 1.
    const basis = { centre: 40, width: 400 };
    expect(windowDrag(basis, 0, 0, 256, -256)).toEqual({ centre: 41, width: 401 });
    expect(basis).toEqual({ centre: 40, width: 400 });
  });

  it("a non-positive width is refused and the basis stays", () => {
    // 256 down subtracts 1 from width 0.5, which is not greater than 0.
    const basis = { centre: 40, width: 0.5 };
    expect(windowDrag(basis, 0, 0, 256, 256)).toBeNull();
    expect(basis).toEqual({ centre: 40, width: 0.5 });
    const zero = { centre: 40, width: 1 };
    expect(windowDrag(zero, 0, 0, 0, 256)).toBeNull();
    expect(zero).toEqual({ centre: 40, width: 1 });
  });

  it("a region window is the mid-span, not the mean", () => {
    // Minimum 10, maximum 30. Centre (10 + 30) / 2 is 20. Width is 20.
    expect(windowRegion([10, 30, 20])).toEqual({ centre: 20, width: 20 });
  });

  it("an empty region and a non-finite sample are refused", () => {
    expect(windowRegion([])).toBeNull();
    expect(windowRegion([10, Number.NaN, 20])).toBeNull();
    expect(windowRegion([7, 7])).toBeNull();
    expect(windowDrag({ centre: Number.NaN, width: 400 }, 0, 0, 256, -256)).toBeNull();
    expect(windowDrag({ centre: 40, width: 400 }, -Number.MAX_VALUE, 0, Number.MAX_VALUE, 0)).toBeNull();
  });

  it("a stack drag truncates 256 pixels to one step", () => {
    // (0 - -256) / 256 is 1. Truncation toward zero leaves 1.
    // 255 / 256 truncates to 0, so a rounded step is refused.
    expect(stackDrag(0, -256)).toBe(1);
    expect(stackDrag(0, -255)).toBeNull();
  });

  it("a stack drag of no pixels returns no step", () => {
    expect(stackDrag(4, 4)).toBeNull();
    expect(stackDrag(Number.NaN, 0)).toBeNull();
    expect(stackDrag(0, 256)).toBe(-1);
    expect(stackDrag(Number.MAX_VALUE, -Number.MAX_VALUE)).toBeNull();
  });

  it("the five names bind to the existing command kinds", () => {
    const tools = createToolRegistry();
    expect(tools.createGroup("g")).toEqual({ ok: true, value: "g" });
    for (const binding of MANIPULATION_TOOLS) {
      expect(tools.registerTool("g", binding).ok).toBe(true);
    }
    const view = tools.group("g");
    expect(view.ok).toBe(true);
    if (!view.ok) {
      return;
    }
    expect(view.value.tools.map((row) => [row.tool, row.command])).toEqual([
      ["Pan", "Pan"],
      ["Zoom", "Zoom"],
      ["WindowLevel", "SetWindow"],
      ["WindowLevelRegion", "SetWindow"],
      ["StackScroll", "Scroll"],
    ]);
    expect(SOURCE.includes("panX")).toBe(false);
    expect(SOURCE.includes("Math.exp")).toBe(false);
  });
});
