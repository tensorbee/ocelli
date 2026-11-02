import { describe, expect, it } from "vitest";

import { magnifyCircle, orientationMarker, scaleLength } from "./magnify.js";

describe("magnify and overlays", () => {
  it("a plain magnify stores scale 1", () => {
    expect(magnifyCircle(5, 6, 4, null)).toEqual({ x: 5, y: 6, radius: 4, scale: 1 });
  });

  it("an advanced scale above 1 is stored", () => {
    expect(magnifyCircle(5, 6, 4, 2)).toEqual({ x: 5, y: 6, radius: 4, scale: 2 });
  });

  it("scale 1 and radius 0 are refused", () => {
    expect(magnifyCircle(5, 6, 4, 1)).toBeNull();
    expect(magnifyCircle(5, 6, 0, null)).toBeNull();
    expect(magnifyCircle(Number.NaN, 6, 4, null)).toBeNull();
    expect(magnifyCircle(5, 6, 4, Number.POSITIVE_INFINITY)).toBeNull();
  });

  it("a scale overlay is the product of the caller factors", () => {
    // Hand-computed. 10 mm times 2 pixels per mm is 20 pixels.
    expect(scaleLength(10, 2)).toBe(20);
    expect(scaleLength(10, 0)).toBeNull();
    expect(scaleLength(Number.NaN, 2)).toBeNull();
    expect(scaleLength(Number.MAX_VALUE, Number.MAX_VALUE)).toBeNull();
  });

  it("quadrant 0 is unchanged and quadrant 1 moves the first letter to the end", () => {
    // One step. A B C D becomes B C D A.
    expect(orientationMarker(["A", "B", "C", "D"], 0)).toEqual(["A", "B", "C", "D"]);
    expect(orientationMarker(["A", "B", "C", "D"], 1)).toEqual(["B", "C", "D", "A"]);
  });

  it("quadrant 4 is refused and does not wrap", () => {
    expect(orientationMarker(["A", "B", "C", "D"], 4)).toBeNull();
    expect(orientationMarker(["A", "B", "C", "D"], 1.5)).toBeNull();
    expect(
      orientationMarker(
        ["A", "B", "C", "D", "E"] as unknown as readonly [string, string, string, string],
        0,
      ),
    ).toBeNull();
    expect(
      orientationMarker(
        ["A", "B", "C", 1] as unknown as readonly [string, string, string, string],
        0,
      ),
    ).toBeNull();
  });
});
