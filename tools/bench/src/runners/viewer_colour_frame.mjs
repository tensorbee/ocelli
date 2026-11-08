// Subject viewer.colour_frame, F-239 AC31's instrument with F-240 AC15's
// additive ICC extension. Four synthetic 8192 by 8192 native Part 10 frames:
// F-239's 8-bit RGB and 16-bit PALETTE COLOR pair, byte for byte as F-239
// wrote them, plus an 8-bit RGB frame carrying the AC2 curv gamma 0x0233
// profile and a 16-bit RGB frame carrying a derived two-entry curv table
// profile. Each goes through the release viewer with no browser, and each
// workload runs in its own viewer process so no two held frames share one
// held budget. The subject times HTTP /preview and the server process only,
// which is what AC31 and AC15 name.
//
// Per workload, in its own viewer process:
//
//   1. untimed POST /file to hold the file and retain its inspect metadata,
//      then the first id-only POST /preview, timed through its last byte.
//      X-Ocelli-Source must be decoded, so this figure contains the first
//      decode, resolve, conversion and reply bytes.
//   2. nine id-only colour window steps with no body. X-Ocelli-Source must
//      be held for every one. Numeric Window Center and Width travel through
//      the existing headers, and the nine timings are recorded per workload.
//   3. peak `ps -o rss=` sampled every 20 ms from when startViewer announces
//      the process's URL to after the last measured preview, as
//      viewer.resident_memory samples it. The sampler never claims the
//      process's whole lifetime: the build report and both uploads are
//      figures-free setup, and the metadata reads after the last step are
//      outside the measured window too.
//
// The ICC workloads add, without touching any F-239 figure, window, position
// or child budget:
//
//   - the /file report must carry icc_applied true and icc_kind "", the
//     profile applied, not merely present,
//   - the 16-bit ICC workload alone starts its child with
//     OCELLI_CACHE_MIB=512, because its 384 MiB encoded and 384 MiB decoded
//     samples cannot be held under the 256 MiB default; requested and actual
//     are recorded and asserted to agree, and no other workload's child
//     budget is touched,
//   - a held census after the first decode and after the window steps, which
//     must hold at least the decoded samples plus the TRC tables
//     (3 * 2^bits * 4 bytes) and must not move across the warm draws. This
//     observes retained charge and frame stability, not transient work. A
//     same-sized parse, table rebuild or replacement can leave it unchanged.
//     Work-once evidence also needs source/lifetime checks and actual timings.
//
// There is no threshold. The figures are the baseline a later colour story
// is compared with, and the recorded value is the untagged 8-bit RGB median
// of the nine window steps, the same shape viewer.window_step records. Every
// reply is consumed in full and representative bytes are checked against the
// synthetic pattern the fixture modules wrote, so a reply that is merely
// the right length cannot pass.

import { execFile } from "node:child_process";
import { createHash } from "node:crypto";
import { Buffer } from "node:buffer";
import { createReadStream } from "node:fs";
import { rm } from "node:fs/promises";
import http from "node:http";
import { performance } from "node:perf_hooks";
import { join } from "node:path";

import { benchPath } from "../paths.mjs";
import {
  ensureViewerBinary,
  median,
  reportedBuild,
  startViewer,
  stopViewer,
} from "../viewer_work.mjs";
import {
  EDGE,
  sampleChecks,
  writeColourFrameFixtures,
} from "../viewer_colour_frame_fixture.mjs";
import {
  iccSampleBytes,
  iccSampleChecks,
  writeIccFrameFixtures,
} from "../viewer_icc_frame_fixture.mjs";

export const id = "viewer.colour_frame";

/** The AC31 sampling period for `ps -o rss=`. */
export const RSS_PERIOD_MS = 20;

/** One MiB, for the one explicit child budget AC15's amendment authorises. */
const MIB = 1024 * 1024;

/**
 * The initial colour window, the identity. The first request sends it and
 * the fixture's expected bytes are the raw colour stage output there.
 */
export const IDENTITY_WINDOW = { center: 128, width: 256 };

/**
 * The nine held colour window steps. Centres move away from the identity so
 * every step applies a real window, at one fixed width, the shape
 * viewer.window_step takes. Distinct from the identity in every entry.
 */
export const WINDOW_STEPS = Array.from({ length: 9 }, (_, index) => ({
  center: 100 + index,
  width: 200,
}));

/** A held request that never finishes is refused, not waited on forever. */
const REQUEST_TIMEOUT_MS = 10 * 60 * 1000;

/** The held report is metadata, so its own wait is much shorter. */
const HELD_REPORT_TIMEOUT_MS = 10 * 1000;

function delay(ms) {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

/**
 * The build the viewer reports must be release. AC31 measures the release
 * build, and a debug figure recorded here would be a slower number wearing
 * the subject's name.
 */
export function assertRelease(reported) {
  if (reported !== "release") {
    throw new Error(
      `the viewer reported the ${JSON.stringify(reported)} build, and ` +
        `${id} measures the release build only`,
    );
  }
}

/**
 * The child cache budget a workload asks for, in MiB. Null means the process
 * environment decides, which is the F-239 behavior every workload but the
 * 16-bit ICC one keeps.
 */
export function requestedCacheMib(spec) {
  return spec.cacheMiB ?? null;
}

/** The environment one workload's child starts with. Only icc16 sets anything. */
export function startEnv(spec) {
  const mib = requestedCacheMib(spec);
  return mib === null ? {} : { OCELLI_CACHE_MIB: String(mib) };
}

/** The sampled reply bytes of one workload and window, ICC or F-239 pattern. */
function checksFor(spec, window, edge) {
  return spec.icc
    ? iccSampleChecks(spec.id, window, edge)
    : sampleChecks(spec.id, window, edge);
}

/**
 * The profiled /file report must say the profile was applied, not merely
 * present. `icc_applied` true with an `icc_kind` would be a profile that was
 * read and then refused, which is not a measurement of the applied path.
 */
export function assertIccApplied(workloadId, report) {
  if (!report || report.icc_applied !== true) {
    throw new Error(
      `the ${workloadId} file report does not carry icc_applied true: ` +
        `${JSON.stringify(report?.icc_applied ?? null)}`,
    );
  }
  if (report.icc_kind !== "") {
    throw new Error(
      `the ${workloadId} file report carries icc_kind ` +
        `${JSON.stringify(report.icc_kind)}, expected the empty string of ` +
        `an applied profile`,
    );
  }
}

/**
 * The one explicit child budget must be the budget the child actually runs
 * with. A workload that asked for 512 MiB and was refused admission would
 * otherwise record figures about a frame it never held.
 */
export function assertHeldBudget(spec, budgetBytes) {
  const mib = requestedCacheMib(spec);
  if (mib === null) return;
  const requested = mib * MIB;
  if (budgetBytes !== requested) {
    throw new Error(
      `the ${spec.id} viewer reported a ${budgetBytes} byte held budget, but ` +
        `this workload alone requested OCELLI_CACHE_MIB=${mib} ` +
        `(${requested} bytes)`,
    );
  }
}

/**
 * The held census must hold at least the decoded samples plus the TRC tables
 * the profile must charge. Below that, the instrument has no evidence of
 * the required retained charge, even if a profile was parsed transiently.
 */
export function assertHeldCensus(spec, census, minimumBytes, when) {
  if (!Number.isSafeInteger(census?.used)) {
    throw new Error(
      `the ${spec.id} held census ${when} carries no used byte count: ` +
        `${JSON.stringify(census ?? null)}`,
    );
  }
  if (census.used < minimumBytes) {
    throw new Error(
      `the ${spec.id} viewer held ${census.used} bytes ${when}, below the ` +
        `${minimumBytes} bytes of decoded samples plus TRC tables`,
    );
  }
  if (!Number.isSafeInteger(census.frames)) {
    throw new Error(
      `the ${spec.id} held census ${when} carries no frame count: ` +
        `${JSON.stringify(census?.frames ?? null)}`,
    );
  }
}

/**
 * The held frame and its charged bytes must not move across the warm draws.
 * This checks retained charge and frame-count stability only. A transient
 * parse, a same-sized TRC rebuild or replacement can leave both unchanged.
 * Source/lifetime checks and actual timings supply the separate work-once
 * evidence. The census alone cannot establish it.
 */
export function assertCensusStable(spec, first, last) {
  if (first.frames !== last.frames) {
    throw new Error(
      `the ${spec.id} held frame count moved across the warm draws, ` +
        `${JSON.stringify(first.frames)} after the first decode and ` +
        `${JSON.stringify(last.frames)} after the window steps`,
    );
  }
  if (first.used !== last.used) {
    throw new Error(
      `the ${spec.id} held byte count moved across the warm draws, ` +
        `${JSON.stringify(first.used)} after the first decode and ` +
        `${JSON.stringify(last.used)} after the window steps`,
    );
  }
}

/**
 * One `ps -o rss=` reading in kilobytes, or null when the read failed. The
 * failure is counted by the sampler, never turned into a reading.
 */
function rssKb(pid) {
  return new Promise((resolve) => {
    let child;
    try {
      child = execFile(
        "ps",
        ["-o", "rss=", "-p", String(pid)],
        { timeout: 5000 },
        (error, stdout) => {
          if (error) {
            resolve(null);
            return;
          }
          const value = Number(String(stdout).trim());
          resolve(Number.isSafeInteger(value) && value > 0 ? value : null);
        },
      );
    } catch {
      // A spawn that fails outright is a failed read like any other, which
      // the sampler counts. A run with no successful read refuses to claim
      // a peak rather than reporting zero.
      if (child === undefined) resolve(null);
    }
  });
}

/**
 * Peak RSS over one viewer process, one serialized reading every
 * `periodMs`. The reads never overlap: the next is scheduled only after the
 * previous one returned, so a slow `ps` cannot stack tasks, and every actual
 * reading is kept with the count of the failed ones. A run in which no
 * reading succeeded refuses to claim a peak rather than claiming zero.
 * `readKb` exists for the sampler's own checks, which run where `ps` may
 * not, and defaults to the real reading.
 */
export function startRssSampler(pid, periodMs = RSS_PERIOD_MS, readKb = rssKb) {
  const samplesKb = [];
  let failures = 0;
  let stopped = false;
  const finished = (async () => {
    while (!stopped) {
      const started = Date.now();
      let kb = null;
      try { kb = await readKb(pid); } catch { /* A failed read is not a sample. */ }
      if (!Number.isSafeInteger(kb) || kb <= 0) failures += 1;
      else samplesKb.push(kb);
      await delay(Math.max(0, periodMs - (Date.now() - started)));
    }
  })();
  let outcome = null;
  return {
    async stop() {
      stopped = true;
      await finished;
      if (outcome === null) {
        if (samplesKb.length === 0) {
          throw new Error(
            `no rss sample was taken from pid ${pid} ` +
              `(${failures} failed reads), so there is no peak to report`,
          );
        }
        outcome = {
          peak_bytes: samplesKb.reduce((best, kb) => Math.max(best, kb), 0) * 1024,
          samples: samplesKb.length,
          readings_kb: [...samplesKb],
          failures,
          period_ms: periodMs,
        };
      }
      return outcome;
    },
  };
}

/**
 * The reply bytes arriving in chunks, checked against expected absolute
 * offsets without holding the reply. One byte of carry bridges a check that
 * straddles a chunk boundary, so the check set stays bounded whatever the
 * socket's chunking is.
 */
export class SampleWindow {
  constructor(checks) {
    this.pending = [...checks].sort((left, right) => left.offset - right.offset);
    this.cursor = 0;
    this.carry = Buffer.alloc(0);
    this.carryBase = 0;
  }

  /** `chunk` lands at absolute body offset `absoluteStart`. */
  push(chunk, absoluteStart) {
    const buffer = Buffer.concat([this.carry, chunk]);
    const base = absoluteStart - this.carry.length;
    while (this.cursor < this.pending.length) {
      const check = this.pending[this.cursor];
      if (check.offset < base) {
        throw new Error(
          `the reply sample at offset ${check.offset} was passed before it arrived`,
        );
      }
      if (check.offset >= base + buffer.length) break;
      const actual = buffer[check.offset - base];
      if (actual !== check.expected) {
        throw new Error(
          `the reply byte at offset ${check.offset} was ${actual}, ` +
            `expected ${check.expected}`,
        );
      }
      this.cursor += 1;
    }
    const keep = Math.min(1, buffer.length);
    this.carry = Buffer.from(buffer.subarray(buffer.length - keep));
    this.carryBase = base + buffer.length - keep;
  }

  /** Every check must have fired by the end of the reply. */
  result() {
    if (this.cursor < this.pending.length) {
      const missed = this.pending[this.cursor];
      throw new Error(
        `the reply ended at ${this.carryBase + this.carry.length} bytes ` +
          `before the sample at offset ${missed.offset} was seen`,
      );
    }
    return { count: this.cursor };
  }
}

/**
 * One POST, the reply consumed in full. The body, when there is one, is
 * streamed from `bodyPath` under its declared `Content-Length`, so a 192 MiB
 * upload never sits in a Node buffer. `checks` are validated as the reply
 * arrives. Resolves after the last reply byte, which is where the timing
 * stops, or rejects on any request, reply or sample failure.
 */
export function consumePreview({
  port,
  path,
  headers = {},
  bodyPath = null,
  bodyLength = null,
  agent = undefined,
  checks = [],
  captureJson = false,
  timeoutMs = REQUEST_TIMEOUT_MS,
}) {
  return new Promise((resolve, reject) => {
    const started = performance.now();
    const requestHeaders = {
      ...headers,
      "Content-Length": bodyPath === null ? "0" : String(bodyLength),
    };
    const request = http.request({
      host: "127.0.0.1",
      port,
      path,
      method: "POST",
      agent,
      headers: requestHeaders,
    });
    let settled = false;
    const fail = (error) => {
      if (settled) return;
      settled = true;
      request.destroy();
      reject(error);
    };
    const finish = (outcome) => {
      if (settled) return;
      settled = true;
      resolve(outcome);
    };
    request.setTimeout(timeoutMs, () => {
      fail(new Error(`the request to ${path} made no progress for ${timeoutMs} ms`));
    });
    request.on("error", (error) => fail(error));
    let bodyBytes = 0;
    let errorHead = Buffer.alloc(0);
    const reportChunks = [];
    const window = new SampleWindow(checks);
    request.on("response", (response) => {
      const outcome = {
        status: response.statusCode,
        headers: response.headers,
        bodyBytes: 0,
        errorText: "",
        elapsed_ms: 0,
        validated: 0,
      };
      response.on("data", (chunk) => {
        bodyBytes += chunk.length;
        if (response.statusCode !== 200) {
          if (errorHead.length < 200) {
            errorHead = Buffer.concat(
              [errorHead, chunk.subarray(0, 200 - errorHead.length)],
            );
          }
          return;
        }
        if (captureJson) {
          if (bodyBytes > 1024 * 1024) { fail(new Error("the file report exceeds 1 MiB")); return; }
          reportChunks.push(chunk);
        }
        try {
          window.push(chunk, bodyBytes - chunk.length);
        } catch (error) {
          fail(error);
        }
      });
      response.on("end", () => {
        outcome.bodyBytes = bodyBytes;
        outcome.errorText = errorHead.toString("utf8");
        outcome.elapsed_ms = performance.now() - started;
        if (response.statusCode === 200) {
          try {
            outcome.validated = window.result().count;
            if (captureJson) outcome.report = JSON.parse(Buffer.concat(reportChunks).toString("utf8"));
          } catch (error) {
            fail(error);
            return;
          }
        }
        finish(outcome);
      });
      response.on("error", (error) => fail(error));
      response.on("close", () => {
        if (!response.complete && !settled) {
          fail(new Error(`the reply from ${path} closed before it ended`));
        }
      });
    });
    if (bodyPath === null) {
      request.end();
      return;
    }
    const stream = createReadStream(bodyPath);
    stream.on("error", (error) => {
      fail(new Error(`the request body could not be read: ${String(error)}`));
    });
    stream.pipe(request);
  });
}

/**
 * What one reply must satisfy. Beyond the status, the channel count and the
 * full three-bytes-per-pixel length, the window the request named has to
 * come back in the existing reply headers, and every sampled byte has to
 * match the synthetic pattern. Anything else is a failed measurement, not a
 * figure.
 */
export function validatePreview(outcome, {
  checks,
  channels,
  replyBytes,
  edge,
  window,
  label,
  source,
}) {
  if (outcome.status !== 200) {
    const code = outcome.headers["x-ocelli-error"] ?? "no X-Ocelli-Error";
    throw new Error(
      `${label} answered ${outcome.status} with ${code}: ${outcome.errorText}`,
    );
  }
  if (source !== undefined && outcome.headers["x-ocelli-source"] !== source) {
    throw new Error(`${label} replied X-Ocelli-Source ${JSON.stringify(outcome.headers["x-ocelli-source"])}, expected ${source}`);
  }
  const replyChannels = outcome.headers["x-ocelli-channels"];
  if (replyChannels !== String(channels)) {
    throw new Error(
      `${label} replied X-Ocelli-Channels ${JSON.stringify(replyChannels)}, ` +
        `expected ${channels}`,
    );
  }
  if (outcome.bodyBytes !== replyBytes) {
    throw new Error(
      `${label} replied ${outcome.bodyBytes} body bytes, expected ` +
        `${replyBytes} (${edge} by ${edge} times ${channels} channels)`,
    );
  }
  for (const [name, expected] of [
    ["x-ocelli-width", edge],
    ["x-ocelli-height", edge],
    ["x-ocelli-window-center", window.center],
    ["x-ocelli-window-width", window.width],
  ]) {
    const header = outcome.headers[name];
    if (header === undefined) {
      throw new Error(`${label} replied no ${name}`);
    }
    if (Number(header) !== expected) {
      throw new Error(
        `${label} replied ${name} ${JSON.stringify(header)}, expected ` +
          `${expected}`,
      );
    }
  }
  if (outcome.validated !== checks.length) {
    throw new Error(
      `${label} validated ${outcome.validated} of ${checks.length} samples`,
    );
  }
}

/** The window headers of one preview request, all of them numeric. */
function windowHeaders(window) {
  return {
    "X-Ocelli-Frame": "0",
    "X-Ocelli-Window-Center": String(window.center),
    "X-Ocelli-Window-Width": String(window.width),
    "X-Ocelli-Window-Function": "LINEAR",
  };
}

/** The held report, the server's own JSON of its budget and held state. */
function getJson(port, path) {
  return new Promise((resolve, reject) => {
    const request = http.request(
      { host: "127.0.0.1", port, path, method: "GET" },
      (response) => {
        let body = "";
        response.setEncoding("utf8");
        response.on("data", (chunk) => {
          body += chunk;
        });
        response.on("end", () => {
          if (response.statusCode !== 200) {
            reject(new Error(`${path} answered ${response.statusCode}`));
            return;
          }
          try {
            resolve(JSON.parse(body));
          } catch (error) {
            reject(new Error(`${path} replied json that did not parse: ${String(error)}`));
          }
        });
        response.on("error", reject);
      },
    );
    request.setTimeout(HELD_REPORT_TIMEOUT_MS, () => {
      request.destroy(new Error(`${path} made no progress for ${HELD_REPORT_TIMEOUT_MS} ms`));
    });
    request.on("error", reject);
    request.end();
  });
}

async function heldReport(port) {
  return getJson(port, "/held");
}

/**
 * Hold one fixture file by the route the page itself uses for a held id,
 * POST /file with the bytes as the body. The reply names the id in
 * X-Ocelli-File, or the viewer refused to hold the file and there are no
 * window steps to take.
 */
export async function holdFile({ port, agent, bodyPath, bodyLength }) {
  const outcome = await consumePreview({
    port,
    agent,
    path: "/file",
    bodyPath,
    bodyLength,
    captureJson: true,
  });
  if (outcome.status !== 200) {
    throw new Error(
      `the viewer refused to hold the file: ${outcome.status} ` +
        `${outcome.errorText}`,
    );
  }
  if (outcome.headers["x-ocelli-held"] !== "1") {
    throw new Error("the viewer did not hold the posted file");
  }
  const file = outcome.headers["x-ocelli-file"];
  if (file === undefined || file === "") {
    throw new Error("the viewer named no file id for the posted file");
  }
  if (outcome.report === null || typeof outcome.report !== "object" || Array.isArray(outcome.report)) {
    throw new Error("the file report is not an object");
  }
  return { id: file, report: outcome.report };
}

/**
 * One workload against one viewer process. Resolves the workload's figures
 * or throws, and stops the sampler, the connections and the process on the
 * way out either way.
 */
export async function measureWorkload({ binary, spec, sampleRss = startRssSampler }) {
  const server = await startViewer(binary, startEnv(spec));
  const agent = new http.Agent({ keepAlive: true, maxSockets: 1 });
  // From the URL announcement to the last measured preview. The sampler is
  // created once startViewer resolves, so it never counts the process's
  // birth, and it is stopped directly after the ninth window step, so the
  // metadata reads that follow are outside the measured window.
  const sampler = sampleRss(server.pid);
  const edge = spec.edge ?? EDGE;
  const minimumHeldBytes = spec.icc
    ? iccSampleBytes(spec.id, edge) + spec.trcTableBytes
    : null;
  try {
    const reported = await reportedBuild(server.url);
    assertRelease(reported);
    const heldAtStart = await heldReport(server.port);

    // File ingestion is untimed. Only /file creates a held id, and this
    // report is retained because encoded bytes may be dropped after decode.
    const held = await holdFile({ port: server.port, agent, bodyPath: spec.path, bodyLength: spec.fileBytes });
    const heldAfterPost = await heldReport(server.port);
    if (spec.icc) assertIccApplied(spec.id, held.report);
    assertHeldBudget(spec, heldAfterPost.budget);
    const identityChecks = checksFor(spec, IDENTITY_WINDOW, edge);
    const first = await consumePreview({
      port: server.port, agent, path: "/preview",
      headers: { ...windowHeaders(IDENTITY_WINDOW), "X-Ocelli-File": held.id },
      checks: identityChecks,
    });
    validatePreview(first, {
      checks: identityChecks, channels: spec.channels, replyBytes: spec.replyBytes,
      edge, window: IDENTITY_WINDOW, label: `${spec.id} first preview`, source: "decoded",
    });

    // The census after the first decode is the profile's charged bytes at
    // rest. The ICC-only census after the steps must retain the same charge
    // and frame count. These extra reads never enter the old AC31 protocol.
    const censusAfterFirst = spec.icc ? await heldReport(server.port) : null;
    if (spec.icc) {
      assertHeldCensus(spec, censusAfterFirst, minimumHeldBytes, "after the first decode");
    }

    const stepSamples = [];
    const stepSources = [];
    let validatedPerStep = 0;
    for (const window of WINDOW_STEPS) {
      const checks = checksFor(spec, window, edge);
      const step = await consumePreview({
        port: server.port,
        agent,
        path: "/preview",
        headers: { ...windowHeaders(window), "X-Ocelli-File": held.id },
        checks,
      });
      validatePreview(step, {
        checks,
        channels: spec.channels,
        replyBytes: spec.replyBytes,
        edge,
        window,
        label: `${spec.id} window step at centre ${window.center}`,
        source: "held",
      });
      validatedPerStep = step.validated;
      stepSamples.push(step.elapsed_ms);
      stepSources.push(step.headers["x-ocelli-source"]);
    }

    const rss = await sampler.stop();
    const censusAfterWindows = spec.icc ? await heldReport(server.port) : null;
    if (spec.icc) {
      assertHeldCensus(spec, censusAfterWindows, minimumHeldBytes, "after the window steps");
      assertCensusStable(spec, censusAfterFirst, censusAfterWindows);
    }
    let heldAtEnd = null;
    try {
      heldAtEnd = await heldReport(server.port);
    } catch (error) {
      // Metadata after the figures. Its failure is recorded, not hidden.
      heldAtEnd = { error: String(error) };
    }
    return {
      reported,
      heldAtStart,
      heldAfterPost,
      heldAtEnd,
      censusAfterFirst,
      censusAfterWindows,
      minimumHeldBytes,
      postReport: held.report,
      first_source: first.headers["x-ocelli-source"],
      stepSources,
      first_ms: first.elapsed_ms,
      first_validated: first.validated,
      stepSamples,
      step_validated: validatedPerStep,
      rss,
    };
  } finally {
    await sampler.stop().catch(() => {});
    agent.destroy();
    await stopViewer(server.child);
  }
}

/**
 * Build all four fixtures, run `workload`, and remove the fixture directory
 * afterwards, failure or not. The four payloads total 896 MiB plus the palette
 * tables and the two profiles: the F-239 pair is 320 MiB of samples plus its
 * tables, and the ICC pair adds 192 MiB and 384 MiB of samples plus their
 * profiles and TRC tables. Nothing synthetic outlives the run. The named edge
 * is for small fixture tests, while run always uses the default largest frame.
 */
export async function withFixtures(dir, workload, edge = EDGE) {
  try {
    const fixtures = await writeColourFrameFixtures(dir, edge);
    const iccFixtures = await writeIccFrameFixtures(dir, edge);
    return await workload({ ...fixtures, ...iccFixtures });
  } finally {
    await rm(dir, { recursive: true, force: true });
  }
}

function workloadDetail(spec, result) {
  const detail = {
    photometric: spec.photometric,
    bits_stored: spec.bitsStored,
    samples_per_pixel: spec.samplesPerPixel,
    fixture_file: spec.file,
    fixture_bytes: spec.fileBytes,
    build_reported: result.reported,
    first_preview_ms: result.first_ms,
    first_preview_source: result.first_source,
    window_sources: result.stepSources,
    file_report: result.postReport,
    held_after_post: result.heldAfterPost,
    held_census_after_first_decode: result.censusAfterFirst,
    held_census_after_windows: result.censusAfterWindows,
    first_preview_window: IDENTITY_WINDOW,
    first_validated_samples: result.first_validated,
    window_samples_ms: result.stepSamples,
    window_steps_median_ms: median(result.stepSamples),
    validated_samples_per_step: result.step_validated,
    rss: result.rss,
    held_at_start: result.heldAtStart,
    held_at_end: result.heldAtEnd,
  };
  if (spec.icc) {
    detail.icc = {
      profile_kind: spec.profileKind,
      profile_bytes: spec.profileBytes,
      trc_table_bytes: spec.trcTableBytes,
      minimum_held_bytes: result.minimumHeldBytes,
      requested_cache_mib: requestedCacheMib(spec),
      held_budget_bytes: result.heldAfterPost.budget,
      icc_reported: {
        icc_applied: result.postReport.icc_applied,
        icc_kind: result.postReport.icc_kind,
      },
      marker_positions: spec.markers,
      marker_codes: spec.codes,
    };
  }
  return detail;
}

/** One run of the subject. Four workloads, each in its own process. */
export async function run({ outDir = benchPath("out"), build = true } = {}) {
  const binary = await ensureViewerBinary("release", build);
  const digest = createHash("sha256");
  for await (const chunk of createReadStream(binary)) digest.update(chunk);
  const buildIdentity = { binary, sha256: digest.digest("hex"), profile: "release" };
  return withFixtures(join(outDir, "colour-frame"), async (fixtures) => {
    const rgb = await measureWorkload({ binary, spec: fixtures.rgb8 });
    const palette = await measureWorkload({ binary, spec: fixtures.palette16 });
    const icc8 = await measureWorkload({ binary, spec: fixtures.icc8 });
    const icc16 = await measureWorkload({ binary, spec: fixtures.icc16 });
    const value = median(rgb.stepSamples);
    return {
      value,
      detail: {
        statistic: "median",
        value_is:
          "the median of the nine held colour window steps on the untagged " +
          "8-bit RGB workload",
        profile: rgb.reported,
        build_identity: buildIdentity,
        rows: EDGE,
        columns: EDGE,
        reply_channels: 3,
        identity_window: IDENTITY_WINDOW,
        window_steps: WINDOW_STEPS,
        rss_period_ms: RSS_PERIOD_MS,
        instrument_note:
          "no browser and no GPU: the subject times HTTP /preview and the " +
          "server process, and each workload runs in its own viewer process. " +
          "The 16-bit ICC workload alone starts its child with " +
          "OCELLI_CACHE_MIB=512 and records requested and actual budget; " +
          "every other workload inherits the default",
        workloads: {
          rgb8: workloadDetail(fixtures.rgb8, rgb),
          palette16: workloadDetail(fixtures.palette16, palette),
          icc8: workloadDetail(fixtures.icc8, icc8),
          icc16: workloadDetail(fixtures.icc16, icc16),
        },
      },
    };
  });
}
