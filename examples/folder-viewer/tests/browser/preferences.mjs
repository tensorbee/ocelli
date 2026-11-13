// F-205. Validated shared preferences in the single showcase viewer.
//
//   schema   one version 1 document under `ocelli.prefs` with exactly the
//            five fields mouseProfile, cineFps, hideSeries, hideInspector
//            and defaultPreset, each from a closed set. The four legacy keys
//            ocelli.mouse, ocelli.fps, ocelli.no-series and
//            ocelli.no-inspector stay beside it and are rewritten from the
//            document on every successful document write. A refused document
//            is never rewritten.
//   apply    a stored default preset is tried once per series load, after
//            the viewport has moved, through chooseWindow with propagate
//            false. While the trial runs, the first-paint hold gates every
//            paint site of the loading viewport, so the series is never
//            drawn at the file window and then again at the preset.
//   fixtures CT slices of 32 by 32, stored 224 at columns 0 to 15 (HU -800
//            with slope 1 and intercept -1024) and stored 1064 at columns 16
//            to 31 (HU 40). PS3.3 C.11.2.1.2.1 LINEAR,
//            y = ((x - (c - 0.5)) / (w - 1) + 0.5) * 255 inside the window,
//            and display_index addresses the largest n with n <= y + 0.5.
//            At the file window 40 and 400: HU 40 gives
//            ((40 - 39.5) / 399 + 0.5) * 255 = 127.8, which addresses 128,
//            and HU -800 clamps to 0. At the lung preset -600 and 1500:
//            HU 40 gives ((40 + 600.5) / 1499 + 0.5) * 255 = 236.4, which
//            addresses 236, and HU -800 gives 93.5, which addresses 94.
//            These are the plan's hand values, typed here, not read back
//            from the page.
//   ports    the viewer starts on a fixed port found by binding port 0 and
//            closing the probe, skipping the reserved 8742 and 8765. It
//            stops by PID. Nothing is left running.
//
// The files are written by this script into a temporary folder, carry no
// patient attributes, and are removed afterwards. `bin/ocelli.sh gate
// viewer-browser` runs it.

import { Buffer } from "node:buffer";
import { createServer } from "node:net";
import { mkdtempSync, mkdirSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { fileURLToPath } from "node:url";

import { colourFile } from "./colour.mjs";
import { startSyntheticDicomweb } from "./dicomweb.mjs";
import { enhanced } from "./window.mjs";
import { ensureViewerBinary, longEl, shortEl, startViewer } from "../../../../tools/bench/src/viewer_work.mjs";
import { loadPlaywright, stopByPid } from "./aspect.mjs";

const ALLOWED_KEYS = ["ocelli.prefs", "ocelli.mouse", "ocelli.fps", "ocelli.no-series", "ocelli.no-inspector"];
const REFUSED_DOCUMENT = "The saved preferences could not be used, so the viewer's usual choices apply and the saved document is left unchanged.";
const NOT_OFFERED = "The saved default preset is not offered on this image, so it is not applied.";
const KEPT_BUT_NOT_COPIED = "The choice was kept in this browser, but the copy that older versions of this viewer read was not updated.";
const LOST_RATE = "This browser did not keep the play rate. It applies until you reload.";
const MOUSE_UNSAVED = "This browser did not keep the choice. It applies until you reload.";
// The five fields in stored order, so an exact-document assertion reads the
// same string the page writes.
function documentJSON(values) {
  return JSON.stringify({
    version: 1,
    mouseProfile: values.mouseProfile,
    cineFps: values.cineFps,
    hideSeries: values.hideSeries,
    hideInspector: values.hideInspector,
    defaultPreset: values.defaultPreset
  });
}
const PLAIN_DOC = { mouseProfile: "ocelli", cineFps: 10, hideSeries: false, hideInspector: false, defaultPreset: "none" };

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
 * One 32 by 32 mono16 slice, explicit VR little endian, no patient
 * attributes. `luts` adds a top-level VOI LUT Sequence with one explained
 * item, which disables every window row of the file.
 */
function slice({ sop, series, number, seriesNumber = "1", modality = "CT", centers = "40", widths = "400", luts = 0 }) {
  const sopClass = "1.2.840.10008.5.1.4.1.1.2";
  const meta = Buffer.concat([
    longEl(0x0002, 0x0001, "OB", Buffer.from([0, 1])),
    shortEl(0x0002, 0x0002, "UI", uid(sopClass)),
    shortEl(0x0002, 0x0003, "UI", uid(sop)),
    shortEl(0x0002, 0x0010, "UI", uid("1.2.840.10008.1.2.1")),
    shortEl(0x0002, 0x0012, "UI", uid("2.25.213")),
  ]);
  const pixels = Buffer.alloc(32 * 32 * 2);
  for (let at = 0; at < 32 * 32; at += 1) pixels.writeUInt16LE(at % 32 < 16 ? 224 : 1064, at * 2);
  const item = (content) => Buffer.concat([u16(0xfffe), u16(0xe000), u32(content.length), content]);
  const lutItems = [];
  for (let at = 0; at < luts; at += 1) {
    lutItems.push(item(Buffer.concat([
      shortEl(0x0028, 0x3002, "US", Buffer.concat([u16(4), u16(0), u16(16)])),
      shortEl(0x0028, 0x3003, "LO", text(`PERFUSION ${at + 1}`)),
      longEl(0x0028, 0x3006, "OW", Buffer.concat([u16(0), u16(85), u16(170), u16(255)])),
    ])));
  }
  return Buffer.concat([
    Buffer.alloc(128),
    Buffer.from("DICM"),
    shortEl(0x0002, 0x0000, "UL", u32(meta.length)),
    meta,
    shortEl(0x0008, 0x0016, "UI", uid(sopClass)),
    shortEl(0x0008, 0x0018, "UI", uid(sop)),
    shortEl(0x0008, 0x0060, "CS", text(modality)),
    shortEl(0x0020, 0x000e, "UI", uid(series)),
    shortEl(0x0020, 0x0011, "IS", text(seriesNumber)),
    shortEl(0x0020, 0x0013, "IS", text(String(number))),
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
    ...(luts ? [longEl(0x0028, 0x3010, "SQ", Buffer.concat(lutItems))] : []),
    longEl(0x7fe0, 0x0010, "OW", pixels),
  ]);
}

function writeFolder(root, name, files) {
  const folder = join(root, name);
  mkdirSync(folder, { recursive: true });
  for (const [file, bytes] of Object.entries(files)) writeFileSync(join(folder, file), bytes);
  return folder;
}

function makeFolders(root) {
  const ctSlice = (series, number) => slice({ sop: `2.25.2170${number}`, series, number });
  const ct = (series, count) => Array.from({ length: count }, (_, at) => ctSlice(series, at + 1));
  return {
    // Two CT slices of one series, for the applied default, Reset, scroll
    // and play, and for the exact stored document.
    two: writeFolder(root, "two", { "ct-1.dcm": ctSlice("2.25.21701", 1), "ct-2.dcm": ctSlice("2.25.21701", 2) }),
    // Four CT slices, for the tile-mode hold case.
    four: writeFolder(root, "four", Object.fromEntries(ct("2.25.21801", 4).map((bytes, at) => [`ct-${at + 1}.dcm`, bytes]))),
    // Three CT series of three, two and two slices, for the pre-move case
    // and for the not-offered and no-propagation cases.
    trio: writeFolder(root, "trio", {
      "a-1.dcm": slice({ sop: "2.25.21411", series: "2.25.21401", number: 1, seriesNumber: "1" }),
      "a-2.dcm": slice({ sop: "2.25.21412", series: "2.25.21401", number: 2, seriesNumber: "1" }),
      "a-3.dcm": slice({ sop: "2.25.21413", series: "2.25.21401", number: 3, seriesNumber: "1" }),
      "b-1.dcm": slice({ sop: "2.25.21421", series: "2.25.21402", number: 1, seriesNumber: "2" }),
      "b-2.dcm": slice({ sop: "2.25.21422", series: "2.25.21402", number: 2, seriesNumber: "2" }),
      "c-1.dcm": slice({ sop: "2.25.21431", series: "2.25.21403", number: 1, seriesNumber: "3" }),
      "c-2.dcm": slice({ sop: "2.25.21432", series: "2.25.21403", number: 2, seriesNumber: "3" }),
    }),
    // One MR slice, whose modality offers no presets.
    mr: writeFolder(root, "mr", { "mr-1.dcm": slice({ sop: "2.25.21501", series: "2.25.21501", number: 1, seriesNumber: "2", modality: "MR", centers: "500", widths: "1000" }) }),
    // One CT slice with a VOI LUT Sequence, whose rows are disabled.
    voi: writeFolder(root, "voi", { "voi-1.dcm": slice({ sop: "2.25.21601", series: "2.25.21601", number: 1, luts: 1 }) }),
    // One RGB image, whose presets do not apply either.
    colour: writeFolder(root, "colour", { "rgb-1.dcm": colourFile({ name: "rgb", columns: 4, samples: [10, 20, 30, 200, 100, 50] }, 1) }),
    // Two CT slices and one MR slice, for the superseding-load case.
    ctMr: writeFolder(root, "ctmr", { "ct-1.dcm": ctSlice("2.25.21701", 1), "ct-2.dcm": ctSlice("2.25.21701", 2), "mr-1.dcm": slice({ sop: "2.25.21501", series: "2.25.21501", number: 1, seriesNumber: "2", modality: "MR", centers: "500", widths: "1000" }) }),
    // One four-frame enhanced CT whose every frame stores 40\-600 and
    // 400\1500 explained A{k}\B{k}, so frames 1 to 3 are asked by /inspect,
    // for the held-inspect tile case.
    frames: writeFolder(root, "frames", { "tiles.dcm": enhanced([0, 1, 2, 3].map((k) => ({ centers: "40\\-600", widths: "400\\1500", explanation: `A${k}\\B${k}` }))) }),
  };
}

function freePort() {
  return new Promise((resolve, reject) => {
    const probe = createServer();
    probe.once("error", reject);
    probe.listen(0, "127.0.0.1", () => {
      const address = probe.address();
      const port = typeof address === "object" && address !== null ? address.port : 0;
      probe.close(() => {
        if (port === 8742 || port === 8765) resolve(freePort());
        else resolve(port);
      });
    });
  });
}

/** Reads the outlined cell's corner line, its fault and the two greys. */
async function sample(page) {
  return page.evaluate(() => {
    const cell = document.querySelector(".cell.active");
    if (!cell) return { corner: "", fault: "no cell", soft: null, lung: null };
    const cornerNode = cell.querySelector(".overlay.br");
    const corner = cornerNode ? (cornerNode.textContent || "").split("\n")[0] : "";
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
    if (!cell) return false;
    const line = (cell.querySelector(".overlay.br").textContent || "").split("\n")[0];
    const fault = cell.querySelector(".fault");
    return line.endsWith("  " + word) || (fault && !fault.hidden);
  }, want, { timeout: 8000 }).catch(() => {});
  await page.waitForTimeout(150);
}

async function noteLines(page) {
  return page.evaluate(() => ({
    hidden: document.getElementById("prefs-note").hidden,
    lines: Array.from(document.querySelectorAll("#prefs-note span")).map((node) => node.textContent),
  }));
}

async function stored(page) {
  return page.evaluate(() => ({
    document: document.defaultView.localStorage.getItem("ocelli.prefs"),
    keys: Object.keys(document.defaultView.localStorage).sort(),
    legacy: {
      mouse: document.defaultView.localStorage.getItem("ocelli.mouse"),
      fps: document.defaultView.localStorage.getItem("ocelli.fps"),
      noSeries: document.defaultView.localStorage.getItem("ocelli.no-series"),
      noInspector: document.defaultView.localStorage.getItem("ocelli.no-inspector"),
    },
  }));
}

/** Plants a value in storage before the page's own scripts run. */
function plant(page, values) {
  return page.addInitScript((entries) => {
    for (const [key, value] of Object.entries(entries)) {
      if (value === null) document.defaultView.localStorage.removeItem(key);
      else document.defaultView.localStorage.setItem(key, value);
    }
  }, values);
}

async function setRate(page, value) {
  await page.locator("#fps").evaluate((el, v) => {
    el.value = String(v);
    el.dispatchEvent(new Event("input", { bubbles: true }));
    el.dispatchEvent(new Event("change", { bubbles: true }));
  }, value);
}

async function openFolder(page, folder, series) {
  await page.locator("#folder").setInputFiles(folder);
  await page.waitForFunction(() => (document.getElementById("status").textContent || "").includes("series."), null, { timeout: 15000 });
  if (series) await settle(page, series);
  else await page.waitForTimeout(400);
}

/** Waits for the active cell's fault to show and settle. */
async function settleFault(page) {
  await page.waitForFunction(() => {
    const cell = document.querySelector(".cell.active");
    const fault = cell && cell.querySelector(".fault");
    return fault && !fault.hidden && Boolean(fault.textContent);
  }, null, { timeout: 8000 }).catch(() => {});
  await page.waitForTimeout(150);
}

/** Counts grey paints of the view canvases and of the tiles. The corner
 * label is read one task after each paint, so paintOverlays of the same
 * applyGrey has run and the label names the window just drawn. */
const PAINT_PROBE = `
  window.__paints = 0;
  window.__paintLabels = [];
  window.__tilePaints = 0;
  const original = CanvasRenderingContext2D.prototype.putImageData;
  CanvasRenderingContext2D.prototype.putImageData = function (...args) {
    const canvas = this.canvas;
    if (canvas) {
      if (canvas.classList.contains("view")) {
        window.__paints += 1;
        window.setTimeout(() => {
          const cell = canvas.closest(".cell");
          const corner = cell ? cell.querySelector(".overlay.br") : null;
          window.__paintLabels.push(corner ? (corner.textContent || "").split("\\n")[0] : "");
        }, 0);
      }
      if (canvas.hasAttribute("data-tile")) window.__tilePaints += 1;
    }
    return original.apply(this, args);
  };
`;

/** Records the #counter text with a timestamp, so a running cine's step rate
 * is measurable. A poll, because the element does not exist at init time. */
const STEP_PROBE = `
  window.__steps = [];
  window.__lastStep = null;
  window.setInterval(() => {
    const node = document.getElementById("counter");
    const text = node ? node.textContent : "";
    if (text !== window.__lastStep) {
      window.__lastStep = text;
      window.__steps.push(Date.now());
    }
  }, 10);
`;

/** One page against one origin, with an optional storage plant. */
async function withPage(browser, body, plantValues) {
  const context = await browser.newContext({ viewport: { width: 1400, height: 900 } });
  const page = await context.newPage();
  if (plantValues) plant(page, plantValues);
  try {
    return await body(page, context);
  } finally {
    await context.close();
  }
}

// AC1. The exact stored document after a full interaction pass, with a
// planted extra field that the next write removes.
async function observeExactDocument(url, browser, folders) {
  return withPage(browser, async (page) => {
    await plant(page, { "ocelli.prefs": documentJSON({ ...PLAIN_DOC, defaultPreset: "bone", extra: "/secret/studies" }) });
    await page.goto(url);
    await page.locator("#mouse-profile").selectOption("scroll-left");
    await setRate(page, 12);
    await page.click("#toggle-series");
    await page.locator("#default-preset").selectOption("bone");
    await openFolder(page, folders.two, "Window 1");
    return stored(page);
  });
}

function exactDocumentProblems(seen) {
  const problems = [];
  const want = documentJSON({ mouseProfile: "scroll-left", cineFps: 12, hideSeries: true, hideInspector: false, defaultPreset: "bone" });
  if (seen.document !== want) {
    problems.push(`the stored document is ${seen.document}, not the exact ${want}`);
  }
  return problems;
}

// AC2. Legacy keys only, all valid. The choices apply, the document is
// written from them, and all four legacy keys are rewritten from it.
async function observeLegacyMigration(url, browser) {
  return withPage(browser, async (page) => {
    await plant(page, { "ocelli.mouse": "window-left", "ocelli.fps": "7", "ocelli.no-series": "1", "ocelli.no-inspector": "0" });
    await page.goto(url);
    const controls = await page.evaluate(() => ({
      profile: document.getElementById("mouse-profile").value,
      rate: document.getElementById("fps").value,
      defaultPreset: document.getElementById("default-preset").value,
      noSeries: document.body.classList.contains("no-series"),
      noInspector: document.body.classList.contains("no-inspector"),
      tool: Array.from(document.querySelectorAll('button[id^="tool-"].active')).map((node) => node.id),
    }));
    const note = await noteLines(page);
    const saved = await stored(page);
    return { controls, note, saved };
  });
}

function legacyMigrationProblems(seen) {
  const problems = [];
  const { controls, note, saved } = seen;
  if (controls.profile !== "window-left") problems.push(`the migrated profile is ${controls.profile}, not window-left`);
  if (controls.rate !== "7") problems.push(`the migrated rate is ${controls.rate}, not 7`);
  if (controls.defaultPreset !== "none") problems.push(`the migrated default preset is ${controls.defaultPreset}, not none`);
  if (!controls.noSeries) problems.push("the migrated series column is not hidden");
  if (controls.noInspector) problems.push("the tools column is hidden without a stored choice");
  if (!controls.tool.includes("tool-wl")) problems.push(`the migrated profile does not start on W/L: ${JSON.stringify(controls.tool)}`);
  if (!note.hidden && note.lines.length) {
    problems.push(`the all-valid migration named ${JSON.stringify(note.lines)}`);
  }
  const want = documentJSON({ mouseProfile: "window-left", cineFps: 7, hideSeries: true, hideInspector: false, defaultPreset: "none" });
  if (saved.document !== want) problems.push(`the migration stored ${saved.document}, not ${want}`);
  if (saved.legacy.mouse !== "window-left" || saved.legacy.fps !== "7" || saved.legacy.noSeries !== "1" || saved.legacy.noInspector !== "0") {
    problems.push(`the migration left the legacy keys ${JSON.stringify(saved.legacy)}`);
  }
  return problems;
}

// AC3. Legacy and document both present. The document wins, and one change
// rewrites all four legacy keys to agree with it.
async function observeBothPresent(url, browser) {
  return withPage(browser, async (page) => {
    await plant(page, {
      "ocelli.mouse": "scroll-left",
      "ocelli.fps": "5",
      "ocelli.no-series": "0",
      "ocelli.no-inspector": "0",
      "ocelli.prefs": documentJSON({ mouseProfile: "window-left", cineFps: 20, hideSeries: true, hideInspector: false, defaultPreset: "none" }),
    });
    await page.goto(url);
    const controls = await page.evaluate(() => ({
      profile: document.getElementById("mouse-profile").value,
      rate: document.getElementById("fps").value,
      noSeries: document.body.classList.contains("no-series"),
    }));
    await setRate(page, 9);
    return { controls, saved: await stored(page) };
  });
}

function bothPresentProblems(seen) {
  const problems = [];
  const { controls, saved } = seen;
  if (controls.profile !== "window-left") problems.push(`the document's profile did not win: ${controls.profile}`);
  if (controls.rate !== "20") problems.push(`the document's rate did not win: ${controls.rate}`);
  if (!controls.noSeries) problems.push("the document's column choice did not win");
  const want = documentJSON({ mouseProfile: "window-left", cineFps: 9, hideSeries: true, hideInspector: false, defaultPreset: "none" });
  if (saved.document !== want) problems.push(`after one change the document is ${saved.document}, not ${want}`);
  if (saved.legacy.mouse !== "window-left" || saved.legacy.fps !== "9" || saved.legacy.noSeries !== "1" || saved.legacy.noInspector !== "0") {
    problems.push(`after one change the legacy keys are ${JSON.stringify(saved.legacy)}, not the document's values`);
  }
  return problems;
}

// AC4. Every refused shape: one sentence, the legacy keys used, the stored
// document byte-identical after a change, and a default preset change in
// the refused state that is kept nowhere.
const REFUSED_SHAPES = [
  ["unparsable", "not json {"],
  ["null", "null"],
  ["array", "[1,2]"],
  ["number", "42"],
  ["version 2", JSON.stringify({ ...PLAIN_DOC, version: 2 })],
  ["version string", JSON.stringify({ ...PLAIN_DOC, version: "1" })],
  ["version missing", JSON.stringify({ mouseProfile: "ocelli", cineFps: 10, hideSeries: false, hideInspector: false, defaultPreset: "none" })],
];

async function observeRefusedShapes(url, browser) {
  const seen = [];
  for (const [name, shape] of REFUSED_SHAPES) {
    seen.push(await withPage(browser, async (page) => {
      await plant(page, {
        "ocelli.mouse": "scroll-left",
        "ocelli.fps": "7",
        "ocelli.no-series": "1",
        "ocelli.no-inspector": "0",
        "ocelli.prefs": shape,
      });
      await page.goto(url);
      const before = {
        note: await noteLines(page),
        profile: await page.evaluate(() => document.getElementById("mouse-profile").value),
        rate: await page.evaluate(() => document.getElementById("fps").value),
        noSeries: await page.evaluate(() => document.body.classList.contains("no-series")),
      };
      await page.locator("#mouse-profile").selectOption("window-left");
      const after = await stored(page);
      await page.locator("#default-preset").selectOption("lung");
      const presetNote = await noteLines(page);
      const presetSaved = await page.evaluate(() => document.defaultView.localStorage.getItem("ocelli.prefs"));
      return { name, before, after, presetNote, presetSaved };
    }));
  }
  return seen;
}

function refusedShapeProblems(seen) {
  const problems = [];
  for (const { name, before, after, presetNote, presetSaved } of seen) {
    if (before.note.hidden || !before.note.lines.includes(REFUSED_DOCUMENT)) {
      problems.push(`${name}: the note reads ${JSON.stringify(before.note)}, not the refused-document sentence`);
    }
    if (before.profile !== "scroll-left") problems.push(`${name}: the legacy profile was not used: ${before.profile}`);
    if (before.rate !== "7") problems.push(`${name}: the legacy rate was not used: ${before.rate}`);
    if (!before.noSeries) problems.push(`${name}: the legacy column choice was not used`);
    if (after.document !== REFUSED_SHAPES.find(([label]) => label === name)[1]) {
      problems.push(`${name}: the refused document was rewritten to ${after.document}`);
    }
    if (!presetNote.lines.some((line) => line === "This browser did not keep the default preset. It applies until you reload.")) {
      problems.push(`${name}: a default preset change while refused shows ${JSON.stringify(presetNote)}, not the not-kept sentence`);
    }
    if (presetSaved !== after.document) {
      problems.push(`${name}: the refused document changed to ${presetSaved} after the preset change`);
    }
  }
  return problems;
}

// AC5. An invalid stored value per field names that field and applies the
// default, and the note is visible while the tools column is hidden.
async function observeFieldRefusals(url, browser) {
  const cases = [
    ["profile", { "ocelli.prefs": documentJSON({ ...PLAIN_DOC, mouseProfile: "radiant" }) }, null],
    ["rate", { "ocelli.prefs": documentJSON({ ...PLAIN_DOC, cineFps: "10" }) }, null],
    ["series column", { "ocelli.prefs": documentJSON({ ...PLAIN_DOC, hideSeries: "1" }) }, null],
    ["default preset", { "ocelli.prefs": documentJSON({ ...PLAIN_DOC, defaultPreset: "lung_x" }) }, null],
    ["legacy rate", { "ocelli.fps": "45" }, "10"],
    ["legacy series column", { "ocelli.no-series": "2" }, "0"],
  ];
  const seen = [];
  for (const [name, planted, repaired] of cases) {
    seen.push(await withPage(browser, async (page) => {
      await plant(page, planted);
      await page.goto(url);
      return {
        name,
        note: await noteLines(page),
        repaired,
        after: repaired === null ? null : await stored(page),
        rate: await page.evaluate(() => document.getElementById("fps").value),
        profile: await page.evaluate(() => document.getElementById("mouse-profile").value),
        defaultPreset: await page.evaluate(() => document.getElementById("default-preset").value),
        noSeries: await page.evaluate(() => document.body.classList.contains("no-series")),
      };
    }));
  }
  // The note's bounding box, with hideInspector planted true and a refused
  // mouse profile naming the sentence beside it.
  seen.push(await withPage(browser, async (page) => {
    await plant(page, { "ocelli.prefs": documentJSON({ ...PLAIN_DOC, mouseProfile: "radiant", hideInspector: true }) });
    await page.goto(url);
    return {
      name: "bounding box",
      note: await noteLines(page),
      box: await page.evaluate(() => {
        const node = document.getElementById("prefs-note");
        const inner = window.innerWidth;
        const innerHeight = window.innerHeight;
        const box = node.getBoundingClientRect();
        return {
          width: box.width,
          height: box.height,
          inside: box.left >= 0 && box.top >= 0 && box.right <= inner && box.bottom <= innerHeight,
        };
      }),
      noInspector: await page.evaluate(() => document.body.classList.contains("no-inspector")),
    };
  }));
  return seen;
}

function fieldRefusalProblems(seen) {
  const problems = [];
  const sentences = {
    profile: "The saved mouse profile is not one this viewer knows, so the Ocelli profile is used.",
    rate: "The saved play rate is not a whole number from 1 to 30, so 10 is used.",
    "series column": "The saved series column choice is not readable, so the series list is shown.",
    "default preset": "The saved default preset is not one this viewer knows, so no default preset is applied.",
    "legacy rate": "The saved play rate is not a whole number from 1 to 30, so 10 is used.",
    "legacy series column": "The saved series column choice is not readable, so the series list is shown.",
  };
  const defaults = {
    profile: ["ocelli", "10", false],
    rate: ["ocelli", "10", false],
    "series column": ["ocelli", "10", false],
    "default preset": ["ocelli", "10", false],
    "legacy rate": ["ocelli", "10", false],
    "legacy series column": ["ocelli", "10", false],
  };
  for (const { name, note, repaired, after, rate, profile, defaultPreset, noSeries } of seen) {
    if (name === "bounding box") continue;
    if (note.hidden || !note.lines.includes(sentences[name])) {
      problems.push(`${name}: the note reads ${JSON.stringify(note)}, not "${sentences[name]}"`);
    }
    const [wantProfile, wantRate, wantNoSeries] = defaults[name];
    if (profile !== wantProfile) problems.push(`${name}: the profile is ${profile}, not ${wantProfile}`);
    if (rate !== wantRate) problems.push(`${name}: the rate is ${rate}, not ${wantRate}`);
    if (Boolean(noSeries) !== wantNoSeries) problems.push(`${name}: the series column is ${noSeries}`);
    if (name === "default preset" && defaultPreset !== "none") {
      problems.push(`${name}: the default preset is ${defaultPreset}, not none`);
    }
    if (repaired !== null && after.legacy[name === "legacy rate" ? "fps" : "noSeries"] !== repaired) {
      problems.push(`${name}: the rewritten legacy key is ${JSON.stringify(after.legacy)}, not ${repaired}`);
    }
  }
  const box = seen.find(({ name }) => name === "bounding box");
  if (!box) {
    problems.push("the bounding box case was not observed");
  } else {
    if (!box.note.lines.includes(sentences.profile)) {
      problems.push(`the bounding box case shows ${JSON.stringify(box.note)}, not the profile sentence`);
    }
    if (!box.noInspector) problems.push("the bounding box case did not hide the tools column");
    if (!(box.box.width > 0 && box.box.height > 0 && box.box.inside)) {
      problems.push(`the note's box is ${JSON.stringify(box.box)}, not non-zero and inside the viewport`);
    }
  }
  return problems;
}

// AC6. A throwing setItem before the page's scripts: every control still
// applies, and each failed write names its own sentence.
async function observeWriteFailures(url, browser) {
  return withPage(browser, async (page) => {
    await page.addInitScript(() => {
      Storage.prototype.setItem = function () {
        throw new Error("this browser refused the write");
      };
    });
    await page.goto(url);
    await page.locator("#mouse-profile").selectOption("window-left");
    const profile = {
      note: await noteLines(page),
      unsaved: await page.evaluate(() => !document.getElementById("mouse-unsaved").hidden),
      unsavedText: await page.evaluate(() => document.getElementById("mouse-unsaved").textContent),
      profile: await page.evaluate(() => document.getElementById("mouse-profile").value),
    };
    await setRate(page, 20);
    const rate = { note: await noteLines(page), rate: await page.evaluate(() => document.getElementById("fps").value) };
    await page.click("#toggle-inspector");
    const inspector = {
      note: await noteLines(page),
      hidden: await page.evaluate(() => document.body.classList.contains("no-inspector")),
      box: await page.evaluate(() => {
        const node = document.getElementById("prefs-note");
        const box = node.getBoundingClientRect();
        return {
          width: box.width,
          height: box.height,
          inside: box.left >= 0 && box.top >= 0 && box.right <= window.innerWidth && box.bottom <= window.innerHeight,
        };
      }),
    };
    await page.click("#toggle-series");
    const series = { note: await noteLines(page), hidden: await page.evaluate(() => document.body.classList.contains("no-series")) };
    await page.locator("#default-preset").selectOption("bone");
    const preset = { note: await noteLines(page), value: await page.evaluate(() => document.getElementById("default-preset").value) };
    return { profile, rate, inspector, series, preset };
  });
}

function writeFailureProblems(seen) {
  const problems = [];
  const { profile, rate, inspector, series, preset } = seen;
  if (!profile.unsaved) problems.push("a failed profile write did not keep the mouse-unsaved sentence");
  if (profile.unsavedText !== MOUSE_UNSAVED) {
    problems.push(`the mouse-unsaved paragraph reads ${JSON.stringify(profile.unsavedText)}`);
  }
  if (profile.profile !== "window-left") problems.push(`the profile choice did not apply: ${profile.profile}`);
  if (rate.rate !== "20") problems.push(`the rate choice did not apply: ${rate.rate}`);
  if (!rate.note.lines.some((line) => line === LOST_RATE)) {
    problems.push(`the failed rate write shows ${JSON.stringify(rate.note)}, not the play rate sentence`);
  }
  if (!inspector.hidden) problems.push("the tools column did not hide");
  if (!inspector.note.lines.some((line) => line === "This browser did not keep the tools column choice. It applies until you reload.")) {
    problems.push(`the failed column write shows ${JSON.stringify(inspector.note)}, not the tools column sentence`);
  }
  if (!(inspector.box.width > 0 && inspector.box.height > 0 && inspector.box.inside)) {
    problems.push(`the note's box under the hidden tools column is ${JSON.stringify(inspector.box)}`);
  }
  if (!series.hidden) problems.push("the series column did not hide");
  if (!series.note.lines.some((line) => line === "This browser did not keep the series column choice. It applies until you reload.")) {
    problems.push(`the failed series write shows ${JSON.stringify(series.note)}`);
  }
  if (preset.value !== "bone") problems.push(`the default preset choice did not apply: ${preset.value}`);
  if (!preset.note.lines.some((line) => line === "This browser did not keep the default preset. It applies until you reload.")) {
    problems.push(`the failed preset write shows ${JSON.stringify(preset.note)}`);
  }
  return problems;
}

// AC18. A key-selective throwing setItem. One key's failure names its own
// partial sentence and no other.
async function observePartialWrites(url, browser) {
  const cases = [
    ["legacy rate", "ocelli.fps", [KEPT_BUT_NOT_COPIED]],
    ["document", "ocelli.prefs", [LOST_RATE]],
    ["unrelated key", "ocelli.absent", []],
  ];
  const seen = [];
  for (const [name, blocked, want] of cases) {
    seen.push(await withPage(browser, async (page) => {
      await page.addInitScript((key) => {
        const original = Storage.prototype.setItem;
        Storage.prototype.setItem = function (blocked_, value) {
          if (blocked_ === key) throw new Error("this browser refused the write");
          return original.call(this, blocked_, value);
        };
      }, blocked);
      await page.goto(url);
      await setRate(page, 21);
      return { name, note: await noteLines(page), want };
    }));
  }
  // The mouse profile's failure keeps its own paragraph and never a note
  // line of its own. With the document written the choice is kept, so
  // mouse-unsaved hides and the legacy partial line names the stale copy.
  seen.push(await withPage(browser, async (page) => {
    await page.addInitScript((key) => {
      const original = Storage.prototype.setItem;
      Storage.prototype.setItem = function (blocked_, value) {
        if (blocked_ === key) throw new Error("this browser refused the write");
        return original.call(this, blocked_, value);
      };
    }, "ocelli.mouse");
    await page.goto(url);
    await page.locator("#mouse-profile").selectOption("window-left");
    return {
      name: "mouse key",
      note: await noteLines(page),
      unsaved: await page.evaluate(() => !document.getElementById("mouse-unsaved").hidden),
      want: [KEPT_BUT_NOT_COPIED],
    };
  }));
  return seen;
}

function partialWriteProblems(seen) {
  const problems = [];
  for (const { name, note, want, unsaved } of seen) {
    const lines = note.hidden ? [] : note.lines;
    if (JSON.stringify(lines) !== JSON.stringify(want)) {
      problems.push(`blocking ${name} shows ${JSON.stringify(lines)}, not ${JSON.stringify(want)}`);
    }
    if (name === "mouse key" && unsaved) {
      problems.push("blocking the mouse key showed mouse-unsaved although the document kept the choice");
    }
  }
  return problems;
}

// S4, review pass 1. A default preset whose document write failed stays in
// effect for this session, and a later successful write of another field
// never replaces it. The one-shot refusal refuses only the first write,
// which is the preset document's, so the later rate write succeeds and its
// merged write re-reads the document's still-valid none. The session keeps
// applying Bone, which the CT load then applies at the corner.
async function observeUntouchedInEffect(url, browser, folders) {
  return withPage(browser, async (page) => {
    await plant(page, { "ocelli.prefs": documentJSON(PLAIN_DOC) });
    await page.addInitScript(() => {
      const original = Storage.prototype.setItem;
      let refused = true;
      Storage.prototype.setItem = function (key, value) {
        if (refused) {
          refused = false;
          throw new Error("this browser refused the write");
        }
        return original.call(this, key, value);
      };
    });
    await page.goto(url);
    await page.locator("#default-preset").selectOption("bone");
    await setRate(page, 20);
    const note = await noteLines(page);
    await openFolder(page, folders.two, "Bone");
    const corner = await sample(page);
    return { note, corner };
  });
}

function untouchedInEffectProblems(seen) {
  const problems = [];
  if (!seen.note.lines.some((line) => line === "This browser did not keep the default preset. It applies until you reload.")) {
    problems.push(`the failed preset write shows ${JSON.stringify(seen.note)}, not the not-kept sentence`);
  }
  if (seen.corner.corner !== "WW 1500  WL 300  Bone" || seen.corner.soft !== 83 || seen.corner.lung !== 0) {
    problems.push(`the session kept ${JSON.stringify(seen.corner)}, not Bone with greys 83 and 0`);
  }
  return problems;
}

// AC15. Two pages in one context. A change is neither lost nor stale, the
// in-helper re-read keeps the other tab's field, and the storage listener
// refreshes without writing and keeps the tool and the running cine.
async function observeCrossTab(url, browser, folders) {
  const browserContext = await browser.newContext({ viewport: { width: 1400, height: 900 } });
  try {
    const a = await browserContext.newPage();
    const b = await browserContext.newPage();
    await a.goto(url);
    await b.goto(url);
    await a.locator("#mouse-profile").selectOption("scroll-left");
    await setRate(b, 7);
    const fresh = await (async () => {
      const page = await browserContext.newPage();
      await page.goto(url);
      const read = await page.evaluate(() => ({
        profile: document.getElementById("mouse-profile").value,
        rate: document.getElementById("fps").value,
      }));
      await page.close();
      return read;
    })();
    // The re-read alone: the rate-changing page drops every storage event
    // before its own listener can run, so only the helper's re-read inside
    // its save can keep the profile page's change.
    const isolated = await withPage(browser, async (page, context2) => {
      const other = await context2.newPage();
      await other.addInitScript(() => {
        const original = window.addEventListener;
        window.addEventListener = function (type, listener, options) {
          if (type === "storage") return undefined;
          return original.call(this, type, listener, options);
        };
      });
      await page.goto(url);
      await other.goto(url);
      await page.locator("#mouse-profile").selectOption("scroll-left");
      await setRate(other, 7);
      const third = await context2.newPage();
      await third.goto(url);
      const read = await third.evaluate(() => ({
        profile: document.getElementById("mouse-profile").value,
        rate: document.getElementById("fps").value,
      }));
      await context2.close();
      return read;
    });
    // The listener: page B spies on setItem, page A holds the tool and a
    // running cine, and B's writes refresh A without a write.
    const listener = await (async () => {
      const context3 = await browser.newContext({ viewport: { width: 1400, height: 900 } });
      try {
        const a = await context3.newPage();
        const b = await context3.newPage();
        await a.addInitScript(PAINT_PROBE);
        await a.addInitScript(STEP_PROBE);
        await a.addInitScript(() => {
          window.__writes = [];
          const original = Storage.prototype.setItem;
          Storage.prototype.setItem = function (key, value) {
            if (this === document.defaultView.localStorage) window.__writes.push(key);
            return original.call(this, key, value);
          };
        });
        await a.goto(url);
        await b.goto(url);
        await openFolder(a, folders.two, "Window 1");
        // A tool that is none of window-left's three bindings, start
        // included, so a listener that applies a profile binding goes red.
        await a.keyboard.press("l");
        await a.click("#play");
        await a.waitForTimeout(500);
        await b.locator("#mouse-profile").selectOption("window-left");
        await setRate(b, 25);
        await a.waitForTimeout(700);
        const after = await a.evaluate(() => ({
          writes: window.__writes,
          profile: document.getElementById("mouse-profile").value,
          rate: document.getElementById("fps").value,
          rateLabel: document.getElementById("fps-value").textContent,
          tool: Array.from(document.querySelectorAll('button[id^="tool-"].active')).map((node) => node.id),
          steps: window.__steps,
        }));
        // The positive control: A's own change registers a write.
        await setRate(a, 22);
        const own = await a.evaluate(() => window.__writes);
        await a.keyboard.press("Space");
        await context3.close();
        return { after, own };
      } finally {
        await context3.close();
      }
    })();
    await browserContext.close();
    return { fresh, isolated, listener };
  } catch (error) {
    await browserContext.close();
    throw error;
  }
}

function crossTabProblems(seen) {
  const problems = [];
  const { fresh, isolated, listener } = seen;
  if (fresh.profile !== "scroll-left" || fresh.rate !== "7") {
    problems.push(`the fresh page reads ${JSON.stringify(fresh)}, not A's scroll-left and B's 7`);
  }
  if (isolated.profile !== "scroll-left" || isolated.rate !== "7") {
    problems.push(`with the listener dropped the fresh page reads ${JSON.stringify(isolated)}, so the helper did not re-read`);
  }
  const { after, own } = listener;
  if (after.writes.length) {
    problems.push(`the listener wrote ${JSON.stringify(after.writes)} while refreshing`);
  }
  if (after.profile !== "window-left") problems.push(`the listener did not refresh the profile: ${after.profile}`);
  if (after.rate !== "25") problems.push(`the listener did not refresh the rate: ${after.rate}`);
  if (after.rateLabel !== "25") problems.push(`the listener did not refresh the rate label: ${after.rateLabel}`);
  // The listener keeps the current tool, so the length tool A chose before
  // the write survives, and the new profile's start tool does not appear.
  if (!after.tool.includes("tool-length") || after.tool.includes("tool-wl")) {
    problems.push(`the listener changed the active tool: ${JSON.stringify(after.tool)}`);
  }
  const steps = after.steps;
  if (steps.length < 4) {
    problems.push(`the running cine recorded ${steps.length} steps, so its rate was not observed`);
  } else {
    const gaps = steps.slice(1).map((at, at_) => at - steps[at_]);
    const late = gaps.filter((gap) => gap > 75).length;
    const early = gaps.filter((gap) => gap <= 75).length;
    if (!early) problems.push(`no cine step ran at the new 40 ms rate, gaps ${JSON.stringify(gaps)}`);
    if (late === gaps.length) problems.push(`every cine step kept the old 100 ms rate, gaps ${JSON.stringify(gaps)}`);
  }
  if (!own.includes("ocelli.prefs")) problems.push(`the positive control registered ${JSON.stringify(own)}, not the document key`);
  return problems;
}

// N1, review pass 1. The listener's refresh replaces the profile, so a
// mouse-unsaved sentence left by this tab's own failed write goes with it:
// the profile this tab now holds is the document's. The one-shot refusal
// eats this tab's first write, which is its own profile document's.
async function observeListenerClearsUnsaved(url, browser) {
  return withPage(browser, async (page, context) => {
    await page.addInitScript(() => {
      const original = Storage.prototype.setItem;
      let refused = true;
      Storage.prototype.setItem = function (key, value) {
        if (refused) {
          refused = false;
          throw new Error("this browser refused the write");
        }
        return original.call(this, key, value);
      };
    });
    const other = await context.newPage();
    await page.goto(url);
    await other.goto(url);
    await page.locator("#mouse-profile").selectOption("scroll-left");
    const shown = await page.evaluate(() => !document.getElementById("mouse-unsaved").hidden);
    await other.locator("#mouse-profile").selectOption("window-left");
    await page.waitForTimeout(300);
    const after = {
      unsaved: await page.evaluate(() => !document.getElementById("mouse-unsaved").hidden),
      profile: await page.evaluate(() => document.getElementById("mouse-profile").value),
    };
    await context.close();
    return { shown, after };
  });
}

function listenerClearsUnsavedProblems(seen) {
  const problems = [];
  if (!seen.shown) problems.push("the tab's own refused profile write did not show mouse-unsaved");
  if (seen.after.unsaved) problems.push("the listener's refresh left the stale mouse-unsaved showing");
  if (seen.after.profile !== "window-left") problems.push(`the listener did not replace the profile: ${seen.after.profile}`);
  return problems;
}

// N1, review pass 2. A field whose own write failed this session keeps the
// value in effect and its sentence when another tab writes a different
// field, and the stored value for the field itself moving is the repair.
// The document is planted so the other tab's load writes nothing and its
// different-field write carries the document's rate, never this tab's lost
// one.
async function observeListenerKeepsFailedField(url, browser) {
  return withPage(browser, async (page, context) => {
    await plant(page, { "ocelli.prefs": documentJSON(PLAIN_DOC) });
    await page.addInitScript(() => {
      const original = Storage.prototype.setItem;
      let refused = true;
      Storage.prototype.setItem = function (key, value) {
        if (refused) {
          refused = false;
          throw new Error("this browser refused the write");
        }
        return original.call(this, key, value);
      };
    });
    const other = await context.newPage();
    await page.goto(url);
    await other.goto(url);
    await setRate(page, 20);
    const shown = await noteLines(page);
    const rate = await page.evaluate(() => document.getElementById("fps").value);
    await other.locator("#mouse-profile").selectOption("window-left");
    await page.waitForTimeout(300);
    const afterOtherField = {
      note: await noteLines(page),
      rate: await page.evaluate(() => document.getElementById("fps").value),
      profile: await page.evaluate(() => document.getElementById("mouse-profile").value),
    };
    await setRate(other, 15);
    await page.waitForTimeout(300);
    const afterSameField = {
      note: await noteLines(page),
      rate: await page.evaluate(() => document.getElementById("fps").value),
    };
    await context.close();
    return { shown, rate, afterOtherField, afterSameField };
  });
}

function listenerKeepsFailedFieldProblems(seen) {
  const problems = [];
  if (!seen.shown.lines.includes(LOST_RATE)) {
    problems.push(`the failed rate write shows ${JSON.stringify(seen.shown)}, not the play rate sentence`);
  }
  if (seen.rate !== "20") problems.push(`the failed rate write did not apply 20: ${seen.rate}`);
  if (seen.afterOtherField.rate !== "20") {
    problems.push(`another tab's profile write moved the rate to ${seen.afterOtherField.rate}`);
  }
  if (!seen.afterOtherField.note.lines.includes(LOST_RATE)) {
    problems.push(`another tab's profile write cleared the rate sentence: ${JSON.stringify(seen.afterOtherField.note)}`);
  }
  if (seen.afterOtherField.profile !== "window-left") {
    problems.push(`the listener did not refresh the other field: ${seen.afterOtherField.profile}`);
  }
  if (seen.afterSameField.rate !== "15") {
    problems.push(`the other tab's kept rate write did not refresh the rate: ${seen.afterSameField.rate}`);
  }
  if (seen.afterSameField.note.lines.includes(LOST_RATE)) {
    problems.push(`the repaired rate kept its sentence: ${JSON.stringify(seen.afterSameField.note)}`);
  }
  return problems;
}

// D1, review pass 3. A kept write of the field before the failed one is the
// baseline the listener measures against, so another field's stored change
// leaves the failed field and its sentence alone, and a fresh stored change
// of the field itself replaces both. The refusal is armed after the kept
// write, so the failed write is the document's alone, and the first write's
// document is what the other tab reads and rewrites.
const REFUSE_PREFS_DOCUMENT = `
  window.__refusePrefsDocument = false;
  const original = Storage.prototype.setItem;
  Storage.prototype.setItem = function (key, value) {
    if (key === "ocelli.prefs" && window.__refusePrefsDocument) {
      throw new Error("this browser refused the write");
    }
    return original.call(this, key, value);
  };
`;

const REFUSE_LEGACY_FPS = `
  window.__refuseLegacyFps = false;
  const original = Storage.prototype.setItem;
  Storage.prototype.setItem = function (key, value) {
    if (key === "ocelli.fps" && window.__refuseLegacyFps) {
      throw new Error("this browser refused the write");
    }
    return original.call(this, key, value);
  };
`;

async function observeListenerAfterOwnKeptWrite(url, browser) {
  const refuseDocument = REFUSE_PREFS_DOCUMENT;
  // (a) The rate, against another field. The stored rate is this tab's own
  // kept 20, not the planted 10 the page loaded with, so the failed 25 and
  // its sentence stand.
  const rateAfterOtherField = await withPage(browser, async (page, context) => {
    await plant(page, { "ocelli.prefs": documentJSON(PLAIN_DOC) });
    await page.addInitScript(refuseDocument);
    const other = await context.newPage();
    await page.goto(url);
    await other.goto(url);
    await setRate(page, 20);
    await page.evaluate(() => { window.__refusePrefsDocument = true; });
    await setRate(page, 25);
    const before = { note: await noteLines(page), rate: await page.evaluate(() => document.getElementById("fps").value) };
    await other.locator("#mouse-profile").selectOption("window-left");
    await page.waitForTimeout(300);
    const after = { note: await noteLines(page), rate: await page.evaluate(() => document.getElementById("fps").value) };
    await context.close();
    return { before, after };
  });
  // (b) The rate, against itself. The other tab's kept 10 is a fresh stored
  // change, so the failed 25 gives way and the sentence goes with it.
  const rateAfterSameField = await withPage(browser, async (page, context) => {
    await plant(page, { "ocelli.prefs": documentJSON(PLAIN_DOC) });
    await page.addInitScript(refuseDocument);
    const other = await context.newPage();
    await page.goto(url);
    await other.goto(url);
    await setRate(page, 20);
    await page.evaluate(() => { window.__refusePrefsDocument = true; });
    await setRate(page, 25);
    await setRate(other, 10);
    await page.waitForTimeout(300);
    const after = { note: await noteLines(page), rate: await page.evaluate(() => document.getElementById("fps").value) };
    await context.close();
    return { after };
  });
  // The refused document. What a load reads then is the legacy keys, so the
  // same two paths hold against them: the kept own write of ocelli.fps
  // advances the baseline, the failed one does not, and the other tab's
  // legacy writes decide.
  const refuseLegacyFps = REFUSE_LEGACY_FPS;
  const refusedRateAfterOtherField = await withPage(browser, async (page, context) => {
    await plant(page, { "ocelli.prefs": "not json", "ocelli.fps": "10" });
    await page.addInitScript(refuseLegacyFps);
    const other = await context.newPage();
    await page.goto(url);
    await other.goto(url);
    await setRate(page, 20);
    await page.evaluate(() => { window.__refuseLegacyFps = true; });
    await setRate(page, 25);
    const before = { note: await noteLines(page), rate: await page.evaluate(() => document.getElementById("fps").value) };
    await other.locator("#mouse-profile").selectOption("window-left");
    await page.waitForTimeout(300);
    const after = { note: await noteLines(page), rate: await page.evaluate(() => document.getElementById("fps").value) };
    await context.close();
    return { before, after };
  });
  const refusedRateAfterSameField = await withPage(browser, async (page, context) => {
    await plant(page, { "ocelli.prefs": "not json", "ocelli.fps": "10" });
    await page.addInitScript(refuseLegacyFps);
    const other = await context.newPage();
    await page.goto(url);
    await other.goto(url);
    await setRate(page, 20);
    await page.evaluate(() => { window.__refuseLegacyFps = true; });
    await setRate(page, 25);
    await setRate(other, 10);
    await page.waitForTimeout(300);
    const after = { note: await noteLines(page), rate: await page.evaluate(() => document.getElementById("fps").value) };
    await context.close();
    return { after };
  });
  // (a) The profile, against another field. The stored profile is this tab's
  // own kept scroll-left, so the failed window-left and its paragraph stand.
  const profileAfterOtherField = await withPage(browser, async (page, context) => {
    await plant(page, { "ocelli.prefs": documentJSON(PLAIN_DOC) });
    await page.addInitScript(refuseDocument);
    const other = await context.newPage();
    await page.goto(url);
    await other.goto(url);
    await page.locator("#mouse-profile").selectOption("scroll-left");
    await page.evaluate(() => { window.__refusePrefsDocument = true; });
    await page.locator("#mouse-profile").selectOption("window-left");
    const before = {
      unsaved: await page.evaluate(() => !document.getElementById("mouse-unsaved").hidden),
      profile: await page.evaluate(() => document.getElementById("mouse-profile").value),
    };
    await setRate(other, 15);
    await page.waitForTimeout(300);
    const after = {
      unsaved: await page.evaluate(() => !document.getElementById("mouse-unsaved").hidden),
      profile: await page.evaluate(() => document.getElementById("mouse-profile").value),
    };
    await context.close();
    return { before, after };
  });
  // (b) The profile, against itself. The other tab's kept Ocelli is a fresh
  // stored change, so the failed window-left gives way and the paragraph
  // goes with it.
  const profileAfterSameField = await withPage(browser, async (page, context) => {
    await plant(page, { "ocelli.prefs": documentJSON(PLAIN_DOC) });
    await page.addInitScript(refuseDocument);
    const other = await context.newPage();
    await page.goto(url);
    await other.goto(url);
    await page.locator("#mouse-profile").selectOption("scroll-left");
    await page.evaluate(() => { window.__refusePrefsDocument = true; });
    await page.locator("#mouse-profile").selectOption("window-left");
    await other.locator("#mouse-profile").selectOption("ocelli");
    await page.waitForTimeout(300);
    const after = {
      unsaved: await page.evaluate(() => !document.getElementById("mouse-unsaved").hidden),
      profile: await page.evaluate(() => document.getElementById("mouse-profile").value),
    };
    await context.close();
    return { after };
  });
  return { rateAfterOtherField, rateAfterSameField, refusedRateAfterOtherField, refusedRateAfterSameField, profileAfterOtherField, profileAfterSameField };
}

function listenerAfterOwnKeptWriteProblems(seen) {
  const problems = [];
  const { rateAfterOtherField, rateAfterSameField, refusedRateAfterOtherField, refusedRateAfterSameField, profileAfterOtherField, profileAfterSameField } = seen;
  if (rateAfterOtherField.before.rate !== "25" || !rateAfterOtherField.before.note.lines.includes(LOST_RATE)) {
    problems.push(`the failed rate write did not apply 25 with its sentence: ${JSON.stringify(rateAfterOtherField.before)}`);
  }
  if (rateAfterOtherField.after.rate !== "25") {
    problems.push(`another field's kept write moved the failed rate to ${rateAfterOtherField.after.rate}`);
  }
  if (!rateAfterOtherField.after.note.lines.includes(LOST_RATE)) {
    problems.push(`another field's kept write cleared the rate sentence: ${JSON.stringify(rateAfterOtherField.after.note)}`);
  }
  if (rateAfterSameField.after.rate !== "10") {
    problems.push(`the kept rate write did not apply the fresh stored rate: ${rateAfterSameField.after.rate}`);
  }
  if (rateAfterSameField.after.note.lines.includes(LOST_RATE)) {
    problems.push(`the repaired rate kept its sentence: ${JSON.stringify(rateAfterSameField.after.note)}`);
  }
  if (refusedRateAfterOtherField.before.rate !== "25" || !refusedRateAfterOtherField.before.note.lines.includes(LOST_RATE)) {
    problems.push(`the failed refused-state rate write did not apply 25 with its sentence: ${JSON.stringify(refusedRateAfterOtherField.before)}`);
  }
  if (refusedRateAfterOtherField.after.rate !== "25") {
    problems.push(`another field's kept legacy write moved the failed rate to ${refusedRateAfterOtherField.after.rate}`);
  }
  if (!refusedRateAfterOtherField.after.note.lines.includes(LOST_RATE)) {
    problems.push(`another field's kept legacy write cleared the rate sentence: ${JSON.stringify(refusedRateAfterOtherField.after.note)}`);
  }
  if (refusedRateAfterSameField.after.rate !== "10") {
    problems.push(`the kept legacy rate write did not apply the fresh stored rate: ${refusedRateAfterSameField.after.rate}`);
  }
  if (refusedRateAfterSameField.after.note.lines.includes(LOST_RATE)) {
    problems.push(`the repaired refused-state rate kept its sentence: ${JSON.stringify(refusedRateAfterSameField.after.note)}`);
  }
  if (!profileAfterOtherField.before.unsaved || profileAfterOtherField.before.profile !== "window-left") {
    problems.push(`the failed profile write did not apply window-left with its paragraph: ${JSON.stringify(profileAfterOtherField.before)}`);
  }
  if (profileAfterOtherField.after.profile !== "window-left") {
    problems.push(`another field's kept write moved the failed profile to ${profileAfterOtherField.after.profile}`);
  }
  if (!profileAfterOtherField.after.unsaved) {
    problems.push("another field's kept write cleared the mouse-unsaved paragraph");
  }
  if (profileAfterSameField.after.profile !== "ocelli") {
    problems.push(`the kept profile write did not apply the fresh stored profile: ${profileAfterSameField.after.profile}`);
  }
  if (profileAfterSameField.after.unsaved) {
    problems.push("the repaired profile kept the mouse-unsaved paragraph");
  }
  return problems;
}

// AC10. The stored default applies once per series load, Reset returns the
// file window, scrolling keeps it, play sends no preset header after Reset
// and only preset headers while the default is applied. The hold cases
// cover the first paint, the cache hit, tile mode and a superseding load.
async function observeStoredDefault(url, browser, folders) {
  const seen = {};
  // The applied default, Reset, scroll and play. Four slices, so a step can
  // reach a frame the prefetch has not covered and play's own fetch is
  // observable. The prefetch races the play, so the play phases assert
  // liveness by the counter's steps and the header by its absence or
  // presence, never by a request count.
  seen.main = await withPage(browser, async (page) => {
    await plant(page, { "ocelli.prefs": documentJSON({ ...PLAIN_DOC, defaultPreset: "lung" }) });
    await page.addInitScript(STEP_PROBE);
    const requests = [];
    page.on("request", (request) => {
      if (new URL(request.url()).pathname === "/preview") requests.push({
        preset: Boolean(request.headers()["x-ocelli-window-preset"]),
        pair: `${request.headers()["x-ocelli-file"] || ""} ${request.headers()["x-ocelli-frame"] || ""}`,
      });
    });
    await page.goto(url);
    await openFolder(page, folders.four, "Lung");
    const applied = await sample(page);
    await page.click("#reset");
    await settle(page, "Window 1");
    const reset = await sample(page);
    // Play first, so the scroll's own prefetch cannot cover the play.
    requests.length = 0;
    await page.evaluate(() => { if (document.activeElement) document.activeElement.blur(); });
    const before = await page.evaluate(() => window.__steps.length);
    await page.keyboard.press("Space");
    await page.waitForTimeout(1200);
    await page.keyboard.press("Space");
    const afterResetRequests = requests.splice(0);
    const stepsAfterReset = (await page.evaluate(() => window.__steps.length)) - before;
    await page.keyboard.press("ArrowDown");
    await settle(page, "Window 1");
    const scrolled = await sample(page);
    // The default still applied: a fresh page of the same context replants.
    await page.reload();
    await openFolder(page, folders.four, "Lung");
    requests.length = 0;
    const beforeApplied = await page.evaluate(() => window.__steps.length);
    await page.keyboard.press("Space");
    await page.waitForTimeout(1200);
    await page.keyboard.press("Space");
    const appliedRequests = requests.splice(0);
    const stepsApplied = (await page.evaluate(() => window.__steps.length)) - beforeApplied;
    return { applied, reset, scrolled, afterResetRequests, appliedRequests, stepsAfterReset, stepsApplied };
  });
  // The first-paint hold: the preset reply is held, nothing paints, and the
  // first painted label names the preset.
  seen.hold = await withPage(browser, async (page) => {
    await plant(page, { "ocelli.prefs": documentJSON({ ...PLAIN_DOC, defaultPreset: "lung" }) });
    await page.addInitScript(PAINT_PROBE);
    let release;
    const gate = new Promise((resolve) => { release = resolve; });
    let armed = true;
    await page.route("**/preview", (route) => {
      if (armed && route.request().headers()["x-ocelli-window-preset"]) {
        armed = false;
        return gate.then(() => route.continue());
      }
      return route.continue();
    });
    await page.goto(url);
    await page.locator("#folder").setInputFiles(folders.two);
    await page.waitForFunction(() => (document.getElementById("status").textContent || "").includes("series."), null, { timeout: 15000 });
    await page.waitForTimeout(800);
    const held = await page.evaluate(() => ({ paints: window.__paints, labels: window.__paintLabels }));
    release();
    await settle(page, "Lung");
    const released = await page.evaluate(() => ({ paints: window.__paints, labels: window.__paintLabels }));
    const corner = await sample(page);
    return { held, released, corner };
  });
  // The cache-hit hold: the file window's grey is already cached, and the
  // held trial still keeps the first paint back.
  seen.cacheHit = await withPage(browser, async (page) => {
    await plant(page, { "ocelli.prefs": documentJSON({ ...PLAIN_DOC }) });
    await page.addInitScript(PAINT_PROBE);
    await page.goto(url);
    await openFolder(page, folders.two, "Window 1");
    await page.click("#reset");
    await settle(page, "Window 1");
    await page.keyboard.press("ArrowDown");
    await settle(page, "Window 1");
    await page.locator("#default-preset").selectOption("lung");
    let release;
    const gate = new Promise((resolve) => { release = resolve; });
    let armed = true;
    await page.route("**/preview", (route) => {
      if (armed && route.request().headers()["x-ocelli-window-preset"]) {
        armed = false;
        return gate.then(() => route.continue());
      }
      return route.continue();
    });
    const before = await page.evaluate(() => window.__paints);
    await page.locator("#series-list button", { hasText: "#1" }).first().click();
    await page.waitForTimeout(700);
    const held = await page.evaluate((b) => ({ before: b, paints: window.__paints, labels: window.__paintLabels.slice(b) }), before);
    release();
    await settle(page, "Lung");
    const released = await page.evaluate((b) => ({ paints: window.__paints, labels: window.__paintLabels.slice(b) }), before);
    return { held, released };
  });
  // The tile-mode hold.
  seen.tiles = await withPage(browser, async (page) => {
    await plant(page, { "ocelli.prefs": documentJSON({ ...PLAIN_DOC }) });
    await page.addInitScript(PAINT_PROBE);
    await page.goto(url);
    await openFolder(page, folders.four, "Window 1");
    await page.keyboard.press("Shift+G");
    await page.waitForTimeout(300);
    await page.locator("#default-preset").selectOption("lung");
    // The tile pass of the first open has settled before the baseline is
    // read, so a paint counted after the click can only be the load's.
    await page.waitForFunction(() => {
      const labels = Array.from(document.querySelectorAll(".cell.active .tile-label"));
      return labels.length === 4 && labels.every((node) => (node.textContent || "").includes("Window 1"));
    }, null, { timeout: 8000 }).catch(() => {});
    await page.waitForTimeout(300);
    let release;
    const gate = new Promise((resolve) => { release = resolve; });
    let armed = true;
    await page.route("**/preview", (route) => {
      if (armed && route.request().headers()["x-ocelli-window-preset"]) {
        armed = false;
        return gate.then(() => route.continue());
      }
      return route.continue();
    });
    const before = await page.evaluate(() => window.__tilePaints);
    await page.locator("#series-list button", { hasText: "#1" }).first().click();
    await page.waitForTimeout(700);
    const held = await page.evaluate((b) => ({ before: b, tiles: window.__tilePaints, labels: Array.from(document.querySelectorAll(".cell.active .tile-label")).map((node) => node.textContent) }), before);
    release();
    await page.waitForFunction(() => Array.from(document.querySelectorAll(".cell.active .tile-label"))
      .every((node) => (node.textContent || "").includes("Lung")), null, { timeout: 8000 }).catch(() => {});
    const released = await page.evaluate(() => ({ tiles: window.__tilePaints, labels: Array.from(document.querySelectorAll(".cell.active .tile-label")).map((node) => node.textContent) }));
    return { held, released };
  });
  // S1, review pass 2. The refreshFrameWindows tile gate. The four-frame
  // per-frame-window multiframe asks /inspect for frames 1 to 3, held from
  // before the open. The planted Bone applies at the first open, Lung is
  // chosen, and the reloaded series' Lung trial is held: the frame lists
  // are released during that hold and paint no tile, and the release
  // paints Lung first.
  seen.inspectTiles = await withPage(browser, async (page) => {
    await plant(page, { "ocelli.prefs": documentJSON({ ...PLAIN_DOC, defaultPreset: "bone" }) });
    await page.addInitScript(PAINT_PROBE);
    let releasePreview;
    const releaseInspects = [];
    await page.route("**/inspect", (route) => {
      const frame = route.request().headers()["x-ocelli-frame"];
      if (frame === "1" || frame === "2" || frame === "3") {
        return new Promise((resolve) => releaseInspects.push(resolve)).then(() => route.continue());
      }
      return route.continue();
    });
    let armPreview = false;
    await page.route("**/preview", (route) => {
      if (armPreview && route.request().headers()["x-ocelli-window-preset"]) {
        armPreview = false;
        return new Promise((resolve) => { releasePreview = resolve; }).then(() => route.continue());
      }
      return route.continue();
    });
    await page.goto(url);
    await openFolder(page, folders.frames, "Bone");
    await page.keyboard.press("Shift+G");
    await page.waitForFunction(() => Array.from(document.querySelectorAll(".cell.active .tile-label"))
      .every((node) => (node.textContent || "").includes("Bone")), null, { timeout: 8000 }).catch(() => {});
    await page.locator("#default-preset").selectOption("lung");
    armPreview = true;
    await page.locator("#series-list button").first().click();
    await page.waitForTimeout(700);
    const before = await page.evaluate(() => window.__tilePaints);
    await Promise.all(releaseInspects.splice(0).map((resolve) => resolve()));
    await page.waitForTimeout(500);
    const held = await page.evaluate(() => ({
      tiles: window.__tilePaints,
      labels: Array.from(document.querySelectorAll(".cell.active .tile-label")).map((node) => node.textContent),
    }));
    releasePreview();
    await page.waitForFunction(() => Array.from(document.querySelectorAll(".cell.active .tile-label"))
      .every((node) => (node.textContent || "").includes("Lung")), null, { timeout: 8000 }).catch(() => {});
    const released = await page.evaluate(() => ({
      tiles: window.__tilePaints,
      labels: Array.from(document.querySelectorAll(".cell.active .tile-label")).map((node) => node.textContent),
    }));
    return { before, held, released };
  });
  // The superseding load: the second series' outcome paints during the held
  // trial, and the released trial paints nothing.
  seen.superseded = await withPage(browser, async (page) => {
    await plant(page, { "ocelli.prefs": documentJSON({ ...PLAIN_DOC, defaultPreset: "lung" }) });
    await page.addInitScript(PAINT_PROBE);
    let release;
    const gate = new Promise((resolve) => { release = resolve; });
    let armed = true;
    await page.route("**/preview", (route) => {
      if (armed && route.request().headers()["x-ocelli-window-preset"]) {
        armed = false;
        return gate.then(() => route.continue());
      }
      return route.continue();
    });
    await page.goto(url);
    await page.locator("#folder").setInputFiles(folders.ctMr);
    await page.waitForFunction(() => (document.getElementById("status").textContent || "").includes("series."), null, { timeout: 15000 });
    await page.waitForTimeout(600);
    const duringHold = await page.evaluate(() => window.__paints);
    await page.locator("#series-list button", { hasText: "#2" }).first().click();
    await settle(page, "Window 1");
    const mrShown = await sample(page);
    const afterSwitch = await page.evaluate(() => window.__paints);
    release();
    await page.waitForTimeout(600);
    const released = await sample(page);
    const paints = await page.evaluate(() => window.__paints);
    return { duringHold, mrShown, afterSwitch, released, paints };
  });
  // Two CT series with both preset replies held. The first series' load is
  // superseded, so releasing its reply first repaints nothing, and only the
  // second release paints, at the second series' preset. A release that
  // opens an older load's hold paints here.
  seen.ctSupersede = await withPage(browser, async (page) => {
    await plant(page, { "ocelli.prefs": documentJSON({ ...PLAIN_DOC, defaultPreset: "lung" }) });
    await page.addInitScript(PAINT_PROBE);
    const releases = [];
    await page.route("**/preview", (route) => {
      if (route.request().headers()["x-ocelli-window-preset"]) {
        return new Promise((resolve) => releases.push(resolve)).then(() => route.continue());
      }
      return route.continue();
    });
    await page.goto(url);
    await page.locator("#folder").setInputFiles(folders.trio);
    await page.waitForFunction(() => (document.getElementById("status").textContent || "").includes("series."), null, { timeout: 15000 });
    await page.waitForTimeout(800);
    const before = await page.evaluate(() => window.__paints);
    await page.locator("#series-list button", { hasText: "#2" }).first().click();
    await page.waitForTimeout(500);
    await releases[0]();
    await page.waitForTimeout(400);
    const afterFirst = await page.evaluate((b) => ({ paints: window.__paints, labels: window.__paintLabels.slice(b) }), before);
    await releases[1]();
    await settle(page, "Lung");
    const afterSecond = await page.evaluate((b) => ({ paints: window.__paints, labels: window.__paintLabels.slice(b) }), before);
    return { afterFirst, afterSecond };
  });
  // A preset reply refused with a 500 ends the trial, and the hold releases
  // all the same, so the file window paints. A release that only follows a
  // commit paints nothing here.
  seen.failedTrial = await withPage(browser, async (page) => {
    await plant(page, { "ocelli.prefs": documentJSON({ ...PLAIN_DOC, defaultPreset: "lung" }) });
    await page.addInitScript(PAINT_PROBE);
    let release;
    const gate = new Promise((resolve) => { release = resolve; });
    let armed = true;
    await page.route("**/preview", (route) => {
      if (armed && route.request().headers()["x-ocelli-window-preset"]) {
        armed = false;
        return gate.then(() => route.fulfill({ status: 500, body: "the held trial was refused" }));
      }
      return route.continue();
    });
    await page.goto(url);
    await page.locator("#folder").setInputFiles(folders.two);
    await page.waitForFunction(() => (document.getElementById("status").textContent || "").includes("series."), null, { timeout: 15000 });
    await page.waitForTimeout(800);
    const held = await page.evaluate(() => ({ paints: window.__paints, labels: window.__paintLabels }));
    release();
    await settle(page, "Window 1");
    const released = await page.evaluate(() => ({ paints: window.__paints, labels: window.__paintLabels }));
    const corner = await sample(page);
    return { held, released, corner };
  });
  // A viewport scrolled to a non-zero index of another series still gets
  // the preset, which a pre-move apply refuses.
  seen.preMove = await withPage(browser, async (page) => {
    await plant(page, { "ocelli.prefs": documentJSON({ ...PLAIN_DOC, defaultPreset: "lung" }) });
    await page.goto(url);
    await openFolder(page, folders.trio, "Window 1");
    await page.keyboard.press("ArrowDown");
    await page.keyboard.press("ArrowDown");
    await settle(page, "Window 1");
    await page.locator("#series-list button", { hasText: "#2" }).first().click();
    await settle(page, "Lung");
    return sample(page);
  });
  return seen;
}

function storedDefaultProblems(seen) {
  const problems = [];
  const { main, hold, cacheHit, tiles, inspectTiles, superseded, ctSupersede, failedTrial, preMove } = seen;
  if (main.applied.corner !== "WW 1500  WL -600  Lung" || main.applied.soft !== 236 || main.applied.lung !== 94) {
    problems.push(`the applied default shows ${JSON.stringify(main.applied)}, not Lung with greys 236 and 94`);
  }
  if (main.reset.corner !== "WW 400  WL 40  Window 1" || main.reset.soft !== 128 || main.reset.lung !== 0) {
    problems.push(`after Reset the corner is ${JSON.stringify(main.reset)}, not the file window with greys 128 and 0`);
  }
  if (main.scrolled.corner !== "WW 400  WL 40  Window 1" || main.scrolled.soft !== 128 || main.scrolled.lung !== 0) {
    problems.push(`after a scroll the corner is ${JSON.stringify(main.scrolled)}, not the file window still`);
  }
  const resetHeaders = main.afterResetRequests.filter((request) => request.preset);
  if (resetHeaders.length) {
    problems.push(`play after Reset sent ${resetHeaders.length} preset-headered requests`);
  }
  if (main.stepsAfterReset < 3) problems.push(`play after Reset took ${main.stepsAfterReset} steps, so it did not run`);
  const appliedHeaders = main.appliedRequests.filter((request) => request.preset);
  if (!appliedHeaders.length) problems.push("play with the default applied sent no preset-headered request");
  if (main.stepsApplied < 3) problems.push(`the applied play took ${main.stepsApplied} steps, so it did not run`);
  if (appliedHeaders.length !== main.appliedRequests.length) {
    problems.push(`play with the default applied sent ${main.appliedRequests.length - appliedHeaders.length} requests without the preset header`);
  }
  const pairs = new Set(appliedHeaders.map((request) => request.pair));
  if (pairs.size !== appliedHeaders.length) {
    problems.push(`play fetched a frame twice: ${appliedHeaders.length} requests for ${pairs.size} frames`);
  }
  if (hold.held.paints !== 0) {
    problems.push(`the held trial painted ${hold.held.paints} greys, and the hold labels were ${JSON.stringify(hold.held.labels)}`);
  }
  if (hold.held.labels.some((label) => label.includes("Lung"))) {
    problems.push(`the corner named the preset during the hold: ${JSON.stringify(hold.held.labels)}`);
  }
  if (!hold.released.paints) problems.push("the released trial painted nothing");
  if (!hold.released.labels.length || !hold.released.labels[0].endsWith("  Lung")) {
    problems.push(`the first painted label is ${JSON.stringify(hold.released.labels[0])}, not Lung`);
  }
  if (hold.corner.corner !== "WW 1500  WL -600  Lung") {
    problems.push(`after the hold the corner is ${JSON.stringify(hold.corner.corner)}`);
  }
  if (cacheHit.held.paints !== cacheHit.held.before) {
    problems.push(`the cache-hit hold painted ${cacheHit.held.paints - cacheHit.held.before} greys, labels ${JSON.stringify(cacheHit.held.labels)}`);
  }
  if (!cacheHit.released.paints) problems.push("the released cache-hit trial painted nothing");
  if (cacheHit.released.labels.length && !cacheHit.released.labels[cacheHit.released.labels.length - 1].endsWith("  Lung")) {
    problems.push(`the released cache-hit trial last painted ${JSON.stringify(cacheHit.released.labels[cacheHit.released.labels.length - 1])}`);
  }
  if (tiles.held.tiles !== tiles.held.before) {
    problems.push(`the tile-mode hold painted ${tiles.held.tiles - tiles.held.before} tiles, labels ${JSON.stringify(tiles.held.labels)}`);
  }
  if (tiles.held.labels.some((label) => (label || "").includes("Lung"))) {
    problems.push(`the tiles named the preset during the hold: ${JSON.stringify(tiles.held.labels)}`);
  }
  if (!tiles.released.labels.every((label) => label.includes("Lung"))) {
    problems.push(`the released tiles read ${JSON.stringify(tiles.released.labels)}`);
  }
  // The frame lists released during the held trial paint nothing: the
  // refreshFrameWindows gate keeps the tiles at their pre-hold count and at
  // the setup's Bone labels, and the release paints Lung first.
  if (inspectTiles.held.tiles !== inspectTiles.before) {
    problems.push(`the released frame lists painted ${inspectTiles.held.tiles - inspectTiles.before} tiles, labels ${JSON.stringify(inspectTiles.held.labels)}`);
  }
  if (inspectTiles.held.labels.some((label) => !(label || "").includes("Bone"))) {
    problems.push(`the released frame lists repainted a tile during the hold: ${JSON.stringify(inspectTiles.held.labels)}`);
  }
  if (inspectTiles.released.tiles <= inspectTiles.before) {
    problems.push("the released trial painted no tile");
  }
  if (!inspectTiles.released.labels.every((label) => label.includes("Lung"))) {
    problems.push(`the released tiles read ${JSON.stringify(inspectTiles.released.labels)}`);
  }
  if (superseded.duringHold !== 0) {
    problems.push(`the superseding case painted ${superseded.duringHold} greys during the held CT trial`);
  }
  if (!superseded.mrShown.corner.includes("Window 1") || superseded.mrShown.corner.includes("Lung")) {
    problems.push(`the second series showed ${JSON.stringify(superseded.mrShown.corner)} during the held trial`);
  }
  if (superseded.paints !== superseded.afterSwitch) {
    problems.push(`the released trial painted ${superseded.paints - superseded.afterSwitch} greys over the second series' outcome`);
  }
  if (ctSupersede.afterFirst.paints !== 0) {
    problems.push(`releasing the superseded series painted ${ctSupersede.afterFirst.paints} greys, labels ${JSON.stringify(ctSupersede.afterFirst.labels)}`);
  }
  if (!ctSupersede.afterSecond.paints) {
    problems.push("the second series' release painted nothing");
  }
  if (!ctSupersede.afterSecond.labels.length || !ctSupersede.afterSecond.labels[0].endsWith("  Lung")) {
    problems.push(`the second series' first painted label is ${JSON.stringify(ctSupersede.afterSecond.labels[0])}, not Lung`);
  }
  if (failedTrial.held.paints !== 0) {
    problems.push(`the refused trial painted ${failedTrial.held.paints} greys before its release`);
  }
  if (!failedTrial.released.paints) {
    problems.push("the refused trial released nothing");
  }
  if (!failedTrial.released.labels.length || !failedTrial.released.labels[0].endsWith("  Window 1")) {
    problems.push(`the refused trial's first painted label is ${JSON.stringify(failedTrial.released.labels[0])}, not the file window`);
  }
  if (failedTrial.corner.corner !== "WW 400  WL 40  Window 1" || failedTrial.corner.soft !== 128 || failedTrial.corner.lung !== 0) {
    problems.push(`after the refused trial the corner is ${JSON.stringify(failedTrial.corner)}, not the file window`);
  }
  if (!preMove.corner.endsWith("  Lung") || preMove.soft !== 236 || preMove.lung !== 94) {
    problems.push(`the pre-moved viewport shows ${JSON.stringify(preMove)}, not Lung with greys 236 and 94`);
  }
  return problems;
}

// AC11. No preset on an MR, a colour image or a VOI LUT file, and no fault
// and no named sentence where the process offers nothing.
async function observeNonApplication(url, browser, folders) {
  const seen = {};
  seen.mr = await withPage(browser, async (page) => {
    await plant(page, { "ocelli.prefs": documentJSON({ ...PLAIN_DOC, defaultPreset: "bone" }) });
    await page.goto(url);
    await openFolder(page, folders.mr, "Window 1");
    return { ...(await sample(page)), note: await noteLines(page) };
  });
  seen.colour = await withPage(browser, async (page) => {
    await plant(page, { "ocelli.prefs": documentJSON({ ...PLAIN_DOC, defaultPreset: "bone" }) });
    await page.goto(url);
    await openFolder(page, folders.colour, "");
    await page.waitForFunction(() => {
      const canvas = document.querySelector(".cell.active canvas.view");
      return canvas && canvas.width > 1;
    }, null, { timeout: 8000 }).catch(() => {});
    return {
      note: await noteLines(page),
      windowNote: await page.evaluate(() => document.getElementById("window-note").textContent),
      presetRequests: 0,
    };
  });
  seen.voi = await withPage(browser, async (page) => {
    const requests = [];
    page.on("request", (request) => {
      if (new URL(request.url()).pathname === "/preview") requests.push(Boolean(request.headers()["x-ocelli-window-preset"]));
    });
    // The process refuses every windowed preview of a file with a VOI LUT
    // Sequence, decided in preview.rs and pinned by its own tests, so the
    // load's own fault is the control: the same fault with and without a
    // stored default proves the apply added nothing.
    await plant(page, { "ocelli.prefs": documentJSON({ ...PLAIN_DOC }) });
    await page.goto(url);
    await openFolder(page, folders.voi, "");
    await settleFault(page);
    const plain = await sample(page);
    const plainRows = await page.evaluate(() => Array.from(document.querySelectorAll("#window-list button")).map((node) => ({ label: node.querySelector(".window-label").textContent, disabled: node.disabled })));
    await plant(page, { "ocelli.prefs": documentJSON({ ...PLAIN_DOC, defaultPreset: "bone" }) });
    await page.goto(url);
    await openFolder(page, folders.voi, "");
    await settleFault(page);
    const rows = await page.evaluate(() => Array.from(document.querySelectorAll("#window-list button")).map((node) => ({ label: node.querySelector(".window-label").textContent, disabled: node.disabled })));
    return { plain, plainRows, rows, note: await noteLines(page), corner: await sample(page), presetRequests: requests.filter(Boolean).length };
  });
  return seen;
}

function nonApplicationProblems(seen) {
  const problems = [];
  const { mr, colour, voi } = seen;
  if (!mr.corner.endsWith("  Window 1") || mr.corner.includes("Bone")) {
    problems.push(`the MR shows ${JSON.stringify(mr.corner)}, not its own file window`);
  }
  if (mr.fault) problems.push(`the MR shows the fault ${mr.fault}`);
  if (!mr.note.hidden && mr.note.lines.includes(NOT_OFFERED)) {
    problems.push(`the MR named the not-offered sentence: ${JSON.stringify(mr.note)}`);
  }
  if (!colour.windowNote.startsWith("This image is colour")) {
    problems.push(`the colour image's window note reads ${JSON.stringify(colour.windowNote)}`);
  }
  if (!colour.note.hidden && colour.note.lines.includes(NOT_OFFERED)) {
    problems.push(`the colour image named the not-offered sentence: ${JSON.stringify(colour.note)}`);
  }
  if (!voi.rows.length || !voi.rows.every((row) => row.disabled)) {
    problems.push(`the VOI LUT file's rows are ${JSON.stringify(voi.rows)}, all disabled wanted`);
  }
  if (!voi.plain.fault || voi.plain.fault !== voi.corner.fault) {
    problems.push(`the VOI LUT load's own fault is ${JSON.stringify(voi.plain.fault)} without a default and ${JSON.stringify(voi.corner.fault)} with one, so the apply changed it`);
  }
  if (voi.corner.corner.includes("Bone")) problems.push(`the VOI LUT file shows ${JSON.stringify(voi.corner.corner)}`);
  if (!voi.note.hidden && voi.note.lines.includes(NOT_OFFERED)) {
    problems.push(`the VOI LUT file named the not-offered sentence: ${JSON.stringify(voi.note)}`);
  }
  if (voi.presetRequests) problems.push(`the VOI LUT load sent ${voi.presetRequests} preset-headered requests`);
  return problems;
}

// AC16. A stored default does not overwrite a linked viewport's reader-set
// window.
async function observeNoPropagation(url, browser, folders) {
  return withPage(browser, async (page) => {
    await plant(page, { "ocelli.prefs": documentJSON({ ...PLAIN_DOC, defaultPreset: "bone" }) });
    await page.goto(url);
    await openFolder(page, folders.trio, "Window 1");
    await page.locator("#layout").click();
    await page.locator('#layout-picker [data-columns="2"][data-rows="1"]').click();
    await page.waitForTimeout(300);
    await page.locator('.cell[data-cell="1"]').click();
    await page.keyboard.press("2");
    await settle(page, "Lung");
    const reader = await page.evaluate(() => document.querySelectorAll('.cell[data-cell="1"] .overlay.br')[0].textContent.split("\n")[0]);
    await page.locator("#link").click();
    await page.locator("#link-window").click();
    await page.keyboard.press("Escape");
    await page.locator('.cell[data-cell="0"]').click();
    await page.locator("#series-list button", { hasText: "#3" }).first().click();
    await settle(page, "Bone");
    const first = await page.evaluate(() => document.querySelectorAll('.cell[data-cell="0"] .overlay.br')[0].textContent.split("\n")[0]);
    const second = await page.evaluate(() => document.querySelectorAll('.cell[data-cell="1"] .overlay.br')[0].textContent.split("\n")[0]);
    return { reader, first, second };
  });
}

function noPropagationProblems(seen) {
  const problems = [];
  if (!seen.reader.endsWith("  Lung")) problems.push(`the reader-set window in cell 2 is ${JSON.stringify(seen.reader)}, not Lung`);
  if (!seen.first.endsWith("  Bone")) problems.push(`the loaded viewport shows ${JSON.stringify(seen.first)}, not Bone`);
  if (!seen.second.endsWith("  Lung")) problems.push(`the linked viewport's window moved to ${JSON.stringify(seen.second)}, so the apply propagated`);
  return problems;
}

// AC17. A stored token the process does not offer is skipped before any
// request, named once per viewport, and cleared by the next load into that
// viewport while a load into another cell leaves it.
async function observeNotOffered(url, browser, folders) {
  return withPage(browser, async (page) => {
    await plant(page, { "ocelli.prefs": documentJSON({ ...PLAIN_DOC, defaultPreset: "bone" }) });
    await page.addInitScript(PAINT_PROBE);
    const requests = [];
    page.on("request", (request) => {
      if (new URL(request.url()).pathname === "/preview") requests.push({ preset: Boolean(request.headers()["x-ocelli-window-preset"]), file: request.headers()["x-ocelli-file"] || "" });
    });
    await page.route("**/file", async (route) => {
      const response = await route.fetch();
      const info = await response.json();
      if (info.seriesUid === "2.25.21401" && Array.isArray(info.presets)) {
        info.presets = info.presets.map((preset) => (preset.token === "bone" ? { ...preset, token: "future_token" } : preset));
      }
      await route.fulfill({ response, json: info });
    });
    await page.goto(url);
    await openFolder(page, folders.trio, "Window 1");
    const named = await noteLines(page);
    const fault = await sample(page);
    const strippedRequests = requests.splice(0);
    // A load into another cell leaves the sentence.
    await page.locator("#layout").click();
    await page.locator('#layout-picker [data-columns="2"][data-rows="1"]').click();
    await page.waitForTimeout(300);
    await page.locator('.cell[data-cell="1"]').click();
    await page.locator("#series-list button", { hasText: "#3" }).first().click();
    await settle(page, "Window 1");
    // While the other cell is outlined, its own clean state is what shows.
    const otherActive = await noteLines(page);
    await page.locator('.cell[data-cell="0"]').click();
    const left = await noteLines(page);
    // The next load into the viewport that showed it clears the sentence.
    await page.locator("#series-list button", { hasText: "#2" }).first().click();
    await settle(page, "Bone");
    const cleared = await noteLines(page);
    const corner = await sample(page);
    return { named, fault, otherActive, left, cleared, corner, strippedRequests };
  });
}

function notOfferedProblems(seen) {
  const problems = [];
  const { named, fault, otherActive, left, cleared, corner, strippedRequests } = seen;
  if (named.hidden || !named.lines.includes(NOT_OFFERED)) {
    problems.push(`the stripped reply shows ${JSON.stringify(named)}, not the not-offered sentence`);
  }
  if (fault.fault) problems.push(`the stripped reply faulted the window: ${fault.fault}`);
  if (strippedRequests.some((request) => request.preset)) {
    problems.push(`the stripped reply sent ${strippedRequests.filter((request) => request.preset).length} preset-headered requests`);
  }
  if (!otherActive.hidden && otherActive.lines.includes(NOT_OFFERED)) {
    problems.push(`the note named the first cell's sentence while the second was outlined: ${JSON.stringify(otherActive)}`);
  }
  if (left.hidden || !left.lines.includes(NOT_OFFERED)) {
    problems.push(`a load into another cell lost the sentence: ${JSON.stringify(left)}`);
  }
  if (cleared.lines.includes(NOT_OFFERED)) {
    problems.push(`the next load into the same viewport kept the sentence: ${JSON.stringify(cleared)}`);
  }
  if (!corner.corner.endsWith("  Bone")) problems.push(`the cleared load shows ${JSON.stringify(corner.corner)}, not Bone`);
  return problems;
}

// AC9. The whole-origin invariant: the five allowed keys and nothing else,
// after every control, a folder open and a DICOMweb open, with a planted
// path in a known field written out as the value in effect. Before any
// library load the page's own writes hold no loglevel key, checked in a
// page whose library import is held off, because dcmjs writes its own level
// at import and the point here is what the page writes.
async function observeWholeOrigin(url, browser, folders, study) {
  const beforeLibrary = await withPage(browser, async (page) => {
    await page.route("**/lib/**", (route) => route.abort());
    await page.goto(url);
    await openFolder(page, folders.two, "Window 1");
    await setRate(page, 11);
    return page.evaluate(() => Object.keys(document.defaultView.localStorage).sort());
  });
  const swept = await withPage(browser, async (page) => {
    await page.goto(url);
    await page.locator("#mouse-profile").selectOption("window-left");
    await setRate(page, 15);
    await page.click("#toggle-series");
    await page.click("#toggle-series");
    await page.locator("#default-preset").selectOption("lung");
    await openFolder(page, folders.two, "Window 1");
    await page.locator("#source-dicomweb").click();
    await page.locator("#remote-list button", { hasText: study }).first().click();
    await page.waitForFunction(() => Boolean(document.querySelector("#remote-list button")), null, { timeout: 20000 }).catch(() => {});
    return page.evaluate(async () => ({
      keys: Object.keys(document.defaultView.localStorage).sort(),
      loglevel: document.defaultView.localStorage.getItem("loglevel"),
      session: document.defaultView.sessionStorage.length,
      databases: (await indexedDB.databases()).map((database) => database.name),
      caches: await caches.keys(),
      cookie: document.cookie,
    }));
  });
  // A path planted in a known field is written out as the value in effect,
  // in the document and in all four legacy keys.
  const planted = await withPage(browser, async (page) => {
    await plant(page, { "ocelli.prefs": documentJSON({ ...PLAIN_DOC, mouseProfile: "/data/patients" }) });
    await page.goto(url);
    await setRate(page, 13);
    return stored(page);
  });
  return { beforeLibrary, swept, planted };
}

function wholeOriginProblems(seen) {
  const problems = [];
  const { beforeLibrary, swept, planted } = seen;
  // Before any library load the page's own writes are the schema's keys
  // alone, so a page write under loglevel fails here.
  for (const key of beforeLibrary) {
    if (!ALLOWED_KEYS.includes(key)) {
      problems.push(`before any library load the origin holds ${JSON.stringify(beforeLibrary)}`);
      break;
    }
  }
  // The page's own writes are exactly the five allowed keys. The one
  // foreign key on this tree is `loglevel`, written by dcmjs's own module
  // top level (its bundled loglevel persists the level at import, seen in
  // node_modules/dcmjs/build/dcmjs.es.js), which the DICOMweb provider
  // loads. It is not the page's store, so the sweep allows exactly that one
  // name, and its value must be one of loglevel's own level names.
  const LOGLEVEL_LEVELS = ["TRACE", "DEBUG", "INFO", "WARN", "ERROR", "SILENT"];
  const extra = swept.keys.filter((key) => !ALLOWED_KEYS.includes(key));
  if (JSON.stringify(extra) !== JSON.stringify(["loglevel"])) {
    problems.push(`the origin holds ${JSON.stringify(swept.keys)}, not the five allowed keys plus only dcmjs's loglevel`);
  }
  if (!LOGLEVEL_LEVELS.includes(swept.loglevel)) {
    problems.push(`dcmjs's loglevel holds ${JSON.stringify(swept.loglevel)}, not one of ${JSON.stringify(LOGLEVEL_LEVELS)}`);
  }
  for (const key of ALLOWED_KEYS) {
    if (!swept.keys.includes(key)) problems.push(`the origin does not hold ${key} after the interaction pass`);
  }
  if (swept.session !== 0) problems.push(`sessionStorage holds ${swept.session} entries`);
  if (swept.databases.length) problems.push(`indexedDB holds ${JSON.stringify(swept.databases)}`);
  if (swept.caches.length) problems.push(`caches holds ${JSON.stringify(swept.caches)}`);
  if (swept.cookie !== "") problems.push(`the origin set a cookie: ${swept.cookie}`);
  const want = documentJSON({ mouseProfile: "ocelli", cineFps: 13, hideSeries: false, hideInspector: false, defaultPreset: "none" });
  if (planted.document !== want) problems.push(`the planted path wrote ${planted.document}, not ${want}`);
  if (planted.legacy.mouse !== "ocelli" || planted.legacy.fps !== "13" || planted.legacy.noSeries !== "0" || planted.legacy.noInspector !== "0") {
    problems.push(`the planted path left the legacy keys ${JSON.stringify(planted.legacy)}`);
  }
  return problems;
}

// AC7 and AC8. A real server restart and a real browser relaunch keep the
// choices on one origin, and a second port is a different origin.
async function observePersistence(chromium, binary, folders, upstream) {
  const port = await freePort();
  const second = await freePort();
  const origin = `http://127.0.0.1:${port}`;
  const server = await startViewer(binary, {
    OCELLI_PORT: String(port),
    OCELLI_VIEWER_STUDIES_SERVICE_URL: upstream.url,
    OCELLI_VIEWER_WADO_URI_URL: `${upstream.url}/wado`,
  });
  const userData = mkdtempSync(join(tmpdir(), "ocelli-f205-profile-"));
  const problems = [];
  try {
    const context = await chromium.launchPersistentContext(userData, { headless: true, viewport: { width: 1400, height: 900 } });
    const page = await context.newPage();
    await page.goto(origin);
    await page.locator("#mouse-profile").selectOption("window-left");
    await setRate(page, 7);
    await page.click("#toggle-inspector");
    await page.locator("#default-preset").selectOption("bone");
    const kept = await page.evaluate(() => document.defaultView.localStorage.getItem("ocelli.prefs"));
    await page.close();
    await stopByPid(server.pid);
    await context.close();
    const restarted = await startViewer(binary, {
      OCELLI_PORT: String(port),
      OCELLI_VIEWER_STUDIES_SERVICE_URL: upstream.url,
      OCELLI_VIEWER_WADO_URI_URL: `${upstream.url}/wado`,
    });
    const secondContext = await chromium.launchPersistentContext(userData, { headless: true, viewport: { width: 1400, height: 900 } });
    const again = await secondContext.newPage();
    await again.goto(origin);
    const restored = await again.evaluate(() => ({
      profile: document.getElementById("mouse-profile").value,
      rate: document.getElementById("fps").value,
      defaultPreset: document.getElementById("default-preset").value,
      noInspector: document.body.classList.contains("no-inspector"),
      tool: Array.from(document.querySelectorAll('button[id^="tool-"].active')).map((node) => node.id),
    }));
    // A different port is a different origin, and the defaults apply there.
    const secondServer = await startViewer(binary, {
      OCELLI_PORT: String(second),
      OCELLI_VIEWER_STUDIES_SERVICE_URL: upstream.url,
      OCELLI_VIEWER_WADO_URI_URL: `${upstream.url}/wado`,
    });
    const elsewhere = await secondContext.newPage();
    await elsewhere.goto(`http://127.0.0.1:${second}`);
    const fresh = await elsewhere.evaluate(() => ({
      profile: document.getElementById("mouse-profile").value,
      rate: document.getElementById("fps").value,
      defaultPreset: document.getElementById("default-preset").value,
      noInspector: document.body.classList.contains("no-inspector"),
      document: document.defaultView.localStorage.getItem("ocelli.prefs"),
    }));
    await elsewhere.close();
    await stopByPid(secondServer.pid);
    await secondContext.close();
    await stopByPid(restarted.pid);
    const want = documentJSON({ mouseProfile: "window-left", cineFps: 7, hideSeries: false, hideInspector: true, defaultPreset: "bone" });
    if (kept !== want) problems.push(`the choices were stored as ${kept}, not ${want}`);
    if (restored.profile !== "window-left") problems.push(`after the restart the profile is ${restored.profile}`);
    if (restored.rate !== "7") problems.push(`after the restart the rate is ${restored.rate}`);
    if (restored.defaultPreset !== "bone") problems.push(`after the restart the default preset is ${restored.defaultPreset}`);
    if (!restored.noInspector) problems.push("after the restart the tools column is shown again");
    if (!restored.tool.includes("tool-wl")) problems.push(`after the restart the tool is ${JSON.stringify(restored.tool)}`);
    if (fresh.profile !== "ocelli" || fresh.rate !== "10" || fresh.defaultPreset !== "none" || fresh.noInspector) {
      problems.push(`the second origin shows ${JSON.stringify(fresh)}, not the defaults`);
    }
    if (fresh.document !== null) problems.push(`the second origin holds a document: ${fresh.document}`);
    return problems;
  } catch (error) {
    problems.push(`the persistence case failed: ${error.message}`);
    return problems;
  } finally {
    rmSync(userData, { recursive: true, force: true });
  }
}

async function main() {
  const { chromium } = await loadPlaywright();
  const browser = await chromium.launch({ headless: true });
  const binary = await ensureViewerBinary("release", true);
  const root = mkdtempSync(join(tmpdir(), "ocelli-f205-"));
  const problems = [];
  const port = await freePort();
  const origin = `http://127.0.0.1:${port}`;
  const study = "2.25.245001";
  const upstream = await startSyntheticDicomweb({
    origin,
    study,
    series: "2.25.245002",
    instances: [{ uid: "2.25.245003", part10: slice({ sop: "2.25.245003", series: "2.25.245002", number: 1 }) }],
  });
  const folders = makeFolders(root);
  const server = await startViewer(binary, {
    OCELLI_PORT: String(port),
    OCELLI_VIEWER_STUDIES_SERVICE_URL: upstream.url,
    OCELLI_VIEWER_WADO_URI_URL: `${upstream.url}/wado`,
  });
  try {
    const exact = await observeExactDocument(origin, browser, folders);
    console.log(`AC1: ${exact.document}`);
    problems.push(...exactDocumentProblems(exact));

    const migration = await observeLegacyMigration(origin, browser);
    problems.push(...legacyMigrationProblems(migration));
    console.log("AC2: the legacy choices are migrated and rewritten");

    const both = await observeBothPresent(origin, browser);
    problems.push(...bothPresentProblems(both));
    console.log("AC3: the document wins and the legacy keys follow");

    const refused = await observeRefusedShapes(origin, browser);
    problems.push(...refusedShapeProblems(refused));
    console.log(`AC4: ${REFUSED_SHAPES.length} refused shapes left byte-identical`);

    const fields = await observeFieldRefusals(origin, browser);
    problems.push(...fieldRefusalProblems(fields));
    console.log(`AC5: ${fields.length - 1} refused fields named, the note visible under a hidden column`);

    const failures = await observeWriteFailures(origin, browser);
    problems.push(...writeFailureProblems(failures));
    console.log("AC6: every failed write names its own sentence and still applies");

    const partial = await observePartialWrites(origin, browser);
    problems.push(...partialWriteProblems(partial));
    console.log("AC18: one blocked key names one partial sentence");

    const inEffect = await observeUntouchedInEffect(origin, browser, folders);
    problems.push(...untouchedInEffectProblems(inEffect));
    console.log("S4: a failed preset write stays in effect past another field's write");

    const cross = await observeCrossTab(origin, browser, folders);
    problems.push(...crossTabProblems(cross));
    console.log("AC15: the two-page change survives, the listener refreshes without writing");

    const clearsUnsaved = await observeListenerClearsUnsaved(origin, browser);
    problems.push(...listenerClearsUnsavedProblems(clearsUnsaved));
    console.log("N1: the listener's refresh clears a stale mouse-unsaved");

    const keepsFailed = await observeListenerKeepsFailedField(origin, browser);
    problems.push(...listenerKeepsFailedFieldProblems(keepsFailed));
    console.log("N1: a failed field keeps its value past another field's write and repairs on its own");

    const afterOwnKept = await observeListenerAfterOwnKeptWrite(origin, browser);
    problems.push(...listenerAfterOwnKeptWriteProblems(afterOwnKept));
    console.log("D1: a kept write before the failed one is the listener's baseline, for the rate and the profile");

    const storedDefault = await observeStoredDefault(origin, browser, folders);
    problems.push(...storedDefaultProblems(storedDefault));
    console.log("AC10: the default applies once, the hold gates every paint site");

    const nonApplication = await observeNonApplication(origin, browser, folders);
    problems.push(...nonApplicationProblems(nonApplication));
    console.log("AC11: no preset on an MR, a colour image or a VOI LUT file");

    const propagation = await observeNoPropagation(origin, browser, folders);
    problems.push(...noPropagationProblems(propagation));
    console.log("AC16: the linked viewport keeps its reader-set window");

    const notOffered = await observeNotOffered(origin, browser, folders);
    problems.push(...notOfferedProblems(notOffered));
    console.log("AC17: a stripped token is named and skipped, then cleared");

    const whole = await observeWholeOrigin(origin, browser, folders, study);
    problems.push(...wholeOriginProblems(whole));
    console.log("AC9: the origin holds only the five allowed keys beside dcmjs's loglevel, and the page writes no loglevel of its own");

    problems.push(...(await observePersistence(chromium, binary, folders, upstream)));
    console.log("AC7 and AC8: the choices survive a restart and a relaunch, the second origin does not");
  } finally {
    await browser.close();
    await stopByPid(server.pid);
    await upstream.close();
    rmSync(root, { recursive: true, force: true });
  }
  if (problems.length) {
    for (const problem of problems) console.error(problem);
    process.exitCode = 1;
    return;
  }
  console.log("the viewer keeps five validated choices, names every refusal, and stores nothing else");
}

if (process.argv[1] === fileURLToPath(import.meta.url)) {
  main().catch((error) => {
    console.error(error);
    process.exitCode = 1;
  });
}
