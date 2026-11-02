import { describe, expect, it } from "vitest";

import {
  overlayGrid,
  segmentationIntersection,
  sliceIntersection,
} from "./intersection.js";

describe("slice and grid overlays", () => {
  it("clips a horizontal segment to the viewport edges", () => {
    // Hand-computed. Entering t is 1/7 and leaving t is 6/7.
    expect(sliceIntersection(-2, 4, 12, 4, 10, 8)).toEqual({
      x0: 0,
      y0: 4,
      x1: 10,
      y1: 4,
    });
  });

  it("leaves a segment that is already inside", () => {
    expect(sliceIntersection(1, 1, 2, 2, 10, 8)).toEqual({
      x0: 1,
      y0: 1,
      x1: 2,
      y1: 2,
    });
    expect(sliceIntersection(0, 0, 10, 8, 10, 8)).toEqual({
      x0: 0,
      y0: 0,
      x1: 10,
      y1: 8,
    });
  });

  it("refuses a miss, a zero delta, and a bad size", () => {
    expect(sliceIntersection(-5, -5, -1, -1, 10, 8)).toBeNull();
    expect(sliceIntersection(1, 1, 1, 1, 10, 8)).toBeNull();
    expect(sliceIntersection(0, 0, 1, 1, 0, 8)).toBeNull();
    expect(sliceIntersection(Number.NaN, 0, 1, 1, 10, 8)).toBeNull();
  });

  it("drops a segmentation segment that misses and keeps the one inside", () => {
    const inside = { x0: 1, y0: 1, x1: 2, y1: 2 };
    const miss = { x0: -5, y0: -5, x1: -1, y1: -1 };
    expect(segmentationIntersection([inside, miss], 10, 8)).toEqual([inside]);
  });

  it("returns an empty list for no segments and refuses a non-finite one", () => {
    expect(segmentationIntersection([], 10, 8)).toEqual([]);
    expect(
      segmentationIntersection([{ x0: Number.NaN, y0: 1, x1: 2, y1: 2 }], 10, 8),
    ).toBeNull();
  });

  it("places the interior midlines of a two by two grid", () => {
    // Two columns place x 5. Two rows place y 4.
    expect(overlayGrid(2, 2, 10, 8)).toEqual([
      { x0: 5, y0: 0, x1: 5, y1: 8 },
      { x0: 0, y0: 4, x1: 10, y1: 4 },
    ]);
  });

  it("returns no lines for one cell", () => {
    expect(overlayGrid(1, 1, 10, 8)).toEqual([]);
  });

  it("refuses a fractional column count", () => {
    expect(overlayGrid(2.5, 2, 10, 8)).toBeNull();
    expect(overlayGrid(0, 2, 10, 8)).toBeNull();
    expect(overlayGrid(2, 2, 0, 8)).toBeNull();
  });
});
