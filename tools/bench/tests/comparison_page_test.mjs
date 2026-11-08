// Opted-in real-browser control of the comparison page's presentation state.
// The ordinary bench gate registers this suite and reports the explicit skip.
import assert from "node:assert/strict";
import test from "node:test";
import { Buffer } from "node:buffer";
import { createRequire } from "node:module";
import { mkdtemp, mkdir, writeFile, readFile, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { mono16Part10 } from "../src/viewer_work.mjs";
import { launchSide, openFolder, scroll, prepareInputs, windowStep, pixelPrecheck, measureAll, runComparison } from "../src/comparison_run.mjs";
import { repoPath } from "../src/paths.mjs";
import { buildPage } from "../../oracle/build-page.mjs";
import { serveDirectory } from "../../oracle/src/server.mjs";

/** These assertions use the visible canvas, independently of the page's flag. */
export async function assertCommittedPageState(actor) {
  const seen = await actor.page.evaluate(() => {
    const canvas = document.querySelector("#view canvas");
    const before = Array.from(canvas.getContext("2d").getImageData(0, 0, canvas.width, canvas.height).data);
    const old = window.__bench.snapshot();
    document.dispatchEvent(new KeyboardEvent("keydown", { key: "ArrowDown", bubbles: true }));
    const pending = window.__bench.snapshot();
    const after = canvas.getContext("2d").getImageData(0, 0, canvas.width, canvas.height).data;
    return { old, pending, samePixels: before.every((value, at) => value === after[at]),
      canvasVisible: canvas.width > 0 && canvas.height > 0 && window.getComputedStyle(canvas).visibility === "visible",
      beforeEdge: before.slice((Math.floor(canvas.height / 2) * canvas.width + Math.floor(canvas.width / 4)) * 4,
        (Math.floor(canvas.height / 2) * canvas.width + Math.floor(canvas.width / 4)) * 4 + 64) };
  });
  assert.equal(seen.samePixels, true);
  assert.equal(seen.canvasVisible, true);
  assert.equal(seen.old.commitSequence, seen.pending.commitSequence);
  assert.equal(seen.pending.blank, false, "pending work retains visible pixels");
  assert.equal(seen.pending.index, seen.old.index, "presented index waits for a real commit");
  await actor.page.waitForFunction((old) => {
    const now = window.__bench.snapshot();
    return now.commitSequence > old.commitSequence && now.index === 1 && !now.blank;
  }, seen.old, { timeout: 30000 });
  const displayed = await actor.page.evaluate(() => {
    const canvas = document.querySelector("#view canvas");
    const edge = Array.from(canvas.getContext("2d").getImageData(Math.floor(canvas.width / 4), Math.floor(canvas.height / 2), 16, 1).data);
    canvas.hidden = true;
    const hidden = window.__bench.snapshot();
    canvas.hidden = false;
    return { edge, hidden, visible: window.__bench.snapshot() };
  });
  assert.notDeepEqual(displayed.edge, seen.beforeEdge, "the opposite ramp really reached the canvas");
  assert.equal(displayed.hidden.blank, true, "an actually hidden canvas is blank");
  assert.equal(displayed.visible.blank, false);
}

test("actual presented frames stay distinct from pending work and hidden stages", {
  skip: process.env.OCELLI_COMPARISON_BROWSER !== "1",
}, async () => {
  const temp = await mkdtemp(join(tmpdir(), "comparison-page-contract-"));
  const context = { actors: new Set(), headed: false, binary: repoPath("target", "release", "ocelli-folder-viewer") };
  let server;
  try {
    const directory = join(temp, "ct"); await mkdir(directory);
    for (const index of [0, 1]) {
      const pixels = Buffer.alloc(512 * 512 * 2);
      for (let at = 0; at < 512 * 512; at += 1) pixels.writeUInt16LE(index ? 511 - at % 512 : at % 512, at * 2);
      await writeFile(join(directory, `${index}.dcm`), mono16Part10({ rows: 512, columns: 512, pixels,
        instanceNumber: String(index + 1), sopClass: "1.2.840.10008.5.1.4.1.1.2",
        sopUid: `2.25.223301${index}`, seriesUid: "2.25.223300", modality: "CT" }));
    }
    const dist = join(temp, "page"); await buildPage({ dist });
    server = await serveDirectory(dist); context.origin = server.origin;
    const require = createRequire(new URL("../../oracle/package.json", import.meta.url));
    context.chromium = require("playwright").chromium;
    const comparator = await launchSide("cornerstone", context);
    await openFolder(comparator, directory);
    await assertCommittedPageState(comparator);
    await comparator.close();
    const native = await launchSide("ocelli", context);
    await openFolder(native, directory);
    await native.page.evaluate(() => { document.querySelector(".cell.active .stage").hidden = true; });
    const delivery = await scroll(native);
    assert(delivery.blank > 0, "the actual hidden native stage is sampled");
    await native.close();
  } finally {
    await Promise.allSettled([...context.actors].map(actor => actor.close()));
    if (server) await server.close();
    await rm(temp, { recursive: true, force: true });
  }
});


// Exercise the actual prepared subset, rather than the regular-file source
// directory used by the presentation-state control above.
test("prepared CT subset reaches both real folder consumers with the declared pixels", {
  skip: process.env.OCELLI_COMPARISON_BROWSER !== "1",
}, async () => {
  const temp = await mkdtemp(join(tmpdir(), "comparison-prepared-browser-"));
  const context = { actors: new Set(), headed: false, binary: repoPath("target", "release", "ocelli-folder-viewer"),
    comparator: repoPath("target", "release", "ocelli-compare") };
  let server;
  try {
    const input = join(temp, "input"), prepared = join(temp, "prepared");
    await mkdir(input); await mkdir(prepared);
    const pixels = Buffer.alloc(512 * 512 * 2);
    for (let at = 0; at < 512 * 512; at += 1) pixels.writeUInt16LE(at % 512, at * 2);
    await writeFile(join(input, "own-ramp.dcm"), mono16Part10({ rows: 512, columns: 512, pixels,
      instanceNumber: "1", sopClass: "1.2.840.10008.5.1.4.1.1.2",
      sopUid: "2.25.223501", seriesUid: "2.25.223500", modality: "CT" }));
    const require = createRequire(new URL("../../oracle/package.json", import.meta.url));
    const inputs = await prepareInputs(input, prepared, require("dicom-parser"), false);
    assert.equal(inputs.selections["pseudo-ct"].count, 1);
    const dist = join(temp, "page"); await buildPage({ dist });
    server = await serveDirectory(dist); context.origin = server.origin;
    context.chromium = require("playwright").chromium;
    for (const side of ["ocelli", "cornerstone"]) {
      const actor = await launchSide(side, context);
      await actor.page.evaluate(() => {
        window.__preparedUploadCount = null;
        document.addEventListener("change", (event) => {
          if (event.target?.id === "folder") window.__preparedUploadCount = event.target.files.length;
        }, true);
      });
      const opening = openFolder(actor, inputs.directories["pseudo-ct"]);
      opening.catch(() => {}); // The browser closes in finally if the upload assertion fails.
      await actor.page.waitForFunction(() => window.__preparedUploadCount !== null);
      assert.equal(await actor.page.evaluate(() => window.__preparedUploadCount), 1, `${side} must receive the selected file`);
      await opening;
      if (side === "ocelli") await windowStep(actor, 40, 400);
      const observed = await actor.page.evaluate(async (side) => {
        if (side === "cornerstone") await window.__bench.capture({ center: 40, width: 400, function: "LINEAR" });
        const canvas = document.querySelector(side === "ocelli" ? ".cell.active canvas.view" : "#view canvas");
        const image = canvas.getContext("2d").getImageData(0, 0, canvas.width, canvas.height);
        const samples = [0, 128, 511].map((x) => Array.from(image.data.slice((256 * canvas.width + x) * 4, (256 * canvas.width + x) * 4 + 4)));
        return { width: canvas.width, height: canvas.height, samples };
      }, side);
      assert.equal(observed.width, 512); assert.equal(observed.height, 512);
      // PS3.3 C.11.2.1.2.2, LINEAR c40/w400. Inputs0 and128 give
      // 40800/399 and73440/399, rounding to102 and184. Input511 clamps255.
      assert.deepEqual(observed.samples, [[102, 102, 102, 255], [184, 184, 184, 255], [255, 255, 255, 255]]);
      await actor.close();
    }
    // This is the real precheck/capture caller and release comparator. Manual
    // page captures above alone would miss a broken controller callback.
    let checked = await pixelPrecheck(context, inputs, temp);
    if (checked.needsHeaded) {
      context.headed = true;
      checked = await pixelPrecheck(context, inputs, temp);
    }
    assert.ok(checked.verdict, "the actual precheck must produce a comparator verdict");
    assert.equal(checked.verdict.passes, true);
    assert.equal(checked.verdict.low_information, false);
  } finally {
    await Promise.allSettled([...context.actors].map(actor => actor.close()));
    if (server) await server.close();
    await rm(temp, { recursive: true, force: true });
  }
});


test("actual warm orchestration retains one page and recovers after a trial input error", {
  skip: process.env.OCELLI_COMPARISON_BROWSER !== "1",
}, async () => {
  const temp = await mkdtemp(join(tmpdir(), "comparison-warm-owner-"));
  const context = { actors: new Set(), headed: false, binary: repoPath("target", "release", "ocelli-folder-viewer") };
  const observed = [];
  let server;
  try {
    const input = join(temp, "input"), prepared = join(temp, "prepared");
    await mkdir(input); await mkdir(prepared);
    const pixels = Buffer.alloc(512 * 512 * 2);
    for (let at = 0; at < 512 * 512; at += 1) pixels.writeUInt16LE(at % 512, at * 2);
    await writeFile(join(input, "own-ramp.dcm"), mono16Part10({ rows: 512, columns: 512, pixels,
      instanceNumber: "1", sopClass: "1.2.840.10008.5.1.4.1.1.2",
      sopUid: "2.25.223601", seriesUid: "2.25.223600", modality: "CT" }));
    const require = createRequire(new URL("../../oracle/package.json", import.meta.url));
    const inputs = await prepareInputs(input, prepared, require("dicom-parser"), false);
    const selected = inputs.directories["pseudo-ct"];
    const dist = join(temp, "page"); await buildPage({ dist });
    server = await serveDirectory(dist); context.origin = server.origin;
    const chromium = require("playwright").chromium;
    context.chromium = { async launch(options) {
      const browser = await chromium.launch(options);
      const state = { browser, before: [], after: [], pages: [] };
      const actorIndex = observed.length;
      observed.push(state);
      const newPage = browser.newPage.bind(browser);
      browser.newPage = async (...args) => {
        state.before.push(browser.contexts().length);
        const page = await newPage(...args);
        state.pages.push(page); state.after.push(browser.contexts().length);
        // A real missing-directory upload on the first actor's second
        // measured trial. Following trials restore the exact selected input.
        inputs.directories["pseudo-ct"] = actorIndex === 0 && state.pages.length === 3
          ? join(temp, "missing-input") : selected;
        return page;
      };
      return browser;
    } };
    // Three trials per side are a lifecycle fixture, not a change to the
    // production comparison.json counts or a performance observation.
    const definition = { metrics: [{ id: "open", n: 3, workloads: ["pseudo-ct"],
      cold: { state: "no", reason: "warm ownership fixture" }, warm: { state: "yes" } }] };
    const result = await measureAll(context, inputs, definition);
    const trials = result.raw.find((row) => row.metric === "open" && row.condition === "warm").trials;
    assert.equal(trials.filter((trial) => trial.error).length, 1);
    assert.equal(trials.find((trial) => trial.error).side, "ocelli");
    assert.equal(result.metrics.open["pseudo-ct/warm"].ocelli.state, "invalid");
    assert.equal(result.metrics.open["pseudo-ct/warm"].cornerstone.state, "measured");
    assert.equal(context.actors.size, 0);
    assert.equal(observed.length, 2);
    for (const state of observed) {
      assert.equal(state.pages.length, 4, "one priming page and three actual trial pages");
      assert(state.before.every((count) => count === 0), "the prior page and its owned context must close before creating the next");
      assert(state.after.every((count) => count === 1));
      assert(state.pages.every((page) => page.isClosed()), "including the page whose trial failed");
      assert.equal(state.browser.isConnected(), false);
    }
  } finally {
    await Promise.allSettled([...context.actors].map(actor => actor.close()));
    if (server) await server.close();
    await rm(temp, { recursive: true, force: true });
  }
});


// Drive the real metadata provider through pixelPrecheck and the controller.
// Only the public version string of one real browser is deliberately changed.
test("actual precheck environment mismatch retains a private refusal after closing both browsers", {
  skip: process.env.OCELLI_COMPARISON_BROWSER !== "1",
}, async () => {
  const temp = await mkdtemp(join(tmpdir(), "comparison-environment-browser-"));
  const observed = []; let server, trials = 0, infrastructureClosed = false;
  try {
    const input = join(temp, "input"); await mkdir(input);
    const pixels = Buffer.alloc(512 * 512 * 2);
    for (let at = 0; at < 512 * 512; at += 1) pixels.writeUInt16LE(at % 512, at * 2);
    await writeFile(join(input, "own-ramp.dcm"), mono16Part10({ rows: 512, columns: 512, pixels,
      instanceNumber: "1", sopClass: "1.2.840.10008.5.1.4.1.1.2",
      sopUid: "2.25.223701", seriesUid: "2.25.223700", modality: "CT" }));
    const dist = join(temp, "page"); await buildPage({ dist });
    server = await serveDirectory(dist);
    const require = createRequire(new URL("../../oracle/package.json", import.meta.url));
    const chromium = require("playwright").chromium;
    const versions = [];
    const wrapped = { async launch(options) {
      const browser = await chromium.launch(options);
      const reported = observed.length === 0 ? browser.version() : "controlled-mismatched-browser";
      versions.push(reported); browser.version = () => reported;
      observed.push(browser); return browser;
    } };
    const out = join(temp, "out");
    await assert.rejects(runComparison({ pseudo: input, out, build: false }, {
      prepare: async (directory, prepared) => prepareInputs(directory, prepared, require("dicom-parser"), false),
      exclusive: async () => {},
      build: async () => ({ actors: new Set(), chromium: wrapped, origin: server.origin,
        binary: repoPath("target", "release", "ocelli-folder-viewer"),
        comparator: repoPath("target", "release", "ocelli-compare"),
        close: async () => { await server.close(); infrastructureClosed = true; } }),
      measure: async () => { trials += 1; throw new Error("environment refusal must not reach measurement"); },
    }), (error) => { assert.equal(error.exitCode, 2); assert(infrastructureClosed); return true; });
    assert.equal(trials, 0); assert.equal(observed.length, 2);
    assert(observed.every((browser) => !browser.isConnected()));
    const record = JSON.parse(await readFile(join(out, "comparison.json"), "utf8"));
    assert.equal(record.state, "refused_environment"); assert.deepEqual(record.metrics, {});
    assert.equal(Object.hasOwn(record, "pair_verdict"), false);
    assert.equal(record.versions["Ocelli Chromium"], versions[0]);
    assert.equal(record.versions["cornerstone3D Chromium"], versions[1]);
    assert(record.versions["Ocelli renderer"].length > 0);
    assert(record.versions["cornerstone3D renderer"].length > 0);
    const text = await readFile(join(out, "comparison.txt"), "utf8");
    for (const value of Object.values(record.versions)) assert(text.includes(value));
    await assert.rejects(readFile(join(out, "lock")), { code: "ENOENT" });
  } finally {
    await Promise.allSettled(observed.map((browser) => browser.close()));
    if (server) await server.close();
    await rm(temp, { recursive: true, force: true });
  }
});
