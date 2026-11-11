// F-227 AC15. The automatic window in the page, on synthetic folders.
//
//   PT     20 slices of 32 by 32, unsigned, no window, Rescale Slope 0.5,
//          0.75, 1 and 1.25 in turn, stored 0 outside a disc of radius 12
//          and a pattern from 10 to 99 inside it
//   MR     3 slices of the same shape, no window, slope 1, stored three
//          times the PT pattern
//   CT     1 slice storing Window Center 40 and Width 400
//
// The design's rule is computed here again, independently of the local
// process: `series_sample(20)` picks `k * 19 / 15` for k from 0 to 15, each
// slice's modality value is `stored * slope` in f32 with stored 0 left out,
// and the window runs between the nearest ranks `ceil(n / 100)` and
// `ceil(99 n / 100)` of the pooled values. The library moves each edge at
// most 16 f32 units inward, so the reply must agree with it to 0.01.
//
// Each check names the wrong page it rejects. Every PT slice must show the
// same WW and WL with `Auto window`, which a per-slice window does not. The
// grey at the disc centre must be that window's LINEAR_EXACT grey on the
// first and the last slice, whose slopes differ, so one slope for the pool
// or a per-slice window draws a different grey. Exactly one
// `/series-window` request is made per series that stores no window, none
// for the windowed CT and none on scrolling, a W/L drag or Reset. A drag
// removes the words, Reset and the `Auto window` row restore them. A
// refused `/series-window` shows `Auto window, this image`. With the window
// link on, Reset in the outlined cell returns the other cell to its own
// series' window, not to the outlined one's. A series first shown after a
// process restart, a 40-file series under a 4 MiB cache that evicts the
// files posted at open, and a series with a changed file outside and one
// inside its sample each still show one series window, which a page that
// re-posts one gone file, stops at the first 410 or posts every image to ask
// does not. The 4 MiB case also requires exactly one 410 then a 200, which a
// process whose round evicts the sampled files it has not read yet does not
// give. Under a 1 MiB cache, which cannot hold the 16 sampled files, the
// page stops within 17 replies and shows `Auto window, this image`, which a
// page that does not remember what it posted never does.
//
// The files are written by this script into a temporary folder, carry no
// patient attributes, and are removed afterwards. Each observation starts its
// own viewer with OCELLI_PORT=0 and reads the URL it prints. The restart case
// starts the second process on the first one's port. The first three are
// stopped by PID, the others through `stopViewer`, through tools/bench's
// pinned Playwright. Every viewer is stopped in a `finally` of its own, so a
// thrown step or a failed browser launch or close cannot skip it, and the run
// ends by confirming that no viewer it started is still alive.
// `bin/ocelli.sh gate viewer-browser` runs it after changed.mjs.

import { Buffer } from "node:buffer";
import { mkdtempSync, rmSync, utimesSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { fileURLToPath } from "node:url";

import { ensureViewerBinary, longEl, shortEl, startViewer, stopViewer } from "../../../../tools/bench/src/viewer_work.mjs";
import { loadPlaywright, stopByPid } from "./aspect.mjs";

// The page's own top-level bindings, read inside `page.evaluate` bodies,
// which run in the page and not in Node.
/* global setLayout, setLink,  activeCell, cellShown, seriesWindows, probeFiles, seriesFiles,  changedFiles */

// Every viewer this run started, so the end of the run can confirm each is gone.
const started = [];

async function track(binary, env) {
  const server = await startViewer(binary, env);
  started.push(server.pid);
  return server;
}

/** `run(holder)` with a viewer in `holder.server`, stopped whatever happens. */
async function withViewer(binary, env, run) {
  const holder = { server: await track(binary, env) };
  try {
    return await run(holder);
  } finally {
    await stopViewer(holder.server.child);
  }
}

/** `run(browser)` with a headless Chromium, closed without masking a throw. */
async function withBrowser(chromium, run) {
  const browser = await chromium.launch({ headless: true });
  try {
    return await run(browser);
  } finally {
    await browser.close().catch(() => {});
  }
}

/** The pids this run started that are still alive. Each is then killed. */
export function survivors(pids = started) {
  const alive = [];
  for (const pid of pids) {
    try {
      process.kill(pid, 0);
      alive.push(pid);
      process.kill(pid, "SIGKILL");
    } catch {
      // Gone, as it should be.
    }
  }
  return alive;
}

export const PT_SLICES = 20;
export const MR_SLICES = 3;
const EDGE = 32;

function text(value) {
  const bytes = Buffer.from(value, "latin1");
  return bytes.length % 2 ? Buffer.concat([bytes, Buffer.from(" ")]) : bytes;
}

function uid(value) {
  const bytes = Buffer.from(value, "latin1");
  return bytes.length % 2 ? Buffer.concat([bytes, Buffer.from([0])]) : bytes;
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

/** The PT slope of slice `k`, exact in binary. */
export function ptSlope(k) {
  return 0.5 + 0.25 * (k % 4);
}

/** The stored value at column `x`, row `y` of slice `k`, times `scale`. */
export function storedAt(x, y, k, scale = 1) {
  const dx = x - 16;
  const dy = y - 16;
  if (dx * dx + dy * dy > 144) return 0;
  return scale * (10 + ((x + 2 * y + 3 * k) % 90));
}

/** `ocelli_pixel::series_sample`, from the design's item 6. */
export function seriesSample(images) {
  const sampled = Math.min(images, 16);
  if (sampled === 1) return [0];
  const out = [];
  for (let k = 0; k < sampled; k += 1) out.push(Math.floor((k * (images - 1)) / (sampled - 1)));
  return out;
}

/** The nearest-rank window of the pooled modality values, before any margin. */
export function expectedWindow(slices, slopeOf, scale) {
  const values = [];
  for (const k of seriesSample(slices)) {
    const slope = Math.fround(slopeOf(k));
    for (let y = 0; y < EDGE; y += 1) {
      for (let x = 0; x < EDGE; x += 1) {
        const stored = storedAt(x, y, k, scale);
        if (stored === 0) continue;
        values.push(Math.fround(stored * slope));
      }
    }
  }
  values.sort((a, b) => a - b);
  const n = values.length;
  const low = values[Math.ceil(n / 100) - 1];
  const high = values[Math.ceil((99 * n) / 100) - 1];
  return { center: low + (high - low) / 2, width: high - low };
}

/** PS3.3 C.11.2.1.3.2 LINEAR_EXACT, then `floor(y + 0.5)`. */
export function exactGrey(x, center, width) {
  if (x <= center - width / 2) return 0;
  if (x > center + width / 2) return 255;
  return Math.floor(((x - center) / width + 0.5) * 255 + 0.5);
}

function slice({ sop, series, number, instance, modality, slope, pixels, window, edge = EDGE }) {
  const sopClass = "1.2.840.10008.5.1.4.1.1.128";
  const meta = Buffer.concat([
    longEl(0x0002, 0x0001, "OB", Buffer.from([0, 1])),
    shortEl(0x0002, 0x0002, "UI", uid(sopClass)),
    shortEl(0x0002, 0x0003, "UI", uid(sop)),
    shortEl(0x0002, 0x0010, "UI", uid("1.2.840.10008.1.2.1")),
    shortEl(0x0002, 0x0012, "UI", uid("2.25.227")),
  ]);
  return Buffer.concat([
    Buffer.alloc(128),
    Buffer.from("DICM"),
    shortEl(0x0002, 0x0000, "UL", u32(meta.length)),
    meta,
    shortEl(0x0008, 0x0016, "UI", uid(sopClass)),
    shortEl(0x0008, 0x0018, "UI", uid(sop)),
    shortEl(0x0008, 0x0060, "CS", text(modality)),
    shortEl(0x0020, 0x000e, "UI", uid(series)),
    shortEl(0x0020, 0x0011, "IS", text(String(number))),
    shortEl(0x0020, 0x0013, "IS", text(String(instance))),
    shortEl(0x0028, 0x0002, "US", u16(1)),
    shortEl(0x0028, 0x0004, "CS", text("MONOCHROME2")),
    shortEl(0x0028, 0x0010, "US", u16(edge)),
    shortEl(0x0028, 0x0011, "US", u16(edge)),
    shortEl(0x0028, 0x0100, "US", u16(16)),
    shortEl(0x0028, 0x0101, "US", u16(16)),
    shortEl(0x0028, 0x0102, "US", u16(15)),
    shortEl(0x0028, 0x0103, "US", u16(0)),
    ...(window ? [shortEl(0x0028, 0x1050, "DS", text(window[0])), shortEl(0x0028, 0x1051, "DS", text(window[1]))] : []),
    shortEl(0x0028, 0x1052, "DS", text("0")),
    shortEl(0x0028, 0x1053, "DS", text(String(slope))),
    longEl(0x7fe0, 0x0010, "OW", pixels),
  ]);
}

function pixelsOf(k, scale) {
  const bytes = Buffer.alloc(EDGE * EDGE * 2);
  for (let y = 0; y < EDGE; y += 1) {
    for (let x = 0; x < EDGE; x += 1) bytes.writeUInt16LE(storedAt(x, y, k, scale), (y * EDGE + x) * 2);
  }
  return bytes;
}

function writeFolder(folder) {
  for (let k = 0; k < PT_SLICES; k += 1) {
    writeFileSync(join(folder, `pt-${String(k).padStart(2, "0")}.dcm`), slice({
      sop: `2.25.2271${k}`,
      series: "2.25.22701",
      number: 1,
      instance: k + 1,
      modality: "PT",
      slope: ptSlope(k),
      pixels: pixelsOf(k, 1),
    }));
  }
  for (let k = 0; k < MR_SLICES; k += 1) {
    writeFileSync(join(folder, `mr-${k}.dcm`), slice({
      sop: `2.25.2272${k}`,
      series: "2.25.22702",
      number: 2,
      instance: k + 1,
      modality: "MR",
      slope: 1,
      pixels: pixelsOf(k, 3),
    }));
  }
  writeFileSync(join(folder, "ct-0.dcm"), slice({
    sop: "2.25.22730",
    series: "2.25.22703",
    number: 3,
    instance: 1,
    modality: "CT",
    slope: 1,
    pixels: pixelsOf(0, 1),
    window: ["40", "400"],
  }));
}

// Review pass 16, D1 and D2. Two no-window series of 20 slices, A shown at
// open and B shown later, for a restart and for files F-224 marks changed.
// `series_sample(20)` takes 0 to 3, 5 to 8, 10 to 13, 15 to 17 and 19, so
// index 4 is outside the sample and index 5 inside it.
export const PAIR_SLICES = 20;
export const OUTSIDE = 4;
export const INSIDE = 5;

function writePair(folder) {
  for (const [series, number] of [["2.25.22711", 1], ["2.25.22712", 2]]) {
    for (let k = 0; k < PAIR_SLICES; k += 1) {
      writeFileSync(join(folder, `s${number}-${String(k).padStart(2, "0")}.dcm`), slice({
        sop: `${series}.${k}`,
        series,
        number,
        instance: k + 1,
        modality: "PT",
        slope: ptSlope(k),
        pixels: pixelsOf(k, number),
      }));
    }
  }
}

// D1 under F-216's budget. 40 slices of 256 by 256, 128 KiB each, more than
// a 4 MiB cache holds, so files posted at open are evicted before the
// series window is asked for.
export const BIG_SLICES = 40;
const BIG_EDGE = 256;

function writeBig(folder) {
  for (let k = 0; k < BIG_SLICES; k += 1) {
    const bytes = Buffer.alloc(BIG_EDGE * BIG_EDGE * 2);
    for (let at = 0; at < BIG_EDGE * BIG_EDGE; at += 1) {
      const x = at % BIG_EDGE;
      const y = Math.floor(at / BIG_EDGE);
      bytes.writeUInt16LE(at % 7 === 0 ? 0 : 10 + ((x + 2 * y + 3 * k) % 90), at * 2);
    }
    writeFileSync(join(folder, `big-${String(k).padStart(2, "0")}.dcm`), slice({
      sop: `2.25.22721.${k}`,
      series: "2.25.22721",
      number: 1,
      instance: k + 1,
      modality: "PT",
      slope: ptSlope(k),
      pixels: bytes,
      edge: BIG_EDGE,
    }));
  }
}

/** The first `count` slices of the outlined cell, stepping with ArrowDown. */
async function slicesOf(page, count) {
  await settle(page, { suffix: "  Auto window" });
  const seen = [await sample(page)];
  for (let k = 1; k < count; k += 1) {
    await page.keyboard.press("ArrowDown");
    await settle(page, { suffix: "  Auto window" });
    seen.push(await sample(page));
  }
  return seen;
}

function watchReplies(page) {
  const replies = [];
  page.on("response", async (response) => {
    if (new URL(response.url()).pathname !== "/series-window") return;
    replies.push({ status: response.status(), body: response.status() === 200 ? await response.json().catch(() => null) : null });
  });
  return replies;
}

/** Series A drawn, the process restarted on the same port, then series B. */
export function observeRestart(binary, chromium, folder) {
  return withViewer(binary, {}, (holder) => withBrowser(chromium, async (browser) => {
    const page = await browser.newPage({ viewport: { width: 1400, height: 900 } });
    const replies = watchReplies(page);
    await page.goto(holder.server.url);
    await page.locator("#folder").setInputFiles(folder);
    await page.waitForFunction(() => (document.getElementById("status").textContent || "").startsWith("2 series."));
    await settle(page, { suffix: "  Auto window" });
    const port = holder.server.port;
    await stopViewer(holder.server.child);
    holder.server = await track(binary, { OCELLI_PORT: String(port) });
    await page.keyboard.press("s");
    await page.locator("#series-list button", { hasText: "#2" }).click();
    const slices = await slicesOf(page, 4);
    return { slices, replies };
  }));
}

/** A 4 MiB cache that evicts the files posted at open. */
export function observeEvicted(binary, chromium, folder) {
  return withViewer(binary, { OCELLI_CACHE_MIB: "4" }, (holder) => withBrowser(chromium, async (browser) => {
    const server = holder.server;
    const page = await browser.newPage({ viewport: { width: 1400, height: 900 } });
    const replies = watchReplies(page);
    await page.goto(server.url);
    await page.locator("#folder").setInputFiles(folder);
    await page.waitForFunction(() => (document.getElementById("status").textContent || "").startsWith("1 series."));
    const slices = await slicesOf(page, 4);
    return { slices, replies };
  }));
}

/**
 * S1, the stopping rule. The 40-file series under a 1 MiB cache, which
 * cannot hold its 16 sampled files. Every `/series-window` reply is counted
 * for 6 s after the first slice settles.
 */
export function observeStarved(binary, chromium, folder) {
  return withViewer(binary, { OCELLI_CACHE_MIB: "1" }, (holder) => withBrowser(chromium, async (browser) => {
    const server = holder.server;
    const page = await browser.newPage({ viewport: { width: 1400, height: 900 } });
    const replies = watchReplies(page);
    await page.goto(server.url);
    await page.locator("#folder").setInputFiles(folder);
    await page.waitForFunction(() => (document.getElementById("status").textContent || "").startsWith("1 series."));
    await settle(page, { suffix: "  Auto window, this image" });
    await page.waitForTimeout(6000);
    return { first: await sample(page), replies: replies.length };
  }));
}

export function starvedProblems(seen) {
  const problems = [];
  if (seen.replies > 17) problems.push(`under a 1 MiB cache the page sent ${seen.replies} series window requests, more than the 17 its stopping rule allows`);
  if (!seen.first.line.endsWith("  Auto window, this image")) problems.push(`under a 1 MiB cache slice 1 reads ${JSON.stringify(seen.first.line)}, wanted its own window`);
  return problems;
}

/** Series B's files at `OUTSIDE` and `INSIDE` changed on disk, then B shown. */
export function observeChanged(binary, chromium, folder) {
  return withViewer(binary, {}, (holder) => withBrowser(chromium, async (browser) => {
    const server = holder.server;
    const page = await browser.newPage({ viewport: { width: 1400, height: 900 } });
    const replies = watchReplies(page);
    await page.goto(server.url);
    await page.locator("#folder").setInputFiles(folder);
    await page.waitForFunction(() => (document.getElementById("status").textContent || "").startsWith("2 series."));
    await settle(page, { suffix: "  Auto window" });
    const later = new Date(Date.now() + 5000);
    for (const k of [OUTSIDE, INSIDE]) utimesSync(join(folder, `s2-${String(k).padStart(2, "0")}.dcm`), later, later);
    // F-224's probe over series B, as a selection would run it, before B is shown.
    const marked = await page.evaluate(async () => {
      const series = Array.from(seriesByKey.values()).find((row) => row.number === "2");
      await probeFiles(seriesFiles(series), currentGeneration);
      return series.images.filter((image) => changedFiles.has(image.file)).length;
    });
    const before = replies.length;
    await page.keyboard.press("s");
    await page.locator("#series-list button", { hasText: "#2" }).click();
    const slices = await slicesOf(page, 4);
    return { marked, slices, replies: replies.slice(before) };
  }));
}

/** Every observed slice reads one series window with Auto window. */
function oneWindow(name, slices) {
  const first = slices[0] ? slices[0].line : "";
  if (!first.endsWith("  Auto window")) return [`${name}: slice 1 reads ${JSON.stringify(first)}, not a series window`];
  const other = slices.find((got) => got.line !== first);
  return other ? [`${name}: a slice reads ${JSON.stringify(other.line)} beside ${JSON.stringify(first)}`] : [];
}

export function restartProblems(seen) {
  return oneWindow("after a restart", seen.slices);
}

export function evictedProblems(seen) {
  const problems = oneWindow("under a 4 MiB cache", seen.slices);
  // Review pass 18 S2. One 410 naming the files evicted at open, then 200.
  // A round that evicts sampled files of its own makes a second 410.
  const statuses = seen.replies.map((reply) => reply.status).join(", ");
  if (statuses !== "410, 200") problems.push(`under a 4 MiB cache the series window replies were ${statuses}, wanted 410, 200`);
  return problems;
}

export function changedProblems(seen) {
  const problems = [];
  if (seen.marked !== 2) problems.push(`the probe marked ${seen.marked} files of series B, wanted 2`);
  problems.push(...oneWindow("with two changed files", seen.slices));
  const done = seen.replies.find((reply) => reply.status === 200);
  if (!done || !done.body || done.body.sampled !== 16 || done.body.skipped !== 1) {
    problems.push(`with two changed files the series window reply was ${JSON.stringify(done)}, wanted sampled 16 and skipped 1`);
  }
  return problems;
}

/** The outlined cell's first corner line, its fault, and the grey at the disc centre. */
function sample(page, cell = ".cell.active") {
  return page.evaluate((selector) => {
    const node = document.querySelector(selector);
    const line = (node.querySelector(".overlay.br").textContent || "").split("\n")[0];
    const faultNode = node.querySelector(".fault");
    const fault = faultNode && !faultNode.hidden ? faultNode.textContent : "";
    const canvas = node.querySelector("canvas.view");
    let grey = null;
    if (canvas && canvas.width === 32) grey = canvas.getContext("2d").getImageData(16, 16, 1, 1).data[0];
    return { line, fault, grey };
  }, cell);
}

/** Wait until the cell's first corner line ends with `want.suffix` or starts with `want.prefix`. */
async function settle(page, want, cell = ".cell.active") {
  await page.waitForFunction(([selector, suffix, prefix]) => {
    const node = document.querySelector(selector);
    const line = (node.querySelector(".overlay.br").textContent || "").split("\n")[0];
    const fault = node.querySelector(".fault");
    const matched = suffix ? line.endsWith(suffix) : line.startsWith(prefix);
    return matched || (fault && !fault.hidden);
  }, [cell, want.suffix || "", want.prefix || ""], { timeout: 8000 }).catch(() => {});
  await page.waitForTimeout(150);
}

function counter(page) {
  const seen = { count: 0 };
  page.on("request", (request) => {
    if (request.method() === "POST" && new URL(request.url()).pathname === "/series-window") seen.count += 1;
  });
  return seen;
}

async function dragWindow(page) {
  await page.keyboard.press("w");
  const box = await page.locator(".cell.active canvas.view").boundingBox();
  const x = box.x + box.width / 2;
  const y = box.y + box.height / 2;
  await page.mouse.move(x, y);
  await page.mouse.down();
  await page.mouse.move(x + 30, y + 20, { steps: 4 });
  await page.mouse.move(x + 60, y + 40, { steps: 4 });
  await page.mouse.up();
}

/** One page: the PT series held in the window, an optional ArrowDown, then release and the row. */
export async function observeTilesBeforeSeriesWindow(url, chromium, folder, scroll = false, fail = false) {
  const browser = await chromium.launch({ headless: true });
  try {
    const page = await browser.newPage({ viewport: { width: 1400, height: 900 } });
    const held = [];
    let asked = 0;
    await page.route("**/series-window", (route) => {
      // A failed request never stores a window, so abort instead of holding.
      if (fail) {
        asked += 1;
        route.abort();
        return;
      }
      held.push(route);
    });
    await page.goto(url);
    await page.evaluate(() => {
      window.__labels = [];
      new MutationObserver(() => {
        for (const label of document.querySelectorAll(".cell .tiles .tile-label")) {
          const text = label.textContent || "";
          if (text) window.__labels.push(text);
        }
      }).observe(document.body, { subtree: true, characterData: true, childList: true });
    });
    await page.locator("#folder").setInputFiles(folder);
    await page.waitForFunction(() => (document.getElementById("status").textContent || "").startsWith("3 series."));
    // page.waitForFunction runs in the page and cannot see Node state, so poll here.
    if (!fail) {
      const heldDeadline = Date.now() + 5000;
      while (held.length < 1) {
        if (Date.now() > heldDeadline) throw new Error("series-window request was never held");
        await page.waitForTimeout(50);
      }
    }
    await page.keyboard.press("Shift+G");
    await page.waitForTimeout(1500);
    if (scroll) {
      // Exercise tile layout again while the series window request is still held.
      await page.keyboard.press("ArrowDown");
      await page.waitForTimeout(300);
    }
    if (fail) {
      // A refusal is not asked again for this open, so count every route call.
      await page.keyboard.press("ArrowDown");
      await page.waitForTimeout(400);
      await page.keyboard.press("ArrowDown");
      await page.waitForTimeout(400);
    }
    const tiles = () => page.$$eval(".cell.active .tiles .tile", (nodes) => nodes.map((node) => ({
      label: (node.querySelector(".tile-label") || {}).textContent || "",
    })));
    const during = await tiles();
    for (const route of held.splice(0)) await route.continue();
    const end = Date.now() + 8000;
    for (;;) {
      const labels = await page.$$eval(".cell.active .tile-label", (nodes) => nodes.map((node) => node.textContent || ""));
      if (labels.length === 4 && (fail ? labels.every((label) => label.includes("WW ")) : labels.every((label) => label.endsWith("  Auto window")))) break;
      if (Date.now() > end) {
        if (fail) break;
        throw new Error("tiles never settled to Auto window");
      }
      await page.waitForTimeout(50);
    }
    const after = await tiles();
    const log = await page.evaluate(() => window.__labels);
    return { during, after, log, fail, asked };
  } finally {
    await browser.close();
  }
}

export async function observeSeriesSwitchDuringWait(url, chromium, folder) {
  const browser = await chromium.launch({ headless: true });
  try {
    const page = await browser.newPage({ viewport: { width: 1400, height: 900 } });
    const held = [];
    let open = false;
    await page.route("**/series-window", (route) => (open ? route.continue() : held.push(route)));
    await page.goto(url);
    await page.locator("#folder").setInputFiles(folder);
    await page.waitForFunction(() => (document.getElementById("status").textContent || "").startsWith("3 series."));
    const deadline = Date.now() + 5000;
    while (held.length < 1) {
      if (Date.now() > deadline) throw new Error("series-window request was never held");
      await page.waitForTimeout(50);
    }
    await page.keyboard.press("Shift+G");
    await page.waitForTimeout(800);
    await page.locator("#series-list button").filter({ hasText: "#2" }).click();
    await page.waitForTimeout(300);
    // The pump is still waiting on the PT request, so the MR request is sent
    // only once that wait ends. Continue the PT request, then read while the
    // MR request is held.
    await held[0].continue();
    const mrDeadline = Date.now() + 5000;
    while (held.length < 2) {
      if (Date.now() > mrDeadline) throw new Error("the MR series-window request was never held");
      await page.waitForTimeout(50);
    }
    await page.waitForTimeout(1200);
    const labels = () => page.$$eval(".cell.active .tile-label", (nodes) => nodes.map((node) => node.textContent || ""));
    const during = await labels();
    open = true;
    for (const route of held.slice(1)) await route.continue();
    const end = Date.now() + 8000;
    let after;
    for (;;) {
      // The MR series holds three images, so the fourth tile stays empty.
      after = (await labels()).filter((label) => label !== "");
      if (after.length > 0 && after.every((label) => label.endsWith("  Auto window"))) break;
      if (Date.now() > end) break;
      await page.waitForTimeout(50);
    }
    return { during, after, held: held.length };
  } finally {
    await browser.close();
  }
}

export function seriesSwitchDuringWaitProblems(seen) {
  const problems = [];
  if (seen.held < 2) {
    problems.push(`expected at least two held series-window requests but saw ${seen.held}`);
  }
  if (seen.during.some((label) => label.includes("WW "))) {
    problems.push(`MR tiles were painted before the MR series-window request was released: ${JSON.stringify(seen.during)}`);
  }
  if (seen.after.length === 0 || !seen.after.every((label) => label.endsWith("  Auto window"))) {
    problems.push(`the MR tiles did not settle to Auto window labels after the MR request: ${JSON.stringify(seen.after)}`);
  }
  return problems;
}

export function failedSeriesWindowTileProblems(seen) {
  // A failed series-window request leaves each image at its own window and never leaves tile mode blank.
  const problems = [];
  if (seen.asked !== 1) {
    problems.push(`a failed request is a refusal for this open and was asked ${seen.asked} times instead of once`);
  }
  if (seen.after.length !== 4 || !seen.after.every((tile) => (tile.label || "").includes("WW "))) {
    problems.push("a failed series-window request did not leave four tiles with WW labels");
  }
  return problems;
}

export function tilesBeforeSeriesWindowProblems(seen) {
  // F-227 never draws an image at two windows, and tile mode must wait for the series window too.
  const problems = [];
  if (!seen.during.every((tile) => tile.label === "")) {
    problems.push("tile mode painted before the series window was stored");
  }
  if (seen.after.length !== 4 || !seen.after.every((tile) => (tile.label || "").endsWith("  Auto window"))) {
    problems.push("tile labels did not all end with Auto window");
  }
  if (seen.log.some((label) => label.includes("this image"))) {
    problems.push("tile labels mentioned this image");
  }
  const windows = new Set(seen.log.map((label) => label.replace(/^Image \d+ {2}/, "").replace("  Auto window", "")).filter((label) => label.includes("WW ")));
  if (windows.size !== 1) problems.push(`tile labels showed ${windows.size} distinct WW/WL strings`);
  return problems;
}

/** One page: the PT series opened and read. */
export async function observeSeries(url, chromium, folder) {
  const browser = await chromium.launch({ headless: true });
  try {
    const page = await browser.newPage({ viewport: { width: 1400, height: 900 } });
    const requests = counter(page);
    const replies = [];
    page.on("response", async (response) => {
      if (new URL(response.url()).pathname !== "/series-window") return;
      replies.push(await response.json().catch(() => null));
    });
    await page.goto(url);
    await page.locator("#folder").setInputFiles(folder);
    await page.waitForFunction(() => (document.getElementById("status").textContent || "").startsWith("3 series."));
    await settle(page, { suffix: "  Auto window" });
    const slices = [];
    slices.push(await sample(page));
    for (let k = 1; k < PT_SLICES; k += 1) {
      await page.keyboard.press("ArrowDown");
      await settle(page, { suffix: "  Auto window" });
      slices.push(await sample(page));
    }
    const afterScroll = requests.count;
    await dragWindow(page);
    await settle(page, { suffix: "  manual" });
    const dragged = await sample(page);
    await page.keyboard.press("Shift+R");
    await settle(page, { suffix: "  Auto window" });
    const reset = await sample(page);
    await dragWindow(page);
    await settle(page, { suffix: "  manual" });
    await page.keyboard.press("1");
    await settle(page, { suffix: "  Auto window" });
    const row = await sample(page);
    const rowLabel = await page.evaluate(() => {
      const active = document.querySelector("#window-list button.active .window-label");
      return active ? active.textContent : "";
    });
    const beforeOthers = requests.count;
    await page.keyboard.press("s");
    await page.locator("#series-list button", { hasText: "#2" }).click();
    await settle(page, { suffix: "  Auto window" });
    const mr = await sample(page);
    await page.locator("#series-list button", { hasText: "#3" }).click();
    await settle(page, { prefix: "WW 400  WL 40" });
    const ct = await sample(page);
    await page.locator("#series-list button", { hasText: "#1" }).click();
    await settle(page, { suffix: "  Auto window" });
    const back = await sample(page);
    return { slices, afterScroll, dragged, reset, row, rowLabel, beforeOthers, mr, ct, back, total: requests.count, replies };
  } finally {
    await browser.close();
  }
}

/** A refused `/series-window`: the image is drawn at its own window, so labelled. */
export async function observeRefused(url, chromium, folder) {
  const browser = await chromium.launch({ headless: true });
  try {
    const page = await browser.newPage({ viewport: { width: 1400, height: 900 } });
    await page.route("**/series-window", (route) => route.fulfill({
      status: 400,
      headers: { "X-Ocelli-Error": "window", "Content-Type": "text/plain" },
      body: "refused",
    }));
    await page.goto(url);
    await page.locator("#folder").setInputFiles(folder);
    await page.waitForFunction(() => (document.getElementById("status").textContent || "").startsWith("3 series."));
    await settle(page, { suffix: "  Auto window, this image" });
    return await sample(page);
  } finally {
    await browser.close();
  }
}

/** Two cells, PT and MR, window link on. A drag then Reset in the PT cell. */
export async function observeLinked(url, chromium, folder) {
  const browser = await chromium.launch({ headless: true });
  try {
    const page = await browser.newPage({ viewport: { width: 1400, height: 900 } });
    await page.goto(url);
    await page.locator("#folder").setInputFiles(folder);
    await page.waitForFunction(() => (document.getElementById("status").textContent || "").startsWith("3 series."));
    await settle(page, { suffix: "  Auto window" });
    await page.evaluate(() => {
      setLayout(2, 1);
      setLink("window", true);
    });
    const other = ".cell:not(.active)";
    await settle(page, { suffix: "  Auto window" }, other);
    const before = await sample(page, other);
    await dragWindow(page);
    await settle(page, { suffix: "  manual" }, other);
    const dragged = await sample(page, other);
    await page.keyboard.press("Shift+R");
    await settle(page, { suffix: "  Auto window" }, other);
    const after = await sample(page, other);
    const own = await page.evaluate(() => {
      const target = viewports.find((vp) => vp.cell !== activeCell && cellShown(vp));
      const entry = target ? seriesWindows.get(target.seriesKey) : null;
      return entry ? "WW " + entry.width + "  WL " + entry.center + "  Auto window" : "";
    });
    const source = await page.evaluate(() => {
      const entry = seriesWindows.get(activeViewport().seriesKey);
      return entry ? "WW " + entry.width + "  WL " + entry.center + "  Auto window" : "";
    });
    return { before, dragged, after, own, source };
  } finally {
    await browser.close();
  }
}

function within(text, want) {
  const value = Number(text);
  return Number.isFinite(value) && Math.abs(value - want) <= 0.01;
}

export function seriesProblems(seen) {
  const problems = [];
  const pt = expectedWindow(PT_SLICES, ptSlope, 1);
  const reply = seen.replies[0];
  if (!reply || !within(reply.center, pt.center) || !within(reply.width, pt.width)) {
    problems.push(`the PT series window was ${JSON.stringify(reply)}, wanted centre ${pt.center} and width ${pt.width}`);
    return problems;
  }
  if (reply.sampled !== 16 || reply.skipped !== 0) {
    problems.push(`the PT series sampled ${reply.sampled} and skipped ${reply.skipped}, wanted 16 and 0`);
  }
  const line = "WW " + reply.width + "  WL " + reply.center + "  Auto window";
  const center = Number(reply.center);
  const width = Number(reply.width);
  seen.slices.forEach((got, k) => {
    if (got.fault) problems.push(`PT slice ${k + 1} shows the fault ${got.fault}`);
    else if (got.line !== line) problems.push(`PT slice ${k + 1} reads ${JSON.stringify(got.line)}, wanted ${JSON.stringify(line)}`);
  });
  for (const k of [0, PT_SLICES - 1]) {
    const want = exactGrey(Math.fround(storedAt(16, 16, k) * ptSlope(k)), center, width);
    const got = seen.slices[k].grey;
    if (got == null || Math.abs(got - want) > 1) problems.push(`PT slice ${k + 1} draws grey ${got} at the disc centre, wanted ${want}`);
  }
  if (seen.afterScroll !== 1) problems.push(`opening and scrolling the PT series made ${seen.afterScroll} series window requests, wanted 1`);
  if (seen.dragged.line.includes("Auto window")) problems.push(`a W/L drag kept the words: ${JSON.stringify(seen.dragged.line)}`);
  if (seen.reset.line !== line) problems.push(`Reset reads ${JSON.stringify(seen.reset.line)}, wanted ${JSON.stringify(line)}`);
  if (seen.row.line !== line || seen.rowLabel !== "Auto window") {
    problems.push(`the Auto window row reads ${JSON.stringify(seen.row.line)} with ${JSON.stringify(seen.rowLabel)} active`);
  }
  if (seen.beforeOthers !== 1) problems.push(`a drag, Reset and the row made ${seen.beforeOthers - 1} more series window requests`);
  const mr = expectedWindow(MR_SLICES, () => 1, 3);
  const mrReply = seen.replies[1];
  if (!mrReply || !within(mrReply.center, mr.center) || !within(mrReply.width, mr.width)) {
    problems.push(`the MR series window was ${JSON.stringify(mrReply)}, wanted centre ${mr.center} and width ${mr.width}`);
  } else if (seen.mr.line !== "WW " + mrReply.width + "  WL " + mrReply.center + "  Auto window") {
    problems.push(`the MR reads ${JSON.stringify(seen.mr.line)}`);
  }
  if (seen.ct.line !== "WW 400  WL 40" && !seen.ct.line.startsWith("WW 400  WL 40  Window")) {
    problems.push(`the windowed CT reads ${JSON.stringify(seen.ct.line)}, which must carry no Auto window`);
  }
  if (seen.ct.line.includes("Auto window")) problems.push("the windowed CT was labelled Auto window");
  if (seen.back.line !== line) problems.push(`the PT series shown again reads ${JSON.stringify(seen.back.line)}`);
  if (seen.total !== 2) problems.push(`three series made ${seen.total} series window requests, wanted 2`);
  return problems;
}

export function refusedProblems(seen) {
  if (!seen.line.endsWith("  Auto window, this image") || seen.fault) {
    return [`a refused series window reads ${JSON.stringify(seen.line)} with fault ${JSON.stringify(seen.fault)}`];
  }
  return [];
}

export function linkedProblems(seen) {
  const problems = [];
  if (!seen.own || seen.own === seen.source) {
    problems.push(`the two cells did not get two series windows: ${JSON.stringify(seen.own)} and ${JSON.stringify(seen.source)}`);
  }
  if (seen.before.line !== seen.own) problems.push(`the MR cell first read ${JSON.stringify(seen.before.line)}, wanted ${JSON.stringify(seen.own)}`);
  if (!seen.dragged.line.endsWith("  manual")) problems.push(`the linked drag left the MR cell at ${JSON.stringify(seen.dragged.line)}`);
  if (seen.after.line !== seen.own) problems.push(`after a linked Reset the MR cell reads ${JSON.stringify(seen.after.line)}, wanted its own ${JSON.stringify(seen.own)}`);
  return problems;
}

async function main() {
  const { chromium } = await loadPlaywright();
  const binary = await ensureViewerBinary("release", true);
  const folder = mkdtempSync(join(tmpdir(), "ocelli-f227-"));
  const alone = async (observe) => {
    const server = await track(binary);
    try {
      return await observe(server.url);
    } finally {
      await stopByPid(server.pid);
    }
  };
  const pairRestart = mkdtempSync(join(tmpdir(), "ocelli-f227-restart-"));
  const pairChanged = mkdtempSync(join(tmpdir(), "ocelli-f227-changed-"));
  const big = mkdtempSync(join(tmpdir(), "ocelli-f227-big-"));
  let seen;
  let tilesScroll;
  let refused;
  let linked;
  let restart;
  let evicted;
  let changed;
  let starved;
  let tiles;
  let tilesFailed;
  let seriesSwitch;
  try {
    writeFolder(folder);
    writePair(pairRestart);
    writePair(pairChanged);
    writeBig(big);
    seen = await alone((url) => observeSeries(url, chromium, folder));
    refused = await alone((url) => observeRefused(url, chromium, folder));
    linked = await alone((url) => observeLinked(url, chromium, folder));
    restart = await observeRestart(binary, chromium, pairRestart);
    evicted = await observeEvicted(binary, chromium, big);
    changed = await observeChanged(binary, chromium, pairChanged);
    starved = await observeStarved(binary, chromium, big);
    tiles = await alone((url) => observeTilesBeforeSeriesWindow(url, chromium, folder));
    tilesScroll = await alone((url) => observeTilesBeforeSeriesWindow(url, chromium, folder, true));
    tilesFailed = await alone((url) => observeTilesBeforeSeriesWindow(url, chromium, folder, false, true));
    seriesSwitch = await alone((url) => observeSeriesSwitchDuringWait(url, chromium, folder));
    console.log("tiles before series window with ArrowDown:", JSON.stringify({ during: tilesScroll.during.length, after: tilesScroll.after.length, labels: tilesScroll.log.length }));
  } finally {
    for (const dir of [folder, pairRestart, pairChanged, big]) rmSync(dir, { recursive: true, force: true });
    // Every case stops its own viewer. Any still alive here is a leak, even
    // when a case threw, and it is killed and reported.
    const orphans = survivors();
    if (orphans.length) {
      console.error(`viewers left running by this run: ${orphans.join(", ")}`);
      process.exitCode = 1;
    }
  }
  console.log(`PT: ${JSON.stringify(seen.slices[0].line)} on slice 1, grey ${seen.slices[0].grey} and ${seen.slices[PT_SLICES - 1].grey} at the centre of slices 1 and ${PT_SLICES}`);
  console.log(`series window requests: ${seen.afterScroll} for the PT series, ${seen.total} in all`);
  console.log(`MR ${JSON.stringify(seen.mr.line)}, CT ${JSON.stringify(seen.ct.line)}, refused ${JSON.stringify(refused.line)}`);
  console.log(`linked: before ${JSON.stringify(linked.before.line)}, after Reset ${JSON.stringify(linked.after.line)}`);
  console.log(`restart: ${restart.replies.map((reply) => reply.status).join(", ")}, slice 1 ${JSON.stringify(restart.slices[0].line)}`);
  console.log(`4 MiB cache: ${evicted.replies.map((reply) => reply.status).join(", ")}, slice 1 ${JSON.stringify(evicted.slices[0].line)}`);
  console.log(`changed: ${changed.marked} marked, ${changed.replies.map((reply) => reply.status).join(", ")}, slice 1 ${JSON.stringify(changed.slices[0].line)}`);
  console.log(`1 MiB cache: ${starved.replies} replies, slice 1 ${JSON.stringify(starved.first.line)}`);
  console.log(`tiles: during ${tiles.during.length} blank expected, after ${tiles.after.length} tiles, ${tiles.log.length} labels seen`);
  const problems = seriesProblems(seen).concat(
    refusedProblems(refused),
    linkedProblems(linked),
    restartProblems(restart),
    evictedProblems(evicted),
    changedProblems(changed),
    starvedProblems(starved),
    tilesBeforeSeriesWindowProblems(tiles),
    tilesBeforeSeriesWindowProblems(tilesScroll),
    failedSeriesWindowTileProblems(tilesFailed),
    seriesSwitchDuringWaitProblems(seriesSwitch),
  );
  if (problems.length) {
    for (const problem of problems) console.error(problem);
    process.exitCode = 1;
    return;
  }
  console.log("a series that stores no window is drawn at one automatic window, labelled as the reply names it");
}

if (process.argv[1] === fileURLToPath(import.meta.url)) {
  main().catch((error) => {
    console.error(error);
    process.exitCode = 1;
  });
}
