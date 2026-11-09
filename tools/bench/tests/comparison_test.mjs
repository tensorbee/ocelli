// The comparison mode's pure logic, against hand-computed fixtures.
//
// Every expected number here is worked out from the F-223 design plan and
// HLD 25.1's text and NOT from running the implementation, per HLD 27.2 R2.
// An agent asked to test a function will assert what it does rather than
// what it should do. The fixture inputs are inline: the ps text, the real
// footprint output captured from `/usr/bin/footprint -f bytes` on this host
// with the process names made neutral, and header records built field by
// field. The one cryptographic expectation, the input identity, is computed
// in this file by hashing the pinned byte string directly, so the test
// states the format instead of copying the code's answer.

import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import { createHash } from "node:crypto";
import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import test from "node:test";

import {
  ACKNOWLEDGEMENT,
  ARCHITECTURE,
  CORNERSTONE,
  FOOTPRINT_PERIOD,
  NOT_SHOWN,
  OCELLI,
  SESSION_WORKLOAD,
  SIDES,
  WORKLOAD_IDS,
  cpuFigure,
  recordCondition,
  inputIdentity,
  loadComparison,
  memoryFigures,
  otherBrowserJobs,
  parseFootprint,
  parseProcessTree,
  publishedRecord,
  renderReport,
  renderSummary,
  scanForDenied,
  scanRecordForDenied,
  selectWorkloads,
  separated,
  summarise,
  tierSentence,
  trialOrder,
} from "../src/comparison.mjs";

const identity = (digests) =>
  createHash("sha256").update(digests.sort().join("\n") + "\n").digest("hex");

// AC5. The plan's own hand-computed sets: [1, 2, 3, 4, 5, 6, 7, 8, 9] gives
// median 5, p25 3, p75 7, because (9 - 1) * 0.25 is exactly 2 and (9 - 1) *
// 0.75 is exactly 6, both integral indices into the sorted list.
test("quartiles are linear between order statistics", () => {
  const nine = summarise([9, 3, 1, 7, 5, 8, 2, 6, 4], 9);
  assert.equal(nine.state, "measured");
  assert.equal(nine.n, 9);
  assert.equal(nine.median, 5);
  assert.equal(nine.p25, 3);
  assert.equal(nine.p75, 7);
  assert.equal(nine.min, 1);
  assert.equal(nine.max, 9);

  // [10, 20, 30, 40]: (4 - 1) * 0.25 = 0.75, so p25 is 10 + 0.75 * 10 =
  // 17.5, and (4 - 1) * 0.75 = 2.25, so p75 is 30 + 0.25 * 10 = 32.5. The
  // median is the mean of the middle two, 25. Nearest-rank quartiles would
  // answer 10 and 30 here, which is exactly the mutation this case kills.
  const four = summarise([40, 10, 30, 20], 4);
  assert.equal(four.state, "measured");
  assert.equal(four.median, 25);
  assert.equal(four.p25, 17.5);
  assert.equal(four.p75, 32.5);
  assert.equal(four.min, 10);
  assert.equal(four.max, 40);
});

test("a metric with fewer valid trials than N is invalid and has no median", () => {
  const invalid = summarise([1, 2, 3], 9);
  assert.equal(invalid.state, "invalid");
  assert.equal(invalid.n, 3);
  assert.equal(Object.hasOwn(invalid, "median"), false,
    "an invalid metric has no median to print");
  assert.equal(Object.hasOwn(invalid, "p25"), false);
  assert.equal(Object.hasOwn(invalid, "p75"), false);
  assert.equal(summarise([], 9).state, "invalid");
  assert.equal(summarise([], 9).n, 0);
  assert.throws(() => summarise([1, NaN], 2), /finite/);
  assert.throws(() => summarise([1, 2], 0), /declared N/);
});

// AC6. The interquartile ranges [10, 20] and [30, 40] do not overlap. The
// touching case is the boundary the plan names: p75(a) == p25(b) is NOT
// separated, because ranges that share a point are not disjoint. The
// overlapping case has medians 20 and 30, far apart, and is still not
// separated, which is what kills a rule on medians alone.
test("a difference is stated only when spreads separate", () => {
  const low = { state: "measured", n: 9, median: 15, p25: 10, p75: 20 };
  const high = { state: "measured", n: 9, median: 35, p25: 30, p75: 40 };
  assert.equal(separated(low, high), true);
  assert.equal(separated(high, low), true);
  const touchingHigh = { state: "measured", n: 9, median: 35, p25: 20, p75: 40 };
  assert.equal(separated(low, touchingHigh), false,
    "p75(a) == p25(b) is a touch, not a separation");
  const overlapping = { state: "measured", n: 9, median: 30, p25: 12, p75: 32 };
  assert.equal(separated(low, overlapping), false,
    "the medians differ by 15 and the spreads still overlap");
  const invalid = { state: "invalid", n: 3 };
  assert.throws(() => separated(low, invalid), /invalid/);
  assert.throws(() => separated(low, { state: "measured", n: 9 }), /p25/);
});

// AC7. Nine trials starting from ocelli alternate ocelli, cornerstone,
// ocelli. The starting side is the caller's, because the plan flips it
// between workloads so slow drift lands on both sides equally.
test("trials alternate sides", () => {
  assert.deepEqual(trialOrder(SIDES, 5, OCELLI),
    [OCELLI, CORNERSTONE, OCELLI, CORNERSTONE, OCELLI]);
  assert.deepEqual(trialOrder(SIDES, 5, CORNERSTONE),
    [CORNERSTONE, OCELLI, CORNERSTONE, OCELLI, CORNERSTONE]);
  assert.deepEqual(trialOrder(SIDES, 1, CORNERSTONE), [CORNERSTONE]);
  assert.throws(() => trialOrder(SIDES, 5, "chrome"), /startSide/);
  assert.throws(() => trialOrder(SIDES, 0, OCELLI), /trial count/);
  assert.throws(() => trialOrder(["ocelli", "ocelli"], 3, OCELLI), /two sides/);
});

// AC8, first figure. The root and every descendant to depth 3, and not the
// sibling's subtree. A parser that stopped at direct children would answer
// {100, 101} and miss 102 and 103.
const TREE_PS_TEXT = [
  "    1     0",
  "  100     1",
  "  101   100",
  "  102   101",
  "  103   102",
  "  200     1",
  "  201   200",
].join("\n");

test("memory counts every process in the inventory", () => {
  const tree = parseProcessTree(TREE_PS_TEXT, 100);
  assert.deepEqual([...tree].sort((a, b) => a - b), [100, 101, 102, 103]);
  assert.equal(tree.has(200), false, "a sibling of the root is not ours");
  assert.equal(tree.has(201), false, "nor the sibling's children");
  assert.equal(tree.has(1), false, "nor the ancestor every process shares");
  assert.deepEqual([...parseProcessTree("", 100)], [100],
    "a root absent from the text has no descendants to add");
  // The review's probe: a root named only as a parent is still the root of
  // its subtree, which is what the doc sentence now says.
  assert.deepEqual([...parseProcessTree("  200   100\n", 100)], [100, 200],
    "a root named only as a parent still gathers its children");
});

// AC8, second figure. The REAL shape of
// `/usr/bin/footprint -f bytes -p <pid> -p <pid>`, captured on this host
// from two throwaway processes (the shell this ran in and a `sleep 30`
// stopped by its pid afterwards), with the process names
// replaced by neutral names and nothing else changed. `-f bytes` alone
// prints phys_footprint and phys_footprint_peak, per process, under
// `Auxiliary data:`, and no other flag is needed. The text also carries
// `Shared with` sub-blocks that name pids, a shared-cache section and a
// closing `Summary Footprint:` line, none of which is a process header.
const FOOTPRINT_TEXT = [
  '======================================================================',
  'process-a [30321]: 64-bit    Footprint: 1982920 B (16384 bytes per page)',
  '======================================================================',
  '',
  '      Dirty         Clean   Reclaimable    Regions    Category',
  '        ---           ---           ---        ---    ---',
  '   606208 B           0 B           0 B          9    Malloc Metadata',
  '   409600 B           0 B           0 B          5    Malloc Small',
  '   328136 B           0 B           0 B          1    page table',
  '   119293 B           0 B           0 B         18    __DATA',
  '   117216 B           0 B           0 B         19    __DATA_DIRTY',
  '   107931 B           0 B           0 B         16    unused dyld shared cache area',
  '    81920 B           0 B           0 B          1    Malloc Tiny',
  '    32768 B           0 B           0 B          4    Untagged',
  '    32768 B           0 B           0 B          2    Stack',
  '    31616 B           0 B           0 B          1    __OBJC_RW',
  '    16384 B           0 B           0 B          1    Deferred Reclaim',
  '    16384 B           0 B           0 B          1    Malloc Nano',
  '    16384 B           0 B           0 B          1    OS Alloc Once',
  '    16384 B           0 B           0 B          1    Activity Tracing',
  '    16312 B           0 B           0 B          3    __TPRO_CONST',
  '      848 B           0 B           0 B          5    __AUTH',
  '        0 B      540672 B           0 B          1    __TEXT',
  '        0 B       16384 B           0 B          1    __DATA_CONST',
  '        0 B       16384 B           0 B          1    __LINKEDIT',
  '        0 B       16384 B           0 B          1    mapped file',
  '        ---           ---           ---        ---    ---',
  '  1950152 B      589824 B           0 B         94    TOTAL',
  '',
  'Shared with process-b [30322]:',
  '      Dirty         Clean   Reclaimable    Regions    Category',
  '        ---           ---           ---        ---    ---',
  '    32768 B           0 B           0 B          1    Untagged',
  '        0 B           0 B           0 B          1    mapped file',
  '        ---           ---           ---        ---    ---',
  '    32768 B           0 B           0 B          2    TOTAL',
  '',
  'Auxiliary data:',
  '    phys_footprint: 1982920 B',
  '    phys_footprint_peak: 1982920 B',
  '',
  '======================================================================',
  'process-b [30322]: 64-bit    Footprint: 1229040 B (16384 bytes per page)',
  '======================================================================',
  '',
  '      Dirty         Clean   Reclaimable    Regions    Category',
  '        ---           ---           ---        ---    ---',
  '   557056 B           0 B           0 B          9    Malloc Metadata',
  '   180464 B           0 B           0 B          1    page table',
  '   117168 B           0 B           0 B         18    __DATA_DIRTY',
  '    65436 B           0 B           0 B         14    __DATA',
  '    60284 B           0 B           0 B         12    unused dyld shared cache area',
  '    49152 B           0 B           0 B          3    Malloc Small',
  '    32768 B           0 B           0 B          1    Malloc Tiny',
  '    32768 B           0 B           0 B          2    Stack',
  '    31616 B           0 B           0 B          1    __OBJC_RW',
  '    20408 B           0 B           0 B          4    __TPRO_CONST',
  '    16384 B           0 B           0 B          1    Deferred Reclaim',
  '    16384 B           0 B           0 B          1    OS Alloc Once',
  '    16384 B           0 B           0 B          1    Malloc Nano',
  '        0 B       16384 B           0 B          1    __TEXT',
  '        0 B       16384 B           0 B          1    __DATA_CONST',
  '        0 B       16384 B           0 B          1    __LINKEDIT',
  '        0 B           0 B           0 B          1    Dyld Private Memory',
  '        0 B           0 B           0 B          4    Untagged',
  '        ---           ---           ---        ---    ---',
  '  1196272 B       49152 B           0 B         78    TOTAL',
  '',
  'Shared with process-a [30321]:',
  '      Dirty         Clean   Reclaimable    Regions    Category',
  '        ---           ---           ---        ---    ---',
  '    32768 B           0 B           0 B          1    Untagged',
  '        0 B           0 B           0 B          1    mapped file',
  '        ---           ---           ---        ---    ---',
  '    32768 B           0 B           0 B          2    TOTAL',
  '',
  'Auxiliary data:',
  '    phys_footprint: 1229040 B',
  '    phys_footprint_peak: 1261808 B',
  '',
  '======================================================================',
  'Shared Cache C284A291-B68B-3D0D-A1A0-3F866F87EFDC',
  '======================================================================',
  '',
  'Shared with process-a [30321], process-b [30322]:',
  '      Dirty         Clean   Reclaimable    Regions    Category',
  '        ---           ---           ---        ---    ---',
  '        0 B           0 B           0 B         23    __AUTH',
  '        0 B           0 B           0 B         33    __DATA_DIRTY',
  '        0 B           0 B           0 B        109    __DATA',
  '        0 B           0 B           0 B         83    __AUTH_CONST',
  '        0 B           0 B           0 B         78    __DATA_CONST',
  '        0 B           0 B           0 B        167    __TEXT',
  '        0 B           0 B           0 B        354    unused dyld shared cache area',
  '        0 B           0 B           0 B          2    __LINKEDIT',
  '        0 B           0 B           0 B          1    __TPRO_CONST',
  '        0 B           0 B           0 B          2    __OBJC_RO',
  '        ---           ---           ---        ---    ---',
  '        0 B           0 B           0 B        852    TOTAL',
  '',
  '======================================================================',
  'Summary Footprint: 3179192 B',
  '======================================================================',
  '',
  '      Dirty         Clean   Reclaimable    Regions    Category',
  '        ---           ---           ---        ---    ---',
  '  1163264 B           0 B           0 B         18    Malloc Metadata',
  '   508600 B           0 B           0 B          2    page table',
  '   458752 B           0 B           0 B          8    Malloc Small',
  '   234384 B           0 B           0 B         70    __DATA_DIRTY',
  '   184729 B           0 B           0 B        141    __DATA',
  '   168215 B           0 B           0 B        382    unused dyld shared cache area',
  '   114688 B           0 B           0 B          2    Malloc Tiny',
  '    65536 B           0 B           0 B         10    Untagged',
  '    65536 B           0 B           0 B          4    Stack',
  '    63232 B           0 B           0 B          2    __OBJC_RW',
  '    36720 B           0 B           0 B          8    __TPRO_CONST',
  '    32768 B           0 B           0 B          2    Deferred Reclaim',
  '    32768 B           0 B           0 B          2    Malloc Nano',
  '    32768 B           0 B           0 B          2    OS Alloc Once',
  '    16384 B           0 B           0 B          1    Activity Tracing',
  '      848 B           0 B           0 B         28    __AUTH',
  '        0 B      557056 B           0 B        169    __TEXT',
  '        0 B       32768 B           0 B         80    __DATA_CONST',
  '        0 B       32768 B           0 B          4    __LINKEDIT',
  '        0 B       16384 B           0 B          3    mapped file',
  '        0 B           0 B           0 B         83    __AUTH_CONST',
  '        0 B           0 B           0 B          2    __OBJC_RO',
  '        0 B           0 B           0 B          1    Dyld Private Memory',
  '        ---           ---           ---        ---    ---',
  '  3179192 B      638976 B           0 B       1028    TOTAL',
].join("\n");

test("the footprint parse reads the recorded shape", () => {
  const figures = parseFootprint(FOOTPRINT_TEXT);
  assert.deepEqual([...figures.keys()].sort((a, b) => a - b), [30321, 30322]);
  assert.deepEqual(figures.get(30321), { footprint: 1982920, peak: 1982920 });
  assert.deepEqual(figures.get(30322), { footprint: 1229040, peak: 1261808 },
    "the captured peak differs from the footprint, so both lines are read");
  // The `Shared with` sub-block names a pid too. A parser keyed on a pid in
  // square brackets re-keys here and then refuses the real block as a
  // duplicate, which is the trap the real shape sets.
  assert.ok(FOOTPRINT_TEXT.includes("Shared with process-b [30322]:"),
    "the fixture carries a Shared with block");
  assert.throws(
    () => parseFootprint(
      "process-a [9]: 64-bit    Footprint: 1 B\n    phys_footprint: 1 B\n"),
    /phys_footprint_peak/,
    "a block with no peak line is refused per pid");
  assert.throws(() => parseFootprint("no process here at all\n"),
    /no process block/,
    "an input with no parsable process is a refusal, never an empty Map");
  assert.throws(() => parseFootprint(42),
    /takes footprint -f bytes output/,
    "a non-string has no lines to read");
  assert.throws(
    () => parseFootprint(
      "process-a [9]: 64-bit    Footprint: 1 B\n" +
      "    phys_footprint: 1 B\n" +
      "    phys_footprint_peak: 1 B\n" +
      "process-a [9]: 64-bit    Footprint: 2 B\n" +
      "    phys_footprint: 2 B\n" +
      "    phys_footprint_peak: 2 B\n"),
    /printed process 9 twice/,
    "the same pid twice is a malformed capture and not a fresh block");
  // The summary line resets the block, so a phys_footprint line after it
  // belongs to the de-duplicated total and to no process named above.
  const afterSummary =
    parseFootprint(`${FOOTPRINT_TEXT}\n    phys_footprint: 9999999 B\n`);
  assert.deepEqual(afterSummary.get(30322), { footprint: 1229040, peak: 1261808 },
    "nothing after the summary is read again into a process");
});

// AC8, third figure. Three samples 250 ms apart in which the peaks do not
// coincide. The per-sample footprint sums are 300, 310 and 280, so the
// sampled concurrent peak is 310. The largest peak ever seen for process
// 401 is 160, in sample two, and for 402 is 240, in sample one, so the sum
// of per-process peaks is 400. The lower bound is strictly below the upper
// one, which is the whole point of reporting both.
const SAMPLES = () => [
  new Map([
    [401, { footprint: 100, peak: 150 }],
    [402, { footprint: 200, peak: 240 }],
  ]),
  new Map([
    [401, { footprint: 120, peak: 160 }],
    [402, { footprint: 190, peak: 210 }],
  ]),
  new Map([
    [401, { footprint: 110, peak: 155 }],
    [402, { footprint: 170, peak: 205 }],
  ]),
];

test("the two memory figures are the lower and the upper bound", () => {
  const figures = memoryFigures(SAMPLES());
  assert.equal(figures.sampled_peak, 310,
    "the largest per-sample sum of phys_footprint");
  assert.equal(figures.sum_of_peaks, 400,
    "the sum over pids of the largest peak each ever reached");
  assert.ok(figures.sampled_peak < figures.sum_of_peaks,
    "a concurrent peak cannot exceed the sum of independent peaks");
  assert.throws(() => memoryFigures([]), /at least one sample/);
  assert.throws(() => memoryFigures([new Map()]),
    /non-empty pid to figures map/,
    "an empty sample invents a zero nothing measured");
  assert.throws(() => memoryFigures([new Map([[401,
    { footprint: NaN, peak: 150 }]])]),
    /no finite footprint and peak pair/,
    "a figure that is not finite cannot be summed into either bound");
});

// AC9. 1.25 to 1.75, 2.5 to 3.0 and 0.75 to 1.25 are three deltas of 0.5,
// so the session consumed 1.5 CPU seconds. A process that exits between
// the snapshots makes the figure invalid rather than summed over a
// changed set, and so does a counter that goes backwards.
test("cpu refuses a changed inventory", () => {
  const before = new Map([[101, 1.25], [102, 2.5], [103, 0.75]]);
  const after = new Map([[101, 1.75], [102, 3.0], [103, 1.25]]);
  assert.deepEqual(cpuFigure(before, after), { state: "measured", seconds: 1.5 });
  const exited = new Map([[101, 2.0], [102, 3.5]]);
  const outcome = cpuFigure(before, exited);
  assert.equal(outcome.state, "invalid");
  assert.match(outcome.reason, /1 exited/);
  const appeared = new Map([...after, [104, 0.5]]);
  assert.match(cpuFigure(before, appeared).reason, /1 appeared/);
  const backwards = new Map([[101, 1.0], [102, 3.0], [103, 1.25]]);
  assert.match(cpuFigure(before, backwards).reason, /backwards/);
  assert.throws(() => cpuFigure(before, new Map([[101, "x"]])), /CPU seconds/);
  assert.throws(() => cpuFigure(before, "three pids"),
    /is a pid to CPU seconds map/,
    "an inventory that is not a map has no pids to compare");
});

// AC4. The fixture's header records. One header record per file, with the
// attributes the rules read and nothing else.
const header = (over = {}) => ({
  source: "pseudo",
  path: "/pseudo/study-1/series-1/0001.dcm",
  digest: "dd1",
  seriesUid: "1.2.3",
  modality: "CT",
  rows: 512,
  columns: 512,
  photometric: "MONOCHROME2",
  hasWindowCenter: true,
  hasWindowWidth: true,
  ...over,
});

test("workload selection is by attribute", () => {
  // A qualifying two-image series comes first, so first-found order would
  // select it and the rule must select the largest instead. The largest CT
  // series has four images and lacks Window Width on one of them, so it
  // does not qualify at all and the three-image series is selected.
  const records = [];
  for (let index = 0; index < 4; index += 1) {
    records.push(header({
      path: `/pseudo/s2/img${index}.dcm`,
      digest: `big-${index}`,
      seriesUid: "2.2.2",
      hasWindowWidth: index !== 3,
    }));
  }
  for (let index = 0; index < 2; index += 1) {
    records.push(header({
      path: `/pseudo/s3/img${index}.dcm`,
      digest: `mid-${index}`,
      seriesUid: "3.3.3",
    }));
  }
  for (let index = 0; index < 3; index += 1) {
    records.push(header({
      path: `/pseudo/s1/img${index}.dcm`,
      digest: `small-${index}`,
      seriesUid: "1.1.1",
    }));
  }
  const selected = selectWorkloads(records);
  assert.equal(selected["pseudo-ct"].state, "selected");
  assert.equal(selected["pseudo-ct"].count, 3);
  assert.deepEqual(selected["pseudo-ct"].files,
    ["/pseudo/s1/img0.dcm", "/pseudo/s1/img1.dcm", "/pseudo/s1/img2.dcm"]);
});

test("a tie is broken by the smaller digest of the series' sorted digests", () => {
  // Both series qualify with two images each, so the rule is the smaller
  // input identity. The identities are computed here from the pinned
  // format, the SHA-256 of the sorted digests one per line, and compared
  // lexicographically, which is the plan's rule stated twice and not the
  // implementation's answer copied once.
  const build = (seriesUid, digests, tag) =>
    digests.map((digest, index) => header({
      path: `/pseudo/${tag}/img${index}.dcm`,
      digest,
      seriesUid,
    }));
  const seriesX = build("3.3.3", ["aaa", "bbb"], "x");
  const seriesY = build("4.4.4", ["ccc", "ddd"], "y");
  const winner = identity(["aaa", "bbb"]) < identity(["ccc", "ddd"])
    ? seriesX : seriesY;
  const loser = winner === seriesX ? seriesY : seriesX;
  const selected = selectWorkloads([...seriesX, ...seriesY]);
  assert.equal(selected["pseudo-ct"].state, "selected");
  assert.deepEqual(selected["pseudo-ct"].files, winner.map((one) => one.path),
    "the tie went to the series whose sorted digests hash smaller");
  assert.notDeepEqual(selected["pseudo-ct"].files, loser.map((one) => one.path));
});

test("the DX object with the most pixels is selected", () => {
  const records = [
    header({ path: "/pseudo/dx-small.dcm", digest: "dx-small", seriesUid: "",
      modality: "DX", rows: 1000, columns: 1000 }),
    header({ path: "/pseudo/dx-large.dcm", digest: "dx-large", seriesUid: "",
      modality: "DX", rows: 2330, columns: 2846 }),
  ];
  const selected = selectWorkloads(records);
  assert.equal(selected["pseudo-dx"].state, "selected");
  assert.deepEqual(selected["pseudo-dx"].files, ["/pseudo/dx-large.dcm"]);

  // A pixel tie is broken the same way, by the smaller identity of the
  // object's sorted content digests, which for one file is that file's own
  // digest hashed with a trailing newline.
  const tied = [
    header({ path: "/pseudo/dx-a.dcm", digest: "p1", seriesUid: "",
      modality: "DX", rows: 1000, columns: 1000 }),
    header({ path: "/pseudo/dx-b.dcm", digest: "p2", seriesUid: "",
      modality: "DX", rows: 1000, columns: 1000 }),
  ];
  const winner = identity(["p1"]) < identity(["p2"])
    ? "/pseudo/dx-a.dcm" : "/pseudo/dx-b.dcm";
  const tieSelected = selectWorkloads(tied);
  assert.deepEqual(tieSelected["pseudo-dx"].files, [winner]);
});

test("a rule that selects nothing is unavailable with its own sentence", () => {
  // A PT series and a CT series that is the wrong size. No CT series
  // qualifies, and nothing is substituted for one.
  const records = [
    header({ seriesUid: "5.5.5", modality: "PT", rows: 128, columns: 128 }),
    header({ seriesUid: "6.6.6", rows: 256, columns: 256 }),
  ];
  const selected = selectWorkloads(records);
  assert.equal(selected["pseudo-ct"].state, "unavailable");
  assert.match(selected["pseudo-ct"].reason,
    /the CT series with the most images/);
  assert.equal(selected["pseudo-dx"].state, "unavailable");
  assert.match(selected["pseudo-dx"].reason, /DX object with the most pixels/);
  assert.equal(selectWorkloads([])["pseudo-full"].state, "unavailable");
  assert.throws(() => selectWorkloads([{ source: "pseudo" }]),
    /is missing path/,
    "the first field the record lacks is the one named");
});

test("the synthetic workloads are the generated files and nothing else", () => {
  const records = [
    header({ source: "synthetic", path: "/synthetic/ct/0001.dcm",
      digest: "syn-ct-1", seriesUid: "9.9.9" }),
    header({ source: "synthetic", path: "/synthetic/ct/0002.dcm",
      digest: "syn-ct-2", seriesUid: "9.9.9" }),
    header({ source: "synthetic", path: "/synthetic/dx/0001.dcm",
      digest: "syn-dx-1", seriesUid: "", modality: "DX", rows: 3000,
      columns: 2900 }),
  ];
  const selected = selectWorkloads(records);
  assert.deepEqual(selected["synthetic-ct"].files,
    ["/synthetic/ct/0001.dcm", "/synthetic/ct/0002.dcm"]);
  assert.deepEqual(selected["synthetic-dx"].files, ["/synthetic/dx/0001.dcm"]);
  assert.equal(selected["pseudo-full"].state, "unavailable",
    "no pseudo record was read, so pseudo full is unavailable and not empty");
});

// The input identity is the SHA-256 of the sorted list of per-file content
// digests. Both orders of the same list hash to the same value, and the
// value is the hash of "a\nb\n", stated here in the test rather than copied
// from the code.
test("the input identity is the sha256 of the sorted digest list", () => {
  const expected = createHash("sha256").update("a\nb\n").digest("hex");
  assert.equal(inputIdentity(["a", "b"]), expected);
  assert.equal(inputIdentity(["b", "a"]), expected,
    "the list is sorted before hashing, so the order it arrived in is gone");
  assert.notEqual(inputIdentity(["a", "c"]), expected);
  assert.throws(() => inputIdentity([]), /at least one/);
  assert.throws(() => inputIdentity(["a", ""]),
    /every content digest is a non-empty string/,
    "an empty digest is a file whose content nobody recorded");
});

// AC12. One of each planted kind: a name, a UID, a date and a path
// component. The kinds come back and the values do not, because an error
// message that prints the hit is the leak the scan exists to refuse.
const DENY_SET = {
  patientName: ["DOE^JOHN"],
  patientId: ["PID-007"],
  birthDate: ["19700101"],
  accession: ["AC-1"],
  institution: ["General Hospital"],
  physician: ["DR^SPOCK"],
  uids: ["1.2.840.10008.3.1.2.3.4"],
  dates: ["20241105"],
  pathComponents: ["anon001"],
};

test("outputs refuse an identifying value", () => {
  const record = JSON.stringify({
    open: { note: "study of DOE^JOHN", uid: "1.2.840.10008.3.1.2.3.4",
      when: "20241105", path: "/anon001/series/img.dcm" },
    clean: "nothing identifying here at all",
  });
  const hits = scanForDenied(record, DENY_SET);
  const kinds = new Set(hits.map((hit) => hit.kind));
  assert.deepEqual(
    ["patientName", "uids", "dates", "pathComponents"].filter(
      (kind) => !kinds.has(kind)),
    [],
    `every planted kind was found, got ${JSON.stringify(hits)}`);
  const printed = JSON.stringify(hits);
  for (const value of ["DOE^JOHN", "1.2.840.10008.3.1.2.3.4", "20241105",
    "anon001"]) {
    assert.equal(printed.includes(value), false,
      "the hit list carries kinds and line numbers, never the value itself");
  }
  for (const hit of hits) {
    assert.equal(typeof hit.line, "number");
  }
  assert.deepEqual(scanForDenied("nothing to see", DENY_SET), []);
  // An empty header value must not match every line.
  assert.deepEqual(scanForDenied("anything", { patientName: [""] }), []);
  assert.throws(() => scanForDenied("x", { uids: "1.2" }), /not a list/);
  assert.throws(() => scanForDenied(42, DENY_SET),
    /takes the output's full text/,
    "a non-string has no lines to scan");
  assert.throws(() => scanForDenied("x", null),
    /takes the run's deny set/,
    "without the deny set there is nothing to scan against");
});

// The serialised and structured halves of the scan. A value carrying a
// backslash or a double quote is changed by JSON.stringify, while the
// renderer rewrites only a pipe to a backslashed pipe, so the text scan
// matches each value's JSON-escaped form as well as its raw one, and the
// record itself is scanned before serialisation.
test("a denied value is caught through JSON escaping and in the record", () => {
  const denySet = { physician: ["GENERAL\\HOSPITAL", 'DOE^JOHN"x'] };
  const record = {
    note: "rendered for GENERAL\\HOSPITAL",
    nested: { verdict: "the physician was DOE^JOHN\"x on the record" },
  };
  // The probe from the review: the raw value is in no line of the
  // serialised JSON, because the backslash and the quote are escaped.
  const serialised = JSON.stringify(record, null, 2);
  assert.equal(serialised.includes("GENERAL\\HOSPITAL"), false,
    "JSON.stringify has escaped the backslash out of the raw form");
  assert.deepEqual(scanForDenied(serialised, denySet), [
    { kind: "physician", line: 2 },
    { kind: "physician", line: 4 },
  ], "the escaped forms are matched, line by line");
  assert.deepEqual(scanRecordForDenied(record, denySet), [
    { kind: "physician", path: "note" },
    { kind: "physician", path: "nested.verdict" },
  ], "the structured record is scanned before serialisation, by path");
  const printed = JSON.stringify([
    ...scanForDenied(serialised, denySet),
    ...scanRecordForDenied(record, denySet),
  ]);
  assert.equal(printed.includes("GENERAL"), false,
    "a hit carries its kind and where, never the value");
  assert.deepEqual(scanRecordForDenied({ clean: "nothing" }, denySet), []);
  assert.deepEqual(scanRecordForDenied(["a", "GENERAL\\HOSPITAL"], denySet),
    [{ kind: "physician", path: "1" }]);
  assert.throws(() => scanRecordForDenied("text", denySet), /record/);
  assert.throws(() => scanRecordForDenied(record, { uids: "1.2" }),
    /not a list/);
  assert.throws(() => scanRecordForDenied(record, null),
    /takes the run's deny set/,
    "without the deny set there is nothing to scan against");
});

// The renderer is the third shape a denied value takes in an output. The
// renderer's one transform, escapeCell, runs in the version table's cells
// alone and rewrites ONLY a pipe to a backslashed pipe, so a denied value
// carrying a pipe in a table cell reaches the rendered summary as neither
// its raw form nor its JSON-escaped one, and the text scan matches that
// rendered form too. A reason or a renderer string reaches the summary raw,
// and the raw match still catches it.
test("a denied value carrying a pipe is caught in the rendered summary", () => {
  const denySet = { physician: ["A|B"] };
  const record = runRecord({
    versions: { ...runRecord().versions,
      "@cornerstonejs/core": "5.8.2 as reported by A|B" },
  });
  const summary = renderSummary(publishedRecord(record));
  assert.ok(summary.includes("A\\|B"),
    "the renderer rewrote the pipe, so the raw form is not what the text holds");
  assert.equal(summary.includes("A|B"), false,
    "and the raw form is gone from the rendered text");
  const hits = scanForDenied(summary, denySet);
  assert.deepEqual(hits.map((hit) => hit.kind), ["physician"],
    "the rendered form is matched, which a scan of the raw and JSON-escaped \
     forms alone misses");
  for (const hit of hits) {
    assert.equal(typeof hit.line, "number",
      "the hit carries where, never the value");
  }
});

// The published record's closed schema. The run record is bigger than the
// published subset and everything not in the schema stays behind.
const cell = (over = {}) => ({
  state: "measured", n: 9, median: 10, p25: 8, p75: 12, min: 5, max: 15,
  ...over,
});

const runRecord = (over = {}) => ({
  started: "2026-11-05T12:00:00.000Z",
  machine: { platform: "darwin", release: "27.0.1", arch: "arm64",
    cpu_model: "Apple M4 Max", cpu_count: 16,
    memory_bytes: 51539607552 },
  versions: { chromium: "151.0.7922.34", playwright: "1.62.1",
    "@cornerstonejs/core": "5.8.2",
    "@cornerstonejs/dicom-image-loader": "5.8.2" },
  // Fact 15: the renderer both sides reported, the mode the browser ran in,
  // and whether --software was passed.
  browser: { renderer: "ANGLE (Apple, ANGLE Metal Renderer: Apple M4 Max)",
    headed: false, software: false },
  defaults: {
    ocelli: { serverCacheBytes: 268435456, pageGreyCacheBytes: 67108864,
      decodePermits: 2 },
    cornerstone: { maxCacheSizeBytes: 3221225472, decodeWorkers: 8 },
  },
  workloads: { "synthetic-ct": { state: "selected", count: 1693 } },
  metrics: { open: { "synthetic-ct": { [OCELLI]: cell(), [CORNERSTONE]: cell() } } },
  pair_verdict: { within_one_lsb_fraction: 0.9999, count_over_max: 0,
    max_abs_diff: 2, signed_mean_bias_informative: 0.031,
    informative_fraction: 0.621, low_information: false, passes: true },
  architecture: ARCHITECTURE,
  not_shown: NOT_SHOWN,
  input_identity: "a".repeat(64),
  ...over,
});

// AC13. The schema is closed, so a field nobody expected refuses the
// publication rather than riding along, and the two fixed texts must be
// exactly the fixed texts.
test("the published record has a closed schema", () => {
  const published = publishedRecord(runRecord());
  assert.equal(published.machine.cpu_model, "Apple M4 Max");
  assert.equal(published.metrics.open["synthetic-ct"][OCELLI].median, 10);
  assert.equal(Object.hasOwn(published, "started"), false,
    "run-only keys stay behind");
  assert.equal(published.architecture, ARCHITECTURE);
  assert.deepEqual(published.not_shown, NOT_SHOWN);

  const withStray = runRecord({
    metrics: { open: { "synthetic-ct": { [OCELLI]: { ...cell(), note: "x" },
      [CORNERSTONE]: cell() } } },
  });
  assert.throws(() => publishedRecord(withStray), /note/,
    "an unknown key in a metric cell is refused, not copied and not dropped");
  assert.throws(() => publishedRecord(runRecord({ architecture: "other" })),
    /architecture/);
  const withoutArchitecture = runRecord();
  delete withoutArchitecture.architecture;
  assert.throws(() => publishedRecord(withoutArchitecture),
    /carries no architecture/,
    "a record with no architecture at all is named as the missing key");
  const withoutNotShown = runRecord();
  delete withoutNotShown.not_shown;
  assert.throws(() => publishedRecord(withoutNotShown), /not_shown/);
  const editedNotShown = runRecord({ not_shown: [...NOT_SHOWN.slice(1)] });
  assert.throws(() => publishedRecord(editedNotShown), /not_shown/);
  assert.throws(() => publishedRecord(runRecord({
    metrics: { bogus: { "synthetic-ct": { [OCELLI]: cell() } } } })),
    /is not a metric the published schema defines/,
    "an unknown metric is named as that, and not by a later refusal");
  assert.throws(() => publishedRecord(runRecord({
    metrics: { open: { bogus: { [OCELLI]: cell() } } } })),
    /is not a workload label/,
    "an unknown workload label is named as that, and not by a later refusal");
  assert.throws(() => publishedRecord(runRecord({
    metrics: { open: { "synthetic-ct": { [OCELLI]: cell(), chrome: cell() } } } })),
    /chrome/);
  assert.throws(() => publishedRecord(runRecord({
    pair_verdict: { ...runRecord().pair_verdict, extra: 1 } })), /extra/);
  assert.throws(() => publishedRecord(runRecord({
    workloads: { bogus: { state: "selected" } } })),
    /is not a workload the published schema defines/,
    "an unknown workload id is named as that, and not by its missing count");
  assert.throws(() => publishedRecord(runRecord({
    defaults: { ocelli: { secretBytes: 1 } } })), /secretBytes/);
  assert.throws(() => publishedRecord(runRecord({ input_identity: "xyz" })),
    /input identity/);
  assert.throws(() => publishedRecord(runRecord({
    metrics: { open: { "synthetic-ct": { [OCELLI]: cell({ state: "bogus" }) } } } })),
    /state/);

  // The review's probes, each a value that arrived before the schema was
  // closed at every level.
  const withHostname = runRecord();
  withHostname.machine.hostname = "STUDY-OF-DOE^JOHN";
  assert.throws(() => publishedRecord(withHostname), /hostname/,
    "the machine block is closed: a hostname is a field nobody expected");
  const withDescription = runRecord();
  withDescription.versions.series_description = "something a header said";
  assert.throws(() => publishedRecord(withDescription), /series_description/,
    "the versions block is not a free-text map");
  assert.throws(() => publishedRecord(runRecord({
    machine: { ...runRecord().machine, cpu_count: "16" } })), /whole count/,
    "the machine block's values are typed");
  assert.throws(() => publishedRecord(runRecord({
    versions: { ...runRecord().versions, chromium: 151 } })), /chromium/,
    "a version is a string, not a number the renderer could misread");
  assert.throws(() => publishedRecord(runRecord({
    pair_verdict: { ...runRecord().pair_verdict, passes: "DOE^JOHN" } })),
    /passes/,
    "the harness reads passes as the whole contract, so a string is refused");
  assert.throws(() => publishedRecord(runRecord({
    pair_verdict: { ...runRecord().pair_verdict, count_over_max: 1.5 } })),
    /count_over_max/,
    "a count of pixels is a whole count");
  assert.throws(() => publishedRecord(runRecord({
    metrics: { open: { "synthetic-ct": { [OCELLI]: cell({ median: "10" }),
      [CORNERSTONE]: cell() } } } })), /median/,
    "a measured cell's median is a finite number");
  assert.throws(() => publishedRecord(runRecord({
    metrics: { open: { "synthetic-ct": { [OCELLI]: cell() } } } })), /cornerstone/,
    "a cell missing a side is not a comparison, and the missing side is named");
  assert.throws(() => publishedRecord(runRecord({
    metrics: { open: { "synthetic-ct": { [CORNERSTONE]: cell() } } } })),
    /no ocelli side/, "whichever side is lost is the one named");
  assert.throws(() => publishedRecord(runRecord({
    metrics: { open: { "synthetic-ct": { [OCELLI]: cell({ reason: "why" }),
      [CORNERSTONE]: cell() } } } })), /reason/,
    "a measured cell has nothing to explain itself with");
  assert.throws(() => publishedRecord(runRecord({
    metrics: { open: { "synthetic-ct": {
      [OCELLI]: cell({ state: "unavailable", median: undefined, p25: undefined,
        p75: undefined, min: undefined, max: undefined }),
      [CORNERSTONE]: cell() } } } })), /no reason/,
    "an unavailable cell explains itself");
  assert.throws(() => publishedRecord(runRecord({
    metrics: { open: { "synthetic-ct": {
      [OCELLI]: { state: "invalid", n: 3, median: 5 },
      [CORNERSTONE]: cell() } } } })), /median/,
    "an invalid cell carries no statistic over a partial sample");
  assert.throws(() => publishedRecord(runRecord({
    browser: { ...runRecord().browser, gpuVendor: "vendor" } })), /gpuVendor/,
    "the browser block is closed");
  assert.throws(() => publishedRecord(runRecord({
    browser: { ...runRecord().browser, headed: "yes" } })), /booleans/,
    "the mode flags are booleans");
  assert.throws(() => publishedRecord(runRecord({
    browser: { ...runRecord().browser, renderer: "SwiftShader" } })),
    /SwiftShader/,
    "a SwiftShader renderer is refused without --software, per fact 15");
  const permitted = publishedRecord(runRecord({
    browser: { ...runRecord().browser, renderer: "SwiftShader",
      software: true } }));
  assert.equal(permitted.browser.software, true,
    "the flag permits the renderer and is recorded beside it");
  assert.throws(() => publishedRecord(runRecord({
    workloads: { "synthetic-ct": { state: "selected", count: -1 } } })),
    /count/, "a workload count is a whole count of inputs");
  assert.throws(() => publishedRecord(runRecord({
    workloads: { "synthetic-ct": { state: "unavailable", count: 0 } } })),
    /no reason/, "an unavailable workload explains itself");
});

// The seven refusals the second review pass found undriven. Each test here
// drives exactly one refusal, so deleting that one refusal in a scratch
// copy turns exactly its own test red and nothing else.

test("the pair verdict refuses a maximum difference above one byte", () => {
  assert.throws(() => publishedRecord(runRecord({
    pair_verdict: { ...runRecord().pair_verdict, max_abs_diff: 256 } })),
    /max_abs_diff is not a byte/,
    "an absolute pixel difference is one byte, and 256 does not fit");
});

test("the pair verdict refuses a fraction that is not finite", () => {
  assert.throws(() => publishedRecord(runRecord({
    pair_verdict: { ...runRecord().pair_verdict,
      within_one_lsb_fraction: NaN } })),
    /within_one_lsb_fraction is not a finite number/,
    "NaN would ride through the closed schema into a published figure");
});

// The informative fraction is a share of the frame's pixels, so the schema
// refuses one below 0 or above 1 and admits both ends. A measured bias rides
// beside each refused value, so only the range check can refuse it.
test("the pair verdict refuses an informative fraction outside [0, 1]", () => {
  for (const fraction of [-0.5, 1.5]) {
    assert.throws(() => publishedRecord(runRecord({
      pair_verdict: { ...runRecord().pair_verdict,
        informative_fraction: fraction } })),
      /informative_fraction is outside \[0, 1\]/,
      `a fraction of ${fraction} is not a share of the frame's pixels`);
  }
});

test("the pair verdict admits an informative fraction of 0 and of 1", () => {
  const atZero = publishedRecord(runRecord({
    pair_verdict: { ...runRecord().pair_verdict,
      signed_mean_bias_informative: null, informative_fraction: 0,
      low_information: true, passes: false } }));
  assert.equal(atZero.pair_verdict.informative_fraction, 0,
    "a fraction of exactly 0 is the floor of the range and publishes");
  const atOne = publishedRecord(runRecord({
    pair_verdict: { ...runRecord().pair_verdict, informative_fraction: 1 } }));
  assert.equal(atOne.pair_verdict.informative_fraction, 1,
    "a fraction of exactly 1 is the top of the range and publishes");
});

// The within-one-LSB fraction is a share of the frame's pixels too, so the
// schema bounds it alike: below 0 and above 1 refused, both ends admitted.
test("the pair verdict refuses a within-one-LSB fraction outside [0, 1]",
  () => {
    for (const fraction of [-0.5, 1.5]) {
      assert.throws(() => publishedRecord(runRecord({
        pair_verdict: { ...runRecord().pair_verdict,
          within_one_lsb_fraction: fraction } })),
        /within_one_lsb_fraction is outside \[0, 1\]/,
        `a fraction of ${fraction} is not a share of the frame's pixels`);
    }
  });

test("the pair verdict admits a within-one-LSB fraction of 0 and of 1", () => {
  for (const fraction of [0, 1]) {
    const published = publishedRecord(runRecord({
      pair_verdict: { ...runRecord().pair_verdict,
        within_one_lsb_fraction: fraction } }));
    assert.equal(published.pair_verdict.within_one_lsb_fraction, fraction,
      `a fraction of exactly ${fraction} is an end of the range and publishes`);
  }
});

test("the pair verdict refuses a low_information flag that is not a boolean",
  () => {
    assert.throws(() => publishedRecord(runRecord({
      pair_verdict: { ...runRecord().pair_verdict,
        low_information: "yes" } })),
      /low_information is not a boolean/,
      "the harness reads the flag as a boolean and nothing else");
  });

test("a selected workload carries no reason", () => {
  assert.throws(() => publishedRecord(runRecord({
    workloads: { "synthetic-ct": { state: "selected", count: 1693,
      reason: "why" } } })),
    /selected and carries a reason/,
    "a reason is what an unselected workload explains itself with");
});

test("a cell's reason, when it is there, is a string", () => {
  assert.throws(() => publishedRecord(runRecord({
    metrics: { open: { "synthetic-ct": {
      [OCELLI]: { state: "invalid", n: 3, reason: 42 },
      [CORNERSTONE]: cell() } } } })),
    /reason is not a string/,
    "a reason of the wrong type is a value the renderer cannot quote");
});

test("a cell's n is a whole count of trials", () => {
  assert.throws(() => publishedRecord(runRecord({
    metrics: { open: { "synthetic-ct": {
      [OCELLI]: cell({ n: 2.5 }), [CORNERSTONE]: cell() } } } })),
    /n is not a whole count/,
    "a fraction of a trial is not a count quartiles can be read against");
});

test("the machine block's string fields are strings", () => {
  assert.throws(() => publishedRecord(runRecord({
    machine: { ...runRecord().machine, cpu_model: 42 } })),
    /cpu_model is not a string/,
    "a number in a string field is a type the renderer cannot quote");
});

// A bias that was not measured is null, per the ruling after review pass 2.
// The Rust verdict serialises null when no pixel was informative, the
// closed schema admits null only when the informative fraction is exactly 0
// and low_information is then true, and the reports say the bias was not
// measured instead of printing a number no pixel measured.
test("a bias that was not measured is null and the reports say so", () => {
  const notMeasured = runRecord({
    pair_verdict: { ...runRecord().pair_verdict,
      signed_mean_bias_informative: null, informative_fraction: 0,
      low_information: true, passes: false },
  });
  const published = publishedRecord(notMeasured);
  assert.equal(published.pair_verdict.signed_mean_bias_informative, null,
    "the null rides through the closed schema beside an exactly zero "
      + "informative fraction and low_information true");
  for (const rendered of [renderSummary(published), renderReport(notMeasured)]) {
    assert.ok(
      rendered.includes(
        "not measured, because no pixel was informative"),
      "the report says the bias was not measured");
    assert.equal(
      rendered.includes(
        "signed mean bias over the informative region: 0.0000"),
      false, "and does not print a placeholder zero beside a zero fraction");
  }
  assert.throws(() => publishedRecord(runRecord({
    pair_verdict: { ...runRecord().pair_verdict,
      signed_mean_bias_informative: null } })),
    /informative fraction is exactly 0/,
    "a null beside a claimed-information run is a schema failure and not a " +
      "measurement");
  // The ruling admits null at an informative fraction of exactly 0 and no
  // other fraction. A null beside a fraction the run measured claims no
  // pixel was informative while five per cent were, and would render as not
  // measured beside a measured fraction.
  assert.throws(() => publishedRecord(runRecord({
    pair_verdict: { ...runRecord().pair_verdict,
      signed_mean_bias_informative: null, informative_fraction: 0.05,
      low_information: true, passes: false } })),
    /informative fraction is exactly 0/,
    "a null bias is admitted at an exactly zero fraction only, and "
      + "low_information is then true");
});

// The bias, when it was measured, is a finite number. Infinity and NaN are
// not measurements a renderer can quote, and the closed schema refuses them
// before publication.
test("a bias that was measured is a finite number", () => {
  for (const bias of [Infinity, NaN]) {
    assert.throws(() => publishedRecord(runRecord({
      pair_verdict: { ...runRecord().pair_verdict,
        signed_mean_bias_informative: bias } })),
      /signed_mean_bias_informative is not a finite number/,
      `a bias of ${bias} is not a measurement the renderer can quote`);
  }
});

// The equivalence runs both ways, per the lead's rulings on pass 4's D1 and
// S1. A fraction of exactly 0 measured no pixel, so there the bias is null
// and a number would be the placeholder the ruling removed, and the null is
// admitted only beside the low_information flag the run itself claimed.
test("a null bias beside a zero fraction needs low_information true", () => {
  assert.throws(() => publishedRecord(runRecord({
    pair_verdict: { ...runRecord().pair_verdict,
      signed_mean_bias_informative: null, informative_fraction: 0,
      low_information: false, passes: false } })),
    /low_information is then true/,
    "the schema admits the null only beside the floor flag, and a record "
      + "that reports the zero fraction without it is refused");
});

test("the pair verdict refuses a number beside a zero informative fraction",
  () => {
    assert.throws(() => publishedRecord(runRecord({
      pair_verdict: { ...runRecord().pair_verdict,
        signed_mean_bias_informative: 0, informative_fraction: 0,
        low_information: true, passes: false } })),
      /beside an informative fraction of exactly 0/,
      "a fraction of exactly 0 measured no pixel, so a number there is the "
        + "placeholder zero and not a measurement");
  });

// The reason lines are what an unmeasured cell publishes in place of a
// number. Both renderers carry them, and a renderer that dropped the line
// would publish a bare state with nothing having gone red.
test("the reports render the reason lines an unmeasured cell carries",
  () => {
    const run = runRecord({
      workloads: { "synthetic-ct": { state: "unavailable", count: 0,
        reason: "the synthetic series was not generated" } },
      metrics: { open: { "synthetic-ct": {
        [OCELLI]: { state: "unavailable", n: 0,
          reason: "the page never reached first paint" },
        [CORNERSTONE]: { state: "invalid", n: 3 } } } },
    });
    const published = publishedRecord(run);
    for (const rendered of [renderSummary(published), renderReport(run)]) {
      assert.ok(
        rendered.includes(
          "- synthetic CT: the synthetic series was not generated"),
        "the workloads section renders the workload's own reason");
      assert.ok(
        rendered.includes(
          "- synthetic CT: the page never reached first paint"),
        "the measurements section renders an unavailable cell's reason");
      assert.ok(rendered.includes(
        "- synthetic CT: fewer valid trials than the declared N for open"),
        "and an invalid cell's note names the metric that fell short");
    }
  });

// The refusal sites the fourth review pass's coverage sweep found with no
// test that reaches them. Each test drives exactly one site, so
// neutralising that one site in a scratch copy turns exactly its own test
// red and nothing else.
test("the run record itself must be an object", () => {
  assert.throws(() => publishedRecord("not a record"), /is not an object/,
    "a run record that is not an object has no schema to check");
});

test("the browser block carries a renderer string", () => {
  assert.throws(() => publishedRecord(runRecord({
    browser: { headed: false, software: false } })),
    /carries no renderer string/,
    "the tier sentence names the renderer, and there is nothing to name");
});

test("the defaults carry finite numbers", () => {
  assert.throws(() => publishedRecord(runRecord({
    defaults: { ocelli: { ...runRecord().defaults.ocelli,
      serverCacheBytes: "256 MiB" },
      cornerstone: runRecord().defaults.cornerstone } })),
    /carry no finite/,
    "a default the renderer cannot format is refused before publication");
});

test("a workload record's state is a state", () => {
  assert.throws(() => publishedRecord(runRecord({
    workloads: { "synthetic-ct": { state: "skipped", count: 1 } } })),
    /state is not a state/,
    "a state the renderers do not know would publish as its raw word");
});

test("the pair verdict carries every key", () => {
  const withoutMaxDiff = runRecord();
  delete withoutMaxDiff.pair_verdict.max_abs_diff;
  assert.throws(() => publishedRecord(withoutMaxDiff),
    /carries no max_abs_diff/,
    "a verdict missing a key is not the seven keys the harness reads");
});

test("selectWorkloads refuses a header record that is not a record", () => {
  assert.throws(() => selectWorkloads([null]), /is not a record/,
    "a null where a header belongs has no attribute to select by");
});

test("selectWorkloads refuses a source that is neither synthetic nor pseudo",
  () => {
    assert.throws(() => selectWorkloads([header({ source: "downloaded" })]),
      /neither synthetic nor pseudo/,
      "a source outside the two the rules read belongs to no workload");
  });

test("selectWorkloads refuses a header record without its flags", () => {
  assert.throws(() => selectWorkloads([header({ hasWindowCenter: undefined })]),
    /is missing hasWindowCenter/,
    "a rule that reads a flag cannot run over a record that lacks it");
});

test("selectWorkloads refuses dimensions outside the US range", () => {
  assert.throws(() => selectWorkloads([header({ rows: 65536 })]),
    /US range/,
    "DICOM's Rows and Columns are US, and 65536 does not fit");
});

test("selectWorkloads takes an array of header records", () => {
  assert.throws(() => selectWorkloads("two records"), /takes an array/,
    "a string has no records to filter");
});

test("summarise takes the array of a cell's valid trials", () => {
  assert.throws(() => summarise("three trials", 9), /takes the array/,
    "a non-array has no trials to order");
});

test("the tier sentence needs the closed browser block", () => {
  assert.throws(() => tierSentence({ renderer: "ANGLE (Apple)" }),
    /needs the closed browser block/,
    "a block without the mode is not the block fact 15 records");
});

test("parseProcessTree takes ps -A -o pid=,ppid= output", () => {
  assert.throws(() => parseProcessTree(42, 100),
    /takes ps -A -o pid=,ppid= output/,
    "a non-string has no lines to walk");
});

test("parseProcessTree's rootPid must be a process id", () => {
  assert.throws(() => parseProcessTree(TREE_PS_TEXT, 0),
    /rootPid must be a process id/,
    "pid 0 is not a process the tree can be rooted at");
});

// The published fixture the summary renders. The medians and spreads are
// chosen so the open row is separated (the interquartile ranges [110, 140]
// and [10, 40] are disjoint, ratio 123.4 / 100 = 1.234, three significant
// figures 1.23), the first image row is not (the ranges [40, 60] and [45,
// 65] overlap, and the medians 50 and 55 still differ, which is what kills
// a rule on medians alone), and the memory row is separated with the ratio
// 0.5 rendered as 0.500.
const published = () => {
  const record = runRecord({
    metrics: {
      open: { "synthetic-ct": {
        [OCELLI]: cell({ median: 123.4, p25: 110, p75: 140 }),
        [CORNERSTONE]: cell({ median: 100, p25: 10, p75: 40 }) } },
      first_image: { "synthetic-ct": {
        [OCELLI]: cell({ median: 50, p25: 40, p75: 60 }),
        [CORNERSTONE]: cell({ median: 55, p25: 45, p75: 65 }) } },
      peak_memory: { [SESSION_WORKLOAD]: {
        [OCELLI]: cell({ n: 3, median: 1048576, p25: 1048576, p75: 1048576,
          min: 1048576, max: 1048576 }),
        [CORNERSTONE]: cell({ n: 3, median: 2097152, p25: 2097152,
          p75: 2097152, min: 2097152, max: 2097152 }) } },
    },
  });
  return publishedRecord(record);
};

const ADJECTIVES = ["faster", "slower", "blazing", "superior", "outperforms",
  "dramatically", "significantly", "best", "better", "worse"];

// AC14. The summary carries the fixed texts, the machine, the versions, the
// defaults table and the not-shown section, and none of the vocabulary a
// ranking would use. prose_check.scan runs over the rendered text with the
// checker's own rules, unchanged.
test("the summary is measured and acknowledging", () => {
  const summary = renderSummary(published());
  assert.ok(summary.includes(ARCHITECTURE), "the architecture text");
  assert.ok(summary.includes(ACKNOWLEDGEMENT), "the acknowledgement");
  assert.ok(summary.includes(tierSentence(published().browser)),
    "the tier sentence, with the renderer named and the mode recorded");
  assert.ok(summary.includes(published().browser.renderer),
    "the renderer string is named, not described");
  assert.ok(summary.includes("The browser ran headless."), "the mode");
  assert.ok(summary.includes("Apple M4 Max, 16 cores"), "the machine");
  assert.ok(summary.includes("151.0.7922.34"), "the Chromium version");
  assert.ok(summary.includes("@cornerstonejs/core"), "the package versions");
  assert.ok(summary.includes("server cache bytes"), "the defaults table");
  assert.ok(summary.includes("## What the numbers do not show"),
    "the not-shown section");
  for (const item of NOT_SHOWN) {
    assert.ok(summary.includes(item), `the not-shown item: ${item}`);
  }
  assert.ok(summary.includes("| 1.23 |"),
    "the ratio is three significant figures");
  assert.ok(summary.includes("| 0.500 |"),
    "the ratio keeps its significant figures at small magnitudes");
  assert.ok(summary.includes("no separation at 9 trials"),
    "an unseparated difference says so at N trials");
  assert.ok(summary.includes("| 1.0 | 1.0 to 1.0 |"),
    "memory renders in MiB to one decimal");
  assert.ok(summary.includes("| 123.4000 |"),
    "durations render at clock precision");
  for (const adjective of ADJECTIVES) {
    assert.equal(summary.includes(adjective), false,
      `the summary never says ${adjective}`);
  }
  assert.equal(summary.includes("—"), false, "no em-dash");
  assert.equal(summary.includes("–"), false, "no en-dash");
  assert.equal(summary.includes(";"), false,
    "no semicolon outside code, and the summary has no code");
  assert.ok(summary.startsWith("<!--"),
    "the first line names the command that wrote it");
  assert.ok(summary.includes("Do not edit."));
});

// The wrong implementation the plan names: a template that drops the caveat
// when the numbers favour one side. Two records, one with every Ocelli
// figure below cornerstone3D's and one with every figure above, must render
// byte-identical fixed texts.
test("the caveats survive numbers that favour either side", () => {
  const lower = publishedRecord(runRecord({
    metrics: {
      open: {
        "synthetic-ct": {
          [OCELLI]: cell({ median: 1, p25: 1, p75: 1, min: 1, max: 1 }),
          [CORNERSTONE]: cell({ median: 100, p25: 100, p75: 100, min: 100,
            max: 100 }),
        },
      },
    },
  }));
  const higher = publishedRecord(runRecord({
    metrics: {
      open: {
        "synthetic-ct": {
          [OCELLI]: cell({ median: 100, p25: 100, p75: 100, min: 100,
            max: 100 }),
          [CORNERSTONE]: cell({ median: 1, p25: 1, p75: 1, min: 1, max: 1 }),
        },
      },
    },
  }));
  const fixed = [ARCHITECTURE, ACKNOWLEDGEMENT,
    tierSentence(runRecord().browser), ...NOT_SHOWN];
  const lowerSummary = renderSummary(lower);
  const higherSummary = renderSummary(higher);
  for (const item of fixed) {
    assert.ok(lowerSummary.includes(item) && higherSummary.includes(item),
      `the caveat survives both ways: ${item.slice(0, 60)}`);
  }
});

// Fact 15: a run made with --software names SwiftShader in its first
// paragraph, in every output, and the tier sentence names the renderer the
// pages actually reported. Without the flag the first paragraph is the
// fixed architecture text alone.
test("a software run names SwiftShader in its first paragraph", () => {
  const software = publishedRecord(runRecord({
    browser: { renderer: "SwiftShader", headed: true, software: true } }));
  const summary = renderSummary(software);
  const summaryFirst = summary.split("\n")[4];
  assert.ok(summaryFirst.includes("SwiftShader"),
    "the first paragraph names SwiftShader");
  assert.ok(summaryFirst.includes(ARCHITECTURE),
    "and still carries the fixed architecture text");
  assert.ok(summary.includes(tierSentence(software.browser)),
    "the tier sentence names the renderer the pages reported");
  assert.ok(summary.includes("The browser ran headed."),
    "the mode is recorded, because fact 15 records headed in the output");
  const report = renderReport(runRecord({
    state: "measured",
    browser: { renderer: "SwiftShader", headed: false, software: true } }));
  const reportFirst = report.split("\n")[4];
  assert.ok(reportFirst.includes("SwiftShader"),
    "the plain report's first paragraph names SwiftShader too");
  const plainFirst = renderSummary(published()).split("\n")[4];
  assert.equal(plainFirst, ARCHITECTURE,
    "without --software the first paragraph is the fixed text alone");
});

test("the summary and the report pass prose_check over the rendered text",
  () => {
    const scripts = fileURLToPath(new URL("../../../scripts", import.meta.url));
    const script = "import sys; sys.path.insert(0, sys.argv[1]); " +
      "import prose_check; " +
      "problems = prose_check.scan(sys.stdin.read(), 'rendered'); " +
      "print('\\n'.join(problems)); " +
      "sys.exit(1 if problems else 0)";
    const rendered = renderSummary(published()) + "\n" +
      renderReport(runRecord());
    const result = spawnSync("python3", ["-c", script, scripts], {
      input: rendered, encoding: "utf8",
    });
    assert.equal(result.status, 0,
      `prose_check found problems:\n${result.stdout}\n${result.stderr}`);
  });

// The plain report is rendered from the ignored record and carries the raw
// run's extras: the started timestamp, the run state and the refusals.
test("the plain report renders the run", () => {
  const report = renderReport(runRecord({
    state: "measured",
    refusals: ["the second workload was refused: no qualifying DX"],
  }));
  assert.ok(report.includes(ARCHITECTURE),
    "the architecture text is the first paragraph");
  assert.ok(report.includes("2026-11-05T12:00:00.000Z"), "the started stamp");
  assert.ok(report.includes("no qualifying DX"), "the refusals");
  assert.ok(report.includes("no separation at 9 trials"),
    "the same measurement table as the summary");
  const refused = renderReport(runRecord({
    state: "refused_pixels",
    metrics: {},
    pair_verdict: { within_one_lsb_fraction: 0.9999, count_over_max: 3,
      max_abs_diff: 3, signed_mean_bias_informative: 0.44,
      informative_fraction: 0.621, low_information: false, passes: false },
  }));
  assert.ok(refused.includes("refused_pixels"),
    "a refused run's report carries the state and no timing figure");
  assert.ok(refused.includes("the verdict passed: no"), "and the verdict");
  assert.ok(refused.includes("No timing figure was recorded"),
    "and says so, because nothing above the section was measured");
  assert.equal(refused.includes(
    "Every timing figure above was taken in a run whose pixel check passed"),
    false,
    "a refused run's report must not say the pixel check passed: that is \
     exactly the state the report has to describe honestly");
  assert.ok(renderReport(runRecord({ state: "measured" })).includes(
    "Every timing figure above was taken in a run whose pixel check passed"),
    "the passing run's report still claims its precondition");
  assert.throws(() => renderReport({ state: "measured" }),
    /requires the architecture text/,
    "the report's first paragraph is the architecture text, and a record "
      + "without it has nothing to open with");
});

// AC3's pure half. The mode refuses on the count, and the count itself is
// here.
test("other browser jobs are counted outside our tree", () => {
  const psText = [
    "  700 /Users/dev/Library/Caches/ms-playwright/chromium-1234/chrome-mac/Chromium.app/Contents/MacOS/Chromium",
    "  701 /Users/dev/Library/Caches/ms-playwright/chromium-1234/chrome-mac/Chromium.app/Contents/MacOS/Chromium Helper",
    "  800 /usr/sbin/cfprefsd",
    "  900 node tools/bench/run.mjs --compare-cornerstone",
    "  901 /bin/zsh -c echo ms-playwright",
  ].join("\n");
  assert.equal(otherBrowserJobs(psText, new Set([900, 901])), 2,
    "the two browser processes outside our tree");
  assert.equal(otherBrowserJobs(psText, new Set([700, 701, 900, 901])), 0,
    "our own browser processes are not other jobs");
  assert.throws(() => otherBrowserJobs(42, new Set()),
    /takes ps -A -o pid=,command= output/,
    "a non-string has no lines to count");
  assert.throws(() => otherBrowserJobs("  1 chrome\n", [1]),
    /takes our own pids as a Set/,
    "our own pids arrive as the set the tree walk produced");
});

// The binding in the other direction. comparison.json is the definitions
// file and comparison.mjs is the vocabulary, and loadComparison refuses a
// file that disagrees with the vocabulary, so the two cannot drift.
const COMPARISON_PATH = new URL("../comparison.json", import.meta.url);
const definitions = loadComparison(readFileSync(COMPARISON_PATH, "utf8"));

test("the comparison file and the code agree", () => {
  assert.deepEqual(definitions.sides, SIDES);
  assert.deepEqual(definitions.workloads.map((one) => one.id), WORKLOAD_IDS);
  // The plan's metric table, asserted against the file. A metric the file
  // drops would silently vanish from the run.
  const plan = {
    page_ready: { n: 9, workloads: [] },
    open: { n: 9, workloads: ["synthetic-ct", "pseudo-full"] },
    first_image: { n: 9,
      workloads: ["synthetic-ct", "synthetic-dx", "pseudo-ct", "pseudo-dx"] },
    scroll: { n: 5, workloads: ["synthetic-ct", "pseudo-ct"] },
    window_change: { n: 9,
      workloads: ["synthetic-ct", "synthetic-dx", "pseudo-ct", "pseudo-dx"] },
    peak_memory: { n: 3, workloads: [SESSION_WORKLOAD] },
    session_cpu: { n: 3, workloads: [SESSION_WORKLOAD] },
  };
  assert.deepEqual(definitions.metrics.map((one) => one.id),
    Object.keys(plan));
  for (const metric of definitions.metrics) {
    const expected = plan[metric.id];
    assert.equal(metric.n, expected.n, `${metric.id} n`);
    assert.deepEqual(metric.workloads, expected.workloads,
      `${metric.id} workloads`);
    assert.ok(metric.mirrors === null || metric.mirrors.startsWith("viewer."),
      `${metric.id} mirrors a viewer subject or nothing`);
  }
  assert.deepEqual(
    definitions.metrics.filter((one) => one.mirrors === null)
      .map((one) => one.id),
    ["page_ready", "session_cpu"],
    "the plan's table names the two metrics that mirror nothing");
  assert.equal(definitions.metrics.find((one) => one.id === "open")
    .cold.state, "yes");
  assert.equal(definitions.metrics.find((one) => one.id === "page_ready")
    .warm.state, "unavailable");
  assert.equal(definitions.metrics.find((one) => one.id === "peak_memory")
    .warm.state, "n/a");
});

test("loadComparison refuses a file that disagrees", () => {
  const broken = (change) => {
    const parsed = JSON.parse(readFileSync(COMPARISON_PATH, "utf8"));
    change(parsed);
    return () => loadComparison(JSON.stringify(parsed));
  };
  assert.throws(broken((parsed) => { parsed.sides = ["ocelli"]; }), /two sides/);
  assert.throws(broken((parsed) => {
    parsed.metrics.find((one) => one.id === "open").unit = "furlongs";
  }), /unit/);
  assert.throws(broken((parsed) => {
    parsed.metrics.push({ id: "extra_metric" });
  }), /extra_metric/);
  assert.throws(broken((parsed) => { parsed.workloads.pop(); }), /workload/);
  assert.throws(broken((parsed) => {
    parsed.metrics.find((one) => one.id === "open").workloads.push("bogus");
  }), /bogus/);
  assert.throws(broken((parsed) => {
    delete parsed.metrics.find((one) => one.id === "open").start;
  }), /start/);
  assert.throws(broken((parsed) => {
    parsed.metrics.find((one) => one.id === "open").n = 0;
  }), /declared N/);
  assert.throws(() => loadComparison("{not json"), /not valid JSON/);
  assert.throws(broken((parsed) => {
    parsed.metrics.find((one) => one.id === "open").workloads =
      "synthetic-ct";
  }), /are not a list/,
  "workloads that are not a list at all are named as that, not as a twice-named label");
  assert.throws(() => loadComparison("[]"), /is not an object/,
    "a file whose root is a list is not a definitions object");
  assert.throws(broken((parsed) => { delete parsed.metrics; }),
    /carries no metrics array/,
    "a definitions file with no metrics defines nothing");
  assert.throws(broken((parsed) => {
    parsed.metrics.push({ ...parsed.metrics[0] });
  }), /names a metric twice/,
  "a metric named twice has no single unit or trial count to read");
  assert.throws(broken((parsed) => { parsed.metrics.pop(); }),
    /must define all/,
    "a metric the file drops would silently vanish from the run");
  assert.throws(broken((parsed) => {
    parsed.metrics.find((one) => one.id === "open").mirrors = "renderer.open";
  }), /mirrors neither null nor/,
  "a mirror outside the viewer's subjects is a subject nobody measures");
  assert.throws(broken((parsed) => {
    parsed.metrics.find((one) => one.id === "open").workloads =
      ["synthetic-ct", "synthetic-ct"];
  }), /names a workload label twice/,
  "a label named twice has no single workload cell to read");
  assert.throws(broken((parsed) => {
    parsed.metrics.find((one) => one.id === "open").cold = { state: "sometimes" };
  }), /state must be one of/,
  "a cold or warm state outside the two is a state the run cannot record");
  assert.throws(broken((parsed) => {
    parsed.metrics.find((one) => one.id === "open").cold =
      { state: "unavailable" };
  }), /names no reason/,
  "an unavailable metric without its reason is an omission dressed as an answer");
});

// The ratio is three significant figures in plain decimal notation,
// including across a decade, and a zero denominator is words saying so,
// per the ruling after review pass 2. The decade is the trap: the decimals
// come from the exponent of the ROUNDED value, so 9.996 renders "10.0" and
// not "10.00" and 0.99951 renders "1.00" and not "1.000", while 999.6
// renders "1000", which is three significant figures and not four.
test("the ratio is three significant figures and a zero denominator is words",
  () => {
    const ratio = (a, b) => renderSummary(publishedRecord(runRecord({
      metrics: { open: { "synthetic-ct": {
        [OCELLI]: cell({ median: a, p25: a, p75: a, min: a, max: a }),
        [CORNERSTONE]: cell({ median: b, p25: b, p75: b, min: b, max: b }),
      } } },
    })));
    assert.ok(ratio(2000, 1).includes("| 2000 |"),
      "a thousandfold ratio stays plain decimal at three significant figures");
    assert.equal(ratio(2000, 1).includes("e+3"), false,
      "no exponent notation in a measured table");
    assert.ok(ratio(9.996, 1).includes("| 10.0 |"),
      "rounding up across a decade keeps three significant figures");
    assert.equal(ratio(9.996, 1).includes("| 10.00 |"), false,
      "and does not gain a fourth digit at the decade");
    assert.ok(ratio(0.99951, 1).includes("| 1.00 |"),
      "rounding down across a decade keeps three significant figures");
    assert.equal(ratio(0.99951, 1).includes("| 1.000 |"), false,
      "and does not gain a fourth digit below it either");
    assert.ok(ratio(999.6, 1).includes("| 1000 |"),
      "999.6 rounds to a power of ten at three significant figures, not four");
    assert.ok(ratio(0.5, 1).includes("| 0.500 |"),
      "a sub-unity ratio keeps its three significant figures");
    const zeroDenominator = ratio(5, 0);
    assert.ok(zeroDenominator.includes("| denominator is zero |"),
      "a zero denominator renders as words saying the denominator is zero");
    assert.equal(zeroDenominator.includes("Infinity"), false,
      "and never as the word Infinity");
    assert.equal(zeroDenominator.includes("NaN"), false,
      "and never as NaN");
  });

// A version string carrying a pipe would otherwise end the cell and break
// the table the summary renders byte for byte.
test("a version string carrying a pipe does not break the versions table",
  () => {
    const record = runRecord({
      versions: { ...runRecord().versions,
        "@cornerstonejs/core": "5.8.2 (pinned|exact)" } });
    const summary = renderSummary(publishedRecord(record));
    assert.ok(summary.includes("5.8.2 (pinned\\|exact)"),
      "the pipe is escaped, so the cell stays one cell");
  });

// Lead ruling2026-11-08: publish cold and warm separately, not one overwritten median.
test("qualified conditions retain both observations and reject unknown or missing conditions",()=>{
 const sides=(median)=>({ocelli:cell({median}),cornerstone:cell({median})});
 const metrics={open:{"synthetic-ct/cold":sides(10),"synthetic-ct/warm":sides(20)}};
 const p=publishedRecord(runRecord({metrics}));assert.equal(p.metrics.open["synthetic-ct/cold"].ocelli.median,10);assert.equal(p.metrics.open["synthetic-ct/warm"].ocelli.median,20);
 assert.match(renderSummary(p),/synthetic-ct.*cold/);assert.match(renderSummary(p),/synthetic-ct.*warm/);
 assert.throws(()=>publishedRecord(runRecord({metrics:{open:{"synthetic-ct/hot":sides(10)}}})),/workload|condition/);
 assert.throws(()=>publishedRecord(runRecord({metrics:{open:{"synthetic-ct/cold":sides(10)}}})),/paired condition/);
});
test("memory publication preserves separate three-session lower and upper bounds",()=>{
 // Three literal sessions yield concurrent sums20,50,70 and sums of
 // process peaks110,300,300. Their medians50 and300 are different facts.
 const observations=[new Map([[1,{footprint:10,peak:100}],[2,{footprint:10,peak:10}]]),new Map([[1,{footprint:30,peak:100}],[2,{footprint:20,peak:200}]]),new Map([[1,{footprint:20,peak:100}],[2,{footprint:50,peak:200}]])].map(s=>memoryFigures([s]));
 const lower=summarise(observations.map(x=>x.sampled_peak),3), upper=summarise(observations.map(x=>x.sum_of_peaks),3);assert.equal(lower.median,50);assert.equal(upper.median,300);
 const make=(u)=>runRecord({metrics:{peak_memory:{"session/cold":{ocelli:{...lower,upper_bound:u},cornerstone:{...lower,upper_bound:upper}},"session/warm":{ocelli:{state:"unavailable",n:0,reason:"not applicable"},cornerstone:{state:"unavailable",n:0,reason:"not applicable"}}}}});
 const p=publishedRecord(make(upper));assert.equal(p.metrics.peak_memory["session/cold"].ocelli.upper_bound.median,300);assert.match(renderSummary(p),/sum of per-process peaks/);
 for(const bad of [undefined,{...upper,n:2},{...upper,median:NaN},{...upper,note:"unexpected"}])assert.throws(()=>publishedRecord(make(bad)),/upper|finite|trial|note/);
});

test("recording a condition twice refuses before overwriting its figure",()=>{
 const metrics={};const sides={ocelli:cell(),cornerstone:cell()};recordCondition(metrics,"open","synthetic-ct","cold",sides);assert.throws(()=>recordCondition(metrics,"open","synthetic-ct","cold",sides),/duplicate/);assert.equal(metrics.open["synthetic-ct/cold"],sides);assert.throws(()=>recordCondition(metrics,"open","synthetic-ct","hot",sides),/condition/);
});


test("both reports identify the memory bounds and the sampler minimum interval", () => {
  assert.equal(FOOTPRINT_PERIOD, 250, "approved minimum sampling pause in milliseconds");
  const lower = cell({ n: 3, median: 1048576, p25: 1048576, p75: 1048576, min: 1048576, max: 1048576 });
  const upper = cell({ n: 3, median: 2097152, p25: 2097152, p75: 2097152, min: 2097152, max: 2097152 });
  const record = runRecord({ metrics: { peak_memory: { "session/cold": {
    ocelli: { ...lower, upper_bound: upper }, cornerstone: { ...lower, upper_bound: upper } },
    "session/warm": { ocelli: { state: "unavailable", n: 0, reason: "not applicable" },
      cornerstone: { state: "unavailable", n: 0, reason: "not applicable" } } } } });
  const published = publishedRecord(record);
  const priorSchema = JSON.stringify(published);
  for (const text of [renderSummary(published), renderReport(record)]) {
    assert.match(text, /session\/cold, sampled concurrent footprint lower bound/);
    assert.match(text, /session\/cold, sum of per-process peaks upper bound/);
    assert(text.includes("Memory sampling pauses at least 250 ms between completed collection rounds."));
    assert(text.includes("Actual sample timestamps remain in raw evidence."));
  }
  assert.equal(JSON.stringify(published), priorSchema, "rendering does not extend the publication schema");
});
