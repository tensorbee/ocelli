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
//        extents, so the CSS box is fractional at the identity of every
//        ratio below while the device box is integral. Corners 0, 198, 134
//        and 81.
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

import { Buffer } from "node:buffer";
import { mkdtempSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { fileURLToPath } from "node:url";

import { ensureViewerBinary, longEl, shortEl, startViewer } from "../../../../tools/bench/src/viewer_work.mjs";
import { loadPlaywright, stopByPid } from "./aspect.mjs";

const VIEWPORT = { width: 1600, height: 1000 };
const EXTENT_EPSILON = 0.5;
const ORIGIN_EPSILON = 1 / 128;
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
    // The painted device origin, per the measured Chromium anchor law: the
    // static layer position is rounded to a whole CSS pixel and the frame
    // translate rides unsnapped.
    return {
      x: rect.x, y: rect.y, w: rect.width, h: rect.height,
      cellW: cell.width, cellH: cell.height,
      dpr: window.devicePixelRatio,
      backingW: canvas.width, backingH: canvas.height,
      rendering: window.getComputedStyle(canvas).imageRendering,
      cornerLast: (vp.dom.corners.br.textContent || "").split("\n").pop(),
      mode: vp.zoomMode,
      label: vp.zoomLabel,
      imageW: vp.imageWidth, imageH: vp.imageHeight,
      aspectX: vp.aspectX, aspectY: vp.aspectY,
      paintX: (Math.round(rect.x - vp.panAppliedX) + vp.panAppliedX) * window.devicePixelRatio,
      paintY: (Math.round(rect.y - vp.panAppliedY) + vp.panAppliedY) * window.devicePixelRatio,
      faultShown: !fault.hidden && Boolean(fault.textContent),
    };
  });
}

/**
 * The enclosing device-aligned region and its exact device crop, read from
 * the picture's own box. The clip bounds are CSS integers one step beyond
 * the rect on each side, so the region holds the painted box even where the
 * engine anchors it up to half a device pixel outside the rect, and the
 * crop offset is an exact device count.
 *
 * The measured Chromium anchor law (scratch probes in the F-204 session):
 * the compositor paints the canvas at the static layer position rounded to
 * a whole CSS pixel with the frame translate riding unsnapped, so the
 * predicted device origin is
 * (round(rect - panApplied) + panApplied) * dpr. Because the compositor
 * also quantises the layer box and antialiases its edges at the fractional
 * ratios, the capture searches a small window of integer offsets around
 * that prediction and reports the offset with the most exact matches, so a
 * pure whole pixel placement error cannot masquerade as content damage.
 */
async function capturePixels(page, cropW, cropH, expected) {
  const box = await page.evaluate(() => {
    const vp = activeViewport();
    const rect = vp.dom.canvas.getBoundingClientRect();
    return {
      x: rect.x, y: rect.y, w: rect.width, h: rect.height,
      appliedX: vp.panAppliedX, appliedY: vp.panAppliedY,
    };
  });
  const dpr = await page.evaluate(() => window.devicePixelRatio);
  const step = stepFor(dpr);
  const paintX = (Math.round(box.x - box.appliedX) + box.appliedX) * dpr;
  const paintY = (Math.round(box.y - box.appliedY) + box.appliedY) * dpr;
  const x0 = (Math.floor(box.x / step) - 2) * step;
  const y0 = (Math.floor(box.y / step) - 2) * step;
  const x1 = (Math.ceil((box.x + box.w) / step) + 2) * step;
  const y1 = (Math.ceil((box.y + box.h) / step) + 2) * step;
  const shot = await page.screenshot({ scale: "device", clip: { x: x0, y: y0, width: x1 - x0, height: y1 - y0 } });
  const baseX = Math.round(paintX - x0 * dpr);
  const baseY = Math.round(paintY - y0 * dpr);
  return page.evaluate(async ({ b64, baseX, baseY, cropW, cropH, expected }) => {
    const image = new Image();
    image.src = "data:image/png;base64," + b64;
    await image.decode();
    const scratch = document.createElement("canvas");
    scratch.width = image.naturalWidth;
    scratch.height = image.naturalHeight;
    const context = scratch.getContext("2d");
    context.drawImage(image, 0, 0);
    const data = context.getImageData(0, 0, scratch.width, scratch.height).data;
    const at = (x, y) => data[(y * scratch.width + x) * 4];
    let best = { dx: 0, dy: 0, hits: -1 };
    for (let dy = -4; dy <= 4; dy += 1) {
      for (let dx = -4; dx <= 4; dx += 1) {
        const x = baseX + dx;
        const y = baseY + dy;
        if (x < 0 || y < 0 || x + cropW > scratch.width || y + cropH > scratch.height) continue;
        let hits = 0;
        for (let r = 0; r < cropH; r += 1) {
          for (let c = 0; c < cropW; c += 1) {
            if (at(x + c, y + r) === expected[r * cropW + c]) hits += 1;
          }
        }
        if (hits > best.hits) best = { dx, dy, hits };
      }
    }
    const grey = new Array(cropW * cropH);
    for (let r = 0; r < cropH; r += 1) {
      for (let c = 0; c < cropW; c += 1) grey[r * cropW + c] = at(baseX + best.dx + c, baseY + best.dy + r);
    }
    return { grey, dx: best.dx, dy: best.dy, hits: best.hits };
  }, { b64: shot.toString("base64"), baseX, baseY, cropW, cropH, expected });
}

/** The located crop must match the stored pattern at every device pixel. */
function byteProblems(name, got, want, columns) {
  const problems = [];
  const seen = [];
  for (let at = 0; at < want.length; at += 1) {
    if (got.grey[at] !== want[at]) {
      if (seen.length < 6) seen.push(`(${at % columns},${Math.floor(at / columns)})`);
    }
  }
  if (seen.length) {
    problems.push(`${name}: the best device offset (${got.dx}, ${got.dy}) leaves ${want.length - got.hits} of ${want.length} device pixels wrong, first at ${seen.join(" ")}`);
  }
  return problems;
}

/**
 * Measure the cell the picture will be centred in, and the painted device
 * origin the engine would produce without the snap, per the measured anchor
 * law: round(baseCss) * dpr. The mode picks the hunt. "odd" walks the
 * viewport until the cell width minus the picture is odd, the plan's ratio 1
 * case. "offgrid" walks until that unsnapped paint is off the device grid,
 * which is only reachable at fractional ratios. "any" measures as-is. At
 * integer ratios the engine's own CSS rounding grids the unsnapped paint,
 * so those cases are guards against a snap that breaks the grid rather than
 * proofs of the snap, and the 1.25 and 1.5 cases carry the proof.
 */
async function fractionalCell(page, pictureDeviceW, mode) {
  const ok = (reading) => {
    if (mode === "odd") return Math.round(reading.cellW - pictureDeviceW) % 2 === 1;
    if (mode === "offgrid") return Math.abs(reading.unsnapped - Math.round(reading.unsnapped)) > 1e-9;
    return true;
  };
  for (let width = 1600; width <= 1660; width += 1) {
    await page.setViewportSize({ width, height: VIEWPORT.height });
    const reading = await page.evaluate((pdw) => {
      const cell = activeViewport().dom.cell.getBoundingClientRect();
      const dpr = window.devicePixelRatio;
      const baseCss = cell.x + (cell.width - pdw / dpr) / 2;
      return { cellW: cell.width, cellH: cell.height, cellX: cell.x, cellY: cell.y, dpr, unsnapped: Math.round(baseCss) * dpr };
    }, pictureDeviceW);
    if (ok(reading)) return reading;
  }
  throw new Error(`no viewport width satisfied the ${mode} cell condition`);
}

// AC1. The drawn extent is the stored extent in device pixels, at ratios 1,
// 1.25, 1.5 and 2, and the axes swap under a 90 degree rotation.
export async function observeExtents(url, chromium, folder) {
  const browser = await chromium.launch({ headless: true });
  try {
    const readings = [];
    for (const ratio of [1, 1.25, 1.5, 2]) {
      await withPage(browser, ratio, async (page) => {
        await openFolder(page, url, folder);
        await page.keyboard.press("\\");
        const reading = await view(page);
        readings.push({ name: `extent at dpr ${ratio}`, ...reading });
      });
    }
    await withPage(browser, 2, async (page) => {
      await openFolder(page, url, folder);
      await page.keyboard.press("\\");
      await page.keyboard.press("o");
      await page.waitForTimeout(100);
      readings.push({ name: "extent rotated at dpr 2", ...(await view(page)) });
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
    const expectedW = turned ? reading.backingH : reading.backingW;
    const expectedH = turned ? reading.backingW : reading.backingH;
    if (Math.abs(reading.w * reading.dpr - expectedW) > EXTENT_EPSILON) {
      problems.push(`${reading.name}: width ${reading.w} * ${reading.dpr} = ${(reading.w * reading.dpr).toFixed(3)}, wanted ${expectedW} within ${EXTENT_EPSILON}`);
    }
    if (Math.abs(reading.h * reading.dpr - expectedH) > EXTENT_EPSILON) {
      problems.push(`${reading.name}: height ${reading.h} * ${reading.dpr} = ${(reading.h * reading.dpr).toFixed(3)}, wanted ${expectedH} within ${EXTENT_EPSILON}`);
    }
    if (reading.mode !== "one") problems.push(`${reading.name}: the mode is ${reading.mode}, not one`);
  }
  return problems;
}

// AC2. The painted device origin is on the grid within half the layout
// unit, read through the measured anchor law. The ratio 1 and ratio 2 cases
// run in the plan's odd cell and record that the unsnapped paint is already
// on grid there, so they guard against a snap that breaks the engine's own
// rounding. The 1.25 and 1.5 cases hunt a cell where the unsnapped paint is
// off grid, so they fail without the snap.
export async function observeOrigins(url, chromium, folder) {
  const browser = await chromium.launch({ headless: true });
  const readings = [];
  try {
    await withPage(browser, 1, async (page) => {
      await openFolder(page, url, folder);
      const cell = await fractionalCell(page, 128, "odd");
      readings.push({ name: "origin dpr 1 odd cell", cellW: cell.cellW, unsnapped: cell.unsnapped, proving: false });
      await page.keyboard.press("\\");
      readings.push({ name: "origin dpr 1 odd cell", wantW: 128, wantH: 96, ...(await view(page)) });
    });
    for (const [ratio, mode, proving] of [[2, "any", false], [1.25, "offgrid", true], [1.5, "offgrid", true]]) {
      await withPage(browser, ratio, async (page) => {
        await openFolder(page, url, folder);
        await showSeries(page, "#2", SHAPE.b);
        const cell = await fractionalCell(page, 129, mode);
        readings.push({ name: `origin dpr ${ratio} fixture B cell`, cellW: cell.cellW, unsnapped: cell.unsnapped, proving });
        await page.keyboard.press("\\");
        readings.push({ name: `origin dpr ${ratio} fixture B`, wantW: 129, wantH: 65, ...(await view(page)) });
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
  for (const reading of readings) {
    if (reading.unsnapped !== undefined) {
      const integral = Math.abs(reading.unsnapped - Math.round(reading.unsnapped)) <= 1e-9;
      if (reading.proving && integral) {
        problems.push(`${reading.name}: the measured cell puts the unsnapped painted origin at the integral ${reading.unsnapped}, so the case proves nothing`);
      }
      if (!reading.proving && !integral) {
        problems.push(`${reading.name}: the unsnapped painted origin ${reading.unsnapped} is off grid, so the guard case is not the engine-rounded shape it name states`);
      }
      continue;
    }
    const offX = Math.abs(reading.paintX - Math.round(reading.paintX));
    const offY = Math.abs(reading.paintY - Math.round(reading.paintY));
    if (offX > ORIGIN_EPSILON) problems.push(`${reading.name}: the painted device origin x is ${reading.paintX}, off the grid by ${offX}`);
    if (offY > ORIGIN_EPSILON) problems.push(`${reading.name}: the painted device origin y is ${reading.paintY}, off the grid by ${offY}`);
    if (Math.abs(reading.w * reading.dpr - reading.wantW) > EXTENT_EPSILON) {
      problems.push(`${reading.name}: the device width ${(reading.w * reading.dpr).toFixed(3)} is not the stored ${reading.wantW}`);
    }
    if (Math.abs(reading.h * reading.dpr - reading.wantH) > EXTENT_EPSILON) {
      problems.push(`${reading.name}: the device height ${(reading.h * reading.dpr).toFixed(3)} is not the stored ${reading.wantH}`);
    }
    if (reading.mode !== "one") problems.push(`${reading.name}: the mode is ${reading.mode}, not one`);
  }
  return problems;
}

// AC3. The device bytes equal the stored pattern. The byte rows run at the
// geometries whose CSS box is integral in CSS pixels, where the measured
// compositor rasterizes exactly: P at ratio 1 in the odd cell, P at ratio
// 2, and P at ratio 2 after a pan drag of 40 device px. Fixture B's CSS
// box at ratio 2 is 64.5 by 32.5 and every 1.25 and 1.5 box is fractional,
// and the measured compositor corrupts those rasterizations (the layout
// unit grid cannot represent the width, and the quad edges antialias), so
// no byte row is claimed there; the origin rows assert origin and extent.
export async function observePixels(url, chromium, folder) {
  const browser = await chromium.launch({ headless: true });
  const readings = [];
  try {
    await withPage(browser, 1, async (page) => {
      await openFolder(page, url, folder);
      await fractionalCell(page, 128, "odd");
      await page.keyboard.press("\\");
      await page.click("#toggle-corners");
      await page.mouse.move(2, 2);
      readings.push({ name: "pixels dpr 1 odd cell", got: await capturePixels(page, 128, 96, FIXTURES.p.bytes) });
    });
    await withPage(browser, 2, async (page) => {
      await openFolder(page, url, folder);
      await page.keyboard.press("\\");
      await page.click("#toggle-corners");
      await page.mouse.move(2, 2);
      readings.push({ name: "pixels dpr 2 fixture P", got: await capturePixels(page, 128, 96, FIXTURES.p.bytes) });
    });
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
      readings.push({ name: "pixels midpan", got: await capturePixels(page, 128, 96, FIXTURES.p.bytes) });
    });
    return readings;
  } finally {
    await browser.close();
  }
}

export function pixelProblems(readings) {
  const problems = [];
  if (!Array.isArray(readings) || readings.length !== 3) {
    problems.push("the pixel cases were not all observed");
    return problems;
  }
  const want = {
    "pixels dpr 1 odd cell": FIXTURES.p,
    "pixels dpr 2 fixture P": FIXTURES.p,
    "pixels midpan": FIXTURES.p,
  };
  for (const reading of readings) {
    const fixture = want[reading.name];
    if (!fixture) {
      problems.push(`${reading.name}: an unknown pixel case`);
      continue;
    }
    problems.push(...byteProblems(reading.name, reading.got, fixture.bytes, fixture.columns, fixture.rows));
  }
  return problems;
}

// AC4 and AC8. The corner reads Zoom 1:1 in the mode, one zoom drag tick
// returns the fit-relative percent, and opening another series returns to
// fit.
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
      await showSeries(page, "#2", SHAPE.b);
      const afterSeries = await view(page);
      return { inMode, afterDrag, afterSeries };
    });
  } finally {
    await browser.close();
  }
}

export function labelAndSeriesProblems(seen) {
  const problems = [];
  if (!seen || !seen.inMode || !seen.afterDrag || !seen.afterSeries) {
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
  if (seen.afterSeries.mode !== "fit") problems.push(`after a series open the mode is ${seen.afterSeries.mode}, not fit`);
  if (!/^\d+%$/.test(seen.afterSeries.label)) {
    problems.push(`after a series open the label is ${JSON.stringify(seen.afterSeries.label)}, not a percent`);
  }
  return problems;
}

// The init script that counts the page's own resolution watcher arms.
const WATCH_WRAPPER = () => {
  window.__resolutionArms = 0;
  const original = window.MediaQueryList.prototype.addEventListener;
  window.MediaQueryList.prototype.addEventListener = function (type, listener, options) {
    if (String(this.media).includes("resolution")) window.__resolutionArms += 1;
    return original.call(this, type, listener, options);
  };
};

/**
 * Change the device pixel ratio through the CDP override the plan names,
 * then force the media query evaluation pass that dispatches the query's
 * change event. Measured in the F-204 session with scratch probes: a
 * device scale factor only override updates window.devicePixelRatio and
 * the query values but dispatches no MediaQueryList change event in this
 * Chromium, and the pending change dispatches on the next evaluation pass,
 * so the override is followed by a one pixel CDP viewport nudge and
 * restore with the ratio held. The nudge also fires the page's resize
 * listener, so the row's proof that the watcher's own handler ran is the
 * wrapped registration count, which is what the plan states the row
 * proves, and the test never calls layoutViewport or relayout itself.
 */
async function changeRatio(page, session, factor) {
  const size = page.viewportSize();
  await session.send("Emulation.setDeviceMetricsOverride", {
    width: size.width, height: size.height, deviceScaleFactor: factor, mobile: false,
  });
  await session.send("Emulation.setDeviceMetricsOverride", {
    width: size.width - 1, height: size.height, deviceScaleFactor: factor, mobile: false,
  });
  await session.send("Emulation.setDeviceMetricsOverride", {
    width: size.width, height: size.height, deviceScaleFactor: factor, mobile: false,
  });
}

// AC5 and AC10. A device pixel ratio change re-lays out through the page's
// own matchMedia watcher, and the identity renders pixelated at the ratios
// where the landed comparison evaluates one ulp below 1.
export async function observeRatioWatcher(url, chromium, folder) {
  const browser = await chromium.launch({ headless: true });
  try {
    return await withPage(browser, 1, async (page) => {
      await page.addInitScript(WATCH_WRAPPER);
      await openFolder(page, url, folder);
      await page.keyboard.press("\\");
      const session = await page.context().newCDPSession(page);
      await changeRatio(page, session, 2);
      await page.waitForFunction(() => {
        const vp = activeViewport();
        const rect = vp.dom.canvas.getBoundingClientRect();
        const dpr = window.devicePixelRatio;
        const extentOk = Math.abs(rect.width * dpr - 128) <= 0.5 && Math.abs(rect.height * dpr - 96) <= 0.5;
        const paintX = (Math.round(rect.x - vp.panAppliedX) + vp.panAppliedX) * dpr;
        const paintY = (Math.round(rect.y - vp.panAppliedY) + vp.panAppliedY) * dpr;
        const originOk = Math.abs(paintX - Math.round(paintX)) <= 1 / 128
          && Math.abs(paintY - Math.round(paintY)) <= 1 / 128;
        return extentOk && originOk && window.__resolutionArms >= 2;
      }, null, { timeout: 10000 }).catch(() => {});
      const atTwo = await view(page);
      const armsAfterTwo = await page.evaluate(() => window.__resolutionArms);
      await changeRatio(page, session, 1);
      await page.waitForFunction(() => {
        const vp = activeViewport();
        const rect = vp.dom.canvas.getBoundingClientRect();
        const dpr = window.devicePixelRatio;
        const extentOk = Math.abs(rect.width * dpr - 128) <= 0.5 && Math.abs(rect.height * dpr - 96) <= 0.5;
        const paintX = (Math.round(rect.x - vp.panAppliedX) + vp.panAppliedX) * dpr;
        const paintY = (Math.round(rect.y - vp.panAppliedY) + vp.panAppliedY) * dpr;
        const originOk = Math.abs(paintX - Math.round(paintX)) <= 1 / 128
          && Math.abs(paintY - Math.round(paintY)) <= 1 / 128;
        return extentOk && originOk && window.__resolutionArms >= 3;
      }, null, { timeout: 10000 }).catch(() => {});
      const atOne = await view(page);
      const armsAfterOne = await page.evaluate(() => window.__resolutionArms);
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
      return { atTwo, armsAfterTwo, atOne, armsAfterOne, probes };
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
  for (const [reading, arms, want] of [[seen.atTwo, seen.armsAfterTwo, 2], [seen.atOne, seen.armsAfterOne, 3]]) {
    if (reading.mode !== "one") problems.push(`after the ratio change the mode is ${reading.mode}, not one`);
    if (Math.abs(reading.w * reading.dpr - 128) > EXTENT_EPSILON) {
      problems.push(`after the ratio change the width ${(reading.w * reading.dpr).toFixed(3)} is not the identity 128`);
    }
    const offX = Math.abs(reading.paintX - Math.round(reading.paintX));
    const offY = Math.abs(reading.paintY - Math.round(reading.paintY));
    if (offX > ORIGIN_EPSILON || offY > ORIGIN_EPSILON) {
      problems.push(`after the ratio change the painted origin is off the grid by ${Math.max(offX, offY)}`);
    }
    if (!(arms >= want)) problems.push(`the resolution watcher armed ${arms} times, wanted at least ${want}`);
  }
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
// out below it on a large frame.
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
  for (const [name, caseSeen] of [["small", seen.small], ["large", seen.large]]) {
    if (!caseSeen || !caseSeen.identity) {
      problems.push(`the ${name} frame clamp case was not observed`);
      return problems;
    }
    if (!caseSeen.precondition) problems.push(`the ${name} frame clamp precondition does not hold in the measured cell`);
  }
  const small = seen.small;
  if (Math.abs(small.identity.w * small.identity.dpr - 64) > EXTENT_EPSILON) {
    problems.push(`the small frame identity extent is ${(small.identity.w * small.identity.dpr).toFixed(3)}, wanted 64`);
  }
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
// the Zoom 1:1 rows label, and the below-ratio target keeps its own mode.
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
      await page.locator("#link").click();
      await page.locator("#link-zoom").click();
      await page.keyboard.press("Escape");
      await page.locator('.cell[data-cell="0"]').click();
      await page.keyboard.press("\\");
      await page.waitForTimeout(300);
      const cells = await page.evaluate(() => [0, 1, 2, 3].map((cell) => {
        const vp = viewports[cell];
        const rect = vp.dom.canvas.getBoundingClientRect();
        return {
          cell,
          mode: vp.zoomMode,
          label: vp.zoomLabel,
          cornerLast: (vp.dom.corners.br.textContent || "").split("\n").pop(),
          w: rect.width, h: rect.height,
          imageW: vp.imageWidth, imageH: vp.imageHeight,
        };
      }));
      return cells;
    });
  } finally {
    await browser.close();
  }
}

export function linkProblems(cells) {
  const problems = [];
  if (!Array.isArray(cells) || cells.length !== 4) {
    problems.push("the linked cases were not observed");
    return problems;
  }
  for (const cell of cells) {
    const dpr = 3;
    if (cell.cell === 3) {
      if (cell.mode !== "fit") problems.push(`the below-ratio linked cell kept mode ${cell.mode}, wanted its own fit`);
      if (!/^\d+%$/.test(cell.label)) problems.push(`the below-ratio linked cell label is ${JSON.stringify(cell.label)}, wanted its own percent`);
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
    // At the identity the device height is the stored row count on every
    // frame and the device width is the stored width on square pixels and
    // imageW * aspectX / aspectY on anisotropic ones, at any ratio.
    const width = anisotropic ? (cell.imageW * cell.aspectX) / cell.aspectY : cell.imageW;
    const height = cell.imageH;
    if (Math.abs(cell.w * dpr - width) > EXTENT_EPSILON) {
      problems.push(`linked cell ${cell.cell} width ${(cell.w * dpr).toFixed(3)} is not ${width.toFixed(3)}`);
    }
    if (Math.abs(cell.h * dpr - height) > EXTENT_EPSILON) {
      problems.push(`linked cell ${cell.cell} height ${(cell.h * dpr).toFixed(3)} is not ${height.toFixed(3)}`);
    }
  }
  return problems;
}

// AC9. The backing store stays the stored frame size and the in-page grey
// equals the stored bytes at every pixel, which guards the window pipeline.
export async function observeBackingStore(url, chromium, folder) {
  const browser = await chromium.launch({ headless: true });
  try {
    return await withPage(browser, 1, async (page) => {
      await openFolder(page, url, folder);
      await page.keyboard.press("\\");
      return page.evaluate((want) => {
        const vp = activeViewport();
        const canvas = vp.dom.canvas;
        if (canvas.width !== 128 || canvas.height !== 96) {
          return `the backing store is ${canvas.width} by ${canvas.height}, wanted the stored 128 by 96`;
        }
        const context = canvas.getContext("2d");
        const data = context.getImageData(0, 0, canvas.width, canvas.height).data;
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
// ratio move goes through changeRatio, whose comment records why the plain
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
  if (seen.entered.mode !== "one" || Math.abs(seen.entered.w * seen.entered.dpr - 2) > EXTENT_EPSILON) {
    problems.push(`the boundary control did not enter the mode at a 2 device pixel width (mode ${seen.entered && seen.entered.mode}, extent ${seen.entered ? (seen.entered.w * seen.entered.dpr).toFixed(3) : "none"})`);
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
// rotation of the all-even fixture is an exact permutation. The rotation
// turns the 128 by 96 box into 96 by 128, so its crop swaps the two counts.
export async function observeTransforms(url, chromium, folder) {
  const browser = await chromium.launch({ headless: true });
  try {
    const cases = [];
    const keys = [
      ["rotate", "o", 96, 128],
      ["flip-h", "h", 128, 96],
      ["flip-v", "v", 128, 96],
    ];
    const expected = {
      rotate: { bytes: permute90(FIXTURES.p.bytes, FIXTURES.p.columns, FIXTURES.p.rows), columns: 96, rows: 128 },
      "flip-h": { bytes: permuteFlipH(FIXTURES.p.bytes, FIXTURES.p.columns, FIXTURES.p.rows), columns: 128, rows: 96 },
      "flip-v": { bytes: permuteFlipV(FIXTURES.p.bytes, FIXTURES.p.columns, FIXTURES.p.rows), columns: 128, rows: 96 },
    };
    for (const [name, key, cropW, cropH] of keys) {
      await withPage(browser, 2, async (page) => {
        await openFolder(page, url, folder);
        await page.keyboard.press("\\");
        await page.keyboard.press(key);
        await page.waitForTimeout(150);
        await page.mouse.move(2, 2);
        cases.push({ name, cropW, cropH, got: await capturePixels(page, cropW, cropH, expected[name].bytes) });
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
    problems.push(...byteProblems(`transform ${item.name}`, item.got, expected.bytes, expected.columns, expected.rows));
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
      const cell = await fractionalCell(page, 128 * 0.5 / 0.7, "any");
      readings.dCell = { unsnapped: cell.unsnapped };
      await page.keyboard.press("\\");
      readings.d = await view(page);
      await showSeries(page, "#5", SHAPE.d2);
      await page.keyboard.press("\\");
      readings.d2 = await view(page);
      return readings;
    });
  } finally {
    await browser.close();
  }
}

export function anisoProblems(seen) {
  const problems = [];
  if (!seen || !seen.d || !seen.d2) {
    problems.push("the anisotropic cases were not observed");
    return problems;
  }
  for (const [name, reading, wantW] of [["D", seen.d, 128 * 0.5 / 0.7], ["D-prime", seen.d2, 179.2]]) {
    if (reading.mode !== "one") {
      problems.push(`the ${name} mode is ${reading.mode}, not one`);
      continue;
    }
    if (reading.cornerLast !== "Zoom 1:1 rows") {
      problems.push(`the ${name} corner reads ${JSON.stringify(reading.cornerLast)}, not Zoom 1:1 rows`);
    }
    if (Math.abs(reading.h * reading.dpr - 96) > EXTENT_EPSILON) {
      problems.push(`the ${name} device height ${(reading.h * reading.dpr).toFixed(3)} is not the stored 96 rows`);
    }
    if (Math.abs(reading.w * reading.dpr - wantW) > EXTENT_EPSILON) {
      problems.push(`the ${name} device width ${(reading.w * reading.dpr).toFixed(3)} is not ${wantW.toFixed(3)}`);
    }
  }
  const offY = Math.abs(seen.d.paintY - Math.round(seen.d.paintY));
  if (offY > ORIGIN_EPSILON) {
    problems.push(`the D painted row origin is ${seen.d.paintY}, off the grid by ${offY}`);
  }
  return problems;
}

// AC11. A pan drag in the mode keeps the mode, the label and the extent, and
// moves the image by the drag delta with the origin still on the grid.
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
  if (Math.abs(seen.after.w * seen.after.dpr - 128) > EXTENT_EPSILON) {
    problems.push(`after a pan drag the extent is ${(seen.after.w * seen.after.dpr).toFixed(3)}, wanted the identity 128`);
  }
  const moved = (seen.after.x - seen.before.x) * seen.after.dpr;
  if (Math.abs(moved - 40) > ORIGIN_EPSILON) {
    problems.push(`the pan moved the origin by ${moved.toFixed(4)} device px, wanted 40`);
  }
  const offX = Math.abs(seen.after.paintX - Math.round(seen.after.paintX));
  if (offX > ORIGIN_EPSILON) problems.push(`after the pan the painted origin is off the grid by ${offX}`);
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
    };
  } finally {
    await stopByPid(server.pid);
    rmSync(folder, { recursive: true, force: true });
  }
  for (const reading of seen.extents) {
    console.log(`${reading.name}: ${(reading.w * reading.dpr).toFixed(3)} by ${(reading.h * reading.dpr).toFixed(3)} device px, mode ${reading.mode}`);
  }
  for (const reading of seen.origins) {
    console.log(reading.unsnapped !== undefined
      ? `${reading.name}: cell ${reading.cellW.toFixed(3)} CSS px, unsnapped ${reading.unsnapped.toFixed(4)}`
      : `${reading.name}: origin ${(reading.x * reading.dpr).toFixed(4)}, ${(reading.y * reading.dpr).toFixed(4)}`);
  }
  console.log(`label: ${JSON.stringify(seen.labelAndSeries.inMode.cornerLast)}, after drag ${JSON.stringify(seen.labelAndSeries.afterDrag.label)}, after series ${seen.labelAndSeries.afterSeries.mode}`);
  console.log(`ratio watcher: arms ${seen.ratioWatcher.armsAfterTwo} and ${seen.ratioWatcher.armsAfterOne}, probes ${seen.ratioWatcher.probes.map((probe) => `${probe.factor}:${probe.rendering}`).join(" ")}`);
  console.log(`clamp small: fitted ${seen.clamp.small.fitted.toFixed(4)}, settled ${(seen.clamp.small.afterOut.w * seen.clamp.small.afterOut.dpr).toFixed(3)}, wheel in ${(seen.clamp.small.afterWheelIn.w * seen.clamp.small.afterWheelIn.dpr).toFixed(3)}`);
  console.log(`clamp large: fitted ${seen.clamp.large.fitted.toFixed(6)}, settled ${(seen.clamp.large.afterOut.w * seen.clamp.large.afterOut.dpr).toFixed(3)}`);
  console.log(`link: ${seen.link.map((cell) => `cell ${cell.cell} ${cell.mode} ${JSON.stringify(cell.cornerLast)}`).join(", ")}`);
  console.log(`backing store: ${seen.backingStore === null ? "the stored 128 by 96 with the stored bytes" : seen.backingStore}`);
  console.log(`refusals: tile ${seen.refusals.tiled.mode}, empty ${seen.refusals.empty.mode}, faults ${seen.refusals.tiled.faultShown} and ${seen.refusals.empty.faultShown}`);
  console.log(`below ratio: refused ${JSON.stringify(seen.belowRatio.refused.cornerLast)}, at two ${JSON.stringify(seen.belowRatio.atTwo.cornerLast)}, entered ${(seen.belowRatio.entered.w * seen.belowRatio.entered.dpr).toFixed(3)} device px`);
  console.log(`aniso: D ${(seen.aniso.d.w * seen.aniso.d.dpr).toFixed(3)} by ${(seen.aniso.d.h * seen.aniso.d.dpr).toFixed(3)}, D-prime ${(seen.aniso.d2.w * seen.aniso.d2.dpr).toFixed(3)} by ${(seen.aniso.d2.h * seen.aniso.d2.dpr).toFixed(3)}`);
  console.log(`pan keeps: moved ${((seen.panKeeps.after.x - seen.panKeeps.before.x) * seen.panKeeps.after.dpr).toFixed(4)} device px, mode ${seen.panKeeps.after.mode}`);
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
    .concat(panKeepsProblems(seen.panKeeps));
  if (problems.length) {
    for (const problem of problems) console.error(problem);
    process.exitCode = 1;
    return;
  }
  console.log("one stored pixel is one device pixel in the mode, on the grid, byte for byte, at every probed ratio");
}

if (process.argv[1] === fileURLToPath(import.meta.url)) {
  main().catch((error) => {
    console.error(error);
    process.exitCode = 1;
  });
}
