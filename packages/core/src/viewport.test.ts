import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

import { describe, expect, it } from "vitest";

import {
  CAMERA_COMPONENTS,
  COLORMAPS,
  INVERT,
  STACK_COMMANDS,
  TIER_AVAILABILITY,
  TIER_C_DRAWING,
  VIEWPORT_STATE_FIELDS,
  VOI_FUNCTIONS,
  WINDOW_PRESETS,
  viewportId,
} from "./viewport.js";
import type { Camera, ViewportId } from "./viewport.js";

const SOURCE = readFileSync(
  join(dirname(fileURLToPath(import.meta.url)), "viewport.ts"),
  "utf8",
);

type NotAssignable<A, B> = [A] extends [B] ? false : true;

describe("stack viewport API", () => {
  it("publishes the eleven commands in plan order", () => {
    expect([...STACK_COMMANDS]).toEqual([
      "Fit",
      "Pan",
      "Zoom",
      "Rotate",
      "FlipHorizontal",
      "FlipVertical",
      "SetWindow",
      "ApplyPreset",
      "SetInvert",
      "SetColormap",
      "Scroll",
    ]);
  });

  it("documents section 17.4's readback fields in struct order", () => {
    const width: Camera["length"] = CAMERA_COMPONENTS;
    expect(width).toBe(16);
    expect([...VIEWPORT_STATE_FIELDS]).toEqual([
      "camera",
      "voiCenter",
      "voiWidth",
      "sliceIndex",
      "numSlices",
      "flags",
    ]);
  });

  it("records tier C drawing as unavailable", () => {
    expect([...TIER_AVAILABILITY]).toEqual([
      "available",
      "degraded",
      "unavailable",
    ]);
    expect(TIER_C_DRAWING).toBe("unavailable");
  });

  it("publishes the colormap, preset, function and invert names", () => {
    expect([...COLORMAPS]).toEqual(["grey", "stops"]);
    expect([...WINDOW_PRESETS]).toEqual(["soft_tissue", "lung", "bone"]);
    expect([...VOI_FUNCTIONS]).toEqual(["linear", "linear_exact", "sigmoid"]);
    expect([...INVERT]).toEqual(["identity", "inverse"]);
  });

  it("keeps a viewport id distinct from a slice index", () => {
    const structural: NotAssignable<{ readonly id: number }, ViewportId> = true;
    const slice: NotAssignable<number, ViewportId> = true;
    expect(structural).toBe(true);
    expect(slice).toBe(true);
    expect(viewportId(4)).toBe(4);
  });

  it("does not touch wasm memory", () => {
    expect(SOURCE.includes("wasm.memory")).toBe(false);
    expect(SOURCE.includes("wasm-bindgen")).toBe(false);
  });
});
