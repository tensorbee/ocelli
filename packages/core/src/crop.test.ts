import { describe, expect, it } from "vitest";

import { cropDrag, cropPlanes } from "./crop.js";

describe("volume crop planes", () => {
  it("returns a copy of one finite plane", () => {
    const plane = { origin: [0, 0, 0] as const, normal: [0, 0, 1] as const };
    const copied = cropPlanes([plane]);
    expect(copied).toEqual([plane]);
    expect(copied).not.toBe([plane]);
    expect(copied?.[0]).not.toBe(plane);
  });

  it("returns an empty copy for an empty list", () => {
    const empty: { origin: readonly [number, number, number]; normal: readonly [number, number, number] }[] = [];
    const copied = cropPlanes(empty);
    expect(copied).toEqual([]);
    expect(copied).not.toBe(empty);
  });

  it("refuses seven planes, a zero normal, and a non-finite component", () => {
    const plane = { origin: [0, 0, 0] as const, normal: [0, 0, 1] as const };
    expect(cropPlanes([plane, plane, plane, plane, plane, plane])).toHaveLength(6);
    expect(cropPlanes([plane, plane, plane, plane, plane, plane, plane])).toBeNull();
    expect(cropPlanes([{ origin: [0, 0, 0], normal: [0, 0, 0] }])).toBeNull();
    expect(cropPlanes([{ origin: [Number.NaN, 0, 0], normal: [0, 0, 1] }])).toBeNull();
    expect(
      cropPlanes([{ origin: [0, 0, 0], normal: [Number.MAX_VALUE, 0, 0] }]),
    ).toBeNull();
  });

  it("moves the origin by the normal times a 256 pixel drag", () => {
    // 256 / 256 is 1. z becomes 0 + 1 * 1.
    expect(cropDrag([0, 0, 0], [0, 0, 1], 0, 256)).toEqual([0, 0, 1]);
    // The normal is not divided by its length. z becomes 0 + 2 * 1.
    expect(cropDrag([0, 0, 0], [0, 0, 2], 0, 256)).toEqual([0, 0, 2]);
  });

  it("a drag to the left moves the origin the other way", () => {
    expect(cropDrag([0, 0, 0], [0, 0, 1], 0, -256)).toEqual([0, 0, -1]);
  });

  it("refuses a zero normal and a non-finite result", () => {
    expect(cropDrag([0, 0, 0], [0, 0, 0], 0, 256)).toBeNull();
    expect(cropDrag([0, 0, 0], [Number.MAX_VALUE, 0, 0], 0, 256)).toEqual([
      Number.MAX_VALUE,
      0,
      0,
    ]);
    expect(cropDrag([Number.NaN, 0, 0], [0, 0, 1], 0, 256)).toBeNull();
    expect(
      cropDrag([Number.MAX_VALUE, 0, 0], [Number.MAX_VALUE, 0, 0], 0, 256),
    ).toBeNull();
  });
});
