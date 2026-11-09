// F-215 AC18 in headless Chromium, and lead amendment 6. Marks saved as a
// DICOM SR TID 1500 document and a Key Object Selection, then read back.
//
//   save     on the synthetic CT, 128 columns by 96 rows, a rectangle is
//            drawn from (20,20) to (40,40) in image pixels and its left edge
//            is dragged 200 columns to the right. Lead amendment 6 keeps
//            every point inside 0 to Columns, PS3.3 Table C.18.6-1, so the
//            rectangle must stop with its right edge at column 128 and its
//            left at 108. A freeform is drawn from (60,40) through (80,40)
//            to (70,60), and the handle on its first point is dragged to
//            column 300, which the same rule stops at column 128 (review
//            pass 1, S1). A length is drawn, a text mark "肺 Läsion a\b" is
//            placed, and the image is flagged as a key image. Save marks must
//            offer two downloads, one SR and one Key Object Selection, and say
//            "Saved 4 marks and 1 key image in 2 files."
//   load     the page is reloaded, which drops every mark, and the synthetic
//            study is opened again. Load marks with the two downloads must
//            place both marks and the key image on the image they were drawn
//            on, the same file name and frame, since the reload makes new
//            `File`s, with the rectangle's points at the FL value of the points
//            the page held when it saved, PS3.6 Graphic Data (0070,0022), and
//            the text whole. The SR is UTF-8, ISO_IR 192, PS3.3 C.12.1.1.2,
//            and Chromium's "latin1" decoder is windows-1252, which maps the
//            byte 0x82 of "肺" (E8 82 BA) above U+00FF (review pass 2, D1).
//   unflag   a new page opens the synthetic study, flags the image as a key
//            image, saves, unflags it and saves again. A Key Object Selection
//            must name at least one image, PS3.16 TID 2010 rows 8 to 10, so
//            the second save writes nothing and must say why (review pass 2,
//            D3).
//   delete   review pass 3, D3. A rectangle is saved on slice 0. A new page
//            opens a folder of slices 1 to 11 and that SR, where the mark's
//            image is missing, then the whole study and the SR, where it is
//            placed. Every mark is removed and Save marks pressed. That save
//            must write an SR of no marks naming the first, PS3.16 TID 1500
//            row 6 and PS3.3 C.17.2, so a third page opening the whole study
//            with both files shows no mark.
//   kept     review pass 4, D3. Report P holds a rectangle on slice 0 and a
//            length on slice 5, and report W a rectangle on slice 5, each
//            saved by its own page. A page opens slices 1 to 11 with P and W,
//            which places P's length and W's rectangle and not P's
//            rectangle, so P is placed only in part. Both shown marks are
//            removed and Save marks pressed. The empty report replaces W
//            only, and the status line must say that 1 report placed only in
//            part is kept, so its marks come back. The whole study with all
//            three files then shows P's two marks.
//   reverse  review pass 6, D1. The same P. A page opens the whole study with
//            P, removes P's rectangle and saves Q, which names P as replaced,
//            PS3.3 C.17.2. A page opens slices 1 to 11 with Q, which places
//            Q's length, then reads P with Load marks. P is replaced, so it
//            is not placed and not kept. The length is removed and Save marks
//            pressed: the status line must not say a report is kept, and the
//            whole study with P, Q and the new file shows no mark.
//   folders  opening B after A clears A's replacements. B's original report
//            is placed whole. Deleting B's marks and saving clears B alone.
//   nodoc    a new folder with no document does not replace A's report.
//   forward  a partial P followed by Q through Load marks retains the
//            within-folder predecessor chain when the length is deleted.
//   reset    a deletion in A is forgotten on opening B. A new B mark is
//            saved independently and does not replace A's report.
//   untracked an untracked report loaded twice into one folder keeps one
//            mark. Deleting it and saving replaces that report.
//   twice    review pass 9, N1. A page draws a rectangle on the synthetic
//            study and presses Save twice. The second Save writes nothing
//            and says no mark was added, moved or deleted.
//
// Without the library build, `npm run build`, the local process does not
// serve `@ocelli/core`, and Save marks must say so and keep the marks. CI
// runs that branch. Which branch ran is printed.
//
// The synthetic study is not a patient, and the downloads are written to a
// temporary folder that is removed afterwards. It starts the viewer with
// OCELLI_PORT=0, reads the URL it prints and stops it by its PID, through
// tools/bench's pinned Playwright. `bin/ocelli.sh gate viewer-browser` runs
// it after changed.mjs.

import { copyFileSync, mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { fileURLToPath } from "node:url";

import { ensureViewerBinary, startViewer } from "../../../../tools/bench/src/viewer_work.mjs";
import { loadPlaywright, stopByPid } from "./aspect.mjs";

export const SAVE_LIBRARY_MISSING = "Saving needs the library build. Run npm run build in the repository, then start the viewer again.";
export const SAVED = "Saved 4 marks and 1 key image in 2 files.";
export const READ = "Read 2 documents: 4 marks and 1 key image placed.";
export const TEXT = "肺 Läsion a\\b";
export const UNCHANGED = "no mark or key image changed since the folder was opened or its last save";
export const KEPT = "1 report placed only in part is kept, so the marks it holds come back.";
export const CLEARED = "Saved 0 marks and 0 key images in 1 file. 1 study has no marks left";
export const UNFLAGGED = "Nothing was saved. 1 study has no key image left.";
const SYNTHETIC_STATUS = "Synthetic study, not a patient. 1 series.";
const COLUMNS = 128;

/**
 * `seen` is `{ served, held, saved, files, loaded, status, keys }`.
 * `held` is the page's marks before saving, `saved` the status after Save
 * marks, `files` the suggested names of the downloads, `loaded` the marks
 * after the reload and Load marks, `status` the status then and `keys` the
 * number of key images then. Without the library `served` is false and
 * `saved` must be the library sentence with `held` kept.
 */
export function marksProblems(seen) {
  const problems = [];
  if (!seen || !Array.isArray(seen.held)) return ["the marks were not observed"];
  const rect = seen.held.find((mark) => mark.type === "rect");
  if (!rect) {
    problems.push("no rectangle was drawn");
  } else {
    const xs = rect.points.map((point) => point.x);
    if (Math.max(...xs) > COLUMNS || Math.min(...xs) < 0) {
      problems.push(`the dragged rectangle left the image: columns ${xs.join(" to ")}`);
    }
    if (!(Math.max(...xs) > COLUMNS - 0.01)) {
      problems.push(`the rectangle stopped at column ${Math.max(...xs)}, not at the edge ${COLUMNS}`);
    }
  }
  if (!seen.held.some((mark) => mark.type === "length")) problems.push("no length was drawn");
  const contour = seen.held.find((mark) => mark.type === "contour");
  if (!contour) {
    problems.push("no freeform was drawn");
  } else {
    const xs = contour.points.map((point) => point.x);
    if (Math.max(...xs) > COLUMNS) problems.push(`the freeform handle left the image: column ${Math.max(...xs)}`);
    if (!(Math.max(...xs) > COLUMNS - 0.01)) problems.push(`the freeform handle stopped at column ${Math.max(...xs)}, not at the edge ${COLUMNS}`);
  }
  if (!seen.served) {
    if (seen.saved !== SAVE_LIBRARY_MISSING) problems.push(`without the library Save marks said ${JSON.stringify(seen.saved)}`);
    if (seen.heldAfter !== seen.held.length) problems.push("without the library Save marks lost marks");
    return problems;
  }
  if (seen.saved !== SAVED) problems.push(`Save marks said ${JSON.stringify(seen.saved)}, not ${JSON.stringify(SAVED)}`);
  const names = (seen.files || []).slice().sort();
  if (names.length !== 2 || !names[0].startsWith("ocelli-key-images-") || !names[1].startsWith("ocelli-marks-")) {
    problems.push(`Save marks offered ${JSON.stringify(names)}`);
  }
  const cleared = seen.deleteAll;
  if (!cleared) {
    problems.push("the delete all case was not observed");
  } else {
    if (!String(cleared.partial).includes("1 item not in this folder was not placed")) {
      problems.push(`the folder without the marked image said ${JSON.stringify(cleared.partial)}`);
    }
    if (!String(cleared.whole).includes("Read 1 document: 1 mark")) problems.push(`the whole study said ${JSON.stringify(cleared.whole)}`);
    if (!String(cleared.save).startsWith(CLEARED)) problems.push(`deleting every mark and saving said ${JSON.stringify(cleared.save)}`);
    if (cleared.reopened !== 0) problems.push(`the deleted mark came back: ${cleared.reopened} marks after reopening with both files, ${JSON.stringify(cleared.reopenStatus)}`);
  }
  const kept = seen.kept;
  if (!kept) {
    problems.push("the report placed only in part was not observed");
  } else {
    if (!String(kept.opened).includes("Read 2 documents: 2 marks")) problems.push(`slices 1 to 11 with P and W said ${JSON.stringify(kept.opened)}`);
    if (!String(kept.save).startsWith("Saved 0 marks") || !String(kept.save).includes(KEPT)) {
      problems.push(`deleting every shown mark and saving said ${JSON.stringify(kept.save)}, which does not say the partly placed report is kept`);
    }
    if (kept.reopened !== 2) problems.push(`the whole study with the three files showed ${kept.reopened} marks, not P's two: ${JSON.stringify(kept.reopenStatus)}`);
  }
  const reverse = seen.reverse;
  if (!reverse) {
    problems.push("the reverse order case was not observed");
  } else {
    if (!String(reverse.opened).includes("Read 1 document: 1 mark")) problems.push(`slices 1 to 11 with Q said ${JSON.stringify(reverse.opened)}`);
    if (!String(reverse.loaded).includes("Replaced by a later document")) problems.push(`Load marks with P after Q said ${JSON.stringify(reverse.loaded)}`);
    if (String(reverse.save).includes("placed only in part")) problems.push(`the save after Q then P said ${JSON.stringify(reverse.save)}, but Q replaces P`);
    if (reverse.reopened !== 0) problems.push(`the whole study with P, Q and the new file showed ${reverse.reopened} marks: ${JSON.stringify(reverse.reopenStatus)}`);
  }
  const folders = seen.folders;
  if (!folders) {
    problems.push("the two folder case was not observed");
  } else {
    if (!String(folders.a).includes("Read 1 document: 1 mark")) problems.push(`folder A with Q said ${JSON.stringify(folders.a)}`);
    if (!String(folders.b).includes("Read 1 document: 2 marks")) problems.push(`folder B with P said ${JSON.stringify(folders.b)}`);
    if (folders.placedInB !== 2) problems.push(`folder B inherited state: ${folders.placedInB} marks`);
    if (!String(folders.save).startsWith("Saved 0 marks")) problems.push(`the save in folder B said ${JSON.stringify(folders.save)}`);
    if (folders.reopened !== 0) problems.push(`folder B with the new file showed ${folders.reopened} marks, which the page had hidden: ${JSON.stringify(folders.reopenStatus)}`);
  }
  const nodoc = seen.nodoc;
  if (!nodoc) {
    problems.push("the folder with no document case was not observed");
  } else if (nodoc.reopened !== 1) {
    problems.push(`folder B with Q after a save from a folder with no document showed ${nodoc.reopened} marks, not Q's one: save ${JSON.stringify(nodoc.save)}, reopen ${JSON.stringify(nodoc.reopenStatus)}`);
  }
  if (seen.replacedCount !== 2) problems.push(`Load P then Q kept ${seen.replacedCount} marks, expected Q and the unrelated drawing`);
  const forward = seen.forward;
  if (!forward) {
    problems.push("the forward order case was not observed");
  } else {
    if (String(forward.save).includes("placed only in part")) problems.push(`the forward order save said ${JSON.stringify(forward.save)}`);
    if (forward.reopened !== 0) problems.push(`slices 1 to 11 with P and the new file showed ${forward.reopened} marks: save ${JSON.stringify(forward.save)}, reopen ${JSON.stringify(forward.reopenStatus)}`);
  }
  if (!seen.pendingSaves || seen.pendingSaves.length !== 2) problems.push("pending saves were not observed");
  else for (const result of seen.pendingSaves) {
    const wanted = result.switchFolder ? 1 : 2;
    if (result.switchFolder && (result.afterPending.documents !== 0 || result.afterPending.changed === 0 || result.pendingFiles !== 0)) problems.push(`stale Save changed the next folder: ${JSON.stringify(result)}`);
    if (result.files !== 1 || result.groupCounts[0]?.count !== wanted || Math.abs(result.groupCounts[0]?.firstColumn - (result.switchFolder ? 60 : 10)) > 1e-3) problems.push(`pending save mixed state: ${JSON.stringify(result)}`);
  }
  const pass9 = seen.pass9;
  if (!pass9) {
    problems.push("the pass 9 cases were not observed");
  } else {
    if (pass9.held.reopened !== 3) problems.push(`folder B save replaced folder A's report: ${pass9.held.reopened} marks`);
    if (!String(pass9.untracked.repeatedStatus).includes("0 marks and 0 key images placed. 1 mark was already shown.")) problems.push(`untracked duplicate status: ${pass9.untracked.repeatedStatus}`);
    if (pass9.untracked.beforeDelete !== 1) problems.push(`repeated Load marks duplicated an untracked group: ${pass9.untracked.beforeDelete}`);
    if (pass9.untracked.reopened !== 0) problems.push(`FU with the new file showed ${pass9.untracked.reopened} marks: save ${JSON.stringify(pass9.untracked.save)}, reopen ${JSON.stringify(pass9.untracked.reopenStatus)}`);
    if (pass9.twice.files !== 1 || !String(pass9.twice.second).includes(UNCHANGED)) {
      problems.push(`two Saves with no change between wrote ${pass9.twice.files} files, the second said ${JSON.stringify(pass9.twice.second)}`);
    }
  }
  if (!String(seen.unflag).startsWith(UNFLAGGED)) {
    problems.push(`after unflagging the last key image Save marks said ${JSON.stringify(seen.unflag)}`);
  }
  if (!String(seen.status).startsWith(READ + " Key images from ocelli-key-images-")) problems.push(`Load marks said ${JSON.stringify(seen.status)}, not ${JSON.stringify(READ)}`);
  if (seen.keys !== 1) problems.push(`${seen.keys} key images after Load marks, not 1`);
  for (const mark of seen.held) {
    const back = (seen.loaded || []).find((loaded) => loaded.type === mark.type && loaded.image === mark.image);
    const want = mark.points.map((point) => ({ x: Math.fround(point.x), y: Math.fround(point.y) }));
    if (mark.type === "text" && mark.text !== TEXT) problems.push(`the text mark was drawn as ${JSON.stringify(mark.text)}`);
    if (back && mark.type === "text" && back.text !== TEXT) {
      problems.push(`the text mark came back as ${JSON.stringify(back.text)}, not ${JSON.stringify(TEXT)}`);
    }
    if (!back) {
      problems.push(`the ${mark.type} did not come back on its image`);
    } else if (JSON.stringify(back.points) !== JSON.stringify(want)) {
      problems.push(`the ${mark.type} came back at ${JSON.stringify(back.points)}, not ${JSON.stringify(want)}`);
    }
  }
  return problems;
}

function marksOf(page) {
  return page.evaluate(() => {
    const found = [];
    for (const [key, list] of notes) {
      // The file name and the frame. A reload makes new `File`s, so the
      // rest of the key changes.
      const parts = key.split("\0");
      const image = parts[0] + ":" + parts[parts.length - 1];
      for (const note of list) found.push({ image, type: note.type, text: note.text, points: note.points.map((point) => ({ x: point.x, y: point.y })) });
    }
    return found;
  });
}

async function client(page, x, y) {
  return page.evaluate(([px, py]) => {
    const at = imageToClient(activeViewport(), { x: px, y: py });
    return [at.x, at.y];
  }, [x, y]);
}

async function drag(page, from, to) {
  const [x0, y0] = await client(page, from[0], from[1]);
  const [x1, y1] = await client(page, to[0], to[1]);
  await page.mouse.move(x0, y0);
  await page.mouse.down();
  await page.mouse.move((x0 + x1) / 2, (y0 + y1) / 2, { steps: 6 });
  await page.mouse.move(x1, y1, { steps: 6 });
  await page.mouse.up();
}

async function openSynthetic(page) {
  await page.locator("#source-synthetic").click();
  await page.waitForFunction(
    (text) => (document.getElementById("status").textContent || "").startsWith(text),
    SYNTHETIC_STATUS,
  );
  await page.waitForFunction(() => {
    const vp = activeViewport();
    return Boolean(vp && vp.imageWidth && vp.scale);
  });
}

export async function observeMarks(url, chromium, downloads) {
  const browser = await chromium.launch({ headless: true });
  try {
    const page = await browser.newPage({ viewport: { width: 1400, height: 900 }, acceptDownloads: true });
    page.setDefaultTimeout(30000);
    await page.goto(url);
    await openSynthetic(page);
    const served = await page.evaluate(async () => (await window.fetch("/lib/ocelli-core/index.js")).ok && (await window.fetch("/lib/dcmjs.js")).ok);
    await page.locator("#tool-rect").click();
    await drag(page, [20, 20], [40, 40]);
    // The left edge, dragged 200 columns to the right.
    await drag(page, [20, 30], [220, 30]);
    await page.locator("#tool-contour").click();
    const [cx0, cy0] = await client(page, 60, 40);
    const [cx1, cy1] = await client(page, 80, 40);
    const [cx2, cy2] = await client(page, 70, 60);
    await page.mouse.move(cx0, cy0);
    await page.mouse.down();
    await page.mouse.move(cx1, cy1, { steps: 12 });
    await page.mouse.move(cx2, cy2, { steps: 12 });
    await page.mouse.up();
    // The new freeform is selected and shows a handle on each point. The
    // first point's handle is dragged to column 300.
    const [hx, hy] = await client(page, 60, 40);
    const [fx, fy] = await client(page, 300, 40);
    await page.mouse.move(hx, hy);
    await page.mouse.down();
    await page.mouse.move(fx, fy, { steps: 8 });
    await page.mouse.up();
    await page.locator("#tool-length").click();
    await drag(page, [10, 60], [50, 80]);
    await page.locator("#tool-text").click();
    const [tx, ty] = await client(page, 30, 80);
    await page.mouse.click(tx, ty);
    await page.locator("#note").waitFor({ state: "visible" });
    await page.keyboard.insertText(TEXT);
    await page.keyboard.press("Enter");
    await page.locator("#key-image").click();
    const held = await marksOf(page);
    const files = [];
    const saving = [];
    page.on("download", (download) => {
      const path = join(downloads, download.suggestedFilename());
      files.push(download.suggestedFilename());
      saving.push(download.saveAs(path).then(() => path));
    });
    await page.locator("#save-marks").click();
    await page.waitForFunction(() => (document.getElementById("status").textContent || "").match(/^(Saved|Saving|Nothing)/u));
    const saved = await page.evaluate(() => document.getElementById("status").textContent);
    if (!served) {
      return { served, held, saved, heldAfter: (await marksOf(page)).length };
    }
    for (let wait = 0; wait < 50 && files.length < 2; wait += 1) await page.waitForTimeout(100);
    const paths = await Promise.all(saving);
    await page.reload();
    await openSynthetic(page);
    await page.locator("#marks-file").setInputFiles(paths);
    await page.waitForFunction(() => (document.getElementById("status").textContent || "").startsWith("Read "));
    const status = await page.evaluate(() => document.getElementById("status").textContent);
    const loaded = await marksOf(page);
    const keys = await page.evaluate(() => keyImages.size);
    const flagging = await browser.newPage({ viewport: { width: 1400, height: 900 }, acceptDownloads: true });
    flagging.setDefaultTimeout(30000);
    flagging.on("download", (download) => { download.cancel().catch(() => {}); });
    await flagging.goto(url);
    await openSynthetic(flagging);
    await flagging.locator("#key-image").click();
    await flagging.locator("#save-marks").click();
    await flagging.waitForFunction(() => (document.getElementById("status").textContent || "").startsWith("Saved "));
    await flagging.locator("#key-image").click();
    await flagging.locator("#save-marks").click();
    await flagging.waitForFunction(() => !(document.getElementById("status").textContent || "").startsWith("Saved "));
    const unflag = await flagging.evaluate(() => document.getElementById("status").textContent);
    return { served, held, saved, files, loaded, status, keys, unflag };
  } finally {
    await browser.close();
  }
}

function countMarks(page) {
  return page.evaluate(() => Array.from(notes.values()).reduce((sum, list) => sum + list.length, 0));
}

async function statusStarting(page, words) {
  await page.waitForFunction((text) => (document.getElementById("status").textContent || "").includes(text), words);
  return page.evaluate(() => document.getElementById("status").textContent);
}

/** Review pass 3, D3, through the page. */
export async function observeDeleteAll(url, chromium, scratch) {
  const browser = await chromium.launch({ headless: true });
  try {
    const saving = async (page, folder) => {
      const paths = [];
      page.on("download", (download) => {
        const path = join(folder, download.suggestedFilename());
        paths.push(download.saveAs(path).then(() => path));
      });
      return paths;
    };
    const first = await browser.newPage({ viewport: { width: 1400, height: 900 }, acceptDownloads: true });
    first.setDefaultTimeout(30000);
    const firstPaths = await saving(first, scratch);
    await first.goto(url);
    await openSynthetic(first);
    await first.locator("#tool-rect").click();
    await drag(first, [20, 20], [40, 40]);
    await first.locator("#save-marks").click();
    await statusStarting(first, "Saved 1 mark");
    const [srPath] = await Promise.all(firstPaths);
    const count = (await (await fetch(url + "synthetic")).json()).slices;
    const partial = join(scratch, "partial");
    const whole = join(scratch, "whole");
    mkdirSync(partial);
    mkdirSync(whole);
    for (let index = 0; index < count; index += 1) {
      const bytes = new Uint8Array(await (await fetch(url + "synthetic/" + index)).arrayBuffer());
      writeFileSync(join(whole, "synthetic-" + index + ".dcm"), bytes);
      if (index > 0) writeFileSync(join(partial, "synthetic-" + index + ".dcm"), bytes);
    }
    copyFileSync(srPath, join(partial, "report.dcm"));
    copyFileSync(srPath, join(whole, "report.dcm"));
    const second = await browser.newPage({ viewport: { width: 1400, height: 900 }, acceptDownloads: true });
    second.setDefaultTimeout(30000);
    const out = join(scratch, "second");
    mkdirSync(out);
    const secondPaths = await saving(second, out);
    await second.goto(url);
    await second.locator("#folder").setInputFiles(partial);
    const partialStatus = await statusStarting(second, "Read 1 document");
    await second.locator("#folder").setInputFiles(whole);
    const wholeStatus = await statusStarting(second, "Read 1 document: 1 mark");
    await second.waitForFunction(() => document.querySelectorAll("#marks .mark .drop").length > 0);
    while (await second.locator("#marks .mark .drop").count()) await second.locator("#marks .mark .drop").first().click();
    await second.locator("#save-marks").click();
    await second.waitForFunction(() => /^(Saved|Nothing|There)/u.test(document.getElementById("status").textContent || ""));
    const save = await second.evaluate(() => document.getElementById("status").textContent);
    await second.waitForTimeout(500);
    for (const path of await Promise.all(secondPaths)) copyFileSync(path, join(whole, "cleared-" + path.split("/").pop()));
    const third = await browser.newPage({ viewport: { width: 1400, height: 900 } });
    third.setDefaultTimeout(30000);
    await third.goto(url);
    await third.locator("#folder").setInputFiles(whole);
    const reopenStatus = await statusStarting(third, "Read ");
    const reopened = await countMarks(third);
    return { partial: partialStatus, whole: wholeStatus, save, reopenStatus, reopened };
  } finally {
    await browser.close();
  }
}

async function toSlice(page, number, total) {
  for (let step = 1; step < number; step += 1) await page.keyboard.press("ArrowDown");
  await page.waitForFunction((want) => (document.getElementById("counter").textContent || "").trim() === want, number + " / " + total);
}

async function savedReport(browser, url, folder, draw) {
  const page = await browser.newPage({ viewport: { width: 1400, height: 900 }, acceptDownloads: true });
  page.setDefaultTimeout(30000);
  const paths = [];
  page.on("download", (download) => {
    paths.push(download.saveAs(join(folder, download.suggestedFilename())).then(() => join(folder, download.suggestedFilename())));
  });
  await page.goto(url);
  await openSynthetic(page);
  await draw(page);
  await page.locator("#save-marks").click();
  await statusStarting(page, "Saved ");
  const [path] = await Promise.all(paths);
  await page.close();
  return path;
}

/** Remove every mark the current image shows. */
async function removeShown(page) {
  await page.waitForFunction(() => document.querySelectorAll("#marks .mark .drop").length > 0);
  while (await page.locator("#marks .mark .drop").count()) await page.locator("#marks .mark .drop").first().click();
}

/** Save marks, then wait for the status line and every download it offers. */
async function saveAll(page, folder) {
  const downloads = [];
  const listener = (download) => {
    downloads.push(download.saveAs(join(folder, download.suggestedFilename())).then(() => join(folder, download.suggestedFilename())));
  };
  page.on("download", listener);
  await page.locator("#save-marks").click();
  await page.waitForFunction(() => /^(Saved|Nothing|There)/u.test(document.getElementById("status").textContent || ""));
  const status = await page.evaluate(() => document.getElementById("status").textContent);
  await page.waitForTimeout(500);
  page.off("download", listener);
  return { status, paths: await Promise.all(downloads) };
}

/** Review pass 6, D1, through the page. `p` is P from `observeKept`. */
export async function observeReverse(url, chromium, scratch, p) {
  const browser = await chromium.launch({ headless: true });
  try {
    const count = (await (await fetch(url + "synthetic")).json()).slices;
    const whole = join(scratch, "reverse-whole");
    const partial = join(scratch, "reverse-partial");
    mkdirSync(whole);
    mkdirSync(partial);
    for (let index = 0; index < count; index += 1) {
      const bytes = new Uint8Array(await (await fetch(url + "synthetic/" + index)).arrayBuffer());
      writeFileSync(join(whole, "synthetic-" + index + ".dcm"), bytes);
      if (index > 0) writeFileSync(join(partial, "synthetic-" + index + ".dcm"), bytes);
    }
    copyFileSync(p, join(whole, "report-p.dcm"));
    const making = await browser.newPage({ viewport: { width: 1400, height: 900 }, acceptDownloads: true });
    making.setDefaultTimeout(30000);
    await making.goto(url);
    await making.locator("#folder").setInputFiles(whole);
    await statusStarting(making, "Read 1 document: 2 marks");
    await removeShown(making);
    const out = join(scratch, "reverse-q");
    mkdirSync(out);
    const made = await saveAll(making, out);
    const q = made.paths[0];
    if (!q) throw new Error(`no Q was saved: ${made.status}`);
    copyFileSync(q, join(partial, "report-q.dcm"));
    await making.close();
    const page = await browser.newPage({ viewport: { width: 1400, height: 900 }, acceptDownloads: true });
    page.setDefaultTimeout(30000);
    await page.goto(url);
    await page.locator("#folder").setInputFiles(partial);
    const opened = await statusStarting(page, "Read ");
    await page.locator("#marks-file").setInputFiles(p);
    await page.waitForFunction((before) => (document.getElementById("status").textContent || "") !== before, opened);
    const loaded = await page.evaluate(() => document.getElementById("status").textContent);
    await toSlice(page, 5, count - 1);
    await removeShown(page);
    const saveOut = join(scratch, "reverse-save");
    mkdirSync(saveOut);
    const saved = await saveAll(page, saveOut);
    copyFileSync(q, join(whole, "report-q.dcm"));
    for (const path of saved.paths) copyFileSync(path, join(whole, "cleared-" + path.split("/").pop()));
    const reopen = await browser.newPage({ viewport: { width: 1400, height: 900 } });
    reopen.setDefaultTimeout(30000);
    await reopen.goto(url);
    await reopen.locator("#folder").setInputFiles(whole);
    const reopenStatus = await statusStarting(reopen, "Read ");
    const reopened = await countMarks(reopen);
    return { opened, loaded, save: saved.status, reopenStatus, reopened, q };
  } finally {
    await browser.close();
  }
}

/** Review pass 7, D1, through the page. `p` and `q` are the reports of `observeReverse`. */
export async function observeTwoFolders(url, chromium, scratch, p, q) {
  const browser = await chromium.launch({ headless: true });
  try {
    const count = (await (await fetch(url + "synthetic")).json()).slices;
    const a = join(scratch, "folder-a");
    const b = join(scratch, "folder-b");
    mkdirSync(a);
    mkdirSync(b);
    for (let index = 0; index < count; index += 1) {
      const bytes = new Uint8Array(await (await fetch(url + "synthetic/" + index)).arrayBuffer());
      writeFileSync(join(a, "synthetic-" + index + ".dcm"), bytes);
      writeFileSync(join(b, "synthetic-" + index + ".dcm"), bytes);
    }
    copyFileSync(q, join(a, "report-q.dcm"));
    copyFileSync(p, join(b, "report-p.dcm"));
    const page = await browser.newPage({ viewport: { width: 1400, height: 900 }, acceptDownloads: true });
    page.setDefaultTimeout(30000);
    await page.goto(url);
    await page.locator("#folder").setInputFiles(a);
    const inA = await statusStarting(page, "Read ");
    await page.locator("#folder").setInputFiles(b);
    await page.waitForFunction((before) => {
      const now = document.getElementById("status").textContent || "";
      return now !== before && now.includes("Read ");
    }, inA);
    const inB = await page.evaluate(() => document.getElementById("status").textContent);
    const placedInB = await countMarks(page);
    if (placedInB !== 2) throw new Error(`folder B has ${placedInB} marks, expected its own two`);
    await removeShown(page);
    await toSlice(page, 6, count);
    await removeShown(page);
    const out = join(scratch, "folders-save");
    mkdirSync(out);
    const saved = await saveAll(page, out);
    for (const path of saved.paths) copyFileSync(path, join(b, "cleared-" + path.split("/").pop()));
    const reopen = await browser.newPage({ viewport: { width: 1400, height: 900 } });
    reopen.setDefaultTimeout(30000);
    await reopen.goto(url);
    await reopen.locator("#folder").setInputFiles(b);
    const reopenStatus = await statusStarting(reopen, "Read ");
    const reopened = await countMarks(reopen);
    return { a: inA, b: inB, placedInB, save: saved.status, reopenStatus, reopened };
  } finally {
    await browser.close();
  }
}

/** Review pass 8 and its N1, through the page. */
export async function observeRulings(url, chromium, scratch, p, q) {
  const browser = await chromium.launch({ headless: true });
  try {
    const count = (await (await fetch(url + "synthetic")).json()).slices;
    const folderA = join(scratch, "rule-a");
    const folderB = join(scratch, "rule-b");
    const partial = join(scratch, "rule-partial");
    for (const folder of [folderA, folderB, partial]) mkdirSync(folder);
    for (let index = 0; index < count; index += 1) {
      const bytes = new Uint8Array(await (await fetch(url + "synthetic/" + index)).arrayBuffer());
      writeFileSync(join(folderA, "synthetic-" + index + ".dcm"), bytes);
      writeFileSync(join(folderB, "synthetic-" + index + ".dcm"), bytes);
      if (index > 0) writeFileSync(join(partial, "synthetic-" + index + ".dcm"), bytes);
    }
    copyFileSync(q, join(folderA, "report-q.dcm"));
    copyFileSync(p, join(partial, "report-p.dcm"));
    // A folder holding a document is opened once the document is read.
    const open = async (page, folder, documents = true) => {
      const before = await page.evaluate(() => document.getElementById("status").textContent || "");
      await page.locator("#folder").setInputFiles(folder);
      await page.waitForFunction(([was, read]) => {
        const now = document.getElementById("status").textContent || "";
        return now !== was && /series\./u.test(now) && (!read || now.includes("Read "));
      }, [before, documents]);
      return page.evaluate(() => document.getElementById("status").textContent);
    };
    const fresh = async () => {
      const page = await browser.newPage({ viewport: { width: 1400, height: 900 }, acceptDownloads: true });
      page.setDefaultTimeout(30000);
      await page.goto(url);
      return page;
    };
    // Folder A with Q, then folder B with no document, then Save marks.
    const first = await fresh();
    await open(first, folderA);
    await open(first, folderB, false);
    const outB = join(scratch, "rule-b-save");
    mkdirSync(outB);
    const savedB = await saveAll(first, outB);
    await first.close();
    copyFileSync(q, join(folderB, "report-q.dcm"));
    for (const path of savedB.paths) copyFileSync(path, join(folderB, "saved-" + path.split("/").pop()));
    const checkB = await fresh();
    const reopenB = await open(checkB, folderB);
    const nodoc = { save: savedB.status, reopenStatus: reopenB, reopened: await countMarks(checkB) };
    await checkB.close();
    // Slices 1 to 11 with P, then Load marks with Q, delete, save.
    const second = await fresh();
    await open(second, partial);
    const before = await second.evaluate(() => document.getElementById("status").textContent || "");
    await second.locator("#marks-file").setInputFiles(q);
    await second.waitForFunction((was) => (document.getElementById("status").textContent || "") !== was, before);
    await toSlice(second, 5, count - 1);
    await removeShown(second);
    const outF = join(scratch, "rule-forward-save");
    mkdirSync(outF);
    const savedF = await saveAll(second, outF);
    await second.close();
    for (const path of savedF.paths) copyFileSync(path, join(partial, "saved-" + path.split("/").pop()));
    const checkF = await fresh();
    const reopenF = await open(checkF, partial);
    const forward = { save: savedF.status, reopenStatus: reopenF, reopened: await countMarks(checkF) };
    // The whole report P followed by Q in this same folder must hide P's
    // rectangle as well as retain Q's length. A new drawing stays visible.
    const replacement = await fresh();
    await openSynthetic(replacement);
    await replacement.locator("#marks-file").setInputFiles(p);
    await statusStarting(replacement, "Read 1 document:");
    await replacement.locator("#tool-rect").click();
    await drag(replacement, [90, 60], [100, 70]);
    await replacement.locator("#marks-file").setInputFiles(q);
    await replacement.waitForFunction(() => (document.getElementById("status").textContent || "").includes("Read 1 document: 1 mark"));
    const replacedCount = await countMarks(replacement);
    await replacement.close();
    return { nodoc, forward, replacedCount };
  } finally {
    await browser.close();
  }
}

/** Review pass 9, D1, D2 and N1, through the page. */
export async function observePass9(url, chromium, scratch) {
  const browser = await chromium.launch({ headless: true });
  try {
    const count = (await (await fetch(url + "synthetic")).json()).slices;
    const study = async (name, withDocuments) => {
      const folder = join(scratch, name);
      mkdirSync(folder);
      for (let index = 0; index < count; index += 1) {
        writeFileSync(join(folder, "synthetic-" + index + ".dcm"), new Uint8Array(await (await fetch(url + "synthetic/" + index)).arrayBuffer()));
      }
      for (const [file, path] of withDocuments) copyFileSync(path, join(folder, file));
      return folder;
    };
    const fresh = async () => {
      const page = await browser.newPage({ viewport: { width: 1400, height: 900 }, acceptDownloads: true });
      page.setDefaultTimeout(30000);
      await page.goto(url);
      return page;
    };
    // The status is cleared first, so opening the same folder again is seen.
    const open = async (page, folder, documents) => {
      await page.evaluate(() => { document.getElementById("status").textContent = ""; });
      const before = "";
      await page.locator("#folder").setInputFiles(folder);
      await page.waitForFunction(([was, read]) => {
        const now = document.getElementById("status").textContent || "";
        return now !== was && /series\./u.test(now) && (!read || now.includes("Read "));
      }, [before, documents]);
      return page.evaluate(() => document.getElementById("status").textContent);
    };
    // T, a rectangle on slice 0 and one on slice 6, saved by its own page.
    const reports = join(scratch, "pass9-reports");
    mkdirSync(reports);
    const t = await savedReport(browser, url, reports, async (page) => {
      await page.locator("#tool-rect").click();
      await drag(page, [20, 20], [40, 40]);
      await toSlice(page, 6, count);
      await drag(page, [60, 20], [90, 40]);
    });
    const folderA = await study("pass9-a", [["report-t.dcm", t]]);
    const folderB = await study("pass9-b", []);
    const heldPage = await fresh();
    await open(heldPage, folderA, true);
    await removeShown(heldPage);
    await open(heldPage, folderB, false);
    await heldPage.locator("#tool-rect").click();
    await drag(heldPage, [50, 50], [70, 70]);
    const heldOut = join(scratch, "pass9-held");
    mkdirSync(heldOut);
    const heldSave = await saveAll(heldPage, heldOut);
    await heldPage.close();
    for (const path of heldSave.paths) copyFileSync(path, join(folderA, "from-b-" + path.split("/").pop()));
    const checkA = await fresh();
    await open(checkA, folderA, true);
    const resetReopened = await countMarks(checkA);
    await checkA.close();
    // U, one rectangle on slice 0 with no Tracking Unique Identifier.
    const { buildDocuments } = await import("../../src/mark-documents.mjs");
    const slice0 = {
      key: "u",
      sopClass: "1.2.840.10008.5.1.4.1.1.2",
      sopInstance: "2.25.41000",
      series: "2.25.42000",
      study: "2.25.43000",
      frame: 0,
      frames: 1,
      columns: 128,
      rows: 96,
      patientName: "",
      patientId: "",
      studyDate: "",
      studyTime: "",
      accession: "",
    };
    const u = buildDocuments({
      entries: [{ image: slice0, marks: [{ type: "rect", points: [{ x: 20, y: 20 }, { x: 40, y: 40 }], tracking: "" }], key: false }],
      predecessors: {},
      contentDate: "20261108",
      contentTime: "090000",
    }).documents[0];
    const uPath = join(reports, "report-u.dcm");
    writeFileSync(uPath, u.bytes);
    const folderU = await study("pass9-u", [["report-u.dcm", uPath]]);
    const untrackedPage = await fresh();
    await open(untrackedPage, folderU, true);
    await untrackedPage.locator("#marks-file").setInputFiles(uPath);
    await statusStarting(untrackedPage, "Read 1 document:");
    const beforeDelete = await countMarks(untrackedPage);
    const repeatedStatus = await untrackedPage.locator("#status").textContent();
    await removeShown(untrackedPage);
    const untrackedOut = join(scratch, "pass9-untracked");
    mkdirSync(untrackedOut);
    const untrackedSave = await saveAll(untrackedPage, untrackedOut);
    await untrackedPage.close();
    for (const path of untrackedSave.paths) copyFileSync(path, join(folderU, "saved-" + path.split("/").pop()));
    const checkU = await fresh();
    const reopenStatus = await open(checkU, folderU, true);
    const reopened = await countMarks(checkU);
    await checkU.close();
    // Draw, Save, Save.
    const twicePage = await fresh();
    await openSynthetic(twicePage);
    await twicePage.locator("#tool-rect").click();
    await drag(twicePage, [20, 20], [40, 40]);
    const twiceOut = join(scratch, "pass9-twice");
    mkdirSync(twiceOut);
    const first = await saveAll(twicePage, twiceOut);
    await twicePage.evaluate(() => { document.getElementById("status").textContent = ""; });
    const second = await saveAll(twicePage, twiceOut);
    return {
      held: { save: heldSave.status, reopened: resetReopened },
      untracked: { save: untrackedSave.status, reopenStatus, reopened, beforeDelete, repeatedStatus },
      twice: { first: first.status, second: second.status, files: first.paths.length + second.paths.length },
    };
  } finally {
    await browser.close();
  }
}

/** Review pass 4, D3, through the page. */
export async function observeKept(url, chromium, scratch) {
  const browser = await chromium.launch({ headless: true });
  try {
    const count = (await (await fetch(url + "synthetic")).json()).slices;
    const reports = join(scratch, "reports");
    mkdirSync(reports);
    const p = await savedReport(browser, url, reports, async (page) => {
      await page.locator("#tool-rect").click();
      await drag(page, [20, 20], [40, 40]);
      await toSlice(page, 6, count);
      await page.locator("#tool-length").click();
      await drag(page, [10, 60], [50, 80]);
    });
    copyFileSync(p, join(scratch, "p.dcm"));
    const w = await savedReport(browser, url, reports, async (page) => {
      await toSlice(page, 6, count);
      await page.locator("#tool-rect").click();
      await drag(page, [60, 20], [90, 40]);
    });
    copyFileSync(w, join(scratch, "w.dcm"));
    const partial = join(scratch, "kept-partial");
    const whole = join(scratch, "kept-whole");
    mkdirSync(partial);
    mkdirSync(whole);
    for (let index = 0; index < count; index += 1) {
      const bytes = new Uint8Array(await (await fetch(url + "synthetic/" + index)).arrayBuffer());
      writeFileSync(join(whole, "synthetic-" + index + ".dcm"), bytes);
      if (index > 0) writeFileSync(join(partial, "synthetic-" + index + ".dcm"), bytes);
    }
    for (const folder of [partial, whole]) {
      copyFileSync(join(scratch, "p.dcm"), join(folder, "report-p.dcm"));
      copyFileSync(join(scratch, "w.dcm"), join(folder, "report-w.dcm"));
    }
    const page = await browser.newPage({ viewport: { width: 1400, height: 900 }, acceptDownloads: true });
    page.setDefaultTimeout(30000);
    const out = join(scratch, "kept-out");
    mkdirSync(out);
    const downloads = [];
    page.on("download", (download) => {
      downloads.push(download.saveAs(join(out, download.suggestedFilename())).then(() => join(out, download.suggestedFilename())));
    });
    await page.goto(url);
    await page.locator("#folder").setInputFiles(partial);
    const opened = await statusStarting(page, "Read ");
    await toSlice(page, 5, count - 1);
    await page.waitForFunction(() => document.querySelectorAll("#marks .mark .drop").length > 0);
    while (await page.locator("#marks .mark .drop").count()) await page.locator("#marks .mark .drop").first().click();
    await page.locator("#save-marks").click();
    await page.waitForFunction(() => /^(Saved|Nothing|There)/u.test(document.getElementById("status").textContent || ""));
    const save = await page.evaluate(() => document.getElementById("status").textContent);
    await page.waitForTimeout(500);
    for (const path of await Promise.all(downloads)) copyFileSync(path, join(whole, "cleared-" + path.split("/").pop()));
    const reopen = await browser.newPage({ viewport: { width: 1400, height: 900 } });
    reopen.setDefaultTimeout(30000);
    await reopen.goto(url);
    await reopen.locator("#folder").setInputFiles(whole);
    const reopenStatus = await statusStarting(reopen, "Read ");
    const reopened = await countMarks(reopen);
    return { opened, save, reopenStatus, reopened };
  } finally {
    await browser.close();
  }
}

/** A lazy library import must not mix a save with another folder or stale edits. */
export async function observePendingSaves(url, chromium, scratch) {
  const browser = await chromium.launch({ headless: true });
  const results = [];
  try {
    for (const switchFolder of [true, false]) {
      const page = await browser.newPage({ viewport: { width: 1400, height: 900 }, acceptDownloads: true });
      page.setDefaultTimeout(30000);
      const paths = [];
      let downloaded;
      const downloadSeen = new Promise((resolve) => { downloaded = resolve; });
      page.on("download", (download) => { downloaded(); paths.push(download.saveAs(join(scratch, (switchFolder ? "folder-" : "same-") + download.suggestedFilename())).then(() => join(scratch, (switchFolder ? "folder-" : "same-") + download.suggestedFilename()))); });
      await page.goto(url);
      await openSynthetic(page);
      await page.locator("#tool-rect").click();
      await drag(page, [10, 10], [20, 20]);
      let release;
      let arrived;
      const held = new Promise((resolve) => { release = resolve; });
      const waiting = new Promise((resolve) => { arrived = resolve; });
      await page.route("**/mark-documents.mjs", async (route) => { arrived(); await held; await route.continue(); });
      // Observe the actual button handler's promise without changing its state.
      await page.evaluate(() => {
        const original = window.saveMarks;
        const button = document.getElementById("save-marks");
        button.removeEventListener("click", original);
        button.addEventListener("click", (...args) => {
          window.pendingSaveProbe = original(...args);
        });
      });
      await page.locator("#save-marks").click();
      await waiting;
      if (!await page.evaluate(() => window.pendingSaveProbe instanceof Promise)) throw new Error("the Save button promise was not observed");
      if (switchFolder) await openSynthetic(page);
      await page.locator("#tool-rect").click();
      await drag(page, [60, 60], [70, 70]);
      release();
      await page.evaluate(() => window.pendingSaveProbe);
      const afterPending = await page.evaluate("({ documents: sessionDocuments.size, changed: changedStudies.size })");
      const pendingFiles = paths.length;
      if (switchFolder) {
        await page.locator("#save-marks").click();
        await page.evaluate(() => window.pendingSaveProbe);
      }
      await downloadSeen;
      const files = await Promise.all(paths);
      const groupCounts = await page.evaluate(async (all) => {
        const module = await import("/mark-documents.mjs");
        return all.map((bytes) => {
          const groups = module.readDocument("pending-save.dcm", new Uint8Array(bytes)).groups;
          return { count: groups.length, firstColumn: module.groupToMark(groups[0]).mark.points[0].x };
        });
      }, files.map((path) => Array.from(readFileSync(path))));
      results.push({ switchFolder, groupCounts, files: files.length, afterPending, pendingFiles });
      await page.close();
    }
    return results;
  } finally {
    await browser.close();
  }
}

async function main() {
  const { chromium } = await loadPlaywright();
  const binary = await ensureViewerBinary("release", true);
  const downloads = mkdtempSync(join(tmpdir(), "ocelli-f215-"));
  const server = await startViewer(binary);
  let seen;
  try {
    seen = await observeMarks(server.url, chromium, downloads);
    if (seen.served) {
      const scratch = join(downloads, "delete");
      mkdirSync(scratch);
      seen.deleteAll = await observeDeleteAll(server.url, chromium, scratch);
      const keptScratch = join(downloads, "kept");
      mkdirSync(keptScratch);
      seen.kept = await observeKept(server.url, chromium, keptScratch);
      const reverseScratch = join(downloads, "reverse");
      mkdirSync(reverseScratch);
      seen.reverse = await observeReverse(server.url, chromium, reverseScratch, join(keptScratch, "p.dcm"));
      const foldersScratch = join(downloads, "folders");
      mkdirSync(foldersScratch);
      seen.folders = await observeTwoFolders(server.url, chromium, foldersScratch, join(keptScratch, "p.dcm"), seen.reverse.q);
      const rulingsScratch = join(downloads, "rulings");
      mkdirSync(rulingsScratch);
      Object.assign(seen, await observeRulings(server.url, chromium, rulingsScratch, join(keptScratch, "p.dcm"), seen.reverse.q));
      const pass9Scratch = join(downloads, "pass9");
      mkdirSync(pass9Scratch);
      seen.pass9 = await observePass9(server.url, chromium, pass9Scratch);
      seen.pendingSaves = await observePendingSaves(server.url, chromium, downloads);
    }
  } finally {
    await stopByPid(server.pid);
    rmSync(downloads, { recursive: true, force: true });
  }
  console.log(`viewer ${server.url} pid ${server.pid}, library ${seen.served ? "served" : "not built"}`);
  console.log(`held ${JSON.stringify(seen.held)}`);
  console.log(`save: ${JSON.stringify(seen.saved)}`);
  if (seen.served) {
    console.log(`files ${JSON.stringify(seen.files)}`);
    console.log(`load: ${JSON.stringify(seen.status)}, ${seen.keys} key images`);
    console.log(`loaded ${JSON.stringify(seen.loaded)}`);
    console.log(`unflag: ${JSON.stringify(seen.unflag)}`);
    console.log(`delete all: ${JSON.stringify(seen.deleteAll)}`);
    console.log(`kept: ${JSON.stringify(seen.kept)}`);
    console.log(`reverse: ${JSON.stringify(seen.reverse)}`);
    console.log(`folders: ${JSON.stringify(seen.folders)}`);
    console.log(`nodoc: ${JSON.stringify(seen.nodoc)}`);
    console.log(`forward: ${JSON.stringify(seen.forward)}`);
    console.log(`pass9: ${JSON.stringify(seen.pass9)}`);
  }
  const problems = marksProblems(seen);
  if (problems.length) {
    for (const problem of problems) console.error(problem);
    process.exitCode = 1;
    return;
  }
  console.log(seen.served
    ? "the marks and the key image were saved as two documents and came back on their image, and a dragged mark stayed inside it"
    : "without the library build Save marks said so and kept the marks, and a dragged mark stayed inside its image");
}

if (process.argv[1] === fileURLToPath(import.meta.url)) {
  main().catch((error) => {
    console.error(error);
    process.exitCode = 1;
  });
}
