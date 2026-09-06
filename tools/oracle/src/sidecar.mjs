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
import { canvasScale } from "./params.mjs";
import { frameIdFor } from "./volume.mjs";

/**
 * The shape discriminator, on EVERY sidecar including the stack ones.
 *
 * F-011 switches on this and must not infer a shape from a filename. It is the
 * single field that makes "a comparator written before F-X007 lands must not
 * assume stack-only input" actionable.
 */
export const STACK_KIND = "stack";
export const VOLUME_KIND = "volume-reformat";

/**
 * The digest of the frame bytes, checked against what the page computed.
 *
 * The page hashes the frame before base64 encoding it out of the browser and
 * this hashes what arrived. A silent truncation between the two would leave
 * F-011 comparing against a frame nobody rendered.
 *
 * `id` is the output name rather than a corpus path, because a volume reformat
 * has one and is not a row.
 */
export function assertFrameIntegrity(id, raw, result) {
  const digest = digestOf(raw);
  if (digest !== result.frame.sha256) {
    throw new Error(
      `${id}: the page hashed its frame as ${result.frame.sha256} and ` +
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
    kind: STACK_KIND,
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
    metadataSources: result.metadataSources,
    cornerstoneMetadata: result.cornerstoneMetadata,
    image: result.image,
    frame: {
      width: result.frame.width,
      height: result.frame.height,
      format: "RGBA8, top row first, as ImageData from the viewport canvas",
      sha256: result.frame.sha256,
      statistics: result.frame.statistics,
    },
    // How many canvas pixels one source pixel covers, per axis, for EVERY row
    // and not only for the two the run lists under `downsampled`. The
    // derivation lives once, in `canvasScale`, and publishing it here is what
    // stops F-011 writing a second copy of it in Rust. HLD section 18's rule
    // about the LUT chain existing exactly once is about arithmetic living in
    // one place, and it generalises.
    canvasPixelsPerSourcePixel: sourcePixelScale(params, result),
    reference: {
      cornerstone3D: installed["@cornerstonejs/core"],
      dicomImageLoader: installed["@cornerstonejs/dicom-image-loader"],
      browser: environment.versions.userAgent,
      adapter: environment.rendering.renderer,
    },
    files: { raw: `${id}.raw`, png: `${id}.png` },
  };
}

/**
 * The per-axis magnification `run.json` also reports for the decimated rows.
 *
 * Rounded to six decimals, matching the `downsampled` entries, so the two
 * readings of one number cannot differ in their last digits and send somebody
 * looking for a cause.
 */
function sourcePixelScale(params, result) {
  const plane = result.cornerstoneMetadata?.imagePlaneModule ?? {};
  const scale = canvasScale({
    parallelScale: result.camera.parallelScale,
    canvasHeight: params.canvas.height,
    rowPixelSpacing: plane.rowPixelSpacing,
    columnPixelSpacing: plane.columnPixelSpacing,
  });
  return {
    vertical: Number(scale.vertical.toFixed(6)),
    horizontal: Number(scale.horizontal.toFixed(6)),
  };
}

/**
 * Assemble one volume reformat's sidecar. F-X007.
 *
 * Additive to the stack sidecar's shape: `renderParams`, `voi`, `camera`,
 * `frame`, `reference` and `files` keep their meanings, so F-011's readers for
 * those are reusable unchanged. Everything new sits under `volume` and
 * `reformat`.
 *
 * **The `volume` block is repeated identically in all three orientation
 * sidecars for a subject, and that is deliberate.** The existing design's rule
 * is that a frame never travels alone, and a comparator that had to join two
 * files to explain one frame would break it.
 */
export function buildVolumeSidecar({
  subject,
  record,
  frame,
  measured,
  comparison,
  volumeParams,
  renderParams,
  environment,
  installed,
}) {
  const id = frameIdFor(subject.id, frame.orientation);
  const { matched, modalityVoiDefaults, ...declaredRenderParams } = renderParams;
  void matched;
  void modalityVoiDefaults;

  return {
    kind: VOLUME_KIND,
    volume: {
      id: subject.id,
      seriesDirectory: subject.seriesDirectory,
      why: subject.why,
      members: record.members.map((member, index) => ({
        path: member.path,
        sha256: member.sha256,
        imageId: record.memberImageIds?.[index] ?? null,
        // The stack frame of the same instance, so F-011 can join a reformat
        // to the two-dimensional renders it was assembled from.
        stackSidecar: member.stackSidecar,
        imagePositionPatient: member.attributes?.imagePositionPatient ?? null,
        imageOrientationPatient: member.attributes?.imageOrientationPatient ?? null,
        pixelSpacing: member.attributes?.pixelSpacing ?? null,
        sliceThickness: member.attributes?.sliceThickness ?? null,
      })),
      referenceSortedImageIds: record.referenceSortedImageIds ?? null,
      referenceGeometry: record.referenceGeometry ?? null,
      measuredGeometry: {
        source: "tools/oracle/src/geometry.mjs, from the files and never from a cornerstone3D module",
        citation: "PS3.3 C.7.6.2.1.1",
        normal: measured.normal,
        order: measured.order.map((entry) => entry.path),
        projectionsMm: measured.projectionsMm,
        gapsMm: measured.gapsMm,
        meanGapMm: measured.meanGapMm,
        medianGapMm: measured.medianGapMm,
        minGapMm: measured.minGapMm,
        maxGapMm: measured.maxGapMm,
        maxDeviationFromMeanMm: measured.maxDeviationFromMeanMm,
        uniform: comparison.uniform,
        toleranceMm: comparison.truth.toleranceMm,
        voxelAxes: measured.voxelAxes,
      },
      truth: comparison.truth,
      referenceAgreesWithTruth: comparison.referenceAgreesWithTruth,
      referenceDivergence: comparison.referenceDivergence,
      zProfile: record.zProfile,
    },
    reformat: frame.reformat,
    volumeParams: {
      orientations: volumeParams.orientations,
      blendMode: volumeParams.blendMode,
      slabThicknessMm: volumeParams.slabThicknessMm,
      cameraMode: volumeParams.cameraMode,
      loadTimeoutMs: volumeParams.loadTimeoutMs,
    },
    renderParams: declaredRenderParams,
    voi: record.voi,
    // One volume viewport carries ONE voiRange, so the window that acted comes
    // from one member and every member's own resolved window is recorded here.
    // `real/mr_eay131` carries fifteen different windows, one per instance, and
    // a sidecar that published only the one that acted would read as though
    // they all agreed.
    voiMembers: {
      member: record.voi?.member ?? null,
      agree: record.voiMembersAgree ?? null,
      windows: record.memberWindows ?? null,
    },
    camera: frame.camera,
    frame: {
      width: frame.frame.width,
      height: frame.frame.height,
      format: "RGBA8, top row first, as ImageData from the viewport canvas",
      sha256: frame.frame.sha256,
      statistics: frame.frame.statistics,
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
