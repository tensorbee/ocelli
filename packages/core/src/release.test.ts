import { describe, expect, it } from "vitest";

import { mayClaimStable, removalMinorsRemaining } from "./release.js";
import { VERSION } from "./index.js";

describe("mayClaimStable", () => {
  it("returns false while D-22 is open", () => {
    expect(
      mayClaimStable({ d22Open: true, matrixRows: 5, certifiedRows: 5 }),
    ).toBe(false);
  });

  it("returns false when no row is certified", () => {
    expect(
      mayClaimStable({ d22Open: false, matrixRows: 5, certifiedRows: 0 }),
    ).toBe(false);
  });

  it("returns false when the certified count is short", () => {
    expect(
      mayClaimStable({ d22Open: false, matrixRows: 5, certifiedRows: 4 }),
    ).toBe(false);
  });

  it("returns true only when D-22 is closed and every row is certified", () => {
    expect(
      mayClaimStable({ d22Open: false, matrixRows: 5, certifiedRows: 5 }),
    ).toBe(true);
  });

  it("returns false for a non-integer count", () => {
    expect(
      mayClaimStable({ d22Open: false, matrixRows: 1.5, certifiedRows: 1.5 }),
    ).toBe(false);
    expect(
      mayClaimStable({ d22Open: false, matrixRows: 1, certifiedRows: 1.5 }),
    ).toBe(false);
  });

  it("returns false for an empty matrix", () => {
    expect(
      mayClaimStable({ d22Open: false, matrixRows: 0, certifiedRows: 0 }),
    ).toBe(false);
  });
});

describe("removalMinorsRemaining", () => {
  it("returns null, null, 1 and 0 for the four channels", () => {
    expect(removalMinorsRemaining("0.x")).toBeNull();
    expect(removalMinorsRemaining("patch")).toBeNull();
    expect(removalMinorsRemaining("minor")).toBe(1);
    expect(removalMinorsRemaining("major")).toBe(0);
  });

  it("returns null for an unknown channel", () => {
    expect(removalMinorsRemaining("beta")).toBeNull();
    expect(removalMinorsRemaining("")).toBeNull();
  });
});

describe("workspace version", () => {
  it("stays 0.1.0", () => {
    expect(VERSION).toBe("0.1.0");
  });
});
