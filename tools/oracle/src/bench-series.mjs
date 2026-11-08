// Series grouping for the F-223 comparator page.
//
// This module runs on BOTH sides of the harness, like `voi.mjs` does for the
// reference render: `build-page.mjs` bundles it into `page/bench.mjs`, and the
// unit tests import the same file under node. The ordering the comparator
// page presents is therefore the ordering these tests exercise.
//
// The rules are the folder viewer's own, transcribed from
// `examples/folder-viewer/src/page.html` `groupSeries` and recorded as fact 4
// of `.claude/plans/F-223-design.md`: group by Series Instance UID with a
// per-file fallback key, order series by series number then description then
// key, order images by instance number then file name then frame, and select
// the series with the most images, earliest on a tie. The comparator page and
// the folder viewer must agree on all of it, because the pixel precondition
// compares "the selected series' first image by the shared ordering" and an
// ordering that disagrees between the sides compares two different pictures.
//
// The existing frame-count number/default interpretation remains unchanged.
// Expansion uses the folder viewer's per-file cap and keeps the omitted count,
// because listing more frames would change both the workload and tie selection.
// Modality is not carried: the comparator page has no series list to label.

const FRAME_LIST_CAP = 65536;

/**
 * A file record the comparator page assembles after the loader parsed it.
 *
 * @typedef {object} BenchFileRow
 * @property {File} file The browser File object, held for `name` and `size`.
 * @property {string} imageId The wadouri image id `fileManager.add` returned.
 * @property {string | undefined} seriesUid Series Instance UID (0020,000E).
 * @property {string | number | undefined} seriesNumber Series Number (0020,0011).
 * @property {string | undefined} seriesDescription Series Description (0008,103E).
 * @property {string | number | undefined} instanceNumber Instance Number (0020,0013).
 * @property {number | undefined} frames Frames per file, PS3.3 C.7.6.6.
 */

/**
 * @typedef {object} BenchImageEntry
 * @property {File} file
 * @property {string} imageId The frame-addressed image id.
 * @property {number} frame Zero-based frame index within the file.
 * @property {number} frames The file's own frame count.
 * @property {string | number | undefined} instanceNumber
 */

/**
 * @typedef {object} BenchSeries
 * @property {string} key The grouping key, `uid:` or `file:` prefixed.
 * @property {string | number} number Series Number, or "" when absent.
 * @property {string} description Series Description, or "" when absent.
 * @property {BenchImageEntry[]} images Ordered images.
 * @property {number} omitted Frames omitted by the per-file listing cap.
 */

/**
 * The folder viewer's `numeric`: absent or unparseable text sorts last.
 *
 * Exported because the ordering rule lives here and the tests derive their
 * expectations from the rule's own words, not from this function.
 *
 * @param {string | number | null | undefined} text
 * @returns {number}
 */
export function numericValue(text) {
  if (text === "" || text == null) {
    return Number.POSITIVE_INFINITY;
  }
  const value = Number(text);
  return Number.isFinite(value) ? value : Number.POSITIVE_INFINITY;
}

/**
 * Group parsed file records into ordered series.
 *
 * @param {BenchFileRow[]} rows One row per parsed file, in folder order.
 * @returns {BenchSeries[]}
 */
export function groupBenchSeries(rows) {
  const map = new Map();
  rows.forEach((row, order) => {
    const key = row.seriesUid ? "uid:" + row.seriesUid : "file:" + order;
    let series = map.get(key);
    if (!series) {
      series = {
        key,
        number: row.seriesNumber ?? "",
        description: row.seriesDescription ?? "",
        images: [],
        omitted: 0,
      };
      map.set(key, series);
    }
    const reported = Number(row.frames);
    const listed =
      Number.isFinite(reported) && reported > 1 ? Math.floor(reported) : 1;
    const steps = Math.min(listed, FRAME_LIST_CAP);
    series.omitted += listed - steps;
    for (let frameIndex = 0; frameIndex < steps; frameIndex += 1) {
      // Frame 0 is the bare image id. Later frames use the loader's own
      // one-based `?frame=` suffix, which `parseImageId` turns back into a
      // zero-based frame index against the same cached data set.
      const imageId =
        frameIndex === 0 ? row.imageId : `${row.imageId}?frame=${frameIndex + 1}`;
      series.images.push({
        file: row.file,
        imageId,
        frame: frameIndex,
        frames: listed,
        instanceNumber: row.instanceNumber,
      });
    }
  });
  const seriesList = Array.from(map.values());
  seriesList.sort(
    (a, b) =>
      numericValue(a.number) - numericValue(b.number) ||
      a.description.localeCompare(b.description) ||
      a.key.localeCompare(b.key),
  );
  for (const series of seriesList) {
    series.images.sort(
      (a, b) =>
        numericValue(a.instanceNumber) - numericValue(b.instanceNumber) ||
        a.file.name.localeCompare(b.file.name) ||
        a.frame - b.frame,
    );
  }
  return seriesList;
}

/**
 * The series with the most images, earliest on a tie.
 *
 * @param {BenchSeries[]} seriesList
 * @returns {BenchSeries | null}
 */
export function selectLargestSeries(seriesList) {
  let best = null;
  for (const series of seriesList) {
    if (best === null || series.images.length > best.images.length) {
      best = series;
    }
  }
  return best;
}


/** Preserve the valid PS3.3 C.11.2 selector through loader5.8.2's [0] read.
 * The installed wadouri provider returns a string, while createImage expects
 * an array. Unknown strings and existing arrays remain unchanged. */
export function loaderVoiValue(value) {
  return ["LINEAR", "LINEAR_EXACT", "SIGMOID"].includes(value) ? [value] : value;
}
