// F-223's equipped comparison. The pure measurement contracts live beside
// this file. Input paths and headers remain private to this run.
import { execFile } from "node:child_process";
import { promisify } from "node:util";
import { createHash } from "node:crypto";
import { createRequire } from "node:module";
import { Buffer } from "node:buffer";
import { constants, createReadStream } from "node:fs";
import { readFile, writeFile, readdir, stat, mkdir, mkdtemp, rm, copyFile, rename } from "node:fs/promises";
import os from "node:os";
import { join, resolve, relative, sep } from "node:path";
import { performance } from "node:perf_hooks";
import { pathToFileURL } from "node:url";
import { repoPath, benchPath } from "./paths.mjs";
import { hostClass } from "./hostclass.mjs";
import { ensureReleaseBinary, ensureWorkloads, startViewer, stopViewer, reportedBuild, SERVER_PERMITS } from "./viewer_work.mjs";
import { readColourComparisonInput, measureColourComparison } from "./comparison_colour.mjs";
import { ARCHITECTURE, NOT_SHOWN, FOOTPRINT_PERIOD, SIDES, WORKLOAD_IDS, SESSION_WORKLOAD, loadComparison, trialOrder, summarise, parseProcessTree, parseFootprint, memoryFigures, cpuFigure, selectWorkloads, inputIdentity, scanForDenied, scanRecordForDenied, publishedRecord, renderSummary, renderReport, otherBrowserJobs, recordCondition } from "./comparison.mjs";

const exec = promisify(execFile);
const oracleRequire = createRequire(pathToFileURL(repoPath("tools", "oracle", "package.json")));
const pause = (ms) => new Promise((done) => setTimeout(done, ms));
const epochNow = () => performance.timeOrigin + performance.now();
const WINDOW = { center: 40, width: 400, function: "LINEAR" };
const CPU_BASELINE_ATTEMPTS = 3;
const RUN_REFUSALS = {
  environment_mismatch: { message: "the two browsers reported different versions or renderers", exitCode: 2 },
  software_after_headed: { message: "the browser still reports SwiftShader, pass --software only to record that renderer", exitCode: 2 },
  invalid_publication: { message: "publication refuses invalid comparison metrics", exitCode: 1 },
};
const DENIED_TAGS = {
  names: ["x00100010", "x00080090"], ids: ["x00100020", "x00080050"],
  dates: ["x00100030", "x00080020", "x00080021", "x00080022"],
  institutions: ["x00080080"], uids: ["x0020000d", "x0020000e", "x00080018"],
};

const COMPARISON_FAILURE = Symbol('comparison refusal');
function comparisonFailure(exitCode) { return { marker: COMPARISON_FAILURE, exitCode }; }

/** Cross-context page-ready endpoints share the host's epoch-mapped clock. */
export function readyDuration(startEpoch, endClock) {
  if (!Number.isFinite(startEpoch) || !Number.isFinite(endClock?.timeOrigin) || !Number.isFinite(endClock?.now)) throw new Error("page-ready clock was not observed", { cause: comparisonFailure(2) });
  const elapsed = endClock.timeOrigin + endClock.now - startEpoch;
  if (elapsed < 0) throw new Error("page-ready clocks do not share the measured interval", { cause: comparisonFailure(2) });
  return Math.round(elapsed * 10000) / 10000;
}

export async function listInputFiles(root) {
  const paths = [];
  async function walk(dir) {
    for (const entry of (await readdir(dir, { withFileTypes: true })).sort((a, b) => a.name.localeCompare(b.name))) {
      const path = join(dir, entry.name);
      if (entry.isDirectory()) await walk(path);
      else if (entry.isFile()) paths.push(path);
    }
  }
  try { await walk(root); } catch { throw new Error("--pseudo needs a readable local directory", { cause: comparisonFailure(2) }); }
  return paths;
}

/** Header values are used for selection and the deny set, never a report. */
export async function readHeaders(paths, source, root, parser, denySet, digests = null) {
  const headers = [];
  for (const path of paths) {
    let bytes;
    try { bytes = await readFile(path); } catch { throw new Error("a comparison input could not be read", { cause: comparisonFailure(2) }); }
    if (source === "pseudo") {
      for (const component of relative(root, path).split(sep)) if (component) denySet.paths.push(component);
    }
    const digest = createHash("sha256").update(bytes).digest("hex");
    if (digests) digests.push(digest);
    if (bytes.length < 132 || bytes.toString("ascii", 128, 132) !== "DICM") continue;
    let data;
    try { data = parser.parseDicom(new Uint8Array(bytes.buffer, bytes.byteOffset, bytes.byteLength), { untilTag: "x7fe00010" }); }
    catch { throw new Error("a comparison Part 10 header could not be parsed", { cause: comparisonFailure(2) }); }
    const string = (tag) => data.string(tag)?.trim() || "";
    if (source === "pseudo") for (const [kind, tags] of Object.entries(DENIED_TAGS)) {
      for (const tag of tags) { const value = string(tag); if (value) denySet[kind].push(value); }
    }
    const rows = data.uint16("x00280010"), columns = data.uint16("x00280011");
    if (!rows || !columns) continue;
    headers.push({ path, source, digest, seriesUid: string("x0020000e"), modality: string("x00080060"), rows, columns, photometric: string("x00280004"), hasWindowCenter: Boolean(string("x00281050")), hasWindowWidth: Boolean(string("x00281051")) });
  }
  return headers;
}

export async function prepareInputs(pseudo, temp, parser, includeSynthetic = true) {
  let info;
  try { info = await stat(pseudo); } catch { throw new Error("--pseudo needs a readable local directory", { cause: comparisonFailure(2) }); }
  if (!info.isDirectory()) throw new Error("--pseudo needs a readable local directory", { cause: comparisonFailure(2) });
  const denySet = { names: [], ids: [], dates: [], institutions: [], uids: [], paths: [] };
  const pseudoFiles = await listInputFiles(pseudo), digests = [];
  const pseudoHeaders = await readHeaders(pseudoFiles, "pseudo", pseudo, parser, denySet, digests);
  if (!pseudoHeaders.length) throw new Error("--pseudo contains no usable Part 10 image", { cause: comparisonFailure(2) });
  const synthetic = [];
  if (includeSynthetic) {
    const generated = await ensureWorkloads(join(temp, "synthetic"));
    for (const dir of [generated.ctDir, generated.dxDir]) synthetic.push(...await readHeaders(await listInputFiles(dir), "synthetic", dir, parser, denySet));
  }
  const selections = selectWorkloads([...synthetic, ...pseudoHeaders]);
  const directories = {};
  for (const id of WORKLOAD_IDS) {
    const selected = selections[id];
    if (selected.state !== "selected") continue;
    if (id === "pseudo-full") { directories[id] = pseudo; continue; }
    const dir = join(temp, id); await mkdir(dir);
    for (let index = 0; index < selected.files.length; index += 1) {
      // Neutral regular files preserve order and work with actual directory
      // uploads. FICLONE attempts CoW, with ordinary copying as its fallback.
      await copyFile(selected.files[index], join(dir, `${String(index).padStart(6, "0")}.dcm`), constants.COPYFILE_FICLONE);
    }
    directories[id] = dir;
  }
  return { denySet, selections, directories, identity: inputIdentity(digests), submitted_file_count: pseudoFiles.length };
}

export function safeOutputs(record, denySet) {
  if (scanRecordForDenied(record, denySet).length) throw new Error("comparison output contains a denied input value", { cause: comparisonFailure(2) });
  const json = JSON.stringify(record, null, 2) + "\n";
  const text = renderReport(record);
  if (scanForDenied(json, denySet).length || scanForDenied(text, denySet).length) throw new Error("comparison output contains a denied input value", { cause: comparisonFailure(2) });
  return { json, text };
}

/** Supplemental records keep the same deny checks and add an exact-byte
 * content digest. Public selector constants are excluded at input selection. */
export function safeColourOutputs(record, text, denySet) {
  const json = JSON.stringify(record, null, 2) + "\n";
  if (scanRecordForDenied(record, denySet).length || scanForDenied(json, denySet).length || scanForDenied(text, denySet).length) throw new Error("comparison output contains a denied input value", { cause: comparisonFailure(2) });
  const digest = createHash("sha256").update(json).digest("hex");
  if (scanForDenied(digest, denySet).length) throw new Error("comparison digest contains a denied input value", { cause: comparisonFailure(2) });
  return { json, text, digest };
}

/** Both files are validated before either target is changed. Roll back a
 * partial replacement if the second rename fails. No raw input survives. */
export async function writePair(jsonPath, markdownPath, json, markdown) {
  const paths = [jsonPath, markdownPath], bodies = [json, markdown], old = [];
  for (const path of paths) { try { old.push(await readFile(path)); } catch (error) { if (error.code !== "ENOENT") throw error; old.push(null); } }
  try {
    for (let i = 0; i < paths.length; i += 1) await writeFile(`${paths[i]}.next`, bodies[i]);
    for (const path of paths) await rename(`${path}.next`, path);
  } catch {
    for (let i = 0; i < paths.length; i += 1) {
      if (old[i] === null) await rm(paths[i], { force: true }); else await writeFile(paths[i], old[i]);
    }
    throw new Error("comparison pair could not be replaced together", { cause: comparisonFailure(2) });
  } finally { for (const path of paths) await rm(`${path}.next`, { force: true }); }
}

function cpuSeconds(text) {
  const parts = text.split(":").map(Number);
  if (parts.length < 2 || parts.length > 3 || parts.some((p) => !Number.isFinite(p) || p < 0)) throw new Error("a process CPU counter was invalid", { cause: comparisonFailure(1) });
  return parts.reduce((seconds, part) => seconds * 60 + part, 0);
}

export async function processInventory(browser, nativePid = null) {
  const session = await browser.newBrowserCDPSession();
  let processes;
  try { processes = (await session.send("SystemInfo.getProcessInfo")).processInfo; }
  finally { await session.detach(); }
  const root = processes.find((process) => process.type === "browser")?.id;
  if (!Number.isSafeInteger(root)) throw new Error("the browser process inventory has no root", { cause: comparisonFailure(1) });
  const { stdout } = await exec("ps", ["-A", "-o", "pid=,ppid="]);
  const pids = parseProcessTree(stdout, root);
  for (const process of processes) pids.add(process.id);
  if (nativePid !== null) pids.add(nativePid);
  return pids;
}

async function cpuSnapshot(pids) {
  const { stdout } = await exec("ps", ["-A", "-o", "pid=,time="]);
  const found = new Map();
  for (const line of stdout.split("\n")) {
    const match = /^\s*(\d+)\s+(\S+)\s*$/.exec(line);
    if (match && pids.has(Number(match[1]))) found.set(Number(match[1]), cpuSeconds(match[2]));
  }
  if (found.size !== pids.size) throw new Error("a process exited during CPU collection", { cause: comparisonFailure(1) });
  return found;
}

// Establish a coherent baseline before navigation or sampling starts. A
// missing PID retries both inventory and counters, at most three total attempts. No
// action, end snapshot or measured trial is repeated to obtain a value.
async function cpuBaseline(browser, nativePid) {
  let attempts = 0;
  for (;;) {
    try { return await cpuSnapshot(await processInventory(browser, nativePid)); }
    catch (error) {
      attempts += 1;
      if (attempts >= CPU_BASELINE_ATTEMPTS || error.cause?.marker !== COMPARISON_FAILURE ||
          error.message !== "a process exited during CPU collection") throw error;
    }
  }
}

export function startProcessSampler(browser, nativePid = null, infrastructurePid = process.pid) {
  let stopped = false, failure = null;
  const samples = [], infrastructure = [], sampleTimes = [];
  const started = performance.now();
  const running = (async () => {
    while (!stopped) {
      let stage = "inventory";
      try {
        const pids = await processInventory(browser, nativePid);
        const args = ["-f", "bytes", ...[...pids].flatMap((pid) => ["-p", String(pid)])];
        stage = "product-tool";
        const { stdout } = await exec("/usr/bin/footprint", args, { maxBuffer: 16 * 1024 * 1024 });
        stage = "product-parse";
        const figures = parseFootprint(stdout);
        stage = "product-coverage";
        if ([...pids].some((pid) => !figures.has(pid))) throw new Error("a footprint sample omitted a product process", { cause: comparisonFailure(1) });
        stage = "product-values";
        if ([...figures.values()].some(({ footprint, peak }) => !Number.isFinite(footprint) || footprint <= 0 || !Number.isFinite(peak) || peak < footprint)) throw new Error("a footprint sample did not observe positive product memory and its peak", { cause: comparisonFailure(1) });
        samples.push(figures); sampleTimes.push(performance.now() - started);
        stage = "infrastructure-tool";
        const infra = await exec("/usr/bin/footprint", ["-f", "bytes", "-p", String(infrastructurePid)], { maxBuffer: 4 * 1024 * 1024 });
        stage = "infrastructure-parse";
        infrastructure.push([...parseFootprint(infra.stdout)]);
      } catch (error) {
        const categories = {
          EPERM: "permission", EACCES: "permission", ENOENT: "tool-missing",
          ERR_CHILD_PROCESS_STDIO_MAXBUFFER: "output-limit",
        };
        const code = error?.code;
        const category = typeof code === "string" && Object.hasOwn(categories, code)
          ? categories[code] : Number.isInteger(code) ? "numeric-code"
            : error?.killed === true ? "killed"
              : error?.cause?.marker === COMPARISON_FAILURE ? "comparison-refusal"
                : "unclassified";
        // Foreign tool errors can carry command arguments and stderr. Only
        // these closed labels cross the existing comparison-error boundary.
        failure = `process memory collection failed [stage=${stage},category=${category}]`;
        break;
      }
      await pause(FOOTPRINT_PERIOD);
    }
  })();
  return { async stop() { stopped = true; await running; if (failure || !samples.length) throw new Error(failure || "no process memory sample was observed", { cause: comparisonFailure(1) }); return { figures: memoryFigures(samples), samples: samples.map((sample) => [...sample]), sample_times_ms: sampleTimes, infrastructure }; } };
}

// Runs before the product page. Its only product intervention is observation
// of the actual visible-canvas commit and the series status endpoint.
function observeOcelli() {
  const state = { generation: 0, openedAt: null, committedAt: null, commitSequence: 0, index: -1, blank: true };
  window.__comparison = state;
  const put = CanvasRenderingContext2D.prototype.putImageData;
  CanvasRenderingContext2D.prototype.putImageData = function (...args) {
    const result = put.apply(this, args);
    if (this.canvas.matches(".cell.active canvas.view")) {
      state.committedAt = performance.now();
      state.commitSequence += 1;
      state.blank = false;
      try { state.index = activeViewport().viewIndex; } catch { state.index = -1; }
    }
    return result;
  };
  document.addEventListener("change", (event) => {
    if (event.target?.id === "folder") {
      state.generation += 1;
      state.openedAt = null;
      state.committedAt = null;
      state.blank = true;
    }
  }, true);
  document.addEventListener("DOMContentLoaded", () => {
    const status = document.getElementById("status");
    new MutationObserver(() => {
      if (state.generation && state.openedAt === null && /\d+ series\./.test(status.textContent || "")) state.openedAt = performance.now();
    }).observe(status, { childList: true, characterData: true, subtree: true });
  });
}

// Observe the first rAF after the actual source events inside each page.
// Controller round trips therefore cannot add an extra frame to a duration.
function observeEndpoints(side) {
  const clock = { ready: null, openedSource: null, openedGeneration: null, openRafAt: null, commits: [] };
  window.__comparisonClock = clock;
  window.addEventListener("load", async () => {
    if (side === "cornerstone") await window.__bench.ready();
    else while (!document.getElementById("folder") || document.getElementById("folder").disabled) await new Promise(requestAnimationFrame);
    requestAnimationFrame(() => { clock.ready = { timeOrigin: performance.timeOrigin, now: performance.now() }; });
  });
  let lastCommit = 0;
  function frame() {
    const state = side === "ocelli" ? window.__comparison : window.__bench?.snapshot();
    const now = performance.now();
    if (Number.isFinite(state?.openedAt) && state.openedAt > 0 && (state.openedAt !== clock.openedSource || state.generation !== clock.openedGeneration)) {
      clock.openedSource = state.openedAt; clock.openedGeneration = state.generation; clock.openRafAt = now;
    }
    if (state?.commitSequence > lastCommit && Number.isFinite(state.committedAt)) {
      lastCommit = state.commitSequence;
      clock.commits.push({ sequence: lastCommit, source: state.committedAt, now, index: state.index, generation: state.generation });
    }
    requestAnimationFrame(frame);
  }
  requestAnimationFrame(frame);
}

async function browserRenderer(page) {
  return page.evaluate(() => {
    const canvas = document.createElement("canvas");
    const gl = canvas.getContext("webgl2") || canvas.getContext("webgl");
    const extension = gl?.getExtension("WEBGL_debug_renderer_info");
    return extension ? gl.getParameter(extension.UNMASKED_RENDERER_WEBGL) : null;
  });
}

export async function launchSide(side, context, navigate = true) {
  if (context.interrupted) throw new Error("comparison interrupted", { cause: comparisonFailure(130) });
  const started = epochNow();
  let server = null, browser = null;
  try {
    let nativeBudgetBytes = null;
    if (side === "ocelli") {
      const caseMiB = context.colourCase?.serverCacheMiB;
      server = await startViewer(context.binary, { OCELLI_CACHE_MIB: caseMiB ? String(caseMiB) : undefined });
      const held = await fetch(`${server.url}held`).then((response) => response.json());
      nativeBudgetBytes = held.budget;
      if (!Number.isSafeInteger(nativeBudgetBytes) || nativeBudgetBytes <= 0 || (caseMiB && nativeBudgetBytes !== caseMiB * 1024 * 1024)) throw new Error("the native viewer did not report the declared cache budget", { cause: comparisonFailure(2) });
    }
    browser = await context.chromium.launch({ channel: "chromium", headless: !context.headed, args: ["--force-device-scale-factor=1"] });
    const actor = { side, browser, server, started, context, nativeBudgetBytes, page: null };
    actor.newPage = async (load = true) => {
      // browser.newPage owns a fresh context. Retain the browser/server,
      // never the previous trial's page and its per-page resources.
      if (actor.page) await actor.page.close();
      const page = await browser.newPage({ viewport: { width: 1600, height: 1000 }, deviceScaleFactor: 1 });
      page.setDefaultTimeout(120000);
      if (side === "ocelli") await page.addInitScript(observeOcelli);
      await page.addInitScript(observeEndpoints, side);
      actor.page = page;
      actor.navigate = async () => {
        const gotoStart = epochNow();
        await page.goto(side === "ocelli" ? server.url : `${context.origin}/bench.html`);
        await page.waitForFunction(() => window.__comparisonClock.ready !== null);
        const end = await page.evaluate(() => window.__comparisonClock.ready);
        actor.ready = readyDuration(side === "ocelli" && actor.ready === undefined ? actor.started : gotoStart, end);
      };
      if (load) await actor.navigate();
      return page;
    };
    let closing = null;
    actor.close = () => {
      closing ||= (async () => {
        try { await browser.close(); }
        finally { try { if (server) await stopViewer(server.child); } finally { context.actors?.delete(actor); } }
      })().catch((error) => { context.cleanupFailed = true; throw error; });
      return closing;
    };
    context.actors?.add(actor);
    await actor.newPage(navigate);
    return actor;
  } catch (error) {
    if (browser) await browser.close().catch(() => {});
    if (server) await stopViewer(server.child).catch(() => {});
    throw error;
  }
}


export async function openFolder(actor, dir) {
  const page = actor.page;
  const before = await page.evaluate(() => ({ start: performance.now(), openedSource: window.__comparisonClock.openedSource, openedGeneration: window.__comparisonClock.openedGeneration }));
  await page.locator("#folder").setInputFiles(dir);
  if (actor.side === "cornerstone") await page.evaluate(() => window.__bench.open());
  await page.waitForFunction((previous) => {
    const clock = window.__comparisonClock;
    return Number.isFinite(clock.openedSource) && (clock.openedSource !== previous.openedSource || clock.openedGeneration !== previous.openedGeneration) && clock.commits.some((commit) => commit.source >= clock.openedSource && commit.generation === clock.openedGeneration);
  }, before);
  const end = await page.evaluate(() => {
    const clock = window.__comparisonClock;
    return { opened: clock.openedSource, openRaf: clock.openRafAt, firstRaf: clock.commits.find((commit) => commit.source >= clock.openedSource && commit.generation === clock.openedGeneration).now };
  });
  return { open: end.openRaf - before.start, first_image: end.firstRaf - end.opened };
}

export async function windowStep(actor, center, width = 400) {
  if (actor.side === "cornerstone") return actor.page.evaluate(async ({ center, width }) => {
    const start = performance.now(); await window.__bench.windowChange({ center, width, function: "LINEAR" });
    return performance.now() - start;
  }, { center, width });
  return actor.page.evaluate(async ({ center, width }) => {
    const before = window.__comparison.commitSequence, start = performance.now();
    const vp = activeViewport();
    if (!await chooseWindow(vp, { override: { center, width, fn: "LINEAR" } })) throw new Error("the comparison window was refused");
    await new Promise((done) => { function frame() {
      if (window.__comparison.commitSequence > before && Number(vp.shownWindow?.center) === center && Number(vp.shownWindow?.width) === width) done(); else requestAnimationFrame(frame);
    } requestAnimationFrame(frame); });
    return performance.now() - start;
  }, { center, width });
}

/** Supplemental controlled cases reuse the actual actors and page endpoints.
 * A warm observation follows one completed trial in the same process pair. */
export async function measureColourTrial(side, { directory, warm, context, caseSpec }) {
  context.colourCase = caseSpec;
  const actor = await launchSide(side, context);
  try {
    if (warm) { await openFolder(actor, directory); await actor.page.close(); await actor.newPage(); }
    const result = await openFolder(actor, directory);
    return { open_ms: result.open, first_image_ms: result.first_image, browser_version: actor.browser.version(), renderer: await browserRenderer(actor.page), native_budget_bytes: actor.nativeBudgetBytes };
  } finally { await actor.close(); }
}

/** Open, first paint and nine window changes, all inside the same native,
 * browser and process-inventory session. There is no extra scroll workload. */
export async function measureColourMemorySession(side, { directory, caseSpec, context }) {
  context.colourCase = caseSpec;
  const actor = await launchSide(side, context, false);
  let sampler;
  try {
    if (process.platform !== "darwin") return { state: "unavailable", reason: `no collector for ${process.platform}`, native_budget_bytes: actor.nativeBudgetBytes };
    const before = await cpuBaseline(actor.browser, actor.server?.pid ?? null);
    sampler = startProcessSampler(actor.browser, actor.server?.pid ?? null);
    await actor.navigate();
    await openFolder(actor, directory);
    for (let index = 0; index < 9; index += 1) await windowStep(actor, 100 + index, 200);
    const memory = await sampler.stop(); sampler = null;
    const after = await cpuSnapshot(await processInventory(actor.browser, actor.server?.pid ?? null));
    return { ...memory, cpu: cpuFigure(before, after), before: [...before], after: [...after], browser_version: actor.browser.version(), renderer: await browserRenderer(actor.page), native_budget_bytes: actor.nativeBudgetBytes };
  } finally {
    try { if (sampler) await sampler.stop(); } finally { await actor.close(); }
  }
}

export async function scroll(actor) {
  return actor.page.evaluate((side) => new Promise((done) => {
    let start = null, keys = 0, frames = 0, blank = 0, previous = -1;
    function observe() {
      const state = side === "ocelli" ? window.__comparison : window.__bench.snapshot();
      const stage = side === "ocelli" ? document.querySelector(".cell.active .stage") : null;
      const fault = side === "ocelli" ? document.querySelector(".cell.active .fault") : null;
      const visibleFault = fault && !fault.hidden && (fault.textContent || "").trim();
      const missing = state.blank || (side === "ocelli" && (!stage || stage.hidden || visibleFault));
      if (missing) blank += 1;
      else if (state.index !== previous) { if (previous >= 0) frames += 1; previous = state.index; }
    }
    function frame() {
      const now = performance.now();
      if (start === null) start = now;
      observe();
      if (keys >= 150 || now - start >= 3000) {
        setTimeout(() => { observe(); done({ keys, frames, blank, elapsed_ms: performance.now() - start }); }, 300);
        return;
      }
      document.dispatchEvent(new KeyboardEvent("keydown", { key: "ArrowDown", bubbles: true })); keys += 1;
      requestAnimationFrame(frame);
    }
    requestAnimationFrame(frame);
  }), actor.side);
}

async function capture(actor) {
  if (actor.side === "cornerstone") return actor.page.evaluate((request) => window.__bench.capture(request), WINDOW);
  await windowStep(actor, 40);
  return actor.page.evaluate(() => {
    const image = currentImage(activeViewport());
    const canvas = document.querySelector(".cell.active canvas.view");
    if (!image || !canvas || canvas.width !== image.info.columns || canvas.height !== image.info.rows) throw new Error("pixel-check canvas dimensions differ");
    const bytes = canvas.getContext("2d").getImageData(0, 0, canvas.width, canvas.height).data;
    let binary = "";
    for (let at = 0; at < bytes.length; at += 32768) binary += String.fromCharCode(...bytes.subarray(at, at + 32768));
    return { width: canvas.width, height: canvas.height, rgba_base64: btoa(binary), name: image.file.name, size: image.file.size };
  });
}

async function metadata(actor) {
  const renderer = await browserRenderer(actor.page);
  if (typeof renderer !== "string" || !renderer) throw new Error("the browser exposed no WebGL renderer", { cause: comparisonFailure(2) });
  if (actor.side === "cornerstone") {
    const found = await actor.page.evaluate(() => window.__bench.metadata());
    return { version: actor.browser.version(), renderer, defaults: { maxCacheSizeBytes: found.cache_bytes, decodeWorkers: found.workers } };
  }
  const held = await fetch(`${actor.server.url}held`).then((response) => response.json());
  const pageBudget = await actor.page.evaluate(() => CACHE_BYTE_BUDGET);
  if (await reportedBuild(actor.server.url) !== "release") throw new Error("the comparison requires a release viewer", { cause: comparisonFailure(2) });
  return { version: actor.browser.version(), renderer, defaults: { serverCacheBytes: held.budget, pageGreyCacheBytes: pageBudget, decodePermits: SERVER_PERMITS } };
}

export async function pixelPrecheck(context, inputs, temp) {
  if (!inputs.directories["pseudo-ct"]) throw new Error("the pixel precondition has no qualifying pseudo CT series", { cause: comparisonFailure(2) });
  let sides = [];
  try {
    for (const side of SIDES) sides.push(await launchSide(side, context));
    const meta = await Promise.all(sides.map(metadata));
    if (meta[0].version !== meta[1].version || meta[0].renderer !== meta[1].renderer) return { environmentRefusal: "environment_mismatch", metadata: meta };
    if (/swiftshader/i.test(meta[0].renderer) && !context.software) return { needsHeaded: true, metadata: meta };
    const frames = [];
    for (const actor of sides) { await openFolder(actor, inputs.directories["pseudo-ct"]); frames.push(await capture(actor)); }
    if (frames[0].name !== frames[1].name || frames[0].size !== frames[1].size) throw new Error("the two sides selected different images", { cause: comparisonFailure(1) });
    if (frames[0].width !== frames[1].width || frames[0].height !== frames[1].height) throw new Error("the pixel-check dimensions disagree", { cause: comparisonFailure(1) });
    const raw = [join(temp, "candidate.raw"), join(temp, "reference.raw")];
    for (let i = 0; i < frames.length; i += 1) {
      const bytes = Buffer.from(frames[i].rgba_base64, "base64");
      if (bytes.length !== frames[i].width * frames[i].height * 4) throw new Error("the pixel-check capture has the wrong byte length", { cause: comparisonFailure(1) });
      await writeFile(raw[i], bytes);
    }
    const result = await exec(context.comparator, ["pair", "--reference", raw[1], "--candidate", raw[0], "--width", String(frames[0].width), "--height", String(frames[0].height)]);
    return { verdict: JSON.parse(result.stdout), metadata: meta };
  } finally { for (const actor of sides) await actor.close().catch(() => {}); }
}

async function compareExclusive() {
  const commands = await exec("ps", ["-A", "-o", "pid=,command="]);
  const parents = await exec("ps", ["-A", "-o", "pid=,ppid="]);
  const ours = parseProcessTree(parents.stdout, process.pid);
  const count = otherBrowserJobs(commands.stdout, ours);
  if (count) throw new Error(`${count} other Playwright browser job(s) prevent a comparison`, { cause: comparisonFailure(2) });
}

async function prepareBuild(options, out) {
  let pins, buildPage, serveDirectory, chromium;
  try {
    const oracle = await import(pathToFileURL(repoPath("tools", "oracle", "run.mjs")).href);
    pins = oracle.checkPins();
    ({ buildPage } = await import(pathToFileURL(repoPath("tools", "oracle", "build-page.mjs")).href));
    ({ serveDirectory } = await import(pathToFileURL(repoPath("tools", "oracle", "src", "server.mjs")).href));
    const loaded = await import(pathToFileURL(oracleRequire.resolve("playwright")).href);
    chromium = loaded.chromium || loaded.default.chromium;
  } catch { throw new Error("the comparison requires the pinned tools/oracle install", { cause: comparisonFailure(2) }); }
  const source = await sourceIdentity();
  const binary = await ensureReleaseBinary(options.build !== false);
  const metadata = await exec("cargo", ["metadata", "--no-deps", "--format-version", "1"], { cwd: repoPath(), maxBuffer: 8 * 1024 * 1024 });
  const target = JSON.parse(metadata.stdout).target_directory;
  const comparator = join(target, "release", process.platform === "win32" ? "ocelli-compare.exe" : "ocelli-compare");
  if (options.build !== false) await exec(repoPath("bin", "ocelli.sh"), ["cargo", "build", "--release", "-p", "ocelli-oracle", "--bin", "ocelli-compare"], { cwd: repoPath(), env: { ...process.env, CARGO_BUILD_JOBS: "2" }, maxBuffer: 8 * 1024 * 1024 });
  try { await stat(comparator); } catch { throw new Error("the release pixel comparator has not been built", { cause: comparisonFailure(2) }); }
  const dist = join(out, "page");
  if (options.build !== false) await buildPage({ dist });
  else { try { await stat(join(dist, "bench.js")); } catch { throw new Error("the comparison page has not been built", { cause: comparisonFailure(2) }); } }
  const binaryHash = async (file) => { const hash = createHash("sha256"); for await (const chunk of createReadStream(file)) hash.update(chunk); return hash.digest("hex"); };
  const pageHash = createHash("sha256");
  for (const file of await listInputFiles(dist)) { pageHash.update(relative(dist, file)); pageHash.update("\0"); pageHash.update(await readFile(file)); }
  Object.assign(source, { binary_sha256: await binaryHash(binary), comparator_sha256: await binaryHash(comparator), page_sha256: pageHash.digest("hex") });
  const server = await serveDirectory(dist);
  if ([8742, 8765].includes(Number(new URL(server.origin).port))) { await server.close(); throw new Error("the comparison static server used a reserved port", { cause: comparisonFailure(2) }); }
  return { binary, comparator, origin: server.origin, chromium, pins, close: server.close, source, binary_sha256: source.binary_sha256 };
}

async function sourceIdentity() {
  try {
    const head = (await exec("git", ["rev-parse", "HEAD"], { cwd: repoPath() })).stdout.trim();
    const tree = (await exec("git", ["write-tree"], { cwd: repoPath() })).stdout.trim();
    await exec("git", ["diff", "--quiet"], { cwd: repoPath() });
    const untracked = await exec("git", ["ls-files", "--others", "--exclude-standard", "tools/bench", "tools/oracle", "examples/folder-viewer", "crates"], { cwd: repoPath() });
    if (untracked.stdout.trim()) throw new Error("stage the comparison source inputs before taking a measurement", { cause: comparisonFailure(2) });
    return { head, index_tree: tree, worktree_matches_index: true };
  } catch (error) { if (error.cause?.marker === COMPARISON_FAILURE) throw error; throw new Error("the comparison source identity is unavailable or differs from the index", { cause: comparisonFailure(2) }); }
}

function unavailable(reason) { return { state: "unavailable", n: 0, reason }; }

/** N is per side. The pure order helper counts individual trials, so the
 * consumer requests 2N and retains each side's observations separately. */
export async function collectTrialSamples(n, startSide, observe) {
  if (!Number.isInteger(n) || n < 1) throw new Error("the comparison needs a positive whole per-side trial count", { cause: comparisonFailure(2) });
  const samples = { ocelli: [], cornerstone: [] }, upper = { ocelli: [], cornerstone: [] }, trials = [];
  const attempts = { ocelli: 0, cornerstone: 0 };
  for (const side of trialOrder(SIDES, n * 2, startSide)) {
    try {
      const result = await observe(side, attempts[side]++);
      if (!Number.isFinite(result?.value) || result.value < 0) throw new Error("a comparison trial did not observe a finite figure", { cause: comparisonFailure(1) });
      if (result.upper !== undefined && (!Number.isFinite(result.upper) || result.upper < result.value)) throw new Error("a memory upper bound was below its concurrent observation", { cause: comparisonFailure(1) });
      samples[side].push(result.value);
      if (result.upper !== undefined) upper[side].push(result.upper);
      trials.push({ side, value: result.value, ...(result.upper !== undefined ? { upper: result.upper } : {}) });
    } catch { trials.push({ side, error: "the trial did not complete" }); }
  }
  return { samples, upper, trials };
}

async function memorySession(actor, inputs) {
  if (process.platform !== "darwin") return { unavailable: `no collector for ${process.platform}` };
  const before = await cpuBaseline(actor.browser, actor.server?.pid ?? null);
  const sampler = startProcessSampler(actor.browser, actor.server?.pid ?? null);
  let figures;
  try {
    await actor.navigate();
    await openFolder(actor, inputs.directories["pseudo-full"]);
    await openFolder(actor, inputs.directories["pseudo-ct"]);
    const delivery = await scroll(actor);
    if (delivery.keys < 120 || delivery.blank) throw new Error("the memory-session scroll did not present its workload", { cause: comparisonFailure(1) });
    for (const id of ["pseudo-ct", "pseudo-dx"]) {
      await openFolder(actor, inputs.directories[id]);
      for (let index = 0; index < 9; index += 1) await windowStep(actor, 40 + index);
    }
  } finally { figures = await sampler.stop(); }
  const after = await cpuSnapshot(await processInventory(actor.browser, actor.server?.pid ?? null));
  return { ...figures, cpu: cpuFigure(before, after), before: [...before], after: [...after] };
}

export async function measureAll(context, inputs, definition) {
  const metrics = {}, raw = [];
  const memory = { ocelli: [], cornerstone: [] };
  let firstSide = 0;
  for (const metric of definition.metrics) {
    metrics[metric.id] = {};
    const workloads = metric.workloads.length ? metric.workloads : [SESSION_WORKLOAD];
    for (const id of workloads) {
      const startSide = SIDES[firstSide];
      firstSide = 1 - firstSide;
      for (const condition of ["cold", "warm"]) {
        const rule = metric[condition];
        const cells = { ocelli: null, cornerstone: null };
        recordCondition(metrics, metric.id, id, condition, cells);
        const selection = inputs.selections[id];
        const missingSession = id === SESSION_WORKLOAD && metric.id !== "page_ready" && ["pseudo-full", "pseudo-ct", "pseudo-dx"].some((id) => !inputs.directories[id]);
        if (rule.state !== "yes" || (selection && selection.state !== "selected") || missingSession) {
          const reason = rule.reason || selection?.reason || (missingSession ? "the declared memory session has an unavailable workload" : "this condition is not applicable");
          for (const side of SIDES) cells[side] = unavailable(reason);
          continue;
        }
        if (["peak_memory", "session_cpu"].includes(metric.id) && process.platform !== "darwin") {
          for (const side of SIDES) cells[side] = unavailable(`no collector for ${process.platform}`);
          continue;
        }
        if (metric.id === "session_cpu") {
          for (const side of SIDES) {
            const values = memory[side].filter((session) => session.cpu.state === "measured").map((session) => session.cpu.seconds);
            cells[side] = summarise(values, metric.n);
            if (cells[side].state === "invalid") cells[side].reason = "the same memory sessions did not retain the required stable CPU inventories";
          }
          continue;
        }
        let samples = { ocelli: [], cornerstone: [] }, upper = { ocelli: [], cornerstone: [] }, trials = [];
        const retained = {};
        try {
          if (condition === "warm") for (const side of SIDES) {
            const actor = await launchSide(side, context); retained[side] = actor;
            await openFolder(actor, inputs.directories[id]);
            if (metric.id !== "window_change") await actor.page.close();
          }
          ({ samples, upper, trials } = await collectTrialSamples(metric.n, startSide, async (side, trialIndex) => {
            let actor = retained[side] || null;
            try {
              if (!actor) actor = await launchSide(side, context, metric.id !== "peak_memory");
              else if (metric.id !== "window_change") await actor.newPage();
              let value, upper;
              if (metric.id === "page_ready") value = actor.ready;
              else if (["peak_memory", "session_cpu"].includes(metric.id)) {
                const result = await memorySession(actor, inputs); raw.push({ metric: metric.id, condition, side, result });
                value = result.figures.sampled_peak; upper = result.figures.sum_of_peaks; memory[side].push(result);
              } else if (metric.id === "window_change") value = await windowStep(actor, 40 + trialIndex);
              else {
                const opened = await openFolder(actor, inputs.directories[id]);
                if (metric.id === "scroll") {
                  const result = await scroll(actor); raw.push({ metric: metric.id, condition, side, workload: id, result });
                  if (result.keys < 120 || result.blank) throw new Error("the scroll did not present its declared workload", { cause: comparisonFailure(1) });
                  value = result.frames / (result.elapsed_ms / 1000);
                } else value = opened[metric.id];
              }
              return { value, upper };
            }
            finally { if (actor && !retained[side]) await actor.close(); }
          }));
        } catch { raw.push({ metric: metric.id, workload: id, condition, error: "the warm priming trial did not complete" }); }
        finally { for (const actor of Object.values(retained)) await actor.close().catch(() => {}); }
        for (const side of SIDES) {
          cells[side] = summarise(samples[side], metric.n);
          if (cells[side].state === "invalid") cells[side].reason = "fewer valid observations than the declared trial count";
          if (metric.id === "peak_memory" && cells[side].state === "measured") cells[side].upper_bound = summarise(upper[side], metric.n);
        }
        raw.push({ metric: metric.id, workload: id, condition, start_side: startSide, samples, upper, trials });
      }
    }
  }
  return { metrics, raw };
}

/** Real browser and process work is isolated behind the same explicit phases
 * that the approved lifecycle tests inject. Injected runs cannot publish. */
async function executeComparison(options = {}, services = {}) {
  const startedAt = new Date().toISOString();
  if (options.publish && Object.keys(services).length) throw new Error("publication refuses injected comparison services", { cause: comparisonFailure(2) });
  if (options.publish && options.build === false) throw new Error("publication refuses a pre-existing comparison build", { cause: comparisonFailure(2) });
  if (options.publish && options.colourCase) throw new Error("publication refuses the supplemental colour case mode", { cause: comparisonFailure(2) });
  const environment = options.environment || process.env;
  const pseudo = options.pseudo || environment.OCELLI_PSEUDO_DIR;
  if (!pseudo) throw new Error("--compare-cornerstone needs --pseudo or OCELLI_PSEUDO_DIR", { cause: comparisonFailure(2) });
  const out = resolve(options.out || benchPath("out", "compare"));
  const colourInput = options.colourCase
    ? await (services.colourInput || readColourComparisonInput)(pseudo, options.colourCase) : null;
  // These two folder names are public generator selectors. Uncontrolled
  // names below each selected root and every header value remain denied.
  const calibrationRoot = colourInput?.calibrationDirectory || pseudo;
  if (!services.prepare) {
    const files = await listInputFiles(calibrationRoot);
    let found = false;
    for (const file of files) {
      const handle = await readFile(file);
      if (handle.length >= 132 && handle.toString("ascii", 128, 132) === "DICM") { found = true; break; }
    }
    if (!found) throw new Error("--pseudo contains no usable Part 10 image", { cause: comparisonFailure(2) });
  }
  await (services.exclusive || compareExclusive)();
  await mkdir(out, { recursive: true });
  const lock = join(out, "lock");
  try { await mkdir(lock); } catch { throw new Error("a comparison lock exists under --out, remove it only after its owner has exited", { cause: comparisonFailure(2) }); }
  let temp, context, outcome, interrupted = null;
  const onSignal = (name) => {
    interrupted ||= name;
    if (context) {
      context.interrupted = true;
      for (const actor of context.actors) actor.close().catch(() => {});
    }
  };
  const sigint = () => onSignal("SIGINT"), sigterm = () => onSignal("SIGTERM");
  const checkSignal = () => { if (context?.cleanupFailed) throw new Error("comparison cleanup did not finish", { cause: comparisonFailure(1) }); if (interrupted) throw new Error("comparison interrupted", { cause: comparisonFailure(interrupted === "SIGINT" ? 130 : 143) }); };
  process.on("SIGINT", sigint); process.on("SIGTERM", sigterm);
  try {
    // This attempt owns these ignored artifact names. A later setup failure
    // must not leave the previous run's green record beside its red exit.
    await Promise.all(["comparison.json", "comparison.txt", "colour-comparison.json", "colour-comparison.md", "colour-comparison.sha256"].map((name) => rm(join(out, name), { force: true })));
    temp = await mkdtemp(join(os.tmpdir(), "ocelli-comparison-"));
    let parser;
    if (!services.prepare) {
      try { parser = oracleRequire("dicom-parser"); } catch { throw new Error("the comparison requires the pinned tools/oracle install", { cause: comparisonFailure(2) }); }
    }
    const inputs = await (services.prepare || prepareInputs)(calibrationRoot, temp, parser, !options.colourCase);
    if (colourInput && !services.prepare) await readHeaders(await listInputFiles(colourInput.targetDirectory), "pseudo", colourInput.targetDirectory, parser, inputs.denySet);
    checkSignal();
    context = { ...(await (services.build || prepareBuild)(options, out)), headed: false, software: Boolean(options.software), actors: new Set(), interrupted: false };
    checkSignal();
    let checked = await (services.checkPixels || pixelPrecheck)(context, inputs, temp);
    checkSignal();
    if (checked.needsHeaded) {
      context.headed = true;
      checked = await (services.checkPixels || pixelPrecheck)(context, inputs, temp);
      checkSignal();
      if (checked.needsHeaded) checked = { ...checked, environmentRefusal: "software_after_headed" };
    }
    const info = checked.metadata;
    if (checked.environmentRefusal) {
      const kind = checked.environmentRefusal;
      if (!["environment_mismatch", "software_after_headed"].includes(kind)) throw new Error("the environment refusal was not recognized", { cause: comparisonFailure(2) });
      const record = {
        architecture: ARCHITECTURE, started: startedAt, state: "refused_environment",
        source: context.source || null, input_identity: inputs.identity,
        versions: { "Ocelli Chromium": info[0].version, "Ocelli renderer": info[0].renderer,
          "cornerstone3D Chromium": info[1].version, "cornerstone3D renderer": info[1].renderer },
        headed: context.headed, metrics: {},
        refusals: [RUN_REFUSALS[kind].message, "The environment check refused before pixel comparison or timings."],
      };
      const bodies = safeOutputs(record, inputs.denySet);
      await writePair(join(out, "comparison.json"), join(out, "comparison.txt"), bodies.json, bodies.text);
      return outcome = { state: record.state, record, refusal: kind };
    }
    const pins = context.pins || {};
    const record = {
      architecture: ARCHITECTURE, not_shown: NOT_SHOWN, machine: hostClass(os),
      versions: { chromium: info[0].version, playwright: pins.playwright || "injected", "@cornerstonejs/core": pins["@cornerstonejs/core"] || "injected", "@cornerstonejs/dicom-image-loader": pins["@cornerstonejs/dicom-image-loader"] || "injected" },
      browser: { renderer: info[0].renderer, headed: context.headed, software: context.software },
      defaults: { ocelli: info[0].defaults, cornerstone: info[1].defaults },
      workloads: Object.fromEntries(Object.entries(inputs.selections).map(([id, selection]) => [id, { state: selection.state, count: selection.count, ...(selection.reason ? { reason: selection.reason } : {}) }])),
      input_identity: inputs.identity, pair_verdict: checked.verdict, metrics: {},
      started: startedAt, binary_sha256: context.binary_sha256 || null,
      conditions: { load_average: os.loadavg(), free_memory_bytes: os.freemem() },
    };
    context.host = record.machine; context.startedAt = record.started; context.source ||= null;
    context.pixelPrecheck = { verdict: checked.verdict, input_identity: inputs.identity, browser: record.browser, versions: record.versions, defaults: record.defaults, not_shown: NOT_SHOWN };
    if (!checked.verdict?.passes || checked.verdict.low_information) {
      record.state = "refused_pixels";
      const bodies = safeOutputs(record, inputs.denySet);
      await writePair(join(out, "comparison.json"), join(out, "comparison.txt"), bodies.json, bodies.text);
      return outcome = { state: record.state, record };
    }
    if (colourInput) {
      const result = await (services.measureColour || measureColourComparison)({ caseId: options.colourCase, context, directory: pseudo,
        tools: { launchSide, openFolder, windowStep, measureColourTrial, measureColourMemorySession } });
      checkSignal();
      if (context.source) {
        const after = await sourceIdentity();
        if (after.head !== context.source.head || after.index_tree !== context.source.index_tree) throw new Error("the comparison source changed during the run", { cause: comparisonFailure(1) });
      }
      const bodies = safeColourOutputs(result.record, result.text, inputs.denySet);
      await writePair(join(out, "colour-comparison.json"), join(out, "colour-comparison.md"), bodies.json, bodies.text);
      await writeFile(join(out, "colour-comparison.sha256"), bodies.digest + "  colour-comparison.json\n");
      return outcome = { state: "measured", record: result.record };
    }
    const definition = loadComparison(await readFile(benchPath("comparison.json"), "utf8"));
    const measured = await (services.measure || measureAll)(context, inputs, definition);
    checkSignal();
    if (context.source) {
      const after = await sourceIdentity();
      if (after.head !== context.source.head || after.index_tree !== context.source.index_tree) throw new Error("the comparison source changed during the run", { cause: comparisonFailure(1) });
      record.source = context.source;
    }
    record.metrics = measured.metrics; record.trials = measured.raw;
    record.state = Object.values(record.metrics).some((workloads) => Object.values(workloads).some((sides) => Object.values(sides).some((cell) => cell.state === "invalid"))) ? "invalid" : "measured";
    const bodies = safeOutputs(record, inputs.denySet);
    await writePair(join(out, "comparison.json"), join(out, "comparison.txt"), bodies.json, bodies.text);
    let pendingPublication;
    if (options.publish) {
      if (record.state !== "measured") return outcome = { state: record.state, record, refusal: "invalid_publication" };
      const published = publishedRecord(record), json = JSON.stringify(published, null, 2) + "\n", text = renderSummary(published);
      if (scanRecordForDenied(published, inputs.denySet).length || scanForDenied(json, inputs.denySet).length || scanForDenied(text, inputs.denySet).length) throw new Error("comparison publication contains a denied input value", { cause: comparisonFailure(2) });
      pendingPublication = { json, text };
    }
    return outcome = { state: record.state, record, ...(pendingPublication ? { pendingPublication } : {}) };
  } catch (error) {
    if (error.cause?.marker === COMPARISON_FAILURE) {
      error.safeMessage = error.message; error.exitCode = error.cause.exitCode;
      throw error;
    }
    const safe = new Error("comparison could not complete", { cause: comparisonFailure(1) });
    safe.safeMessage = safe.message; safe.exitCode = 1; throw safe;
  } finally {
    const cleanup = await Promise.allSettled([
      ...(context?.actors ? [...context.actors].map((actor) => Promise.resolve().then(() => actor.close())) : []),
      ...(context?.close ? [Promise.resolve().then(() => context.close())] : []),
      ...(temp ? [rm(temp, { recursive: true, force: true })] : []),
      rm(lock, { recursive: true, force: true }),
    ]);
    if (outcome && interrupted) outcome.interrupted = interrupted;
    if (!outcome || interrupted || context?.cleanupFailed || cleanup.some((result) => result.status === "rejected")) {
      if (outcome) outcome.cleanupFailed = true;
      await Promise.allSettled(["comparison.json", "comparison.txt", "colour-comparison.json", "colour-comparison.md", "colour-comparison.sha256"].map((name) => rm(join(out, name), { force: true })));
    }
    process.removeListener("SIGINT", sigint); process.removeListener("SIGTERM", sigterm);
  }
}

/** The real tracked replacement occurs only after owned cleanup succeeds.
 * Destination injection is for filesystem unit controls, not CLI acceptance. */
export async function publishComparison(result, destination = repoPath("docs", "benchmarks")) {
  if (result?.cleanupFailed || result?.state !== "measured") throw new Error("publication requires successful measured cleanup", { cause: comparisonFailure(1) });
  const pending = result.pendingPublication;
  if (!pending || Object.keys(pending).sort().join(",") !== "json,text" ||
      typeof pending.json !== "string" || !pending.json.endsWith("\n") ||
      typeof pending.text !== "string" || !pending.text) throw new Error("publication has no validated prepared pair", { cause: comparisonFailure(2) });
  await mkdir(destination, { recursive: true });
  await writePair(join(destination, "cornerstone-comparison.json"), join(destination, "cornerstone-comparison.md"), pending.json, pending.text);
}

/** Only locally constructed fixed refusal messages cross the CLI boundary.
 * Foreign I/O or browser errors may contain input paths or header values. */
export async function runComparison(options = {}, services = {}) {
  try {
    const result = await executeComparison(options, services);
    if (result.interrupted) throw new Error("comparison interrupted", { cause: comparisonFailure(result.interrupted === "SIGINT" ? 130 : 143) });
    if (result.cleanupFailed) throw new Error("comparison cleanup did not finish", { cause: comparisonFailure(1) });
    if (result.refusal) {
      if (!Object.hasOwn(RUN_REFUSALS, result.refusal)) throw new Error("the comparison refusal was not recognized", { cause: comparisonFailure(2) });
      const refusal = RUN_REFUSALS[result.refusal];
      throw new Error(refusal.message, { cause: comparisonFailure(refusal.exitCode) });
    }
    if (result.pendingPublication) await publishComparison(result);
    return result;
  }
  catch (error) {
    if (error.cause?.marker === COMPARISON_FAILURE) {
      error.safeMessage = error.message; error.exitCode = error.cause.exitCode;
      throw error;
    }
    const safe = new Error("comparison could not complete", { cause: comparisonFailure(1) });
    safe.safeMessage = safe.message; safe.exitCode = 1; throw safe;
  }
}
