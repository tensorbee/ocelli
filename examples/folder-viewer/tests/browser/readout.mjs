// F-208. The pointer readout, the cross and the magnifier are placed for one
// view geometry. With the pointer still, each change below must leave them
// hidden, and the next pointer move must place them again.
//
//   layout   1 by 1 to 2 by 1 through the keyboard picker (G, Enter)
//   resize   the window from 1600 by 1000 to 1200 by 800
//   zoom     the `=` key
//   tiles    Shift G on, then Shift G off
//
// Two drags must keep the readout and the magnifier showing on most moves,
// because each move places them again after the picture moves:
//
//   pan      a right-button drag
//   zoom     a middle-button drag
//
// And with the pointer still, a new image in the cell must reach the
// magnifier, which must then hold the same pixels a fresh draw at that
// point gives, and not the previous image's:
//
//   step     ArrowRight
//   play     Space, about 1 s of Play, Space
//
// Two more cases, each on a fresh page:
//
//   empty    in Four images with the pointer on tile 4, the scrubber moves
//            to image 10, so tile 4 has no image. The magnifier must not
//            keep image 4's pixels.
//   throw    a pan drag whose last layout throws. Afterwards `=` with the
//            pointer still must still hide the readout and the magnifier.
//   fault    the main preview fails after ArrowRight, so the cell shows a
//            fault. A pointer move over it must show no cross, readout or
//            magnifier and send no POST /value.
//   gap      in Four images, a pointer move over the strip between tile 1's
//            picture and tile 2's must show no magnifier.
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

/** Each drag is `{ name, moves, readShown, magShown, posts }`. */
export function dragProblems(drags) {
  const problems = [];
  if (!Array.isArray(drags) || drags.length === 0) {
    problems.push("no drag was observed");
    return problems;
  }
  for (const drag of drags) {
    if (!(drag.moves > 0)) {
      problems.push(`${drag.name}: no move was made`);
      continue;
    }
    if (!(drag.readShown * 2 > drag.moves)) {
      problems.push(`${drag.name}: the readout showed on ${drag.readShown} of ${drag.moves} moves`);
    }
    if (!(drag.magShown * 2 > drag.moves)) {
      problems.push(`${drag.name}: the magnifier showed on ${drag.magShown} of ${drag.moves} moves`);
    }
  }
  return problems;
}

/** Each step is `{ name, before, after, fresh, magHidden }`, magnifier pixel hashes. */
export function stepProblems(steps) {
  const problems = [];
  if (!Array.isArray(steps) || steps.length === 0) {
    problems.push("no image step was observed");
    return problems;
  }
  for (const step of steps) {
    if (step.before === step.fresh) {
      problems.push(`${step.name}: the two images do not differ at the pointer, so the case shows nothing`);
      continue;
    }
    if (step.magHidden) continue;
    if (step.after !== step.fresh) {
      problems.push(`${step.name}: the magnifier kept pixels that are not the image now shown`);
    }
  }
  return problems;
}

/**
 * `empty` is `{ name, before, after, magHidden }`, magnifier pixel hashes.
 * `thrown` is `{ name, threw, before, after }`, readout states.
 */
/**
 * `faulted` is `{ name, fault, readHidden, crossHidden, magHidden, posts }`.
 * `gap` is `{ name, inGap, magHidden }`.
 */
export function noPictureProblems(faulted, gap) {
  const problems = [];
  if (!faulted || !gap) {
    problems.push("the faulted-cell or the tile-gap case was not observed");
    return problems;
  }
  if (!faulted.fault) {
    problems.push(`${faulted.name}: the cell did not show a fault, so the case shows nothing`);
  } else {
    if (!faulted.readHidden || !faulted.crossHidden || !faulted.magHidden) {
      problems.push(`${faulted.name}: a pointer over a fault showed the cross, readout or magnifier`);
    }
    if (faulted.posts !== 0) {
      problems.push(`${faulted.name}: a pointer over a fault sent ${faulted.posts} POST /value`);
    }
  }
  if (!gap.inGap) {
    problems.push(`${gap.name}: the pointer was not between the tiles' pictures, so the case shows nothing`);
  } else if (!gap.magHidden) {
    problems.push(`${gap.name}: the magnifier showed over the strip between tiles`);
  }
  return problems;
}

export function faultProblems(empty, thrown) {
  const problems = [];
  if (!empty || !thrown) {
    problems.push("the empty-tile or the thrown-layout case was not observed");
    return problems;
  }
  if (!empty.magHidden && empty.after === empty.before) {
    problems.push(`${empty.name}: the magnifier kept the pixels of an image the tile no longer shows`);
  }
  if (!thrown.threw) {
    problems.push(`${thrown.name}: the injected throw did not happen, so the case shows nothing`);
  } else if (thrown.before.readHidden || thrown.before.magHidden) {
    problems.push(`${thrown.name}: nothing was shown before the change`);
  } else if (!thrown.after.readHidden || !thrown.after.magHidden) {
    problems.push(`${thrown.name}: after a throw inside a drag, a zoom left the readout or magnifier showing`);
  }
  return problems;
}

async function openSynthetic(page, url) {
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
}

async function emptyTileCase(page, url) {
  await openSynthetic(page, url);
  await page.keyboard.press("m");
  await page.keyboard.press("Shift+G");
  const tile = page.locator('.cell[data-cell="0"] canvas[data-tile="3"]');
  await page.waitForFunction(() => {
    const node = document.querySelector('.cell[data-cell="0"] canvas[data-tile="3"]');
    return node && node.width > 1;
  });
  const box = await tile.boundingBox();
  const point = { x: box.x + box.width / 2, y: box.y + box.height / 2 };
  await page.mouse.move(point.x + 3, point.y);
  await page.mouse.move(point.x, point.y);
  await page.waitForFunction(() => !document.getElementById("mag").hidden);
  await page.waitForTimeout(300);
  const before = await magHash(page);
  await page.evaluate(() => {
    const scrub = document.getElementById("scrub");
    scrub.value = "9";
    scrub.dispatchEvent(new Event("input", { bubbles: true }));
  });
  await page.waitForTimeout(1200);
  const after = await magHash(page);
  const magHidden = await page.evaluate(() => document.getElementById("mag").hidden);
  return { name: "tile 4 empties in Four images", before, after, magHidden };
}

async function thrownLayoutCase(page, url) {
  await openSynthetic(page, url);
  await page.keyboard.press("c");
  await page.keyboard.press("m");
  const start = await centreOf(page, 0);
  await pointAt(page, start, ["readout", "magnifier"]);
  await page.evaluate(() => {
    const original = paintSelection;
    window.__threw = false;
    window.paintSelection = function throwing(...args) {
      if (window.__throwOnce) {
        window.__throwOnce = false;
        window.__threw = true;
        throw new Error("injected by readout.mjs");
      }
      return original.apply(this, args);
    };
  });
  await page.mouse.down({ button: "right" });
  for (let step = 1; step <= 4; step += 1) {
    if (step === 4) await page.evaluate(() => { window.__throwOnce = true; });
    await page.mouse.move(start.x + step * 4, start.y + step * 3);
    await page.waitForTimeout(60);
  }
  await page.mouse.up({ button: "right" });
  const threw = await page.evaluate(() => window.__threw === true);
  await page.mouse.move(start.x + 20, start.y + 14);
  await page.waitForTimeout(500);
  const before = await state(page);
  await page.keyboard.press("=");
  await page.waitForTimeout(400);
  const after = await state(page);
  return { name: "a zoom after a throw inside a pan drag", threw, before, after };
}

async function faultedCellCase(page, url) {
  await openSynthetic(page, url);
  await page.keyboard.press("c");
  await page.keyboard.press("m");
  const start = await centreOf(page, 0);
  await pointAt(page, start, ["readout", "magnifier"]);
  await page.evaluate(() => {
    window.requestGrey = () => Promise.reject(new Error("injected main failure"));
    dropCache();
  });
  await page.keyboard.press("ArrowRight");
  await page.waitForFunction(() => {
    const fault = document.querySelector('.cell[data-cell="0"] .fault');
    return fault && !fault.hidden;
  });
  let posts = 0;
  const count = (request) => { if (request.url().endsWith("/value")) posts += 1; };
  page.on("request", count);
  await page.mouse.move(start.x + 30, start.y + 20);
  await page.mouse.move(start.x, start.y);
  await page.mouse.move(start.x + 1, start.y);
  await page.waitForTimeout(600);
  page.off("request", count);
  const seen = await state(page);
  const fault = await page.evaluate(() => {
    const node = document.querySelector('.cell[data-cell="0"] .fault');
    return node && !node.hidden ? node.textContent : "";
  });
  return {
    name: "a pointer move over a faulted cell",
    fault,
    readHidden: seen.readHidden,
    crossHidden: seen.crossHidden,
    magHidden: seen.magHidden,
    posts,
  };
}

async function tileGapCase(page, url) {
  await openSynthetic(page, url);
  await page.keyboard.press("m");
  await page.keyboard.press("Shift+G");
  await page.waitForFunction(() => {
    const node = document.querySelector('.cell[data-cell="0"] canvas[data-tile="0"]');
    return node && node.width > 1;
  });
  const spot = await page.evaluate(() => {
    const tile = document.querySelector('.cell[data-cell="0"] .tile');
    const picture = tile.querySelector("canvas").getBoundingClientRect();
    const box = tile.getBoundingClientRect();
    return { x: (picture.right + box.right) / 2, y: picture.top + picture.height / 2, room: box.right - picture.right };
  });
  await page.mouse.move(spot.x - 3, spot.y);
  await page.mouse.move(spot.x, spot.y);
  await page.waitForTimeout(400);
  const inGap = spot.room > 4 && await page.evaluate(({ x, y }) => {
    const node = document.elementFromPoint(x, y);
    return Boolean(node && node.classList.contains("tile"));
  }, spot);
  const magHidden = await page.evaluate(() => document.getElementById("mag").hidden);
  return { name: "a pointer move over the strip between tiles", inGap, magHidden };
}

function magHash(page) {
  return page.evaluate(() => {
    const node = document.getElementById("mag");
    const data = node.getContext("2d").getImageData(0, 0, node.width, node.height).data;
    let hash = 2166136261;
    for (let index = 0; index < data.length; index += 1) {
      hash ^= data[index];
      hash = Math.imul(hash, 16777619) >>> 0;
    }
    return hash;
  });
}

async function dragCase(page, name, button) {
  const start = await centreOf(page, 0);
  await pointAt(page, start, ["readout", "magnifier"]);
  let posts = 0;
  const count = (request) => { if (request.url().endsWith("/value")) posts += 1; };
  page.on("request", count);
  await page.mouse.down({ button });
  let readShown = 0;
  let magShown = 0;
  const moves = 20;
  for (let step = 1; step <= moves; step += 1) {
    await page.mouse.move(start.x + step * 4, start.y + step * 3);
    await page.waitForTimeout(40);
    const now = await state(page);
    if (!now.readHidden) readShown += 1;
    if (!now.magHidden) magShown += 1;
  }
  await page.mouse.up({ button });
  page.off("request", count);
  await page.keyboard.press("0");
  await page.waitForTimeout(300);
  return { name, moves, readShown, magShown, posts };
}

// The synthetic slices differ from one to the next at the edge of the
// outer ring, which on row 50 is near column 105 and moves out by one
// column per slice. The magnifier there spans about a dozen image columns.
const RING_EDGE = { x: 104.5, y: 49.5 };

async function stepCase(page, name, change) {
  const point = await page.evaluate((at) => imageToClient(activeViewport(), at), RING_EDGE);
  await pointAt(page, point, ["magnifier"]);
  await page.waitForTimeout(300);
  const before = await magHash(page);
  await change();
  await page.waitForTimeout(600);
  const after = await magHash(page);
  const magHidden = await page.evaluate(() => document.getElementById("mag").hidden);
  await page.mouse.move(point.x + 5, point.y + 1);
  await page.waitForTimeout(100);
  await page.mouse.move(point.x + 1, point.y);
  await page.waitForTimeout(400);
  const fresh = await magHash(page);
  return { name, before, after, fresh, magHidden };
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

// The centre of a cell's top-left quarter. In Four images that is the
// middle of tile 1's picture, and in the single view it is on the image.
async function quarterOf(page, cell) {
  const box = await page.locator(`.cell[data-cell="${cell}"]`).boundingBox();
  return { x: box.x + box.width / 4, y: box.y + box.height / 4 };
}

async function observe(page, name, shown, again, change, where = centreOf) {
  const cell = 0;
  const point = await where(page, cell);
  try {
    await pointAt(page, point, shown);
  } catch (error) {
    throw new Error(`${name}: the readout was not shown before the change, ${error instanceof Error ? error.message : String(error)}`);
  }
  const before = await state(page);
  await change();
  await page.waitForTimeout(400);
  const after = await state(page);
  const next = await where(page, cell);
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
  const drags = [];
  const steps = [];
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
    observations.push(await observe(page, "tile mode on", both, mag, () => page.keyboard.press("Shift+G"), quarterOf));
    observations.push(await observe(page, "tile mode off", mag, both, () => page.keyboard.press("Shift+G"), quarterOf));
    drags.push(await dragCase(page, "pan by right-button drag", "right"));
    drags.push(await dragCase(page, "zoom by middle-button drag", "middle"));
    steps.push(await stepCase(page, "ArrowRight", () => page.keyboard.press("ArrowRight")));
    steps.push(await stepCase(page, "Play", async () => {
      await page.keyboard.press(" ");
      await page.waitForTimeout(1000);
      await page.keyboard.press(" ");
    }));
    observations.push(await observe(page, "layout 1 by 1 to 2 by 1", both, both, async () => {
      await page.keyboard.press("g");
      await page.locator('#layout-picker [data-columns="2"][data-rows="1"]').focus();
      await page.keyboard.press("Enter");
    }));
    const fresh = await browser.newPage({ viewport: { width: 1600, height: 1000 } });
    fresh.setDefaultTimeout(30000);
    const empty = await emptyTileCase(fresh, url);
    const thrownPage = await browser.newPage({ viewport: { width: 1600, height: 1000 } });
    thrownPage.setDefaultTimeout(30000);
    const thrown = await thrownLayoutCase(thrownPage, url);
    const faultPage = await browser.newPage({ viewport: { width: 1600, height: 1000 } });
    faultPage.setDefaultTimeout(30000);
    const faulted = await faultedCellCase(faultPage, url);
    const gapPage = await browser.newPage({ viewport: { width: 1600, height: 1000 } });
    gapPage.setDefaultTimeout(30000);
    const gap = await tileGapCase(gapPage, url);
    return { observations, drags, steps, empty, thrown, faulted, gap, chromium: browser.version() };
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
  for (const drag of result.drags) {
    console.log(`${drag.name}: readout on ${drag.readShown} of ${drag.moves} moves, magnifier on ${drag.magShown}, ${drag.posts} POST /value`);
  }
  for (const step of result.steps) {
    console.log(`${step.name}: magnifier ${step.before} before, ${step.after} after, ${step.fresh} fresh${step.magHidden ? ", hidden" : ""}`);
  }
  console.log(`${result.empty.name}: magnifier ${result.empty.before} before, ${result.empty.after} after${result.empty.magHidden ? ", hidden" : ", shown"}`);
  console.log(`${result.thrown.name}: threw ${result.thrown.threw}, before ${JSON.stringify(result.thrown.before.read)}, after hidden ${result.thrown.after.readHidden}/${result.thrown.after.magHidden}`);
  const problems = readoutProblems(result.observations)
    .concat(dragProblems(result.drags), stepProblems(result.steps), faultProblems(result.empty, result.thrown),
      noPictureProblems(result.faulted, result.gap));
  console.log(`${result.faulted.name}: fault ${JSON.stringify(result.faulted.fault)}, hidden readout ${result.faulted.readHidden} cross ${result.faulted.crossHidden} magnifier ${result.faulted.magHidden}, ${result.faulted.posts} POST /value`);
  console.log(`${result.gap.name}: in the gap ${result.gap.inGap}, magnifier ${result.gap.magHidden ? "hidden" : "shown"}`);
  if (problems.length) {
    for (const problem of problems) console.error(problem);
    process.exitCode = 1;
    return;
  }
  console.log(`all ${result.observations.length} changes hid the readout and a move showed it again, ${result.drags.length} drags kept it, ${result.steps.length} image steps reached the magnifier, an emptied tile and a thrown layout left nothing stale, a faulted cell and a tile gap showed nothing`);
}

if (process.argv[1] === fileURLToPath(import.meta.url)) {
  main().catch((error) => {
    console.error(error instanceof Error ? error.message : String(error));
    process.exitCode = 1;
  });
}
