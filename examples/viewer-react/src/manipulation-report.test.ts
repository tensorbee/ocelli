import { ERROR_CODE, describeError, packCommands } from "@ocelli/core";
import { describe, expect, it, vi } from "vitest";

import { PAGE_MANIPULATION, manipulationReport } from "./manipulation-report.js";

describe("manipulation report", () => {
  it("names the preset and counts that command", () => {
    const lung = manipulationReport(PAGE_MANIPULATION);
    const bone = manipulationReport({ ...PAGE_MANIPULATION, preset: "bone" });
    expect(lung.preset).toBe("lung");
    expect(bone.preset).toBe("bone");
    expect(lung.presetPacked).toEqual(packCommands(1, [{ kind: "ApplyPreset", preset: "lung" }]));
    expect(bone.presetPacked).toEqual(packCommands(1, [{ kind: "ApplyPreset", preset: "bone" }]));
    expect(lung.presetBytes).toBe(lung.presetPacked.byteLength);
    expect(lung.presetBytes).toBeGreaterThan(0);
  });

  it("follows the window drag and the region span", () => {
    const report = manipulationReport(PAGE_MANIPULATION);
    expect(report.centre).toBe(41);
    expect(report.width).toBe(401);
    expect(report.regionCentre).toBe(5);
    expect(report.regionWidth).toBe(10);
    const dragged = manipulationReport({ ...PAGE_MANIPULATION, lastX: 512, lastY: -512 });
    expect(dragged.centre).toBe(42);
    expect(dragged.width).toBe(402);
    const wide = manipulationReport({ ...PAGE_MANIPULATION, samples: [0, 20] });
    expect(wide.regionCentre).toBe(10);
    expect(wide.regionWidth).toBe(20);
    expect(manipulationReport({ ...PAGE_MANIPULATION, samples: [4, 4] }).regionCentre).toBeNull();
    expect(manipulationReport({ ...PAGE_MANIPULATION, samples: [4, 4] }).regionWidth).toBeNull();
  });

  it("turns 512 px into 2 radians and quadrant 1", () => {
    const report = manipulationReport(PAGE_MANIPULATION);
    expect(report.radians).toBe(2);
    expect(report.quadrant).toBe(1);
    expect(manipulationReport({ ...PAGE_MANIPULATION, rotateLastX: 256 }).radians).toBe(1);
    const still = manipulationReport({ ...PAGE_MANIPULATION, rotateLastX: 0 });
    expect(still.radians).toBe(0);
    expect(still.quadrant).toBe(0);
  });

  it("moves the marker, stores a plain magnify scale, and multiplies the scale bar", () => {
    const report = manipulationReport(PAGE_MANIPULATION);
    expect(report.marker).toBe("A L P R");
    expect(manipulationReport({ ...PAGE_MANIPULATION, markerQuadrant: 2 }).marker).toBe("L P R A");
    expect(report.magnifyScale).toBe(1);
    expect(report.scalePixels).toBe(20);
    expect(manipulationReport({ ...PAGE_MANIPULATION, millimetres: 5 }).scalePixels).toBe(10);
    expect(manipulationReport({ ...PAGE_MANIPULATION, magnifyScale: 1 }).magnifyScale).toBeNull();
  });

  it("counts one crop plane and uses describeError for the caller code", () => {
    const report = manipulationReport(PAGE_MANIPULATION);
    expect(report.cropCount).toBe(1);
    expect(manipulationReport({ ...PAGE_MANIPULATION, cropPlaneCount: 2 }).cropCount).toBe(2);
    expect(manipulationReport({ ...PAGE_MANIPULATION, cropPlaneCount: 7 }).cropCount).toBeNull();
    expect(report.colormap).toBe("stops");
    expect(report.colormapPacked).toEqual(
      packCommands(1, [{ kind: "SetColormap", colormap: "stops" }]),
    );
    expect(report.warning).toBe(describeError(ERROR_CODE.Unavailable));
    expect(manipulationReport({ ...PAGE_MANIPULATION, code: ERROR_CODE.Workgroup }).warning).toBe(
      describeError(ERROR_CODE.Workgroup),
    );
    expect(report.warning).not.toBe(
      manipulationReport({ ...PAGE_MANIPULATION, code: ERROR_CODE.Workgroup }).warning,
    );
    expect(report.flipPacked).toEqual(
      packCommands(1, [
        { kind: "FlipHorizontal", enabled: true },
        { kind: "FlipVertical", enabled: false },
      ]),
    );
    expect(report.flipBytes).toBe(report.flipPacked.byteLength);
  });

  it("hands a huge crop count to the core as seven planes", async () => {
    const core = await import("@ocelli/core");
    const real = core.cropPlanes;
    const spy = vi.spyOn(core, "cropPlanes").mockImplementation((planes) => real(planes));
    try {
      const report = manipulationReport({ ...PAGE_MANIPULATION, cropPlaneCount: 100 });
      expect(spy).toHaveBeenCalled();
      const passed = spy.mock.calls[0];
      if (passed === undefined) {
        throw new Error("cropPlanes was not called");
      }
      expect(passed[0].length).toBe(7);
      expect(report.cropCount).toBeNull();
    } finally {
      spy.mockRestore();
    }
  });
});
