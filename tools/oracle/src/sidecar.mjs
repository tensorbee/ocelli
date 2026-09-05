// The sidecar written beside every reference frame.
//
// HLD section 11: "metadata diffed alongside pixels because a wrong rescale
// slope can still produce a plausible image". So a frame never travels alone.
// The sidecar carries the manifest row it came from, the parameters that
// produced it, the DICOM attributes read from the file, the same attributes as
// cornerstone3D resolved them, and the version and adapter that rendered it.
//
// Two readings of the same attributes are recorded on purpose. The `attributes`
// block is read straight from the bytes and the `cornerstoneMetadata` block is
// what the reference itself used, so a sidecar can show the reference reading a
// file wrong. A sidecar that only transcribed the reference's own reading could
// not.

import { digestOf, rowId } from "./manifest.mjs";

/**
 * The digest of the frame bytes, checked against what the page computed.
 *
 * The page hashes the frame before base64 encoding it out of the browser and
 * this hashes what arrived. A silent truncation between the two would leave
 * F-011 comparing against a frame nobody rendered.
 */
export function assertFrameIntegrity(path, raw, result) {
  const digest = digestOf(raw);
  if (digest !== result.frame.sha256) {
    throw new Error(
      `row ${path}: the page hashed its frame as ${result.frame.sha256} and ` +
        `the bytes that arrived hash to ${digest}. The frame did not survive ` +
        `the trip out of the browser intact.`,
    );
  }
  return digest;
}

/** Assemble one row's sidecar. */
export function buildSidecar({ row, params, result, environment, installed }) {
  const id = rowId(row.path);
  // `matched` is republished below under a name that says what it is, and
  // `modalityVoiDefaults` is the whole committed table rather than this row's
  // parameter. Neither belongs inside `renderParams`, where it would read as
  // one.
  const { matched, modalityVoiDefaults, ...renderParams } = params;
  void modalityVoiDefaults;

  return {
    row: {
      path: row.path,
      modality: row.modality,
      transferSyntax: row.transferSyntax,
      category: row.category,
      categories: row.categories,
      source: row.source,
      licence: row.licence,
      licenceUrl: row.licenceUrl,
      sha256: row.sha256,
    },
    renderParams,
    renderParamRulesApplied: matched,
    voi: result.voi,
    camera: result.camera,
    attributes: result.attributes,
    attributesError: result.attributesError,
    cornerstoneMetadata: result.cornerstoneMetadata,
    image: result.image,
    frame: {
      width: result.frame.width,
      height: result.frame.height,
      format: "RGBA8, top row first, as ImageData from the viewport canvas",
      sha256: result.frame.sha256,
      statistics: result.frame.statistics,
    },
    reference: {
      cornerstone3D: installed["@cornerstonejs/core"],
      dicomImageLoader: installed["@cornerstonejs/dicom-image-loader"],
      browser: environment.versions.userAgent,
      adapter: environment.rendering.renderer,
    },
    files: { raw: `${id}.raw`, png: `${id}.png` },
  };
}
