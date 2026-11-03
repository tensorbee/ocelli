import { describe, expect, it } from "vitest";

import { shadowDivergence } from "./shadow.js";

function bytes(values: readonly number[]): Uint8Array {
  return Uint8Array.from(values);
}

describe("F-109 shadow divergence", () => {
  it("AC1 counts a sample above the limit and ignores one at the limit", () => {
    const ocelli = bytes([0]);
    const other = bytes([2]);
    expect(shadowDivergence(ocelli, other, 1)).toBe(1);
    expect(Array.from(ocelli)).toEqual([0]);
    expect(Array.from(other)).toEqual([2]);
    expect(shadowDivergence(bytes([0]), bytes([1]), 1)).toBe(0);
  });

  it("AC2 does not count equal bytes", () => {
    expect(shadowDivergence(bytes([0, 5]), bytes([0, 5]), 0)).toBe(0);
  });

  it("AC3 refuses unequal lengths and a limit outside 0 to 255", () => {
    const ocelli = bytes([4]);
    const other = bytes([4, 9]);
    expect(shadowDivergence(ocelli, other, 0)).toBeNull();
    expect(shadowDivergence(bytes([0]), bytes([0]), 1.5)).toBeNull();
    expect(shadowDivergence(bytes([0]), bytes([0]), 256)).toBeNull();
    expect(shadowDivergence(bytes([0]), bytes([0]), -1)).toBeNull();
    expect(Array.from(ocelli)).toEqual([4]);
    expect(Array.from(other)).toEqual([4, 9]);
  });

  it("AC4 does not write either buffer", () => {
    const first = bytes([0, 10]);
    const same = bytes([0, 10]);
    expect(shadowDivergence(first, same, 0)).toBe(0);
    const changed = bytes([0, 11]);
    expect(shadowDivergence(first, changed, 0)).toBe(1);
    expect(Array.from(first)).toEqual([0, 10]);
    expect(Array.from(same)).toEqual([0, 10]);
    expect(Array.from(changed)).toEqual([0, 11]);
  });
});
