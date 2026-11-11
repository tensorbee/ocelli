// Shared work for the folder viewer subjects.
//
// Playwright is imported inside the functions that launch a browser. A static
// import here would keep the pure checks in viewer_work_test.mjs from loading
// without a browser install.

import { spawn, execFile } from "node:child_process";
import { Buffer } from "node:buffer";
import { mkdir, readFile, stat, writeFile } from "node:fs/promises";
import { join } from "node:path";

import { repoPath } from "./paths.mjs";

export const CT_COUNT = 1693;
export const CT_EDGE = 512;
export const DX_ROWS = 3000;
export const DX_COLUMNS = 2900;
export const STORED = 50;
// F-227's informational workload, the operator's PT shape: 267 files of 128
// by 128, signed, no window, a Rescale Slope that changes per file, and
// stored 0 outside a disc.
export const PET_COUNT = 267;
export const PET_EDGE = 128;
export const PET_RADIUS = 50;
const MIB = 1024 * 1024;
const CT_SOP = "1.2.840.10008.5.1.4.1.1.2";
const DX_SOP = "1.2.840.10008.5.1.4.1.1.1.1";
const PET_SOP = "1.2.840.10008.5.1.4.1.1.128";

const binaryPromises = new Map();

/** Middle value. An even count uses the mean of the two middle values. */
export function median(values) {
  if (!Array.isArray(values) || values.length === 0) {
    throw new Error("median needs at least one value");
  }
  const sorted = [...values].sort((left, right) => left - right);
  const mid = Math.floor(sorted.length / 2);
  if (sorted.length % 2 === 1) return sorted[mid];
  return (sorted[mid - 1] + sorted[mid]) / 2;
}

/**
 * One grey byte replicated into a little-endian RGBA word.
 * The page uses the same 256-entry table. This is channel copy, not a window.
 */
export function greyWord(value) {
  return (0xff000000 | (value << 16) | (value << 8) | value) >>> 0;
}

export function assertPageHooks(requestGreyFn, drawGreyFn) {
  const request = requestGreyFn === undefined ? globalThis.requestGrey : requestGreyFn;
  const draw = drawGreyFn === undefined ? globalThis.drawGrey : drawGreyFn;
  if (typeof request !== "function" || typeof draw !== "function") {
    throw new Error("the page did not expose requestGrey and drawGrey");
  }
}

export function assertScrollStage(blanks, faults) {
  if (faults > 0) throw new Error("the stage showed a fault during the scroll");
  if (blanks > 0) throw new Error("the stage was blank during the scroll");
}

export function assertScrollDelivery(keys) {
  if (keys < 120) {
    throw new Error(`scroll delivered ${keys} keys, which is fewer than 120`);
  }
}

export function assertForwardCounters(counters) {
  let previous = -1;
  for (const text of counters) {
    const match = /^(\d+)/.exec(text || "");
    const value = match ? Number(match[1]) : null;
    if (value == null) continue;
    if (value < previous) {
      throw new Error("the counter went backwards during a forward scroll");
    }
    previous = value;
  }
}

export function assertWindowSample(recorded) {
  if (recorded == null) throw new Error("a window step did not reach drawGrey");
}

export function assertCanvasCheck(result) {
  if (result.error) throw new Error(`the canvas check failed: ${result.error}`);
  if (result.points !== 16) throw new Error(`canvas check sampled ${result.points} points`);
  if (result.mismatches && result.mismatches.length > 0) {
    const first = result.mismatches[0];
    throw new Error(`canvas pixel ${first.x},${first.y} was not the server grey`);
  }
}

export function assertRestartRecovery(fault, reposts) {
  if (fault && fault.includes("no longer held")) {
    throw new Error("a restart left the gone sentence on the page");
  }
  if (reposts < 1) throw new Error("the restart did not re-post the file");
}

export function assertNoPreviewBody(count) {
  if (count !== 0) {
    throw new Error(`/preview carried a body ${count} times while the file was held`);
  }
}

/** `P`, the folder viewer's decode permits and its series window rounds. */
export const SERVER_PERMITS = 2;

/** F-227's series window pool, at most 2^20 f32 values reserved per round. */
export const SERIES_POOL_BYTES = 4 * MIB;

/**
 * RSS_peak <= B + 4 * S_max + P * (2 * S_max + 5 * N_max) + P * 4 MiB + 24 MiB.
 * The terms are the design's, stated before the measurement.
 * An upload is held only while it is read and decoded, and those bytes
 * are the upload term 4 * S_max. A decode copy is in flight until the
 * frame is in the cache. F-227 adds the named pool term P * 4 MiB: at most
 * P series window rounds run at once, each reserving one pool. The
 * operator kept F-216's total of 528,194,192 bytes at its named budget, so
 * the fixed constant for the binary, thread stacks and allocator slack drops
 * from 32 MiB to 24 MiB to absorb the pool term.
 */
export function serverRssBound(budget, sMax, nMax, permits = SERVER_PERMITS) {
  return budget + 4 * sMax + permits * (2 * sMax + 5 * nMax) + permits * SERIES_POOL_BYTES + 24 * MIB;
}

/** Two retained-heap readings within this gap count as the same reading. */
export const HEAP_AGREE_BYTES = 64 * 1024;

/** Collections after which a heap reading that still moves is an invalid run. */
export const HEAP_READ_CAP = 8;

/**
 * First retained-heap pair whose absolute gap is within `tolerance`.
 * `collections` is the 1-based index of the later reading.
 * When no pair inside `cap` agrees, the run is invalid.
 */
export function foldHeapReadings(samples, tolerance, cap) {
  const limit = Math.min(Array.isArray(samples) ? samples.length : 0, cap);
  for (let index = 1; index < limit; index += 1) {
    const previous = samples[index - 1];
    const current = samples[index];
    const gap = previous > current ? previous - current : current - previous;
    if (gap <= tolerance) {
      return { ok: true, retained: current, collections: index + 1 };
    }
  }
  return { ok: false, retained: null, collections: limit };
}

/** JS heap <= 64 MiB grey cache + 4 * W * H per canvas + 16 MiB. */
export function pageHeapBound(canvases, cacheBytes = 64 * MIB) {
  let bytes = cacheBytes + 16 * MIB;
  for (const [width, height] of canvases) bytes += 4 * width * height;
  return bytes;
}

function u16(value) {
  const bytes = Buffer.alloc(2);
  bytes.writeUInt16LE(value);
  return bytes;
}

function u32(value) {
  const bytes = Buffer.alloc(4);
  bytes.writeUInt32LE(value);
  return bytes;
}

function textPad(text) {
  const bytes = Buffer.from(text, "latin1");
  if (bytes.length % 2 === 0) return bytes;
  return Buffer.concat([bytes, Buffer.from(" ")]);
}

function ui(text) {
  const bytes = Buffer.from(text, "latin1");
  if (bytes.length % 2 === 0) return bytes;
  return Buffer.concat([bytes, Buffer.from([0])]);
}

export function shortEl(group, element, vr, value) {
  if (value.length % 2 !== 0 || value.length > 0xffff) {
    throw new Error("a short element has a bad length");
  }
  return Buffer.concat([
    u16(group),
    u16(element),
    Buffer.from(vr, "ascii"),
    u16(value.length),
    value,
  ]);
}

export function longEl(group, element, vr, value) {
  if (value.length % 2 !== 0) throw new Error("a long element has a bad length");
  return Buffer.concat([
    u16(group),
    u16(element),
    Buffer.from(vr, "ascii"),
    Buffer.from([0, 0]),
    u32(value.length),
    value,
  ]);
}

export function filledPixels(count, stored) {
  const sample = Buffer.alloc(2);
  sample.writeUInt16LE(stored);
  const chunk = Buffer.alloc(8192);
  for (let offset = 0; offset < chunk.length; offset += 2) sample.copy(chunk, offset);
  const bytes = Buffer.alloc(count * 2);
  for (let offset = 0; offset < bytes.length; offset += chunk.length) {
    chunk.copy(bytes, offset, 0, Math.min(chunk.length, bytes.length - offset));
  }
  return bytes;
}

/**
 * Explicit VR little endian Part 10, one mono16 frame. By default unsigned,
 * slope 1 and Window Center 40, Width 400 LINEAR. `window: null` writes no
 * window, `signed` sets Pixel Representation 1.
 */
export function mono16Part10({
  rows,
  columns,
  sopClass,
  sopUid,
  seriesUid,
  modality,
  instanceNumber,
  pixels,
  window = ["40", "400"],
  signed = false,
  slope = "1",
}) {
  const metaBody = Buffer.concat([
    longEl(0x0002, 0x0001, "OB", Buffer.from([0, 1])),
    shortEl(0x0002, 0x0002, "UI", ui(sopClass)),
    shortEl(0x0002, 0x0003, "UI", ui(sopUid)),
    shortEl(0x0002, 0x0010, "UI", ui("1.2.840.10008.1.2.1")),
    shortEl(0x0002, 0x0012, "UI", ui("2.25.216")),
  ]);
  const dataset = Buffer.concat([
    shortEl(0x0008, 0x0016, "UI", ui(sopClass)),
    shortEl(0x0008, 0x0018, "UI", ui(sopUid)),
    shortEl(0x0008, 0x0060, "CS", textPad(modality)),
    shortEl(0x0010, 0x0010, "PN", textPad("SYNTHETIC")),
    shortEl(0x0020, 0x000e, "UI", ui(seriesUid)),
    shortEl(0x0020, 0x0011, "IS", textPad("1")),
    shortEl(0x0020, 0x0013, "IS", textPad(instanceNumber)),
    shortEl(0x0028, 0x0002, "US", u16(1)),
    shortEl(0x0028, 0x0004, "CS", textPad("MONOCHROME2")),
    shortEl(0x0028, 0x0010, "US", u16(rows)),
    shortEl(0x0028, 0x0011, "US", u16(columns)),
    shortEl(0x0028, 0x0100, "US", u16(16)),
    shortEl(0x0028, 0x0101, "US", u16(16)),
    shortEl(0x0028, 0x0102, "US", u16(15)),
    shortEl(0x0028, 0x0103, "US", u16(signed ? 1 : 0)),
    ...(window ? [
      shortEl(0x0028, 0x1050, "DS", textPad(window[0])),
      shortEl(0x0028, 0x1051, "DS", textPad(window[1])),
    ] : []),
    shortEl(0x0028, 0x1052, "DS", textPad("0")),
    shortEl(0x0028, 0x1053, "DS", textPad(slope)),
    ...(window ? [shortEl(0x0028, 0x1056, "CS", textPad("LINEAR"))] : []),
    longEl(0x7fe0, 0x0010, "OW", pixels),
  ]);
  return Buffer.concat([
    Buffer.alloc(128),
    Buffer.from("DICM"),
    shortEl(0x0002, 0x0000, "UL", u32(metaBody.length)),
    metaBody,
    dataset,
  ]);
}

/** Cargo args for a release viewer. `build: false` stays on release and adds `--offline`. */
export function ensureReleaseBinaryArgs(build) {
  const args = ["build", "--release", "-p", "ocelli-folder-viewer", "--message-format=json"];
  if (build === false) args.push("--offline");
  return args;
}

/** Cargo args for `release` or `debug`. The profile is not implied by `build`. */
export function ensureViewerBinaryArgs(profile, build) {
  if (profile === "debug") {
    const args = ["build", "-p", "ocelli-folder-viewer", "--message-format=json"];
    if (build === false) args.push("--offline");
    return args;
  }
  return ensureReleaseBinaryArgs(build);
}

function chosenProfile(profile) {
  return profile === "debug" ? "debug" : "release";
}

export function ensureReleaseBinary(build) {
  return ensureViewerBinary("release", build);
}

export function ensureViewerBinary(profile, build, runCargo) {
  const chosen = chosenProfile(profile);
  const args = ensureViewerBinaryArgs(chosen, build);
  if (typeof runCargo === "function") return Promise.resolve(runCargo(args));
  const key = `${chosen}:${build === false ? "offline" : "build"}`;
  if (!binaryPromises.has(key)) {
    binaryPromises.set(key, locateBinary(args));
  }
  return binaryPromises.get(key);
}

/** Build name from a `/held` report. Only `release` and `debug` are accepted. */
export function profileFromHeld(report) {
  const build = report && report.build;
  if (build !== "release" && build !== "debug") {
    throw new Error("the viewer did not report its build");
  }
  return build;
}

/** Fetch `/held` and return the build the process reports. */
export async function reportedBuild(url) {
  const response = await fetch(`${url}held`);
  if (!response.ok) throw new Error("the viewer did not report its build");
  return profileFromHeld(await response.json());
}

function locateBinary(args) {
  return new Promise((resolve, reject) => {
    const child = spawn("cargo", args, {
      cwd: repoPath(),
      stdio: ["ignore", "pipe", "pipe"],
    });
    let stdout = "";
    let stderr = "";
    child.stdout.on("data", (chunk) => {
      stdout += chunk.toString("utf8");
    });
    child.stderr.on("data", (chunk) => {
      stderr += chunk.toString("utf8");
    });
    child.on("error", reject);
    child.on("exit", (code) => {
      if (code !== 0) {
        reject(new Error(`the viewer build failed (${code}): ${stderr.slice(-2000)}`));
        return;
      }
      let executable = null;
      for (const line of stdout.split("\n")) {
        if (!line.startsWith("{")) continue;
        const message = JSON.parse(line);
        if (message.reason === "compiler-artifact" && message.executable) {
          executable = message.executable;
        }
      }
      if (!executable) {
        reject(new Error("the viewer build did not name a binary"));
        return;
      }
      resolve(executable);
    });
  });
}

export function startViewer(binary, env = {}) {
  return new Promise((resolve, reject) => {
    const child = spawn(binary, [], {
      env: { ...process.env, OCELLI_PORT: "0", ...env },
      stdio: ["ignore", "pipe", "pipe"],
    });
    let buffer = "";
    let settled = false;
    const timer = setTimeout(() => {
      if (settled) return;
      settled = true;
      child.kill("SIGTERM");
      reject(new Error(`the viewer did not print a URL: ${buffer}`));
    }, 20000);
    child.stdout.on("data", (chunk) => {
      buffer += chunk.toString("utf8");
      const found = buffer.match(/http:\/\/127\.0\.0\.1:(\d+)\//);
      if (!found || settled) return;
      const port = Number(found[1]);
      if (port === 8742 || port === 8765) {
        settled = true;
        clearTimeout(timer);
        child.kill("SIGTERM");
        reject(new Error("the viewer bound a reserved port"));
        return;
      }
      settled = true;
      clearTimeout(timer);
      resolve({ child, url: found[0], port, pid: child.pid });
    });
    child.on("exit", (code) => {
      if (settled) return;
      settled = true;
      clearTimeout(timer);
      reject(new Error(`the viewer exited ${code} before printing a URL`));
    });
  });
}

export function stopViewer(child) {
  if (!child || child.exitCode !== null || child.signalCode) return Promise.resolve();
  return new Promise((resolve) => {
    const timer = setTimeout(() => {
      child.kill("SIGKILL");
      resolve();
    }, 2000);
    child.once("exit", () => {
      clearTimeout(timer);
      resolve();
    });
    child.kill("SIGTERM");
  });
}

function rssKb(pid) {
  return new Promise((resolve) => {
    execFile("ps", ["-o", "rss=", "-p", String(pid)], (error, stdout) => {
      if (error) {
        resolve(null);
        return;
      }
      const value = Number(String(stdout).trim());
      resolve(Number.isFinite(value) ? value : null);
    });
  });
}

async function ensureSeries(dir, spec) {
  await mkdir(dir, { recursive: true });
  const stampPath = join(dir, "stamp.json");
  const stamp = JSON.stringify(spec);
  try {
    if ((await readFile(stampPath, "utf8")) === stamp) return;
  } catch {
    // The directory is new, or an older writer left no stamp.
  }
  const pixels = filledPixels(spec.rows * spec.columns, spec.stored);
  for (let index = 0; index < spec.count; index += 1) {
    const name = `${spec.prefix}-${String(index).padStart(4, "0")}.dcm`;
    const bytes = mono16Part10({
      rows: spec.rows,
      columns: spec.columns,
      sopClass: spec.sopClass,
      sopUid: `2.25.216${spec.uid}${index}`,
      seriesUid: `2.25.216${spec.uid}0001`,
      modality: spec.modality,
      instanceNumber: String(index + 1),
      pixels,
    });
    await writeFile(join(dir, name), bytes);
  }
  await writeFile(stampPath, stamp);
}

/** The Rescale Slope of PT file `index`, exact in binary. */
export function petSlope(index) {
  return 0.5 + 0.125 * (index % 7);
}

/** Stored samples of PT file `index`: 0 outside the disc, 1 to 900 inside. */
export function petPixels(index) {
  const bytes = Buffer.alloc(PET_EDGE * PET_EDGE * 2);
  const middle = PET_EDGE / 2;
  for (let y = 0; y < PET_EDGE; y += 1) {
    for (let x = 0; x < PET_EDGE; x += 1) {
      const dx = x - middle;
      const dy = y - middle;
      const inside = dx * dx + dy * dy <= PET_RADIUS * PET_RADIUS;
      bytes.writeInt16LE(inside ? 1 + ((3 * x + 5 * y + index) % 900) : 0, (y * PET_EDGE + x) * 2);
    }
  }
  return bytes;
}

/** PT file `index` of the `pet_no_window` workload. */
export function petPart10(index) {
  return mono16Part10({
    rows: PET_EDGE,
    columns: PET_EDGE,
    sopClass: PET_SOP,
    sopUid: `2.25.2273${index}`,
    seriesUid: "2.25.22730001",
    modality: "PT",
    instanceNumber: String(index + 1),
    pixels: petPixels(index),
    window: null,
    signed: true,
    slope: String(petSlope(index)),
  });
}

async function ensurePetSeries(dir) {
  await mkdir(dir, { recursive: true });
  const stampPath = join(dir, "stamp.json");
  const stamp = JSON.stringify({ workload: "pet_no_window", count: PET_COUNT, edge: PET_EDGE, radius: PET_RADIUS, writer: 1 });
  try {
    if ((await readFile(stampPath, "utf8")) === stamp) return;
  } catch {
    // The directory is new, or an older writer left no stamp.
  }
  for (let index = 0; index < PET_COUNT; index += 1) {
    await writeFile(join(dir, `pt-${String(index).padStart(4, "0")}.dcm`), petPart10(index));
  }
  await writeFile(stampPath, stamp);
}

export async function ensureWorkloads(outDir) {
  const ctDir = join(outDir, "viewer-ct");
  const dxDir = join(outDir, "viewer-dx");
  await ensureSeries(ctDir, {
    count: CT_COUNT,
    rows: CT_EDGE,
    columns: CT_EDGE,
    stored: STORED,
    prefix: "ct",
    uid: "1",
    sopClass: CT_SOP,
    modality: "CT",
  });
  await ensureSeries(dxDir, {
    count: 1,
    rows: DX_ROWS,
    columns: DX_COLUMNS,
    stored: STORED,
    prefix: "dx",
    uid: "2",
    sopClass: DX_SOP,
    modality: "DX",
  });
  const petDir = join(outDir, "viewer-pet-no-window");
  await ensurePetSeries(petDir);
  return { ctDir, dxDir, petDir };
}

async function launchBrowser() {
  const { chromium } = await import("playwright");
  const browser = await chromium.launch({
    headless: true,
    args: ["--enable-precise-memory-info", "--js-flags=--expose-gc"],
  });
  return { browser, chromium: browser.version() };
}

async function openPage(browser, url) {
  const page = await browser.newPage({ viewport: { width: 1600, height: 1000 } });
  page.setDefaultTimeout(180000);
  await page.addInitScript(() => {
    const original = CanvasRenderingContext2D.prototype.putImageData;
    CanvasRenderingContext2D.prototype.putImageData = function patched(...args) {
      const result = original.apply(this, args);
      const canvas = this.canvas;
      // F-208. The image is the active viewport's `canvas.view`, and these
      // runs keep the 1 by 1 layout, so there is one.
      if (!canvas || !canvas.classList.contains("view")) return result;
      queueMicrotask(() => {
        const stage = document.querySelector("#wrap .cell.active .stage");
        const counter = document.getElementById("counter");
        const draws = window.__draws || (window.__draws = []);
        const at = performance.now();
        draws.push({
          hidden: Boolean(stage && stage.hidden),
          counter: counter ? counter.textContent : "",
          at,
        });
        if (window.__listAt != null && window.__firstDrawAt == null) window.__firstDrawAt = at;
      });
      return result;
    };
  });
  await page.goto(url);
  await page.evaluate(assertPageHooks);
  await page.evaluate(() => {
    const original = window.fetch.bind(window);
    window.__previewBodySizes = [];
    window.__fileBodySizes = [];
    window.fetch = function watched(input, init) {
      const url = typeof input === "string" ? input : (input && input.url) || "";
      const body = init && init.body;
      let size = 0;
      if (body == null) size = 0;
      else if (typeof body.byteLength === "number") size = body.byteLength;
      else if (typeof body.size === "number") size = body.size;
      else size = -1;
      if (url.endsWith("/preview")) window.__previewBodySizes.push(size);
      if (url.endsWith("/file")) window.__fileBodySizes.push(size);
      return original(input, init);
    };
    const status = document.getElementById("status");
    const observer = new MutationObserver(() => {
      if (window.__listAt == null && /\d+ series\./.test(status.textContent || "")) {
        window.__listAt = performance.now();
      }
    });
    observer.observe(status, { childList: true, subtree: true, characterData: true });
    const requestOriginal = requestGrey;
    const drawOriginal = drawGrey;
    window.requestGrey = function timedRequest(...args) {
      const started = performance.now();
      const pending = requestOriginal.apply(this, args);
      if (window.__armWindow === true) window.__windowStarted = started;
      return pending;
    };
    window.drawGrey = function timedDraw(...args) {
      const result = drawOriginal.apply(this, args);
      if (window.__windowStarted != null) {
        const steps = window.__windowSteps || (window.__windowSteps = []);
        steps.push(performance.now() - window.__windowStarted);
        window.__windowStarted = null;
        window.__armWindow = false;
      }
      return result;
    };
  });
  return page;
}

function watchPreviewBodies(page) {
  const lengths = [];
  page.on("request", (request) => {
    if (!request.url().includes("/preview")) return;
    const body = request.postDataBuffer();
    lengths.push(body ? body.length : 0);
  });
  return lengths;
}

async function openFolder(page, dir) {
  await page.evaluate(() => {
    window.__listAt = null;
    window.__firstDrawAt = null;
    window.__draws = [];
  });
  await page.locator("#folder").setInputFiles(dir);
  await page.waitForFunction(() => window.__listAt != null);
  await page.waitForFunction(() => window.__firstDrawAt != null);
  const times = await page.evaluate(() => ({
    listAt: window.__listAt,
    firstDrawAt: window.__firstDrawAt,
  }));
  return times.firstDrawAt - times.listAt;
}

async function scrollOnce(page) {
  const result = await page.evaluate(() => new Promise((resolve) => {
    const existing = window.__draws || [];
    const last = existing.length ? existing[existing.length - 1] : null;
    const baseline = last ? last.counter : "";
    window.__draws = [];
    let started = null;
    let keys = 0;
    let presented = 0;
    let previous = null;
    const samples = [];
    const indexOf = (text) => {
      const match = /^(\d+)/.exec(text || "");
      return match ? Number(match[1]) : null;
    };
    const drawnIndex = () => {
      const draws = window.__draws || [];
      for (let cursor = draws.length - 1; cursor >= 0; cursor -= 1) {
        const value = indexOf(draws[cursor].counter);
        if (value != null) return value;
      }
      return indexOf(baseline);
    };
    // A presented frame is a drawn image index that changed since the
    // previous animation callback. The first callback only records the index.
    // Visibility is sampled on this callback, including the frames between draws.
    const observe = () => {
      const stage = document.querySelector("#wrap .cell.active .stage");
      const fault = document.querySelector("#wrap .cell.active .fault");
      const faultText = fault && !fault.hidden ? (fault.textContent || "").trim() : "";
      const hidden = !stage || stage.hidden;
      samples.push({ hidden, fault: faultText });
      const index = drawnIndex();
      if (index != null && previous != null && index !== previous) presented += 1;
      if (index != null) previous = index;
    };
    const finish = () => {
      setTimeout(() => {
        observe();
        const draws = window.__draws || [];
        resolve({
          keys,
          presented,
          blanks: samples.filter((sample) => sample.hidden || sample.fault).length,
          faults: samples.filter((sample) => sample.fault).map((sample) => sample.fault),
          counters: draws.map((draw) => draw.counter),
          seconds: (performance.now() - started) / 1000,
        });
      }, 300);
    };
    const frame = () => {
      const now = performance.now();
      if (started == null) started = now;
      observe();
      if (now - started >= 3000) {
        finish();
        return;
      }
      window.dispatchEvent(new KeyboardEvent("keydown", { key: "ArrowDown", bubbles: true }));
      keys += 1;
      window.requestAnimationFrame(frame);
    };
    window.requestAnimationFrame(frame);
  }));
  assertScrollDelivery(result.keys);
  assertForwardCounters(result.counters);
  assertScrollStage(result.blanks, result.faults ? result.faults.length : 0);
  return {
    keys: result.keys,
    presented: result.presented,
    blanks: result.blanks,
    framesPerSecond: result.presented / result.seconds,
  };
}

async function plainTimerCeiling(page) {
  // Same 3 s window and 300 ms settle as the scroll. The callback does no work.
  return page.evaluate(() => new Promise((resolve) => {
    const started = performance.now();
    let dispatches = 0;
    const send = () => {
      if (performance.now() - started >= 3000) {
        setTimeout(() => {
          const seconds = (performance.now() - started) / 1000;
          resolve({
            dispatches,
            seconds,
            perSecond: dispatches / seconds,
          });
        }, 300);
        return;
      }
      dispatches += 1;
      setTimeout(send, 16);
    };
    send();
  }));
}

async function windowSteps(page, count) {
  const samples = [];
  for (let step = 0; step < count; step += 1) {
    await page.evaluate((center) => {
      window.__armWindow = true;
      const viewport = activeViewport();
      viewport.overrideWindow = { center, width: 400, fn: "LINEAR" };
      viewport.activePreset = null;
      return showIndex(viewport, viewport.viewIndex);
    }, 40 + step);
    await page.waitForFunction(() => window.__armWindow !== true);
    const recorded = await page.evaluate(() => {
      const steps = window.__windowSteps || [];
      return steps.length ? steps[steps.length - 1] : null;
    });
    assertWindowSample(recorded);
    samples.push(recorded);
  }
  return samples;
}

async function canvasMatchesReply(page) {
  // The timed steps have already received their replies. A listener started
  // here would miss them. One more id-form preview is the reply the canvas
  // is checked against, and it is not part of the timed samples.
  const result = await page.evaluate(async () => {
    const viewport = activeViewport();
    const image = currentImage(viewport);
    const chosen = viewport.overrideWindow;
    const headers = { "X-Ocelli-Frame": String(image.frame) };
    if (chosen) {
      headers["X-Ocelli-Window-Center"] = String(chosen.center);
      headers["X-Ocelli-Window-Width"] = String(chosen.width);
      headers["X-Ocelli-Window-Function"] = chosen.fn;
    }
    const response = await postPreview(image, headers, null);
    if (!response.ok) return { error: `preview status ${response.status}` };
    const grey = new Uint8Array(await response.arrayBuffer());
    const canvas = document.querySelector("#wrap .cell.active canvas.view");
    const context = canvas.getContext("2d");
    const width = canvas.width;
    const height = canvas.height;
    if (grey.length !== width * height) {
      return { error: `reply ${grey.length} did not match canvas ${width} by ${height}` };
    }
    const xs = [0, Math.floor(width / 3), Math.floor((2 * width) / 3), Math.max(0, width - 1)];
    const ys = [0, Math.floor(height / 3), Math.floor((2 * height) / 3), Math.max(0, height - 1)];
    const mismatches = [];
    for (const y of ys) {
      for (const x of xs) {
        const pixel = context.getImageData(x, y, 1, 1).data;
        const expected = grey[y * width + x];
        if (pixel[0] !== expected || pixel[1] !== expected || pixel[2] !== expected || pixel[3] !== 255) {
          mismatches.push({ x, y, expected, r: pixel[0], g: pixel[1], b: pixel[2], a: pixel[3] });
        }
      }
    }
    return { points: xs.length * ys.length, mismatches };
  });
  assertCanvasCheck(result);
}

async function restartAndDraw(page, binary, server) {
  const before = await page.evaluate(() => (window.__draws || []).length);
  const port = server.port;
  await stopViewer(server.child);
  const next = await startViewer(binary, { OCELLI_PORT: String(port) });
  server.child = next.child;
  server.port = next.port;
  server.pid = next.pid;
  server.url = next.url;
  await page.evaluate(() => {
    cache.clear();
    cacheBytes = 0;
    window.__filePostsAtRestart = (window.__fileBodySizes || []).length;
  });
  await page.keyboard.press("ArrowDown");
  await page.waitForFunction(
    (seen) => (window.__draws || []).length > seen,
    before,
    { timeout: 30000 },
  );
  // F-208 gave each viewport cell its own `.fault`, and the page has no
  // `#fault`. The active cell's is read as the scroll sampler reads it, and
  // a hidden one is no fault.
  const fault = await page.evaluate(() => {
    const node = document.querySelector("#wrap .cell.active .fault");
    return node && !node.hidden ? (node.textContent || "").trim() : "";
  });
  const reposts = await page.evaluate(
    () => (window.__fileBodySizes || []).length - (window.__filePostsAtRestart || 0),
  );
  assertRestartRecovery(fault, reposts);
}

function instrumentOf(chromium) {
  return { chromium };
}

export async function measureOpen({ outDir, build, profile }) {
  const chosen = chosenProfile(profile);
  const binary = await ensureViewerBinary(chosen, build);
  const { ctDir, petDir } = await ensureWorkloads(outDir);
  const server = await startViewer(binary);
  const reported = await reportedBuild(server.url);
  const { browser, chromium } = await launchBrowser();
  const samples = [];
  const pet = [];
  const openOnce = async (dir) => {
    const page = await openPage(browser, server.url);
    const started = Date.now();
    await page.locator("#folder").setInputFiles(dir);
    await page.waitForFunction(() => /\d+ series\./.test(document.querySelector("#status")?.textContent || ""));
    const took = Date.now() - started;
    await page.close();
    return took;
  };
  try {
    for (let index = 0; index < 9; index += 1) samples.push(await openOnce(ctDir));
    // F-227, informational. No threshold.
    for (let index = 0; index < 9; index += 1) pet.push(await openOnce(petDir));
  } finally {
    await browser.close();
    await stopViewer(server.child);
  }
  const value = median(samples);
  return {
    value,
    detail: {
      profile: reported,
      samples_ms: samples,
      threshold_ms: 1100,
      thresholds_met: value <= 1100,
      pet_no_window_ms: pet,
      pet_no_window_median_ms: median(pet),
    },
    instrument: instrumentOf(chromium),
  };
}

export async function measureFirstImage({ outDir, build, profile }) {
  const chosen = chosenProfile(profile);
  const binary = await ensureViewerBinary(chosen, build);
  const dirs = await ensureWorkloads(outDir);
  const server = await startViewer(binary);
  const reported = await reportedBuild(server.url);
  const { browser, chromium } = await launchBrowser();
  const ct = [];
  const dx = [];
  const pet = [];
  try {
    for (let index = 0; index < 9; index += 1) {
      const page = await openPage(browser, server.url);
      ct.push(await openFolder(page, dirs.ctDir));
      await page.close();
    }
    for (let index = 0; index < 9; index += 1) {
      const page = await openPage(browser, server.url);
      dx.push(await openFolder(page, dirs.dxDir));
      await page.close();
    }
    // F-227, informational. The first image of a series that stores no
    // window waits for its /series-window reply, so this includes it.
    for (let index = 0; index < 9; index += 1) {
      const page = await openPage(browser, server.url);
      pet.push(await openFolder(page, dirs.petDir));
      await page.close();
    }
    const page = await openPage(browser, server.url);
    await openFolder(page, dirs.ctDir);
    await restartAndDraw(page, binary, server);
    await page.close();
  } finally {
    await browser.close();
    await stopViewer(server.child);
  }
  const value = median(ct);
  const dxValue = median(dx);
  return {
    value,
    detail: {
      profile: reported,
      ct_ms: ct,
      dx_ms: dx,
      dx_median_ms: dxValue,
      ct_threshold_ms: 20,
      dx_threshold_ms: 250,
      pet_no_window_ms: pet,
      pet_no_window_median_ms: median(pet),
      restart_recovered: true,
      thresholds_met: value <= 20 && dxValue <= 250,
    },
    instrument: instrumentOf(chromium),
  };
}

async function oneScroll(binary, ctDir) {
  const server = await startViewer(binary);
  const { browser, chromium } = await launchBrowser();
  try {
    const page = await openPage(browser, server.url);
    await openFolder(page, ctDir);
    const sample = await scrollOnce(page);
    await page.close();
    return { sample, chromium };
  } finally {
    await browser.close();
    await stopViewer(server.child);
  }
}

export async function measureScroll({ outDir, build, profile }) {
  const chosen = chosenProfile(profile);
  const releaseRun = chosen !== "debug";
  const binary = await ensureViewerBinary(chosen, build);
  const { ctDir } = await ensureWorkloads(outDir);
  const server = await startViewer(binary);
  const reported = await reportedBuild(server.url);
  const { browser, chromium } = await launchBrowser();
  const rates = [];
  let blanks = 0;
  let keys = 0;
  let ceiling = null;
  try {
    const ceilingPage = await browser.newPage();
    ceiling = await plainTimerCeiling(ceilingPage);
    await ceilingPage.close();
    const runs = releaseRun ? 5 : 1;
    for (let index = 0; index < runs; index += 1) {
      const page = await openPage(browser, server.url);
      await openFolder(page, ctDir);
      const sample = await scrollOnce(page);
      rates.push(sample.framesPerSecond);
      blanks += sample.blanks;
      keys += sample.keys;
      await page.close();
    }
  } finally {
    await browser.close();
    await stopViewer(server.child);
  }
  let debugBlanks = null;
  if (releaseRun) {
    const debugBinary = await ensureViewerBinary("debug", true);
    const debug = await oneScroll(debugBinary, ctDir);
    debugBlanks = debug.sample.blanks;
  }
  const value = median(rates);
  const fpsMet = value >= 55;
  const debugMet = debugBlanks == null || debugBlanks === 0;
  return {
    value,
    detail: {
      profile: reported,
      rates,
      keys,
      blank_samples: blanks,
      debug_blank_samples: debugBlanks,
      threshold_frames_per_second: 55,
      timer_ceiling_per_second: ceiling.perSecond,
      timer_ceiling_dispatches: ceiling.dispatches,
      timer_ceiling_seconds: ceiling.seconds,
      thresholds_met: fpsMet && blanks === 0 && debugMet,
    },
    instrument: instrumentOf(chromium),
  };
}

export async function measureWindow({ outDir, build, profile }) {
  const chosen = chosenProfile(profile);
  const binary = await ensureViewerBinary(chosen, build);
  const dirs = await ensureWorkloads(outDir);
  const server = await startViewer(binary);
  const reported = await reportedBuild(server.url);
  const { browser, chromium } = await launchBrowser();
  let ct = [];
  let dx = [];
  const bodies = [];
  let posted = 0;
  let pagePosted = 0;
  try {
    const ctPage = await openPage(browser, server.url);
    bodies.push(watchPreviewBodies(ctPage));
    await openFolder(ctPage, dirs.ctDir);
    ct = await windowSteps(ctPage, 9);
    await canvasMatchesReply(ctPage);
    const ctBodies = await ctPage.evaluate(() => window.__previewBodySizes || []);
    await ctPage.close();
    const dxPage = await openPage(browser, server.url);
    bodies.push(watchPreviewBodies(dxPage));
    await openFolder(dxPage, dirs.dxDir);
    dx = await windowSteps(dxPage, 9);
    await canvasMatchesReply(dxPage);
    const dxBodies = await dxPage.evaluate(() => window.__previewBodySizes || []);
    await dxPage.close();
    posted = bodies.reduce((sum, list) => sum + list.filter((length) => length > 0).length, 0);
    pagePosted = [...ctBodies, ...dxBodies].filter((length) => length !== 0).length;
    assertNoPreviewBody(posted + pagePosted);
  } finally {
    await browser.close();
    await stopViewer(server.child);
  }
  const value = median(ct);
  const dxValue = median(dx);
  return {
    value,
    detail: {
      profile: reported,
      ct_ms: ct,
      dx_ms: dx,
      dx_median_ms: dxValue,
      preview_bodies: posted + pagePosted,
      ct_threshold_ms: 5,
      dx_threshold_ms: 70,
      thresholds_met: value <= 5 && dxValue <= 70,
    },
    instrument: instrumentOf(chromium),
  };
}

async function sizedCanvases(page) {
  return page.evaluate(() => {
    const sized = [];
    // After a draw a viewport's canvas width is its image width. 300 by 150
    // is the unset element. Every viewport canvas the page has made counts.
    for (const view of document.querySelectorAll("#wrap canvas.view")) {
      if (view.width <= 1 || view.height <= 1) continue;
      if (view.width === 300 && view.height === 150) continue;
      sized.push([view.width, view.height]);
    }
    for (const node of document.querySelectorAll("canvas[data-tile]")) {
      if (node.width === 300 && node.height === 150) continue;
      if (node.width <= 1 || node.height <= 1) continue;
      sized.push([node.width, node.height]);
    }
    return sized;
  });
}

export async function readRetainedHeap(page) {
  const before = await page.evaluate(() => (
    performance.memory ? performance.memory.usedJSHeapSize : null
  ));
  if (before == null) throw new Error("the page did not expose performance.memory");
  const samples = [];
  while (samples.length < HEAP_READ_CAP) {
    const reading = await page.evaluate(() => {
      if (typeof globalThis.gc !== "function") return null;
      globalThis.gc();
      if (!performance.memory) return null;
      return performance.memory.usedJSHeapSize;
    });
    if (reading == null) throw new Error("the page did not expose gc or performance.memory");
    samples.push(reading);
    const folded = foldHeapReadings(samples, HEAP_AGREE_BYTES, HEAP_READ_CAP);
    if (folded.ok) {
      return { before, retained: folded.retained, collections: folded.collections };
    }
  }
  throw new Error("retained heap readings did not agree");
}

export async function measureResident({ outDir, build, profile }) {
  const chosen = chosenProfile(profile);
  const binary = await ensureViewerBinary(chosen, build);
  const dirs = await ensureWorkloads(outDir);
  const server = await startViewer(binary);
  const reported = await reportedBuild(server.url);
  const samples = [];
  const heldPeaks = { used: 0, free: 0, inflight: 0, workspace: 0 };
  let heldAtPeak = null;
  let peakKbSeen = 0;
  let stop = false;
  let ticks = 0;
  const sampler = (async () => {
    while (!stop) {
      const kb = await rssKb(server.pid);
      if (kb != null) {
        samples.push(kb);
        if (kb > peakKbSeen) {
          peakKbSeen = kb;
          try {
            const response = await fetch(`${server.url}held`);
            if (response.ok) heldAtPeak = await response.json();
          } catch {
            // The process may already be gone. The RSS samples still stand.
          }
        }
      }
      ticks += 1;
      if (ticks % 10 === 0) {
        try {
          const response = await fetch(`${server.url}held`);
          if (response.ok) {
            const body = await response.json();
            heldPeaks.used = Math.max(heldPeaks.used, Number(body.used) || 0);
            heldPeaks.free = Math.max(heldPeaks.free, Number(body.free) || 0);
            heldPeaks.inflight = Math.max(heldPeaks.inflight, Number(body.inflight) || 0);
            heldPeaks.workspace = Math.max(heldPeaks.workspace, Number(body.workspace) || 0);
          }
        } catch {
          // The process may already be gone. The RSS samples still stand.
        }
      }
      await new Promise((resolve) => setTimeout(resolve, 20));
    }
  })();
  const { browser, chromium } = await launchBrowser();
  let heap = null;
  let held = null;
  let canvases = [];
  try {
    for (let index = 0; index < 9; index += 1) {
      const page = await openPage(browser, server.url);
      await page.locator("#folder").setInputFiles(dirs.ctDir);
      await page.waitForFunction(() => /\d+ series\./.test(document.querySelector("#status")?.textContent || ""));
      await page.close();
    }
    for (let index = 0; index < 9; index += 1) {
      const page = await openPage(browser, server.url);
      await openFolder(page, dirs.ctDir);
      await page.close();
    }
    for (let index = 0; index < 9; index += 1) {
      const page = await openPage(browser, server.url);
      await openFolder(page, dirs.dxDir);
      await page.close();
    }
    for (let index = 0; index < 5; index += 1) {
      const page = await openPage(browser, server.url);
      await openFolder(page, dirs.ctDir);
      await scrollOnce(page);
      await page.close();
    }
    const ctPage = await openPage(browser, server.url);
    await openFolder(ctPage, dirs.ctDir);
    await windowSteps(ctPage, 9);
    await ctPage.close();
    // F-227. A series that stores no window, so the run includes three
    // series window rounds, one per open.
    for (let index = 0; index < 3; index += 1) {
      const page = await openPage(browser, server.url);
      await openFolder(page, dirs.petDir);
      await page.close();
    }
    const dxPage = await openPage(browser, server.url);
    await openFolder(dxPage, dirs.dxDir);
    await windowSteps(dxPage, 9);
    // Retained heap is read after forced collection until two readings agree
    // within HEAP_AGREE_BYTES. The reading before any collection is reported
    // beside it and has no threshold. A run that does not agree is invalid.
    heap = await readRetainedHeap(dxPage);
    canvases = await sizedCanvases(dxPage);
    held = await dxPage.evaluate(async () => {
      const response = await fetch("/held");
      return response.json();
    });
    await dxPage.close();
  } finally {
    stop = true;
    await sampler;
    await browser.close();
    await stopViewer(server.child);
  }
  const peakKb = samples.reduce((best, value) => Math.max(best, value), 0);
  const value = peakKb * 1024;
  const budget = held && Number.isFinite(held.budget) ? held.budget : 256 * MIB;
  const sMax = (await stat(join(dirs.dxDir, "dx-0000.dcm"))).size;
  const nMax = DX_ROWS * DX_COLUMNS;
  const rssBound = serverRssBound(budget, sMax, nMax);
  const heapBound = pageHeapBound(canvases);
  const retained = heap == null ? null : heap.retained;
  const before = heap == null ? null : heap.before;
  return {
    value,
    detail: {
      profile: reported,
      peak_rss_bytes: value,
      samples: samples.length,
      held,
      held_at_peak: heldAtPeak,
      held_peaks: heldPeaks,
      js_heap_bytes: retained,
      js_heap_collections: heap == null ? null : heap.collections,
      js_heap_before_gc: before,
      canvases,
      rss_bound_bytes: rssBound,
      series_pool_term_bytes: SERVER_PERMITS * SERIES_POOL_BYTES,
      heap_bound_bytes: heapBound,
      thresholds_met: value <= rssBound && (retained == null || retained <= heapBound),
    },
    instrument: instrumentOf(chromium),
  };
}
