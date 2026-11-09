// The comparison mode's pure logic: parsing, statistics, selection, the
// closed published schema and the two renderers.
//
// F-223's `--compare-cornerstone` compares the local folder viewer against
// cornerstone3D 5.8.2 on one machine, in one run, with the same browser
// build. Every function in this file is pure: no browser, no process, no
// file system. The orchestration around them lives in `comparison_run.mjs`,
// which imports this module, so the floor suite can test every rule here
// without a browser or an oracle install.
//
// Three rules shape the file.
//
// **No pixel arithmetic.** The grey values compared are each side's own
// output, and the one piece of arithmetic performed over them, the 25.1
// verdict, is the oracle's `ocelli-compare pair`. Nothing here recomputes a
// window function or a pixel difference, which would be a second copy of
// the tolerance and the exact defect the oracle's comments warn about.
//
// **The definitions live in comparison.json and the vocabulary lives here.**
// `loadComparison` refuses a file whose side list, workload rules, metric
// ids or units disagree with the constants below, so the file and the code
// cannot drift apart unseen. The reverse binding is
// `tests/comparison_test.mjs`, which asserts the file still matches the
// plan's metric table.
//
// **A difference is stated only where the measured spreads separate.** The
// summary prints medians, spreads and, where the interquartile ranges do
// not overlap, a ratio to three significant figures. It never prints an
// adjective, because the acknowledgement paragraph's tone is a standing
// rule of the repository and a number with an adjective bolted on is a
// ranking wearing a measurement's clothes.

import { createHash } from "node:crypto";

// `atClockPrecision` is imported and not copied. It is the harness's one
// rounding decision and it lives in wasm_cold_start.mjs, whose playwright
// import sits inside run() precisely so that pure importers such as this
// file and browser_session.mjs stay loadable without a browser. HLD 27.3
// makes every rounding decision a review item, and two copies of one is
// how a review item becomes two.
import { atClockPrecision } from "./runners/wasm_cold_start.mjs";

/** Minimum pause between complete process-memory sampling rounds, in ms. */
export const FOOTPRINT_PERIOD = 250;

/** The two sides, in trial order. The alternation starts from the first. */
export const SIDES = ["ocelli", "cornerstone"];

/** The Ocelli side's label, as the metric records spell it. */
export const OCELLI = "ocelli";

/** The cornerstone3D side's label, as the metric records spell it. */
export const CORNERSTONE = "cornerstone";

/** The five workloads `selectWorkloads` can return, in definition order. */
export const WORKLOAD_IDS = [
  "synthetic-ct",
  "synthetic-dx",
  "pseudo-full",
  "pseudo-ct",
  "pseudo-dx",
];

/**
 * The memory session's label. It selects nothing: it is the one page the
 * peak memory and session CPU figures are taken in, and the two metrics
 * that name it say so in comparison.json.
 */
export const SESSION_WORKLOAD = "session";

/** Store one explicitly named condition once, before it can overwrite a figure. */
export function recordCondition(metrics, metric, workload, condition, sides) {
  if (!METRIC_IDS.includes(metric) || !WORKLOAD_LABELS.includes(workload) || !["cold", "warm"].includes(condition)) throw new Error("unknown comparison metric, workload or condition");
  const label = `${workload}/${condition}`;
  metrics[metric] ||= {};
  if (Object.hasOwn(metrics[metric], label)) throw new Error("duplicate comparison condition");
  metrics[metric][label] = sides;
}


/** Every workload label a metric may name. */
export const WORKLOAD_LABELS = [...WORKLOAD_IDS, SESSION_WORKLOAD];

/**
 * The five workload rules, in the words the plan fixed. `loadComparison`
 * holds comparison.json to exactly these sentences and
 * `selectWorkloads` returns them verbatim as the unavailable reason, so a
 * rule that selects nothing reports the rule itself.
 */
const WORKLOAD_RULES = [
  {
    id: "synthetic-ct",
    rule:
      "the synthetic CT series ensureWorkloads generated: 512 by 512 16-bit " +
      "CT files with stored value 50 everywhere",
  },
  {
    id: "synthetic-dx",
    rule:
      "the synthetic DX object ensureWorkloads generated: 3000 by 2900 with " +
      "stored value 50 everywhere",
  },
  { id: "pseudo-full", rule: "every file under the pseudo-PHI root" },
  {
    id: "pseudo-ct",
    rule:
      "the CT series with the most images among series whose every image is " +
      "MONOCHROME2, 512 by 512, carrying Window Center and Window Width, " +
      "ties broken by the smaller digest of the series' sorted content " +
      "digests",
  },
  {
    id: "pseudo-dx",
    rule:
      "the DX object with the most pixels, ties broken by the smaller " +
      "digest of the object's sorted content digests",
  },
];

/** The seven metrics, in definition order. */
export const METRIC_IDS = [
  "page_ready",
  "open",
  "first_image",
  "scroll",
  "window_change",
  "peak_memory",
  "session_cpu",
];

/**
 * The unit per metric. Milliseconds for durations, frames per second for
 * scroll, bytes for memory, seconds for CPU. loadComparison holds
 * comparison.json to exactly this table, so the definitions file and the
 * renderer cannot disagree about a unit.
 */
export const METRIC_UNITS = {
  page_ready: "ms",
  open: "ms",
  first_image: "ms",
  scroll: "frames_per_second",
  window_change: "ms",
  peak_memory: "bytes",
  session_cpu: "seconds",
};

/**
 * The architectural difference, carried verbatim as the first paragraph of
 * the tracked summary and of the plain report, and as a top-level field in
 * both JSON files. The run records it and both renderers require it, so
 * neither output can omit it.
 */
export const ARCHITECTURE =
  "The two sides do different work in different places. The Ocelli folder " +
  "viewer parses and decodes each file in a native process on this machine " +
  "and sends 8-bit grey frames over loopback to the page, which draws them " +
  "on a 2D canvas on the CPU. The cornerstone3D page parses and decodes in " +
  "the browser, in dicom-image-loader's WebAssembly workers, and renders " +
  "through WebGL. Memory and CPU are therefore counted over every process " +
  "each side runs: for Ocelli the native process and every Chromium " +
  "process, and for cornerstone3D every Chromium process, including the " +
  "GPU process. This measures the local folder viewer example, not the " +
  "Ocelli browser core, which has no viewer page to measure yet.";

/**
 * The fixed acknowledgement, the second paragraph of the summary. It is
 * rendered identically whatever the numbers say, which
 * tests/comparison_test.mjs checks from both directions.
 */
export const ACKNOWLEDGEMENT =
  "cornerstone3D 5.8.2 is the reference this project learns from and " +
  "validates against. This page records cost on one machine, for one set " +
  "of inputs, at each side's shipped defaults. It is a measurement, not a " +
  "ranking.";

/**
 * The tier, as its own sentence, with the WebGL renderer NAMED in it and the
 * mode the browser ran in recorded beside it. Fact 15 makes the tier
 * sentence's "the named WebGL renderer" a promise about the output, and a
 * renderer nobody named is a renderer nobody can check. The mode is
 * evidence too: fact 15 records `headed` in the output when the probe made
 * the mode launch headed, and a choice that changed the measurement is
 * recorded whether or not it was the first choice.
 *
 * @param {object} browser the closed browser block
 * @returns {string}
 */
export function tierSentence(browser) {
  if (browser === null || typeof browser !== "object" ||
    typeof browser.renderer !== "string" || browser.renderer.length === 0 ||
    typeof browser.headed !== "boolean") {
    throw new Error("the tier sentence needs the closed browser block");
  }
  const mode = browser.headed ? "headed" : "headless";
  return "The tier: Ocelli tier C on the CPU, cornerstone3D on " +
    `${browser.renderer}. The browser ran ${mode}.`;
}

/**
 * What the numbers do not show, fixed, rendered as its own section. The
 * published record carries this list and `publishedRecord` refuses a run
 * whose list differs, because a caveat that can be edited away is a caveat
 * that will be.
 */
export const NOT_SHOWN = [
  "one machine and one browser build",
  "loopback only and no network",
  "a warm operating system file cache",
  "different default budgets on the two sides",
  "a comparator page written for this measurement rather than an " +
    "application such as OHIF",
  "the matched prefetch policy",
  "a comparator-only adapter wraps valid VOI strings for loader5.8.2, with pinned package bytes and cache and worker defaults unchanged",
  "uncompressed inputs only so no codec cost",
  "a 2D canvas against WebGL",
  "the tier: Ocelli tier C on the CPU, cornerstone3D on the named WebGL " +
    "renderer",
];

const SUMMARY_HEADER =
  "<!-- Written by bin/ocelli.sh bench --compare-cornerstone --publish. " +
  "Do not edit. -->";

const METRIC_DISPLAY = {
  page_ready: "page ready",
  open: "open",
  first_image: "first image",
  scroll: "scroll",
  window_change: "window change",
  peak_memory: "peak memory",
  session_cpu: "session CPU",
};

const UNIT_WORDS = {
  ms: "milliseconds",
  frames_per_second: "presented frames per second",
  bytes: "MiB, from the exact bytes the JSON records",
  seconds: "seconds",
};

const WORKLOAD_DISPLAY = {
  "synthetic-ct": "synthetic CT",
  "synthetic-dx": "synthetic DX",
  "pseudo-full": "pseudo full",
  "pseudo-ct": "pseudo CT",
  "pseudo-dx": "pseudo DX",
  session: "the session",
};

const SIDE_DISPLAY = { ocelli: "Ocelli", cornerstone: "cornerstone3D" };

const DEFAULT_LABELS = {
  serverCacheBytes: "server cache bytes",
  pageGreyCacheBytes: "page grey cache bytes",
  decodePermits: "decode permits",
  maxCacheSizeBytes: "max cache size bytes",
  decodeWorkers: "decode workers",
};

const DEFAULT_ORDER = {
  ocelli: ["serverCacheBytes", "pageGreyCacheBytes", "decodePermits"],
  cornerstone: ["maxCacheSizeBytes", "decodeWorkers"],
};

// The published schema's closed key sets. A key outside its set refuses the
// publication, so a value cannot arrive through a field nobody expected,
// which is the free-text leak the closed schema exists to close.
const HOST_CLASS_KEYS = ["platform", "release", "arch", "cpu_model",
  "cpu_count", "memory_bytes"];
const VERSION_KEYS = ["chromium", "playwright", "@cornerstonejs/core",
  "@cornerstonejs/dicom-image-loader"];
// Fact 15: the pages report `UNMASKED_RENDERER_WEBGL`, the run refuses two
// differing renderer strings, a SwiftShader renderer needs `--software`, and
// the mode records `headed` when the probe made it launch headed. The
// renderer is the one string both sides reported, because a run whose sides
// disagreed never reaches a publication.
const BROWSER_KEYS = ["renderer", "headed", "software"];
const METRIC_CELL_KEYS = ["state", "n", "median", "p25", "p75", "min", "max",
  "reason", "upper_bound"];
const METRIC_CELL_STATES = ["measured", "unavailable", "invalid"];
const PAIR_VERDICT_KEYS = ["within_one_lsb_fraction", "count_over_max",
  "max_abs_diff", "signed_mean_bias_informative", "informative_fraction",
  "low_information", "passes"];
const WORKLOAD_CELL_KEYS = ["state", "count", "reason"];
const WORKLOAD_STATES = ["selected", "unavailable"];

/**
 * Parse and validate comparison.json.
 *
 * The definitions are data and the vocabulary is code, so this function is
 * the binding: a file whose sides, workload rules, metric ids, units or
 * trial counts disagree with the constants above is refused rather than
 * half-honoured. Returns a plain copy of the three definitions.
 *
 * @param {string} text the file's text
 * @returns {object} { sides, workloads, metrics }
 */
export function loadComparison(text) {
  let parsed;
  try {
    parsed = JSON.parse(text);
  } catch (error) {
    throw new Error(`comparison.json is not valid JSON: ${error.message}`);
  }
  if (parsed === null || typeof parsed !== "object" || Array.isArray(parsed)) {
    throw new Error("comparison.json is not an object");
  }
  if (!Array.isArray(parsed.sides) || parsed.sides.join("\u0000") !==
    SIDES.join("\u0000")) {
    throw new Error(
      `comparison.json must carry exactly the two sides [${SIDES.join(", ")}] ` +
        `in trial order, got ${JSON.stringify(parsed.sides)}`,
    );
  }
  const workloads = parsed.workloads;
  if (!Array.isArray(workloads) ||
    !sameWorkloads(workloads, WORKLOAD_RULES)) {
    throw new Error(
      "comparison.json must define exactly the five workloads the code " +
        "selects, with the rule sentences the plan fixed. The rules live in " +
        "both places on purpose, and a change to one is a change to both.",
    );
  }
  if (!Array.isArray(parsed.metrics)) {
    throw new Error("comparison.json carries no metrics array");
  }
  const ids = parsed.metrics.map((metric) => metric?.id);
  const unknown = ids.filter((id) => !METRIC_IDS.includes(id));
  if (unknown.length > 0) {
    throw new Error(
      `comparison.json names metrics the code does not know: ${unknown.join(", ")}`,
    );
  }
  if (new Set(ids).size !== ids.length) {
    throw new Error("comparison.json names a metric twice");
  }
  if (ids.length !== METRIC_IDS.length) {
    throw new Error(
      `comparison.json must define all ${METRIC_IDS.length} metrics, got ` +
        `${ids.length}`,
    );
  }
  const metrics = parsed.metrics.map((metric) => {
    const { id } = metric;
    if (metric.mirrors !== null &&
      !(typeof metric.mirrors === "string" &&
        metric.mirrors.startsWith("viewer."))) {
      throw new Error(
        `the ${id} metric mirrors neither null nor a viewer.* subject`,
      );
    }
    if (metric.unit !== METRIC_UNITS[id]) {
      throw new Error(
        `the ${id} metric's unit is ${JSON.stringify(metric.unit)} and the ` +
          `code renders ${JSON.stringify(METRIC_UNITS[id])}`,
      );
    }
    if (!Number.isInteger(metric.n) || metric.n < 1) {
      throw new Error(
        `the ${id} metric's declared N must be a whole count of trials, got ` +
          `${JSON.stringify(metric.n)}`,
      );
    }
    const labels = metric.workloads;
    if (!Array.isArray(labels)) {
      throw new Error(`the ${id} metric's workloads are not a list`);
    }
    if (new Set(labels).size !== labels.length) {
      throw new Error(`the ${id} metric names a workload label twice`);
    }
    const unknownLabel = labels.find((label) =>
      !WORKLOAD_LABELS.includes(label));
    if (unknownLabel !== undefined) {
      throw new Error(
        `the ${id} metric names the unknown workload label ` +
          `${JSON.stringify(unknownLabel)}, and the known labels are ` +
          `${WORKLOAD_LABELS.join(", ")}`,
      );
    }
    validateAvailability(id, "cold", metric.cold, ["yes", "unavailable"]);
    validateAvailability(id, "warm", metric.warm, ["yes", "n/a", "unavailable"]);
    for (const endpoint of ["start", "end"]) {
      if (typeof metric[endpoint] !== "string" || metric[endpoint].length === 0) {
        throw new Error(`the ${id} metric's ${endpoint} is missing`);
      }
    }
    return { ...metric };
  });
  return {
    sides: [...SIDES],
    workloads: workloads.map((one) => ({ ...one })),
    metrics,
  };
}

function sameWorkloads(file, code) {
  return file.length === code.length && code.every((rule, index) =>
    file[index]?.id === rule.id && file[index]?.rule === rule.rule);
}

function validateAvailability(id, kind, availability, states) {
  const state = availability?.state;
  if (!states.includes(state)) {
    throw new Error(
      `the ${id} metric's ${kind} state must be one of ${states.join(", ")}`,
    );
  }
  if (state === "unavailable" &&
    (typeof availability.reason !== "string" || availability.reason.length === 0)) {
    throw new Error(
      `the ${id} metric's ${kind} is unavailable and names no reason, and ` +
        "an unavailable metric without its reason is an omission dressed as " +
        "an answer",
    );
  }
}

/**
 * The trial order for one workload: the two sides alternating, starting
 * from `startSide`, so slow drift lands on both sides equally. The caller
 * flips the starting side between workloads.
 *
 * @param {string[]} sides exactly two distinct side labels
 * @param {number} n the trial count, at least 1
 * @param {string} startSide the side trial one belongs to
 * @returns {string[]} n side labels, alternating
 */
export function trialOrder(sides, n, startSide) {
  if (!Array.isArray(sides) || sides.length !== 2 ||
    new Set(sides).size !== 2) {
    throw new Error("trialOrder needs exactly two sides, and distinct ones");
  }
  if (!Number.isInteger(n) || n < 1) {
    throw new Error(`the trial count must be at least 1, got ${n}`);
  }
  const start = sides.indexOf(startSide);
  if (start === -1) {
    throw new Error(
      `startSide ${JSON.stringify(startSide)} is not one of the sides`,
    );
  }
  return Array.from({ length: n }, (_, index) => sides[(start + index) % 2]);
}

/**
 * Summarise one metric's valid trials at one cell: median, quartiles by
 * linear interpolation between order statistics (the method numpy calls
 * `linear`), minimum and maximum.
 *
 * Fewer valid trials than the declared N is `invalid` and carries no
 * statistics at all, because a median of a partial sample is a number the
 * plan refused in advance.
 *
 * @param {number[]} values every valid trial's figure
 * @param {number} declaredN the metric's declared trial count
 */
export function summarise(values, declaredN) {
  if (!Array.isArray(values)) {
    throw new Error("summarise takes the array of a cell's valid trials");
  }
  if (!Number.isInteger(declaredN) || declaredN < 1) {
    throw new Error(`the declared N must be at least 1 trial, got ${declaredN}`);
  }
  values.forEach((value, index) => {
    if (typeof value !== "number" || !Number.isFinite(value)) {
      throw new Error(
        `summarise takes finite numbers and position ${index} is not one`,
      );
    }
  });
  const n = values.length;
  if (n < declaredN) {
    return { state: "invalid", n };
  }
  const sorted = [...values].sort((a, b) => a - b);
  const percentile = (q) => {
    const index = (n - 1) * q;
    const lower = Math.floor(index);
    const fraction = index - lower;
    if (lower + 1 >= n) {
      return sorted[lower];
    }
    return sorted[lower] + fraction * (sorted[lower + 1] - sorted[lower]);
  };
  return {
    state: "measured",
    n,
    median: percentile(0.5),
    p25: percentile(0.25),
    p75: percentile(0.75),
    min: sorted[0],
    max: sorted[n - 1],
  };
}

/**
 * Whether a difference can be stated: only when the two sides'
 * interquartile ranges do not overlap. Ranges that touch, p75 of one equal
 * to p25 of the other, are not disjoint and so are not separated.
 *
 * @param {object} a one side's summary
 * @param {object} b the other side's summary
 */
export function separated(a, b) {
  for (const [label, summary] of [["a", a], ["b", b]]) {
    if (summary?.state !== "measured") {
      throw new Error(
        `separated needs two measured summaries and ${label} is ` +
          `${JSON.stringify(summary?.state)}, which has no spread to compare`,
      );
    }
    if (typeof summary.p25 !== "number" || typeof summary.p75 !== "number") {
      throw new Error(
        `a measured summary without p25 and p75 has no interquartile range ` +
          `to separate ${label} on`,
      );
    }
  }
  return a.p75 < b.p25 || b.p75 < a.p25;
}

/**
 * The set of pids rooted at `rootPid` in `ps -A -o pid=,ppid=` output: the
 * root itself and every descendant, to any depth. A root the text names
 * neither as a pid nor as a parent has no descendants to add and returns as
 * a set of one, and the empty text is the simplest such case. A root named
 * only as a parent still gathers its children. Lines that do not parse as
 * two integers are skipped.
 *
 * @param {string} psText the ps output
 * @param {number} rootPid the side's own root process
 * @returns {Set<number>}
 */
export function parseProcessTree(psText, rootPid) {
  if (typeof psText !== "string") {
    throw new Error("parseProcessTree takes ps -A -o pid=,ppid= output");
  }
  if (!Number.isInteger(rootPid) || rootPid < 1) {
    throw new Error(`rootPid must be a process id, got ${rootPid}`);
  }
  const parentOf = new Map();
  for (const line of psText.split("\n")) {
    const match = /^\s*(\d+)\s+(\d+)\s*$/.exec(line);
    if (match) {
      parentOf.set(Number(match[1]), Number(match[2]));
    }
  }
  const tree = new Set([rootPid]);
  const frontier = [rootPid];
  while (frontier.length > 0) {
    const pid = frontier.pop();
    for (const [child, parent] of parentOf) {
      if (parent === pid && !tree.has(child)) {
        tree.add(child);
        frontier.push(child);
      }
    }
  }
  return tree;
}

const FOOTPRINT_LINE = /^\s*phys_footprint:\s*(\d+)\s*B\s*$/;
const FOOTPRINT_PEAK_LINE = /^\s*phys_footprint_peak:\s*(\d+)\s*B\s*$/;

// The per-process header the tool prints between `====` rules:
// `<name> [<pid>]: 64-bit    Footprint: <n> B (16384 bytes per page)`. The
// `-bit Footprint:` tail is what separates a process header from the
// `Shared with <name> [<pid>]:` and
// `Shared with <name> [<pid>], <name> [<pid>]:` sub-blocks the tool also
// prints inside a process's section, which a regex keyed on `[<pid>]` alone
// would re-key on.
const FOOTPRINT_PROCESS = /^\S.*\[(\d+)\]: \d+-bit\s+Footprint:/;
const FOOTPRINT_SUMMARY = /^Summary Footprint:/;

/**
 * One `/usr/bin/footprint -f bytes -p <pid> -p <pid>` call's output: the
 * real shape, captured on this host from two throwaway processes and
 * recorded as the fixture in tests/comparison_test.mjs. Per
 * process the tool prints a header naming the pid, a per-category table,
 * optional `Shared with` sub-blocks, and then `Auxiliary data:` carrying
 * `phys_footprint: <n> B` and `phys_footprint_peak: <n> B`. `-f bytes`
 * alone prints both, and no other flag is needed. The exact command the
 * fixture was captured with is
 * `/usr/bin/footprint -f bytes -p <pid> -p <pid>`. The tool closes with a
 * `Summary Footprint: <n> B` line, the de-duplicated total across the
 * processes named, which is not the sum of the per-process figures when
 * they share pages and is not read again.
 *
 * An input with no parsable process is a refusal and not an empty Map,
 * because an empty Map reads as a measured zero downstream and a session
 * whose collector captured nothing is not a measurement of zero.
 *
 * @param {string} text the tool's output
 * @returns {Map<number, {footprint: number, peak: number}>}
 */
export function parseFootprint(text) {
  if (typeof text !== "string") {
    throw new Error("parseFootprint takes footprint -f bytes output");
  }
  const figures = new Map();
  let current = null;
  for (const line of text.split("\n")) {
    if (FOOTPRINT_SUMMARY.test(line)) {
      // The summary section is the tool's last. Anything after it belongs to
      // the de-duplicated total and not to the last process named.
      current = null;
      continue;
    }
    const header = FOOTPRINT_PROCESS.exec(line);
    if (header) {
      const pid = Number(header[1]);
      if (figures.has(pid)) {
        throw new Error(`footprint printed process ${pid} twice`);
      }
      current = { footprint: null, peak: null };
      figures.set(pid, current);
      continue;
    }
    if (current === null) {
      continue;
    }
    const footprint = FOOTPRINT_LINE.exec(line);
    if (footprint) {
      current.footprint = Number(footprint[1]);
      continue;
    }
    const peak = FOOTPRINT_PEAK_LINE.exec(line);
    if (peak) {
      current.peak = Number(peak[1]);
    }
  }
  if (figures.size === 0) {
    throw new Error(
      "footprint printed no process block: a header shaped " +
        "`<name> [<pid>]: 64-bit    Footprint:` was found nowhere, so there " +
        "is no phys_footprint to read and an empty inventory is a refusal, " +
        "not a measurement of zero",
    );
  }
  for (const [pid, { footprint, peak }] of figures) {
    if (footprint === null || peak === null) {
      const missing = [
        footprint === null ? "phys_footprint" : null,
        peak === null ? "phys_footprint_peak" : null,
      ].filter(Boolean).join(" and no ");
      throw new Error(`footprint printed no ${missing} for process ${pid}`);
    }
  }
  return figures;
}

/**
 * The two memory figures over one side's samples, at least 250 ms apart.
 *
 * The **sampled concurrent peak** is the largest per-sample sum of
 * `phys_footprint`, a lower bound at the sampling interval. The **sum of
 * per-process peaks** is the sum over pids of the largest
 * `phys_footprint_peak` each pid ever reached, an upper bound because
 * peaks of different processes need not coincide.
 *
 * @param {Map<number, {footprint: number, peak: number}>[]} samples
 * @returns {{sampled_peak: number, sum_of_peaks: number}}
 */
export function memoryFigures(samples) {
  if (!Array.isArray(samples) || samples.length === 0) {
    throw new Error("memory figures need at least one sample");
  }
  let sampledPeak = 0;
  let first = true;
  const peaks = new Map();
  for (const sample of samples) {
    if (!(sample instanceof Map) || sample.size === 0) {
      throw new Error("each memory sample is a non-empty pid to figures map");
    }
    let sum = 0;
    for (const [pid, figures] of sample) {
      if (!Number.isFinite(figures?.footprint) ||
        !Number.isFinite(figures?.peak)) {
        throw new Error(
          `pid ${pid} has no finite footprint and peak pair to sum`,
        );
      }
      sum += figures.footprint;
      const known = peaks.get(pid);
      if (known === undefined || figures.peak > known) {
        peaks.set(pid, figures.peak);
      }
    }
    if (first || sum > sampledPeak) {
      sampledPeak = sum;
      first = false;
    }
  }
  let sumOfPeaks = 0;
  for (const peak of peaks.values()) {
    sumOfPeaks += peak;
  }
  return { sampled_peak: sampledPeak, sum_of_peaks: sumOfPeaks };
}

/**
 * CPU seconds consumed by one session's inventory, from `ps -o time=` at
 * the session's start and end. The inputs are pid to CPU seconds maps.
 *
 * A process that appears or exits between the snapshots makes the session's
 * figure invalid, because a sum over a changed set is a sum over two
 * different sets of work. So does a counter that goes backwards, which no
 * correct counter does.
 *
 * @param {Map<number, number>} before pid to CPU seconds at session start
 * @param {Map<number, number>} after pid to CPU seconds at session end
 * @returns {{state: "measured", seconds: number} |
 *           {state: "invalid", reason: string}}
 */
export function cpuFigure(before, after) {
  for (const [label, inventory] of [["before", before], ["after", after]]) {
    if (!(inventory instanceof Map)) {
      throw new Error(
        `the ${label} inventory is a pid to CPU seconds map`,
      );
    }
    for (const [pid, seconds] of inventory) {
      if (typeof seconds !== "number" || !Number.isFinite(seconds)) {
        throw new Error(
          `the ${label} inventory's values must be CPU seconds, and pid ` +
            `${pid} reported ${JSON.stringify(seconds)}`,
        );
      }
    }
  }
  let exited = 0;
  let appeared = 0;
  for (const pid of before.keys()) {
    if (!after.has(pid)) {
      exited += 1;
    }
  }
  for (const pid of after.keys()) {
    if (!before.has(pid)) {
      appeared += 1;
    }
  }
  if (exited > 0 || appeared > 0) {
    return {
      state: "invalid",
      reason:
        `the process inventory changed between the snapshots: ${exited} ` +
        `exited and ${appeared} appeared`,
    };
  }
  let total = 0;
  let backwards = 0;
  for (const [pid, start] of before) {
    const delta = after.get(pid) - start;
    if (delta < 0) {
      backwards += 1;
    }
    total += delta;
  }
  if (backwards > 0) {
    return {
      state: "invalid",
      reason:
        `the CPU counter went backwards for ${backwards} process(es), which ` +
        "no correct counter does",
    };
  }
  return { state: "measured", seconds: atClockPrecision(total) };
}

// DICOM's Rows and Columns are US, unsigned 16-bit, PS3.5. A product of two
// such values is below 2**32 and far below 2**53, so the pixel-count
// comparison below loses no precision.
const MAX_US = 65535;

function validateHeader(record, index) {
  // seriesUid may be empty: a standalone object carries no series, and the
  // pseudo CT rule skips it rather than inventing a one-image series.
  const strings = ["path", "digest", "modality", "photometric"];
  const flags = ["hasWindowCenter", "hasWindowWidth"];
  const where = `header record ${index}`;
  if (record === null || typeof record !== "object") {
    throw new Error(`${where} is not a record`);
  }
  for (const field of [...strings, "source", "seriesUid"]) {
    if (typeof record[field] !== "string") {
      throw new Error(`${where} is missing ${field}`);
    }
  }
  if (record.source !== "synthetic" && record.source !== "pseudo") {
    throw new Error(`${where} has a source that is neither synthetic nor pseudo`);
  }
  for (const field of flags) {
    if (typeof record[field] !== "boolean") {
      throw new Error(`${where} is missing ${field}`);
    }
  }
  for (const field of ["rows", "columns"]) {
    if (!Number.isInteger(record[field]) || record[field] < 1 ||
      record[field] > MAX_US) {
      throw new Error(
        `${where} is missing a ${field} inside the US range of DICOM PS3.5`,
      );
    }
  }
}

const selectedWorkload = (records) => ({
  state: "selected",
  files: records.map((record) => record.path),
  count: records.length,
});

const unavailableWorkload = (rule) => ({
  state: "unavailable",
  reason: rule,
  count: 0,
});

function seriesIdentity(records) {
  return inputIdentity(records.map((record) => record.digest));
}

function selectPseudoCt(pseudo) {
  const series = new Map();
  for (const record of pseudo) {
    // A record with no Series UID is a standalone object and cannot be the
    // series the rule selects, whatever its other attributes say.
    if (record.seriesUid === "") {
      continue;
    }
    const images = series.get(record.seriesUid) ?? [];
    images.push(record);
    series.set(record.seriesUid, images);
  }
  const qualifying = [...series].filter(([, images]) => images.every((image) =>
    image.modality === "CT" &&
    image.photometric === "MONOCHROME2" &&
    image.rows === 512 &&
    image.columns === 512 &&
    image.hasWindowCenter &&
    image.hasWindowWidth));
  if (qualifying.length === 0) {
    return null;
  }
  const most = Math.max(...qualifying.map(([, images]) => images.length));
  const tied = qualifying.filter(([, images]) => images.length === most);
  if (tied.length === 1) {
    return selectedWorkload(tied[0][1]);
  }
  const winner = tied.reduce((best, entry) =>
    seriesIdentity(entry[1]) < seriesIdentity(best[1]) ? entry : best);
  return selectedWorkload(winner[1]);
}

function selectPseudoDx(pseudo) {
  const candidates = pseudo.filter((record) => record.modality === "DX");
  if (candidates.length === 0) {
    return null;
  }
  const most = Math.max(...candidates.map((record) =>
    record.rows * record.columns));
  const tied = candidates.filter((record) =>
    record.rows * record.columns === most);
  if (tied.length === 1) {
    return selectedWorkload(tied);
  }
  const winner = tied.reduce((best, record) =>
    inputIdentity([record.digest]) < inputIdentity([best.digest])
      ? record
      : best);
  return selectedWorkload([winner]);
}

/**
 * The five workload selections over one header record per file. Selection
 * is by attribute and never by path or by first found, and a rule that
 * selects nothing is unavailable with its own sentence as the reason. No
 * other input is substituted for a rule that selects nothing.
 *
 * @param {object[]} headers one record per file: source, path, digest,
 *   seriesUid, modality, rows, columns, photometric, hasWindowCenter,
 *   hasWindowWidth
 * @returns {object} the five workload ids to a selection each
 */
export function selectWorkloads(headers) {
  if (!Array.isArray(headers)) {
    throw new Error("selectWorkloads takes an array of header records");
  }
  headers.forEach((record, index) => validateHeader(record, index));
  const synthetic = headers.filter((record) => record.source === "synthetic");
  const pseudo = headers.filter((record) => record.source === "pseudo");
  const rules = Object.fromEntries(WORKLOAD_RULES.map((rule) =>
    [rule.id, rule.rule]));
  const syntheticCt = synthetic.filter((record) => record.modality === "CT");
  const syntheticDx = synthetic.filter((record) => record.modality === "DX");
  const pseudoCt = selectPseudoCt(pseudo);
  const pseudoDx = selectPseudoDx(pseudo);
  return {
    "synthetic-ct": syntheticCt.length > 0
      ? selectedWorkload(syntheticCt)
      : unavailableWorkload(rules["synthetic-ct"]),
    "synthetic-dx": syntheticDx.length > 0
      ? selectedWorkload(syntheticDx)
      : unavailableWorkload(rules["synthetic-dx"]),
    "pseudo-full": pseudo.length > 0
      ? selectedWorkload(pseudo)
      : unavailableWorkload(rules["pseudo-full"]),
    "pseudo-ct": pseudoCt ?? unavailableWorkload(rules["pseudo-ct"]),
    "pseudo-dx": pseudoDx ?? unavailableWorkload(rules["pseudo-dx"]),
  };
}

/**
 * The input identity: the SHA-256 of the sorted list of per-file content
 * digests, one per line. It identifies the input set for a rerun and
 * reveals nothing about it, which is why it and not a path list is what
 * the outputs record.
 *
 * @param {string[]} contentDigests one sha256 hex digest per input file
 * @returns {string} the sha256 hex of the sorted digests
 */
export function inputIdentity(contentDigests) {
  if (!Array.isArray(contentDigests) || contentDigests.length === 0) {
    throw new Error("input identity needs at least one content digest");
  }
  const hash = createHash("sha256");
  for (const digest of [...contentDigests].sort()) {
    if (typeof digest !== "string" || digest.length === 0) {
      throw new Error("every content digest is a non-empty string");
    }
    hash.update(digest);
    hash.update("\n");
  }
  return hash.digest("hex");
}

/**
 * Scan one output's text against the deny set. Returns one hit per matching
 * line as `{ kind, line }` and never the value itself, because an error
 * message that prints the hit is the leak the scan exists to refuse.
 *
 * Each denied value is matched in its raw form, in its JSON-escaped form
 * and in the form the renderer leaves in a Markdown table cell, because
 * serialised JSON doubles a backslash and escapes a double quote, and the
 * renderer's one transform rewrites a pipe to a backslashed pipe. A
 * multi-valued DICOM string such as `GENERAL\HOSPITAL` arrives in the
 * ignored JSON only as `GENERAL\\HOSPITAL`. escapeCell runs in the version
 * table's cells alone, so a value such as `A|B` reaches the tracked summary
 * as `A\|B` from a table cell and raw from a reason or a renderer string,
 * and the raw match still catches the raw form. A value whose escaped form
 * is itself is checked once.
 *
 * @param {string} text the output's full text
 * @param {object} denySet the nine denied kinds, each a list of values
 * @returns {{kind: string, line: number}[]}
 */
export function scanForDenied(text, denySet) {
  if (typeof text !== "string") {
    throw new Error("scanForDenied takes the output's full text");
  }
  if (denySet === null || typeof denySet !== "object") {
    throw new Error("scanForDenied takes the run's deny set");
  }
  const hits = [];
  const lines = text.split("\n");
  for (const [kind, values] of Object.entries(denySet)) {
    if (values === undefined) {
      continue;
    }
    if (!Array.isArray(values)) {
      throw new Error(`the deny set's ${kind} is not a list`);
    }
    for (const value of values) {
      // An empty header value would match every line and refuse every run,
      // so an empty entry denies nothing rather than everything.
      if (typeof value !== "string" || value.length === 0) {
        continue;
      }
      // JSON.stringify wraps the value in quotes and escapes what the JSON
      // grammar needs escaped, and slicing them off leaves the body the
      // value becomes inside a serialised record. escapeCell, the
      // renderer's one transform, rewrites a pipe to a backslashed pipe,
      // which is the third shape a value takes in an output and neither of
      // the other two.
      const escaped = JSON.stringify(value).slice(1, -1);
      const cellEscaped = value.replaceAll("|", "\\|");
      for (let index = 0; index < lines.length; index += 1) {
        if (lines[index].includes(value) ||
          (escaped !== value && lines[index].includes(escaped)) ||
          (cellEscaped !== value && lines[index].includes(cellEscaped))) {
          hits.push({ kind, line: index + 1 });
        }
      }
    }
  }
  return hits.sort((a, b) => a.line - b.line ||
    (a.kind < b.kind ? -1 : a.kind > b.kind ? 1 : 0));
}

/**
 * Scan a structured record's string values against the deny set, before the
 * record is serialised. This is the structured half of the scan: the text
 * scan sees the value only in the shape the serialisation and the renderer
 * left it in, and a value carrying a backslash or a double quote is changed
 * by the serialisation while the renderer rewrites only a pipe to a
 * backslashed pipe. Walking the record sees the value as it arrived.
 *
 * Hits are `{ kind, path }` and carry the property path, never the value.
 *
 * @param {object} record the structured record, of any depth
 * @param {object} denySet the nine denied kinds, each a list of values
 * @returns {{kind: string, path: string}[]}
 */
export function scanRecordForDenied(record, denySet) {
  if (record === null || typeof record !== "object") {
    throw new Error("scanRecordForDenied takes the structured record");
  }
  if (denySet === null || typeof denySet !== "object") {
    throw new Error("scanRecordForDenied takes the run's deny set");
  }
  const denied = [];
  for (const [kind, values] of Object.entries(denySet)) {
    if (values === undefined) {
      continue;
    }
    if (!Array.isArray(values)) {
      throw new Error(`the deny set's ${kind} is not a list`);
    }
    for (const value of values) {
      if (typeof value === "string" && value.length > 0) {
        denied.push({ kind, value });
      }
    }
  }
  const hits = [];
  const walk = (node, path) => {
    if (Array.isArray(node)) {
      node.forEach((item, index) =>
        walk(item, path === "" ? String(index) : `${path}.${index}`));
      return;
    }
    if (node === null || typeof node !== "object") {
      if (typeof node === "string") {
        for (const { kind, value } of denied) {
          if (node.includes(value)) {
            hits.push({ kind, path });
          }
        }
      }
      return;
    }
    for (const [key, value] of Object.entries(node)) {
      walk(value, path === "" ? key : `${path}.${key}`);
    }
  };
  walk(record, "");
  return hits;
}

function requireObject(value, label) {
  if (value === null || typeof value !== "object" || Array.isArray(value)) {
    throw new Error(`${label} is not an object`);
  }
  return value;
}

function refuseUnknownKeys(record, allowed, label) {
  for (const key of Object.keys(record)) {
    if (!allowed.includes(key)) {
      throw new Error(
        `${label} carries "${key}", which the published schema does not ` +
          "define. A closed schema refuses rather than copies, because a " +
          "field nobody expected is where a value nobody expected arrives.",
      );
    }
  }
}

/**
 * The published subset of a run record, under a closed schema: the machine,
 * the versions, the browser, the defaults, the workloads by label, the per
 * metric and side statistics, the pair verdict, the two fixed texts and the
 * input identity. Everything else the ignored record carries stays behind.
 *
 * Throws on an unknown key anywhere the schema is closed, on a side,
 * workload or metric the schema does not know, on a value of the wrong type,
 * on a metric cell that is missing a side, and on a missing or altered
 * `architecture` or `not_shown`, because those two texts are fixed and a
 * renderer that cannot trust them cannot promise the caveat. A closed key
 * set without typed values would still let a header value arrive through a
 * field everybody expected, which is the same leak one level down.
 *
 * @param {object} run the ignored record
 * @returns {object} the published record
 */
export function publishedRecord(run) {
  requireObject(run, "the run record");
  const required = ["machine", "versions", "browser", "defaults", "workloads",
    "metrics", "pair_verdict", "architecture", "not_shown",
    "input_identity"];
  for (const key of required) {
    if (run[key] === undefined) {
      throw new Error(
        `the run record carries no ${key}, which the published schema requires`,
      );
    }
  }
  if (run.architecture !== ARCHITECTURE) {
    throw new Error(
      "the run record's architecture text is not the fixed text the closed " +
        "schema requires",
    );
  }
  if (JSON.stringify(run.not_shown) !== JSON.stringify(NOT_SHOWN)) {
    throw new Error(
      "the run record's not_shown list is not the fixed list the closed " +
        "schema requires",
    );
  }
  const machine = requireObject(run.machine, "the machine block");
  refuseUnknownKeys(machine, HOST_CLASS_KEYS, "the machine block");
  for (const field of ["platform", "release", "arch", "cpu_model"]) {
    if (typeof machine[field] !== "string" || machine[field].length === 0) {
      throw new Error(`the machine block's ${field} is not a string`);
    }
  }
  for (const field of ["cpu_count", "memory_bytes"]) {
    if (!Number.isSafeInteger(machine[field]) || machine[field] < 1) {
      throw new Error(`the machine block's ${field} is not a whole count`);
    }
  }
  const versions = requireObject(run.versions, "the versions block");
  refuseUnknownKeys(versions, VERSION_KEYS, "the versions block");
  for (const key of VERSION_KEYS) {
    if (typeof versions[key] !== "string" || versions[key].length === 0) {
      throw new Error(`the versions block carries no ${key} version string`);
    }
  }
  const browser = requireObject(run.browser, "the browser block");
  refuseUnknownKeys(browser, BROWSER_KEYS, "the browser block");
  if (typeof browser.renderer !== "string" || browser.renderer.length === 0) {
    throw new Error("the browser block carries no renderer string");
  }
  if (typeof browser.headed !== "boolean" ||
    typeof browser.software !== "boolean") {
    throw new Error("the browser block's headed and software flags are booleans");
  }
  if (!browser.software && /swiftshader/i.test(browser.renderer)) {
    throw new Error(
      "the browser block's renderer names SwiftShader and its software flag " +
        "is false. A SwiftShader renderer is refused unless --software was " +
        "passed, and a publication may not record the renderer while hiding " +
        "the flag that permitted it",
    );
  }
  const defaults = requireObject(run.defaults, "the defaults block");
  const publishedDefaults = {};
  for (const side of SIDES) {
    const block = requireObject(defaults[side], `the ${side} defaults`);
    publishedDefaults[side] = {};
    for (const key of Object.keys(block)) {
      if (!DEFAULT_ORDER[side].includes(key)) {
        throw new Error(
          `the ${side} defaults carry "${key}", which no side records`,
        );
      }
    }
    for (const key of DEFAULT_ORDER[side]) {
      if (typeof block[key] !== "number" || !Number.isFinite(block[key])) {
        throw new Error(`the ${side} defaults carry no finite ${key}`);
      }
      publishedDefaults[side][key] = block[key];
    }
  }
  const workloads = requireObject(run.workloads, "the workloads block");
  const publishedWorkloads = {};
  for (const [id, record] of Object.entries(workloads)) {
    if (!WORKLOAD_IDS.includes(id)) {
      throw new Error(
        `${id} is not a workload the published schema defines`,
      );
    }
    const cell = requireObject(record, `the ${id} workload record`);
    refuseUnknownKeys(cell, WORKLOAD_CELL_KEYS, `the ${id} workload record`);
    if (!WORKLOAD_STATES.includes(cell.state)) {
      throw new Error(`the ${id} workload record's state is not a state`);
    }
    if (!Number.isSafeInteger(cell.count) || cell.count < 0) {
      throw new Error(
        `the ${id} workload record's count is not a whole count of inputs`,
      );
    }
    if (cell.state === "unavailable" &&
      (typeof cell.reason !== "string" || cell.reason.length === 0)) {
      throw new Error(
        `the ${id} workload record is unavailable and names no reason, and ` +
          "an unavailable workload without its reason is an omission dressed " +
          "as an answer",
      );
    }
    if (cell.state === "selected" && cell.reason !== undefined) {
      throw new Error(
        `the ${id} workload record is selected and carries a reason, and a ` +
          "reason is what an unselected workload explains itself with",
      );
    }
    publishedWorkloads[id] = { ...cell };
  }
  const metrics = requireObject(run.metrics, "the metrics block");
  const publishedMetrics = {};
  for (const [metricId, workloadsForMetric] of Object.entries(metrics)) {
    if (!METRIC_IDS.includes(metricId)) {
      throw new Error(
        `${metricId} is not a metric the published schema defines`,
      );
    }
    const byWorkload = requireObject(workloadsForMetric,
      `the ${metricId} metrics`);
    publishedMetrics[metricId] = {};
    for (const label of Object.keys(byWorkload)) {
      const parts = label.split("/");
      if (parts.length > 2 || !WORKLOAD_LABELS.includes(parts[0]) || (parts.length === 2 && !["cold", "warm"].includes(parts[1]))) throw new Error("the comparison key is not a workload label or condition");
      if (parts.length === 2) {
        if (Object.hasOwn(byWorkload, parts[0])) throw new Error("duplicate legacy and qualified comparison condition");
        const paired = `${parts[0]}/${parts[1] === "cold" ? "warm" : "cold"}`;
        if (!Object.hasOwn(byWorkload, paired)) throw new Error("comparison is missing its paired condition");
      }
    }
    for (const [workloadId, sides] of Object.entries(byWorkload)) {
      if (!WORKLOAD_LABELS.includes(workloadId.split("/")[0])) {
        throw new Error(
          `${workloadId} is not a workload label the ${metricId} metrics may ` +
            "use",
        );
      }
      const bySide = requireObject(sides, `the ${metricId} ${workloadId} cell`);
      publishedMetrics[metricId][workloadId] = {};
      for (const [side, cell] of Object.entries(bySide)) {
        if (!SIDES.includes(side)) {
          throw new Error(
            `${side} is not a side the published schema defines`,
          );
        }
        const record = requireObject(cell,
          `the ${metricId} ${workloadId} ${side} cell`);
        refuseUnknownKeys(record,
          METRIC_CELL_KEYS,
          `the ${metricId} ${workloadId} ${side} cell`);
        if (!METRIC_CELL_STATES.includes(record.state)) {
          throw new Error(
            `the ${metricId} ${workloadId} ${side} cell's state is not one ` +
              `of ${METRIC_CELL_STATES.join(", ")}`,
          );
        }
        validateMetricCell(metricId, workloadId, side, record);
        publishedMetrics[metricId][workloadId][side] = { ...record };
      }
      // After the entries, so an unknown side is named before a missing
      // one: a cell carrying `chrome` is refused for `chrome`, and a cell
      // carrying one side only is refused for the side it lost.
      for (const side of SIDES) {
        if (bySide[side] === undefined) {
          throw new Error(
            `the ${metricId} ${workloadId} cell carries no ${side} side, and ` +
              "a comparison cell with one side is not a comparison",
          );
        }
      }
    }
  }
  const pair = requireObject(run.pair_verdict, "the pair verdict");
  refuseUnknownKeys(pair, PAIR_VERDICT_KEYS, "the pair verdict");
  for (const key of PAIR_VERDICT_KEYS) {
    if (pair[key] === undefined) {
      throw new Error(`the pair verdict carries no ${key}`);
    }
  }
  validatePairVerdict(pair);
  if (typeof run.input_identity !== "string" ||
    !/^[0-9a-f]{64}$/.test(run.input_identity)) {
    throw new Error(
      "the input identity is not the sha256 hex of the sorted digest list",
    );
  }
  return {
    machine: { ...machine },
    versions: { ...versions },
    browser: { ...browser },
    defaults: publishedDefaults,
    workloads: publishedWorkloads,
    metrics: publishedMetrics,
    pair_verdict: { ...pair },
    architecture: run.architecture,
    not_shown: run.not_shown.map((one) => one),
    input_identity: run.input_identity,
  };
}

/**
 * The pair verdict's values, typed: the two fractions are finite numbers and
 * each lies in [0, 1] because each is a share of the frame's pixels, the
 * over-maximum count is a whole count of pixels, the maximum absolute
 * difference is one byte, and the two verdict flags are booleans. The bias
 * over the informative region is a finite number exactly when the
 * informative fraction is above 0, and null exactly when the fraction is 0,
 * per the ruling after review pass 2: a fraction of exactly 0 measured no
 * pixel, so the bias there is null and a number would be the placeholder
 * zero the ruling removed. Null is further admitted only beside a
 * `low_information` of true. A string `passes` would otherwise publish, and
 * the harness reads `passes` as the whole contract.
 *
 * @param {object} pair the verdict as the run recorded it
 */
function validatePairVerdict(pair) {
  // A bias that was not measured serialises as null, and the schema admits
  // null only when the informative fraction is exactly 0 and
  // low_information is then true, because a null beside an informative
  // pixel is a schema failure and not a measurement. The equivalence holds
  // both ways: a fraction of exactly 0 measured no pixel, so a numeric bias
  // beside it is the placeholder the ruling removed and not a measurement.
  // Any other bias is a finite number.
  if (pair.signed_mean_bias_informative === null) {
    if (pair.informative_fraction !== 0 || pair.low_information !== true) {
      throw new Error(
        "the pair verdict's signed_mean_bias_informative is null, which the " +
          "schema admits only when the informative fraction is exactly 0 and " +
          "low_information is then true, because a bias no pixel measured is " +
          "stated as absent and not as zero",
      );
    }
  } else if (!Number.isFinite(pair.signed_mean_bias_informative)) {
    throw new Error(
      "the pair verdict's signed_mean_bias_informative is not a finite number");
  } else if (pair.informative_fraction === 0) {
    throw new Error(
      "the pair verdict's signed_mean_bias_informative is a number beside an " +
        "informative fraction of exactly 0, where no pixel was measured and " +
        "the bias is null and not a placeholder zero",
    );
  }
  for (const key of ["within_one_lsb_fraction", "informative_fraction"]) {
    if (!Number.isFinite(pair[key])) {
      throw new Error(`the pair verdict's ${key} is not a finite number`);
    }
  }
  // Each fraction is a share of the frame's pixels, so neither below 0 nor
  // above 1. Without these a bias beside a negative informative fraction
  // would publish as measured, and an agreement share above 1 would publish
  // as more pixels than the frame holds. Each refusal has its own words so
  // the census sees each one separately.
  if (pair.within_one_lsb_fraction < 0 || pair.within_one_lsb_fraction > 1) {
    throw new Error(
      "the pair verdict's within_one_lsb_fraction is outside [0, 1], and a " +
        "share of the frame's pixels agreeing within one LSB cannot be",
    );
  }
  if (pair.informative_fraction < 0 || pair.informative_fraction > 1) {
    throw new Error(
      "the pair verdict's informative_fraction is outside [0, 1], and a " +
        "share of the frame's pixels cannot be",
    );
  }
  if (!Number.isSafeInteger(pair.count_over_max) || pair.count_over_max < 0) {
    throw new Error(
      "the pair verdict's count_over_max is not a whole count of pixels",
    );
  }
  if (!Number.isSafeInteger(pair.max_abs_diff) || pair.max_abs_diff < 0 ||
    pair.max_abs_diff > 255) {
    throw new Error(
      "the pair verdict's max_abs_diff is not a byte of absolute difference",
    );
  }
  for (const key of ["low_information", "passes"]) {
    if (typeof pair[key] !== "boolean") {
      throw new Error(`the pair verdict's ${key} is not a boolean`);
    }
  }
}

/**
 * One metric cell's values, typed against its state. A measured cell carries
 * the five statistics and no reason, because a measured cell has nothing to
 * explain. An unavailable cell carries its reason and no statistics. An
 * invalid cell may carry the reason the trials fell short and carries no
 * statistic, because a statistic over a partial sample is the number the
 * plan refused in advance.
 *
 * @param {string} metricId the metric the cell belongs to
 * @param {string} workloadId the workload the cell belongs to
 * @param {string} side the side the cell belongs to
 * @param {object} record the cell as the run recorded it
 */
function validateMetricCell(metricId, workloadId, side, record, upper = false) {
  const where = `the ${metricId} ${workloadId} ${side} cell`;
  if (!upper && metricId === "peak_memory" && workloadId.includes("/") && record.state === "measured") {
    const bound = requireObject(record.upper_bound, "the memory upper bound");
    refuseUnknownKeys(bound, METRIC_CELL_KEYS.filter((key) => key !== "upper_bound"), "the memory upper bound");
    if (record.n !== 3 || bound.n !== 3 || bound.state !== "measured") throw new Error("the memory upper bound needs the same three measured trials");
    validateMetricCell(metricId, workloadId, side, bound, true);
  } else if (record.upper_bound !== undefined) {
    throw new Error("an upper bound belongs only to a measured qualified memory cell");
  }
  if (!Number.isSafeInteger(record.n) || record.n < 0) {
    throw new Error(`${where}'s n is not a whole count of trials`);
  }
  const statistics = ["median", "p25", "p75", "min", "max"];
  if (record.state === "measured") {
    for (const key of statistics) {
      if (typeof record[key] !== "number" || !Number.isFinite(record[key])) {
        throw new Error(`${where} is measured and carries no finite ${key}`);
      }
    }
    if (record.reason !== undefined) {
      throw new Error(
        `${where} is measured and carries a reason, and a reason is what an ` +
          "unmeasured cell explains itself with",
      );
    }
    return;
  }
  for (const key of statistics) {
    if (record[key] !== undefined) {
      throw new Error(
        `${where} is ${record.state} and carries ${key}, and a statistic ` +
          "over a partial or absent sample is the number the plan refused " +
          "in advance",
      );
    }
  }
  if (record.state === "unavailable" &&
    (typeof record.reason !== "string" || record.reason.length === 0)) {
    throw new Error(`${where} is unavailable and names no reason`);
  }
  if (record.reason !== undefined && typeof record.reason !== "string") {
    throw new Error(`${where}'s reason is not a string`);
  }
}

// The per-unit cell formatters. Durations render at the clock precision the
// harness rounds to, memory renders in MiB to one decimal from the exact
// bytes the JSON records, CPU seconds render at the centisecond quantum ps
// reports, and rates render to two decimals.
const CELL_FORMATTERS = {
  ms: (value) => atClockPrecision(value).toFixed(4),
  frames_per_second: (value) => value.toFixed(2),
  bytes: (value) => (value / 1048576).toFixed(1),
  seconds: (value) => value.toFixed(2),
};

function formatCell(unit, cell, column) {
  if (cell.state === "measured") {
    return CELL_FORMATTERS[unit](cell[column]);
  }
  return cell.state;
}

function formatSpread(unit, cell) {
  if (cell.state === "measured") {
    return `${CELL_FORMATTERS[unit](cell.p25)} to ${CELL_FORMATTERS[unit](cell.p75)}`;
  }
  return cell.state;
}

function measurementNotes(metricId, workloadId, cell) {
  const notes = [];
  const workload = WORKLOAD_DISPLAY[workloadId] ?? workloadId;
  if (cell.state === "unavailable") {
    notes.push(`- ${workload}: ${cell.reason ?? "no reason was recorded"}`);
  }
  if (cell.state === "invalid") {
    notes.push(
      `- ${workload}: fewer valid trials than the declared N for ` +
        `${METRIC_DISPLAY[metricId]}`,
    );
  }
  return notes;
}

/**
 * The ratio to three significant figures, in plain decimal notation, taken
 * from the numerator and the denominator rather than from a pre-divided
 * value, because the zero-denominator case is decided on the denominator.
 * A denominator of zero renders as words saying the denominator is zero and
 * never as "Infinity", per the ruling after review pass 2. Every other
 * ratio renders to three significant figures, including across a decade:
 * the decimals come from the exponent of the ROUNDED value, so 9.996
 * renders "10.0" and not "10.00" and 0.99951 renders "1.00" and not
 * "1.000". `toPrecision(3)` switches to exponent notation from 1000 up,
 * which `Number` unwinds, and `toFixed` would re-apply it from 1e21 up, a
 * magnitude no measured median reaches.
 *
 * @param {number} numerator the side's own median
 * @param {number} denominator the other side's median
 * @returns {string}
 */
function ratioText(numerator, denominator) {
  if (denominator === 0) {
    return "denominator is zero";
  }
  const rounded = Number((numerator / denominator).toPrecision(3));
  if (rounded === 0) {
    return "0";
  }
  const exponent = Math.floor(Math.log10(Math.abs(rounded)));
  return rounded.toFixed(Math.max(0, 2 - exponent));
}

function measurementSection(metrics) {
  const lines = ["## The measurements", ""];
  for (const [metricId, workloads] of Object.entries(metrics)) {
    const unit = METRIC_UNITS[metricId];
    lines.push(`### ${METRIC_DISPLAY[metricId]}, ${UNIT_WORDS[unit]}`, "");
    if (metricId === "peak_memory") lines.push(
      `Memory sampling pauses at least ${FOOTPRINT_PERIOD} ms between completed collection rounds. Actual sample timestamps remain in raw evidence.`, "",
    );
    lines.push(
      "| workload | Ocelli median | Ocelli p25 to p75 | " +
        "cornerstone3D median | cornerstone3D p25 to p75 | ratio | trials |",
    );
    lines.push("| --- | --- | --- | --- | --- | --- | --- |");
    const notes = [];
    for (const [workloadId, sides] of Object.entries(workloads)) {
      const a = sides[OCELLI];
      const b = sides[CORNERSTONE];
      let ratio = "-";
      if (a.state === "measured" && b.state === "measured") {
        ratio = separated(a, b)
          ? ratioText(a.median, b.median)
          : `no separation at ${a.n} trials`;
      }
      const workload = WORKLOAD_DISPLAY[workloadId] ?? workloadId;
      const bound = metricId === "peak_memory" ? ", sampled concurrent footprint lower bound" : "";
      lines.push(
        `| ${workload}${bound} | ${formatCell(unit, a, "median")} | ` +
          `${formatSpread(unit, a)} | ${formatCell(unit, b, "median")} | ` +
          `${formatSpread(unit, b)} | ${ratio} | ${a.n} |`,
      );
      if (metricId === "peak_memory" && a.upper_bound && b.upper_bound) {
        const upperA = a.upper_bound, upperB = b.upper_bound;
        const upperRatio = separated(upperA, upperB) ? ratioText(upperA.median, upperB.median) : `no separation at ${upperA.n} trials`;
        lines.push(`| ${workload}, sum of per-process peaks upper bound | ${formatCell(unit, upperA, "median")} | ${formatSpread(unit, upperA)} | ${formatCell(unit, upperB, "median")} | ${formatSpread(unit, upperB)} | ${upperRatio} | ${upperA.n} |`);
      }
      notes.push(...measurementNotes(metricId, workloadId, a));
      notes.push(...measurementNotes(metricId, workloadId, b));
    }
    if (notes.length > 0) {
      lines.push("", ...notes);
    }
    lines.push("");
  }
  return lines;
}

function machineSection(machine) {
  return [
    "## The machine",
    "",
    `- CPU: ${machine.cpu_model}, ${machine.cpu_count} cores`,
    `- memory: ${machine.memory_bytes} bytes ` +
      `(${(machine.memory_bytes / 1048576).toFixed(1)} MiB)`,
    `- operating system: ${machine.platform} ${machine.release} ` +
      `(${machine.arch})`,
    "",
  ];
}

/**
 * One Markdown table cell, with the pipe escaped. A version string carrying
 * a pipe would otherwise end the cell and break the table the summary
 * renders byte for byte.
 *
 * @param {string} value the cell's text
 * @returns {string}
 */
function escapeCell(value) {
  return String(value).replaceAll("|", "\\|");
}

function versionsSection(versions) {
  const lines = ["## Versions", "", "| tool or package | version |",
    "| --- | --- |"];
  for (const [key, value] of Object.entries(versions)) {
    lines.push(`| ${escapeCell(key)} | ${escapeCell(value)} |`);
  }
  lines.push("");
  return lines;
}

function defaultsSection(defaults) {
  const lines = ["## The defaults each side ran at", "",
    "| side | default | value |", "| --- | --- | --- |"];
  for (const side of SIDES) {
    for (const key of DEFAULT_ORDER[side]) {
      lines.push(`| ${SIDE_DISPLAY[side]} | ${DEFAULT_LABELS[key]} | ${defaults[side][key]} |`);
    }
  }
  lines.push("");
  return lines;
}

function workloadsSection(workloads) {
  const lines = ["## The workloads", "", "| workload | state | inputs |",
    "| --- | --- | --- |"];
  const notes = [];
  for (const [id, record] of Object.entries(workloads)) {
    const label = WORKLOAD_DISPLAY[id] ?? id;
    lines.push(`| ${label} | ${record.state} | ${record.count ?? 0} |`);
    if (record.state === "unavailable") {
      notes.push(`- ${label}: ${record.reason ?? "no reason was recorded"}`);
    }
  }
  if (notes.length > 0) {
    lines.push("", ...notes);
  }
  lines.push("");
  return lines;
}

/**
 * The pixel section. The opening sentence is the run's own: a run whose
 * check passed took every timing figure above it in that run, and a refused
 * run took none, so the refused text says no timing figure was recorded
 * instead of claiming one was. Rendering the passing sentence over a
 * refused run is how the one state the plan says the report must describe
 * honestly, `refused_pixels`, gets described falsely. A bias of null is the
 * run's own statement that no pixel was informative, and the bullet says
 * the bias was not measured rather than printing a number no pixel
 * measured.
 *
 * @param {object} verdict the pair verdict as the run recorded it
 * @param {boolean} refused whether the run's state is `refused_pixels`
 */
function pixelSection(verdict, refused) {
  const precondition = refused
    ? "No timing figure was recorded, because the pixel check below " +
      "refused the run. One frame of each side's pseudo CT workload, at " +
      "centre 40, width 400, LINEAR, compared through HLD 25.1's " +
      "monochrome rule and bias bound."
    : "Every timing figure above was taken in a run whose pixel check " +
      "passed. One frame of each side's pseudo CT workload, at centre 40, " +
      "width 400, LINEAR, compared through HLD 25.1's monochrome rule and " +
      "bias bound.";
  return [
    "## The pixel agreement check",
    "",
    precondition,
    "",
    `- within one LSB: ${verdict.within_one_lsb_fraction.toFixed(4)} of pixels`,
    `- pixels over the maximum: ${verdict.count_over_max}`,
    `- maximum absolute difference: ${verdict.max_abs_diff}`,
    ...(verdict.signed_mean_bias_informative === null
      ? ["- signed mean bias over the informative region: not measured, " +
         "because no pixel was informative"]
      : [`- signed mean bias over the informative region: ` +
         `${verdict.signed_mean_bias_informative.toFixed(4)}`]),
    `- informative fraction: ${verdict.informative_fraction.toFixed(4)}`,
    `- low information: ${verdict.low_information ? "yes" : "no"}`,
    `- the verdict passed: ${verdict.passes ? "yes" : "no"}`,
    "",
  ];
}

function notShownSection(notShown) {
  return ["## What the numbers do not show", "",
    ...notShown.map((item) => `- ${item}`), ""];
}

function inputIdentityLine(identity) {
  return [
    `The input identity is ${identity}, the SHA-256 of the sorted list of ` +
      "per-file content digests.",
    "",
  ];
}

/**
 * The paragraphs and sections both renderers carry. `refused` decides the
 * pixel section's opening sentence, and it is the run's state: the
 * published record carries no state, because a publication exists only for
 * a run whose check passed.
 *
 * When the run was made with --software, the first paragraph names
 * SwiftShader, per fact 15: "in which case every output names SwiftShader
 * in its first paragraph". The architecture text stays the fixed text the
 * closed schema checks, and the sentence rides after it.
 *
 * @param {object} record the record being rendered
 * @param {boolean} refused whether the run's state is `refused_pixels`
 * @returns {string[]}
 */
function sharedBody(record, refused) {
  const firstParagraph = record.browser?.software
    ? `${record.architecture} The run was made with --software, which ` +
      "permits a SwiftShader renderer, and the renderer the pages " +
      `reported is ${record.browser.renderer}.`
    : record.architecture;
  return [
    firstParagraph,
    "",
    ACKNOWLEDGEMENT,
    "",
    ...(record.browser ? [tierSentence(record.browser), ""] : []),
    ...(record.machine ? machineSection(record.machine) : []),
    ...(record.versions ? versionsSection(record.versions) : []),
    ...(record.defaults ? defaultsSection(record.defaults) : []),
    ...(record.workloads ? workloadsSection(record.workloads) : []),
    ...(record.metrics ? measurementSection(record.metrics) : []),
    ...(record.pair_verdict
      ? pixelSection(record.pair_verdict, refused === true)
      : []),
    ...(record.not_shown ? notShownSection(record.not_shown) : []),
    ...(record.input_identity
      ? inputIdentityLine(record.input_identity)
      : []),
  ];
}

/**
 * The tracked summary, rendered byte for byte from the published record
 * beside it. The first line names the command that wrote it, because a
 * generated file that looks hand-editable will be hand-edited.
 *
 * @param {object} published the published record
 * @returns {string} the markdown
 */
export function renderSummary(published) {
  return [
    SUMMARY_HEADER,
    "",
    "# Comparative cost of the local folder viewer and cornerstone3D 5.8.2",
    "",
    ...sharedBody(published, false),
  ].join("\n");
}

/**
 * The plain report, rendered from the ignored record. The architecture text
 * is its first paragraph, as it is the summary's, and the run's own extras,
 * the started stamp, the state and the refusals, are rendered here and
 * nowhere else.
 *
 * @param {object} run the ignored record
 * @returns {string} the plain report
 */
export function renderReport(run) {
  requireObject(run, "the run record");
  if (typeof run.architecture !== "string" || run.architecture.length === 0) {
    throw new Error("the plain report requires the architecture text");
  }
  const header = [
    "# Comparative cost, plain report",
    "",
    `Started ${run.started ?? "at an unstated time"}, state ` +
      `${run.state ?? "unrecorded"}. The ignored record is ` +
      "tools/bench/out/compare/comparison.json and this report renders it.",
    "",
  ];
  const refusals = Array.isArray(run.refusals) && run.refusals.length > 0
    ? ["## Refusals", "", ...run.refusals.map((one) => `- ${one}`), ""]
    : [];
  return [...header, ...sharedBody(run, run.state === "refused_pixels"),
    ...refusals].join("\n");
}

/**
 * The count of other Playwright browser processes: those whose command line
 * names ms-playwright and whose pid is outside our own tree. The mode
 * refuses the run on a non-zero count and never signals a process.
 *
 * @param {string} psText `ps -A -o pid=,command=` output
 * @param {Set<number>} ownPids our own process and its descendants
 * @returns {number}
 */
export function otherBrowserJobs(psText, ownPids) {
  if (typeof psText !== "string") {
    throw new Error("otherBrowserJobs takes ps -A -o pid=,command= output");
  }
  if (!(ownPids instanceof Set)) {
    throw new Error("otherBrowserJobs takes our own pids as a Set");
  }
  const jobs = new Set();
  for (const line of psText.split("\n")) {
    const match = /^\s*(\d+)\s+(.+)$/.exec(line);
    if (!match) {
      continue;
    }
    const pid = Number(match[1]);
    if (!ownPids.has(pid) && match[2].includes("ms-playwright")) {
      jobs.add(pid);
    }
  }
  return jobs.size;
}
