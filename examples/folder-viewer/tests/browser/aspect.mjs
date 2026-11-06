// F-208 AC14. The displayed width over height of the synthetic study.
//
// The synthetic CT has 128 columns, 96 rows and Pixel Spacing `0.7\0.5`.
// PS3.3 10.7.1.3: the first value is the row spacing, the distance between
// the centres of adjacent rows, and the second is the column spacing. So a
// row of the image is 128 * 0.5 = 64 mm wide and a column is
// 96 * 0.7 = 67.2 mm tall, and the picture on screen must be 64 : 67.2,
// which is 0.952381 to six places. Swapping the two spacings gives
// 128 * 0.7 : 96 * 0.5 = 89.6 : 48 = 1.866667, which this check refuses.
//
// The script starts the viewer with OCELLI_PORT=0, reads the one URL it
// prints, measures `getBoundingClientRect()` of each canvas in headless
// Chromium through tools/bench's pinned Playwright, and stops the viewer by
// its PID. `bin/ocelli.sh gate viewer-browser` runs it.

import { createRequire } from "node:module";
import { fileURLToPath, pathToFileURL } from "node:url";

import { ensureViewerBinary, startViewer } from "../../../../tools/bench/src/viewer_work.mjs";

export const EXPECTED_ASPECT = 64 / 67.2;
export const ASPECT_TOLERANCE = 1e-3;
const SYNTHETIC_STATUS = "Synthetic study, not a patient. 1 series.";

/** Each reading is `{ name, width, height }` in CSS pixels. */
export function aspectProblems(readings, expected = EXPECTED_ASPECT, tolerance = ASPECT_TOLERANCE) {
  const problems = [];
  if (!Array.isArray(readings) || readings.length === 0) {
    problems.push("no canvas was measured");
    return problems;
  }
  for (const reading of readings) {
    if (!(reading.width > 0) || !(reading.height > 0)) {
      problems.push(`${reading.name} has no displayed size`);
      continue;
    }
    const ratio = reading.width / reading.height;
    if (!(Math.abs(ratio - expected) <= tolerance)) {
      problems.push(`${reading.name} is displayed at ${ratio.toFixed(6)}, not ${expected.toFixed(6)}`);
    }
  }
  return problems;
}

async function loadPlaywright() {
  const require = createRequire(new URL("../../../../tools/bench/package.json", import.meta.url));
  const loaded = await import(pathToFileURL(require.resolve("playwright")).href);
  // The package is CommonJS, so its names may arrive only on `default`.
  return loaded.chromium ? loaded : loaded.default;
}

async function rectOf(page, selector) {
  return page.locator(selector).evaluate((node) => {
    const box = node.getBoundingClientRect();
    return { width: box.width, height: box.height };
  });
}

async function waitDrawn(page, selector) {
  await page.waitForFunction((target) => {
    const node = document.querySelector(target);
    if (!node || node.width <= 1) return false;
    const box = node.getBoundingClientRect();
    return box.width > 0 && box.height > 0;
  }, selector);
}

export async function measureAspects(url, chromium) {
  const browser = await chromium.launch({ headless: true });
  const readings = [];
  try {
    const page = await browser.newPage({ viewport: { width: 1600, height: 1000 } });
    page.setDefaultTimeout(60000);
    await page.goto(url);
    await page.locator("#source-synthetic").click();
    await page.waitForFunction(
      (text) => (document.getElementById("status").textContent || "").startsWith(text),
      SYNTHETIC_STATUS,
    );
    const single = '.cell[data-cell="0"] canvas.view';
    await waitDrawn(page, single);
    readings.push({ name: "the single view at 1 by 1", ...(await rectOf(page, single)) });

    await page.keyboard.press("Shift+G");
    for (let tile = 0; tile < 4; tile += 1) {
      await waitDrawn(page, `.cell[data-cell="0"] canvas[data-tile="${tile}"]`);
    }
    for (let tile = 0; tile < 4; tile += 1) {
      const selector = `.cell[data-cell="0"] canvas[data-tile="${tile}"]`;
      readings.push({ name: `tile ${tile + 1} in tile mode`, ...(await rectOf(page, selector)) });
    }
    await page.keyboard.press("Shift+G");

    await page.locator("#layout").click();
    await page.locator('#layout-picker [data-columns="2"][data-rows="2"]').click();
    const row = page.locator("#series-list button").first();
    for (let cell = 1; cell < 4; cell += 1) {
      await row.dragTo(page.locator(`.cell[data-cell="${cell}"]`));
    }
    for (let cell = 0; cell < 4; cell += 1) {
      const selector = `.cell[data-cell="${cell}"] canvas.view`;
      await waitDrawn(page, selector);
      await page.waitForFunction((target) => {
        const node = document.querySelector(target);
        return node && !node.closest(".stage").hidden;
      }, selector);
    }
    for (let cell = 0; cell < 4; cell += 1) {
      const selector = `.cell[data-cell="${cell}"] canvas.view`;
      readings.push({ name: `cell ${cell + 1} at 2 by 2`, ...(await rectOf(page, selector)) });
    }
    return { readings, chromium: browser.version() };
  } finally {
    await browser.close();
  }
}

function stopByPid(pid) {
  return new Promise((resolve) => {
    try {
      process.kill(pid, "SIGTERM");
    } catch {
      resolve();
      return;
    }
    const started = Date.now();
    const poll = () => {
      try {
        process.kill(pid, 0);
      } catch {
        resolve();
        return;
      }
      if (Date.now() - started > 3000) {
        try { process.kill(pid, "SIGKILL"); } catch { /* already gone */ }
        resolve();
        return;
      }
      setTimeout(poll, 50);
    };
    poll();
  });
}

async function main() {
  const { chromium } = await loadPlaywright();
  const binary = await ensureViewerBinary("release", true);
  const server = await startViewer(binary);
  let result;
  try {
    result = await measureAspects(server.url, chromium);
  } finally {
    await stopByPid(server.pid);
  }
  console.log(`viewer ${server.url} pid ${server.pid}, Chromium ${result.chromium}`);
  for (const reading of result.readings) {
    const ratio = reading.width / reading.height;
    console.log(`${reading.name}: ${reading.width.toFixed(3)} by ${reading.height.toFixed(3)}, ratio ${ratio.toFixed(6)}`);
  }
  const problems = aspectProblems(result.readings);
  if (problems.length) {
    for (const problem of problems) console.error(problem);
    process.exitCode = 1;
    return;
  }
  console.log(`all ${result.readings.length} canvases are within ${ASPECT_TOLERANCE} of ${EXPECTED_ASPECT.toFixed(6)}`);
}

if (process.argv[1] === fileURLToPath(import.meta.url)) {
  main().catch((error) => {
    console.error(error instanceof Error ? error.message : String(error));
    process.exitCode = 1;
  });
}
