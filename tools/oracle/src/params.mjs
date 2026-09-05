// The render parameters, resolved per row from the committed declaration.
//
// HLD section 11 compares frames. A frame is a function of the window, the
// camera, the canvas size and the interpolation, and the DICOM file carries
// only the first of those. `render-params.json` writes the rest down so that
// two runs are comparable at all, and so F-011's tolerance policy has
// something to be a tolerance ON.

import { readFileSync } from "node:fs";

import { oraclePath } from "./paths.mjs";

export const RENDER_PARAMS_PATH = oraclePath("render-params.json");

/** Keys a rule may match on. A typo here would match nothing, silently. */
const MATCH_KEYS = new Set(["path", "modality", "category", "transferSyntax"]);

/**
 * Keys a rule may SET.
 *
 * Deliberately narrower than the set of keys in `base`. `canvas` and
 * `background` are properties of the RUN rather than of a row: one viewport
 * serves all ninety-one rows, and it is sized and coloured once, from the
 * first row's resolution. A rule that set either would be copied into the
 * sidecar as "the parameters that produced this frame" while having produced
 * nothing, which is the exact failure the refusal below exists for. Widening
 * this set means giving each row its own viewport first.
 */
const APPLY_KEYS = new Set(["interpolation", "camera", "voi", "allowUniform"]);

/** Read the committed declaration. */
export function readRenderParams() {
  return JSON.parse(readFileSync(RENDER_PARAMS_PATH, "utf8"));
}

function matches(match, row) {
  for (const key of Object.keys(match)) {
    if (!MATCH_KEYS.has(key)) {
      throw new Error(
        `render-params.json rule matches on ${JSON.stringify(key)}, which is ` +
          `not one of ${[...MATCH_KEYS].join(", ")}. A rule matching on a key ` +
          `nobody reads matches nothing and leaves the base in place.`,
      );
    }
  }
  if (match.path !== undefined && match.path !== row.path) {
    return false;
  }
  if (match.modality !== undefined && match.modality !== row.modality) {
    return false;
  }
  if (
    match.transferSyntax !== undefined &&
    match.transferSyntax !== row.transferSyntax
  ) {
    return false;
  }
  if (
    match.category !== undefined &&
    !(row.categories ?? []).includes(match.category)
  ) {
    return false;
  }
  return true;
}

/**
 * Apply one rule's `apply` block over the resolved parameters.
 *
 * A key is REPLACED wholesale, never merged into. Merging would let a rule
 * setting `voi.source` to "none" keep the base's `voi.why`, which explains why
 * the file's own window is used, and that sentence would then travel into the
 * sidecar beside the value it contradicts. The sidecar is load-bearing output
 * under HLD section 11 and must not carry a rationale for something that did
 * not happen. Wholesale replacement also leaves one rule for a reader to hold:
 * later wins.
 */
function applyBlock(target, block) {
  for (const [key, value] of Object.entries(block)) {
    if (!APPLY_KEYS.has(key)) {
      throw new Error(
        `render-params.json rule sets ${JSON.stringify(key)}, which is not ` +
          `one of ${[...APPLY_KEYS].join(", ")}. A parameter nobody reads is ` +
          `a parameter that did not take effect.`,
      );
    }
    // Cloned, not shared. `spec` is read once per run and resolved ninety-one
    // times, so handing every row a reference into the same declaration would
    // make one row's mutation everybody's. Nothing mutates them today, and
    // this is what keeps that from being a thing to remember.
    target[key] = structuredClone(value);
  }
}

/**
 * Resolve one row's render parameters.
 *
 * Rules apply in file order and a later rule wins, so the file reads top to
 * bottom from general to specific. `matched` records which rules fired AND why
 * each exists, and it goes into the row's sidecar, so a frame carries the
 * reasoning that shaped it rather than only the values.
 */
export function resolveRenderParams(spec, row) {
  const resolved = structuredClone(spec.base);
  const matched = [];
  const rules = spec.rules ?? [];
  for (let index = 0; index < rules.length; index += 1) {
    const rule = rules[index];
    if (matches(rule.match ?? {}, row)) {
      applyBlock(resolved, rule.apply ?? {});
      matched.push({ index, match: rule.match ?? {}, why: rule.why ?? null });
    }
  }
  resolved.matched = matched;
  resolved.modalityVoiDefaults = structuredClone(spec.modalityVoiDefaults);
  return resolved;
}

/**
 * How many canvas pixels one source pixel covers, per axis.
 *
 * `resetCamera` fits the image by its PHYSICAL extent, not by its pixel count,
 * so a frame with non-square pixels is scaled differently in each direction and
 * neither factor is `canvasWidth / columns`. VTK's `parallelScale` is half the
 * viewport's height in world millimetres, so
 *
 *   millimetres per canvas pixel = 2 * parallelScale / canvasHeight
 *
 * and a source pixel `rowPixelSpacing` millimetres tall covers that many canvas
 * pixels vertically. PS3.3 C.7.6.2.1.1: `PixelSpacing[0]` is the spacing
 * BETWEEN ROWS, which is the vertical one, so it is the row spacing that gives
 * the vertical factor.
 *
 * A factor below 1 means the reference frame is a DECIMATION of the source,
 * which matters because a per-modality tolerance written against a magnified
 * frame does not automatically hold for a decimated one.
 *
 * `parallelScale` is read back from the reference rather than modelled, so this
 * reports what cornerstone3D actually did.
 */
export function canvasScale({
  parallelScale,
  canvasHeight,
  rowPixelSpacing,
  columnPixelSpacing,
}) {
  if (!(parallelScale > 0) || !(canvasHeight > 0)) {
    throw new Error(
      `canvasScale needs a positive parallelScale and canvas height, and got ` +
        `${parallelScale} and ${canvasHeight}`,
    );
  }
  const millimetresPerCanvasPixel = (2 * parallelScale) / canvasHeight;
  // Pixel Spacing is Type 1 in PS3.3 Table C.7-10, so it is required WHERE THE
  // IMAGE PLANE MODULE APPLIES, and three corpus rows have none at the top
  // level: the DX and US rows, whose IODs do not include that module, and the
  // enhanced CT row, which carries it in a shared functional group. The
  // reference resolves a value for all three, so this fallback is reached only
  // by the unit test, and it matches what the reference does when it cannot
  // resolve one: treat a pixel as one unit square.
  const vertical = (rowPixelSpacing ?? 1) / millimetresPerCanvasPixel;
  const horizontal = (columnPixelSpacing ?? 1) / millimetresPerCanvasPixel;
  return { vertical, horizontal };
}
