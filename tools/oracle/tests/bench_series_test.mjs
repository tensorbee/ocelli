// Unit tests for the comparator page's series grouping.
//
// The F-223 comparator page groups a folder's parsed files exactly as the
// folder viewer does, because the first image the pixel precondition compares
// is "the selected series' first image by the shared ordering", and an
// ordering that disagrees between the sides would compare two different
// pictures. The rules below are transcribed from
// `examples/folder-viewer/src/page.html` `groupSeries` (F-223 plan, fact 4):
// group by Series Instance UID with a per-file fallback key, order series by
// series number then description then key, order images by instance number
// then file name then frame, and select the series with the most images,
// earliest on a tie.
//
// Every expected order below is hand-derived from those rules, not read back
// from the implementation.

import test from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import vm from "node:vm";
import { repoPath } from "../src/paths.mjs";

import { groupBenchSeries, selectLargestSeries, numericValue, loaderVoiValue } from "../src/bench-series.mjs";

/** A stand-in for a browser `File`: the grouping reads `name` only. */
const file = (name) => ({ name, size: 10 });

const row = (over = {}) => ({
  file: file("a.dcm"),
  imageId: "dicomfile:0",
  seriesUid: "1.2.3",
  seriesNumber: 1,
  seriesDescription: "Axial",
  instanceNumber: 1,
  frames: 1,
  ...over,
});

// ---------------------------------------------------------------------------
// numericValue, the folder viewer's numeric()
// ---------------------------------------------------------------------------

test("numericValue sends absent and unparseable numbers last", () => {
  assert.equal(numericValue("7"), 7);
  assert.equal(numericValue("12.5"), 12.5);
  assert.equal(numericValue(0), 0, "a series number of zero sorts first, not last");
  assert.equal(numericValue(""), Number.POSITIVE_INFINITY);
  assert.equal(numericValue(undefined), Number.POSITIVE_INFINITY);
  assert.equal(numericValue(null), Number.POSITIVE_INFINITY);
  assert.equal(numericValue("abc"), Number.POSITIVE_INFINITY);
});

// ---------------------------------------------------------------------------
// Grouping
// ---------------------------------------------------------------------------

test("files sharing a Series Instance UID land in one series", () => {
  const seriesList = groupBenchSeries([
    row({ file: file("a.dcm"), imageId: "dicomfile:0", instanceNumber: 2 }),
    row({ file: file("b.dcm"), imageId: "dicomfile:1", instanceNumber: 1 }),
  ]);
  assert.equal(seriesList.length, 1);
  assert.equal(seriesList[0].key, "uid:1.2.3");
  assert.deepEqual(
    seriesList[0].images.map((image) => image.file.name),
    ["b.dcm", "a.dcm"],
    "images order by instance number before file name",
  );
});

test("a file with no Series Instance UID is its own series, keyed by its position", () => {
  const seriesList = groupBenchSeries([
    row({ imageId: "dicomfile:0", seriesUid: undefined }),
    row({ file: file("b.dcm"), imageId: "dicomfile:1", seriesUid: undefined }),
  ]);
  assert.equal(seriesList.length, 2);
  assert.deepEqual(
    seriesList.map((series) => series.key),
    ["file:0", "file:1"],
  );
});

test("series order by number, then description, then key", () => {
  const seriesList = groupBenchSeries([
    row({ seriesUid: "1.2.10", seriesNumber: 10, seriesDescription: "Z" }),
    row({ file: file("b.dcm"), imageId: "dicomfile:1", seriesUid: "1.2.2", seriesNumber: 2, seriesDescription: "Z" }),
    row({ file: file("c.dcm"), imageId: "dicomfile:2", seriesUid: "1.2.30", seriesNumber: "not-a-number", seriesDescription: "B" }),
    row({ file: file("d.dcm"), imageId: "dicomfile:3", seriesUid: "1.2.40", seriesNumber: undefined, seriesDescription: "A" }),
  ]);
  assert.deepEqual(
    seriesList.map((series) => series.key),
    [
      "uid:1.2.2",
      "uid:1.2.10",
      "uid:1.2.40",
      "uid:1.2.30",
    ],
    "2 before 10 numerically; the unparseable and absent numbers go last and break their tie on description",
  );
});

test("images order by instance number, then file name, then frame", () => {
  const seriesList = groupBenchSeries([
    row({ file: file("same.dcm"), imageId: "dicomfile:0", instanceNumber: 3 }),
    row({ file: file("same.dcm"), imageId: "dicomfile:1", instanceNumber: 1 }),
    row({ file: file("aaa.dcm"), imageId: "dicomfile:2", instanceNumber: 2 }),
    row({ file: file("zzz.dcm"), imageId: "dicomfile:3", instanceNumber: 2 }),
    row({ file: file("no-number.dcm"), imageId: "dicomfile:4", instanceNumber: undefined }),
  ]);
  assert.deepEqual(
    seriesList[0].images.map((image) => image.file.name),
    [
      // Instance 1, then the two instance-2 files by name, then 3, then the
      // absent instance number last.
      "same.dcm",
      "aaa.dcm",
      "zzz.dcm",
      "same.dcm",
      "no-number.dcm",
    ],
  );
});

test("a multi-frame file expands to one entry per frame", () => {
  const seriesList = groupBenchSeries([
    row({ imageId: "dicomfile:0", frames: 3, instanceNumber: 4 }),
  ]);
  assert.equal(seriesList[0].images.length, 3);
  assert.deepEqual(
    seriesList[0].images.map((image) => image.imageId),
    ["dicomfile:0", "dicomfile:0?frame=2", "dicomfile:0?frame=3"],
    "frame 0 is the bare image id and later frames use the loader's one-based frame suffix",
  );
  assert.deepEqual(seriesList[0].images.map((image) => image.frame), [0, 1, 2]);
  assert.ok(seriesList[0].images.every((image) => image.frames === 3));
});

test("a frame count below two means one frame", () => {
  for (const frames of [undefined, 0, 1, -4, Number.NaN]) {
    const seriesList = groupBenchSeries([row({ frames })]);
    assert.equal(
      seriesList[0].images.length,
      1,
      `frames ${String(frames)} must list one entry`,
    );
  }
});

// ---------------------------------------------------------------------------
// Selection
// ---------------------------------------------------------------------------

test("the series with the most images is selected, earliest on a tie", () => {
  const seriesList = groupBenchSeries([
    row({ seriesUid: "1.2.small", imageId: "dicomfile:0" }),
    row({ file: file("b.dcm"), imageId: "dicomfile:1", seriesUid: "1.2.big", instanceNumber: 1 }),
    row({ file: file("c.dcm"), imageId: "dicomfile:2", seriesUid: "1.2.big", instanceNumber: 2 }),
    row({ file: file("d.dcm"), imageId: "dicomfile:3", seriesUid: "1.2.big2", instanceNumber: 1 }),
    row({ file: file("e.dcm"), imageId: "dicomfile:4", seriesUid: "1.2.big2", instanceNumber: 2 }),
  ]);
  assert.equal(selectLargestSeries(seriesList).key, "uid:1.2.big");
});

test("an empty series list selects nothing", () => {
  assert.equal(selectLargestSeries([]), null);
  assert.equal(selectLargestSeries(groupBenchSeries([])), null);
});


test("loader VOI adaptation wraps recognized selectors and preserves typed alternatives", () => {
  for (const value of ["LINEAR", "LINEAR_EXACT", "SIGMOID"]) {
    assert.deepEqual(loaderVoiValue(value), [value]);
    assert.equal(loaderVoiValue(value)[0], value, "loader [0] reads the whole selector");
  }
  const array = ["LINEAR"];
  assert.equal(loaderVoiValue(array), array);
  for (const value of [undefined, null, 1, {}, "L", "OTHER", "", "linear"]) assert.equal(loaderVoiValue(value), value);
});


test("frame cap preserves the actual viewer's expansion and selected first series", () => {
  const page = readFileSync(repoPath("examples", "folder-viewer", "src", "page.html"), "utf8");
  const cap = Number(page.match(/^const FRAME_LIST_CAP = (\d+);$/m)?.[1]);
  assert.equal(cap, 65536, "the consumer fixture names the viewer's existing policy");
  // Execute the actual pure consumer functions, without substituting a model
  // of their expansion or ordering. Nested closing braces remain indented.
  const source = ["groupSeries", "numeric", "drawable"].map(name => {
    const found = page.match(new RegExp("^function " + name + "\\([^)]*\\) \\{[\\s\\S]*?^\\}", "m"));
    assert(found, "the actual folder grouping function is present");
    return found[0];
  }).join("\n");
  const actual = vm.createContext({ FRAME_LIST_CAP: cap, seriesList: [], seriesByKey: new Map() });
  vm.runInContext(source, actual);
  const inputs = [row({ file: file("first.dcm"), seriesUid: "2.25.223401", seriesNumber: "1", frames: cap + 1 }),
    row({ file: file("second.dcm"), imageId: "dicomfile:1", seriesUid: "2.25.223402", seriesNumber: "2", frames: cap + 2 })];
  const nativeRows = inputs.map(input => ({ file: input.file, info: { seriesUid: input.seriesUid,
    seriesNumber: input.seriesNumber, seriesDescription: input.seriesDescription,
    instanceNumber: input.instanceNumber, frames: input.frames, monochrome: true } }));
  actual.found = nativeRows;
  vm.runInContext("groupSeries(found)", actual);
  const native = Array.from(actual.seriesList);
  const compared = groupBenchSeries(inputs);
  assert.deepEqual(native.map(series => series.images.length), [65536, 65536]);
  assert.deepEqual(native.map(series => series.omitted), [1, 2]);
  assert.deepEqual(compared.map(series => series.images.length), [65536, 65536]);
  assert.deepEqual(compared.map(series => series.omitted), [1, 2]);
  let selected = native[0];
  for (const series of native.slice(1)) if (series.images.length > selected.images.length) selected = series;
  assert.equal(selectLargestSeries(compared).images[0].file, selected.images[0].file);
  assert.equal(selectLargestSeries(compared).number, "1");
  assert.equal(compared[0].images.at(-1).frame, 65535);
  const combined = groupBenchSeries(inputs.map(input => ({ ...input, seriesUid: "2.25.223403" })));
  assert.equal(combined.length, 1);
  assert.equal(combined[0].images.length, 131072, "the cap is per file, not per series");
  assert.equal(combined[0].omitted, 3);
});
