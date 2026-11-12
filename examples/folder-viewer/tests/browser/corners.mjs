// F-267 AC11 and AC7. The acquisition corner lines of the folder viewer.
//
//   frame0      the page opens its largest series, an enhanced CT of four
//               frames whose per-frame Pixel Measures name 2.0, 3.0, 4.0
//               and 5.0 and whose top level names no thickness. Frame 0
//               must name its own 2 mm.
//   frame3Held  three ArrowDowns reach frame 3, past the prefetched
//               frames, with its preview reply held through the page's
//               route. A repaint through the corner toggle must paint the
//               scan values, which name no thickness here, and never a
//               previous frame's facts.
//   frame3      released, frame 3 must name its own 5 mm and no other
//               frame's value.
//   classicHeld the first series, a classic CT storing Slice Thickness
//               -12.50, KVP 120 and Exposure 200, has its reply held. A
//               repaint must paint its scan values, -12.5 mm, 120 kVp and
//               200 mAs, and never the enhanced frame's facts.
//   classic     released, the corner prints the same values from the reply
//               headers, through `Number`, so -12.5 and never -12.50.
//   mr          the third series, an enhanced MR of one frame whose
//               per-frame MR Timing Sequence names Repetition Time 500 and
//               whose per-frame MR Echo Sequence names Effective Echo Time
//               30, must name its own TR and TE lines. They come only from
//               the reply headers, so a header name the page mistypes or a
//               swapped repetitionTime and echoTime leaves them out.
//
// The files are written by this script into a temporary folder, carry no
// patient attributes, and are removed afterwards. It starts the viewer with
// OCELLI_PORT=0, reads the URL it prints and stops it by its PID, through
// tools/bench's pinned Playwright. `bin/ocelli.sh gate viewer-browser` runs
// it after window.mjs.

import { Buffer } from "node:buffer";
import { mkdtempSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { fileURLToPath } from "node:url";

import { ensureViewerBinary, longEl, shortEl, startViewer } from "../../../../tools/bench/src/viewer_work.mjs";
import { loadPlaywright, stopByPid } from "./aspect.mjs";

/** The classic CT, the enhanced CT and the enhanced MR of the folder. */
const FILES = ["a-classic.dcm", "b-enhanced.dcm", "c-enhanced-mr.dcm"];

/** What each reading must and must not name. */
export const WANTED = {
  frame0: { yes: ["Thickness 2 mm"], no: [] },
  frame3Held: {
    yes: [],
    no: ["Thickness", "Thickness 2 mm", "Thickness 3 mm", "Thickness 4 mm", "-12.5"],
  },
  frame3: {
    yes: ["Thickness 5 mm"],
    no: ["Thickness 2 mm", "Thickness 3 mm", "Thickness 4 mm"],
  },
  classicHeld: {
    yes: ["Thickness -12.5 mm", "120 kVp", "200 mAs"],
    no: ["Thickness 5 mm", "Thickness 4 mm", "Thickness 3 mm", "Thickness 2 mm"],
  },
  classic: { yes: ["Thickness -12.5 mm", "120 kVp", "200 mAs"], no: ["-12.50"] },
  mr: { yes: ["TR 500 ms", "TE 30 ms"], no: [] },
};

export function cornerProblems(seen) {
  const problems = [];
  for (const name of Object.keys(WANTED)) {
    const text = seen[name];
    if (typeof text !== "string") {
      problems.push(`the ${name} corner was not read`);
      continue;
    }
    for (const word of WANTED[name].yes) {
      if (!text.includes(word)) {
        problems.push(`the ${name} corner is ${JSON.stringify(text)}, wanted ${word}`);
      }
    }
    for (const word of WANTED[name].no) {
      if (text.includes(word)) {
        problems.push(`the ${name} corner names ${word}: ${JSON.stringify(text)}`);
      }
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

function item(content) {
  return Buffer.concat([u16(0xfffe), u16(0xe000), u32(content.length), content]);
}

/** The encoded element, in the long form the VRs of PS3.5 7.1.2 require. */
function elementOf({ group, element, vr, value }) {
  return ["OB", "OD", "OF", "OL", "OV", "OW", "SQ", "UC", "UR", "UT", "UN"].includes(vr)
    ? longEl(group, element, vr, value)
    : shortEl(group, element, vr, value);
}

function ordered(elements) {
  return elements
    .slice()
    .sort((a, b) => a.group - b.group || a.element - b.element)
    .map(elementOf);
}

function meta(sopClass, sop) {
  return Buffer.concat([
    longEl(0x0002, 0x0001, "OB", Buffer.from([0, 1])),
    shortEl(0x0002, 0x0002, "UI", uid(sopClass)),
    shortEl(0x0002, 0x0003, "UI", uid(sop)),
    shortEl(0x0002, 0x0010, "UI", uid("1.2.840.10008.1.2.1")),
    shortEl(0x0002, 0x0012, "UI", uid("2.25.213")),
  ]);
}

const PIXELS = 32;

function body(sopClass, sop, series, number, elements) {
  const pixels = Buffer.alloc(PIXELS * PIXELS * 2);
  for (let at = 0; at < PIXELS * PIXELS; at += 1) pixels.writeUInt16LE(1000, at * 2);
  return Buffer.concat([
    Buffer.alloc(128),
    Buffer.from("DICM"),
    shortEl(0x0002, 0x0000, "UL", u32(meta(sopClass, sop).length)),
    meta(sopClass, sop),
    ...ordered([
      { group: 0x0008, element: 0x0016, vr: "UI", value: uid(sopClass) },
      { group: 0x0008, element: 0x0060, vr: "CS", value: text("CT") },
      { group: 0x0020, element: 0x000e, vr: "UI", value: uid(series) },
      { group: 0x0020, element: 0x0011, vr: "IS", value: text(String(number)) },
      { group: 0x0020, element: 0x0013, vr: "IS", value: text("1") },
      { group: 0x0028, element: 0x0002, vr: "US", value: u16(1) },
      { group: 0x0028, element: 0x0004, vr: "CS", value: text("MONOCHROME2") },
      { group: 0x0028, element: 0x0010, vr: "US", value: u16(PIXELS) },
      { group: 0x0028, element: 0x0011, vr: "US", value: u16(PIXELS) },
      { group: 0x0028, element: 0x0100, vr: "US", value: u16(16) },
      { group: 0x0028, element: 0x0101, vr: "US", value: u16(16) },
      { group: 0x0028, element: 0x0102, vr: "US", value: u16(15) },
      { group: 0x0028, element: 0x0103, vr: "US", value: u16(0) },
      { group: 0x0028, element: 0x1050, vr: "DS", value: text("40") },
      { group: 0x0028, element: 0x1051, vr: "DS", value: text("400") },
      { group: 0x0028, element: 0x1052, vr: "DS", value: text("-1024") },
      { group: 0x0028, element: 0x1053, vr: "DS", value: text("1") },
      ...elements,
      { group: 0x7fe0, element: 0x0010, vr: "OW", value: pixels },
    ]),
  ]);
}

/** One classic CT frame with the top-level acquisition attributes planted. */
export function classicCT({ series, sop, number, thickness, kvp, exposure }) {
  const plants = [];
  if (thickness != null) {
    plants.push({ group: 0x0018, element: 0x0050, vr: "DS", value: text(thickness) });
  }
  if (kvp != null) plants.push({ group: 0x0018, element: 0x0060, vr: "DS", value: text(kvp) });
  if (exposure != null) {
    plants.push({ group: 0x0018, element: 0x1152, vr: "IS", value: text(exposure) });
  }
  return body("1.2.840.10008.5.1.4.1.1.2", sop, series, number, plants);
}

/**
 * One enhanced CT of one frame per thickness, each frame's own Pixel
 * Measures Sequence carrying the slice thickness and nothing at the top
 * level, PS3.3 2026c C.7.6.16.2.1 Table C.7.6.16-2.
 */
export function enhancedCT({ series, sop, number, thicknesses }) {
  const sopClass = "1.2.840.10008.5.1.4.1.1.2.1";
  const rescaleGroup = longEl(
    0x0028,
    0x9145,
    "SQ",
    item(Buffer.concat([
      shortEl(0x0028, 0x1052, "DS", text("0")),
      shortEl(0x0028, 0x1053, "DS", text("1")),
    ])),
  );
  const shared = { group: 0x5200, element: 0x9229, vr: "SQ", value: item(rescaleGroup) };
  const perFrameItems = thicknesses.map((thickness) =>
    item(
      longEl(
        0x0028,
        0x9110,
        "SQ",
        item(shortEl(0x0018, 0x0050, "DS", text(thickness))),
      ),
    ),
  );
  const perFrame = {
    group: 0x5200,
    element: 0x9230,
    vr: "SQ",
    value: Buffer.concat(perFrameItems),
  };
  const pixels = Buffer.alloc(PIXELS * PIXELS * 2 * thicknesses.length);
  for (let at = 0; at < PIXELS * PIXELS * thicknesses.length; at += 1) {
    pixels.writeUInt16LE(1000, at * 2);
  }
  return Buffer.concat([
    Buffer.alloc(128),
    Buffer.from("DICM"),
    shortEl(0x0002, 0x0000, "UL", u32(meta(sopClass, sop).length)),
    meta(sopClass, sop),
    ...ordered([
      { group: 0x0008, element: 0x0016, vr: "UI", value: uid(sopClass) },
      { group: 0x0008, element: 0x0060, vr: "CS", value: text("CT") },
      { group: 0x0020, element: 0x000e, vr: "UI", value: uid(series) },
      { group: 0x0020, element: 0x0011, vr: "IS", value: text(String(number)) },
      { group: 0x0020, element: 0x0013, vr: "IS", value: text("1") },
      { group: 0x0028, element: 0x0002, vr: "US", value: u16(1) },
      { group: 0x0028, element: 0x0004, vr: "CS", value: text("MONOCHROME2") },
      { group: 0x0028, element: 0x0008, vr: "IS", value: text(String(thicknesses.length)) },
      { group: 0x0028, element: 0x0010, vr: "US", value: u16(PIXELS) },
      { group: 0x0028, element: 0x0011, vr: "US", value: u16(PIXELS) },
      { group: 0x0028, element: 0x0100, vr: "US", value: u16(16) },
      { group: 0x0028, element: 0x0101, vr: "US", value: u16(16) },
      { group: 0x0028, element: 0x0102, vr: "US", value: u16(15) },
      { group: 0x0028, element: 0x0103, vr: "US", value: u16(0) },
      { group: 0x0028, element: 0x1050, vr: "DS", value: text("40") },
      { group: 0x0028, element: 0x1051, vr: "DS", value: text("400") },
      shared,
      perFrame,
      { group: 0x7fe0, element: 0x0010, vr: "OW", value: pixels },
    ]),
  ]);
}

/**
 * One enhanced MR of one frame whose per-frame item carries the MR Timing
 * and Related Parameters Sequence with Repetition Time and the MR Echo
 * Sequence with Effective Echo Time, PS3.3 2026c C.8.13.5.2 Table C.8-89
 * and C.8.13.5.4 Table C.8-91, and nothing at the top level.
 */
export function enhancedMR({ series, sop, number, timing, echo }) {
  const sopClass = "1.2.840.10008.5.1.4.1.1.4.1";
  const rescaleGroup = longEl(
    0x0028,
    0x9145,
    "SQ",
    item(Buffer.concat([
      shortEl(0x0028, 0x1052, "DS", text("0")),
      shortEl(0x0028, 0x1053, "DS", text("1")),
    ])),
  );
  const shared = { group: 0x5200, element: 0x9229, vr: "SQ", value: item(rescaleGroup) };
  const echoMs = Buffer.alloc(8);
  echoMs.writeDoubleLE(echo);
  const perFrameItem = item(Buffer.concat([
    longEl(
      0x0018,
      0x9112,
      "SQ",
      item(shortEl(0x0018, 0x0080, "DS", text(timing))),
    ),
    longEl(0x0018, 0x9114, "SQ", item(shortEl(0x0018, 0x9082, "FD", echoMs))),
  ]));
  const perFrame = { group: 0x5200, element: 0x9230, vr: "SQ", value: perFrameItem };
  const pixels = Buffer.alloc(PIXELS * PIXELS * 2);
  for (let at = 0; at < PIXELS * PIXELS; at += 1) pixels.writeUInt16LE(1000, at * 2);
  return Buffer.concat([
    Buffer.alloc(128),
    Buffer.from("DICM"),
    shortEl(0x0002, 0x0000, "UL", u32(meta(sopClass, sop).length)),
    meta(sopClass, sop),
    ...ordered([
      { group: 0x0008, element: 0x0016, vr: "UI", value: uid(sopClass) },
      { group: 0x0008, element: 0x0060, vr: "CS", value: text("MR") },
      { group: 0x0020, element: 0x000e, vr: "UI", value: uid(series) },
      { group: 0x0020, element: 0x0011, vr: "IS", value: text(String(number)) },
      { group: 0x0020, element: 0x0013, vr: "IS", value: text("1") },
      { group: 0x0028, element: 0x0002, vr: "US", value: u16(1) },
      { group: 0x0028, element: 0x0004, vr: "CS", value: text("MONOCHROME2") },
      { group: 0x0028, element: 0x0008, vr: "IS", value: text("1") },
      { group: 0x0028, element: 0x0010, vr: "US", value: u16(PIXELS) },
      { group: 0x0028, element: 0x0011, vr: "US", value: u16(PIXELS) },
      { group: 0x0028, element: 0x0100, vr: "US", value: u16(16) },
      { group: 0x0028, element: 0x0101, vr: "US", value: u16(16) },
      { group: 0x0028, element: 0x0102, vr: "US", value: u16(15) },
      { group: 0x0028, element: 0x0103, vr: "US", value: u16(0) },
      { group: 0x0028, element: 0x1050, vr: "DS", value: text("40") },
      { group: 0x0028, element: 0x1051, vr: "DS", value: text("400") },
      shared,
      perFrame,
      { group: 0x7fe0, element: 0x0010, vr: "OW", value: pixels },
    ]),
  ]);
}

function corner(page) {
  return page.evaluate(() => {
    const cell = document.querySelector(".cell.active");
    const node = cell && cell.querySelector(".overlay.bl");
    return node ? node.textContent : "";
  });
}

/** Repaints every viewport through the corner toggle, off and on. */
async function repaint(page) {
  await page.click("#toggle-corners");
  await page.click("#toggle-corners");
  await page.waitForTimeout(100);
}

/** Waits until the corner names `want` or the fault shows. */
async function settle(page, want) {
  await page.waitForFunction((word) => {
    const cell = document.querySelector(".cell.active");
    const node = cell && cell.querySelector(".overlay.bl");
    const fault = cell && cell.querySelector(".fault");
    return (node && (node.textContent || "").includes(word)) || (fault && !fault.hidden);
  }, want, { timeout: 8000 }).catch(() => {});
  await page.waitForTimeout(150);
}

/** The corner of every case, in order. */
export async function observeCorners(url, chromium, folder) {
  const browser = await chromium.launch({ headless: true });
  try {
    const page = await browser.newPage({ viewport: { width: 1400, height: 900 } });
    page.setDefaultTimeout(15000);
    await page.goto(url);
    await page.locator("#folder").setInputFiles(folder);
    await page.waitForFunction(
      () => (document.getElementById("status").textContent || "").startsWith("3 series."),
    );
    await settle(page, "Thickness 2 mm");
    const seen = { frame0: await corner(page) };
    // Every preview reply from here is held until released. The prefetch
    // holds frames 1 and 2, so frame 3 is the one that reaches the route.
    await page.waitForTimeout(500);
    let openGate = null;
    const waits = [];
    await page.route("**/preview", async (route) => {
      if (openGate) await openGate;
      await route.continue();
    });
    const hold = () => {
      openGate = new Promise((resolve) => waits.push(resolve));
    };
    const release = () => {
      openGate = null;
      while (waits.length) waits.pop()();
    };
    hold();
    for (let step = 0; step < 3; step += 1) await page.keyboard.press("ArrowDown");
    await page.waitForTimeout(400);
    await repaint(page);
    seen.frame3Held = await corner(page);
    release();
    await settle(page, "Thickness 5 mm");
    seen.frame3 = await corner(page);
    hold();
    await page.locator("#series-list button", { hasText: "#1" }).click();
    await page.waitForTimeout(400);
    await repaint(page);
    seen.classicHeld = await corner(page);
    release();
    await settle(page, "Thickness -12.5 mm");
    seen.classic = await corner(page);
    // The enhanced MR's TR and TE come only from its reply headers, so
    // their corner lines prove the page parsed both header names and read
    // each into its own key.
    await page.locator("#series-list button", { hasText: "#3" }).click();
    await settle(page, "TR 500 ms");
    seen.mr = await corner(page);
    return seen;
  } finally {
    await browser.close();
  }
}

async function main() {
  const { chromium } = await loadPlaywright();
  const binary = await ensureViewerBinary("release", true);
  const folder = mkdtempSync(join(tmpdir(), "ocelli-f267-"));
  const run = async (observe) => {
    const server = await startViewer(binary);
    try {
      return await observe(server.url);
    } finally {
      await stopByPid(server.pid);
    }
  };
  let seen;
  try {
    writeFileSync(join(folder, FILES[0]), classicCT({
      series: "2.25.21301",
      sop: "2.25.21311",
      number: 1,
      thickness: "-12.50",
      kvp: "120",
      exposure: "200",
    }));
    writeFileSync(join(folder, FILES[1]), enhancedCT({
      series: "2.25.21302",
      sop: "2.25.21312",
      number: 2,
      thicknesses: ["2.0", "3.0", "4.0", "5.0"],
    }));
    writeFileSync(join(folder, FILES[2]), enhancedMR({
      series: "2.25.21303",
      sop: "2.25.21313",
      number: 3,
      timing: "500",
      echo: 30,
    }));
    seen = await run((url) => observeCorners(url, chromium, folder));
  } finally {
    rmSync(folder, { recursive: true, force: true });
  }
  for (const name of Object.keys(WANTED)) {
    console.log(`${name}: ${JSON.stringify(seen[name])}`);
  }
  const problems = cornerProblems(seen);
  if (problems.length) {
    for (const problem of problems) console.error(problem);
    process.exitCode = 1;
    return;
  }
  console.log("the acquisition corners show the shown frame's own values and never a held previous image's facts");
}

if (process.argv[1] === fileURLToPath(import.meta.url)) {
  main().catch((error) => {
    console.error(error);
    process.exitCode = 1;
  });
}
