// Unit tests for the render parameters and the VOI resolution rule.
//
// A rendered frame is a function of the window, the camera, the canvas size
// and the interpolation, and none of those is in the DICOM file. Two runs that
// disagree on any of them produce two correct frames that differ, which would
// read in F-011 as a port defect. So the parameters are declared in
// `render-params.json` and resolved by one tested function, and the VOI values
// below are hand-computed from PS3.3 rather than read back from the resolver.

import test from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";

import { Enums } from "@cornerstonejs/core";

import {
  canvasScale,
  resolveRenderParams,
  RENDER_PARAMS_PATH,
} from "../src/params.mjs";
import { fullRange, minimumWidth, resolveVoi, FULL_RANGE } from "../src/voi.mjs";
import { parseManifest } from "../src/manifest.mjs";
import { repoPath } from "../src/paths.mjs";

const SPEC = JSON.parse(readFileSync(RENDER_PARAMS_PATH, "utf8"));

const MANIFEST = parseManifest(
  readFileSync(repoPath("corpus/manifest.tsv"), "utf8"),
);

const row = (over = {}) => ({
  path: "synthetic/ct_unsigned_16.dcm",
  modality: "CT",
  transferSyntax: "1.2.840.10008.1.2.1",
  categories: ["synthetic", "mono16", "unsigned-16"],
  ...over,
});

// ---------------------------------------------------------------------------
// The committed spec
// ---------------------------------------------------------------------------

test("the committed spec declares every parameter a frame depends on", () => {
  assert.equal(SPEC.version, 1);
  assert.equal(typeof SPEC.base.canvas.width, "number");
  assert.equal(typeof SPEC.base.canvas.height, "number");
  assert.ok(Array.isArray(SPEC.base.background));
  assert.equal(SPEC.base.interpolation, "NEAREST");
  assert.equal(typeof SPEC.base.camera.mode, "string");
  assert.equal(typeof SPEC.base.voi.source, "string");
  assert.equal(typeof SPEC.base.allowUniform, "boolean");
  assert.ok(SPEC.modalityVoiDefaults["*"], "a catch-all default is declared");
  assert.equal(
    typeof SPEC.informationFloor.extremeFractionWarnAbove,
    "number",
    "the run refuses to start without this, so the spec must carry it",
  );
});

// `reset` is the only camera the page implements, and it refuses any other
// value. A spec declaring a mode nothing implements would be a parameter that
// did not take effect, recorded in every sidecar as though it had.
test("the only declared camera mode is the one the page implements", () => {
  assert.equal(SPEC.base.camera.mode, "reset");
});

// The same, for interpolation, and asked of cornerstone3D itself rather than
// of a list written here. `InterpolationType` is a TypeScript numeric enum, so
// it is reverse mapped and it sits on Object.prototype: a lookup would have
// admitted `constructor` and `"0"` alike, which is why the page checks the
// NAME against these keys. Asserting the same way here means cornerstone3D
// renaming or dropping `NEAREST` goes red rather than silently changing what
// every reference frame was rendered with.
test("the declared interpolation is a name cornerstone3D defines", () => {
  const names = Object.keys(Enums.InterpolationType).filter(
    (name) => !/^\d+$/.test(name),
  );
  assert.ok(names.length > 0, "cornerstone3D names no interpolation types");
  assert.ok(
    names.includes(SPEC.base.interpolation),
    `render-params.json declares ${SPEC.base.interpolation} and cornerstone3D ` +
      `names ${names.join(", ")}`,
  );
  // The two shapes a lookup would have admitted and a name check does not.
  assert.ok(!names.includes("constructor"));
  assert.ok(!names.some((name) => /^\d+$/.test(name)));
});

test("every modality in the committed manifest has a declared VOI default", () => {
  const modalities = new Set(MANIFEST.map((r) => r.modality));
  for (const modality of modalities) {
    const chosen =
      SPEC.modalityVoiDefaults[modality] ?? SPEC.modalityVoiDefaults["*"];
    assert.ok(chosen, `no VOI default resolves for modality ${modality}`);
  }
});

test("every rule in the committed spec resolves for at least one row", () => {
  const used = new Set();
  for (const r of MANIFEST) {
    for (const applied of resolveRenderParams(SPEC, r).matched) {
      used.add(applied.index);
    }
  }
  const unused = SPEC.rules
    .map((rule, index) => ({ rule, index }))
    .filter(({ index }) => !used.has(index));
  assert.deepEqual(
    unused.map(({ rule }) => rule.why ?? rule.match),
    [],
    "a rule matching nothing is a rule nobody has checked",
  );
});

test("every rule in the committed spec says why it exists", () => {
  for (const rule of SPEC.rules) {
    assert.equal(
      typeof rule.why,
      "string",
      `a rule matching ${JSON.stringify(rule.match)} carries no reason, and ` +
        `the reason is what travels into the sidecar`,
    );
  }
});

test("every committed row resolves without throwing", () => {
  for (const r of MANIFEST) {
    const params = resolveRenderParams(SPEC, r);
    assert.equal(params.canvas.width, SPEC.base.canvas.width);
    assert.ok(params.voi.source);
  }
});

// ---------------------------------------------------------------------------
// Resolution
// ---------------------------------------------------------------------------

const MINIMAL = {
  version: 1,
  base: {
    canvas: { width: 8, height: 8 },
    background: [0, 0, 0],
    interpolation: "NEAREST",
    camera: { mode: "reset" },
    voi: { source: "file", why: "the base says so" },
    allowUniform: false,
  },
  modalityVoiDefaults: { "*": FULL_RANGE },
  rules: [],
};

test("with no rules, the base applies unchanged", () => {
  const params = resolveRenderParams(MINIMAL, row());
  assert.deepEqual(params.canvas, { width: 8, height: 8 });
  assert.equal(params.voi.source, "file");
  assert.deepEqual(params.matched, []);
});

test("a category rule overrides the base", () => {
  const spec = {
    ...MINIMAL,
    rules: [
      {
        match: { category: "colour" },
        apply: { voi: { source: "none" } },
        why: "an RGB frame has no VOI stage",
      },
    ],
  };
  assert.equal(resolveRenderParams(spec, row()).voi.source, "file");
  const colour = row({ categories: ["synthetic", "colour"], modality: "OT" });
  const params = resolveRenderParams(spec, colour);
  assert.equal(params.voi.source, "none");
  assert.deepEqual(params.matched, [
    { index: 0, match: { category: "colour" }, why: "an RGB frame has no VOI stage" },
  ]);
});

// A rule REPLACES a key, it does not merge into it. If it merged, the colour
// rule above would keep the base's `why`, which explains why the file's own
// window is used, and the sidecar would carry that sentence beside
// `source: "none"`. HLD section 11 makes the sidecar load-bearing output, so it
// must not carry a rationale for something that did not happen.
test("a rule replaces a key wholesale rather than merging into it", () => {
  const spec = {
    ...MINIMAL,
    rules: [
      {
        match: { category: "colour" },
        apply: { voi: { source: "none" } },
        why: "an RGB frame has no VOI stage",
      },
    ],
  };
  const colour = row({ categories: ["synthetic", "colour"], modality: "OT" });
  assert.deepEqual(resolveRenderParams(spec, colour).voi, { source: "none" });
});

// `spec` is read once per run and resolved ninety-one times. If a rule's value
// or the defaults table were handed out by reference, one row's mutation would
// be every row's, and the row that noticed would be whichever came last.
// Nothing mutates them today, and these are what stop that being a thing to
// remember.
test("a rule's value is cloned into the result, not shared with the spec", () => {
  const spec = {
    ...MINIMAL,
    rules: [
      {
        match: { modality: "CT" },
        apply: { camera: { mode: "reset" } },
        why: "d",
      },
    ],
  };
  const first = resolveRenderParams(spec, row());
  first.camera.mode = "mutated";
  assert.equal(spec.rules[0].apply.camera.mode, "reset");
  assert.equal(resolveRenderParams(spec, row()).camera.mode, "reset");
});

test("the defaults table is cloned per row, not shared", () => {
  const first = resolveRenderParams(MINIMAL, row());
  first.modalityVoiDefaults["*"] = { rule: "invented" };
  assert.deepEqual(MINIMAL.modalityVoiDefaults["*"], FULL_RANGE);
  assert.deepEqual(
    resolveRenderParams(MINIMAL, row()).modalityVoiDefaults["*"],
    FULL_RANGE,
  );
});

test("a later rule wins over an earlier one", () => {
  const spec = {
    ...MINIMAL,
    rules: [
      { match: { category: "mono16" }, apply: { interpolation: "LINEAR" }, why: "a" },
      {
        match: { path: "synthetic/ct_unsigned_16.dcm" },
        apply: { interpolation: "NEAREST" },
        why: "b",
      },
    ],
  };
  const params = resolveRenderParams(spec, row());
  assert.equal(params.interpolation, "NEAREST");
  assert.deepEqual(
    params.matched.map((applied) => applied.index),
    [0, 1],
  );
});

test("a modality match is exact, not a prefix", () => {
  const spec = {
    ...MINIMAL,
    rules: [{ match: { modality: "CT" }, apply: { allowUniform: true }, why: "c" }],
  };
  assert.equal(resolveRenderParams(spec, row()).allowUniform, true);
  assert.equal(resolveRenderParams(spec, row({ modality: "CR" })).allowUniform, false);
});

// A typo in a match key would match nothing and quietly leave the base in
// place, which is exactly the failure this story exists to make loud.
test("an unknown match key is refused", () => {
  const spec = {
    ...MINIMAL,
    rules: [{ match: { catagory: "colour" }, apply: { allowUniform: true } }],
  };
  assert.throws(() => resolveRenderParams(spec, row()), /catagory/);
});

test("an unknown apply key is refused", () => {
  const spec = {
    ...MINIMAL,
    rules: [{ match: { modality: "CT" }, apply: { interplation: "LINEAR" } }],
  };
  assert.throws(() => resolveRenderParams(spec, row()), /interplation/);
});

// `canvas` and `background` are properties of the run, not of a row. One
// viewport serves every row and is sized and coloured once, so a rule setting
// either would be published in the sidecar as a parameter that produced the
// frame, having produced nothing.
test("a rule that tries to set a run-level parameter is refused", () => {
  for (const key of ["canvas", "background"]) {
    const spec = {
      ...MINIMAL,
      rules: [{ match: { modality: "CT" }, apply: { [key]: { width: 1 } } }],
    };
    assert.throws(
      () => resolveRenderParams(spec, row()),
      new RegExp(key),
      `${key} must not be settable per row`,
    );
  }
});

// ---------------------------------------------------------------------------
// How much of the source survives into the canvas
// ---------------------------------------------------------------------------

// Hand-computed for syntax/reference_mono12.dcm, whose numbers the reference
// itself produced and which the sidecar records: 64 rows by 96 columns,
// PixelSpacing [0.5, 0.25], parallelScale 16 after resetCamera, on a 512 by
// 512 canvas.
//
//   millimetres per canvas pixel = 2 * 16 / 512 = 0.0625
//   vertical   = 0.5  / 0.0625 = 8
//   horizontal = 0.25 / 0.0625 = 4
//
// So the image occupies 64 * 8 = 512 canvas rows, the whole height, and
// 96 * 4 = 384 canvas columns of 512, which leaves 128 / 512 = 25% of the
// canvas as letterbox. The recorded frame statistics for that row report
// blackFraction 0.25 exactly, which is the independent confirmation that the
// fit is by physical extent and not by pixel count.
test("the canvas scale is per axis and follows the physical fit", () => {
  assert.deepEqual(
    canvasScale({
      parallelScale: 16,
      canvasHeight: 512,
      rowPixelSpacing: 0.5,
      columnPixelSpacing: 0.25,
    }),
    { vertical: 8, horizontal: 4 },
  );
});

// A model that ignored pixel spacing would answer
// min(512 / 96, 512 / 64) = 5.333 for both axes, which is neither of the two
// real factors and is not even between them. It also cannot produce two
// different factors at all, and the two here differ by exactly the ratio of
// the two Pixel Spacing values.
test("the canvas scale is not the pixel-count ratio", () => {
  const { vertical, horizontal } = canvasScale({
    parallelScale: 16,
    canvasHeight: 512,
    rowPixelSpacing: 0.5,
    columnPixelSpacing: 0.25,
  });
  const pixelCountFit = Math.min(512 / 96, 512 / 64);
  assert.notEqual(vertical, pixelCountFit);
  assert.notEqual(horizontal, pixelCountFit);
  assert.equal(vertical / horizontal, 0.5 / 0.25);
});

// Pixel Spacing is Type 1 in PS3.3 Table C.7-10, so this is the case where the
// Image Plane Module does not apply at all and the reference has nothing to
// resolve. No corpus row reaches it, which is why it is asserted here.
test("an absent pixel spacing is one unit square", () => {
  assert.deepEqual(
    canvasScale({ parallelScale: 6, canvasHeight: 24 }),
    { vertical: 2, horizontal: 2 },
  );
});

test("a frame larger than the canvas scales below one", () => {
  // 1168 columns at 1 mm across a 512 canvas: parallelScale is half the
  // vertical extent, 584 mm, so one source pixel covers under half a pixel.
  const { horizontal } = canvasScale({
    parallelScale: 584,
    canvasHeight: 512,
    rowPixelSpacing: 1,
    columnPixelSpacing: 1,
  });
  assert.ok(horizontal < 1, `expected a decimation, got ${horizontal}`);
});

test("a camera with no parallel scale is refused rather than assumed", () => {
  assert.throws(() => canvasScale({ parallelScale: 0, canvasHeight: 512 }), /positive/);
  assert.throws(
    () => canvasScale({ parallelScale: 16, canvasHeight: undefined }),
    /positive/,
  );
});

// ---------------------------------------------------------------------------
// VOI, PS3.3 C.11.2.1.2 and C.11.2.1.3
// ---------------------------------------------------------------------------

const DEFAULTS = {
  CT: { windowCenter: 40, windowWidth: 400 },
  "*": FULL_RANGE,
};

const context = (over = {}) => ({
  modality: "CT",
  minPixelValue: -1024,
  maxPixelValue: 3071,
  defaults: DEFAULTS,
  ...over,
});

const FROM_FILE = { source: "file" };

test("the file's own window is used when the file has one", () => {
  const voi = resolveVoi(
    FROM_FILE,
    context({
      fileWindowCenter: [40],
      fileWindowWidth: [400],
      fileVoiLutFunction: "LINEAR",
    }),
  );
  assert.deepEqual(voi, {
    source: "file",
    windowCenter: 40,
    windowWidth: 400,
    voiLutFunction: "LINEAR",
  });
});

// PS3.3 C.11.2.1.2: a multi-valued Window Center and Window Width pair is a
// set of alternative presentations. The oracle takes the first, always, so
// two runs cannot disagree about which.
test("a multi-valued window takes the first pair", () => {
  const voi = resolveVoi(
    FROM_FILE,
    context({ fileWindowCenter: [40, -600], fileWindowWidth: [400, 1500] }),
  );
  assert.equal(voi.windowCenter, 40);
  assert.equal(voi.windowWidth, 400);
});

// VOI LUT Function absent means LINEAR. Stated without a clause number, for
// the reason src/voi.mjs's header gives.
test("an absent VOI LUT Function is LINEAR", () => {
  const voi = resolveVoi(
    FROM_FILE,
    context({
      fileWindowCenter: [40],
      fileWindowWidth: [400],
      fileVoiLutFunction: undefined,
    }),
  );
  assert.equal(voi.voiLutFunction, "LINEAR");
});

test("LINEAR_EXACT in the file is carried through, not normalised away", () => {
  const voi = resolveVoi(
    FROM_FILE,
    context({
      fileWindowCenter: [40],
      fileWindowWidth: [400],
      fileVoiLutFunction: "LINEAR_EXACT",
    }),
  );
  assert.equal(voi.voiLutFunction, "LINEAR_EXACT");
});

test("the declared modality default is used when the file has no window", () => {
  const voi = resolveVoi(FROM_FILE, context());
  assert.equal(voi.source, "modality-default");
  assert.equal(voi.windowCenter, 40);
  assert.equal(voi.windowWidth, 400);
});

// The full-range rule, hand-computed from PS3.3 C.11.2.1.2's LINEAR formula.
//
//   Below the window:  x <= c - 0.5 - (w - 1) / 2
//   Above the window:  x >  c - 0.5 + (w - 1) / 2
//
// Choosing w = max - min + 1 and c = (max + min + 1) / 2 makes the lower
// threshold exactly min and the upper exactly max:
//
//   c - 0.5 - (w - 1) / 2 = (max + min + 1) / 2 - 0.5 - (max - min) / 2 = min
//   c - 0.5 + (w - 1) / 2 = (max + min + 1) / 2 - 0.5 + (max - min) / 2 = max
//
// For min = -1024 and max = 3071 that is w = 4096 and c = 1024.
test("the full-range rule maps exactly min to max onto the display range", () => {
  const voi = resolveVoi(FROM_FILE, context({ modality: "MR" }));
  assert.equal(voi.source, "full-range");
  assert.equal(voi.windowWidth, 4096);
  assert.equal(voi.windowCenter, 1024);
  assert.equal(voi.voiLutFunction, "LINEAR");
});

// The full-range rule computes the window FROM the image, so without the
// image's own minimum and maximum there is nothing to compute it from. The
// refusal was watched by nothing: with `if (false)` in its place `fullRange`
// received undefined and the sidecar would have recorded a NaN window as
// though it had been resolved.
test("the full-range rule without the image's range is refused", () => {
  for (const over of [
    { minPixelValue: undefined },
    { maxPixelValue: undefined },
    { minPixelValue: "0", maxPixelValue: "255" },
  ]) {
    assert.throws(
      () => resolveVoi(FROM_FILE, context({ modality: "MR", ...over })),
      /needs the image's own minimum and maximum/,
      `${JSON.stringify(over)} was accepted`,
    );
  }
});

// A modality default is either a rule or a window pair, and an entry that is
// neither is a typo in render-params.json rather than a shape to interpret.
// Also watched by nothing before this test.
test("a modality default that is neither a rule nor a window pair is refused", () => {
  for (const declared of [{}, { windowCenter: 40 }, { windowWidth: 400 },
                          { rule: "half-range" }]) {
    assert.throws(
      () => resolveVoi(FROM_FILE, context({ defaults: { CT: declared } })),
      /neither a rule nor a windowCenter and windowWidth pair/,
      `${JSON.stringify(declared)} was accepted`,
    );
  }
});

test("full range over 0 to 255 is width 256 centre 128", () => {
  const voi = resolveVoi(
    FROM_FILE,
    context({ modality: "MR", minPixelValue: 0, maxPixelValue: 255 }),
  );
  assert.equal(voi.windowWidth, 256);
  assert.equal(voi.windowCenter, 128);
});

// A constant image answers w = 1, which is the smallest width C.11.2.1.2
// allows AND is degenerate: w' = w - 1 is zero and every value maps to the
// same output. Nothing clamps it here. The uniform frame it produces is
// refused by the read-back degeneracy guard instead, which is where a frame
// that shows nothing belongs.
test("full range over a constant image is the degenerate width 1", () => {
  const voi = resolveVoi(
    FROM_FILE,
    context({ modality: "MR", minPixelValue: 7, maxPixelValue: 7 }),
  );
  assert.equal(voi.windowWidth, 1);
  assert.equal(voi.windowCenter, 7.5);
});

// Not a range. A negative width would pass straight through cornerstone3D's
// `toLowHighRange` and produce an inverted window nobody asked for.
test("full range over a reversed range is refused, not silently clamped", () => {
  assert.throws(() => fullRange(10, 3), /not a range/);
  assert.throws(
    () =>
      resolveVoi(
        FROM_FILE,
        context({ modality: "MR", minPixelValue: 10, maxPixelValue: 3 }),
      ),
    /not a range/,
  );
});

// PS3.3 C.11.2.1.2 requires w >= 1 for LINEAR, because it divides by w - 1.
// C.11.2.1.3.1 (SIGMOID) and C.11.2.1.3.2 (LINEAR_EXACT) require only w > 0,
// because they divide by w. One constant shared between them would be the same
// class of defect as one branch shared between them.
test("the minimum window width is per function, not shared", () => {
  assert.equal(minimumWidth("LINEAR"), 1);
  assert.equal(minimumWidth(undefined), 1);
  assert.ok(minimumWidth("LINEAR_EXACT") < 1);
  assert.ok(minimumWidth("LINEAR_EXACT") > 0);
  assert.ok(minimumWidth("SIGMOID") < 1);
});

test("a LINEAR window narrower than 1 falls back to the default", () => {
  const voi = resolveVoi(
    FROM_FILE,
    context({
      fileWindowCenter: [40],
      fileWindowWidth: [0.5],
      fileVoiLutFunction: "LINEAR",
    }),
  );
  assert.equal(voi.source, "modality-default");
});

// Several widths, all legal under C.11.2.1.3.2's `w > 0` and all narrower than
// LINEAR's minimum. Asserting an interval rather than values would leave the
// constant free to sit anywhere inside it, and a threshold of, say, 0.5 would
// silently discard a conformant window of 0.25.
test("the same window is honoured under LINEAR_EXACT, which allows it", () => {
  for (const width of [0.5, 0.25, 0.001, Number.MIN_VALUE]) {
    const voi = resolveVoi(
      FROM_FILE,
      context({
        fileWindowCenter: [40],
        fileWindowWidth: [width],
        fileVoiLutFunction: "LINEAR_EXACT",
      }),
    );
    assert.equal(voi.source, "file", `width ${width} was discarded`);
    assert.equal(voi.windowWidth, width);
    assert.equal(voi.voiLutFunction, "LINEAR_EXACT");
  }
});

// The other side of the same boundary: zero and below are refused, and this is
// what makes the acceptance above a boundary rather than a blanket.
test("a negative window width is refused under every function", () => {
  for (const fn of ["LINEAR", "LINEAR_EXACT", "SIGMOID"]) {
    const voi = resolveVoi(
      FROM_FILE,
      context({
        fileWindowCenter: [40],
        fileWindowWidth: [-400],
        fileVoiLutFunction: fn,
      }),
    );
    assert.equal(voi.source, "modality-default", `width -400 accepted under ${fn}`);
  }
});

test("a window width of zero is refused under every function", () => {
  for (const fn of ["LINEAR", "LINEAR_EXACT", "SIGMOID", undefined]) {
    const voi = resolveVoi(
      FROM_FILE,
      context({
        fileWindowCenter: [40],
        fileWindowWidth: [0],
        fileVoiLutFunction: fn,
      }),
    );
    assert.equal(voi.source, "modality-default", `width 0 accepted under ${fn}`);
  }
});

test("source none resolves to no window at all", () => {
  const voi = resolveVoi(
    { source: "none" },
    context({
      modality: "OT",
      fileWindowCenter: [128],
      fileWindowWidth: [256],
      minPixelValue: 0,
      maxPixelValue: 255,
    }),
  );
  assert.deepEqual(voi, { source: "none" });
});

test("an unknown VOI source is refused rather than defaulted", () => {
  assert.throws(() => resolveVoi({ source: "whatever" }, context()), /whatever/);
});

test("a modality with no default and no catch-all is refused", () => {
  assert.throws(
    () => resolveVoi(FROM_FILE, context({ modality: "XA", defaults: { CT: {} } })),
    /XA/,
  );
});
