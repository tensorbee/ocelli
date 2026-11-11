// F-213 AC13 and lead amendment 2. The window rows on the number keys, the
// file window index in the cache key, and the region window.
//
//   CT     three 32 by 32 slices, slope 1, intercept -1024. Columns 0 to 15
//          store HU -800 and columns 16 to 31 store HU 40. Slices 1 and 2
//          store Window Center `40\-600`, Window Width `400\1500` and
//          Window Center & Width Explanation `WINDOW1\WINDOW2`. Slice 3
//          stores one window, 40 and 400, explained `ONLY`.
//   MR     one slice storing one window, 500 and 1000, with no explanation.
//
// PS3.3 C.11.2.1.2.1 LINEAR, `c' = c - 0.5`, `w' = w - 1`, then
// `y = ((x - c') / w' + 0.5) * 255` inside the window, and `display_index`
// addresses the largest n with `n <= y + 0.5`. At HU 40 and HU -800:
//
//   soft tissue and WINDOW1, 40 and 400   128 and 0
//   lung and WINDOW2, -600 and 1500       236 and 94
//   bone, 300 and 1500                    83 and 0
//
// On the CT, keys 1, 2 and 3 are the presets and 4 and 5 the file windows.
// Key 5 after key 4 must draw lung grey under WINDOW2. A cache key without
// the file window index would hand back key 4's grey. Two arrow presses
// from WINDOW2 reach slice 3, whose list is shorter, and it shows ONLY with
// no fault. On the MR, key 1 is its one file window and key 2 does nothing.
// A region drag over columns 8 to 24 spans HU -800 to 40, so
// `windowRegion` gives centre -380 and width 840. When the library build is
// not served the drag must say so instead.
//
// Review pass 1, D1 and D2. An enhanced CT of two frames carries a Frame VOI
// LUT item per frame. Frame 0 stores `40\-600` and `400\1500` explained
// `F0A\F0B`, frame 1 stores 300 and 1500 explained `F1ONLY`. Key 5 on frame
// 0 is F0B, grey 236 and 94. One ArrowDown reaches frame 1, whose own list
// holds one window, so it must show F1ONLY at bone, grey 83 and 0, and no
// fault at any sampled moment on the way. A request that sent index 1 for
// frame 1 before its list was known is the window error, which shows as a
// fault. By then the prefetch holds both frames and the process has dropped
// the posted bytes, so the frame's list must come from `/inspect` by id
// with no `POST /file`.
//
// Review pass 3, D2. A three-frame enhanced CT whose frame 0 has no Frame
// VOI LUT, so it takes the top-level `40\-600`, `400\1500`, `TOPA\TOPB`,
// while frames 1 and 2 carry 300 and 1500 explained `F1ONLY`. Key 5 on frame
// 0 is TOPB, grey 236 and 94. ArrowDown must name F1ONLY at grey 83 and 0,
// never TOPB beside frame 1's own window and never a fault.
//
// Review pass 2, D2. In Four images every tile follows the single view's
// rule. A four-frame enhanced CT stores `40\-600` and `400\1500` in every
// frame, explained `Ak\Bk` for frame k. Key 5 then Shift G must draw all
// four tiles at lung, grey 236 and 94, each labelled with its own frame's
// B name, once each frame's list is known.
//
// F-214 review pass 1, D1 and D4. The mouse profile control. A choice made
// on the control and then W must pick the W/L tool and keep the chosen
// profile and its stored key. A tool key pressed while the control has the
// focus belongs to the control, so P must not pick Pan there, and no option
// starts with P. After a reload on Window on left the active tool button
// must be W/L before any folder is opened.
//
// Review pass 2, S1. A choice after a key on the control is a keyboard
// choice and keeps the focus there. A key, then a press on the control, then
// a choice is a pointer choice, so the control is blurred and W picks W/L
// with the profile and its stored key kept.
//
// The files are written by this script into a temporary folder, carry no
// patient attributes, and are removed afterwards. It starts the viewer with
// OCELLI_PORT=0, reads the URL it prints and stops it by its PID, through
// tools/bench's pinned Playwright. `bin/ocelli.sh gate viewer-browser` runs
// it after forget.mjs.

import { Buffer } from "node:buffer";
import { mkdtempSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { fileURLToPath } from "node:url";

import { ensureViewerBinary, longEl, shortEl, startViewer } from "../../../../tools/bench/src/viewer_work.mjs";
import { loadPlaywright, stopByPid } from "./aspect.mjs";

export const LIBRARY_MISSING = "The region window needs the library build. Run npm run build -w @ocelli/core in the repository, then start the viewer again.";

/** What each step must show: the corner word, and the grey at HU 40 and HU -800. */
export const EXPECTED = [
  { name: "key 1 on the CT", word: "Soft tissue", soft: 128, lung: 0 },
  { name: "key 2 on the CT", word: "Lung", soft: 236, lung: 94 },
  { name: "key 3 on the CT", word: "Bone", soft: 83, lung: 0 },
  { name: "key 4 on the CT", word: "WINDOW1", soft: 128, lung: 0 },
  { name: "key 5 on the CT", word: "WINDOW2", soft: 236, lung: 94 },
  { name: "two slices on, a list of one", word: "ONLY", soft: 128, lung: 0 },
  { name: "key 1 on the MR", word: "Window 1" },
  { name: "key 2 on the MR", word: "Window 1" },
];

/**
 * `seen` is `{ before, after, faults, posts }`: the corner line and greys
 * after key 5 and after one ArrowDown, the faults sampled between them, and
 * the `POST /file` requests made after the first image was drawn.
 */
/** The two multiframe files: frame 0's first and chosen names, frame 1's. */
export const MULTIFRAME = { name: "per-frame multiframe", first: "F0A", chosen: "F0B", next: "F1ONLY", frames: 2 };
export const LATE = { name: "late per-frame multiframe", first: "TOPA", chosen: "TOPB", next: "F1ONLY", frames: 3 };

export function multiframeProblems(seen, want = MULTIFRAME) {
  const problems = [];
  if (!seen || !seen.before || !seen.after) {
    problems.push(`the ${want.name} case was not observed`);
    return problems;
  }
  if (!seen.before.line.endsWith("  " + want.chosen) || seen.before.soft !== 236 || seen.before.lung !== 94) {
    problems.push(`${want.name}: key 5 on frame 0 showed ${JSON.stringify(seen.before.line)}, grey ${seen.before.soft} and ${seen.before.lung}, wanted ${want.chosen} with 236 and 94`);
  }
  if (!seen.after.line.endsWith("  " + want.next) || seen.after.soft !== 83 || seen.after.lung !== 0) {
    problems.push(`${want.name}: frame 1 showed ${JSON.stringify(seen.after.line)}, grey ${seen.after.soft} and ${seen.after.lung}, wanted ${want.next} with 83 and 0`);
  }
  if (seen.faults.length) {
    problems.push(`${want.name}: frame 1 showed the fault ${JSON.stringify(seen.faults[0])} before its own window list arrived`);
  }
  if (seen.posts !== 0) {
    problems.push(`${want.name}: one ArrowDown on a fully held multiframe posted the file ${seen.posts} times`);
  }
  return problems;
}

/** `tiles` is one `{ label, soft, lung }` per tile of the four-frame file. */
export function tileProblems(tiles) {
  const problems = [];
  if (!Array.isArray(tiles) || tiles.length !== 4) {
    problems.push("the four tiles were not observed");
    return problems;
  }
  tiles.forEach((tile, k) => {
    if (!String(tile.label).endsWith("  B" + k) || tile.soft !== 236 || tile.lung !== 94) {
      problems.push(`tile ${k + 1} showed ${JSON.stringify(tile.label)}, grey ${tile.soft} and ${tile.lung}, wanted B${k} with 236 and 94`);
    }
  });
  return problems;
}

/**
 * Review pass 4, S2. `tiles` is the four tiles read while every `/inspect`
 * is held back. Tile 1 is frame 0, whose list the scan gave, so it is B0 at
 * lung, grey 236 and 94. Tiles 2 to 4 are drawn at their frame's first
 * usable window, centre 40 width 400, which gives HU 40 grey 128 and HU -800
 * grey 0, and their labels must give those numbers.
 */
export function interimProblems(tiles) {
  const problems = [];
  if (!Array.isArray(tiles) || tiles.length !== 4) {
    problems.push("the interim tiles were not observed");
    return problems;
  }
  tiles.forEach((tile, k) => {
    const want = k === 0
      ? { tail: "  B0", soft: 236, lung: 94 }
      : { tail: "  WW 400  WL 40", soft: 128, lung: 0 };
    if (!String(tile.label).endsWith(want.tail) || tile.soft !== want.soft || tile.lung !== want.lung) {
      problems.push(`while its list was on its way tile ${k + 1} showed ${JSON.stringify(tile.label)}, grey ${tile.soft} and ${tile.lung}, wanted ${want.tail.trim()} with ${want.soft} and ${want.lung}`);
    }
  });
  return problems;
}

/**
 * `seen` is `{ steps, mrRows, region }`. Each step is `{ name, corner, soft,
 * lung, fault }`. `region` is `{ served, corner, sentence }`.
 */
export function windowProblems(seen) {
  const problems = [];
  if (!seen || !Array.isArray(seen.steps) || seen.steps.length !== EXPECTED.length) {
    problems.push("the window steps were not all observed");
    return problems;
  }
  EXPECTED.forEach((want, at) => {
    const got = seen.steps[at];
    if (got.fault) {
      problems.push(`${want.name}: the cell shows the fault ${got.fault}`);
      return;
    }
    if (!String(got.corner || "").split("\n")[0].endsWith("  " + want.word)) {
      problems.push(`${want.name}: the corner reads ${JSON.stringify(got.corner)}, not the word ${want.word}`);
    }
    if (want.soft !== undefined && (got.soft !== want.soft || got.lung !== want.lung)) {
      problems.push(`${want.name}: grey ${got.soft} and ${got.lung}, wanted ${want.soft} and ${want.lung}`);
    }
  });
  if (seen.mrRows !== 1) {
    problems.push(`the MR lists ${seen.mrRows} window rows, wanted its one file window and no preset`);
  }
  const region = seen.region;
  if (!region) {
    problems.push("the region window was not observed");
  } else if (region.served) {
    if (!String(region.corner || "").startsWith("WW 840  WL -380  region")) {
      problems.push(`the region window reads ${JSON.stringify(region.corner)}, wanted WW 840 WL -380 region`);
    }
  } else if (region.sentence !== LIBRARY_MISSING) {
    problems.push(`with no library build the region window said ${JSON.stringify(region.sentence)}`);
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

/** One 32 by 32 mono16 slice, explicit VR little endian, no patient attributes. */
function slice({ sop, series, number, instance, modality, centers, widths, explanation }) {
  const sopClass = "1.2.840.10008.5.1.4.1.1.2";
  const meta = Buffer.concat([
    longEl(0x0002, 0x0001, "OB", Buffer.from([0, 1])),
    shortEl(0x0002, 0x0002, "UI", uid(sopClass)),
    shortEl(0x0002, 0x0003, "UI", uid(sop)),
    shortEl(0x0002, 0x0010, "UI", uid("1.2.840.10008.1.2.1")),
    shortEl(0x0002, 0x0012, "UI", uid("2.25.213")),
  ]);
  // Stored = HU + 1024: 224 is HU -800 and 1064 is HU 40.
  const pixels = Buffer.alloc(32 * 32 * 2);
  for (let at = 0; at < 32 * 32; at += 1) pixels.writeUInt16LE(at % 32 < 16 ? 224 : 1064, at * 2);
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
    shortEl(0x0028, 0x0010, "US", u16(32)),
    shortEl(0x0028, 0x0011, "US", u16(32)),
    shortEl(0x0028, 0x0100, "US", u16(16)),
    shortEl(0x0028, 0x0101, "US", u16(16)),
    shortEl(0x0028, 0x0102, "US", u16(15)),
    shortEl(0x0028, 0x0103, "US", u16(0)),
    shortEl(0x0028, 0x1050, "DS", text(centers)),
    shortEl(0x0028, 0x1051, "DS", text(widths)),
    shortEl(0x0028, 0x1052, "DS", text("-1024")),
    shortEl(0x0028, 0x1053, "DS", text("1")),
    ...(explanation ? [shortEl(0x0028, 0x1055, "LO", text(explanation))] : []),
    longEl(0x7fe0, 0x0010, "OW", pixels),
  ]);
}

function item(content) {
  return Buffer.concat([u16(0xfffe), u16(0xe000), u32(content.length), content]);
}

/** Two 32 by 32 frames with a Frame VOI LUT item each, no patient attributes. */
/**
 * One enhanced CT of 32 by 32 frames. A `null` frame has an empty per-frame
 * item, so it falls back to `top`, the top-level window when one is given.
 */
function enhanced(frames, top) {
  const sopClass = "1.2.840.10008.5.1.4.1.1.2.1";
  const sop = "2.25.213030";
  const meta = Buffer.concat([
    longEl(0x0002, 0x0001, "OB", Buffer.from([0, 1])),
    shortEl(0x0002, 0x0002, "UI", uid(sopClass)),
    shortEl(0x0002, 0x0003, "UI", uid(sop)),
    shortEl(0x0002, 0x0010, "UI", uid("1.2.840.10008.1.2.1")),
    shortEl(0x0002, 0x0012, "UI", uid("2.25.213")),
  ]);
  const one = Buffer.alloc(32 * 32 * 2);
  for (let at = 0; at < 32 * 32; at += 1) one.writeUInt16LE(at % 32 < 16 ? 224 : 1064, at * 2);
  const shared = item(longEl(0x0028, 0x9145, "SQ", item(Buffer.concat([
    shortEl(0x0028, 0x1052, "DS", text("-1024")),
    shortEl(0x0028, 0x1053, "DS", text("1")),
  ]))));
  const perFrame = Buffer.concat(frames.map((win) => (win == null ? item(Buffer.alloc(0)) : item(longEl(0x0028, 0x9132, "SQ", item(Buffer.concat([
    shortEl(0x0028, 0x1050, "DS", text(win.centers)),
    shortEl(0x0028, 0x1051, "DS", text(win.widths)),
    shortEl(0x0028, 0x1055, "LO", text(win.explanation)),
  ])))))));
  const topWindow = top ? [
    shortEl(0x0028, 0x1050, "DS", text(top.centers)),
    shortEl(0x0028, 0x1051, "DS", text(top.widths)),
    shortEl(0x0028, 0x1055, "LO", text(top.explanation)),
  ] : [];
  return Buffer.concat([
    Buffer.alloc(128),
    Buffer.from("DICM"),
    shortEl(0x0002, 0x0000, "UL", u32(meta.length)),
    meta,
    shortEl(0x0008, 0x0016, "UI", uid(sopClass)),
    shortEl(0x0008, 0x0018, "UI", uid(sop)),
    shortEl(0x0008, 0x0060, "CS", text("CT")),
    shortEl(0x0020, 0x000e, "UI", uid("2.25.21303")),
    shortEl(0x0020, 0x0011, "IS", text("3")),
    shortEl(0x0020, 0x0013, "IS", text("1")),
    shortEl(0x0028, 0x0002, "US", u16(1)),
    shortEl(0x0028, 0x0004, "CS", text("MONOCHROME2")),
    shortEl(0x0028, 0x0008, "IS", text(String(frames.length))),
    shortEl(0x0028, 0x0010, "US", u16(32)),
    shortEl(0x0028, 0x0011, "US", u16(32)),
    shortEl(0x0028, 0x0100, "US", u16(16)),
    shortEl(0x0028, 0x0101, "US", u16(16)),
    shortEl(0x0028, 0x0102, "US", u16(15)),
    shortEl(0x0028, 0x0103, "US", u16(0)),
    ...topWindow,
    longEl(0x5200, 0x9229, "SQ", shared),
    longEl(0x5200, 0x9230, "SQ", perFrame),
    longEl(0x7fe0, 0x0010, "OW", Buffer.concat(frames.map(() => one))),
  ]);
}

function writeTiles(folder) {
  const frames = [0, 1, 2, 3].map((k) => ({ centers: "40\\-600", widths: "400\\1500", explanation: `A${k}\\B${k}` }));
  writeFileSync(join(folder, "tiles.dcm"), enhanced(frames));
}

function readTiles(page) {
  return page.evaluate(() => Array.from(document.querySelectorAll(".cell.active .tiles .tile")).map((tile) => {
    const canvas = tile.querySelector("canvas");
    const label = tile.querySelector(".tile-label").textContent;
    if (!canvas || canvas.width !== 32) return { label, soft: null, lung: null };
    const context = canvas.getContext("2d");
    return { label, soft: context.getImageData(24, 16, 1, 1).data[0], lung: context.getImageData(8, 16, 1, 1).data[0] };
  }));
}

/** The tiles while every `/inspect` is held back for `delay` ms, then settled. */
export async function observeTiles(url, chromium, folder, delay = 0) {
  const browser = await chromium.launch({ headless: true });
  try {
    const page = await browser.newPage({ viewport: { width: 1400, height: 900 } });
    if (delay > 0) {
      await page.route("**/inspect", async (route) => {
        await page.waitForTimeout(delay);
        await route.continue();
      });
    }
    await page.goto(url);
    await page.locator("#folder").setInputFiles(folder);
    await page.waitForFunction(() => (document.getElementById("status").textContent || "").startsWith("1 series."));
    await settle(page, { word: "A0" });
    await page.keyboard.press("5");
    await settle(page, { word: "B0" });
    await page.keyboard.press("Shift+G");
    let interim = null;
    if (delay > 0) {
      // Every tile is drawn before any held-back list is answered.
      await page.waitForFunction(() => Array.from(document.querySelectorAll(".cell.active .tiles canvas"))
        .every((node) => node.width === 32), null, { timeout: Math.max(500, delay - 300) }).catch(() => {});
      interim = await readTiles(page);
    }
    await page.waitForFunction(() => Array.from(document.querySelectorAll(".cell.active .tile-label"))
      .every((node, k) => (node.textContent || "").endsWith("  B" + k)), null, { timeout: 8000 }).catch(() => {});
    await page.waitForTimeout(300);
    const settled = await readTiles(page);
    return delay > 0 ? { interim, settled } : settled;
  } finally {
    await browser.close();
  }
}

function writeMultiframe(folder) {
  writeFileSync(join(folder, "enhanced.dcm"), enhanced([
    { centers: "40\\-600", widths: "400\\1500", explanation: "F0A\\F0B" },
    { centers: "300", widths: "1500", explanation: "F1ONLY" },
  ]));
}

function writeLate(folder) {
  const own = { centers: "300", widths: "1500", explanation: "F1ONLY" };
  writeFileSync(join(folder, "late.dcm"), enhanced([null, own, own], { centers: "40\\-600", widths: "400\\1500", explanation: "TOPA\\TOPB" }));
}

/** Poll `GET /held` until the process holds every frame and no posted bytes. */
async function waitFullyHeld(page, url, frames) {
  const deadline = Date.now() + 8000;
  for (;;) {
    const held = await (await page.request.get(url + "held")).json();
    if (held.files === 0 && held.frames === frames) return;
    if (Date.now() > deadline) {
      throw new Error(`the process never held all ${frames} frames without the posted bytes: ${JSON.stringify(held)}`);
    }
    await page.waitForTimeout(100);
  }
}

export async function observeMultiframe(url, chromium, folder, want = MULTIFRAME) {
  const browser = await chromium.launch({ headless: true });
  try {
    const page = await browser.newPage({ viewport: { width: 1400, height: 900 } });
    let drawn = false;
    let posts = 0;
    page.on("request", (request) => {
      if (drawn && request.method() === "POST" && new URL(request.url()).pathname === "/file") posts += 1;
    });
    await page.goto(url);
    await page.locator("#folder").setInputFiles(folder);
    await page.waitForFunction(() => (document.getElementById("status").textContent || "").startsWith("1 series."));
    await settle(page, { word: want.first });
    drawn = true;
    await page.keyboard.press("5");
    await settle(page, { word: want.chosen });
    // The prefetch holds every frame, and the process then drops the bytes.
    // A run that never reaches that state fails here rather than passing.
    await waitFullyHeld(page, url, want.frames);
    const first = await sample(page);
    const before = { line: first.corner.split("\n")[0], soft: first.soft, lung: first.lung };
    await page.evaluate(() => {
      window.__faults = [];
      const cell = document.querySelector(".cell.active");
      const fault = cell.querySelector(".fault");
      const watch = () => {
        if (!fault.hidden && fault.textContent) window.__faults.push(fault.textContent);
      };
      new window.MutationObserver(watch).observe(cell, { subtree: true, childList: true, attributes: true, characterData: true });
      window.__faultTimer = window.setInterval(watch, 5);
    });
    await page.keyboard.press("ArrowDown");
    await settle(page, { word: want.next });
    await page.waitForTimeout(400);
    const faults = await page.evaluate(() => {
      window.clearInterval(window.__faultTimer);
      return window.__faults;
    });
    const last = await sample(page);
    const after = { line: last.corner.split("\n")[0], soft: last.soft, lung: last.lung };
    return { before, after, faults, posts };
  } finally {
    await browser.close();
  }
}

function writeFolder(folder) {
  const ct = [
    { centers: "40\\-600", widths: "400\\1500", explanation: "WINDOW1\\WINDOW2" },
    { centers: "40\\-600", widths: "400\\1500", explanation: "WINDOW1\\WINDOW2" },
    { centers: "40", widths: "400", explanation: "ONLY" },
  ];
  ct.forEach((windows, k) => {
    writeFileSync(join(folder, `ct-${k}.dcm`), slice({
      sop: `2.25.21301${k}`,
      series: "2.25.21301",
      number: 1,
      instance: k + 1,
      modality: "CT",
      ...windows,
    }));
  });
  writeFileSync(join(folder, "mr-0.dcm"), slice({
    sop: "2.25.213020",
    series: "2.25.21302",
    number: 2,
    instance: 1,
    modality: "MR",
    centers: "500",
    widths: "1000",
    explanation: "",
  }));
}

/** The active cell's corner, its fault, and the grey at image (24, 16) and (8, 16). */
function sample(page) {
  return page.evaluate(() => {
    const cell = document.querySelector(".cell.active");
    const corner = cell.querySelector(".overlay.br").textContent;
    const faultNode = cell.querySelector(".fault");
    const fault = faultNode && !faultNode.hidden ? faultNode.textContent : "";
    const canvas = cell.querySelector("canvas.view");
    let soft = null;
    let lung = null;
    if (canvas && canvas.width === 32) {
      const context = canvas.getContext("2d");
      soft = context.getImageData(24, 16, 1, 1).data[0];
      lung = context.getImageData(8, 16, 1, 1).data[0];
    }
    return { corner, fault, soft, lung };
  });
}

async function settle(page, want) {
  await page.waitForFunction((word) => {
    const cell = document.querySelector(".cell.active");
    const line = (cell.querySelector(".overlay.br").textContent || "").split("\n")[0];
    const fault = cell.querySelector(".fault");
    return line.endsWith("  " + word) || (fault && !fault.hidden);
  }, want.word, { timeout: 5000 }).catch(() => {});
  // The corner can move before the grey of the same reply is painted.
  await page.waitForTimeout(150);
}

async function press(page, key, want) {
  await page.keyboard.press(key);
  await settle(page, want);
  return { name: want.name, ...(await sample(page)) };
}

async function dragImage(page, from, to) {
  const box = await page.locator(".cell.active canvas.view").boundingBox();
  const at = (point) => [box.x + (point[0] / 32) * box.width, box.y + (point[1] / 32) * box.height];
  const [x0, y0] = at(from);
  const [x1, y1] = at(to);
  await page.mouse.move(x0, y0);
  await page.mouse.down();
  await page.mouse.move((x0 + x1) / 2, (y0 + y1) / 2, { steps: 4 });
  await page.mouse.move(x1, y1, { steps: 4 });
  await page.mouse.up();
}

export async function observeWindows(url, chromium, folder) {
  const browser = await chromium.launch({ headless: true });
  try {
    const page = await browser.newPage({ viewport: { width: 1400, height: 900 } });
    await page.goto(url);
    await page.locator("#folder").setInputFiles(folder);
    await page.waitForFunction(() => (document.getElementById("status").textContent || "").startsWith("2 series."));
    await settle(page, EXPECTED[3]);
    const steps = [];
    for (const [key, at] of [["1", 0], ["2", 1], ["3", 2], ["4", 3], ["5", 4]]) {
      steps.push(await press(page, key, EXPECTED[at]));
    }
    await page.keyboard.press("ArrowDown");
    steps.push(await press(page, "ArrowDown", EXPECTED[5]));
    const served = await page.evaluate(async () => (await window.fetch("/lib/ocelli-core/index.js")).ok);
    await page.keyboard.press("y");
    await dragImage(page, [8, 8], [24, 24]);
    let region;
    if (served) {
      await page.waitForFunction(() => (document.querySelector(".cell.active .overlay.br").textContent || "").includes("region"), null, { timeout: 8000 }).catch(() => {});
      region = { served, corner: (await sample(page)).corner };
    } else {
      await page.waitForFunction((words) => document.getElementById("sentence").textContent === words, LIBRARY_MISSING, { timeout: 8000 }).catch(() => {});
      region = { served, sentence: await page.evaluate(() => document.getElementById("sentence").textContent) };
    }
    await page.keyboard.press("s");
    await page.locator("#series-list button", { hasText: "#2" }).click();
    await settle(page, EXPECTED[6]);
    steps.push(await press(page, "1", EXPECTED[6]));
    steps.push(await press(page, "2", EXPECTED[7]));
    const mrRows = await page.evaluate(() => document.querySelectorAll("#window-list button").length);
    return { steps, mrRows, region };
  } finally {
    await browser.close();
  }
}

// Waits until no request is in flight and the main pump is idle.
async function settled(page) {
  await page.waitForFunction(() => inflight.size === 0 && !activeViewport().mainBusy, null, { timeout: 8000 });
  await page.waitForTimeout(50);
}

// A reply fetched for Lung while Bone was pressed must not be painted under
// the Bone label (PS3.3 C.11.2.1.2.1 gold, Bone 83 and 0, Lung 236 and 94).
export async function observeOutOfOrderPresets(url, chromium, folder) {
  const browser = await chromium.launch({ headless: true });
  try {
    const page = await browser.newPage({ viewport: { width: 1400, height: 900 } });
    await page.goto(url);
    await page.locator("#folder").setInputFiles(folder);
    await page.waitForFunction(() => (document.getElementById("status").textContent || "").startsWith("2 series."));
    await settle(page, EXPECTED[3]);
    let held = 0;
    let release;
    const gate = new Promise((r) => { release = r; });
    let armed = true;
    let heldPreset = '';
    await page.route('**/preview', (route) => {
      const req = route.request();
      const preset = req.headers()['x-ocelli-window-preset'] || '';
      if (armed && req.url().endsWith('/preview') && preset) {
        armed = false;
        heldPreset = preset;
        held += 1;
        return gate.then(() => route.continue());
      }
      return route.continue();
    });
    await page.keyboard.press('2');
    await page.waitForTimeout(200);
    await page.keyboard.press('3');
    await settle(page, EXPECTED[2]);
    let waited = 0;
    while (held < 1 && waited < 3000) { await page.waitForTimeout(25); waited += 25; }
    const heldUrlPromise = page.waitForResponse((r) => r.url().endsWith('/preview') && (r.request().headers()['x-ocelli-window-preset'] || '') === heldPreset);
    release();
    await heldUrlPromise;
    await settled(page);
    await settle(page, EXPECTED[2]);
    return { ...(await sample(page)), held, heldPreset };
  } finally {
    await browser.close();
  }
}

function outOfOrderPresetProblems(seen) {
  const problems = [];
  if (seen.held !== 1) problems.push('the held preset case did not hold exactly one preview request');
  if (!seen.heldPreset) problems.push('the held preset case did not record an x-ocelli-window-preset request header');
  const corner = String(seen.corner).split("\n")[0];
  if (!corner.endsWith('  ' + EXPECTED[2].word) || seen.soft !== EXPECTED[2].soft || seen.lung !== EXPECTED[2].lung) {
    problems.push(`the held preset corner is ${JSON.stringify(corner)} with greys ${seen.soft} and ${seen.lung}, not the Bone line with greys ${EXPECTED[2].soft} and ${EXPECTED[2].lung}`);
  }
  return problems;
}

export async function observeHeldPresetThenReset(url, chromium, folder) {
  const browser = await chromium.launch({ headless: true });
  try {
    const page = await browser.newPage({ viewport: { width: 1400, height: 900 } });
    await page.goto(url);
    await page.locator("#folder").setInputFiles(folder);
    await page.waitForFunction(() => (document.getElementById("status").textContent || "").startsWith("2 series."));
    await settle(page, EXPECTED[3]);
    let held = 0;
    let heldPreset = '';
    let release;
    const gate = new Promise((r) => { release = r; });
    let armed = true;
    await page.route('**/preview', (route) => {
      const req = route.request();
      const preset = req.headers()['x-ocelli-window-preset'] || '';
      if (armed && req.url().endsWith('/preview') && preset) {
        armed = false;
        heldPreset = preset;
        held += 1;
        return gate.then(() => route.continue());
      }
      return route.continue();
    });
    await page.keyboard.press('2');
    let waited = 0;
    while (held < 1 && waited < 3000) { await page.waitForTimeout(25); waited += 25; }
    await page.click('#reset');
    await settle(page, EXPECTED[3]);
    const done = page.waitForResponse((r) => r.url().endsWith('/preview') && (r.request().headers()['x-ocelli-window-preset'] || '') === heldPreset);
    release();
    await done;
    await settled(page);
    return { ...(await sample(page)), held, heldPreset };
  } finally {
    await browser.close();
  }
}

export async function observeHeldPresetDuringDrag(url, chromium, folder) {
  const browser = await chromium.launch({ headless: true });
  try {
    const page = await browser.newPage({ viewport: { width: 1400, height: 900 } });
    await page.goto(url);
    await page.locator("#folder").setInputFiles(folder);
    await page.waitForFunction(() => (document.getElementById("status").textContent || "").startsWith("2 series."));
    await settle(page, EXPECTED[3]);
    let held = 0;
    let heldPreset = '';
    let release;
    const gate = new Promise((r) => { release = r; });
    let armed = true;
    await page.route('**/preview', (route) => {
      const req = route.request();
      const preset = req.headers()['x-ocelli-window-preset'] || '';
      if (armed && req.url().endsWith('/preview') && preset) {
        armed = false;
        heldPreset = preset;
        held += 1;
        return gate.then(() => route.continue());
      }
      return route.continue();
    });
    await page.keyboard.press('2');
    let waited = 0;
    while (held < 1 && waited < 3000) { await page.waitForTimeout(25); waited += 25; }
    await page.keyboard.press('w');
    // Drag the active cell while the preset request is still held.
    const box = await page.locator('.cell.active canvas.view').boundingBox();
    await page.mouse.move(box.x + box.width / 2, box.y + box.height / 2);
    await page.mouse.down();
    for (let step = 1; step <= 6; step += 1) {
      await page.mouse.move(box.x + box.width / 2 + step * 10, box.y + box.height / 2);
    }
    const done = page.waitForResponse((r) => r.url().endsWith('/preview') && (r.request().headers()['x-ocelli-window-preset'] || '') === heldPreset);
    release();
    await done;
    await settled(page);
    await page.mouse.up();
    await settled(page);
    return { held, ...(await sample(page)) };
  } finally {
    await browser.close();
  }
}

export function heldPresetDuringDragProblems(seen) {
  const problems = [];
  if (seen.held !== 1) {
    problems.push(`the Lung preset was held ${seen.held} times instead of once`);
  }
  const first = (seen.corner || "").split("\n")[0];
  if (first.includes("Lung")) {
    problems.push("the corner still names Lung after the drag");
  }
  if (first.startsWith("WW 1500  WL -600")) {
    problems.push("the corner still shows the preset window after the drag");
  }
  if (!first.endsWith("  manual")) {
    problems.push(`the corner's first line does not end with the manual marker: ${first}`);
  }
  return problems;
}

function heldPresetThenResetProblems(seen) {
  const problems = [];
  if (seen.held !== 1) problems.push('the held preset then reset case did not hold exactly one preview request');
  if (!seen.heldPreset) problems.push('the held preset then reset case did not record an x-ocelli-window-preset request header');
  const corner = String(seen.corner).split("\n")[0];
  if (!corner.endsWith('  ' + EXPECTED[3].word) || seen.soft !== EXPECTED[3].soft || seen.lung !== EXPECTED[3].lung) {
    problems.push(`the reset corner is ${JSON.stringify(corner)} with greys ${seen.soft} and ${seen.lung}, not the ${EXPECTED[3].word} line with greys ${EXPECTED[3].soft} and ${EXPECTED[3].lung}, because the reset window must not be replaced by the late Lung reply`);
  }
  return problems;
}

export async function observeHeldLoad(url, chromium, folder) {
  const browser = await chromium.launch({ headless: true });
  try {
    const page = await browser.newPage({ viewport: { width: 1400, height: 900 } });
    const idOf = new Map();
    await page.route("**/file", async (route) => {
      const body = route.request().postDataBuffer() || Buffer.alloc(0);
      const response = await route.fetch();
      const id = response.headers()["x-ocelli-file"] || "";
      for (const k of [0, 1, 2]) if (body.includes(Buffer.from(`2.25.21301${k}`))) idOf.set(id, k);
      await route.fulfill({ response });
    });
    const sliceOf = (req) => { const id = req.headers()["x-ocelli-file"]; if (id) return idOf.get(id);
      const b = req.postDataBuffer() || Buffer.alloc(0);
      for (const k of [0, 1, 2]) if (b.includes(Buffer.from(`2.25.21301${k}`))) return k; return undefined; };
    let held = 0;
    let release;
    const gate = new Promise((r) => { release = r; });
    await page.route("**/preview", async (route) => {
      const req = route.request();
      if (sliceOf(req) === 1 && !req.headers()["x-ocelli-window-preset"]) { held += 1; await gate; }
      await route.continue().catch(() => {});
    });
    await page.goto(url);
    await page.locator("#folder").setInputFiles(folder);
    await page.waitForFunction(() => (document.getElementById("status").textContent || "").startsWith("2 series."));
    await settle(page, EXPECTED[3]);
    await page.keyboard.press('ArrowDown');
    await page.waitForTimeout(300);
    await page.keyboard.press('3');
    await settle(page, EXPECTED[2]);
    release();
    await page.waitForResponse((r) => r.url().endsWith('/preview'), { timeout: 5000 }).catch(() => null);
    await settled(page);
    return { ...(await sample(page)), held };
  } finally {
    await browser.close();
  }
}

function heldLoadProblems(seen) {
  const problems = [];
  // The thumbnail fetch of the middle slice and its prefetch are held.
  // pumpMain's request for that slice joins the held thumbnail promise.
  // The preset's own trial request passes, so only the repaired key check keeps the stale WINDOW1 reply off the Bone label.
  if (seen.held !== 2) problems.push('the held load case did not hold exactly two preview requests');
  const corner = String(seen.corner).split("\n")[0];
  if (!corner.endsWith('  Bone') || seen.soft !== 83 || seen.lung !== 0) {
    problems.push(`the held load corner is ${JSON.stringify(corner)} with greys ${seen.soft} and ${seen.lung}, not the Bone line with greys 83 and 0, because the held slice reply fetched at WINDOW1 must not be painted under the Bone label`);
  }
  return problems;
}

/** The profile control, then a key, then a reload on Window on left. */
export async function observeProfileKeys(url, chromium) {
  const browser = await chromium.launch({ headless: true });
  try {
    const page = await browser.newPage({ viewport: { width: 1400, height: 900 } });
    await page.goto(url);
    const read = () => page.evaluate(() => ({
      profile: document.getElementById("mouse-profile").value,
      stored: document.defaultView.localStorage.getItem("ocelli.mouse"),
      tools: Array.from(document.querySelectorAll('button[id^="tool-"].active')).map((node) => node.id),
      focused: document.activeElement === document.getElementById("mouse-profile"),
    }));
    const control = page.locator("#mouse-profile");
    await control.focus();
    await control.selectOption("scroll-left");
    await page.keyboard.press("w");
    const chosen = await read();
    await control.focus();
    await page.keyboard.press("p");
    const focusedKey = await read();
    await control.selectOption("window-left");
    await page.reload();
    const reloaded = await read();
    await control.focus();
    await control.dispatchEvent("keydown", { key: "ArrowUp" });
    await control.selectOption("ocelli");
    const keyboardChoice = await read();
    await control.dispatchEvent("keydown", { key: "ArrowDown" });
    await control.dispatchEvent("pointerdown");
    await control.selectOption("scroll-left");
    await page.keyboard.press("w");
    const pressedChoice = await read();
    return { chosen, focusedKey, reloaded, keyboardChoice, pressedChoice };
  } finally {
    await browser.close();
  }
}

export function profileProblems(seen) {
  const problems = [];
  if (!seen || !seen.chosen || !seen.focusedKey || !seen.reloaded || !seen.keyboardChoice || !seen.pressedChoice) {
    problems.push("the profile control was not observed");
    return problems;
  }
  const { chosen, focusedKey, reloaded, keyboardChoice, pressedChoice } = seen;
  if (chosen.profile !== "scroll-left" || chosen.stored !== "scroll-left") {
    problems.push(`W after choosing Scroll on left left the profile ${chosen.profile}, stored ${chosen.stored}`);
  }
  if (chosen.focused || chosen.tools.join() !== "tool-wl") {
    problems.push(`W after a choice did not reach the page: focus on the control ${chosen.focused}, active ${chosen.tools.join()}`);
  }
  if (focusedKey.tools.join() !== "tool-wl" || focusedKey.profile !== "scroll-left") {
    problems.push(`P on the focused control ran a page key: active ${focusedKey.tools.join()}, profile ${focusedKey.profile}`);
  }
  if (reloaded.profile !== "window-left" || reloaded.tools.join() !== "tool-wl") {
    problems.push(`after a reload on Window on left the profile was ${reloaded.profile} and the active tool ${reloaded.tools.join()}`);
  }
  if (keyboardChoice.profile !== "ocelli" || !keyboardChoice.focused) {
    problems.push(`a choice after a key left the profile ${keyboardChoice.profile} and focus on the control ${keyboardChoice.focused}`);
  }
  if (pressedChoice.focused || pressedChoice.profile !== "scroll-left" || pressedChoice.stored !== "scroll-left" || pressedChoice.tools.join() !== "tool-wl") {
    problems.push(`W after a key, a press and a choice: focus on the control ${pressedChoice.focused}, profile ${pressedChoice.profile}, stored ${pressedChoice.stored}, active ${pressedChoice.tools.join()}`);
  }
  return problems;
}

/** Hold the region request, switch to Bone, then release and sample. */
export async function observeHeldRegionThenPreset(url, chromium, folder) {
  const browser = await chromium.launch({ headless: true });
  try {
    const page = await browser.newPage({ viewport: { width: 1400, height: 900 } });
    await page.goto(url);
    await page.locator("#folder").setInputFiles(folder);
    await page.waitForFunction(() => (document.getElementById("status").textContent || "").startsWith("2 series."));
    await settle(page, EXPECTED[3]);
    const served = await page.evaluate(async () => (await window.fetch("/lib/ocelli-core/index.js")).ok);
    if (!served) return { served: false };
    let held = 0;
    let release = null;
    const gate = new Promise((r) => release = r);
    await page.route("**/region", async (route) => {
      held += 1;
      await gate;
      await route.continue();
    });
    await page.keyboard.press("y");
    await dragImage(page, [8, 8], [24, 24]);
    const deadline = Date.now() + 5000;
    while (held < 1) {
      if (Date.now() > deadline) throw new Error("region request was never held");
      await page.waitForTimeout(50);
    }
    await page.keyboard.press("3");
    await settle(page, EXPECTED[2]);
    const done = page.waitForEvent("requestfinished", (r) => r.url().endsWith("/region"));
    release();
    await done;
    // The region reply is read and its trial queued before settled reads inflight.
    await page.waitForTimeout(300);
    await settled(page);
    return { served: true, held, ...(await sample(page)) };
  } finally {
    await browser.close();
  }
}

export function heldRegionThenPresetProblems(seen) {
  if (!seen.served) return [];
  // The preset waits behind the held region, so the reply must show Bone.
  const problems = [];
  if (seen.held !== 1) problems.push("the region request was not held");
  const line = (seen.corner || "").split("\n")[0];
  if (!line.startsWith("WW 1500  WL 300  Bone")) {
    problems.push("the corner did not show WW 1500  WL 300  Bone after the preset");
  }
  if (seen.soft !== EXPECTED[2].soft || seen.lung !== EXPECTED[2].lung) {
    problems.push("the Bone greys did not match the expected values");
  }
  return problems;
}

async function main() {
  const { chromium } = await loadPlaywright();
  const binary = await ensureViewerBinary("release", true);
  const folder = mkdtempSync(join(tmpdir(), "ocelli-f213-"));
  const multi = mkdtempSync(join(tmpdir(), "ocelli-f213-multi-"));
  const tiled = mkdtempSync(join(tmpdir(), "ocelli-f213-tiles-"));
  const late = mkdtempSync(join(tmpdir(), "ocelli-f213-late-"));
  // Each observation has its own viewer process, so `GET /held` counts that
  // observation's file alone.
  const alone = async (observe) => {
    const server = await startViewer(binary);
    try {
      return await observe(server.url);
    } finally {
      await stopByPid(server.pid);
    }
  };
  let seen;
  let heldPreset;
  let heldLoad;
  let heldPresetThenReset;
  let heldRegion;
  let heldDrag;
  let frames;
  let tiles;
  let lateFrames;
  let delayed;
  let profiles;
  try {
    writeFolder(folder);
    writeMultiframe(multi);
    writeTiles(tiled);
    writeLate(late);
    seen = await alone((url) => observeWindows(url, chromium, folder));
    heldPreset = await alone((url) => observeOutOfOrderPresets(url, chromium, folder));
    heldPresetThenReset = await alone((url) => observeHeldPresetThenReset(url, chromium, folder));
    heldRegion = await alone((url) => observeHeldRegionThenPreset(url, chromium, folder));
    heldDrag = await alone((url) => observeHeldPresetDuringDrag(url, chromium, folder));
    heldLoad = await alone((url) => observeHeldLoad(url, chromium, folder));
    frames = await alone((url) => observeMultiframe(url, chromium, multi));
    tiles = await alone((url) => observeTiles(url, chromium, tiled));
    delayed = await alone((url) => observeTiles(url, chromium, tiled, 2500));
    lateFrames = await alone((url) => observeMultiframe(url, chromium, late, LATE));
    profiles = await alone((url) => observeProfileKeys(url, chromium));
  } finally {
    rmSync(folder, { recursive: true, force: true });
    rmSync(multi, { recursive: true, force: true });
    rmSync(tiled, { recursive: true, force: true });
    rmSync(late, { recursive: true, force: true });
  }
  for (const step of seen.steps) {
    console.log(`${step.name}: ${JSON.stringify(step.corner.split("\n")[0])}, grey ${step.soft} and ${step.lung}`);
  }
  console.log(`the MR lists ${seen.mrRows} window rows`);
  console.log(heldRegion.served ? `held region then preset: ${JSON.stringify(heldRegion.corner.split("\n")[0])}, grey ${heldRegion.soft} and ${heldRegion.lung}` : "held region then preset: library missing");
  console.log(seen.region.served
    ? `region window: ${JSON.stringify(String(seen.region.corner).split("\n")[0])}`
    : `region window, no library build: ${JSON.stringify(seen.region.sentence)}`);
  console.log(`multiframe: key 5 ${JSON.stringify(frames.before.line)} grey ${frames.before.soft} and ${frames.before.lung}, ArrowDown ${JSON.stringify(frames.after.line)} grey ${frames.after.soft} and ${frames.after.lung}, ${frames.faults.length} faults seen, ${frames.posts} file posts`);
  console.log(`tiles: ${tiles.map((tile) => `${JSON.stringify(tile.label)} ${tile.soft}/${tile.lung}`).join(", ")}`);
  console.log(`tiles, lists held back: ${delayed.interim.map((tile) => `${JSON.stringify(tile.label)} ${tile.soft}/${tile.lung}`).join(", ")}`);
  console.log(`late: key 5 ${JSON.stringify(lateFrames.before.line)} grey ${lateFrames.before.soft} and ${lateFrames.before.lung}, ArrowDown ${JSON.stringify(lateFrames.after.line)} grey ${lateFrames.after.soft} and ${lateFrames.after.lung}, ${lateFrames.faults.length} faults seen, ${lateFrames.posts} file posts`);
  console.log(`profile: W after a choice ${JSON.stringify(profiles.chosen)}, P on the control ${JSON.stringify(profiles.focusedKey)}, reload ${JSON.stringify(profiles.reloaded)}, keyboard choice ${JSON.stringify(profiles.keyboardChoice)}, key then press then choice ${JSON.stringify(profiles.pressedChoice)}`);
  console.log(`out of order presets: ${JSON.stringify(heldPreset.corner.split("\n")[0])}, grey ${heldPreset.soft} and ${heldPreset.lung}, held ${heldPreset.held}`);
  console.log(`held load: ${JSON.stringify(heldLoad.corner.split("\n")[0])}, grey ${heldLoad.soft} and ${heldLoad.lung}, held ${heldLoad.held}`);
  const problems = windowProblems(seen).concat(
    heldPresetThenResetProblems(heldPresetThenReset),
    heldRegionThenPresetProblems(heldRegion),
    heldPresetDuringDragProblems(heldDrag),
    multiframeProblems(frames),
    tileProblems(tiles),
    multiframeProblems(lateFrames, LATE),
    interimProblems(delayed.interim),
    tileProblems(delayed.settled),
    profileProblems(profiles),
    outOfOrderPresetProblems(heldPreset),
    heldLoadProblems(heldLoad),
  );
  if (problems.length) {
    for (const problem of problems) console.error(problem);
    process.exitCode = 1;
    return;
  }
  console.log("the window rows, the file window index and the region window draw what they name, and the profile control keeps the chosen profile");
}

if (process.argv[1] === fileURLToPath(import.meta.url)) {
  main().catch((error) => {
    console.error(error);
    process.exitCode = 1;
  });
}
