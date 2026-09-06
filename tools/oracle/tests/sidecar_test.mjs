// Unit tests for the sidecar the driver writes beside every reference frame.
//
// HLD section 11 diffs metadata alongside pixels "because a wrong rescale
// slope can still produce a plausible image", so the sidecar is load-bearing
// output of this story and not a log. These tests cover its shape. Its DICOM
// content is cross-read against pydicom by `check_sidecars.py`, which is the
// fixture the design plan asks for: expected values from PS3.3 through a
// second reader, never from what the harness itself printed.

import test from "node:test";
import assert from "node:assert/strict";
import { createHash } from "node:crypto";

import {
  STACK_KIND,
  VOLUME_KIND,
  assertFrameIntegrity,
  buildSidecar,
  buildVolumeSidecar,
} from "../src/sidecar.mjs";

const ROW = {
  path: "synthetic/ct_unsigned_16.dcm",
  modality: "CT",
  transferSyntax: "1.2.840.10008.1.2.1",
  category: "synthetic, mono16, unsigned-16",
  categories: ["synthetic", "mono16", "unsigned-16"],
  source: "Ocelli synthetic, scripts/corpus_synth.py",
  licence: "MIT OR Apache-2.0",
  licenceUrl: "https://www.apache.org/licenses/LICENSE-2.0",
  sha256: "b".repeat(64),
  url: "",
  line: 6,
};

const PARAMS = {
  canvas: { width: 512, height: 512 },
  background: [0, 0, 0],
  interpolation: "NEAREST",
  camera: { mode: "reset" },
  voi: { source: "file" },
  allowUniform: false,
  matched: [
    { index: 0, match: { category: "mono16" }, why: "because the plan says so" },
  ],
  modalityVoiDefaults: { CT: { windowCenter: 40, windowWidth: 400 } },
};

const RESULT = {
  ok: true,
  voi: { source: "file", windowCenter: 40, windowWidth: 400, voiLutFunction: "LINEAR", origin: "file-top-level" },
  camera: { parallelScale: 16 },
  attributes: { rescaleSlope: 1, rescaleIntercept: -1024 },
  attributesError: null,
  cornerstoneMetadata: {
    modalityLutModule: { rescaleSlope: 1 },
    // docs/lld/oracle.md's worked case: 64 by 96 at spacing [0.5, 0.25] with
    // parallelScale 16 on a 512-high canvas gives 8 canvas pixels per source
    // pixel vertically and 4 horizontally.
    imagePlaneModule: { rowPixelSpacing: 0.5, columnPixelSpacing: 0.25 },
  },
  image: { minPixelValue: 0, maxPixelValue: 4095 },
  frame: {
    width: 512,
    height: 512,
    sha256: "a".repeat(64),
    statistics: { pixels: 262144, black: 100, white: 200, opaque: 262144, blackFraction: 0.1, whiteFraction: 0.2 },
  },
};

const ENVIRONMENT = {
  versions: { cornerstoneCore: "5.8.2", userAgent: "HeadlessChrome/151" },
  rendering: { renderer: "ANGLE (SwiftShader)" },
};

const INSTALLED = {
  "@cornerstonejs/core": "5.8.2",
  "@cornerstonejs/dicom-image-loader": "5.8.2",
};

function build() {
  return buildSidecar({
    row: ROW,
    params: PARAMS,
    result: RESULT,
    environment: ENVIRONMENT,
    installed: INSTALLED,
  });
}

test("the manifest row travels with the frame, verbatim", () => {
  const sidecar = build();
  assert.equal(sidecar.row.path, ROW.path);
  assert.equal(sidecar.row.sha256, ROW.sha256);
  assert.equal(sidecar.row.licence, ROW.licence);
  assert.deepEqual(sidecar.row.categories, ROW.categories);
});

// A row's provenance is what says whether the frame may be looked at, so it
// is never dropped: every real row carries `burned-in-unchecked`.
test("the row's own category tokens are carried, not summarised", () => {
  const real = { ...ROW, categories: ["real", "mono16", "burned-in-unchecked"] };
  const sidecar = buildSidecar({
    row: real,
    params: PARAMS,
    result: RESULT,
    environment: ENVIRONMENT,
    installed: INSTALLED,
  });
  assert.ok(sidecar.row.categories.includes("burned-in-unchecked"));
});

test("the parameters that produced the frame travel with it", () => {
  const sidecar = build();
  assert.deepEqual(sidecar.renderParams.canvas, { width: 512, height: 512 });
  assert.equal(sidecar.renderParams.interpolation, "NEAREST");
  assert.equal(sidecar.voi.windowCenter, 40);
  assert.deepEqual(sidecar.camera, RESULT.camera);
});

// Not just which rules fired, but why each exists. A reader who finds a colour
// frame with no window should be able to see the reason on the frame rather
// than reconstructing it from an index into a file they have to go and open.
test("the rules that fired travel with their reasons", () => {
  const sidecar = build();
  assert.deepEqual(sidecar.renderParamRulesApplied, [
    { index: 0, match: { category: "mono16" }, why: "because the plan says so" },
  ]);
});

// HLD section 11 diffs metadata alongside pixels "because a wrong rescale
// slope can still produce a plausible image", so this block is the reason the
// sidecar exists at all. It is cross-read against pydicom by
// `check_sidecars.py`, and it has to reach the file first.
test("both readings of the file's metadata travel with the frame", () => {
  const sidecar = build();
  assert.deepEqual(sidecar.attributes, RESULT.attributes);
  assert.equal(sidecar.attributesError, null);
  assert.deepEqual(sidecar.cornerstoneMetadata, RESULT.cornerstoneMetadata);
});

// A row the page could not parse independently records why, and
// `check_sidecars.py` refuses a sidecar carrying neither.
test("an unreadable file records the reason instead of the attributes", () => {
  const sidecar = buildSidecar({
    row: ROW,
    params: PARAMS,
    result: { ...RESULT, attributes: null, attributesError: "big endian" },
    environment: ENVIRONMENT,
    installed: INSTALLED,
  });
  assert.equal(sidecar.attributes, null);
  assert.equal(sidecar.attributesError, "big endian");
});

// `matched` and `modalityVoiDefaults` are resolution machinery. `matched` is
// republished under a name that says what it is, and the defaults table is the
// whole committed file rather than this row's parameter, so neither belongs
// inside `renderParams` where it would read as one.
test("resolution machinery is not passed off as a render parameter", () => {
  const sidecar = build();
  assert.equal(sidecar.renderParams.matched, undefined);
  assert.equal(sidecar.renderParams.modalityVoiDefaults, undefined);
});

test("the reference names the version and the adapter that produced it", () => {
  const sidecar = build();
  assert.equal(sidecar.reference.cornerstone3D, "5.8.2");
  assert.equal(sidecar.reference.adapter, "ANGLE (SwiftShader)");
  assert.equal(sidecar.reference.browser, "HeadlessChrome/151");
});

test("the sidecar names its own frame files", () => {
  const sidecar = build();
  assert.deepEqual(sidecar.files, {
    raw: "synthetic__ct_unsigned_16.raw",
    png: "synthetic__ct_unsigned_16.png",
  });
});

// F-011 dispatches on this field and must not infer a shape from a filename.
test("every sidecar says which shape it is", () => {
  assert.equal(build().kind, STACK_KIND);
  assert.equal(STACK_KIND, "stack");
  assert.equal(VOLUME_KIND, "volume-reformat");
});

// For EVERY row and not only for the two the run lists under `downsampled`.
// The derivation lives once, in `canvasScale`, and publishing it here is what
// stops F-011 writing a second copy of it in Rust.
//
// docs/lld/oracle.md's worked case, hand-computed from the numbers above:
//   millimetres per canvas pixel = 2 * 16 / 512 = 0.0625
//   vertical   = rowPixelSpacing    / 0.0625 = 0.5  / 0.0625 = 8
//   horizontal = columnPixelSpacing / 0.0625 = 0.25 / 0.0625 = 4
// A pixel-count model would have answered 5.333 for both axes.
test("the canvas scale travels on every frame, per axis", () => {
  assert.deepEqual(build().canvasPixelsPerSourcePixel, {
    vertical: 8,
    horizontal: 4,
  });
});

test("the raw format is stated, because F-011 reads the raw bytes", () => {
  const sidecar = build();
  assert.match(sidecar.frame.format, /RGBA8/);
  assert.equal(sidecar.frame.sha256, RESULT.frame.sha256);
});

// The page hashes the frame before it is base64'd out of the browser, and the
// driver hashes what arrived. A silent truncation between the two would leave
// F-011 comparing against a frame nobody rendered.
test("a frame that did not survive the trip out of the browser is refused", () => {
  const bytes = Buffer.from([1, 2, 3, 4]);
  const digest = createHash("sha256").update(bytes).digest("hex");
  assert.equal(
    assertFrameIntegrity(ROW.path, bytes, { frame: { sha256: digest } }),
    digest,
  );
  assert.throws(
    () => assertFrameIntegrity(ROW.path, bytes, { frame: { sha256: "c".repeat(64) } }),
    /did not survive/,
  );
});

// ---------------------------------------------------------------------------
// The volume reformat sidecar, F-X007
// ---------------------------------------------------------------------------

const SUBJECT = {
  id: "volume__synthetic__ct_series_nonuniform",
  seriesDirectory: "synthetic/ct_series_nonuniform",
  why: "the subject this story exists for",
  members: [
    "synthetic/ct_series_nonuniform/slice_000.dcm",
    "synthetic/ct_series_nonuniform/slice_001.dcm",
  ],
};

const VOLUME_RECORD = {
  ok: true,
  members: [
    {
      path: "synthetic/ct_series_nonuniform/slice_000.dcm",
      sha256: "d".repeat(64),
      stackSidecar: "synthetic__ct_series_nonuniform__slice_000.json",
      attributes: {
        imagePositionPatient: [0, 0, 0],
        imageOrientationPatient: [0.8, 0.6, 0, 0, 0, -1],
        pixelSpacing: [0.5, 0.25],
        sliceThickness: 2.5,
      },
    },
    {
      path: "synthetic/ct_series_nonuniform/slice_001.dcm",
      sha256: "e".repeat(64),
      stackSidecar: "synthetic__ct_series_nonuniform__slice_001.json",
      attributes: {
        imagePositionPatient: [-1.5, 2, 0],
        imageOrientationPatient: [0.8, 0.6, 0, 0, 0, -1],
        pixelSpacing: [0.5, 0.25],
        sliceThickness: 2.5,
      },
    },
  ],
  memberImageIds: ["dicomfile:0", "dicomfile:1"],
  referenceSortedImageIds: ["dicomfile:0", "dicomfile:1"],
  referenceGeometry: { dimensions: [20, 12, 2], spacing: [0.25, 0.5, 2.5] },
  zProfile: { voxel: [10, 6], values: [1056, 1072] },
  voi: { source: "file", windowCenter: 40, windowWidth: 400, voiLutFunction: "LINEAR", member: SUBJECT.members[0] },
  memberWindows: [{ path: SUBJECT.members[0] }, { path: SUBJECT.members[1] }],
  voiMembersAgree: true,
  frames: [],
};

const VOLUME_FRAME = {
  orientation: "SAGITTAL",
  camera: { parallelScale: 12, viewPlaneNormal: [1, 0, 0] },
  reformat: {
    orientation: "SAGITTAL",
    viewportType: "orthographic",
    blendMode: "COMPOSITE",
    slabThicknessMm: null,
    millimetresPerCanvasPixel: 0.046875,
    cameraMode: "reset",
  },
  frame: {
    width: 512,
    height: 512,
    sha256: "f".repeat(64),
    statistics: { pixels: 262144, black: 1, white: 2, opaque: 262144, blackFraction: 0.1, whiteFraction: 0.2 },
  },
};

function buildVolume(orientation = "SAGITTAL") {
  return buildVolumeSidecar({
    subject: SUBJECT,
    record: VOLUME_RECORD,
    frame: {
      ...VOLUME_FRAME,
      orientation,
      reformat: { ...VOLUME_FRAME.reformat, orientation },
    },
    measured: {
      normal: [-0.6, 0.8, 0],
      order: SUBJECT.members.map((path) => ({ path })),
      projectionsMm: [0, 2.5],
      gapsMm: [2.5],
      meanGapMm: 2.5,
      medianGapMm: 2.5,
      minGapMm: 2.5,
      maxGapMm: 2.5,
      maxDeviationFromMeanMm: 0,
      voxelAxes: {
        columnStepMm: [0.2, 0.15, 0],
        rowStepMm: [0, 0, -0.5],
        sliceStepMm: [-1.5, 2, 0],
      },
    },
    comparison: {
      truth: { source: "volume-truth.json", uniform: false, citation: "PS3.3 C.7.6.2.1.1", toleranceMm: 1e-6 },
      uniform: false,
      referenceAgreesWithTruth: false,
      referenceDivergence: { field: "spacing[2]", reference: 2.5, truth: null },
    },
    volumeParams: {
      orientations: ["AXIAL", "SAGITTAL", "CORONAL"],
      blendMode: "COMPOSITE",
      slabThicknessMm: null,
      cameraMode: "reset",
      loadTimeoutMs: 300000,
    },
    renderParams: PARAMS,
    environment: ENVIRONMENT,
    installed: INSTALLED,
  });
}

test("a volume sidecar says which shape it is and names its own frame files", () => {
  const sidecar = buildVolume();
  assert.equal(sidecar.kind, VOLUME_KIND);
  assert.deepEqual(sidecar.files, {
    raw: "volume__synthetic__ct_series_nonuniform__SAGITTAL.raw",
    png: "volume__synthetic__ct_series_nonuniform__SAGITTAL.png",
  });
});

// Two readings of the same geometry, exactly as the stack sidecar carries two
// readings of the same pixel metadata, and for the same reason: a sidecar that
// transcribed only the reference's own answer could not show the reference
// getting a series wrong.
test("both readings of the geometry travel with the reformat", () => {
  const sidecar = buildVolume();
  assert.deepEqual(sidecar.volume.referenceGeometry.spacing, [0.25, 0.5, 2.5]);
  assert.deepEqual(sidecar.volume.measuredGeometry.gapsMm, [2.5]);
  assert.equal(sidecar.volume.measuredGeometry.citation, "PS3.3 C.7.6.2.1.1");
  assert.equal(sidecar.volume.referenceAgreesWithTruth, false);
  assert.equal(sidecar.volume.referenceDivergence.field, "spacing[2]");
});

// A reformat is meaningless without the view plane normal, and the stack
// sidecar does not carry it.
test("a reformat carries the view plane normal the stack sidecar has no use for", () => {
  assert.deepEqual(buildVolume().camera.viewPlaneNormal, [1, 0, 0]);
});

// Every member's geometry, so `check_sidecars.py` can cross-read it against
// pydicom and so F-011 can join a reformat to the stack frames of the same
// instances.
test("every member's own geometry and its stack sidecar travel with the reformat", () => {
  const [first] = buildVolume().volume.members;
  assert.deepEqual(first.imagePositionPatient, [0, 0, 0]);
  assert.deepEqual(first.pixelSpacing, [0.5, 0.25]);
  assert.equal(first.sliceThickness, 2.5);
  assert.equal(first.stackSidecar, "synthetic__ct_series_nonuniform__slice_000.json");
});

// Deliberate. The existing design's rule is that a frame never travels alone,
// and a comparator that had to join two files to explain one frame would break
// it.
test("the volume block is identical in all three orientation sidecars", () => {
  const axial = buildVolume("AXIAL");
  const sagittal = buildVolume("SAGITTAL");
  assert.deepEqual(axial.volume, sagittal.volume);
  assert.notDeepEqual(axial.reformat, sagittal.reformat);
});

// `downsampled` and `canvasPixelsPerSourcePixel` stay stack concepts. A
// reformat plane has no source pixel grid to be a magnification of, and
// publishing a number that means something else under the same name is the
// defect this project calls quietly wrong.
test("a reformat publishes its own scale and not the stack's", () => {
  const sidecar = buildVolume();
  assert.equal(sidecar.canvasPixelsPerSourcePixel, undefined);
  assert.equal(sidecar.reformat.millimetresPerCanvasPixel, 0.046875);
});

// One volume viewport carries one voiRange. `real/mr_eay131` carries fifteen
// different windows, one per instance, so a sidecar publishing only the window
// that acted would read as though they all agreed.
test("the window that acted names the member it came from", () => {
  const sidecar = buildVolume();
  assert.equal(sidecar.voiMembers.member, SUBJECT.members[0]);
  assert.equal(sidecar.voiMembers.agree, true);
  assert.equal(sidecar.voiMembers.windows.length, 2);
});
