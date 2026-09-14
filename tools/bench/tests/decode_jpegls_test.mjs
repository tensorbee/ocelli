import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import test from "node:test";

import { repoPath } from "../src/paths.mjs";
import { parseJpegLs } from "../src/runners/decode_transfer_syntax_jpegls.mjs";

function record(changes = {}) {
  return JSON.stringify({
    value: 0.5,
    iterations: 31,
    warmup_iterations: 1,
    decodes_per_sample: 4,
    range_ms: [0.4, 0.6],
    checksum: 42,
    ...changes,
  });
}

test("JPEG-LS result retains measured evidence", () => {
  const result = parseJpegLs(record());
  assert.equal(result.value, 0.5);
  assert.equal(result.iterations, 31);
  assert.equal(result.warmup_iterations, 1);
  assert.equal(result.decodes_per_sample, 4);
  assert.deepEqual(result.range_ms, [0.4, 0.6]);
  assert.equal(result.checksum, 42);
});

test("JPEG-LS result requires the recorded warm-up", () => {
  const parsed = JSON.parse(record());
  delete parsed.warmup_iterations;
  assert.throws(() => parseJpegLs(JSON.stringify(parsed)), /warm-up iteration/);
});

test("JPEG-LS result requires the runner iteration contract", () => {
  for (const changes of [
    { iterations: -1 },
    { iterations: 31.5 },
    { iterations: 30 },
    { warmup_iterations: -1 },
    { warmup_iterations: 1.5 },
    { warmup_iterations: 2 },
  ]) {
    assert.throws(() => parseJpegLs(record(changes)), /iteration/);
  }
});

test("JPEG-LS iteration provenance counts timing samples", () => {
  assert.throws(
    () => parseJpegLs(record({ iterations: 30 })),
    /31 timing samples/,
  );
});

test("JPEG-LS result requires the four-decode normalized sample", () => {
  const missing = JSON.parse(record());
  delete missing.decodes_per_sample;
  for (const candidate of [
    JSON.stringify(missing),
    record({ decodes_per_sample: 0 }),
    record({ decodes_per_sample: 4.5 }),
    record({ decodes_per_sample: 8 }),
  ]) {
    assert.throws(() => parseJpegLs(candidate), /decodes per timing sample/);
  }
});

test("JPEG-LS result refuses a missing or malformed range", () => {
  const missing = JSON.parse(record());
  delete missing.range_ms;
  for (const candidate of [
    JSON.stringify(missing),
    record({ range_ms: [0.4] }),
    record({ range_ms: ["0.4", 0.6] }),
    record({ range_ms: null }),
  ]) {
    assert.throws(() => parseJpegLs(candidate), /finite observed range/);
  }
});

test("JPEG-LS result refuses non-positive range endpoints", () => {
  for (const range_ms of [[-2, -1], [0, 0.6]]) {
    assert.throws(() => parseJpegLs(record({ range_ms })), /positive observed range/);
  }
});

test("JPEG-LS result refuses a reversed range", () => {
  assert.throws(
    () => parseJpegLs(record({ range_ms: [0.6, 0.4] })),
    /ordered observed range/,
  );
});

test("JPEG-LS result requires the range to enclose the median", () => {
  for (const range_ms of [[0.2, 0.4], [0.6, 0.8]]) {
    assert.throws(() => parseJpegLs(record({ range_ms })), /enclose the median/);
  }
});

test("JPEG-LS result refuses missing duration or checksum evidence", () => {
  assert.throws(() => parseJpegLs(record({ value: 0 })), /positive finite duration/);
  assert.throws(() => parseJpegLs(record({ checksum: 0 })), /output checksum/);
});

test("the JPEG-LS baseline is the median of its complete calibration series", () => {
  const baseline = JSON.parse(readFileSync(repoPath("ci", "bench-baseline.json"), "utf8"));
  const entries = Object.values(baseline.host_classes).flatMap((host) =>
    Object.entries(host.subjects)
      .filter(([id]) => id === "decode.transfer_syntax.jpegls")
      .map(([, entry]) => entry));
  assert.equal(entries.length, 1, "expected exactly one JPEG-LS baseline");
  const [entry] = entries;
  const series = entry.detail.calibration_medians_ms;
  assert(Array.isArray(series) && series.length === 15,
    "the baseline must retain exactly fifteen calibration medians");
  assert.equal(entry.tolerance, 0.15,
    "the JPEG-LS tolerance differs from the documented 15 per cent");
  assert.match(entry.tolerance_provenance, /retained 15 per cent/,
    "the tolerance provenance does not name the exact documented value");
  assert(series.every((value) => Number.isFinite(value) && value > 0),
    "every calibration median must be a positive finite duration");
  // Both codec subjects now carry a calibration paragraph with the same
  // wording, and a non-global regex returns the first match, so the search is
  // scoped to this subject's paragraph rather than to the whole file. Without
  // this the JPEG-LS test reads JPEG 2000's series and compares it with its own.
  const whole = readFileSync(repoPath("docs", "lld", "benchmarks.md"), "utf8");
  const marker = "F-028 adds `decode.transfer_syntax.jpegls`";
  assert(whole.includes(marker), "the LLD does not introduce the JPEG-LS subject");
  const lld = whole.slice(whole.indexOf(marker));
  assert.match(lld, /The retained 15 per cent is the\s+smallest declared symmetric band/,
    "the LLD does not retain the exact documented tolerance");
  const documented = lld.match(
    /controlled calibration series of 15\s+consecutive production\s+runner medians, in run order, was\s+`([^`]+)`\s+ms\./,
  );
  assert(documented, "the LLD does not retain the complete calibration series");
  assert.deepEqual(documented[1].split(", ").map(Number), series,
    "the LLD calibration series differs from the baseline record");
  const ordered = series.toSorted((left, right) => left - right);
  assert.equal(entry.value, ordered[Math.floor(ordered.length / 2)],
    "the baseline value is not the median of its calibration medians");
  assert.deepEqual(entry.detail.calibration_range_ms,
    [ordered[0], ordered.at(-1)],
    "the documented calibration range does not match the complete series");
  assert(lld.includes(
    `Its median is ${entry.value.toFixed(4)} ms and its range is ` +
    `${ordered[0].toFixed(4)} to ${ordered.at(-1).toFixed(4)} ms`),
  "the LLD median or range differs from the structured calibration record");
  const lower = entry.value * (1 - entry.tolerance);
  const upper = entry.value * (1 + entry.tolerance);
  assert(series.every((value) => lower <= value && value <= upper),
    `the calibration range is outside the recorded ${entry.tolerance} band`);
});

test("JPEG-LS timing provenance names the normalized batch clock", () => {
  const expected = "std::time::Instant surrounds each four-decode batch, then elapsed time is normalized by four";
  const baseline = JSON.parse(readFileSync(repoPath("ci", "bench-baseline.json"), "utf8"));
  const entry = Object.values(baseline.host_classes)
    .flatMap((host) => Object.entries(host.subjects))
    .find(([id]) => id === "decode.transfer_syntax.jpegls")?.[1];
  assert(entry, "expected the JPEG-LS baseline");
  assert.equal(entry.detail.clock, expected,
    "the accepted clock provenance does not describe the four-decode timing boundary");
  const runner = readFileSync(
    repoPath("tools", "bench", "src", "runners", "decode_transfer_syntax_jpegls.mjs"),
    "utf8",
  );
  assert(runner.includes(`clock: ${JSON.stringify(expected)}`),
    "the runner clock provenance differs from the accepted baseline");
});
