/**
 * The shell parser for the F-050 record.
 *
 * Magic, version and the error codes are the generated bindings. This file
 * does not restate a second table.
 */
import { describe, expect, it } from "vitest";

import { SNAPSHOT_ERROR as GENERATED_ERROR } from "./generated.js";
import {
  SNAPSHOT_ERROR,
  SNAPSHOT_MAGIC,
  SNAPSHOT_VERSION,
  parseSnapshot,
} from "./index.js";

const RECORD = {
  magic: "OCS1",
  version: 1,
  viewportId: 1,
  displayArea: {
    cssWidth: 800,
    cssHeight: 600,
    columns: 512,
    rows: 512,
    columnMillimetres: 1,
    rowMillimetres: 1,
    shiftColumn: 0,
    shiftRow: 0,
    fitted: true,
    anchorX: 400,
    anchorY: 300,
    fitScale: 1,
  },
  camera: {
    panX: 12.5,
    panY: -4,
    zoom: 2,
    radians: 0,
    flipHorizontal: false,
    flipVertical: false,
  },
  presentation: {
    modality: { kind: "rescale", slope: 1, intercept: -1024 },
    voi: {
      kind: "window",
      center: 40,
      width: 400,
      function: "linear",
      ymin: 0,
      ymax: 255,
    },
    photometric: "monochrome2",
    evidence: { kind: "shape", shape: "inverse" },
    colormap: "grey",
  },
};

function bytesOf(value: unknown): Uint8Array {
  return new TextEncoder().encode(JSON.stringify(value));
}

describe("parseSnapshot", () => {
  it("reads the generated magic, version and error codes", () => {
    expect(SNAPSHOT_MAGIC).toBe("OCS1");
    expect(SNAPSHOT_VERSION).toBe(1);
    expect(SNAPSHOT_ERROR).toBe(GENERATED_ERROR);
    const codes = Object.values(SNAPSHOT_ERROR);
    expect(Math.min(...codes)).toBeGreaterThan(0);
    expect(new Set(codes).size).toBe(codes.length);
  });

  it("parses the documented record", () => {
    const parsed = parseSnapshot(bytesOf(RECORD));
    expect(parsed.ok).toBe(true);
    if (!parsed.ok) {
      return;
    }
    expect(parsed.snapshot.magic).toBe(SNAPSHOT_MAGIC);
    expect(parsed.snapshot.version).toBe(SNAPSHOT_VERSION);
    expect(parsed.snapshot.camera.panX).toBe(12.5);
    expect(parsed.snapshot.camera.zoom).toBe(2);
    expect(parsed.snapshot.displayArea.anchorX).toBe(400);
    expect(parsed.snapshot.displayArea.fitScale).toBe(1);
    expect(parsed.snapshot.presentation.evidence).toEqual({
      kind: "shape",
      shape: "inverse",
    });
    expect(parsed.snapshot.presentation.modality).toEqual({
      kind: "rescale",
      slope: 1,
      intercept: -1024,
    });
    expect(JSON.stringify(parsed.snapshot)).not.toContain("matrix");
  });

  it("refuses an unknown version", () => {
    const parsed = parseSnapshot(bytesOf({ ...RECORD, version: 2 }));
    expect(parsed).toEqual({ ok: false, code: SNAPSHOT_ERROR.UnknownVersion });
  });

  it("refuses a bad magic", () => {
    const parsed = parseSnapshot(bytesOf({ ...RECORD, magic: "NOPE" }));
    expect(parsed).toEqual({ ok: false, code: SNAPSHOT_ERROR.BadMagic });
  });

  it("refuses malformed JSON", () => {
    const parsed = parseSnapshot(new TextEncoder().encode("{"));
    expect(parsed).toEqual({ ok: false, code: SNAPSHOT_ERROR.Json });
  });

  it("refuses an unknown field", () => {
    const parsed = parseSnapshot(bytesOf({ ...RECORD, matrix: [1] }));
    expect(parsed).toEqual({ ok: false, code: SNAPSHOT_ERROR.Json });
  });

  it("refuses an out-of-domain photometric name", () => {
    const parsed = parseSnapshot(
      bytesOf({
        ...RECORD,
        presentation: { ...RECORD.presentation, photometric: "cmyk" },
      }),
    );
    expect(parsed).toEqual({ ok: false, code: SNAPSHOT_ERROR.BadEnum });
  });
});
