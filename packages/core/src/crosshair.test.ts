import { describe, expect, it } from "vitest";

import { crosshairLines, referenceCursor, referenceLines } from "./crosshair.js";

const HORIZONTAL = { x0: 0, y0: 3, x1: 10, y1: 3 };
const VERTICAL = { x0: 4, y0: 0, x1: 4, y1: 8 };

describe("crosshairs", () => {
  it("draws the horizontal span at y and the vertical span at x", () => {
    // Hand-computed. y stays 3 across the width. x stays 4 across the height.
    expect(crosshairLines(4, 3, 10, 8)).toEqual([HORIZONTAL, VERTICAL]);
  });

  it("keeps an edge point, both segments and the cursor", () => {
    expect(crosshairLines(0, 8, 10, 8)).toEqual([
      { x0: 0, y0: 8, x1: 10, y1: 8 },
      { x0: 0, y0: 0, x1: 0, y1: 8 },
    ]);
    expect(referenceCursor(0, 8, 10, 8)).toEqual({ x: 0, y: 8 });
  });

  it("refuses a point outside and a non-positive side", () => {
    expect(crosshairLines(-1, 3, 10, 8)).toBeNull();
    expect(referenceCursor(-1, 3, 10, 8)).toBeNull();
    expect(crosshairLines(4, 3, 0, 8)).toBeNull();
    expect(referenceCursor(4, 3, 0, 8)).toBeNull();
    expect(crosshairLines(Number.NaN, 3, 10, 8)).toBeNull();
    expect(crosshairLines(4, 3, 10, 0)).toBeNull();
    expect(crosshairLines(11, 3, 10, 8)).toBeNull();
  });

  it("reference lines are the same two segments", () => {
    expect(referenceLines(4, 3, 10, 8)).toEqual([HORIZONTAL, VERTICAL]);
    expect(referenceLines).toBe(crosshairLines);
  });
});
