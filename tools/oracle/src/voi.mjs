// VOI resolution for the reference render.
//
// This module runs on BOTH sides of the harness. `build-page.mjs` bundles it
// into the browser page, where `page/app.mjs` calls it to choose the window a
// frame is rendered with AND to fill the window recorded in that frame's
// sidecar. The unit tests import the same file under node. So the function the
// tests exercise is the function the browser executes, and the rendered window
// and the recorded window are one value rather than two that agree today.
//
// PS3.3 C.11.2.1.2 is the normative source for the LINEAR formula quoted in
// `fullRange` below. C.11.2.1.3.1 and C.11.2.1.3.2 give SIGMOID's and
// LINEAR_EXACT's own width constraint, which is NOT the same as LINEAR's.
//
// The rule that an ABSENT VOI LUT Function means LINEAR is stated here without
// a clause number, deliberately. Earlier rounds of this file cited
// C.11.2.1.2.1 for it and nobody could quote the sentence being relied on, so
// the citation was removed rather than left as a reference a reader would stop
// at. The rule itself is not in doubt: LINEAR is the only function of the
// three that needs no attribute to select it, and the reference itself agrees:
// cornerstone3D 5.8.2's `utilities/windowLevel.js` declares
// `toLowHighRange(windowWidth, windowCenter, voiLUTFunction =
// VOILUTFunctionType.LINEAR)`, so an absent function reaches LINEAR there too,
// which is what this harness has to match whatever the clause number is.

/** The declared "use the image's own range" default. */
export const FULL_RANGE = Object.freeze({ rule: "full-range" });

const DEFAULT_FUNCTION = "LINEAR";

/**
 * The smallest Window Width PS3.3 permits for a VOI LUT Function.
 *
 * C.11.2.1.2 (LINEAR): "w >= 1", because the function divides by `w - 1`.
 * C.11.2.1.3.1 (SIGMOID) and C.11.2.1.3.2 (LINEAR_EXACT): "w > 0", because
 * both divide by `w` itself.
 *
 * Sharing one constant between them would be the same class of defect as
 * sharing one branch: the difference between LINEAR and LINEAR_EXACT is a half
 * and a one, and it is the most commonly mis-ported detail in DICOM viewers.
 */
export function minimumWidth(voiLutFunction) {
  return (voiLutFunction || DEFAULT_FUNCTION) === "LINEAR"
    ? 1
    : Number.MIN_VALUE;
}

/**
 * Window centre and width that map exactly `min` to `max` onto the display
 * range under PS3.3 C.11.2.1.2's LINEAR function.
 *
 * C.11.2.1.2 places the window's lower edge at `c - 0.5 - (w - 1) / 2` and its
 * upper edge at `c - 0.5 + (w - 1) / 2`. Substituting
 *
 *   w = max - min + 1
 *   c = (max + min + 1) / 2
 *
 * gives lower = min and upper = max exactly, so no stored value is clipped and
 * none of the display range is wasted.
 *
 * `max === min` is a constant image, and the formula answers `w = 1`. That is
 * the smallest width C.11.2.1.2 allows and it is also degenerate: `w' = w - 1`
 * is then zero, and every stored value maps to the same output. Nothing is
 * clamped here to hide that, because a constant image genuinely has no window
 * that shows anything, and the frame it produces is caught downstream by the
 * read-back degeneracy guard in `page/app.mjs` rather than papered over here.
 *
 * `max < min` is not a range at all and is refused, because a negative width
 * would sail through `toLowHighRange` and produce an inverted window nobody
 * asked for.
 */
export function fullRange(min, max) {
  if (max < min) {
    throw new Error(
      `the full-range VOI rule was given max ${max} below min ${min}. That is ` +
        `not a range, and a negative Window Width is not a window.`,
    );
  }
  return { windowCenter: (max + min + 1) / 2, windowWidth: max - min + 1 };
}

function firstNumber(value) {
  if (value === undefined || value === null) {
    return undefined;
  }
  const candidate = Array.isArray(value) ? value[0] : value;
  const numeric = typeof candidate === "string" ? Number(candidate) : candidate;
  return typeof numeric === "number" && Number.isFinite(numeric)
    ? numeric
    : undefined;
}

/**
 * Resolve the window a row is rendered with.
 *
 * @param {{source: string}} policy from render-params.json
 * @param {object} context the file's own values and the image's range
 * @returns {{source: string, windowCenter?: number, windowWidth?: number,
 *   voiLutFunction?: string}}
 */
export function resolveVoi(policy, context) {
  const source = policy?.source;
  if (source === "none") {
    return { source: "none" };
  }
  if (source !== "file") {
    throw new Error(
      `render-params.json declares VOI source ${JSON.stringify(source)}. The ` +
        `only sources are "file" and "none".`,
    );
  }

  const centre = firstNumber(context.fileWindowCenter);
  const width = firstNumber(context.fileWindowWidth);
  // An absent VOI LUT Function means LINEAR. See the module header for why
  // this rule carries no clause number.
  const declaredFunction = context.fileVoiLutFunction || DEFAULT_FUNCTION;
  // A file carrying a width below its function's own minimum, or only one of
  // the pair, has no usable window, so the declared default applies.
  if (
    centre !== undefined &&
    width !== undefined &&
    width >= minimumWidth(declaredFunction)
  ) {
    return {
      source: "file",
      windowCenter: centre,
      windowWidth: width,
      voiLutFunction: declaredFunction,
    };
  }

  const defaults = context.defaults ?? {};
  const declared = defaults[context.modality] ?? defaults["*"];
  if (!declared) {
    throw new Error(
      `render-params.json declares no VOI default for modality ` +
        `${context.modality} and no "*" catch-all. The default is written ` +
        `down rather than inherited from whatever the renderer happens to do.`,
    );
  }

  if (declared.rule === "full-range") {
    const { minPixelValue: min, maxPixelValue: max } = context;
    if (typeof min !== "number" || typeof max !== "number") {
      throw new Error(
        `the full-range VOI rule needs the image's own minimum and maximum, ` +
          `and got ${min} and ${max}`,
      );
    }
    return { source: "full-range", ...fullRange(min, max), voiLutFunction: DEFAULT_FUNCTION };
  }

  const declaredCentre = firstNumber(declared.windowCenter);
  const declaredWidth = firstNumber(declared.windowWidth);
  if (declaredCentre === undefined || declaredWidth === undefined) {
    throw new Error(
      `render-params.json's VOI default for ${context.modality} declares ` +
        `neither a rule nor a windowCenter and windowWidth pair`,
    );
  }
  return {
    source: "modality-default",
    windowCenter: declaredCentre,
    windowWidth: declaredWidth,
    voiLutFunction: declared.voiLutFunction || DEFAULT_FUNCTION,
  };
}
