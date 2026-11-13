// F-204. One stored pixel per device pixel in the single showcase viewer.
//
// The suite writes its own synthetic files into a temporary folder. Seven
// fixtures, all 8-bit explicit VR little endian MONOCHROME2, no Modality LUT
// and no rescale, Window Center 128, Window Width 256, VOI LUT Function
// LINEAR, written like window.mjs's MR file so the non-CT window list has
// the file window as its one row.
//
//   P    128 by 96, no Pixel Spacing. Stored (x, y) is
//        (x * 29 + y * 57) mod 251, except (127, 95) which stores 255 so the
//        linear branch is exercised at the top boundary. Corners 0, 169,
//        144 and 255, hand-computed from the formula.
//   B    129 by 65, no Pixel Spacing, the same rule with no override. Odd
//        extents, so the drawn device box is the stored count while the
//        ideal centring is fractional at every ratio below. Corners 0, 198,
//        134 and 81.
//   C    64 by 64, no Pixel Spacing, the same rule. Corners 0, 70, 77 and
//        147. A small frame in a large cell is what makes the unmodified
//        zoom clamp floor bind.
//   D    128 by 96, Pixel Spacing 0.7\0.5, the same rule. The expected
//        device width at the identity is 128 * 0.5 / 0.7 = 91.43.
//   D2   128 by 96, Pixel Spacing 0.5\0.7, the same rule. The expected
//        device width at the identity is 179.2.
//   E    2048 by 8, no Pixel Spacing, the same rule. Its rows are longer
//        than the 251-value cycle, so values repeat along a row, which no
//        byte row consumes. Adjacent neighbours still differ everywhere.
//   F    2 by 64, no Pixel Spacing, the same rule. Below the device pixel
//        ratio at deviceScaleFactor 3 and suitable at 2. P and F are two
//        instances of one series of mixed sizes, so the move between them
//        is a scroll and not a series open.
//
// PS3.3 C.11.2.1.2.1 and its Note 4: a Window Center of 2^(n-1) and a Window
// Width of 2^n on n-bit unsigned stored values is the mathematical identity
// VOI LUT transformation, so with the files' Window Center 128 and Window
// Width 256 the expected display byte at every device pixel is the stored
// value across 0 to 255. The bottom clamp bound is exercised at the stored
// value 0 and the top clamp bound is unreachable for 8-bit input, which no
// row claims otherwise. The drawn byte is taken through display_index's
// round to nearest (crates/ocelli-render/src/colour.rs 518 via
// examples/folder-viewer/src/preview.rs 1531), which restores the identity
// for the 95 of 256 inputs whose f32 linear-branch value misses by one ulp.
//
// The pattern (x * 29 + y * 57) mod 251 is distinct along each row and along
// each column of every fixture whose longest side is at most 251, because 29
// and 57 are both coprime with 251. It is not distinct across fixtures,
// which no row consumes.
//
// The script starts the release viewer with OCELLI_PORT=0, reads the one URL
// it prints and stops it by its PID, through tools/bench's pinned
// Playwright. Playwright passes --force-color-profile=srgb by default and
// this suite does not set ignoreDefaultArgs, so the exact-byte rows read the
// same values on every host. It is registered in the viewer-browser gate
// before launcher.mjs, which stays last.
//
// The mode draws into a device-sized backing, per the lead amendment after
// implementation pass 1: the canvas element is sized in whole CSS pixels to
// its cell, the backing is that box times the device pixel ratio, and the
// page writes the stored pixels one to one at an integer device offset that
// carries the centring, the pan and the snap, with rotation and flips by
// index. The measured Chromium anchor law (scratch probes under target in
// the F-204 fix round): the painted layer origin is the static position
// rounded to a whole CSS pixel with the frame translate riding unsnapped,
// so the painted device origin is (round(static) + translate) * ratio, and
// a device-sized backing is byte-exact exactly when that origin is
// integral. The page records the painted device box it wrote, and every
// byte row crops the screenshot at that recorded origin, never by a search.

import { Buffer } from "node:buffer";
import { mkdtempSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { fileURLToPath } from "node:url";

import { ensureViewerBinary, longEl, shortEl, startViewer } from "../../../../tools/bench/src/viewer_work.mjs";
import { loadPlaywright, stopByPid } from "./aspect.mjs";

const VIEWPORT = { width: 1600, height: 1000 };
const EXTENT_EPSILON = 0.5;
const ORIGIN_SLACK = 1;
const SERIES_COUNT = 6;
const REFUSAL = "1:1 needs a frame at least the device pixel ratio big";

/** The stored value rule of the fixtures, from the pattern above. */
function stored(x, y) {
  return (x * 29 + y * 57) % 251;
}

function buildPattern(columns, rows, override) {
  const bytes = new Array(columns * rows);
  for (let y = 0; y < rows; y += 1) {
    for (let x = 0; x < columns; x += 1) {
      bytes[y * columns + x] = override && override.x === x && override.y === y ? override.value : stored(x, y);
    }
  }
  return bytes;
}

const FIXTURES = {
  p: { name: "P", columns: 128, rows: 96, override: { x: 127, y: 95, value: 255 } },
  b: { name: "B", columns: 129, rows: 65 },
  c: { name: "C", columns: 64, rows: 64 },
  d: { name: "D", columns: 128, rows: 96, spacing: "0.7\\0.5" },
  d2: { name: "D-prime", columns: 128, rows: 96, spacing: "0.5\\0.7" },
  e: { name: "E", columns: 2048, rows: 8 },
  f: { name: "F", columns: 2, rows: 64 },
};

for (const fixture of Object.values(FIXTURES)) {
  fixture.bytes = buildPattern(fixture.columns, fixture.rows, fixture.override);
}

/** The hand-computed corner values of the plan, checked against the formula. */
export function fixtureSanityProblems() {
  const problems = [];
  const at = (fixture, x, y) => fixture.bytes[y * fixture.columns + x];
  const check = (name, got, want) => {
    if (got !== want) problems.push(`${name} stores ${got}, hand-computed ${want}`);
  };
  check("P (0, 0)", at(FIXTURES.p, 0, 0), 0);
  check("P (127, 0)", at(FIXTURES.p, 127, 0), 169);
  check("P (0, 95)", at(FIXTURES.p, 0, 95), 144);
  check("P (127, 95)", at(FIXTURES.p, 127, 95), 255);
  check("B (0, 0)", at(FIXTURES.b, 0, 0), 0);
  check("B (128, 0)", at(FIXTURES.b, 128, 0), 198);
  check("B (0, 64)", at(FIXTURES.b, 0, 64), 134);
  check("B (128, 64)", at(FIXTURES.b, 128, 64), 81);
  check("C (0, 0)", at(FIXTURES.c, 0, 0), 0);
  check("C (63, 0)", at(FIXTURES.c, 63, 0), 70);
  check("C (0, 63)", at(FIXTURES.c, 0, 63), 77);
  check("C (63, 63)", at(FIXTURES.c, 63, 63), 147);
  check("F (0, 0)", at(FIXTURES.f, 0, 0), 0);
  check("F (1, 0)", at(FIXTURES.f, 1, 0), 29);
  check("F (0, 63)", at(FIXTURES.f, 0, 63), 77);
  check("F (1, 63)", at(FIXTURES.f, 1, 63), 106);
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
 * One Part 10 file of the identity fixture shape: 8-bit unsigned,
 * MONOCHROME2, Window Center 128, Window Width 256, VOI LUT Function LINEAR
 * and no rescale, per PS3.3 C.11.2.1.2.1 and its Note 4. The pixel data
 * comes from the fixture's own stored pattern.
 */
function part10({ columns, rows, spacing, sop, series, seriesNumber, instance }, fixture) {
  const sopClass = "1.2.840.10008.5.1.4.1.1.4";
  const meta = Buffer.concat([
    longEl(0x0002, 0x0001, "OB", Buffer.from([0, 1])),
    shortEl(0x0002, 0x0002, "UI", uid(sopClass)),
    shortEl(0x0002, 0x0003, "UI", uid(sop)),
    shortEl(0x0002, 0x0010, "UI", uid("1.2.840.10008.1.2.1")),
    shortEl(0x0002, 0x0012, "UI", uid("2.25.213")),
  ]);
  const pixels = fixturePixels(fixture);
  return Buffer.concat([
    Buffer.alloc(128),
    Buffer.from("DICM"),
    shortEl(0x0002, 0x0000, "UL", u32(meta.length)),
    meta,
    shortEl(0x0008, 0x0016, "UI", uid(sopClass)),
    shortEl(0x0008, 0x0018, "UI", uid(sop)),
    shortEl(0x0008, 0x0060, "CS", text("MR")),
    shortEl(0x0020, 0x000e, "UI", uid(series)),
    shortEl(0x0020, 0x0011, "IS", text(String(seriesNumber))),
    shortEl(0x0020, 0x0013, "IS", text(String(instance))),
    shortEl(0x0028, 0x0002, "US", u16(1)),
    shortEl(0x0028, 0x0004, "CS", text("MONOCHROME2")),
    shortEl(0x0028, 0x0010, "US", u16(rows)),
    shortEl(0x0028, 0x0011, "US", u16(columns)),
    ...(spacing ? [shortEl(0x0028, 0x0030, "DS", text(spacing))] : []),
    shortEl(0x0028, 0x0100, "US", u16(8)),
    shortEl(0x0028, 0x0101, "US", u16(8)),
    shortEl(0x0028, 0x0102, "US", u16(7)),
    shortEl(0x0028, 0x0103, "US", u16(0)),
    shortEl(0x0028, 0x1050, "DS", text("128")),
    shortEl(0x0028, 0x1051, "DS", text("256")),
    shortEl(0x0028, 0x1056, "CS", text("LINEAR")),
    longEl(0x7fe0, 0x0010, "OB", pixels),
  ]);
}

/** The per-fixture pixel data with the override applied where one exists. */
function fixturePixels(fixture) {
  const bytes = Buffer.alloc(fixture.columns * fixture.rows + ((fixture.columns * fixture.rows) % 2));
  for (let y = 0; y < fixture.rows; y += 1) {
    for (let x = 0; x < fixture.columns; x += 1) {
      const value = fixture.override && fixture.override.x === x && fixture.override.y === y
        ? fixture.override.value
        : stored(x, y);
      bytes[y * fixture.columns + x] = value;
    }
  }
  return bytes;
}

function writeFixtures(folder) {
  const files = [
    ["f204-p.dcm", { columns: 128, rows: 96, series: "2.25.21340", seriesNumber: 1, instance: 1, sop: "2.25.21341" }, FIXTURES.p],
    ["f204-f.dcm", { columns: 2, rows: 64, series: "2.25.21340", seriesNumber: 1, instance: 2, sop: "2.25.21342" }, FIXTURES.f],
    ["f204-b.dcm", { columns: 129, rows: 65, series: "2.25.21343", seriesNumber: 2, instance: 1, sop: "2.25.21344" }, FIXTURES.b],
    ["f204-c.dcm", { columns: 64, rows: 64, series: "2.25.21345", seriesNumber: 3, instance: 1, sop: "2.25.21346" }, FIXTURES.c],
    ["f204-d.dcm", { columns: 128, rows: 96, spacing: "0.7\\0.5", series: "2.25.21347", seriesNumber: 4, instance: 1, sop: "2.25.21348" }, FIXTURES.d],
    ["f204-d2.dcm", { columns: 128, rows: 96, spacing: "0.5\\0.7", series: "2.25.21349", seriesNumber: 5, instance: 1, sop: "2.25.21350" }, FIXTURES.d2],
    ["f204-e.dcm", { columns: 2048, rows: 8, series: "2.25.21351", seriesNumber: 6, instance: 1, sop: "2.25.21352" }, FIXTURES.e],
  ];
  for (const [name, spec, fixture] of files) {
    writeFileSync(join(folder, name), part10(spec, fixture));
  }
}

function permute90(bytes, columns, rows) {
  // rotate(90deg) is clockwise, so the displayed pixel at (dx, dy) is the
  // stored pixel at (x = dy, y = rows - 1 - dx).
  const out = new Array(columns * rows);
  for (let dy = 0; dy < columns; dy += 1) {
    for (let dx = 0; dx < rows; dx += 1) {
      out[dy * rows + dx] = bytes[(rows - 1 - dx) * columns + dy];
    }
  }
  return out;
}

function permuteFlipH(bytes, columns, rows) {
  const out = new Array(columns * rows);
  for (let y = 0; y < rows; y += 1) {
    for (let x = 0; x < columns; x += 1) out[y * columns + x] = bytes[y * columns + (columns - 1 - x)];
  }
  return out;
}

function permuteFlipV(bytes, columns, rows) {
  const out = new Array(columns * rows);
  for (let y = 0; y < rows; y += 1) {
    for (let x = 0; x < columns; x += 1) out[y * columns + x] = bytes[(rows - 1 - y) * columns + x];
  }
  return out;
}

/** The smallest integer CSS step whose product with the ratio is integral. */
function stepFor(ratio) {
  for (let step = 1; step <= 64; step += 1) {
    if (Math.abs(step * ratio - Math.round(step * ratio)) < 1e-9) return step;
  }
  throw new Error(`no integer step lands on the device grid at ratio ${ratio}`);
}

async function withPage(browser, ratio, run) {
  const page = await browser.newPage({ viewport: VIEWPORT, deviceScaleFactor: ratio });
  try {
    return await run(page);
  } finally {
    await page.close();
  }
}

async function openFolder(page, url, folder) {
  await page.goto(url);
  await page.locator("#folder").setInputFiles(folder);
  await page.waitForFunction(
    (count) => (document.getElementById("status").textContent || "").startsWith(count + " series."),
    SERIES_COUNT,
    { timeout: 20000 },
  );
  await page.waitForFunction(() => Boolean(activeViewport().imageWidth), null, { timeout: 20000 });
  await page.waitForTimeout(200);
}

/** Switch series by its row text and wait for that fixture's own shape. */
async function showSeries(page, rowText, match) {
  await page.locator("#series-list button", { hasText: rowText }).click();
  await page.waitForFunction(match, null, { timeout: 20000 });
  await page.waitForTimeout(200);
}

const SHAPE = {
  p: () => activeViewport().imageWidth === 128 && activeViewport().aspectY === 1,
  b: () => activeViewport().imageWidth === 129,
  c: () => activeViewport().imageWidth === 64,
  d: () => activeViewport().aspectY === 1.4,
  d2: () => activeViewport().aspectX === 1.4,
  e: () => activeViewport().imageWidth === 2048,
  f: () => activeViewport().imageWidth === 2,
};

function view(page) {
  return page.evaluate(() => {
    const vp = activeViewport();
    const canvas = vp.dom.canvas;
    const rect = canvas.getBoundingClientRect();
    const cell = vp.dom.cell.getBoundingClientRect();
    const fault = vp.dom.fault;
    return {
      x: rect.x, y: rect.y, w: rect.width, h: rect.height,
      cellX: cell.x, cellY: cell.y, cellW: cell.width, cellH: cell.height,
      dpr: window.devicePixelRatio,
      backingW: canvas.width, backingH: canvas.height,
      rendering: window.getComputedStyle(canvas).imageRendering,
      cornerLast: (vp.dom.corners.br.textContent || "").split("\n").pop(),
      mode: vp.zoomMode,
      label: vp.zoomLabel,
      imageW: vp.imageWidth, imageH: vp.imageHeight,
      aspectX: vp.aspectX, aspectY: vp.aspectY,
      scale: vp.scale, panX: vp.panX, panY: vp.panY,
      paintX: vp.paintX, paintY: vp.paintY, paintW: vp.paintW, paintH: vp.paintH,
      faultShown: !fault.hidden && Boolean(fault.textContent),
    };
  });
}

/**
 * The device bytes at the recorded painted offset, read from the canvas's
 * own backing store through getImageData, decoded in-page from the shot
 * when a raw session is given. The backing is the device resolution raster
 * the amendment's mechanism defines: the element box in whole CSS pixels
 * times the device pixel ratio, with the stored pixels written one to one
 * at the integer device offset the page records. The crop offset is that
 * recorded integer, never a search: if the bytes are not there the row
 * fails. The compositor maps this backing one to one onto the device grid
 * when the canvas's static position times the ratio is a whole device
 * pixel, which the mode's alignment guarantees and the AC1 element box
 * check pins; scratch probes under target in the F-204 fix round measured
 * that mapping byte exact at ratios 1, 1.25, 1.5 and 2, and measured that
 * the compositor's own subpixel placement of a fractional stage chain can
 * shift the whole layer by a whole device pixel without ever resampling
 * it, which is why the bytes are asserted on the backing and the mapping
 * on the box.
 */
async function capturePixels(page, cropW, cropH) {
  return page.evaluate(({ cropW, cropH }) => {
    const vp = activeViewport();
    const canvas = vp.dom.canvas;
    if (vp.paintW !== cropW || vp.paintH !== cropH) {
      return { error: `the recorded painted box ${vp.paintW} by ${vp.paintH} is not the expected ${cropW} by ${cropH}` };
    }
    const context = canvas.getContext("2d");
    const data = context.getImageData(vp.paintOffsetX, vp.paintOffsetY, cropW, cropH).data;
    const grey = new Array(cropW * cropH);
    for (let at = 0; at < cropW * cropH; at += 1) grey[at] = data[at * 4];
    return { grey };
  }, { cropW, cropH });
}

/** The located crop must match the stored pattern at every device pixel. */
function byteProblems(name, got, want, columns) {
  if (got.error) return [`${name}: ${got.error}`];
  const problems = [];
  let wrong = 0;
  const seen = [];
  for (let at = 0; at < want.length; at += 1) {
    if (got.grey[at] !== want[at]) {
      wrong += 1;
      if (seen.length < 6) seen.push(`(${at % columns},${Math.floor(at / columns)})`);
    }
  }
  if (wrong) {
    problems.push(`${name}: ${wrong} of ${want.length} device pixels wrong, first at ${seen.join(" ")}`);
  }
  return problems;
}

/**
 * Walk the viewport width until the mode's device box puts the ideal
 * centring of a screenW device pixel picture on a half device pixel, so the
 * origin cases prove the snap against a centring the engine cannot render
 * unpicked. The box is the largest whole CSS pixel count not exceeding the
 * cell whose product with the ratio is integral, the page's own rule.
 */
async function fractionalCell(page, ratio, screenW) {
  const step = stepFor(ratio);
  const fractional = (cellW) => {
    const whole = Math.floor(cellW);
    const box = whole - (whole % step);
    return (box * ratio - screenW) % 2 === 1;
  };
  for (let width = 1600; width <= 1720; width += 1) {
    await page.setViewportSize({ width, height: VIEWPORT.height });
    const cellW = await page.evaluate(() => activeViewport().dom.cell.getBoundingClientRect().width);
    if (fractional(cellW)) return { cellW };
  }
  throw new Error(`no viewport width makes the centring fractional for a ${screenW} device pixel picture at ratio ${ratio}`);
}

// AC1. The drawn extent is the stored extent in device pixels at ratios 1,
// 1.25, 1.5 and 2, and the axes swap under a 90 degree rotation, read from
// the recorded painted box. The canvas element maps its whole CSS pixel box
// one to one onto the device grid, which the backing check pins.
export async function observeExtents(url, chromium, folder) {
  const browser = await chromium.launch({ headless: true });
  try {
    const readings = [];
    for (const ratio of [1, 1.25, 1.5, 2]) {
      await withPage(browser, ratio, async (page) => {
        await openFolder(page, url, folder);
        await page.keyboard.press("\\");
        readings.push({ name: `extent at dpr ${ratio}`, ...await view(page) });
      });
    }
    await withPage(browser, 2, async (page) => {
      await openFolder(page, url, folder);
      await page.keyboard.press("\\");
      await page.keyboard.press("o");
      await page.waitForTimeout(100);
      readings.push({ name: "extent rotated at dpr 2", ...await view(page) });
    });
    return readings;
  } finally {
    await browser.close();
  }
}

export function extentProblems(readings) {
  const problems = [];
  if (!Array.isArray(readings) || readings.length !== 5) {
    problems.push("the extent cases were not all observed");
    return problems;
  }
  for (const reading of readings) {
    const turned = reading.name.includes("rotated");
    const expectedW = turned ? reading.imageH : reading.imageW;
    const expectedH = turned ? reading.imageW : reading.imageH;
    if (reading.paintW !== expectedW || reading.paintH !== expectedH) {
      problems.push(`${reading.name}: the painted box is ${reading.paintW} by ${reading.paintH}, wanted ${expectedW} by ${expectedH}`);
    }
    if (Math.abs(reading.w * reading.dpr - reading.backingW) > EXTENT_EPSILON
      || Math.abs(reading.h * reading.dpr - reading.backingH) > EXTENT_EPSILON) {
      problems.push(`${reading.name}: the element box ${reading.w} by ${reading.h} at ratio ${reading.dpr} does not map one to one onto the ${reading.backingW} by ${reading.backingH} backing`);
    }
    if (!Number.isInteger(reading.paintX) || !Number.isInteger(reading.paintY)) {
      problems.push(`${reading.name}: the painted origin ${reading.paintX}, ${reading.paintY} is not whole device pixels`);
    }
    if (reading.mode !== "one") problems.push(`${reading.name}: the mode is ${reading.mode}, not one`);
  }
  return problems;
}

// AC2. The painted origin is observed, not predicted: the byte row cropped
// at the recorded origin must match, and the origin must sit within a
// device pixel of the ideal centring plus the pan. Each case first asserts
// that the ideal centring is fractional in its measured cell, so a missing
// or misrounded snap cannot pass silently.
export async function observeOrigins(url, chromium, folder) {
  const browser = await chromium.launch({ headless: true });
  const readings = [];
  try {
    await withPage(browser, 1, async (page) => {
      await openFolder(page, url, folder);
      await fractionalCell(page, 1, 128);
      await page.keyboard.press("\\");
      const reading = await view(page);
      readings.push({ name: "origin dpr 1 odd cell", ...reading });
      readings.push({ name: "origin dpr 1 odd cell", crop: await capturePixels(page, 128, 96) });
    });
    for (const ratio of [2, 1.25, 1.5]) {
      await withPage(browser, ratio, async (page) => {
        await openFolder(page, url, folder);
        await showSeries(page, "#2", SHAPE.b);
        await fractionalCell(page, ratio, 129);
        await page.keyboard.press("\\");
        readings.push({ name: `origin dpr ${ratio} fixture B`, ...await view(page) });
        readings.push({ name: `origin dpr ${ratio} fixture B`, crop: await capturePixels(page, 129, 65) });
      });
    }
    return readings;
  } finally {
    await browser.close();
  }
}

export function originProblems(readings) {
  const problems = [];
  if (!Array.isArray(readings) || readings.length !== 8) {
    problems.push("the origin cases were not all observed");
    return problems;
  }
  for (let index = 0; index < readings.length; index += 2) {
    const reading = readings[index];
    const crop = readings[index + 1].crop;
    const name = reading.name;
    const screenW = reading.paintW;
    const screenH = reading.paintH;
    const boxDevW = Math.round(reading.w * reading.dpr);
    if ((boxDevW - screenW) % 2 !== 1) {
      problems.push(`${name}: the device box minus the picture is ${boxDevW - screenW}, so the ideal centring is not fractional and the case proves nothing`);
    }
    const want = reading.name.includes("fixture B") ? FIXTURES.b : FIXTURES.p;
    problems.push(...byteProblems(`${name} at the recorded origin`, crop, want.bytes, want.columns));
    const centreDevX = (reading.cellX + reading.cellW / 2) * reading.dpr;
    const centreDevY = (reading.cellY + reading.cellH / 2) * reading.dpr;
    // The ideal is the cell centre plus the pan: the alignment shift the
    // margins carry is compensated inside the device offset, so the
    // picture's centre lands there up to the offset's half device pixel
    // round and the layout unit the alignment quantises on.
    const offX = Math.abs(reading.paintX + screenW / 2 - centreDevX - reading.panX * reading.dpr);
    const offY = Math.abs(reading.paintY + screenH / 2 - centreDevY - reading.panY * reading.dpr);
    if (offX > ORIGIN_SLACK) problems.push(`${name}: the painted origin x sits ${offX.toFixed(4)} device px from the ideal centring plus pan`);
    if (offY > ORIGIN_SLACK) problems.push(`${name}: the painted origin y sits ${offY.toFixed(4)} device px from the ideal centring plus pan`);
    if (reading.mode !== "one") problems.push(`${name}: the mode is ${reading.mode}, not one`);
  }
  return problems;
}

// AC3. The device bytes equal the stored pattern at every byte row case:
// both square-pixel fixtures at ratios 1, 1.25, 1.5 and 2, and fixture P
// again after a pan drag of 40 device pixels. Each crop is taken at the
// recorded painted origin, an exact integer device offset.
export async function observePixels(url, chromium, folder) {
  const browser = await chromium.launch({ headless: true });
  const readings = [];
  try {
    for (const ratio of [1, 1.25, 1.5, 2]) {
      for (const key of ["p", "b"]) {
        await withPage(browser, ratio, async (page) => {
          await openFolder(page, url, folder);
          if (key === "b") {
            await showSeries(page, "#2", SHAPE.b);
          }
          await page.keyboard.press("\\");
          await page.click("#toggle-corners");
          await page.mouse.move(2, 2);
          const fixture = key === "b" ? FIXTURES.b : FIXTURES.p;
          readings.push({
            name: `pixels dpr ${ratio} fixture ${fixture.name}`,
            got: await capturePixels(page, fixture.columns, fixture.rows),
          });
        });
      }
    }
    await withPage(browser, 2, async (page) => {
      await openFolder(page, url, folder);
      await page.keyboard.press("\\");
      await page.click("#toggle-corners");
      await page.mouse.move(2, 2);
      await page.keyboard.press("p");
      const box = await page.locator('.cell[data-cell="0"] canvas.view').boundingBox();
      await page.mouse.move(box.x + box.width / 2, box.y + box.height / 2);
      await page.mouse.down();
      await page.mouse.move(box.x + box.width / 2 + 20, box.y + box.height / 2, { steps: 4 });
      await page.mouse.up();
      readings.push({ name: "pixels midpan", got: await capturePixels(page, 128, 96) });
    });
    return readings;
  } finally {
    await browser.close();
  }
}

export function pixelProblems(readings) {
  const problems = [];
  if (!Array.isArray(readings) || readings.length !== 9) {
    problems.push("the pixel cases were not all observed");
    return problems;
  }
  for (const reading of readings) {
    const fixture = reading.name.endsWith("fixture B") ? FIXTURES.b : FIXTURES.p;
    problems.push(...byteProblems(reading.name, reading.got, fixture.bytes, fixture.columns));
  }
  return problems;
}

// AC4 and AC8, with the S4 mutant controls. The corner reads Zoom 1:1 in
// the mode, one zoom drag tick returns the fit-relative percent, opening
// another series returns to fit, and both the Fit button and the 0 key
// leave the mode.
export async function observeLabelAndSeries(url, chromium, folder) {
  const browser = await chromium.launch({ headless: true });
  try {
    return await withPage(browser, 1, async (page) => {
      await openFolder(page, url, folder);
      await page.keyboard.press("\\");
      const inMode = await view(page);
      await page.keyboard.press("z");
      const box = await page.locator('.cell[data-cell="0"] canvas.view').boundingBox();
      await page.mouse.move(box.x + box.width / 2, box.y + box.height / 2);
      await page.mouse.down();
      await page.mouse.move(box.x + box.width / 2, box.y + box.height / 2 - 40, { steps: 4 });
      await page.mouse.up();
      const afterDrag = await view(page);
      await page.keyboard.press("\\");
      await page.click("#fit");
      const afterFitButton = await view(page);
      await page.keyboard.press("\\");
      await page.keyboard.press("0");
      const afterZeroKey = await view(page);
      await showSeries(page, "#2", SHAPE.b);
      const afterSeries = await view(page);
      return { inMode, afterDrag, afterFitButton, afterZeroKey, afterSeries };
    });
  } finally {
    await browser.close();
  }
}

export function labelAndSeriesProblems(seen) {
  const problems = [];
  if (!seen || !seen.inMode || !seen.afterDrag || !seen.afterFitButton || !seen.afterZeroKey || !seen.afterSeries) {
    problems.push("the label and series cases were not observed");
    return problems;
  }
  if (seen.inMode.cornerLast !== "Zoom 1:1") {
    problems.push(`in the mode the corner reads ${JSON.stringify(seen.inMode.cornerLast)}, not Zoom 1:1`);
  }
  if (seen.afterDrag.mode !== "free") problems.push(`after one zoom drag tick the mode is ${seen.afterDrag.mode}, not free`);
  if (!/^\d+%$/.test(seen.afterDrag.label)) {
    problems.push(`after one zoom drag tick the label is ${JSON.stringify(seen.afterDrag.label)}, not a fit-relative percent`);
  }
  for (const [name, reading] of [["the Fit button", seen.afterFitButton], ["the 0 key", seen.afterZeroKey]]) {
    if (reading.mode !== "fit") problems.push(`after ${name} the mode is ${reading.mode}, not fit`);
    if (!/^\d+%$/.test(reading.label)) {
      problems.push(`after ${name} the label is ${JSON.stringify(reading.label)}, not a percent`);
    }
  }
  if (seen.afterSeries.mode !== "fit") problems.push(`after a series open the mode is ${seen.afterSeries.mode}, not fit`);
  if (!/^\d+%$/.test(seen.afterSeries.label)) {
    problems.push(`after a series open the label is ${JSON.stringify(seen.afterSeries.label)}, not a percent`);
  }
  return problems;
}

// The init script that counts the page's own resolution watcher arms and
// the window resize events, so a case can prove the watcher re-laid out
// while the CSS viewport, and so the resize listener, never fired.
const WATCH_WRAPPER = () => {
  window.__resolutionArms = 0;
  window.__resizeEvents = 0;
  window.addEventListener("resize", () => { window.__resizeEvents += 1; });
  const original = window.MediaQueryList.prototype.addEventListener;
  window.MediaQueryList.prototype.addEventListener = function (type, listener, options) {
    if (String(this.media).includes("resolution")) window.__resolutionArms += 1;
    return original.call(this, type, listener, options);
  };
};

/**
 * Change the device pixel ratio through the CDP override the plan names,
 * with the CSS viewport held constant so the page's resize listener cannot
 * fire. Measured with scratch probes under target in the F-204 fix round: a
 * device scale factor only override updates window.devicePixelRatio and the
 * query values but dispatches no MediaQueryList change event in this
 * Chromium, and the pending change dispatches on the next evaluation pass.
 * A clipped raw CDP page capture is one that neither resizes nor lays out,
 * and unlike an unclipped one it dispatches the pending change reliably on
 * every transition, so the override is followed by one.
 */
async function changeRatio(page, session, factor) {
  const size = page.viewportSize();
  await session.send("Emulation.setDeviceMetricsOverride", {
    width: size.width, height: size.height, deviceScaleFactor: factor, mobile: false,
  });
  await session.send("Page.captureScreenshot", {
    format: "png",
    clip: { x: 0, y: 0, width: size.width, height: size.height, scale: 1 },
  });
}

// AC5 and AC10. A device pixel ratio change re-lays out through the page's
// own matchMedia watcher with the CSS viewport held constant, and the
// identity renders pixelated at the ratios where the landed comparison
// evaluates one ulp below 1.
export async function observeRatioWatcher(url, chromium, folder) {
  const browser = await chromium.launch({ headless: true });
  try {
    return await withPage(browser, 1, async (page) => {
      await page.addInitScript(WATCH_WRAPPER);
      await openFolder(page, url, folder);
      await page.keyboard.press("\\");
      const before = await view(page);
      const session = await page.context().newCDPSession(page);
      await changeRatio(page, session, 2);
      await page.waitForFunction((stale) => {
        const vp = activeViewport();
        return vp.paintX !== null && vp.paintX !== stale && window.__resolutionArms >= 2;
      }, before.paintX, { timeout: 10000 }).catch(() => {});
      const atTwo = await view(page);
      const armsAfterTwo = await page.evaluate(() => window.__resolutionArms);
      const resizedAfterTwo = await page.evaluate(() => window.__resizeEvents);
      const cropAtTwo = await capturePixels(page, 128, 96);
      await changeRatio(page, session, 1);
      await page.waitForFunction((stale) => {
        const vp = activeViewport();
        return vp.paintX !== null && vp.paintX !== stale && window.__resolutionArms >= 3;
      }, atTwo.paintX, { timeout: 10000 }).catch(() => {});
      const atOne = await view(page);
      const armsAfterOne = await page.evaluate(() => window.__resolutionArms);
      const resizedAfterOne = await page.evaluate(() => window.__resizeEvents);
      const cropAtOne = await capturePixels(page, 128, 96);
      const probes = [];
      for (const factor of [1.38, 1.45, 1.54, 1.57]) {
        await changeRatio(page, session, factor);
        await page.waitForTimeout(300);
        probes.push(await page.evaluate(() => {
          const vp = activeViewport();
          const canvas = vp.dom.canvas;
          return {
            factor: window.devicePixelRatio,
            product: (1 / window.devicePixelRatio) * window.devicePixelRatio,
            rendering: window.getComputedStyle(canvas).imageRendering,
            mode: vp.zoomMode,
          };
        }));
      }
      const resizeEvents = await page.evaluate(() => window.__resizeEvents);
      return { before, atTwo, armsAfterTwo, resizedAfterTwo, cropAtTwo, atOne, armsAfterOne, resizedAfterOne, cropAtOne, probes, resizeEvents };
    });
  } finally {
    await browser.close();
  }
}

export function ratioWatcherProblems(seen) {
  const problems = [];
  if (!seen || !seen.atTwo || !seen.atOne) {
    problems.push("the ratio change case was not observed");
    return problems;
  }
  for (const [name, reading, arms, resized, crop] of [
    ["ratio 2", seen.atTwo, seen.armsAfterTwo, seen.resizedAfterTwo, seen.cropAtTwo],
    ["ratio 1", seen.atOne, seen.armsAfterOne, seen.resizedAfterOne, seen.cropAtOne],
  ]) {
    if (reading.mode !== "one") problems.push(`after the ratio change the mode is ${reading.mode}, not one`);
    if (reading.paintW !== 128 || reading.paintH !== 96) {
      problems.push(`after the ratio change to ${name} the painted box is ${reading.paintW} by ${reading.paintH}, wanted the stored 128 by 96`);
    }
    if (!Number.isInteger(reading.paintX) || !Number.isInteger(reading.paintY)) {
      problems.push(`after the ratio change to ${name} the painted origin is not whole device pixels`);
    }
    if (!(arms >= (name === "ratio 2" ? 2 : 3))) {
      problems.push(`the resolution watcher armed ${arms} times at ${name}, wanted more`);
    }
    if (resized !== 0) problems.push(`${resized} resize events fired at ${name}, wanted none with the CSS viewport held`);
    problems.push(...byteProblems(`the ratio change to ${name}`, crop, FIXTURES.p.bytes, FIXTURES.p.columns));
  }
  if (seen.resizeEvents !== 0) problems.push(`${seen.resizeEvents} resize events fired in the case, wanted none`);
  if (!Array.isArray(seen.probes) || seen.probes.length !== 4) {
    problems.push("the pixelated probes were not observed");
    return problems;
  }
  for (const probe of seen.probes) {
    if (!(probe.product < 1)) {
      problems.push(`at ratio ${probe.factor} the precondition (1 / dpr) * dpr < 1 does not hold (${probe.product}), so the probe proves nothing`);
    }
    if (probe.rendering !== "pixelated") {
      problems.push(`at ratio ${probe.factor} the identity renders ${probe.rendering}, not pixelated`);
    }
  }
  return problems;
}

// AC6 and AC20. The clamp reaches the identity on a small frame and zooms
// out below it on a large frame. The identity readings come from the
// recorded painted box, because the mode's element box is the cell; the
// readings after a tick are free mode and read from the element as before.
export async function observeClamp(url, chromium, folder) {
  const browser = await chromium.launch({ headless: true });
  const settleExtent = async (page) => {
    let last = -1;
    for (let tick = 0; tick < 80; tick += 1) {
      await page.keyboard.press("-");
      const width = await page.evaluate(() => activeViewport().dom.canvas.getBoundingClientRect().width * window.devicePixelRatio);
      if (Math.abs(width - last) < 1e-9) break;
      last = width;
    }
    return last;
  };
  try {
    const small = await withPage(browser, 2, async (page) => {
      await openFolder(page, url, folder);
      await showSeries(page, "#3", SHAPE.c);
      const fitted = await page.evaluate(() => fitScale(activeViewport()));
      const precondition = fitted * 0.05 > 1 / 2;
      await page.keyboard.press("\\");
      const identity = await view(page);
      await page.keyboard.press("=");
      const afterIn = await view(page);
      const settled = await settleExtent(page);
      const afterOut = await view(page);
      // The wheel acts on the viewport under the pointer, so the pointer
      // must sit over the canvas for the tick to reach the page at all.
      const wheelBox = await page.locator('.cell[data-cell="0"] canvas.view').boundingBox();
      await page.mouse.move(wheelBox.x + wheelBox.width / 2, wheelBox.y + wheelBox.height / 2);
      await page.keyboard.down("Control");
      await page.mouse.wheel(0, 120);
      await page.keyboard.up("Control");
      const afterWheelOut = await view(page);
      await page.keyboard.down("Control");
      await page.mouse.wheel(0, -120);
      await page.keyboard.up("Control");
      const afterWheelIn = await view(page);
      return { fitted, precondition, identity, afterIn, settled, afterOut, afterWheelOut, afterWheelIn };
    });
    const large = await withPage(browser, 2, async (page) => {
      await openFolder(page, url, folder);
      await showSeries(page, "#6", SHAPE.e);
      const fitted = await page.evaluate(() => fitScale(activeViewport()));
      const precondition = fitted * 0.05 < 1 / 2;
      await page.keyboard.press("\\");
      const identity = await view(page);
      await page.keyboard.press("-");
      const firstOut = await view(page);
      const settled = await settleExtent(page);
      const afterOut = await view(page);
      return { fitted, precondition, identity, firstOut, settled, afterOut };
    });
    return { small, large };
  } finally {
    await browser.close();
  }
}

export function clampProblems(seen) {
  const problems = [];
  for (const [name, caseSeen, wantPaintW] of [["small", seen.small, 64], ["large", seen.large, 2048]]) {
    if (!caseSeen || !caseSeen.identity) {
      problems.push(`the ${name} frame clamp case was not observed`);
      return problems;
    }
    if (!caseSeen.precondition) problems.push(`the ${name} frame clamp precondition does not hold in the measured cell`);
    if (caseSeen.identity.paintW !== wantPaintW) {
      problems.push(`the ${name} frame identity painted width is ${caseSeen.identity.paintW}, wanted ${wantPaintW}`);
    }
    if (caseSeen.identity.mode !== "one") problems.push(`the ${name} frame identity mode is ${caseSeen.identity.mode}, not one`);
  }
  const small = seen.small;
  if (Math.abs(small.afterIn.w * small.afterIn.dpr - 64) <= EXTENT_EPSILON) {
    problems.push("the small frame zoom-in tick did not leave the identity");
  }
  if (Math.abs(small.afterOut.w * small.afterOut.dpr - 64) > EXTENT_EPSILON) {
    problems.push(`the small frame zoom-out ticks settled at ${(small.afterOut.w * small.afterOut.dpr).toFixed(3)}, wanted the identity 64`);
  }
  if (Math.abs(small.afterWheelOut.w * small.afterWheelOut.dpr - 64) > EXTENT_EPSILON) {
    problems.push(`the small frame wheel zoom-out tick left the extent at ${(small.afterWheelOut.w * small.afterWheelOut.dpr).toFixed(3)}, wanted the identity 64`);
  }
  if (!(small.afterWheelIn.w * small.afterWheelIn.dpr > 64 + EXTENT_EPSILON)) {
    problems.push(`the small frame wheel zoom-in control did not move the extent (${(small.afterWheelIn.w * small.afterWheelIn.dpr).toFixed(3)}), so the wheel tick proved nothing`);
  }
  const wantLabel = Math.round((1 / 2 / small.fitted) * 100) + "%";
  if (small.afterOut.label !== wantLabel) {
    problems.push(`the small frame label reads ${JSON.stringify(small.afterOut.label)}, wanted ${JSON.stringify(wantLabel)}`);
  }
  const large = seen.large;
  if (!(large.firstOut.w * large.firstOut.dpr < 2048 - EXTENT_EPSILON)) {
    problems.push(`the large frame first zoom-out tick kept the identity extent ${(large.firstOut.w * large.firstOut.dpr).toFixed(3)}`);
  }
  const wantExtent = 2048 * large.fitted * 0.05 * 2;
  if (Math.abs(large.afterOut.w * large.afterOut.dpr - wantExtent) > EXTENT_EPSILON) {
    problems.push(`the large frame settled extent is ${(large.afterOut.w * large.afterOut.dpr).toFixed(3)}, wanted ${wantExtent.toFixed(3)}`);
  }
  return problems;
}

// AC7a. With link zoom on, a linked viewport whose own guards pass enters
// the mode over its own image and cell, the anisotropic target enters with
// the Zoom 1:1 rows label, and the below-ratio target keeps its own free
// scale and pan. Cell 3 is zoomed and panned into a free state first, so
// the keep is observable against a non-fit state.
export async function observeLink(url, chromium, folder) {
  const browser = await chromium.launch({ headless: true });
  try {
    return await withPage(browser, 3, async (page) => {
      await openFolder(page, url, folder);
      await page.locator("#layout").click();
      await page.locator('#layout-picker [data-columns="2"][data-rows="2"]').click();
      const rows = {
        "#2": 1,
        "#4": 2,
        "#1": 3,
      };
      for (const [row, cell] of Object.entries(rows)) {
        await page.locator("#series-list button", { hasText: row }).first().dragTo(page.locator(`.cell[data-cell="${cell}"]`));
      }
      await page.waitForFunction(() => viewports[1].imageWidth === 129 && viewports[2].aspectY === 1.4 && viewports[3].imageWidth === 128, null, { timeout: 20000 });
      await page.evaluate(() => showIndex(viewports[3], 1));
      await page.waitForFunction(() => viewports[3].imageWidth === 2, null, { timeout: 20000 });
      // Zoom cell 3 into a free scale and pan it, so its state is not fit.
      await page.locator('.cell[data-cell="3"]').click();
      await page.keyboard.press("=");
      const box = await page.locator('.cell[data-cell="3"] canvas.view').boundingBox();
      await page.keyboard.press("p");
      await page.mouse.move(box.x + box.width / 2, box.y + box.height / 2);
      await page.mouse.down();
      await page.mouse.move(box.x + box.width / 2 + 24, box.y + box.height / 2 + 12, { steps: 4 });
      await page.mouse.up();
      const before = await page.evaluate(() => {
        const vp = viewports[3];
        return { mode: vp.zoomMode, scale: vp.scale, panX: vp.panX, panY: vp.panY };
      });
      await page.locator("#link").click();
      await page.locator("#link-zoom").click();
      await page.keyboard.press("Escape");
      await page.locator('.cell[data-cell="0"]').click();
      await page.keyboard.press("\\");
      await page.waitForTimeout(300);
      const cells = await page.evaluate(() => [0, 1, 2, 3].map((cell) => {
        const vp = viewports[cell];
        return {
          cell,
          mode: vp.zoomMode,
          label: vp.zoomLabel,
          cornerLast: (vp.dom.corners.br.textContent || "").split("\n").pop(),
          imageW: vp.imageWidth, imageH: vp.imageHeight,
          aspectX: vp.aspectX, aspectY: vp.aspectY,
          paintX: vp.paintX, paintY: vp.paintY, paintW: vp.paintW, paintH: vp.paintH,
          scale: vp.scale, panX: vp.panX, panY: vp.panY,
        };
      }));
      return { before, cells };
    });
  } finally {
    await browser.close();
  }
}

export function linkProblems(seen) {
  const problems = [];
  if (!seen || !Array.isArray(seen.cells) || seen.cells.length !== 4 || !seen.before) {
    problems.push("the linked cases were not observed");
    return problems;
  }
  if (seen.before.mode !== "free") {
    problems.push(`the below-ratio cell was not left in a free state before the propagation (${seen.before.mode})`);
  }
  for (const cell of seen.cells) {
    if (cell.cell === 3) {
      if (cell.mode !== "free") problems.push(`the below-ratio linked cell kept mode ${cell.mode}, wanted its own free`);
      if (cell.scale !== seen.before.scale || cell.panX !== seen.before.panX || cell.panY !== seen.before.panY) {
        problems.push(`the below-ratio linked cell moved: scale ${seen.before.scale} to ${cell.scale}, pan ${seen.before.panX}, ${seen.before.panY} to ${cell.panX}, ${cell.panY}`);
      }
      continue;
    }
    if (cell.mode !== "one") {
      problems.push(`linked cell ${cell.cell} has mode ${cell.mode}, not one`);
      continue;
    }
    const anisotropic = cell.cell === 2;
    if (anisotropic && cell.cornerLast !== "Zoom 1:1 rows") {
      problems.push(`the anisotropic linked cell corner reads ${JSON.stringify(cell.cornerLast)}, not Zoom 1:1 rows`);
    }
    if (!anisotropic && cell.cornerLast !== "Zoom 1:1") {
      problems.push(`linked cell ${cell.cell} corner reads ${JSON.stringify(cell.cornerLast)}, not Zoom 1:1`);
    }
    // At the identity the painted device box is the stored row count on
    // every frame and the stored width on square pixels and the spacing
    // ratio on anisotropic ones, at any ratio.
    const width = Math.round(cell.imageW * cell.aspectX / cell.aspectY);
    const height = cell.imageH;
    if (cell.paintW !== width || cell.paintH !== height) {
      problems.push(`linked cell ${cell.cell} painted box ${cell.paintW} by ${cell.paintH} is not ${width} by ${height}`);
    }
    if (!Number.isInteger(cell.paintX) || !Number.isInteger(cell.paintY)) {
      problems.push(`linked cell ${cell.cell} painted origin is not whole device pixels`);
    }
  }
  return problems;
}

// AC9. The backing is the element box times the ratio, and the in-page grey
// at the recorded painted offset equals the stored bytes at every pixel,
// which guards the window pipeline.
export async function observeBackingStore(url, chromium, folder) {
  const browser = await chromium.launch({ headless: true });
  try {
    return await withPage(browser, 1, async (page) => {
      await openFolder(page, url, folder);
      await page.keyboard.press("\\");
      return page.evaluate((want) => {
        const vp = activeViewport();
        const canvas = vp.dom.canvas;
        const boxW = Math.round(parseFloat(canvas.style.width));
        const boxH = Math.round(parseFloat(canvas.style.height));
        const ratio = window.devicePixelRatio || 1;
        if (canvas.width !== boxW * ratio || canvas.height !== boxH * ratio) {
          return `the backing is ${canvas.width} by ${canvas.height}, wanted the ${boxW} by ${boxH} CSS box times the ratio ${ratio}`;
        }
        if (vp.paintW !== 128 || vp.paintH !== 96) {
          return `the painted box is ${vp.paintW} by ${vp.paintH}, wanted the stored 128 by 96`;
        }
        const context = canvas.getContext("2d");
        const data = context.getImageData(vp.paintOffsetX, vp.paintOffsetY, 128, 96).data;
        for (let at = 0; at < want.length; at += 1) {
          if (data[at * 4] !== want[at]) {
            return `pixel ${at % 128},${Math.floor(at / 128)} shows ${data[at * 4]}, stored ${want[at]}`;
          }
        }
        return null;
      }, FIXTURES.p.bytes);
    });
  } finally {
    await browser.close();
  }
}

export function backingStoreProblems(seen) {
  return seen ? [seen] : [];
}

// AC12. In Four images tile mode, or with no image drawn, the control is a
// no-op with no fault and no layout write.
export async function observeRefusals(url, chromium, folder) {
  const browser = await chromium.launch({ headless: true });
  try {
    const tiled = await withPage(browser, 1, async (page) => {
      await openFolder(page, url, folder);
      await page.keyboard.press("Shift+G");
      await page.waitForFunction(() => activeViewport().tileMode, null, { timeout: 10000 });
      await page.keyboard.press("\\");
      return view(page);
    });
    const empty = await withPage(browser, 1, async (page) => {
      await page.goto(url);
      await page.keyboard.press("\\");
      return page.evaluate(() => {
        const vp = activeViewport();
        const fault = vp ? vp.dom.fault : null;
        return {
          mode: vp ? vp.zoomMode : "missing",
          faultShown: fault ? !fault.hidden && Boolean(fault.textContent) : true,
        };
      });
    });
    return { tiled, empty };
  } finally {
    await browser.close();
  }
}

export function refusalProblems(seen) {
  const problems = [];
  if (!seen || !seen.tiled || !seen.empty) {
    problems.push("the refusal cases were not observed");
    return problems;
  }
  if (seen.tiled.faultShown) problems.push("the control showed a fault in tile mode");
  if (seen.tiled.mode !== "fit") problems.push(`the tile mode control left the mode ${seen.tiled.mode}`);
  if (seen.empty.faultShown) problems.push("the control showed a fault with no image drawn");
  if (seen.empty.mode !== "fit") problems.push(`the control with no image drawn left the mode ${seen.empty.mode}`);
  return problems;
}

// AC13. The below-ratio refusal on fixture F at ratio 3, its clear at ratio
// 2, the no-stale-reason case, and the boundary positive control. Each
// ratio move goes through changeRatio, whose comment records why a plain
// override cannot dispatch the query change this row's clear depends on.
export async function observeBelowRatio(url, chromium, folder) {
  const browser = await chromium.launch({ headless: true });
  try {
    return await withPage(browser, 3, async (page) => {
      await openFolder(page, url, folder);
      await page.keyboard.press("ArrowDown");
      await page.waitForFunction(() => activeViewport().imageWidth === 2, null, { timeout: 20000 });
      await page.keyboard.press("\\");
      const refused = await view(page);
      const session = await page.context().newCDPSession(page);
      await changeRatio(page, session, 2);
      await page.waitForTimeout(400);
      const atTwo = await view(page);
      await changeRatio(page, session, 3);
      await page.waitForTimeout(400);
      const backAtThree = await view(page);
      await page.keyboard.press("ArrowDown");
      await page.waitForFunction(() => activeViewport().imageWidth === 128, null, { timeout: 20000 });
      await page.keyboard.press("ArrowUp");
      await page.waitForFunction(() => activeViewport().imageWidth === 2, null, { timeout: 20000 });
      await page.waitForTimeout(200);
      const afterScroll = await view(page);
      await changeRatio(page, session, 2);
      await page.waitForTimeout(400);
      await page.keyboard.press("\\");
      const entered = await view(page);
      // The state table's scroll row: with the mode held on a suitable
      // frame, a scroll to another suitable frame keeps the mode.
      await page.keyboard.press("ArrowUp");
      await page.waitForFunction(() => activeViewport().imageWidth === 128, null, { timeout: 20000 });
      const keptOne = await view(page);
      await page.keyboard.press("ArrowDown");
      await page.waitForFunction(() => activeViewport().imageWidth === 2, null, { timeout: 20000 });
      await changeRatio(page, session, 3);
      await page.waitForTimeout(400);
      const dropped = await view(page);
      return { refused, atTwo, backAtThree, afterScroll, entered, keptOne, dropped };
    });
  } finally {
    await browser.close();
  }
}

export function belowRatioProblems(seen) {
  const problems = [];
  if (!seen || !seen.refused) {
    problems.push("the below-ratio case was not observed");
    return problems;
  }
  if (seen.refused.cornerLast !== REFUSAL) {
    problems.push(`at ratio 3 the corner reads ${JSON.stringify(seen.refused.cornerLast)}, not ${JSON.stringify(REFUSAL)}`);
  }
  if (seen.refused.mode !== "fit") problems.push(`the refusal left the mode ${seen.refused.mode}`);
  if (seen.atTwo.cornerLast === REFUSAL) problems.push("at ratio 2 the corner still shows the refusal reason");
  if (!/^\d+%$/.test(seen.atTwo.label) && seen.atTwo.cornerLast !== "Zoom 1:1") {
    problems.push(`at ratio 2 the corner shows ${JSON.stringify(seen.atTwo.cornerLast)}, wanted the zoom percent`);
  }
  if (seen.backAtThree.cornerLast === REFUSAL) problems.push("back at ratio 3 the stale refusal reason is shown again");
  if (seen.afterScroll.cornerLast === REFUSAL) problems.push("after a scroll away and back the refusal reason is shown again");
  if (seen.entered.mode !== "one" || seen.entered.paintW !== 2) {
    problems.push(`the boundary control did not enter the mode at a 2 device pixel width (mode ${seen.entered && seen.entered.mode}, painted width ${seen.entered && seen.entered.paintW})`);
  }
  if (!seen.keptOne) {
    problems.push("the scroll keep case was not observed");
    return problems;
  }
  if (seen.keptOne.mode !== "one") {
    problems.push(`after the scroll to a suitable frame the mode is ${seen.keptOne.mode}, not the kept one (label ${JSON.stringify(seen.keptOne.label)}, image ${seen.keptOne.imageW} by ${seen.keptOne.imageH}, dpr ${seen.keptOne.dpr}, corner ${JSON.stringify(seen.keptOne.cornerLast)})`);
  }
  if (seen.dropped.mode !== "fit" || !/^\d+%$/.test(seen.dropped.label)) {
    problems.push(`back at ratio 3 the held mode did not drop to fit (mode ${seen.dropped && seen.dropped.mode}, label ${seen.dropped && JSON.stringify(seen.dropped.label)})`);
  }
  if (seen.dropped.cornerLast === REFUSAL) problems.push("the drop showed a refusal reason");
  return problems;
}

// AC14. At the identity with the 90 degree rotate and each flip applied, the
// clipped screenshot equals the permuted pattern, because every reachable
// rotation is an exact permutation of the index the page writes. The
// recorded painted box carries the swap.
export async function observeTransforms(url, chromium, folder) {
  const browser = await chromium.launch({ headless: true });
  try {
    const cases = [];
    const keys = [
      ["rotate", "o"],
      ["flip-h", "h"],
      ["flip-v", "v"],
    ];
    const expected = {
      rotate: { bytes: permute90(FIXTURES.p.bytes, FIXTURES.p.columns, FIXTURES.p.rows), columns: 96, rows: 128 },
      "flip-h": { bytes: permuteFlipH(FIXTURES.p.bytes, FIXTURES.p.columns, FIXTURES.p.rows), columns: 128, rows: 96 },
      "flip-v": { bytes: permuteFlipV(FIXTURES.p.bytes, FIXTURES.p.columns, FIXTURES.p.rows), columns: 128, rows: 96 },
    };
    for (const [name, key] of keys) {
      await withPage(browser, 2, async (page) => {
        await openFolder(page, url, folder);
        await page.keyboard.press("\\");
        await page.keyboard.press(key);
        await page.waitForTimeout(150);
        await page.mouse.move(2, 2);
        const want = expected[name];
        cases.push({ name, got: await capturePixels(page, want.columns, want.rows) });
      });
    }
    return cases;
  } finally {
    await browser.close();
  }
}

export function transformProblems(cases) {
  const problems = [];
  if (!Array.isArray(cases) || cases.length !== 3) {
    problems.push("the transform cases were not observed");
    return problems;
  }
  const fixture = FIXTURES.p;
  const want = {
    rotate: { bytes: permute90(fixture.bytes, fixture.columns, fixture.rows), columns: 96, rows: 128 },
    "flip-h": { bytes: permuteFlipH(fixture.bytes, fixture.columns, fixture.rows), columns: 128, rows: 96 },
    "flip-v": { bytes: permuteFlipV(fixture.bytes, fixture.columns, fixture.rows), columns: 128, rows: 96 },
  };
  for (const item of cases) {
    const expected = want[item.name];
    if (!expected) {
      problems.push(`${item.name}: an unknown transform case`);
      continue;
    }
    problems.push(...byteProblems(`transform ${item.name}`, item.got, expected.bytes, expected.columns));
  }
  return problems;
}

// AC17, AC18 and AC19. On the anisotropic fixtures the mode enters with the
// row axis exact and the columns resampled, the row origin is on the grid,
// and the device rows are order-preserving subsamples of the stored rows.
export async function observeAniso(url, chromium, folder) {
  const browser = await chromium.launch({ headless: true });
  try {
    return await withPage(browser, 2, async (page) => {
      const readings = {};
      await openFolder(page, url, folder);
      await showSeries(page, "#4", SHAPE.d);
      await page.keyboard.press("\\");
      readings.d = await view(page);
      readings.dCrop = await capturePixels(page, Math.round(128 * 0.5 / 0.7), 96);
      await showSeries(page, "#5", SHAPE.d2);
      await page.keyboard.press("\\");
      readings.d2 = await view(page);
      readings.d2Crop = await capturePixels(page, Math.round(128 * 0.7 / 0.5), 96);
      return readings;
    });
  } finally {
    await browser.close();
  }
}

/** The device rows must appear, in order, as subsamples of the stored rows. */
function anisoRowProblems(name, crop, fixture, paintW) {
  const problems = [];
  if (crop.error) return [`${name}: ${crop.error}`];
  const stored = fixture.bytes;
  const columns = fixture.columns;
  for (let row = 0; row < fixture.rows; row += 1) {
    let cursor = 0;
    for (let c = 0; c < paintW; c += 1) {
      const byte = crop.grey[row * paintW + c];
      while (cursor < columns && stored[row * columns + cursor] !== byte) cursor += 1;
      if (cursor === columns) {
        problems.push(`${name}: device row ${row} column ${c} shows ${byte}, which is not an in-order sample of the stored row`);
        return problems;
      }
      // The cursor stays on the match, because neighbouring device columns
      // may sample the same stored column when the axis is upscaled.
    }
  }
  return problems;
}

export function anisoProblems(seen) {
  const problems = [];
  if (!seen || !seen.d || !seen.d2) {
    problems.push("the anisotropic cases were not observed");
    return problems;
  }
  const wantW = { d: Math.round(128 * 0.5 / 0.7), d2: Math.round(128 * 0.7 / 0.5) };
  for (const [name, reading] of [["D", seen.d], ["D-prime", seen.d2]]) {
    if (reading.mode !== "one") {
      problems.push(`the ${name} mode is ${reading.mode}, not one`);
      continue;
    }
    if (reading.cornerLast !== "Zoom 1:1 rows") {
      problems.push(`the ${name} corner reads ${JSON.stringify(reading.cornerLast)}, not Zoom 1:1 rows`);
    }
    if (reading.paintH !== 96) {
      problems.push(`the ${name} painted height is ${reading.paintH}, not the stored 96 rows`);
    }
    if (Math.abs(reading.paintW - wantW[name === "D" ? "d" : "d2"]) > EXTENT_EPSILON) {
      problems.push(`the ${name} painted width ${reading.paintW} is not ${wantW[name === "D" ? "d" : "d2"]}`);
    }
    if (!Number.isInteger(reading.paintY)) {
      problems.push(`the ${name} painted row origin ${reading.paintY} is not a whole device pixel`);
    }
  }
  problems.push(...anisoRowProblems("D", seen.dCrop, FIXTURES.d, seen.d.paintW));
  problems.push(...anisoRowProblems("D-prime", seen.d2Crop, FIXTURES.d2, seen.d2.paintW));
  return problems;
}

// AC11. A pan drag in the mode keeps the mode, the label and the stored
// extent, and moves the image by the drag delta through the recorded device
// offset while the element box stays put.
export async function observePanKeeps(url, chromium, folder) {
  const browser = await chromium.launch({ headless: true });
  try {
    return await withPage(browser, 2, async (page) => {
      await openFolder(page, url, folder);
      await page.keyboard.press("\\");
      const before = await view(page);
      await page.keyboard.press("p");
      const box = await page.locator('.cell[data-cell="0"] canvas.view').boundingBox();
      await page.mouse.move(box.x + box.width / 2, box.y + box.height / 2);
      await page.mouse.down();
      await page.mouse.move(box.x + box.width / 2 + 20, box.y + box.height / 2, { steps: 4 });
      await page.mouse.up();
      const after = await view(page);
      return { before, after };
    });
  } finally {
    await browser.close();
  }
}

export function panKeepsProblems(seen) {
  const problems = [];
  if (!seen || !seen.before || !seen.after) {
    problems.push("the pan case was not observed");
    return problems;
  }
  if (seen.after.mode !== "one") problems.push(`after a pan drag the mode is ${seen.after.mode}, not one`);
  if (seen.after.cornerLast !== "Zoom 1:1") {
    problems.push(`after a pan drag the corner reads ${JSON.stringify(seen.after.cornerLast)}, not Zoom 1:1`);
  }
  if (seen.after.paintW !== 128 || seen.after.paintH !== 96) {
    problems.push(`after a pan drag the painted box is ${seen.after.paintW} by ${seen.after.paintH}, wanted the stored 128 by 96`);
  }
  const moved = seen.after.paintX - seen.before.paintX;
  if (moved !== 40) problems.push(`the pan moved the painted origin by ${moved} device px, wanted 40`);
  if (Math.abs(seen.after.x - seen.before.x) > 1e-9 || Math.abs(seen.after.y - seen.before.y) > 1e-9) {
    problems.push(`the pan moved the element box (${seen.before.x}, ${seen.before.y}) to (${seen.after.x}, ${seen.after.y}), wanted it still`);
  }
  return problems;
}

// S4, m15. Entering the mode keeps the image point at the cell centre, so
// an entry that zeroes the pan instead of scaling it moves the picture.
export async function observeEntryCentring(url, chromium, folder) {
  const browser = await chromium.launch({ headless: true });
  try {
    return await withPage(browser, 2, async (page) => {
      await openFolder(page, url, folder);
      await page.keyboard.press("p");
      const box = await page.locator('.cell[data-cell="0"] canvas.view').boundingBox();
      await page.mouse.move(box.x + box.width / 2, box.y + box.height / 2);
      await page.mouse.down();
      await page.mouse.move(box.x + box.width / 2 + 30, box.y + box.height / 2 + 18, { steps: 4 });
      await page.mouse.up();
      const under = () => page.evaluate(() => {
        const vp = activeViewport();
        const cell = vp.dom.cell.getBoundingClientRect();
        return clientToImage(vp, cell.left + cell.width / 2, cell.top + cell.height / 2);
      });
      const before = await under();
      await page.keyboard.press("\\");
      const after = await under();
      const state = await view(page);
      return { before, after, state };
    });
  } finally {
    await browser.close();
  }
}

export function entryCentringProblems(seen) {
  const problems = [];
  if (!seen || !seen.before || !seen.after || !seen.state) {
    problems.push("the entry centring case was not observed");
    return problems;
  }
  if (seen.state.mode !== "one") problems.push(`after the entry the mode is ${seen.state.mode}, not one`);
  if (Math.abs(seen.before.x - seen.after.x) > 0.5 || Math.abs(seen.before.y - seen.after.y) > 0.5) {
    problems.push(`the image point at the cell centre moved from (${seen.before.x.toFixed(3)}, ${seen.before.y.toFixed(3)}) to (${seen.after.x.toFixed(3)}, ${seen.after.y.toFixed(3)}), so the entry did not keep the centre`);
  }
  return problems;
}

async function main() {
  const sanity = fixtureSanityProblems();
  if (sanity.length) {
    for (const problem of sanity) console.error(problem);
    process.exitCode = 1;
    return;
  }
  const { chromium } = await loadPlaywright();
  const binary = await ensureViewerBinary("release", true);
  const folder = mkdtempSync(join(tmpdir(), "ocelli-f204-"));
  writeFixtures(folder);
  const server = await startViewer(binary);
  let seen;
  try {
    seen = {
      extents: await observeExtents(server.url, chromium, folder),
      origins: await observeOrigins(server.url, chromium, folder),
      pixels: await observePixels(server.url, chromium, folder),
      labelAndSeries: await observeLabelAndSeries(server.url, chromium, folder),
      ratioWatcher: await observeRatioWatcher(server.url, chromium, folder),
      clamp: await observeClamp(server.url, chromium, folder),
      link: await observeLink(server.url, chromium, folder),
      backingStore: await observeBackingStore(server.url, chromium, folder),
      refusals: await observeRefusals(server.url, chromium, folder),
      belowRatio: await observeBelowRatio(server.url, chromium, folder),
      transforms: await observeTransforms(server.url, chromium, folder),
      aniso: await observeAniso(server.url, chromium, folder),
      panKeeps: await observePanKeeps(server.url, chromium, folder),
      entryCentring: await observeEntryCentring(server.url, chromium, folder),
    };
  } finally {
    await stopByPid(server.pid);
    rmSync(folder, { recursive: true, force: true });
  }
  for (const reading of seen.extents) {
    console.log(`${reading.name}: painted ${reading.paintW} by ${reading.paintH} device px at (${reading.paintX}, ${reading.paintY}), mode ${reading.mode}`);
  }
  console.log(`label: ${JSON.stringify(seen.labelAndSeries.inMode.cornerLast)}, after drag ${JSON.stringify(seen.labelAndSeries.afterDrag.label)}, fit ${seen.labelAndSeries.afterFitButton.mode}, zero ${seen.labelAndSeries.afterZeroKey.mode}, series ${seen.labelAndSeries.afterSeries.mode}`);
  console.log(`ratio watcher: arms ${seen.ratioWatcher.armsAfterTwo} and ${seen.ratioWatcher.armsAfterOne}, resizes ${seen.ratioWatcher.resizeEvents}, probes ${seen.ratioWatcher.probes.map((probe) => `${probe.factor}:${probe.rendering}`).join(" ")}`);
  console.log(`clamp small: fitted ${seen.clamp.small.fitted.toFixed(4)}, identity painted ${seen.clamp.small.identity.paintW}, settled ${(seen.clamp.small.afterOut.w * seen.clamp.small.afterOut.dpr).toFixed(3)}`);
  console.log(`clamp large: fitted ${seen.clamp.large.fitted.toFixed(6)}, identity painted ${seen.clamp.large.identity.paintW}, settled ${(seen.clamp.large.afterOut.w * seen.clamp.large.afterOut.dpr).toFixed(3)}`);
  console.log(`link: ${seen.link.cells.map((cell) => `cell ${cell.cell} ${cell.mode} painted ${cell.paintW} by ${cell.paintH}`).join(", ")}`);
  console.log(`backing store: ${seen.backingStore === null ? "the box times the ratio with the stored bytes at the recorded offset" : seen.backingStore}`);
  console.log(`refusals: tile ${seen.refusals.tiled.mode}, empty ${seen.refusals.empty.mode}, faults ${seen.refusals.tiled.faultShown} and ${seen.refusals.empty.faultShown}`);
  console.log(`below ratio: refused ${JSON.stringify(seen.belowRatio.refused.cornerLast)}, at two ${JSON.stringify(seen.belowRatio.atTwo.cornerLast)}, entered painted ${seen.belowRatio.entered.paintW} device px`);
  console.log(`aniso: D painted ${seen.aniso.d.paintW} by ${seen.aniso.d.paintH}, D-prime painted ${seen.aniso.d2.paintW} by ${seen.aniso.d2.paintH}`);
  console.log(`pan keeps: moved ${seen.panKeeps.after.paintX - seen.panKeeps.before.paintX} device px, mode ${seen.panKeeps.after.mode}`);
  console.log(`entry centring: the image point at the cell centre moved ${Math.abs(seen.entryCentring.before.x - seen.entryCentring.after.x).toFixed(4)} and ${Math.abs(seen.entryCentring.before.y - seen.entryCentring.after.y).toFixed(4)} stored px`);
  const problems = extentProblems(seen.extents)
    .concat(originProblems(seen.origins))
    .concat(pixelProblems(seen.pixels))
    .concat(labelAndSeriesProblems(seen.labelAndSeries))
    .concat(ratioWatcherProblems(seen.ratioWatcher))
    .concat(clampProblems(seen.clamp))
    .concat(linkProblems(seen.link))
    .concat(backingStoreProblems(seen.backingStore))
    .concat(refusalProblems(seen.refusals))
    .concat(belowRatioProblems(seen.belowRatio))
    .concat(transformProblems(seen.transforms))
    .concat(anisoProblems(seen.aniso))
    .concat(panKeepsProblems(seen.panKeeps))
    .concat(entryCentringProblems(seen.entryCentring));
  if (problems.length) {
    for (const problem of problems) console.error(problem);
    process.exitCode = 1;
    return;
  }
  console.log("one stored pixel is one device pixel in the mode, byte for byte at every byte row case in the pinned Chromium under the emulated ratios");
}

if (process.argv[1] === fileURLToPath(import.meta.url)) {
  main().catch((error) => {
    console.error(error);
    process.exitCode = 1;
  });
}
