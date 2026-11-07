// F-226 AC13 to AC15. The local process forgets the folder the page left.
//
//   left     folder A of four files is opened and drawn, then folder B of
//            three. `GET /held` must count B's three files and frames and
//            nothing of A, and a preview of each of A's ids must answer 410.
//   late     folder A of one file is opened and its one preview is held by a
//            Playwright route. Folder B is opened, then the held preview is
//            let through. It must answer 410, and the page must not post A's
//            file again.
//   hide     a second page opens folder B and draws it. A folder of
//            3,400 files is opened on the first page and a `pagehide` event
//            is dispatched to it. The first page must send exactly one
//            request to `/forget`, under 1 KiB, and `GET /held` must then
//            count B's three files and frames and none of the 3,400. The
//            event is dispatched rather than caused by leaving, because
//            Playwright reports no request a document sends while it
//            unloads.
//   leave    folder B is opened and drawn, and the page navigates away. The
//            process must then hold none of B, which is the real `pagehide`
//            seen from the process side.
//
// Every file is written by this script into a temporary folder, is 8 by 8
// mono16 CT with no patient attributes, and is removed afterwards. Each case
// starts its own viewer with OCELLI_PORT=0, reads the URL it prints and stops
// it by its PID, through tools/bench's pinned Playwright.
// `bin/ocelli.sh gate viewer-browser` runs it after link.mjs.

import { Buffer } from "node:buffer";
import { mkdirSync, mkdtempSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { fileURLToPath } from "node:url";

import { ensureViewerBinary, longEl, shortEl, startViewer } from "../../../../tools/bench/src/viewer_work.mjs";
import { loadPlaywright, stopByPid } from "./aspect.mjs";

/** Files in the `hide` folder, more ids than one 64 KiB beacon could list. */
export const HIDE_FILES = 3400;
/** The most bytes the one `pagehide` beacon may carry. */
export const BEACON_BYTES = 1024;
const A_FILES = 4;
const B_FILES = 3;

/**
 * `seen` is `{ left, late, hide }`.
 *
 * `left` is `{ bFiles, held: { files, frames }, aStatuses }`, the census
 * after B opened and the status each of A's ids answered.
 *
 * `late` is `{ caught, lateStatus, reposts }`: whether one of A's previews
 * was held, what it answered once let through after B opened, and how many
 * `/file` bodies carried A's file after that.
 *
 * `hide` is `{ ids, beacons, held, other }`: how many ids the first page
 * received, the body length in bytes of each `/forget` request its
 * `pagehide` sent, the files and frames `GET /held` counted afterwards, and
 * the files and frames `GET /held` counted while only the second page held
 * any, measured before the first page opened its folder.
 *
 * `leave` is `{ held }`, the files and frames `GET /held` counted after the
 * page navigated away.
 */
export function forgetProblems(seen) {
  const problems = [];
  const left = seen && seen.left;
  if (!left || !left.held || !Array.isArray(left.aStatuses)) {
    problems.push("the second open was not observed");
  } else {
    const counted = left.held.files + left.held.frames;
    if (counted !== left.bFiles) {
      problems.push(`after B opened, GET /held counted ${left.held.files} files and ${left.held.frames} frames, not B's ${left.bFiles}`);
    }
    if (left.aStatuses.length === 0) {
      problems.push("no id of A was observed");
    }
    for (const status of left.aStatuses) {
      if (status !== 410) problems.push(`an id of A answered ${status} after B opened, not 410`);
    }
  }
  const late = seen && seen.late;
  if (!late || !late.caught) {
    problems.push("no preview of A was held, so the late reply was not observed");
  } else {
    if (late.lateStatus !== 410) {
      problems.push(`the late preview of A answered ${late.lateStatus}, not 410`);
    }
    if (late.reposts !== 0) {
      problems.push(`A's file was posted again ${late.reposts} times after B opened`);
    }
  }
  const hide = seen && seen.hide;
  if (!hide || !Array.isArray(hide.beacons)) {
    problems.push("leaving the page was not observed");
  } else {
    if (hide.beacons.length !== 1) {
      problems.push(`the pagehide sent ${hide.beacons.length} forget requests for ${hide.ids} ids, not 1`);
    }
    for (const bytes of hide.beacons) {
      if (bytes >= BEACON_BYTES) problems.push(`a pagehide beacon carried ${bytes} bytes, not under ${BEACON_BYTES}`);
    }
    if (!(hide.other > 0)) {
      problems.push("the second page held nothing before the pagehide, so the case shows nothing");
    } else if (hide.held !== hide.other) {
      problems.push(`after the pagehide GET /held counted ${hide.held} files and frames, not the second page's ${hide.other}`);
    }
  }
  const leave = seen && seen.leave;
  if (!leave || typeof leave.held !== "number") {
    problems.push("navigating away was not observed");
  } else if (leave.held !== 0) {
    problems.push(`after the page navigated away GET /held still counted ${leave.held} files and frames`);
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

/** One 8 by 8 mono16 CT slice, explicit VR little endian, no patient attributes. */
function slice(sop, series, instance) {
  const sopClass = "1.2.840.10008.5.1.4.1.1.2";
  const meta = Buffer.concat([
    longEl(0x0002, 0x0001, "OB", Buffer.from([0, 1])),
    shortEl(0x0002, 0x0002, "UI", uid(sopClass)),
    shortEl(0x0002, 0x0003, "UI", uid(sop)),
    shortEl(0x0002, 0x0010, "UI", uid("1.2.840.10008.1.2.1")),
    shortEl(0x0002, 0x0012, "UI", uid("2.25.226")),
  ]);
  const pixels = Buffer.alloc(8 * 8 * 2);
  for (let at = 0; at < 8 * 8; at += 1) pixels.writeUInt16LE(1000 + ((at * 7) % 200), at * 2);
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
    shortEl(0x0028, 0x0010, "US", u16(8)),
    shortEl(0x0028, 0x0011, "US", u16(8)),
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

/** `count` slices of one series. The SOP UIDs start with `tag`, so a posted body names its folder. */
function writeFolder(root, name, tag, count) {
  const folder = join(root, name);
  mkdirSync(folder);
  for (let k = 0; k < count; k += 1) {
    writeFileSync(join(folder, `${name}-${k}.dcm`), slice(`${tag}.${k + 1}`, `${tag}.0`, k + 1));
  }
  return folder;
}

async function heldCensus(url) {
  const response = await globalThis.fetch(`${url}held`, { cache: "no-store" });
  if (!response.ok) throw new Error(`GET /held answered ${response.status}`);
  return response.json();
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

/** Ids from every `/file` reply, in arrival order. */
function collectIds(page, sink) {
  page.on("response", (response) => {
    if (!response.url().endsWith("/file") || response.request().method() !== "POST") return;
    const id = response.headers()["x-ocelli-file"];
    if (id) sink.push(id);
  });
}

async function until(test, what, limit = 30000) {
  const started = Date.now();
  while (!(await test())) {
    if (Date.now() - started > limit) throw new Error(`${what} did not happen within ${limit} ms`);
    await new Promise((resolve) => setTimeout(resolve, 50));
  }
}

function drawnFor(page, ids) {
  return page.waitForResponse((response) => response.url().endsWith("/preview")
    && response.status() === 200
    && ids.includes(response.request().headers()["x-ocelli-file"]), { timeout: 30000 });
}

/** A shows four files and is drawn, B shows three. */
export async function observeLeft(url, chromium, folders) {
  const browser = await chromium.launch({ headless: true });
  try {
    const page = await browser.newPage({ viewport: { width: 1400, height: 900 } });
    const ids = [];
    collectIds(page, ids);
    await page.goto(url);
    const drawnA = drawnFor(page, ids);
    await page.locator("#folder").setInputFiles(folders.a);
    await drawnA;
    await until(() => ids.length === A_FILES, "A's posts");
    const aIds = ids.slice();
    const bIds = [];
    const forgot = page.waitForResponse((response) => response.url().endsWith("/forget"), { timeout: 10000 })
      .catch(() => null);
    const drawnB = drawnFor(page, bIds);
    page.on("response", (response) => {
      if (!response.url().endsWith("/file")) return;
      const id = response.headers()["x-ocelli-file"];
      if (id && !aIds.includes(id) && !bIds.includes(id)) bIds.push(id);
    });
    await page.locator("#folder").setInputFiles(folders.b);
    await until(() => bIds.length === B_FILES, "B's posts");
    await drawnB;
    await forgot;
    const held = await heldCensus(url);
    const aStatuses = [];
    for (const id of aIds) aStatuses.push(await previewStatus(url, id));
    return { bFiles: B_FILES, held: { files: held.files, frames: held.frames }, aStatuses };
  } finally {
    await browser.close();
  }
}

/** A's only preview is held until B is open, then let through. */
export async function observeLate(url, chromium, folders) {
  const browser = await chromium.launch({ headless: true });
  try {
    const page = await browser.newPage({ viewport: { width: 1400, height: 900 } });
    let phase = "a";
    let held = null;
    let caught = () => {};
    const caughtOnce = new Promise((resolve) => { caught = resolve; });
    await page.route("**/preview", async (route) => {
      const request = route.request();
      if (phase === "a" && !held && request.headers()["x-ocelli-file"]) {
        held = route;
        caught();
        return;
      }
      await route.continue();
    });
    let reposts = 0;
    page.on("request", (request) => {
      if (phase !== "released" || !request.url().endsWith("/file")) return;
      const body = request.postDataBuffer();
      if (body && body.includes(Buffer.from("2.25.2263."))) reposts += 1;
    });
    let lateStatus = null;
    page.on("response", (response) => {
      if (response.request() === (held && held.request())) lateStatus = response.status();
    });
    await page.goto(url);
    await page.locator("#folder").setInputFiles(folders.lateA);
    const wasCaught = await Promise.race([
      caughtOnce.then(() => true),
      new Promise((resolve) => setTimeout(() => resolve(false), 30000)),
    ]);
    if (!wasCaught) return { caught: false, lateStatus: null, reposts: 0 };
    phase = "b";
    const bPosts = [];
    page.on("response", (response) => {
      if (response.url().endsWith("/file")) bPosts.push(response.status());
    });
    const forgot = page.waitForResponse((response) => response.url().endsWith("/forget"), { timeout: 10000 })
      .catch(() => null);
    await page.locator("#folder").setInputFiles(folders.b);
    await until(() => bPosts.length === B_FILES, "B's posts");
    await forgot;
    phase = "released";
    await held.continue();
    await until(() => lateStatus !== null, "the late reply", 10000).catch(() => {});
    await page.waitForTimeout(1500);
    return { caught: true, lateStatus, reposts };
  } finally {
    await browser.close();
  }
}

/** A second page holds B, the first holds 3,400 files and gets a `pagehide` event. */
export async function observeHide(url, chromium, folders) {
  const browser = await chromium.launch({ headless: true });
  try {
    const other = await browser.newPage({ viewport: { width: 1400, height: 900 } });
    const otherIds = [];
    collectIds(other, otherIds);
    await other.goto(url);
    const drawn = drawnFor(other, otherIds);
    await other.locator("#folder").setInputFiles(folders.b);
    await until(() => otherIds.length === B_FILES, "the second page's posts");
    await drawn;
    const otherCensus = await heldCensus(url);
    const otherHeld = otherCensus.files + otherCensus.frames;
    const page = await browser.newPage({ viewport: { width: 1400, height: 900 } });
    page.setDefaultTimeout(180000);
    const ids = [];
    collectIds(page, ids);
    await page.goto(url);
    await page.locator("#folder").setInputFiles(folders.hide);
    await until(() => ids.length === HIDE_FILES, "the 3,400 posts", 180000);
    await page.waitForFunction(() => (document.getElementById("status").textContent || "").startsWith("1 series."));
    const beacons = [];
    const answered = [];
    page.on("request", (request) => {
      if (request.url().endsWith("/forget")) beacons.push(request.postDataBuffer()?.length ?? 0);
    });
    page.on("response", (response) => {
      if (response.url().endsWith("/forget")) answered.push(response.status());
    });
    await page.evaluate(() => window.dispatchEvent(new window.PageTransitionEvent("pagehide", { persisted: false })));
    await until(() => answered.length >= 1, "the beacon reply", 10000).catch(() => {});
    await page.waitForTimeout(500);
    const census = await heldCensus(url);
    return { ids: ids.length, beacons, held: census.files + census.frames, other: otherHeld };
  } finally {
    await browser.close();
  }
}

/** B is opened and drawn, then the page navigates away. */
export async function observeLeave(url, chromium, folders) {
  const browser = await chromium.launch({ headless: true });
  try {
    const page = await browser.newPage({ viewport: { width: 1400, height: 900 } });
    const ids = [];
    collectIds(page, ids);
    await page.goto(url);
    const drawn = drawnFor(page, ids);
    await page.locator("#folder").setInputFiles(folders.b);
    await until(() => ids.length === B_FILES, "B's posts");
    await drawn;
    await page.goto("about:blank");
    let census = await heldCensus(url);
    await until(async () => {
      census = await heldCensus(url);
      return census.files + census.frames === 0;
    }, "the forget after leaving", 3000).catch(() => {});
    return { held: census.files + census.frames };
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
  const root = mkdtempSync(join(tmpdir(), "ocelli-f226-"));
  const seen = {};
  try {
    const folders = {
      a: writeFolder(root, "a", "2.25.2261", A_FILES),
      b: writeFolder(root, "b", "2.25.2262", B_FILES),
      lateA: writeFolder(root, "late", "2.25.2263", 1),
      hide: writeFolder(root, "hide", "2.25.2264", HIDE_FILES),
    };
    seen.left = await withViewer(binary, (url) => observeLeft(url, chromium, folders));
    console.log(`left: B ${seen.left.bFiles} files, held ${seen.left.held.files} files and ${seen.left.held.frames} frames, A answered ${seen.left.aStatuses.join(" ")}`);
    seen.late = await withViewer(binary, (url) => observeLate(url, chromium, folders));
    console.log(`late: caught ${seen.late.caught}, answered ${seen.late.lateStatus}, ${seen.late.reposts} posts of A after B opened`);
    seen.hide = await withViewer(binary, (url) => observeHide(url, chromium, folders));
    const largest = seen.hide.beacons.length ? Math.max(...seen.hide.beacons) : 0;
    console.log(`hide: ${seen.hide.ids} ids, ${seen.hide.beacons.length} forget requests, the largest ${largest} bytes, then ${seen.hide.held} files and frames held, the second page's measured ${seen.hide.other}`);
    seen.leave = await withViewer(binary, (url) => observeLeave(url, chromium, folders));
    console.log(`leave: ${seen.leave.held} files and frames held after the page navigated away`);
  } finally {
    rmSync(root, { recursive: true, force: true });
  }
  const problems = forgetProblems(seen);
  if (problems.length) {
    for (const problem of problems) console.error(problem);
    process.exitCode = 1;
    return;
  }
  console.log("the process forgets the folder the page left, and leaving the page forgets every file that page posted");
}

if (process.argv[1] === fileURLToPath(import.meta.url)) {
  main().catch((error) => {
    console.error(error);
    process.exitCode = 1;
  });
}
