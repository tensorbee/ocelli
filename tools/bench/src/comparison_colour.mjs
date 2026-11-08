// Opt-in controlled colour comparison, F239 AC32 and F240 AC16.
// The existing F223 CT pixel calibration is mandatory and remains separate.
// ICC images deliberately differ: Ocelli applies the profile and cornerstone
// draws device RGB. This module defines no colour score or tolerance.

import { createHash } from "node:crypto";
import { createReadStream } from "node:fs";
import { readFile, readdir, lstat, realpath } from "node:fs/promises";
import { join, resolve, relative, isAbsolute } from "node:path";
import { SIDES, summarise, memoryFigures } from "./comparison.mjs";

export const COLOUR_CASE_IDS = Object.freeze(["rgb8", "ybr422", "palette16", "icc8-gamma22", "icc16-table"]);
export const COLOUR_WINDOWS = Object.freeze(Array.from({ length: 9 }, (_, at) => Object.freeze({ center: 100 + at, width: 200 })));
const MANIFEST = "colour-comparison-input.json";
const FORMAT = "ocelli-controlled-colour-comparison-1";
const MIB = 1048576;
const CASES = {
  rgb8: [512, 200, "RGB", 3, 8, 256, null],
  ybr422: [512, 200, "YBR_FULL_422", 3, 8, 256, null],
  palette16: [512, 200, "PALETTE COLOR", 1, 16, 256, null],
  "icc8-gamma22": [8192, 1, "RGB", 3, 8, 256, "gamma22"],
  "icc16-table": [8192, 1, "RGB", 3, 16, 512, "curv-two-entry-linear"],
};

export function colourCaseSpec(id) {
  if (!Object.hasOwn(CASES, id)) throw new Error("unknown controlled colour comparison case");
  const [edge, frames, photometric, samplesPerPixel, bitsAllocated, serverCacheMiB, profileKind] = CASES[id];
  return { id, rows: edge, columns: edge, frames, photometric, samplesPerPixel,
    bitsAllocated, bitsStored: bitsAllocated, serverCacheMiB, profileKind };
}

function closed(value, keys, label) {
  if (value === null || typeof value !== "object" || Array.isArray(value) ||
      Object.keys(value).sort().join("\0") !== [...keys].sort().join("\0")) {
    throw new Error(`${label} has missing or unknown fields`);
  }
}

function finite(value, label) {
  if (typeof value !== "number" || !Number.isFinite(value) || value < 0) {
    throw new Error(`${label} is not a nonnegative finite measurement`);
  }
  return value;
}

function hashBytes(bytes) { return createHash("sha256").update(bytes).digest("hex"); }

async function fileDigest(path) {
  const hash = createHash("sha256");
  for await (const chunk of createReadStream(path)) hash.update(chunk);
  return hash.digest("hex");
}

/** Validate exact declared series inputs without loading frame payloads into memory. */
export async function readColourComparisonInput(directory, caseId) {
  const spec = colourCaseSpec(caseId);
  const root = await realpath(directory);
  const manifestPath = join(root, MANIFEST);
  if (!(await lstat(manifestPath)).isFile()) throw new Error("the comparison manifest is not a regular file");
  const bytes = await readFile(manifestPath);
  const manifest = JSON.parse(bytes.toString("utf8"));
  closed(manifest, ["schemaVersion", "generator", "caseId", "rows", "columns", "frames",
    "measurementSize", "photometric", "samplesPerPixel", "bitsAllocated", "bitsStored",
    "seriesUid", "pixelPattern", "serverCacheMiB", "profile", "calibrationDirectory",
    "calibrationFiles", "targetDirectory", "targetFiles"], "controlled input manifest");
  if (manifest.schemaVersion !== 1 || manifest.generator !== "scripts/corpus_synth.py:comparison-colour-v1" ||
      manifest.caseId !== caseId || manifest.measurementSize !== true || manifest.pixelPattern !== "position-frame-v1" ||
      manifest.calibrationDirectory !== "calibration-ct" || manifest.targetDirectory !== "colour" ||
      !/^2\.25\.[0-9]+$/.test(manifest.seriesUid)) {
    throw new Error("the controlled input declaration does not name the measurement dataset");
  }
  for (const field of ["rows", "columns", "frames", "photometric", "samplesPerPixel",
    "bitsAllocated", "bitsStored", "serverCacheMiB"]) {
    if (manifest[field] !== spec[field]) throw new Error(`the controlled input ${field} differs from its case`);
  }
  if (spec.profileKind === null) {
    if (manifest.profile !== null) throw new Error("an unprofiled case declares a profile");
  } else {
    closed(manifest.profile, ["kind", "sha256", "bytes"], "controlled profile");
    if (manifest.profile.kind !== spec.profileKind || !/^[a-f0-9]{64}$/.test(manifest.profile.sha256) ||
        manifest.profile.bytes !== 560) throw new Error("the controlled profile differs from its case");
  }
  const identities = [];
  for (const [group, folder, count] of [[manifest.calibrationFiles, "calibration-ct", 1],
    [manifest.targetFiles, "colour", spec.frames]]) {
    if (!Array.isArray(group) || group.length !== count) throw new Error("the controlled series has the wrong file count");
    if (!(await lstat(join(root, folder))).isDirectory()) throw new Error("a controlled series directory is not a plain directory");
    const actualNames = (await readdir(join(root, folder))).sort();
    const expectedNames = Array.from({ length: count }, (_, index) => `${String(index).padStart(6, "0")}.dcm`);
    if (JSON.stringify(actualNames) !== JSON.stringify(expectedNames)) throw new Error("the controlled series inventory differs from its declaration");
    for (let index = 0; index < count; index += 1) {
      const entry = group[index];
      closed(entry, ["path", "bytes", "sha256"], "controlled file");
      const expected = `${folder}/${expectedNames[index]}`;
      const path = resolve(root, entry.path);
      const outside = relative(root, path);
      if (entry.path !== expected || outside.startsWith("..") || isAbsolute(outside) ||
          !/^[a-f0-9]{64}$/.test(entry.sha256) || !Number.isSafeInteger(entry.bytes) || entry.bytes < 1) {
        throw new Error("a controlled file identity is invalid");
      }
      const info = await lstat(path);
      if (!info.isFile() || info.size !== entry.bytes || await fileDigest(path) !== entry.sha256) {
        throw new Error("a controlled file does not match its declared bytes");
      }
      identities.push(`${entry.path}\0${entry.sha256}\0${entry.bytes}\n`);
    }
  }
  return { spec, targetDirectory: join(root, "colour"), calibrationDirectory: join(root, "calibration-ct"),
    publicIdentity: { manifest_sha256: hashBytes(bytes), dataset_sha256: hashBytes(identities.join("")),
      series_sha256: hashBytes(manifest.seriesUid), profile: manifest.profile } };
}

function calibrationProof(context) {
  const proof = context?.pixelPrecheck;
  if (proof?.verdict?.passes !== true || proof.verdict.low_information !== false) {
    throw new Error("the unchanged CT pixel calibration must pass before colour measurements");
  }
  for (const key of ["input_identity", "browser", "versions", "defaults"]) {
    if (proof[key] === undefined) throw new Error(`the CT pixel calibration lacks ${key}`);
  }
  return globalThis.structuredClone(proof);
}

function executionIdentity(context) {
  closed(context.host, ["platform", "release", "arch", "cpu_model", "cpu_count", "memory_bytes"], "measurement host");
  for (const key of ["platform", "release", "arch", "cpu_model"]) {
    if (typeof context.host[key] !== "string" || !context.host[key]) throw new Error("measurement host identity is incomplete");
  }
  for (const key of ["cpu_count", "memory_bytes"]) {
    if (!Number.isSafeInteger(context.host[key]) || context.host[key] < 1) throw new Error("measurement host capacity is invalid");
  }
  closed(context.source, ["head", "index_tree", "worktree_matches_index", "binary_sha256", "comparator_sha256", "page_sha256"], "measurement source");
  for (const key of ["head", "index_tree"]) {
    if (!/^(?:[a-f0-9]{40}|[a-f0-9]{64})$/.test(context.source[key])) throw new Error("measurement source Git identity is incomplete");
  }
  for (const key of ["binary_sha256", "comparator_sha256", "page_sha256"]) {
    if (!/^[a-f0-9]{64}$/.test(context.source[key])) throw new Error("measurement artifact identity is incomplete");
  }
  if (context.source.worktree_matches_index !== true || typeof context.startedAt !== "string" ||
      !Number.isFinite(Date.parse(context.startedAt))) throw new Error("measurement source is dirty or start identity is absent");
  return { machine: globalThis.structuredClone(context.host), source: globalThis.structuredClone(context.source), started_at: context.startedAt };
}

function budget(side, bytes, spec) {
  if (side === "ocelli" && bytes !== spec.serverCacheMiB * MIB) {
    throw new Error("the actual native cache budget differs from the controlled case");
  }
  if (side === "cornerstone" && bytes !== null) throw new Error("a cornerstone receipt claims a native cache budget");
}

function trialReceipt(result, side, warm, trial, spec) {
  finite(result.open_ms, "open"); finite(result.first_image_ms, "first image");
  if (typeof result.browser_version !== "string" || !result.browser_version ||
      typeof result.renderer !== "string" || !result.renderer) throw new Error("a colour trial lacks its actual browser or renderer");
  budget(side, result.native_budget_bytes, spec);
  return { side, warm, trial, open_ms: result.open_ms, first_image_ms: result.first_image_ms,
    browser_version: result.browser_version, renderer: result.renderer, native_budget_bytes: result.native_budget_bytes };
}

function processSample(rows) {
  if (!Array.isArray(rows) || !rows.length) throw new Error("a memory process sample is empty");
  const seen = new Map();
  for (const row of rows) {
    if (!Array.isArray(row) || row.length !== 2 || !Number.isSafeInteger(row[0]) || row[0] < 1 || seen.has(row[0])) {
      throw new Error("a memory process sample has an invalid PID");
    }
    closed(row[1], ["footprint", "peak"], "process memory");
    finite(row[1].footprint, "process memory footprint"); finite(row[1].peak, "process memory peak");
    if (row[1].footprint <= 0 || row[1].peak < row[1].footprint) throw new Error("a memory process sample has impossible footprint or peak");
    seen.set(row[0], row[1]);
  }
  return seen;
}

function memoryReceipt(result, side, trial, spec) {
  budget(side, result.native_budget_bytes, spec);
  finite(result.figures?.sampled_peak, "sampled concurrent memory");
  finite(result.figures?.sum_of_peaks, "sum of process peaks");
  if (!Array.isArray(result.samples) || result.samples.length === 0 ||
      !Array.isArray(result.sample_times_ms) || result.sample_times_ms.length !== result.samples.length ||
      !Array.isArray(result.before) || !result.before.length || !Array.isArray(result.after) || !result.after.length ||
      !Array.isArray(result.infrastructure) || result.infrastructure.length !== result.samples.length ||
      !["measured", "invalid"].includes(result.cpu?.state)) {
    throw new Error("the all-process memory session lacks its actual process receipts");
  }
  const samples = [];
  for (const [index, rows] of result.samples.entries()) {
    samples.push(processSample(rows));
    processSample(result.infrastructure[index]);
    finite(result.sample_times_ms[index], "memory sample time");
    if (index && result.sample_times_ms[index] < result.sample_times_ms[index - 1]) throw new Error("memory sample times are not ordered");
  }
  for (const snapshot of [result.before, result.after]) {
    const seen = new Set();
    for (const row of snapshot) {
      if (!Array.isArray(row) || row.length !== 2 || !Number.isSafeInteger(row[0]) || row[0] < 1 || seen.has(row[0])) {
        throw new Error("the CPU process inventory is invalid");
      }
      seen.add(row[0]); finite(row[1], "process CPU");
    }
  }
  const actual = memoryFigures(samples);
  if (actual.sampled_peak !== result.figures.sampled_peak || actual.sum_of_peaks !== result.figures.sum_of_peaks) {
    throw new Error("the memory figures differ from their process receipts");
  }
  if (result.cpu.state === "measured") finite(result.cpu.seconds, "session CPU");
  else if (typeof result.cpu.reason !== "string" || !result.cpu.reason) throw new Error("an invalid CPU session lacks a reason");
  return { ...globalThis.structuredClone(result), side, trial };
}

/** The prepared core is also used by fake-tool controls, without a browser or large files. */
export async function collectPreparedColourComparison(input, context, tools) {
  const proof = calibrationProof(context);
  const identity = executionIdentity(context);
  const spec = colourCaseSpec(input.spec.id);
  for (const key of Object.keys(spec)) if (input.spec[key] !== spec[key]) throw new Error("the prepared colour case differs from its closed definition");
  closed(input.publicIdentity, ["manifest_sha256", "dataset_sha256", "series_sha256", "profile"], "prepared input identity");
  for (const key of ["manifest_sha256", "dataset_sha256", "series_sha256"]) {
    if (!/^[a-f0-9]{64}$/.test(input.publicIdentity[key])) throw new Error("the prepared input lacks a complete digest");
  }
  if (spec.profileKind === null ? input.publicIdentity.profile !== null
    : input.publicIdentity.profile?.kind !== spec.profileKind) {
    throw new Error("the prepared profile does not match the controlled case");
  }
  const caseContext = { ...context, colourCase: spec };
  const trials = [];
  for (const warm of [false, true]) {
    for (let index = 0; index < 9; index += 1) {
      const order = index % 2 ? [...SIDES].reverse() : SIDES;
      for (const side of order) {
        const result = await tools.measureColourTrial(side, { directory: input.targetDirectory,
          warm, caseSpec: spec, context: caseContext });
        trials.push(trialReceipt(result, side, warm, index + 1, spec));
      }
    }
  }
  if (new Set(trials.map(trial => trial.browser_version)).size !== 1) {
    throw new Error("controlled colour trials used different browser builds");
  }
  const actors = {};
  const windows = [];
  let windowFailure;
  try {
    for (const side of SIDES) {
      actors[side] = await tools.launchSide(side, caseContext);
      budget(side, actors[side].nativeBudgetBytes, spec);
      await tools.openFolder(actors[side], input.targetDirectory);
    }
    for (let index = 0; index < COLOUR_WINDOWS.length; index += 1) {
      const step = COLOUR_WINDOWS[index];
      for (const side of index % 2 ? [...SIDES].reverse() : SIDES) {
        const elapsed = await tools.windowStep(actors[side], step.center, step.width);
        windows.push({ side, step: index + 1, ...step, elapsed_ms: finite(elapsed, "window step") });
      }
    }
  } catch (error) { windowFailure = error; }
  const closes = await Promise.allSettled(Object.values(actors).map(actor => Promise.resolve().then(() => actor.close())));
  if (windowFailure) throw windowFailure;
  const failed = closes.find(result => result.status === "rejected");
  if (failed) throw failed.reason;
  const memory = [];
  for (let index = 0; index < 3; index += 1) {
    for (const side of index % 2 ? [...SIDES].reverse() : SIDES) {
      const result = await tools.measureColourMemorySession(side, { directory: input.targetDirectory,
        caseSpec: spec, context: caseContext });
      memory.push(memoryReceipt(result, side, index + 1, spec));
    }
  }
  const summary = {};
  for (const side of SIDES) {
    const own = trials.filter(trial => trial.side === side);
    summary[side] = {
      open_cold_ms: summarise(own.filter(trial => !trial.warm).map(trial => trial.open_ms), 9),
      open_warm_ms: summarise(own.filter(trial => trial.warm).map(trial => trial.open_ms), 9),
      first_cold_ms: summarise(own.filter(trial => !trial.warm).map(trial => trial.first_image_ms), 9),
      first_warm_ms: summarise(own.filter(trial => trial.warm).map(trial => trial.first_image_ms), 9),
      window_ms: summarise(windows.filter(step => step.side === side).map(step => step.elapsed_ms), 9),
      sampled_peak_bytes: summarise(memory.filter(row => row.side === side).map(row => row.figures.sampled_peak), 3),
      sum_of_process_peaks_bytes: summarise(memory.filter(row => row.side === side).map(row => row.figures.sum_of_peaks), 3),
    };
  }
  const record = { format: FORMAT, ...identity, caseId: spec.id, case: spec,
    input: globalThis.structuredClone(input.publicIdentity), calibration: proof,
    colour_pixel_score: "not-scored", profile_semantics: spec.profileKind === null
      ? "same unprofiled controlled input" : "Ocelli applies ICC, cornerstone draws device RGB",
    trials, window_steps: windows, memory, summary };
  const text = ["# Controlled colour comparison", "", `Case: ${spec.id}.`,
    "CT pixel calibration passed before measurement. Colour has no scored threshold.",
    record.profile_semantics, "", "| Side | Cold open ms | Warm open ms | Cold first ms | Warm first ms | Window ms | Sampled peak bytes |",
    "|---|---:|---:|---:|---:|---:|---:|"];
  for (const side of SIDES) {
    const s = summary[side];
    text.push(`| ${side} | ${s.open_cold_ms.median} | ${s.open_warm_ms.median} | ${s.first_cold_ms.median} | ${s.first_warm_ms.median} | ${s.window_ms.median} | ${s.sampled_peak_bytes.median} |`);
  }
  text.push("", "Raw alternating trials and all-process memory receipts are in the accompanying JSON.", "No baseline or published five-workload record is changed.", "");
  return { record, text: text.join("\n") };
}

export async function measureColourComparison({ caseId, context, directory, tools }) {
  calibrationProof(context);
  const input = await readColourComparisonInput(directory, caseId);
  return collectPreparedColourComparison(input, context, tools);
}
