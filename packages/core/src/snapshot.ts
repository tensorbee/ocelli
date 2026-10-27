/**
 * Versioned camera, display-area and presentation record.
 *
 * The bytes are UTF-8 JSON written by `ocelli-viewport`. This module parses
 * them and forwards them. It does not rebuild a camera matrix and it does not
 * view wasm linear memory. Magic, version and error codes come from
 * `generated.ts`.
 */

import { SNAPSHOT_ERROR, SNAPSHOT_MAGIC, SNAPSHOT_VERSION } from "./generated.js";

export { SNAPSHOT_ERROR, SNAPSHOT_MAGIC, SNAPSHOT_VERSION };

/** Wasm exports that copy the record out and accept it back. */
export interface SessionSnapshotExports {
  snapshot(): Uint8Array;
  snapshot_error(): number;
  restore_snapshot(bytes: Uint8Array): number;
}

/** The record the Rust writer produces. Field names are the JSON names. */
export interface ViewportSnapshot {
  readonly magic: typeof SNAPSHOT_MAGIC;
  readonly version: typeof SNAPSHOT_VERSION;
  readonly viewportId: number;
  readonly displayArea: DisplayAreaSnapshot;
  readonly camera: CameraSnapshot;
  readonly presentation: PresentationSnapshot;
}

/** CSS box, grid, spacing, index shift and letterbox. */
export interface DisplayAreaSnapshot {
  readonly cssWidth: number;
  readonly cssHeight: number;
  readonly columns: number;
  readonly rows: number;
  readonly columnMillimetres: number;
  readonly rowMillimetres: number;
  readonly shiftColumn: number;
  readonly shiftRow: number;
  readonly fitted: boolean;
  readonly anchorX: number;
  readonly anchorY: number;
  readonly fitScale: number;
}

/** Camera inputs. The inverse matrix is not one of them. */
export interface CameraSnapshot {
  readonly panX: number;
  readonly panY: number;
  readonly zoom: number;
  readonly radians: number;
  readonly flipHorizontal: boolean;
  readonly flipVertical: boolean;
}

/** One selected modality stage. A sequence does not also carry rescale. */
export type ModalitySnapshot =
  | { readonly kind: "rescale"; readonly slope: number; readonly intercept: number }
  | {
      readonly kind: "lut";
      readonly entryCount: number;
      readonly firstMappedInput: number;
      readonly bitsPerEntry: number;
      readonly values: readonly number[];
    };

/** One selected VOI stage. A sequence does not also carry a window. */
export type VoiSnapshot =
  | {
      readonly kind: "window";
      readonly center: number;
      readonly width: number;
      readonly function: "linear" | "linear_exact" | "sigmoid";
      readonly ymin: number;
      readonly ymax: number;
    }
  | {
      readonly kind: "lut";
      readonly entryCount: number;
      readonly firstMappedInput: number;
      readonly bitsPerEntry: number;
      readonly values: readonly number[];
    };

/** Presentation LUT evidence. Absent is not an explicit shape. */
export type EvidenceSnapshot =
  | { readonly kind: "absent" }
  | { readonly kind: "shape"; readonly shape: "identity" | "inverse" }
  | { readonly kind: "sequence" };

/** Photometric names the public API already publishes. */
export type PhotometricSnapshot =
  | "monochrome1"
  | "monochrome2"
  | "palette_color"
  | "rgb"
  | "ybr_full"
  | "ybr_full_422"
  | "ybr_partial_422"
  | "ybr_ict"
  | "ybr_rct";

/** The presentation fields the record stores. */
export interface PresentationSnapshot {
  readonly modality: ModalitySnapshot;
  readonly voi: VoiSnapshot;
  readonly photometric: PhotometricSnapshot;
  readonly evidence: EvidenceSnapshot;
  readonly colormap: "grey" | "stops";
}

/** A parsed record, or the code that names the refusal. */
export type ParsedSnapshot =
  | { readonly ok: true; readonly snapshot: ViewportSnapshot }
  | { readonly ok: false; readonly code: number };

const RECORD_KEYS = [
  "magic",
  "version",
  "viewportId",
  "displayArea",
  "camera",
  "presentation",
] as const;

const DISPLAY_KEYS = [
  "cssWidth",
  "cssHeight",
  "columns",
  "rows",
  "columnMillimetres",
  "rowMillimetres",
  "shiftColumn",
  "shiftRow",
  "fitted",
  "anchorX",
  "anchorY",
  "fitScale",
] as const;

const CAMERA_KEYS = [
  "panX",
  "panY",
  "zoom",
  "radians",
  "flipHorizontal",
  "flipVertical",
] as const;

const PRESENTATION_KEYS = ["modality", "voi", "photometric", "evidence", "colormap"] as const;

const PHOTOMETRIC = new Set<string>([
  "monochrome1",
  "monochrome2",
  "palette_color",
  "rgb",
  "ybr_full",
  "ybr_full_422",
  "ybr_partial_422",
  "ybr_ict",
  "ybr_rct",
]);

const FUNCTIONS = new Set<string>(["linear", "linear_exact", "sigmoid"]);

/**
 * Copy the record out of the session.
 *
 * An empty array is a refusal. `session.snapshot_error()` names it.
 */
export function sessionSnapshot(session: SessionSnapshotExports): Uint8Array {
  return session.snapshot();
}

/** The code from the last `snapshot` call. Zero after a success. */
export function sessionSnapshotError(session: SessionSnapshotExports): number {
  return session.snapshot_error();
}

/**
 * Restore `bytes`. Zero is success. Any other value is `SNAPSHOT_ERROR`.
 *
 * A nonzero code leaves the session's camera and presentation unchanged.
 */
export function restoreSessionSnapshot(
  session: SessionSnapshotExports,
  bytes: Uint8Array,
): number {
  return session.restore_snapshot(bytes);
}

/**
 * Parse one record.
 *
 * Unknown version, bad magic, malformed JSON and an unknown field are refused
 * before a caller asks the core to restore the bytes. The core refuses them
 * again.
 */
export function parseSnapshot(bytes: Uint8Array): ParsedSnapshot {
  let text: string;
  try {
    text = new TextDecoder("utf-8", { fatal: true }).decode(bytes);
  } catch {
    return refused(SNAPSHOT_ERROR.Json);
  }
  let parsed: unknown;
  try {
    parsed = JSON.parse(text);
  } catch {
    return refused(SNAPSHOT_ERROR.Json);
  }
  if (!isRecord(parsed) || extra(parsed, RECORD_KEYS)) {
    return refused(SNAPSHOT_ERROR.Json);
  }
  if (typeof parsed.magic !== "string") {
    return refused(SNAPSHOT_ERROR.Json);
  }
  if (parsed.magic !== SNAPSHOT_MAGIC) {
    return refused(SNAPSHOT_ERROR.BadMagic);
  }
  if (typeof parsed.version !== "number" || !Number.isInteger(parsed.version)) {
    return refused(SNAPSHOT_ERROR.Json);
  }
  if (parsed.version !== SNAPSHOT_VERSION) {
    return refused(SNAPSHOT_ERROR.UnknownVersion);
  }
  const displayArea = readDisplay(parsed.displayArea);
  const camera = readCamera(parsed.camera);
  const presentation = readPresentation(parsed.presentation);
  if (displayArea === "json" || camera === "json" || presentation === "json") {
    return refused(SNAPSHOT_ERROR.Json);
  }
  if (presentation === "enum") {
    return refused(SNAPSHOT_ERROR.BadEnum);
  }
  if (displayArea === "nonfinite" || camera === "nonfinite" || presentation === "nonfinite") {
    return refused(SNAPSHOT_ERROR.NonFinite);
  }
  if (typeof parsed.viewportId !== "number" || !Number.isInteger(parsed.viewportId)) {
    return refused(SNAPSHOT_ERROR.Json);
  }
  return {
    ok: true,
    snapshot: {
      magic: SNAPSHOT_MAGIC,
      version: SNAPSHOT_VERSION,
      viewportId: parsed.viewportId,
      displayArea,
      camera,
      presentation,
    },
  };
}

function refused(code: number): ParsedSnapshot {
  return { ok: false, code };
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

function extra(value: Record<string, unknown>, allowed: readonly string[]): boolean {
  return Object.keys(value).some((key) => !allowed.includes(key));
}

function readDisplay(value: unknown): DisplayAreaSnapshot | "json" | "nonfinite" {
  if (!isRecord(value) || extra(value, DISPLAY_KEYS)) {
    return "json";
  }
  const numbers = [
    value.cssWidth,
    value.cssHeight,
    value.columnMillimetres,
    value.rowMillimetres,
    value.shiftColumn,
    value.shiftRow,
    value.anchorX,
    value.anchorY,
    value.fitScale,
  ];
  if (numbers.some((item) => typeof item !== "number")) {
    return "json";
  }
  if (numbers.some((item) => typeof item === "number" && !Number.isFinite(item))) {
    return "nonfinite";
  }
  if (
    typeof value.columns !== "number" ||
    typeof value.rows !== "number" ||
    !Number.isInteger(value.columns) ||
    !Number.isInteger(value.rows) ||
    typeof value.fitted !== "boolean"
  ) {
    return "json";
  }
  return {
    cssWidth: value.cssWidth as number,
    cssHeight: value.cssHeight as number,
    columns: value.columns,
    rows: value.rows,
    columnMillimetres: value.columnMillimetres as number,
    rowMillimetres: value.rowMillimetres as number,
    shiftColumn: value.shiftColumn as number,
    shiftRow: value.shiftRow as number,
    fitted: value.fitted,
    anchorX: value.anchorX as number,
    anchorY: value.anchorY as number,
    fitScale: value.fitScale as number,
  };
}

function readCamera(value: unknown): CameraSnapshot | "json" | "nonfinite" {
  if (!isRecord(value) || extra(value, CAMERA_KEYS)) {
    return "json";
  }
  const numbers = [value.panX, value.panY, value.zoom, value.radians];
  if (numbers.some((item) => typeof item !== "number")) {
    return "json";
  }
  if (numbers.some((item) => typeof item === "number" && !Number.isFinite(item))) {
    return "nonfinite";
  }
  if (typeof value.flipHorizontal !== "boolean" || typeof value.flipVertical !== "boolean") {
    return "json";
  }
  return {
    panX: value.panX as number,
    panY: value.panY as number,
    zoom: value.zoom as number,
    radians: value.radians as number,
    flipHorizontal: value.flipHorizontal,
    flipVertical: value.flipVertical,
  };
}

function readPresentation(
  value: unknown,
): PresentationSnapshot | "json" | "enum" | "nonfinite" {
  if (!isRecord(value) || extra(value, PRESENTATION_KEYS)) {
    return "json";
  }
  if (typeof value.photometric !== "string" || !PHOTOMETRIC.has(value.photometric)) {
    return typeof value.photometric === "string" ? "enum" : "json";
  }
  if (value.colormap !== "grey" && value.colormap !== "stops") {
    return typeof value.colormap === "string" ? "enum" : "json";
  }
  const modality = readModality(value.modality);
  const voi = readVoi(value.voi);
  const evidence = readEvidence(value.evidence);
  if (modality === "json" || voi === "json" || evidence === "json") {
    return "json";
  }
  if (modality === "enum" || voi === "enum" || evidence === "enum") {
    return "enum";
  }
  if (modality === "nonfinite" || voi === "nonfinite") {
    return "nonfinite";
  }
  return {
    modality,
    voi,
    photometric: value.photometric as PhotometricSnapshot,
    evidence,
    colormap: value.colormap,
  };
}

function readModality(value: unknown): ModalitySnapshot | "json" | "enum" | "nonfinite" {
  if (!isRecord(value) || typeof value.kind !== "string") {
    return "json";
  }
  if (value.kind === "rescale") {
    if (extra(value, ["kind", "slope", "intercept"])) {
      return "json";
    }
    if (typeof value.slope !== "number" || typeof value.intercept !== "number") {
      return "json";
    }
    if (!Number.isFinite(value.slope) || !Number.isFinite(value.intercept)) {
      return "nonfinite";
    }
    return { kind: "rescale", slope: value.slope, intercept: value.intercept };
  }
  if (value.kind === "lut") {
    return readLut(value);
  }
  return "enum";
}

function readVoi(value: unknown): VoiSnapshot | "json" | "enum" | "nonfinite" {
  if (!isRecord(value) || typeof value.kind !== "string") {
    return "json";
  }
  if (value.kind === "window") {
    if (extra(value, ["kind", "center", "width", "function", "ymin", "ymax"])) {
      return "json";
    }
    const numbers = [value.center, value.width, value.ymin, value.ymax];
    if (numbers.some((item) => typeof item !== "number")) {
      return "json";
    }
    if (numbers.some((item) => typeof item === "number" && !Number.isFinite(item))) {
      return "nonfinite";
    }
    if (typeof value.function !== "string" || !FUNCTIONS.has(value.function)) {
      return typeof value.function === "string" ? "enum" : "json";
    }
    return {
      kind: "window",
      center: value.center as number,
      width: value.width as number,
      function: value.function as "linear" | "linear_exact" | "sigmoid",
      ymin: value.ymin as number,
      ymax: value.ymax as number,
    };
  }
  if (value.kind === "lut") {
    return readLut(value);
  }
  return "enum";
}

function readLut(
  value: Record<string, unknown>,
): Extract<ModalitySnapshot, { kind: "lut" }> | "json" | "nonfinite" {
  if (extra(value, ["kind", "entryCount", "firstMappedInput", "bitsPerEntry", "values"])) {
    return "json";
  }
  if (
    typeof value.entryCount !== "number" ||
    typeof value.firstMappedInput !== "number" ||
    typeof value.bitsPerEntry !== "number" ||
    !Number.isInteger(value.entryCount) ||
    !Number.isInteger(value.firstMappedInput) ||
    !Number.isInteger(value.bitsPerEntry) ||
    !Array.isArray(value.values)
  ) {
    return "json";
  }
  if (value.values.some((item) => typeof item !== "number")) {
    return "json";
  }
  if (value.values.some((item) => typeof item === "number" && !Number.isFinite(item))) {
    return "nonfinite";
  }
  return {
    kind: "lut",
    entryCount: value.entryCount,
    firstMappedInput: value.firstMappedInput,
    bitsPerEntry: value.bitsPerEntry,
    values: value.values as number[],
  };
}

function readEvidence(value: unknown): EvidenceSnapshot | "json" | "enum" {
  if (!isRecord(value) || typeof value.kind !== "string") {
    return "json";
  }
  if (value.kind === "absent") {
    return extra(value, ["kind"]) ? "json" : { kind: "absent" };
  }
  if (value.kind === "sequence") {
    return extra(value, ["kind"]) ? "json" : { kind: "sequence" };
  }
  if (value.kind === "shape") {
    if (extra(value, ["kind", "shape"])) {
      return "json";
    }
    if (value.shape !== "identity" && value.shape !== "inverse") {
      return typeof value.shape === "string" ? "enum" : "json";
    }
    return { kind: "shape", shape: value.shape };
  }
  return "enum";
}
