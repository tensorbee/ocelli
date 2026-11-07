// F-224 AC13 to AC17. The page does not keep showing a file that changed on
// disk after the folder was opened.
//
//   changed   a folder of three files is opened and image 1 is drawn. File 0
//             is rewritten to the same size with one stored value changed
//             and a modification time 5 s later, and a window `focus` event
//             is dispatched. The viewport must show the changed-file
//             sentence, the process must hold one file or frame fewer and
//             answer 410 for file 0's id, and image 2 must still draw. The
//             same folder is then chosen again, and the canvas at 16 points
//             must equal the grey the process returns for the new bytes,
//             which differs from the old grey at column 5, row 2.
//   restored  file 0 is rewritten with its modification time restored. This
//             is the limit decision 11 states: after a focus the page marks
//             nothing and the old image stays. Choosing the folder again
//             shows the new bytes.
//   late      every preview reply is held after the process answered, image
//             1 is inverted, file 0 is found changed, and the replies are let
//             through. The sentence must show and no grey of file 0 may stay.
//   thumb     the file the series thumbnail was drawn from changes. The
//             thumbnail must go black and image 1 must still draw.
//   tiles     in the four-image view file 1 changes, then file 0. Each
//             changed tile must carry the sentence and the others must draw.
//   cells     two cells on two series. Activating the other cell must drop
//             the first series' count, and must find a change in its own
//             series with no focus.
//   probe     a folder of 1,693 files, one series. A click on the series row
//             must read one byte of each file, at most four at a time, in
//             under 1 s.
//
// The script counts the probe's reads by wrapping `Blob.prototype.arrayBuffer`
// before the page loads. The page reads no other blob that way.
//
// Every file is written by this script into a temporary folder, is 8 by 8
// mono16 CT with no patient attributes, and is removed afterwards. Each case
// starts its own viewer with OCELLI_PORT=0, reads the URL it prints and stops
// it by its PID, through tools/bench's pinned Playwright.
// `bin/ocelli.sh gate viewer-browser` runs it after forget.mjs.

import { Buffer } from "node:buffer";
import { mkdirSync, mkdtempSync, readFileSync, rmSync, utimesSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { fileURLToPath } from "node:url";

import { ensureViewerBinary, longEl, shortEl, startViewer } from "../../../../tools/bench/src/viewer_work.mjs";
import { loadPlaywright, stopByPid } from "./aspect.mjs";

/** The sentence decision 10 shows in place of a changed file's image. */
export const CHANGED_SENTENCE = "This file changed or was removed on disk after the folder was opened. Open the folder again to see it.";
/** What the status line ends with while one file of the active series has changed. */
export const CHANGED_STATUS = "1 file of this series changed on disk after the folder was opened.";
/** Files in the `probe` folder, the count of F-216's measured CT folder. */
export const PROBE_FILES = 1693;
/** The longest a probe of PROBE_FILES may take, in milliseconds. */
export const PROBE_LIMIT_MS = 1000;
/** Reads a probe may have in flight at once. */
export const PROBE_WIDTH = 4;
const SMALL_FILES = 3;
const EDGE = 8;
// Column 5, row 2. Its old stored value is 1000 + (21 * 7) % 200 = 1147,
// which is 123 HU under rescale -1024 and slope 1, and PS3.3 C.11.2.1.2.1's
// LINEAR window 40 / 400 gives ((123 - 39.5) / 399 + 0.5) * 255 = 180.86.
// The rewrite stores 1000, which is -24 HU and
// ((-24 - 39.5) / 399 + 0.5) * 255 = 86.92. The two greys cannot agree.
export const CHANGED_POINT = { x: 5, y: 2 };
const POINTS = [];
for (const y of [0, 2, 5, 7]) for (const x of [0, 2, 5, 7]) POINTS.push({ x, y });

function grey16(canvas, grey) {
  // `canvas` and `grey` are each one value per sampled point.
  const differ = [];
  if (!Array.isArray(canvas) || !Array.isArray(grey)) return ["has no sampled points"];
  for (let index = 0; index < POINTS.length; index += 1) {
    if (canvas[index] !== grey[index]) differ.push(`at (${POINTS[index].x}, ${POINTS[index].y}) is ${canvas[index]}, not ${grey[index]}`);
  }
  return differ;
}

/**
 * `seen` is `{ changed, restored, late, thumb, tiles, cells, probe }`.
 *
 * `changed` is `{ fault, focusCanvas, focusStatus, heldBefore, heldAfter,
 * oldStatus, otherFault, otherCanvas, otherGrey, reopenFault, reopenCanvas,
 * newGrey, oldGrey }`. `fault` is the text the viewport showed after the
 * focus, `focusCanvas` the canvas then, which is reported and not judged
 * because a fault hides it, and `focusStatus` the status line then. `heldBefore` and `heldAfter` are the files and frames
 * `GET /held` counted before and after the focus, `oldStatus` the status
 * file 0's old id answered, `other*` the fault text, canvas and process grey
 * of image 2, and `reopen*` the same for image 1 after the folder was chosen
 * again. Canvas and grey values are one per sampled point.
 *
 * `restored` is `{ fault, status, canvas, oldGrey, reopenCanvas, newGrey }`.
 *
 * `late` is `{ caught, fault, keys }`: whether a preview of file 0 was held,
 * the viewport's fault text after the release, and file 0's grey keys left.
 *
 * `thumb` is `{ before, after, fault }`, the thumbnail's red sum before and
 * after the change and image 1's fault text.
 *
 * `tiles` is `{ second, first }`, each `{ hidden, widths, labels, fault }`
 * after file 1 and then file 0 changed.
 *
 * `cells` is `{ aStatus, bQuiet, bFault, bStatus }`, the status with A's
 * change, the status once B's cell was active, and B's fault and status after
 * B's file changed and its cell was activated again.
 *
 * `probe` is `{ files, reads, bytes, peak, ms }`, the reads one series
 * selection made, their summed length, the most in flight at once and the
 * time from the first read to the last settling.
 */
export function changedProblems(seen) {
  const problems = [];
  const changed = seen && seen.changed;
  if (!changed) {
    problems.push("the changed file case was not observed");
  } else {
    if (changed.fault !== CHANGED_SENTENCE) {
      problems.push(`after file 0 changed and the window took focus the viewport showed "${changed.fault}", not the changed-file sentence`);
    }
    const focusStatus = String(changed.focusStatus || "");
    if (!focusStatus.endsWith(CHANGED_STATUS)) {
      problems.push(`after the focus the status line read "${focusStatus}", which does not count the changed file`);
    }
    if (changed.heldAfter !== changed.heldBefore - 1) {
      problems.push(`GET /held counted ${changed.heldBefore} files and frames before the change and ${changed.heldAfter} after it, not one fewer`);
    }
    if (changed.oldStatus !== 410) {
      problems.push(`file 0's id answered ${changed.oldStatus} after the change, not 410`);
    }
    if (changed.otherFault !== "") {
      problems.push(`image 2 showed "${changed.otherFault}" instead of drawing`);
    }
    for (const line of grey16(changed.otherCanvas, changed.otherGrey)) problems.push(`image 2 ${line}`);
    if (changed.reopenFault !== "") {
      problems.push(`after the folder was chosen again image 1 showed "${changed.reopenFault}"`);
    }
    for (const line of grey16(changed.reopenCanvas, changed.newGrey)) problems.push(`image 1 after the folder was chosen again ${line}`);
    const at = POINTS.findIndex((point) => point.x === CHANGED_POINT.x && point.y === CHANGED_POINT.y);
    if (!Array.isArray(changed.newGrey) || !Array.isArray(changed.oldGrey)) {
      problems.push("the greys at the changed point were not read, so the case shows nothing");
    } else if (changed.newGrey[at] === changed.oldGrey[at]) {
      problems.push(`the new grey at the changed point is the old one, ${changed.oldGrey[at]}, so the case shows nothing`);
    }
  }
  const restored = seen && seen.restored;
  if (!restored) {
    problems.push("the restored modification time case was not observed");
  } else {
    if (restored.fault !== "") {
      problems.push(`a rewrite that restored the modification time showed "${restored.fault}", which is detection the page does not claim`);
    }
    if (String(restored.status || "").includes("changed")) {
      problems.push(`a rewrite that restored the modification time was counted as changed: "${restored.status}"`);
    }
    for (const line of grey16(restored.canvas, restored.oldGrey)) problems.push(`the held image after the restored rewrite ${line}`);
    for (const line of grey16(restored.reopenCanvas, restored.newGrey)) problems.push(`image 1 after the restored rewrite and a new open ${line}`);
  }
  const late = seen && seen.late;
  if (!late) {
    problems.push("the held reply case was not observed");
  } else {
    if (!late.caught) problems.push("no preview of file 0 was held, so the late reply case shows nothing");
    if (late.fault !== CHANGED_SENTENCE) {
      problems.push(`after a held reply of the old bytes was let through the viewport showed "${late.fault}", not the changed-file sentence`);
    }
    if (late.keys !== 0) problems.push(`a held reply of the old bytes left ${late.keys} grey keys of file 0 in the cache`);
  }
  const thumb = seen && seen.thumb;
  if (!thumb) {
    problems.push("the thumbnail case was not observed");
  } else {
    if (!(thumb.before > 0)) problems.push("the thumbnail was never drawn, so the case shows nothing");
    if (thumb.after !== 0) problems.push(`the thumbnail drawn from the changed file still showed it, red sum ${thumb.after}`);
    if (thumb.fault !== "") problems.push(`image 1, whose file did not change, showed "${thumb.fault}"`);
  }
  const tiles = seen && seen.tiles;
  if (!tiles || !tiles.second || !tiles.first) {
    problems.push("the tile case was not observed");
  } else {
    const expect = [
      ["file 1 changed", tiles.second, [EDGE, 1, EDGE, EDGE]],
      ["files 0 and 1 changed", tiles.first, [1, 1, EDGE, EDGE]],
    ];
    for (const [name, state, widths] of expect) {
      if (state.hidden) problems.push(`with ${name} the tiles were hidden`);
      if (state.fault !== "") problems.push(`with ${name} the viewport showed "${state.fault}" over the tiles`);
      for (let at = 0; at < widths.length; at += 1) {
        const changed = widths[at] === 1;
        if ((state.widths || [])[at] !== widths[at]) problems.push(`with ${name} tile ${at + 1} was ${(state.widths || [])[at]} wide, not ${widths[at]}`);
        if (changed && (state.labels || [])[at] !== CHANGED_SENTENCE) problems.push(`with ${name} tile ${at + 1} read "${(state.labels || [])[at]}", not the changed-file sentence`);
        if (!changed && (state.labels || [])[at] === CHANGED_SENTENCE) problems.push(`with ${name} tile ${at + 1}, whose file did not change, read the changed-file sentence`);
      }
    }
  }
  const cells = seen && seen.cells;
  if (!cells) {
    problems.push("the two-cell case was not observed");
  } else {
    if (!String(cells.aStatus || "").endsWith(CHANGED_STATUS)) problems.push(`with A's file changed the status read "${cells.aStatus}"`);
    if (String(cells.bQuiet || "").includes("changed")) problems.push(`after B's cell became active the status read "${cells.bQuiet}" for a series with no changed file`);
    if (cells.bFault !== CHANGED_SENTENCE) problems.push(`activating B's cell after its file changed showed "${cells.bFault}", not the changed-file sentence`);
    if (!String(cells.bStatus || "").endsWith(CHANGED_STATUS)) problems.push(`activating B's cell after its file changed left the status "${cells.bStatus}"`);
  }
  const probe = seen && seen.probe;
  if (!probe) {
    problems.push("the probe case was not observed");
  } else {
    if (probe.reads !== probe.files) {
      problems.push(`a series selection made ${probe.reads} reads for ${probe.files} files`);
    }
    if (probe.bytes !== probe.files) {
      problems.push(`a series selection read ${probe.bytes} bytes for ${probe.files} files, not one each`);
    }
    if (probe.peak !== PROBE_WIDTH) {
      problems.push(`the probe had at most ${probe.peak} reads in flight, not ${PROBE_WIDTH}`);
    }
    if (!(probe.ms < PROBE_LIMIT_MS)) {
      problems.push(`the probe of ${probe.files} files took ${probe.ms} ms, not under ${PROBE_LIMIT_MS}`);
    }
  }
  return problems;
}

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

/**
 * One 8 by 8 mono16 CT slice, explicit VR little endian, no patient
 * attributes. `rewritten` stores 1000 at CHANGED_POINT and keeps the length.
 */
function slice(sop, series, instance, rewritten) {
  const sopClass = "1.2.840.10008.5.1.4.1.1.2";
  const meta = Buffer.concat([
    longEl(0x0002, 0x0001, "OB", Buffer.from([0, 1])),
    shortEl(0x0002, 0x0002, "UI", uid(sopClass)),
    shortEl(0x0002, 0x0003, "UI", uid(sop)),
    shortEl(0x0002, 0x0010, "UI", uid("1.2.840.10008.1.2.1")),
    shortEl(0x0002, 0x0012, "UI", uid("2.25.224")),
  ]);
  const pixels = Buffer.alloc(EDGE * EDGE * 2);
  for (let at = 0; at < EDGE * EDGE; at += 1) pixels.writeUInt16LE(1000 + ((at * 7) % 200), at * 2);
  if (rewritten) pixels.writeUInt16LE(1000, (CHANGED_POINT.y * EDGE + CHANGED_POINT.x) * 2);
  return Buffer.concat([
    Buffer.alloc(128),
    Buffer.from("DICM"),
    shortEl(0x0002, 0x0000, "UL", u32(meta.length)),
    meta,
    shortEl(0x0008, 0x0016, "UI", uid(sopClass)),
    shortEl(0x0008, 0x0018, "UI", uid(sop)),
    shortEl(0x0008, 0x0060, "CS", text("CT")),
    shortEl(0x0020, 0x000e, "UI", uid(series)),
    shortEl(0x0020, 0x0011, "IS", text("1")),
    shortEl(0x0020, 0x0013, "IS", text(String(instance))),
    shortEl(0x0028, 0x0002, "US", u16(1)),
    shortEl(0x0028, 0x0004, "CS", text("MONOCHROME2")),
    shortEl(0x0028, 0x0010, "US", u16(EDGE)),
    shortEl(0x0028, 0x0011, "US", u16(EDGE)),
    shortEl(0x0028, 0x0100, "US", u16(16)),
    shortEl(0x0028, 0x0101, "US", u16(16)),
    shortEl(0x0028, 0x0102, "US", u16(15)),
    shortEl(0x0028, 0x0103, "US", u16(0)),
    shortEl(0x0028, 0x1050, "DS", text("40")),
    shortEl(0x0028, 0x1051, "DS", text("400")),
    shortEl(0x0028, 0x1052, "DS", text("-1024")),
    shortEl(0x0028, 0x1053, "DS", text("1")),
    longEl(0x7fe0, 0x0010, "OW", pixels),
  ]);
}

// A whole second, so restoring it is exact at every precision the file
// system and the browser keep.
const STAMP = Math.floor(Date.now() / 1000) - 3600;

/** `count` slices of one series, each stamped STAMP. Returns the paths in instance order. */
function writeFolder(root, name, tag, count) {
  const folder = join(root, name);
  mkdirSync(folder);
  const paths = [];
  for (let k = 0; k < count; k += 1) {
    const path = join(folder, `${name}-${k}.dcm`);
    writeFileSync(path, slice(`${tag}.${k + 1}`, `${tag}.0`, k + 1, false));
    utimesSync(path, STAMP, STAMP);
    paths.push(path);
  }
  return { folder, paths, tag };
}

/** File `index` of `set` gets the rewritten bytes and the modification time `stamp`. */
function rewriteFile(set, index, stamp) {
  const path = set.paths[index];
  writeFileSync(path, slice(`${set.tag}.${index + 1}`, `${set.tag}.0`, index + 1, true));
  utimesSync(path, stamp, stamp);
}

function rewriteFirst(set, stamp) {
  rewriteFile(set, 0, stamp);
}

/** Two series of `count` files each in one folder, `a` then `b`. */
function writeTwoSeries(root, name, tagA, tagB, count) {
  const folder = join(root, name);
  mkdirSync(folder);
  const series = [];
  for (const [prefix, tag] of [["a", tagA], ["b", tagB]]) {
    const paths = [];
    for (let k = 0; k < count; k += 1) {
      const path = join(folder, `${prefix}-${k}.dcm`);
      writeFileSync(path, slice(`${tag}.${k + 1}`, `${tag}.0`, k + 1, false));
      utimesSync(path, STAMP, STAMP);
      paths.push(path);
    }
    series.push({ folder, paths, tag });
  }
  return { folder, a: series[0], b: series[1], count: 2 * count };
}

/**
 * Files plus frames `GET /held` counts. A one-frame file is held as its bytes
 * until its frame decodes and as that frame afterwards, so each of these
 * files counts once.
 */
async function heldCount(url) {
  const response = await globalThis.fetch(`${url}held`, { cache: "no-store" });
  if (!response.ok) throw new Error(`GET /held answered ${response.status}`);
  const census = await response.json();
  return census.files + census.frames;
}

async function previewStatus(url, id) {
  const response = await globalThis.fetch(`${url}preview`, {
    method: "POST",
    headers: { "X-Ocelli-File": id },
    body: "",
  });
  await response.arrayBuffer();
  return response.status;
}

/** The process's grey for frame 0 of the bytes at `path`, at the sampled points. */
async function processGrey(url, path) {
  const response = await globalThis.fetch(`${url}preview`, {
    method: "POST",
    headers: { "X-Ocelli-Frame": "0" },
    body: readFileSync(path),
  });
  if (!response.ok) throw new Error(`a preview of ${path} answered ${response.status}`);
  const width = Number(response.headers.get("x-ocelli-width"));
  const bytes = new Uint8Array(await response.arrayBuffer());
  return POINTS.map((point) => bytes[point.y * width + point.x]);
}

async function canvasGrey(page) {
  return page.evaluate((points) => {
    const node = document.querySelector('.cell[data-cell="0"] canvas.view');
    const data = node.getContext("2d").getImageData(0, 0, node.width, node.height).data;
    return points.map((point) => data[(point.y * node.width + point.x) * 4]);
  }, POINTS);
}

async function faultText(page, cell = 0) {
  return page.evaluate((at) => {
    const node = document.querySelector(`.cell[data-cell="${at}"] .fault`);
    return node && !node.hidden ? node.textContent || "" : "";
  }, cell);
}

function statusText(page) {
  return page.evaluate(() => document.getElementById("status").textContent || "");
}

/** Each tile's canvas width and label, and whether the grid is hidden, in cell 0. */
function tileState(page) {
  return page.evaluate(() => {
    const tiles = document.querySelector('.cell[data-cell="0"] .tiles');
    const canvases = Array.from(tiles.querySelectorAll("canvas"));
    const labels = Array.from(tiles.querySelectorAll(".tile-label"));
    return {
      hidden: tiles.hidden,
      widths: canvases.map((node) => node.width),
      labels: labels.map((node) => node.textContent || ""),
    };
  });
}

/** The summed red channel of the first series row's thumbnail. */
function thumbSum(page) {
  return page.evaluate(() => {
    const node = document.querySelector("#series-list button canvas");
    if (!node) return -1;
    const data = node.getContext("2d").getImageData(0, 0, node.width, node.height).data;
    let sum = 0;
    for (let at = 0; at < data.length; at += 4) sum += data[at];
    return sum;
  });
}

/** The id of each `/file` reply, in arrival order. */
function collectIds(page, sink) {
  page.on("response", (response) => {
    if (!response.url().endsWith("/file") || response.request().method() !== "POST") return;
    const id = response.headers()["x-ocelli-file"];
    if (id) sink.push(id);
  });
}

/**
 * The id the page holds for image 1's file. Read from the page's own map,
 * because Playwright keeps no body for a request that posts a `File` and may
 * evict a reply's body.
 */
function firstId(page) {
  return page.evaluate(() => {
    const series = seriesByKey.get(activeViewport().seriesKey);
    return (series && filePosts.get(series.images[0].file)) || "";
  });
}

async function until(test, what, limit = 30000) {
  const started = Date.now();
  while (!(await test())) {
    if (Date.now() - started > limit) throw new Error(`${what} did not happen within ${limit} ms`);
    await new Promise((resolve) => setTimeout(resolve, 50));
  }
}

/**
 * Opens `set` and waits for a preview of an id this open was given, and its
 * paint. A page that answers from a grey it kept from the last open asks for
 * no preview, so that wait gives up after 5 s and the canvas comparison that
 * follows names the stale grey.
 */
async function openAndDraw(page, sink, set, before) {
  const known = new Set(sink);
  const drawn = page.waitForResponse((response) => {
    const id = response.request().headers()["x-ocelli-file"];
    return response.url().endsWith("/preview") && response.status() === 200 && Boolean(id) && !known.has(id);
  }, { timeout: 5000 }).catch(() => null);
  await page.locator("#folder").setInputFiles(set.folder);
  await until(() => sink.length >= before + set.paths.length, "the folder's posts");
  await drawn;
  await page.waitForFunction(() => (document.getElementById("status").textContent || "").startsWith("1 series."));
  await until(() => page.evaluate(() => {
    const view = document.querySelector('.cell[data-cell="0"] canvas.view');
    const stage = document.querySelector('.cell[data-cell="0"] .stage');
    return Boolean(view && stage && !stage.hidden && view.width === 8);
  }), "image 1 drawn");
}

/** Counts every `Blob.prototype.arrayBuffer` call the page makes. */
function countReads() {
  const original = window.Blob.prototype.arrayBuffer;
  const state = { reads: 0, bytes: 0, inFlight: 0, peak: 0, first: 0, last: 0 };
  window.__f224Reads = state;
  window.Blob.prototype.arrayBuffer = function countedArrayBuffer() {
    state.reads += 1;
    state.bytes += this.size;
    state.inFlight += 1;
    if (state.inFlight > state.peak) state.peak = state.inFlight;
    if (!state.first) state.first = window.performance.now();
    return original.call(this).finally(() => {
      state.inFlight -= 1;
      state.last = window.performance.now();
    });
  };
}

async function resetReads(page) {
  await page.evaluate(() => {
    const state = window.__f224Reads;
    state.reads = 0;
    state.bytes = 0;
    state.peak = 0;
    state.first = 0;
    state.last = 0;
  });
}

async function readCounts(page) {
  return page.evaluate(() => ({ ...window.__f224Reads }));
}

async function quiet(page) {
  // The probe a series load starts at open has settled when no read is in
  // flight and none began for 300 ms.
  let last = -1;
  await until(async () => {
    const state = await readCounts(page);
    const settled = state.inFlight === 0 && state.reads === last;
    last = state.reads;
    if (!settled) await new Promise((resolve) => setTimeout(resolve, 300));
    return settled;
  }, "the page's reads to settle");
}

function focus(page) {
  return page.evaluate(() => window.dispatchEvent(new Event("focus")));
}

export async function observeChanged(url, chromium, set) {
  const browser = await chromium.launch({ headless: true });
  try {
    const page = await browser.newPage({ viewport: { width: 1400, height: 900 } });
    await page.addInitScript(countReads);
    const sink = [];
    collectIds(page, sink);
    await page.goto(url);
    await openAndDraw(page, sink, set, 0);
    await quiet(page);
    const oldId = await firstId(page);
    const oldGrey = await processGrey(url, set.paths[0]);
    const heldBefore = await heldCount(url);
    rewriteFirst(set, STAMP + 5);
    await focus(page);
    await until(async () => (await faultText(page)) !== "", "a fault after the focus", 5000).catch(() => {});
    await quiet(page);
    const fault = await faultText(page);
    const focusCanvas = await canvasGrey(page);
    const focusStatus = await page.evaluate(() => document.getElementById("status").textContent || "");
    const heldAfter = await heldCount(url);
    const oldStatus = oldId ? await previewStatus(url, oldId) : 0;
    await page.keyboard.press("ArrowDown");
    await until(async () => (await faultText(page)) === "" && (await page.evaluate(() => activeViewport().viewIndex)) === 1, "image 2 shown", 5000).catch(() => {});
    await page.waitForTimeout(300);
    const otherFault = await faultText(page);
    const otherCanvas = await canvasGrey(page);
    const otherGrey = await processGrey(url, set.paths[1]);
    const before = sink.length;
    await openAndDraw(page, sink, set, before);
    await quiet(page);
    const reopenFault = await faultText(page);
    const reopenCanvas = await canvasGrey(page);
    const newGrey = await processGrey(url, set.paths[0]);
    return { fault, focusCanvas, focusStatus, heldBefore, heldAfter, oldStatus, otherFault, otherCanvas, otherGrey, reopenFault, reopenCanvas, newGrey, oldGrey };
  } finally {
    await browser.close();
  }
}

export async function observeRestored(url, chromium, set) {
  const browser = await chromium.launch({ headless: true });
  try {
    const page = await browser.newPage({ viewport: { width: 1400, height: 900 } });
    await page.addInitScript(countReads);
    const sink = [];
    collectIds(page, sink);
    await page.goto(url);
    await openAndDraw(page, sink, set, 0);
    await quiet(page);
    const oldGrey = await processGrey(url, set.paths[0]);
    rewriteFirst(set, STAMP);
    await focus(page);
    await page.waitForTimeout(300);
    await quiet(page);
    const fault = await faultText(page);
    const status = await page.evaluate(() => document.getElementById("status").textContent || "");
    const canvas = await canvasGrey(page);
    const before = sink.length;
    await openAndDraw(page, sink, set, before);
    await quiet(page);
    const reopenCanvas = await canvasGrey(page);
    const newGrey = await processGrey(url, set.paths[0]);
    return { fault, status, canvas, oldGrey, reopenCanvas, newGrey };
  } finally {
    await browser.close();
  }
}

export async function observeProbe(url, chromium, set) {
  const browser = await chromium.launch({ headless: true });
  try {
    const page = await browser.newPage({ viewport: { width: 1400, height: 900 } });
    page.setDefaultTimeout(180000);
    await page.addInitScript(countReads);
    const sink = [];
    collectIds(page, sink);
    await page.goto(url);
    await page.locator("#folder").setInputFiles(set.folder);
    await until(() => sink.length >= set.paths.length, "the 1,693 posts", 180000);
    await page.waitForFunction(() => (document.getElementById("status").textContent || "").startsWith("1 series."));
    await quiet(page);
    await resetReads(page);
    await page.locator("#series-list button").first().click();
    await until(async () => (await readCounts(page)).reads >= set.paths.length, "the probe's reads", 10000).catch(() => {});
    await quiet(page);
    const state = await readCounts(page);
    return {
      files: set.paths.length,
      reads: state.reads,
      bytes: state.bytes,
      peak: state.peak,
      ms: state.reads ? Math.round((state.last - state.first) * 10) / 10 : Number.POSITIVE_INFINITY,
    };
  } finally {
    await browser.close();
  }
}

/**
 * Every preview reply is held after the process answered, while image 1 is
 * inverted, which is a new grey key. File 0 is then found changed, and the
 * replies are let through. They carry the old bytes, so none may be painted
 * or kept.
 */
export async function observeLate(url, chromium, set) {
  const browser = await chromium.launch({ headless: true });
  try {
    const page = await browser.newPage({ viewport: { width: 1400, height: 900 } });
    await page.addInitScript(countReads);
    const sink = [];
    collectIds(page, sink);
    await page.goto(url);
    await openAndDraw(page, sink, set, 0);
    await quiet(page);
    const oldId = await firstId(page);
    const serial = await page.evaluate(() => fileSerials.get(seriesByKey.get(activeViewport().seriesKey).images[0].file));
    let holding = true;
    let caught = false;
    const held = [];
    await page.route("**/preview", async (route) => {
      if (!holding) {
        await route.continue();
        return;
      }
      const response = await route.fetch();
      if (route.request().headers()["x-ocelli-file"] === oldId) caught = true;
      held.push({ route, response });
    });
    await page.keyboard.press("i");
    await until(() => caught, "a held preview of file 0", 5000).catch(() => {});
    rewriteFirst(set, STAMP + 5);
    await focus(page);
    await until(async () => (await statusText(page)).endsWith(CHANGED_STATUS), "the change counted", 5000).catch(() => {});
    holding = false;
    for (const entry of held) await entry.route.fulfill({ response: entry.response });
    await page.waitForTimeout(500);
    await quiet(page);
    const fault = await faultText(page);
    const keys = await page.evaluate((prefix) => Array.from(cache.keys()).filter((key) => key.startsWith(prefix)).length, `${serial}\0`);
    return { caught, fault, keys };
  } finally {
    await browser.close();
  }
}

/** The thumbnail is drawn from the middle file, which then changes. */
export async function observeThumb(url, chromium, set) {
  const browser = await chromium.launch({ headless: true });
  try {
    const page = await browser.newPage({ viewport: { width: 1400, height: 900 } });
    await page.addInitScript(countReads);
    const sink = [];
    collectIds(page, sink);
    await page.goto(url);
    await openAndDraw(page, sink, set, 0);
    await until(async () => (await thumbSum(page)) > 0, "the thumbnail drawn").catch(() => {});
    await quiet(page);
    const before = await thumbSum(page);
    rewriteFile(set, Math.floor(set.paths.length / 2), STAMP + 5);
    await focus(page);
    await until(async () => (await statusText(page)).endsWith(CHANGED_STATUS), "the change counted", 5000).catch(() => {});
    await quiet(page);
    const after = await thumbSum(page);
    const fault = await faultText(page);
    return { before, after, fault };
  } finally {
    await browser.close();
  }
}

/** The four-image view, file 1 changed and then file 0. */
export async function observeTiles(url, chromium, set) {
  const browser = await chromium.launch({ headless: true });
  try {
    const page = await browser.newPage({ viewport: { width: 1400, height: 900 } });
    await page.addInitScript(countReads);
    const sink = [];
    collectIds(page, sink);
    await page.goto(url);
    await openAndDraw(page, sink, set, 0);
    await quiet(page);
    await page.keyboard.press("Shift+G");
    await until(async () => (await tileState(page)).widths.every((width) => width === EDGE), "four tiles drawn").catch(() => {});
    rewriteFile(set, 1, STAMP + 5);
    await focus(page);
    await until(async () => (await tileState(page)).labels[1] === CHANGED_SENTENCE, "tile 2's sentence", 5000).catch(() => {});
    await quiet(page);
    const second = { ...(await tileState(page)), fault: await faultText(page) };
    rewriteFile(set, 0, STAMP + 5);
    await focus(page);
    await until(async () => (await tileState(page)).labels[0] === CHANGED_SENTENCE, "tile 1's sentence", 5000).catch(() => {});
    await quiet(page);
    await page.waitForTimeout(300);
    const first = { ...(await tileState(page)), fault: await faultText(page) };
    return { second, first };
  } finally {
    await browser.close();
  }
}

/**
 * Two cells on two series. A's file changes and is counted. Activating
 * B's cell must drop the count, and after B's file changes, activating B's
 * cell again must find it with no focus.
 */
export async function observeCells(url, chromium, set) {
  const browser = await chromium.launch({ headless: true });
  try {
    const page = await browser.newPage({ viewport: { width: 1400, height: 900 } });
    await page.addInitScript(countReads);
    const sink = [];
    collectIds(page, sink);
    await page.goto(url);
    const drawn = page.waitForResponse((response) => response.url().endsWith("/preview") && response.status() === 200, { timeout: 30000 });
    await page.locator("#folder").setInputFiles(set.folder);
    await until(() => sink.length >= set.count, "the folder's posts");
    await drawn;
    await page.waitForFunction(() => (document.getElementById("status").textContent || "").startsWith("2 series."));
    await page.locator("#layout").click();
    await page.locator('#layout-picker [data-columns="2"][data-rows="1"]').click();
    await until(() => page.evaluate(() => {
      const view = document.querySelector('.cell[data-cell="1"] canvas.view');
      return Boolean(view && view.width === 8);
    }), "cell 2 drawn");
    await quiet(page);
    rewriteFirst(set.a, STAMP + 5);
    await focus(page);
    await until(async () => (await statusText(page)).endsWith(CHANGED_STATUS), "A's change counted", 5000).catch(() => {});
    const aStatus = await statusText(page);
    await page.locator('.cell[data-cell="1"]').click();
    await quiet(page);
    const bQuiet = await statusText(page);
    rewriteFirst(set.b, STAMP + 5);
    await page.locator('.cell[data-cell="0"]').click();
    await quiet(page);
    await page.locator('.cell[data-cell="1"]').click();
    await until(async () => (await faultText(page, 1)) !== "", "B's sentence", 5000).catch(() => {});
    await quiet(page);
    const bFault = await faultText(page, 1);
    const bStatus = await statusText(page);
    return { aStatus, bQuiet, bFault, bStatus };
  } finally {
    await browser.close();
  }
}

async function withViewer(binary, body) {
  const server = await startViewer(binary);
  try {
    return await body(server.url);
  } finally {
    await stopByPid(server.pid);
  }
}

async function main() {
  const { chromium } = await loadPlaywright();
  const binary = await ensureViewerBinary("release", true);
  const root = mkdtempSync(join(tmpdir(), "ocelli-f224-"));
  const seen = {};
  try {
    const changedSet = writeFolder(root, "changed", "2.25.2241", SMALL_FILES);
    const restoredSet = writeFolder(root, "restored", "2.25.2242", SMALL_FILES);
    const probeSet = writeFolder(root, "probe", "2.25.2243", PROBE_FILES);
    const lateSet = writeFolder(root, "late", "2.25.2244", SMALL_FILES);
    const thumbSet = writeFolder(root, "thumb", "2.25.2245", SMALL_FILES);
    const tileSet = writeFolder(root, "tiles", "2.25.2246", 5);
    const cellSet = writeTwoSeries(root, "cells", "2.25.2247", "2.25.2248", 2);
    seen.changed = await withViewer(binary, (url) => observeChanged(url, chromium, changedSet));
    const kept = seen.changed.focusCanvas.filter((value, index) => value === seen.changed.oldGrey[index]).length;
    console.log(`changed: viewport "${seen.changed.fault}", the canvas after the focus held the old grey at ${kept} of ${POINTS.length} points, status "${seen.changed.focusStatus}", held ${seen.changed.heldBefore} then ${seen.changed.heldAfter} files and frames, old id ${seen.changed.oldStatus}, image 2 "${seen.changed.otherFault}", chosen again "${seen.changed.reopenFault}"`);
    seen.restored = await withViewer(binary, (url) => observeRestored(url, chromium, restoredSet));
    console.log(`restored: viewport "${seen.restored.fault}", status "${seen.restored.status}"`);
    seen.late = await withViewer(binary, (url) => observeLate(url, chromium, lateSet));
    console.log(`late: held ${seen.late.caught}, viewport "${seen.late.fault}", ${seen.late.keys} grey keys of file 0`);
    seen.thumb = await withViewer(binary, (url) => observeThumb(url, chromium, thumbSet));
    console.log(`thumb: red sum ${seen.thumb.before} then ${seen.thumb.after}, image 1 "${seen.thumb.fault}"`);
    seen.tiles = await withViewer(binary, (url) => observeTiles(url, chromium, tileSet));
    console.log(`tiles: file 1 changed ${seen.tiles.second.widths.join(" ")}, then file 0 ${seen.tiles.first.widths.join(" ")}, fault "${seen.tiles.first.fault}"`);
    seen.cells = await withViewer(binary, (url) => observeCells(url, chromium, cellSet));
    console.log(`cells: A "${seen.cells.aStatus}", B active "${seen.cells.bQuiet}", B changed "${seen.cells.bFault}"`);
    seen.probe = await withViewer(binary, (url) => observeProbe(url, chromium, probeSet));
    console.log(`probe: ${seen.probe.reads} reads of ${seen.probe.bytes} bytes for ${seen.probe.files} files, at most ${seen.probe.peak} in flight, ${seen.probe.ms} ms`);
  } finally {
    rmSync(root, { recursive: true, force: true });
  }
  const problems = changedProblems(seen);
  if (problems.length) {
    for (const problem of problems) console.error(problem);
    process.exitCode = 1;
    return;
  }
  console.log("a file changed on disk is refused rather than shown, choosing the folder again shows the new bytes, and the probe reads one byte a file");
}

if (process.argv[1] === fileURLToPath(import.meta.url)) {
  main().catch((error) => {
    console.error(error);
    process.exitCode = 1;
  });
}
