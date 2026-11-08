// F-225 AC33, the browser half. The Headers badge and the Checks section on
// the plan's reproducer, and when the page asks for a report.
//
// The folder is one series of six synthetic 32 by 32 CT files, written by
// this script into a temporary folder and removed afterwards. Instance 1 is
// the reproducer. The other five are the plan's valid CT base data set.
// The expected findings on the reproducer come from the standard:
//
//   (0008,0018) SOP Instance UID, Type 1 in PS3.3 Table C.12-1, zero length.
//               PS3.5 7.4.1, TYPE1-EMPTY.
//   (0008,0060) Modality, Type 1 in PS3.3 Table C.7-5a, zero length.
//               PS3.5 7.4.1, TYPE1-EMPTY.
//   (0008,0080) Institution Name, LO, 170 characters. PS3.5 Table 6.2-1
//               "64 chars maximum", VALUE-LENGTH value 1.
//   (0010,1010) Patient's Age, AS, `040 `. PS3.5 Table 6.2-1 "nnnD, nnnW,
//               nnnM, nnnY", AS-FORMAT value 1.
//
// So the badge reads 4 and the list holds four rows. The valid files give no
// finding, so the badge is hidden on them. The steps:
//
//   open      the badge reads 4 after one request, sent with no `POST /file`
//             and no thumbnail run in progress, at least 300 ms after the
//             first paint, and the frame is drawn
//   sheet     four rows, the scope sentence, no value of the file
//   scroll    five steps 60 ms apart send nothing, and the resting file is
//             asked for once, at least 300 ms after the last step
//   back      the reproducer again shows 4 from the page's cache, no request
//   busy      with a thumbnail run, then a file post, counted as open, a
//             step to a file with no report sends nothing until the count
//             is zero again, and then one report is asked for
//   reset     a second open of the series forgets the first open's reports
//   library   with the library build answering 404 the section reads the
//             library sentence, the badge stays hidden, nothing is asked
//   failed    a failed request leaves the badge hidden, the section reads
//             "the checks could not be read", the headers table still fills
//   gone      a /conformance that always answers 410 is asked twice, by id
//             and then by body, with no `POST /file`, then never again
//   held      the first reply is held while a second folder opens, and the
//             second open's reproducer is still asked for and shows 4
//   aborted   a stale check ends in AbortError after its held /file repost,
//             and the new folder's due report is still asked for
//   colour    an RGB reproducer draws its literal three-channel pixel, and
//             the badge reads 4 without the sheet being opened
//   refused   the same for a reproducer whose Pixel Data is too short
//
// Without the library build, `npm run build`, the local process does not
// serve `@ocelli/core`. Then only the unbuilt step runs: the section reads
// the library sentence, the badge stays hidden, nothing is asked for and the
// frame is drawn. CI runs that branch. Which branch ran is printed.
//
// It starts the viewer with OCELLI_PORT=0, reads the URL it prints and stops
// it by its PID, through tools/bench's pinned Playwright.
// `bin/ocelli.sh gate viewer-browser` runs it after marks.mjs.

import { Buffer } from "node:buffer";
import { mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { fileURLToPath } from "node:url";

import { ensureViewerBinary, longEl, shortEl, startViewer } from "../../../../tools/bench/src/viewer_work.mjs";
import { loadPlaywright, stopByPid } from "./aspect.mjs";

// The scope sentence, read from the tracked mirror of `ocelli-dicom`'s
// constant, so this check needs no library build.
const CONFORMANCE_SCOPE = (() => {
  const source = readFileSync(new URL("../../../../packages/core/src/conformance.generated.ts", import.meta.url), "utf8");
  const found = source.match(/^export const CONFORMANCE_SCOPE = "([^"]+)";$/m);
  if (!found) throw new Error("conformance.generated.ts has no CONFORMANCE_SCOPE");
  return found[1];
})();
const CT = "1.2.840.10008.5.1.4.1.1.2";
const FILES = 6;
const DELAY_MS = 300;
const LIBRARY_SENTENCE = "The checks need the library build. Run npm run build in the repository, then start the viewer again.";
const DCMJS_SENTENCE = "The checks need dcmjs, which the library imports. Run npm ci in the repository, then start the viewer again.";
const UNREAD = "the checks could not be read";
const EXPECTED_ROWS = [
  ["(0008,0018)", "SOPInstanceUID", "TYPE1-EMPTY", "PS3.5 7.4.1"],
  ["(0008,0060)", "Modality", "TYPE1-EMPTY", "PS3.5 7.4.1"],
  ["(0008,0080)", "InstitutionName", "VALUE-LENGTH", "PS3.5 6.2 Table 6.2-1"],
  ["(0010,1010)", "PatientAge", "AS-FORMAT", "PS3.5 6.2 Table 6.2-1"],
];
// Written values the Checks section must never repeat.
const SECRETS = ["040", "SECRET", "AAAAAAAA"];

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
 * The plan's CT base data set at 32 by 32, every Type 1 and Type 2
 * attribute of the seven modules present. `reproducer` applies the four
 * changes. Elements are written in tag order.
 */
function ctFile(instance, reproducer, variant = "") {
  const sop = reproducer ? "" : `2.25.2250${instance}`;
  const meta = Buffer.concat([
    longEl(0x0002, 0x0001, "OB", Buffer.from([0, 1])),
    shortEl(0x0002, 0x0002, "UI", uid(CT)),
    shortEl(0x0002, 0x0003, "UI", uid(`2.25.2250${instance}`)),
    shortEl(0x0002, 0x0010, "UI", uid("1.2.840.10008.1.2.1")),
    shortEl(0x0002, 0x0012, "UI", uid("2.25.225")),
  ]);
  // `colour` is RGB, 8 bits, colour-by-pixel, with Planar Configuration
  // present because PS3.3 C.7-11c makes it 1C when Samples per Pixel is 3.
  // `refused` holds 16 bytes of Pixel Data where 32 * 32 * 2 are owed, so
  // the preview refuses the frame. Neither change is a rule the report
  // checks, so both still give the reproducer's four findings.
  const colour = variant === "colour";
  let pixels = Buffer.alloc(colour ? 32 * 32 * 3 : 32 * 32 * 2);
  if (!colour) for (let at = 0; at < 32 * 32; at += 1) pixels.writeUInt16LE((at * 13 + instance * 50) % 400, at * 2);
  // Native RGB identity, F-239 AC1: literal channels, no computed gold.
  if (colour) for (let at = 0; at < pixels.length; at += 3) pixels.set([10, 100, 200], at);
  if (variant === "refused") pixels = pixels.subarray(0, 16);
  const elements = [
    shortEl(0x0008, 0x0016, "UI", uid(CT)),
    shortEl(0x0008, 0x0018, "UI", uid(sop)),
    shortEl(0x0008, 0x0020, "DA", text("19991231")),
    shortEl(0x0008, 0x0030, "TM", text("153045")),
    shortEl(0x0008, 0x0050, "SH", text("A1")),
    shortEl(0x0008, 0x0060, "CS", text(reproducer ? "" : "CT")),
  ];
  if (reproducer) elements.push(shortEl(0x0008, 0x0080, "LO", text("A".repeat(170))));
  elements.push(
    shortEl(0x0008, 0x0090, "PN", text("N")),
    shortEl(0x0010, 0x0010, "PN", text(reproducer ? "SECRET^NAME" : "N")),
    shortEl(0x0010, 0x0020, "LO", text("P1")),
    shortEl(0x0010, 0x0030, "DA", text("19991231")),
    shortEl(0x0010, 0x0040, "CS", text("O")),
  );
  if (reproducer) elements.push(shortEl(0x0010, 0x1010, "AS", Buffer.from("040 ", "latin1")));
  elements.push(
    shortEl(0x0018, 0x0050, "DS", text("5")),
    shortEl(0x0018, 0x5100, "CS", text("HFS")),
    shortEl(0x0020, 0x000d, "UI", uid("9.8.7")),
    shortEl(0x0020, 0x000e, "UI", uid("9.8.8")),
    shortEl(0x0020, 0x0010, "SH", text("S1")),
    shortEl(0x0020, 0x0011, "IS", text("1")),
    shortEl(0x0020, 0x0013, "IS", text(String(instance))),
    shortEl(0x0020, 0x0032, "DS", text(`0\\0\\${instance}`)),
    shortEl(0x0020, 0x0037, "DS", text("1\\0\\0\\0\\1\\0")),
    shortEl(0x0028, 0x0002, "US", u16(colour ? 3 : 1)),
    shortEl(0x0028, 0x0004, "CS", text(colour ? "RGB" : "MONOCHROME2")),
    ...(colour ? [shortEl(0x0028, 0x0006, "US", u16(0))] : []),
    shortEl(0x0028, 0x0010, "US", u16(32)),
    shortEl(0x0028, 0x0011, "US", u16(32)),
    shortEl(0x0028, 0x0030, "DS", text("1\\1")),
    shortEl(0x0028, 0x0100, "US", u16(colour ? 8 : 16)),
    shortEl(0x0028, 0x0101, "US", u16(colour ? 8 : 16)),
    shortEl(0x0028, 0x0102, "US", u16(colour ? 7 : 15)),
    shortEl(0x0028, 0x0103, "US", u16(0)),
    longEl(0x7fe0, 0x0010, "OW", pixels),
  );
  return Buffer.concat([
    Buffer.alloc(128),
    Buffer.from("DICM"),
    shortEl(0x0002, 0x0000, "UL", u32(meta.length)),
    meta,
    ...elements,
  ]);
}

function writeFolder(folder) {
  mkdirSync(join(folder, "series"));
  for (let instance = 1; instance <= FILES; instance += 1) {
    writeFileSync(join(folder, "series", `c${instance}.dcm`), ctFile(instance, instance === 1));
  }
  for (const variant of ["colour", "refused"]) {
    mkdirSync(join(folder, variant));
    writeFileSync(join(folder, variant, `${variant}.dcm`), ctFile(1, true, variant));
  }
}

// Before the page's own script, so every `/conformance` fetch and every
// paint of cell 0 is seen. The wrapper reads the page's `filePostsOpen` and
// `thumbRuns` when it is called, by which time the page script has declared
// them.
function instrument() {
  window.__checks = { asked: [], paints: [] };
  const original = window.fetch.bind(window);
  window.fetch = (input, init) => {
    const url = typeof input === "string" ? input : input.url;
    if (url === "/conformance" || url.endsWith("/conformance")) {
      const entry = {
        at: performance.now(),
        files: filePostsOpen,
        thumbs: thumbRuns,
        file: checks.due ? checks.due.name : null,
        form: init && init.headers && init.headers["X-Ocelli-File"] ? "id" : "body",
        status: 0,
      };
      window.__checks.asked.push(entry);
      const answered = original(input, init);
      answered.then((response) => { entry.status = response.status; }, () => { entry.status = -1; });
      return answered;
    }
    return original(input, init);
  };
  const put = CanvasRenderingContext2D.prototype.putImageData;
  CanvasRenderingContext2D.prototype.putImageData = function (...args) {
    if (this.canvas.matches('.cell[data-cell="0"] canvas.view')) window.__checks.paints.push(performance.now());
    return put.apply(this, args);
  };
}

async function openPage(browser, url, folder, setup) {
  const page = await browser.newPage({ viewport: { width: 1600, height: 1000 } });
  page.setDefaultTimeout(20000);
  await page.addInitScript(instrument);
  if (setup) await setup(page);
  await page.goto(url);
  await page.locator("#folder").setInputFiles(folder);
  await page.waitForFunction(() => (document.getElementById("status").textContent || "").startsWith("1 series."));
  await page.waitForFunction(() => {
    const node = document.querySelector('.cell[data-cell="0"] canvas.view');
    return node && node.width === 32 && window.__checks.paints.length > 0;
  });
  return page;
}

/**
 * Reports asked for. A report is one id-form request, followed by one
 * body-form request only when the id form answered 410. Any other body-form
 * request is a problem, because every file of the open has an id.
 */
export function reportCount(asked, problems, name) {
  let count = 0;
  for (let index = 0; index < asked.length; index += 1) {
    const entry = asked[index];
    if (entry.form === "id") {
      count += 1;
      continue;
    }
    const before = asked[index - 1];
    if (!before || before.form !== "id" || before.status !== 410 || before.file !== entry.file) {
      problems.push(`${name}: a body-form request did not follow a 410 for the same file, ${JSON.stringify(asked)}`);
    }
  }
  return count;
}

const seen = (page) => page.evaluate(() => ({
  asked: window.__checks.asked.slice(),
  paints: window.__checks.paints.slice(),
  badge: document.getElementById("headers-badge").hidden ? null : document.getElementById("headers-badge").textContent,
}));

async function sheet(page) {
  await page.locator("#headers").click();
  await page.waitForFunction(() => document.getElementById("checks-message").textContent !== "Checking.");
  await page.waitForFunction(() => document.querySelectorAll("#header-rows tr").length > 0);
  const result = await page.evaluate(() => ({
    scope: document.getElementById("checks-scope").textContent,
    modules: document.getElementById("checks-modules").textContent,
    message: document.getElementById("checks-message").textContent,
    rows: Array.from(document.querySelectorAll("#checks-rows tr")).map((row) => Array.from(row.cells).map((cell) => cell.textContent)),
    section: document.getElementById("checks").textContent,
    headerRows: document.querySelectorAll("#header-rows tr").length,
  }));
  await page.locator("#close-headers").click();
  return result;
}

async function step(page, key, times, gap) {
  for (let index = 0; index < times; index += 1) {
    await page.keyboard.press(key);
    await page.waitForTimeout(gap);
  }
  return page.evaluate(() => performance.now());
}

/**
 * Without the library build the checks say so, the badge stays hidden and
 * nothing is asked for, and the frame is still drawn. CI runs this branch.
 */
async function observeUnbuilt(browser, url, folder, problems, notes, expected) {
  const page = await openPage(browser, url, folder);
  await page.waitForTimeout(800);
  const absent = await sheet(page);
  const state = await seen(page);
  if (absent.message !== expected) problems.push(`unbuilt: the section reads "${absent.message}"`);
  if (state.badge !== null) problems.push(`unbuilt: the badge reads ${state.badge}`);
  if (state.asked.length !== 0) problems.push(`unbuilt: ${state.asked.length} reports were asked for`);
  if (!(absent.headerRows > 1)) problems.push("unbuilt: the headers table did not fill");
  notes.push(`unbuilt: "${absent.message}"`);
  await page.close();
}

export async function observeChecks(url, chromium, folder) {
  const problems = [];
  const notes = [];
  const built = (await fetch(new URL("lib/ocelli-core/index.js", url))).ok;
  const dcmjs = (await fetch(new URL("lib/dcmjs.js", url))).ok;
  // The series, and beside it one folder per fault case.
  const series = join(folder, "series");
  const browser = await chromium.launch({ headless: true });
  try {
    if (!built || !dcmjs) {
      await observeUnbuilt(browser, url, series, problems, notes, built ? DCMJS_SENTENCE : LIBRARY_SENTENCE);
      return { problems, notes, served: false, chromium: browser.version() };
    }
    // open, sheet, scroll, back
    const page = await openPage(browser, url, series);
    await page.waitForFunction(() => !document.getElementById("headers-badge").hidden, null, { timeout: 10000 }).catch(() => {});
    let state = await seen(page);
    if (state.badge !== "4") problems.push(`open: the badge reads ${state.badge}, not 4`);
    const opened = reportCount(state.asked, problems, "open");
    if (opened !== 1) problems.push(`open: ${opened} reports were asked for, not 1, ${JSON.stringify(state.asked)}`);
    const first = state.asked[0];
    if (first) {
      if (first.files !== 0 || first.thumbs !== 0) problems.push(`open: asked with ${first.files} file posts and ${first.thumbs} thumbnail runs open`);
      const after = first.at - state.paints[0];
      notes.push(`open: asked ${after.toFixed(0)} ms after the first paint`);
      if (!(after >= DELAY_MS)) problems.push(`open: asked ${after.toFixed(0)} ms after the first paint, under ${DELAY_MS}`);
    }
    const shown = await sheet(page);
    if (shown.scope !== CONFORMANCE_SCOPE) problems.push(`sheet: the scope line is "${shown.scope}"`);
    if (!shown.modules.startsWith("CT Image, PS3.3 A.3.3.")) problems.push(`sheet: the module line is "${shown.modules}"`);
    if (shown.message !== "4 findings.") problems.push(`sheet: the count line is "${shown.message}"`);
    const rows = shown.rows.map((row) => row.slice(0, 4));
    if (JSON.stringify(rows) !== JSON.stringify(EXPECTED_ROWS)) problems.push(`sheet: the rows are ${JSON.stringify(shown.rows)}`);
    if (!shown.rows[2] || shown.rows[2][4] !== "value 1, 170 characters, limit 64") problems.push(`sheet: the length detail is ${JSON.stringify(shown.rows[2])}`);
    for (const secret of SECRETS) if (shown.section.includes(secret)) problems.push(`sheet: the Checks section repeats ${secret}`);
    notes.push(`sheet: ${shown.rows.length} rows, ${shown.headerRows} header rows, "${shown.modules}"`);

    const asked = state.asked.length;
    const painted = state.paints.length;
    await step(page, "ArrowDown", FILES - 1, 60);
    state = await seen(page);
    // Each step drew its file, so the steps were a scroll and not one jump.
    if (state.paints.length - painted < FILES - 1) problems.push(`scroll: ${state.paints.length - painted} paints for ${FILES - 1} steps`);
    if (state.asked.length !== asked) problems.push(`scroll: ${state.asked.length - asked} requests were sent during the steps`);
    await page.waitForTimeout(1200);
    state = await seen(page);
    const rested = reportCount(state.asked.slice(asked), problems, "scroll");
    if (rested !== 1) problems.push(`scroll: ${rested} reports were asked for after the steps, not 1, ${JSON.stringify(state.asked)}`);
    else {
      const at = state.asked[asked].at;
      const lastPaint = Math.max(...state.paints.filter((paint) => paint < at));
      const rest = at - lastPaint;
      notes.push(`scroll: the resting file was asked for ${rest.toFixed(0)} ms after its paint`);
      if (!(rest >= DELAY_MS)) problems.push(`scroll: asked ${rest.toFixed(0)} ms after the last paint, under ${DELAY_MS}`);
    }
    if (state.badge !== null) problems.push(`scroll: the badge reads ${state.badge} on a valid file`);

    const scrolled = state.asked.length;
    await step(page, "ArrowUp", FILES - 1, 60);
    await page.waitForTimeout(800);
    state = await seen(page);
    if (state.badge !== "4") problems.push(`back: the badge reads ${state.badge}, not 4`);
    if (state.asked.length !== scrolled) problems.push(`back: ${state.asked.length - scrolled} more requests were sent`);

    // busy. The page counts each `POST /file` and each thumbnail run, which
    // the page source test pins. Here the count is raised by hand while one
    // step goes to a file with no report, so the wait in `checksDue` is
    // what is observed.
    for (const counter of ["thumbRuns", "filePostsOpen"]) {
      const before = (await seen(page)).asked.length;
      await page.evaluate((name) => {
        if (name === "thumbRuns") thumbRuns += 1;
        else filePostsOpen += 1;
      }, counter);
      await step(page, "ArrowDown", 1, 0);
      await page.waitForTimeout(900);
      const held = (await seen(page)).asked.length - before;
      if (held !== 0) problems.push(`busy: ${held} requests were sent while ${counter} was above zero`);
      await page.evaluate((name) => {
        if (name === "thumbRuns") thumbRuns -= 1;
        else filePostsOpen -= 1;
      }, counter);
      await page.waitForTimeout(900);
      const after = (await seen(page)).asked.slice(before);
      const count = reportCount(after, problems, "busy");
      if (count !== 1) problems.push(`busy: ${count} reports were asked for once ${counter} was zero, not 1`);
    }
    // reset. A new open forgets every report the last open kept. Pass 1 N4.
    if ((await page.evaluate(() => checks.reports.size)) === 0) problems.push("reset: the first open kept no report to forget");
    await page.locator("#folder").setInputFiles(series);
    const kept = await page.evaluate(() => checks.reports.size);
    if (kept !== 0) problems.push(`reset: ${kept} reports of the last open were kept after a new open`);
    await page.close();

    // library
    const missing = await openPage(browser, url, series, (target) => target.route("**/lib/ocelli-core/index.js", (route) => route.fulfill({
      status: 404,
      headers: { "X-Ocelli-Error": "library" },
      body: "the library build is not on disk",
    })));
    await missing.waitForTimeout(800);
    const absent = await sheet(missing);
    state = await seen(missing);
    if (absent.message !== LIBRARY_SENTENCE) problems.push(`library: the section reads "${absent.message}"`);
    if (state.badge !== null) problems.push(`library: the badge reads ${state.badge}`);
    if (state.asked.length !== 0) problems.push(`library: ${state.asked.length} reports were asked for`);
    await missing.close();

    // failed
    const failing = await openPage(browser, url, series, (target) => target.route("**/conformance", (route) => route.abort()));
    await failing.waitForTimeout(800);
    const failed = await sheet(failing);
    state = await seen(failing);
    if (failed.message !== UNREAD) problems.push(`failed: the section reads "${failed.message}"`);
    if (state.badge !== null) problems.push(`failed: the badge reads ${state.badge}`);
    if (!(failed.headerRows > 1)) problems.push("failed: the headers table did not fill");
    await failing.close();

    // gone
    let posts = 0;
    const going = await openPage(browser, url, series, async (target) => {
      await target.route("**/conformance", (route) => route.fulfill({
        status: 410,
        headers: { "X-Ocelli-Error": "file-gone" },
        body: "the file is no longer held by this process",
      }));
      target.on("request", (request) => {
        if (request.method() === "POST" && new URL(request.url()).pathname === "/file") posts += 1;
      });
    });
    const posted = posts;
    await going.waitForTimeout(1500);
    state = await seen(going);
    // Before the sheet opens, because `/headers` posts a file again on its
    // own 410, which is F-216's rule for that route.
    if (posts !== posted) problems.push(`gone: ${posts - posted} files were posted again`);
    const gone = await sheet(going);
    const forms = state.asked.map((entry) => `${entry.form} ${entry.status}`).join(", ");
    if (forms !== "id 410, body 410") problems.push(`gone: the requests were ${forms}, not one by id and one by body`);
    if (gone.message !== UNREAD) problems.push(`gone: the section reads "${gone.message}"`);
    if (state.badge !== null) problems.push(`gone: the badge reads ${state.badge}`);
    await going.close();

    // held. The first reply is held while a second folder is opened, so the
    // new open's request comes due while the old one is out. The new open's
    // reproducer must still be asked for and show 4.
    let release = () => {};
    const held = new Promise((resolve) => { release = resolve; });
    let holdNext = true;
    const holding = await openPage(browser, url, series, (target) => target.route("**/conformance", async (route) => {
      if (holdNext) {
        holdNext = false;
        await held;
      }
      await route.continue();
    }));
    await holding.waitForFunction(() => window.__checks.asked.length > 0);
    const reopenedAt = await holding.evaluate(() => performance.now());
    await holding.locator("#folder").setInputFiles(series);
    await holding.waitForFunction((at) => window.__checks.paints.some((paint) => paint > at), reopenedAt);
    await holding.waitForTimeout(800);
    release();
    await holding.waitForFunction(() => !document.getElementById("headers-badge").hidden, null, { timeout: 6000 }).catch(() => {});
    state = await seen(holding);
    // A report starts with its id-form request. The held report's own
    // body-form follow-up is also sent after the second open, so it is not
    // counted.
    const fresh = state.asked.filter((entry) => entry.at > reopenedAt && entry.form === "id");
    if (state.badge !== "4") problems.push(`held: the badge reads ${state.badge} after a second open, not 4, ${JSON.stringify(state.asked)}`);
    if (fresh.length !== 1) problems.push(`held: ${fresh.length} reports were asked for the second open, not 1, ${JSON.stringify(state.asked)}`);
    await holding.close();

    // aborted. Pass 2 D1. The first open's check is busy inside the held
    // lazy import. Opening Headers asks the displayed file by id, which
    // answers 410 and reposts the file, and that repost is held. A second
    // folder then opens with the repost and the import both still out, so
    // its due report finds the loop busy and no timer re-arms. Releasing
    // the repost leaves the old open, so the old check must end in
    // AbortError and still hand the loop to the second folder's due report.
    let releaseImport = () => {};
    const importHeld = new Promise((resolve) => { releaseImport = resolve; });
    let releasePost = () => {};
    const postHeld = new Promise((resolve) => { releasePost = resolve; });
    let importing = false;
    let holdPost = false;
    let reposting = false;
    const aborted = await openPage(browser, url, series, async (target) => {
      await target.route("**/lib/ocelli-core/index.js", async (route) => {
        importing = true;
        await importHeld;
        await route.continue();
      });
      await target.route("**/file", async (route) => {
        if (holdPost) {
          reposting = true;
          await postHeld;
        }
        await route.continue();
      });
    });
    try {
      for (let index = 0; index < 500 && !importing; index += 1) await aborted.waitForTimeout(20);
      if (!importing) problems.push("aborted: the check never reached the held lazy import");
      if (!(await aborted.evaluate(() => checks.busy))) problems.push("aborted: the check is not busy on the held import");
      holdPost = true;
      await aborted.locator("#headers").click();
      for (let index = 0; index < 100 && !reposting; index += 1) await aborted.waitForTimeout(20);
      if (!reposting) problems.push("aborted: the headers retry never reposted the file");
      holdPost = false;
      await aborted.locator("#close-headers").click();
      const reopenedAt = await aborted.evaluate(() => performance.now());
      await aborted.locator("#folder").setInputFiles(series);
      await aborted.waitForFunction((at) => window.__checks.paints.some((paint) => paint > at), reopenedAt);
      releasePost();
      // The new folder must already be due while the old check is busy.
      // Waiting for that state keeps a slow timer from masking the defect.
      await aborted.waitForFunction(() => filePostsOpen === 0 && checks.busy && checks.due !== null && checks.due === checks.shown);
      releaseImport();
      await aborted.waitForFunction(() => !document.getElementById("headers-badge").hidden, null, { timeout: 6000 }).catch(() => {});
      const state = await seen(aborted);
      const fresh = state.asked.filter((entry) => entry.at > reopenedAt && entry.form === "id");
      if (state.badge !== "4") problems.push(`aborted: the badge reads ${state.badge} after a stale abort, not 4, ${JSON.stringify(state.asked)}`);
      if (fresh.length !== 1) problems.push(`aborted: ${fresh.length} reports were asked for the second open, not 1, ${JSON.stringify(state.asked)}`);
      const left = await aborted.evaluate(() => ({ due: checks.due ? checks.due.name : null, busy: checks.busy, kept: checks.reports.size }));
      if (left.due !== null) problems.push(`aborted: the second open's due is ${left.due}, not null`);
      if (left.busy) problems.push("aborted: the check is still busy after the stale abort");
      if (left.kept !== 1) problems.push(`aborted: ${left.kept} reports were kept for the second open, not 1`);
    } finally {
      releasePost();
      releaseImport();
      await aborted.close();
    }

    // Both a drawn colour frame and a refused frame show the conformance
    // badge without opening the sheet. F-239 now draws supported RGB.
    for (const variant of ["colour", "refused"]) {
      const faulted = await browser.newPage({ viewport: { width: 1600, height: 1000 } });
      faulted.setDefaultTimeout(20000);
      await faulted.addInitScript(instrument);
      await faulted.goto(url);
      await faulted.locator("#folder").setInputFiles(join(folder, variant));
      if (variant === "colour") {
        await faulted.waitForFunction(() => {
          const canvas = document.querySelector('.cell[data-cell="0"] canvas.view');
          const fault = document.querySelector('.cell[data-cell="0"] .fault');
          return canvas && canvas.width === 32 && window.__checks.paints.length > 0 && fault.hidden;
        });
        const pixel = await faulted.evaluate(() => [...document.querySelector(
          '.cell[data-cell="0"] canvas.view').getContext("2d").getImageData(0, 0, 1, 1).data]);
        if (JSON.stringify(pixel) !== "[10,100,200,255]") problems.push(`colour: painted pixel ${JSON.stringify(pixel)}, expected literal RGB identity`);
      } else {
        await faulted.waitForFunction(() => {
          const node = document.querySelector('.cell[data-cell="0"] .fault');
          return node && !node.hidden && node.textContent.length > 0;
        });
      }
      await faulted.waitForFunction(() => !document.getElementById("headers-badge").hidden, null, { timeout: 4000 }).catch(() => {});
      const fault = await faulted.evaluate(() => document.querySelector('.cell[data-cell="0"] .fault').textContent);
      state = await seen(faulted);
      notes.push(`${variant}: fault "${fault}", badge ${state.badge}`);
      if (state.badge !== "4") problems.push(`${variant}: the badge reads ${state.badge} beside the fault "${fault}", not 4`);
      await faulted.close();
    }
    return { problems, notes, served: true, chromium: browser.version() };
  } finally {
    await browser.close();
  }
}

async function main() {
  const { chromium } = await loadPlaywright();
  const binary = await ensureViewerBinary("release", true);
  const folder = mkdtempSync(join(tmpdir(), "ocelli-f225-"));
  const server = await startViewer(binary);
  let result;
  try {
    writeFolder(folder);
    result = await observeChecks(server.url, chromium, folder);
  } finally {
    await stopByPid(server.pid);
    rmSync(folder, { recursive: true, force: true });
  }
  console.log(`viewer ${server.url} pid ${server.pid}, library ${result.served ? "served" : "not built"}, Chromium ${result.chromium}`);
  for (const note of result.notes) console.log(note);
  if (result.problems.length) {
    for (const problem of result.problems) console.error(problem);
    process.exitCode = 1;
    return;
  }
  console.log(result.served
    ? "the badge, the Checks section and the request schedule hold on the reproducer"
    : "without the library build the Checks section said so, asked for nothing and the frame was drawn");
}

if (process.argv[1] === fileURLToPath(import.meta.url)) {
  main().catch((error) => {
    console.error(error);
    process.exitCode = 1;
  });
}
