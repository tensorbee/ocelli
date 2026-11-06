// F-208. The pointer readout, the cross and the magnifier are placed for one
// view geometry. With the pointer still, each change below must leave them
// hidden, and the next pointer move must place them again.
//
//   layout   1 by 1 to 2 by 1 through the keyboard picker (G, Enter)
//   resize   the window from 1600 by 1000 to 1200 by 800
//   zoom     the `=` key
//   tiles    Shift G on, then Shift G off
//
// It runs on the synthetic study, starts the viewer with OCELLI_PORT=0,
// reads the URL it prints and stops it by its PID, through tools/bench's
// pinned Playwright. `bin/ocelli.sh gate viewer-browser` runs it after
// aspect.mjs.

import { fileURLToPath } from "node:url";

import { ensureViewerBinary, startViewer } from "../../../../tools/bench/src/viewer_work.mjs";
import { loadPlaywright, stopByPid } from "./aspect.mjs";

const SYNTHETIC_STATUS = "Synthetic study, not a patient. 1 series.";
// A readout that carries the reply's value after the column and row.
const VALUE_SHOWN = /^C \d+ {2}R \d+ {2}\S/;

/**
 * Each observation is `{ name, before, after, moved, shown, again }`.
 * `before`, `after` and `moved` are `{ read, readHidden, crossHidden,
 * magHidden }`. `shown` names what was showing before the change and
 * `again` what the move must show again, each "readout", "magnifier" or
 * both. A cell in tile mode shows the magnifier and no readout.
 */
export function readoutProblems(observations) {
  const problems = [];
  if (!Array.isArray(observations) || observations.length === 0) {
    problems.push("no readout change was observed");
    return problems;
  }
  for (const seen of observations) {
    if ((seen.shown.includes("readout") && (seen.before.readHidden || !VALUE_SHOWN.test(seen.before.read))) ||
        (seen.shown.includes("magnifier") && seen.before.magHidden)) {
      problems.push(`${seen.name}: nothing was shown before the change`);
      continue;
    }
    if (!seen.after.readHidden || !seen.after.crossHidden || !seen.after.magHidden) {
      problems.push(`${seen.name}: the readout, cross or magnifier survived the change`);
    }
    if (seen.again.includes("readout") && (seen.moved.readHidden || !VALUE_SHOWN.test(seen.moved.read))) {
      problems.push(`${seen.name}: a pointer move did not show the readout again`);
    }
    if (seen.again.includes("magnifier") && seen.moved.magHidden) {
      problems.push(`${seen.name}: a pointer move did not show the magnifier again`);
    }
  }
  return problems;
}

function state(page) {
  return page.evaluate(() => {
    const read = document.getElementById("cross-read");
    return {
      read: read.textContent || "",
      readHidden: read.hidden,
      crossHidden: document.getElementById("cross-h").hidden && document.getElementById("cross-v").hidden,
      magHidden: document.getElementById("mag").hidden,
    };
  });
}

async function centreOf(page, cell) {
  const box = await page.locator(`.cell[data-cell="${cell}"]`).boundingBox();
  return { x: box.x + box.width / 2 + 7, y: box.y + box.height / 2 + 5 };
}

async function pointAt(page, point, expect) {
  await page.mouse.move(point.x, point.y);
  await page.mouse.move(point.x + 1, point.y);
  await page.waitForFunction(({ wantsReadout, wantsMag, pattern }) => {
    const read = document.getElementById("cross-read");
    const shown = !read.hidden && new RegExp(pattern).test(read.textContent || "");
    const mag = !document.getElementById("mag").hidden;
    return (!wantsReadout || shown) && (!wantsMag || mag);
  }, { wantsReadout: expect.includes("readout"), wantsMag: expect.includes("magnifier"), pattern: VALUE_SHOWN.source });
}

async function observe(page, name, shown, again, change, cell = 0) {
  const point = await centreOf(page, cell);
  try {
    await pointAt(page, point, shown);
  } catch (error) {
    throw new Error(`${name}: the readout was not shown before the change, ${error instanceof Error ? error.message : String(error)}`);
  }
  const before = await state(page);
  await change();
  await page.waitForTimeout(400);
  const after = await state(page);
  const next = await centreOf(page, cell);
  let moved;
  try {
    await pointAt(page, { x: next.x + 3, y: next.y + 2 }, again);
  } catch {
    // The problem list says what did not come back.
  }
  moved = await state(page);
  return { name, before, after, moved, shown, again };
}

export async function observeReadout(url, chromium) {
  const browser = await chromium.launch({ headless: true });
  const observations = [];
  try {
    const page = await browser.newPage({ viewport: { width: 1600, height: 1000 } });
    page.setDefaultTimeout(30000);
    await page.goto(url);
    await page.locator("#source-synthetic").click();
    await page.waitForFunction(
      (text) => (document.getElementById("status").textContent || "").startsWith(text),
      SYNTHETIC_STATUS,
    );
    await page.waitForFunction(() => {
      const stage = document.querySelector('.cell[data-cell="0"] .stage');
      return stage && !stage.hidden;
    });
    await page.keyboard.press("c");
    await page.keyboard.press("m");
    const both = ["readout", "magnifier"];
    const mag = ["magnifier"];
    observations.push(await observe(page, "zoom with =", both, both, () => page.keyboard.press("=")));
    observations.push(await observe(page, "window resize", both, both,
      () => page.setViewportSize({ width: 1200, height: 800 })));
    observations.push(await observe(page, "tile mode on", both, mag, () => page.keyboard.press("Shift+G")));
    observations.push(await observe(page, "tile mode off", mag, both, () => page.keyboard.press("Shift+G")));
    observations.push(await observe(page, "layout 1 by 1 to 2 by 1", both, both, async () => {
      await page.keyboard.press("g");
      await page.locator('#layout-picker [data-columns="2"][data-rows="1"]').focus();
      await page.keyboard.press("Enter");
    }));
    return { observations, chromium: browser.version() };
  } finally {
    await browser.close();
  }
}

async function main() {
  const { chromium } = await loadPlaywright();
  const binary = await ensureViewerBinary("release", true);
  const server = await startViewer(binary);
  let result;
  try {
    result = await observeReadout(server.url, chromium);
  } finally {
    await stopByPid(server.pid);
  }
  console.log(`viewer ${server.url} pid ${server.pid}, Chromium ${result.chromium}`);
  for (const seen of result.observations) {
    console.log(`${seen.name}: before ${JSON.stringify(seen.before.read)}, after hidden ${seen.after.readHidden}/${seen.after.magHidden}, after a move ${JSON.stringify(seen.moved.read)} magnifier ${seen.moved.magHidden ? "hidden" : "shown"}`);
  }
  const problems = readoutProblems(result.observations);
  if (problems.length) {
    for (const problem of problems) console.error(problem);
    process.exitCode = 1;
    return;
  }
  console.log(`all ${result.observations.length} changes hid the readout and a move showed it again`);
}

if (process.argv[1] === fileURLToPath(import.meta.url)) {
  main().catch((error) => {
    console.error(error instanceof Error ? error.message : String(error));
    process.exitCode = 1;
  });
}
