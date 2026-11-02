import { describe, expect, it } from "vitest";

import { eraseNearest, mipIndex, sculptVertex } from "./sculpt.js";

describe("sculpt, eraser and MIP index", () => {
  it("moves one vertex by the pixel delta", () => {
    const points = [
      { x: 0, y: 0 },
      { x: 1, y: 0 },
      { x: 0, y: 1 },
    ];
    // Vertex 1 starts at (1, 0). Adding (2, 3) gives (3, 3).
    expect(sculptVertex(points, 1, 0, 0, 2, 3)).toEqual([
      { x: 0, y: 0 },
      { x: 3, y: 3 },
      { x: 0, y: 1 },
    ]);
    expect(points[1]).toEqual({ x: 1, y: 0 });
  });

  it("refuses a short list and an index outside it", () => {
    expect(
      sculptVertex(
        [
          { x: 0, y: 0 },
          { x: 1, y: 0 },
        ],
        0,
        0,
        0,
        1,
        1,
      ),
    ).toBeNull();
    expect(
      sculptVertex(
        [
          { x: 0, y: 0 },
          { x: 1, y: 0 },
          { x: 0, y: 1 },
        ],
        3,
        0,
        0,
        1,
        1,
      ),
    ).toBeNull();
    expect(
      sculptVertex(
        [
          { x: Number.NaN, y: 0 },
          { x: 1, y: 0 },
          { x: 0, y: 1 },
        ],
        0,
        0,
        0,
        1,
        1,
      ),
    ).toBeNull();
  });

  it("names the nearer mark inside the radius", () => {
    const marks = [
      { id: "a", x: 0, y: 0 },
      { id: "b", x: 3, y: 4 },
    ];
    // Distance to a is 1. Distance to b is greater than 2.
    expect(eraseNearest(marks, 0, 1, 2)).toBe("a");
    expect(eraseNearest(marks, 3, 4, 1)).toBe("b");
  });

  it("keeps the earlier mark when the distances match", () => {
    expect(
      eraseNearest(
        [
          { id: "a", x: 0, y: 0 },
          { id: "b", x: 0, y: 0 },
        ],
        0,
        0,
        1,
      ),
    ).toBe("a");
    expect(eraseNearest([], 0, 0, 1)).toBeNull();
    expect(eraseNearest([{ id: "a", x: 0, y: 0 }], 5, 0, 1)).toBeNull();
    expect(eraseNearest([{ id: "a", x: 0, y: 0 }], 0, 0, 0)).toBeNull();
    expect(eraseNearest([{ id: "", x: 0, y: 0 }], 0, 0, 1)).toBeNull();
  });

  it("returns the earlier index of the maximum sample", () => {
    // The maximum is 5 at index 1 and index 2. The earlier index is 1.
    expect(mipIndex([1, 5, 5, 2])).toBe(1);
    expect(mipIndex([1, 3, 2])).toBe(1);
  });

  it("refuses an empty sample list and a non-finite sample", () => {
    expect(mipIndex([])).toBeNull();
    expect(mipIndex([1, Number.NaN])).toBeNull();
  });
});
