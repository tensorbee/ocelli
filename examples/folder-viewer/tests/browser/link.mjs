// F-209. A reference line is drawn from the outlined viewport's image. When
// the outlined viewport has no image there is no source, so no line may stay.
//
//   layout   3 by 3, three series in one Frame of Reference, so cells 3 to
//            8 are empty
//   source   the coronal series is opened in the outlined cell 1, and the
//            axial cells show a line drawn from it
//   empty    a click makes empty cell 5 the outlined one
//
// After the click no `.ref-lines` layer may be displayed. The files are
// written by this script into a temporary folder, carry no patient
// attributes, and are removed afterwards. It starts the viewer with
// OCELLI_PORT=0, reads the URL it prints and stops it by its PID, through
// tools/bench's pinned Playwright. `bin/ocelli.sh gate viewer-browser` runs
// it after readout.mjs.

import { Buffer } from "node:buffer";
import { mkdtempSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { fileURLToPath } from "node:url";

import { ensureViewerBinary, longEl, shortEl, startViewer } from "../../../../tools/bench/src/viewer_work.mjs";
import { loadPlaywright, stopByPid } from "./aspect.mjs";

const FRAME_OF_REFERENCE = "2.25.209000";

/**
 * `seen` is `{ name, before, after }`. `before` and `after` are `{ active,
 * lines }`, the outlined cell and how many cells display a reference line.
 */
export function linkProblems(seen) {
  const problems = [];
  if (!seen || !seen.before || !seen.after) {
    problems.push("the empty-source case was not observed");
    return problems;
  }
  if (!(seen.before.lines > 0)) {
    problems.push(`${seen.name}: no reference line showed before the change, so the case shows nothing`);
  } else if (seen.after.lines !== 0) {
    problems.push(`${seen.name}: ${seen.after.lines} reference lines stayed with no source image`);
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

/** One 32 by 32 mono16 CT slice, explicit VR little endian, no patient attributes. */
function slice({ sop, series, number, instance, position, orientation }) {
  const sopClass = "1.2.840.10008.5.1.4.1.1.2";
  const meta = Buffer.concat([
    longEl(0x0002, 0x0001, "OB", Buffer.from([0, 1])),
    shortEl(0x0002, 0x0002, "UI", uid(sopClass)),
    shortEl(0x0002, 0x0003, "UI", uid(sop)),
    shortEl(0x0002, 0x0010, "UI", uid("1.2.840.10008.1.2.1")),
    shortEl(0x0002, 0x0012, "UI", uid("2.25.209")),
  ]);
  const pixels = Buffer.alloc(32 * 32 * 2);
  for (let at = 0; at < 32 * 32; at += 1) pixels.writeUInt16LE(1000 + ((at * 7) % 200), at * 2);
  return Buffer.concat([
    Buffer.alloc(128),
    Buffer.from("DICM"),
    shortEl(0x0002, 0x0000, "UL", u32(meta.length)),
    meta,
    shortEl(0x0008, 0x0016, "UI", uid(sopClass)),
    shortEl(0x0008, 0x0018, "UI", uid(sop)),
    shortEl(0x0008, 0x0060, "CS", text("CT")),
    shortEl(0x0020, 0x000e, "UI", uid(series)),
    shortEl(0x0020, 0x0011, "IS", text(String(number))),
    shortEl(0x0020, 0x0013, "IS", text(String(instance))),
    shortEl(0x0020, 0x0032, "DS", text(position)),
    shortEl(0x0020, 0x0037, "DS", text(orientation)),
    shortEl(0x0020, 0x0052, "UI", uid(FRAME_OF_REFERENCE)),
    shortEl(0x0028, 0x0002, "US", u16(1)),
    shortEl(0x0028, 0x0004, "CS", text("MONOCHROME2")),
    shortEl(0x0028, 0x0010, "US", u16(32)),
    shortEl(0x0028, 0x0011, "US", u16(32)),
    shortEl(0x0028, 0x0030, "DS", text("1\\1")),
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

/** Series 1 and 3 axial at z = 0 to 3, series 2 coronal at y = 0 to 24, rows covering z 31 to 0. */
function writeFolder(folder) {
  const series = [
    { number: 1, orientation: "1\\0\\0\\0\\1\\0", position: (k) => `0\\0\\${k}` },
    { number: 2, orientation: "1\\0\\0\\0\\0\\-1", position: (k) => `0\\${k * 8}\\31` },
    { number: 3, orientation: "1\\0\\0\\0\\1\\0", position: (k) => `0\\0\\${k + 0.5}` },
  ];
  for (const { number, orientation, position } of series) {
    for (let k = 0; k < 4; k += 1) {
      const bytes = slice({
        sop: `2.25.2090${number}${k}`,
        series: `2.25.2090${number}`,
        number,
        instance: k + 1,
        position: position(k),
        orientation,
      });
      writeFileSync(join(folder, `s${number}-${k}.dcm`), bytes);
    }
  }
}

function linesShown(page) {
  return page.evaluate(() => Array.from(document.querySelectorAll(".cell:not([hidden]) .ref-lines"))
    .filter((node) => node.style.display !== "none").length);
}

function activeCellNumber(page) {
  return page.evaluate(() => {
    const node = document.querySelector(".cell.active");
    return node ? Number(node.dataset.cell) : -1;
  });
}

async function clickCell(page, cell) {
  const box = await page.locator(`.cell[data-cell="${cell}"]`).boundingBox();
  await page.mouse.click(box.x + box.width / 2, box.y + box.height / 2);
}

export async function observeEmptySource(url, chromium, folder) {
  const browser = await chromium.launch({ headless: true });
  try {
    const page = await browser.newPage({ viewport: { width: 1600, height: 1000 } });
    await page.goto(url);
    await page.locator("#folder").setInputFiles(folder);
    await page.waitForFunction(() => (document.getElementById("status").textContent || "").startsWith("3 series."));
    await page.locator("#layout").click();
    await page.locator('#layout-picker [aria-label="3 by 3"]').click();
    await clickCell(page, 1);
    await page.locator("#series-list button", { hasText: "#2" }).click();
    await page.waitForFunction(() => Array.from(document.querySelectorAll(".cell:not([hidden]) .ref-lines"))
      .some((node) => node.style.display !== "none"), null, { timeout: 10000 }).catch(() => {});
    const before = { active: await activeCellNumber(page), lines: await linesShown(page) };
    await clickCell(page, 5);
    await page.waitForTimeout(800);
    const after = { active: await activeCellNumber(page), lines: await linesShown(page) };
    return { name: "an empty outlined cell after a coronal source", before, after };
  } finally {
    await browser.close();
  }
}

async function main() {
  const { chromium } = await loadPlaywright();
  const binary = await ensureViewerBinary("release", true);
  const folder = mkdtempSync(join(tmpdir(), "ocelli-f209-"));
  const server = await startViewer(binary);
  let seen;
  try {
    writeFolder(folder);
    seen = await observeEmptySource(server.url, chromium, folder);
  } finally {
    await stopByPid(server.pid);
    rmSync(folder, { recursive: true, force: true });
  }
  console.log(`${seen.name}: before, cell ${seen.before.active} outlined with ${seen.before.lines} lines, after, cell ${seen.after.active} outlined with ${seen.after.lines} lines`);
  const problems = linkProblems(seen);
  if (problems.length) {
    for (const problem of problems) console.error(problem);
    process.exitCode = 1;
    return;
  }
  console.log("an outlined cell with no image leaves no reference line");
}

if (process.argv[1] === fileURLToPath(import.meta.url)) {
  main().catch((error) => {
    console.error(error);
    process.exitCode = 1;
  });
}
