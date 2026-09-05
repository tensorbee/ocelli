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

import { assertFrameIntegrity, buildSidecar } from "../src/sidecar.mjs";

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
  cornerstoneMetadata: { modalityLutModule: { rescaleSlope: 1 } },
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
