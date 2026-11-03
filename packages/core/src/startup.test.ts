import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

import { describe, expect, it } from "vitest";

import {
  startupArtifact,
  summariseColdStart,
  summariseFirstFrame,
} from "./startup.js";
import type { ColdPhases } from "./startup.js";

const SOURCE = readFileSync(
  join(dirname(fileURLToPath(import.meta.url)), "startup.ts"),
  "utf8",
);

const RELEASE: ColdPhases = {
  glue: 1,
  fetch: 2,
  compile: 4,
  instantiate: 8,
  firstCall: 16,
};

describe("startupArtifact", () => {
  it("names the render module", () => {
    expect(startupArtifact("render")).toBe("pkg/ocelli_wasm_bg.wasm");
  });

  it("names the decode module", () => {
    expect(startupArtifact("decode")).toBe("pkg-decode/ocelli_wasm_bg.wasm");
  });

  it("does not send render at the decode path", () => {
    expect(startupArtifact("render")).not.toContain("pkg-decode");
  });

  it("does not send decode at the render path", () => {
    const path = startupArtifact("decode");
    expect(path).not.toContain("pkg/ocelli_wasm_bg.wasm");
  });

  it("returns null for an unknown role", () => {
    expect(startupArtifact("native")).toBeNull();
    expect(startupArtifact("")).toBeNull();
  });
});

describe("summariseColdStart", () => {
  it("adds the five release phases", () => {
    expect(summariseColdStart("release", RELEASE)).toBe(31);
  });

  it("returns null for a dev profile", () => {
    expect(summariseColdStart("dev", RELEASE)).toBeNull();
  });

  it("returns null for a negative phase", () => {
    expect(
      summariseColdStart("release", { ...RELEASE, fetch: -0.01 }),
    ).toBeNull();
  });

  it("returns null for a non-finite phase", () => {
    expect(
      summariseColdStart("release", { ...RELEASE, compile: Number.NaN }),
    ).toBeNull();
  });

  it("does not read the baseline file", () => {
    expect(SOURCE.includes("bench-baseline")).toBe(false);
    expect(SOURCE.includes("2.3")).toBe(false);
  });
});

describe("summariseFirstFrame", () => {
  it("returns a release interval unchanged", () => {
    expect(summariseFirstFrame("release", 4.5)).toBe(4.5);
  });

  it("returns zero for a release interval of zero", () => {
    expect(summariseFirstFrame("release", 0)).toBe(0);
  });

  it("returns null for a dev profile", () => {
    expect(summariseFirstFrame("dev", 4.5)).toBeNull();
  });

  it("returns null for a non-finite or negative interval", () => {
    expect(summariseFirstFrame("release", Number.POSITIVE_INFINITY)).toBeNull();
    expect(summariseFirstFrame("release", -1)).toBeNull();
  });
});
