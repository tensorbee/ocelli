import { describe, expect, it } from "vitest";

import { ROTATION_TOOLS, orientationQuadrant, rotationDrag } from "./orientation.js";
import { createToolRegistry } from "./tools.js";

describe("rotation tools", () => {
  it("a drag of 256 right adds one radian", () => {
    // Hand-computed. 256 / 256 is 1. The basis gains that radian.
    expect(rotationDrag(0, 0, 256)).toBe(1);
  });

  it("a non-finite basis is refused", () => {
    expect(rotationDrag(Number.NaN, 0, 256)).toBeNull();
    expect(rotationDrag(0, 0, Number.NaN)).toBeNull();
    expect(rotationDrag(0, -Number.MAX_VALUE, Number.MAX_VALUE)).toBeNull();
    expect(orientationQuadrant(Number.NaN)).toBeNull();
  });

  it("one quarter turn is quadrant 1", () => {
    // Math.PI / 2 divided by the same quarter is 1.
    expect(orientationQuadrant(Math.PI / 2)).toBe(1);
  });

  it("a negative quarter turn wraps to quadrant 3", () => {
    // -Math.PI / 2 divided by Math.PI / 2 is -1. The wrap is 3.
    expect(orientationQuadrant(-Math.PI / 2)).toBe(3);
  });

  it("the three names bind to Rotate", () => {
    const tools = createToolRegistry();
    expect(tools.createGroup("g")).toEqual({ ok: true, value: "g" });
    for (const binding of ROTATION_TOOLS) {
      expect(tools.registerTool("g", binding).ok).toBe(true);
    }
    const view = tools.group("g");
    expect(view.ok).toBe(true);
    if (!view.ok) {
      return;
    }
    expect(view.value.tools.map((row) => [row.tool, row.command])).toEqual([
      ["TrackballRotate", "Rotate"],
      ["VolumeRotate", "Rotate"],
      ["PlanarRotate", "Rotate"],
    ]);
  });
});
