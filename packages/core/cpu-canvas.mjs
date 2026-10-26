/**
 * F-X003 worker publication. A Chromium module worker owns an
 * OffscreenCanvas, commits one f32 stored sample 40, and publishes it.
 * A second worker commits interleaved RGB 10, 20, 30, 40, 50, 60.
 * A mode worker rejects mono-to-RGB and RGB-to-mono requests from settled
 * and invalidated states, then draws a valid interaction and settles.
 *
 * Identity LINEAR centre 40 width 1 maps stored 40 to display 255, so the
 * settled pixel is opaque white. SetInvert of that frame is opaque black.
 * This file does not alias CanvasRenderingContext2D. It serves the dev
 * boundary module and is not a release-size measurement, a GPU observation,
 * or a representative-host run.
 *
 * Build first:
 *
 *     wasm-pack build crates/ocelli-wasm --dev --target web \
 *       --out-dir target/boundary-wasm
 *     node packages/core/cpu-canvas.mjs
 */

import { createServer } from "node:http";
import { readFile } from "node:fs/promises";
import { createRequire } from "node:module";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import assert from "node:assert/strict";

const require = createRequire(import.meta.url);
const { chromium } = require("../../tools/oracle/node_modules/playwright");

const HERE = dirname(fileURLToPath(import.meta.url));
const ROOT = join(HERE, "..", "..");
const PKG = join(ROOT, "crates", "ocelli-wasm", "target", "boundary-wasm");

const HARNESS = `<!DOCTYPE html>
<meta charset="utf-8">
<title>ocelli cpu canvas</title>
`;

const WORKER = `
let api = null;
let memory = null;
let canvas = null;
let puts = 0;
let failNext = false;
let patched = false;

async function load() {
  if (api) {
    return;
  }
  const glue = await import("./ocelli_wasm.js");
  await glue.default();
  api = glue;
  memory = glue.__wasm.memory;
}

function sample40() {
  const bytes = new Uint8Array(4);
  new DataView(bytes.buffer).setFloat32(0, 40, true);
  return bytes;
}

function commitSample() {
  const bytes = sample40();
  const ptr = api.alloc(4);
  const ticket = api.bulk_ticket();
  if (ptr === 0 || ticket === 0) {
    throw new Error("sample allocation was refused");
  }
  new Uint8Array(memory.buffer, ptr, 4).set(bytes);
  if (api.commit_frame(ticket, 4, 1, 1, 1, 1) !== 1) {
    throw new Error("sample commit was refused");
  }
}

function setInvert(enabled) {
  const packed = new Uint8Array([9, 0, 1, 0, 1, 0, 0, 0, enabled ? 1 : 0]);
  const ptr = api.alloc(packed.byteLength);
  const ticket = api.bulk_ticket();
  if (ptr === 0 || ticket === 0) {
    throw new Error("command allocation was refused");
  }
  new Uint8Array(memory.buffer, ptr, packed.byteLength).set(packed);
  if (api.apply_commands(ticket, packed.byteLength) !== 1) {
    throw new Error("invert command was refused");
  }
}

function readPixels() {
  const context = canvas.getContext("2d");
  const image = context.getImageData(0, 0, canvas.width, canvas.height);
  return Array.from(image.data);
}

function u32(bytes, offset) {
  return new DataView(bytes.buffer, bytes.byteOffset, bytes.byteLength).getUint32(offset, true);
}

function drain() {
  const bytes = api.drain_events();
  const count = bytes.byteLength >= 16 ? u32(bytes, 0) : 0;
  const events = [];
  for (let index = 0; index < count; index += 1) {
    const at = 16 + index * 48;
    events.push({
      kind: u32(bytes, at),
      generation: u32(bytes, at + 16),
      width: u32(bytes, at + 20),
      height: u32(bytes, at + 24),
      flags: u32(bytes, at + 28),
    });
  }
  return events;
}

function stateFlags() {
  const state = api.copy_state();
  if (state.byteLength !== 96) {
    throw new Error("state copy was " + state.byteLength);
  }
  return u32(state, 80);
}

function publish() {
  let error = "";
  try {
    api.publishCpuCanvas();
  } catch (caught) {
    error = String(caught);
  }
  return {
    error,
    pixels: readPixels(),
    events: drain(),
    flags: stateFlags(),
    puts,
  };
}

function resize(cssWidth, cssHeight, dpr) {
  let error = "";
  try {
    api.resizeCpuCanvas(cssWidth, cssHeight, dpr);
  } catch (caught) {
    error = String(caught);
  }
  return {
    error,
    width: canvas.width,
    height: canvas.height,
    pixels: canvas.width > 0 && canvas.height > 0 ? readPixels() : [],
    events: drain(),
    flags: stateFlags(),
  };
}

function patchBlit() {
  if (patched) {
    return;
  }
  const prototype = globalThis.OffscreenCanvasRenderingContext2D.prototype;
  const original = prototype.putImageData;
  prototype.putImageData = function patchedPut(...args) {
    puts += 1;
    if (failNext) {
      failNext = false;
      throw new Error("injected blit failure");
    }
    return original.apply(this, args);
  };
  patched = true;
}

self.onmessage = async () => {
  const report = { htmlContext: typeof CanvasRenderingContext2D };
  try {
    await load();
    canvas = new OffscreenCanvas(1, 1);
    api.bindCpuCanvas(canvas, 1, 1, 1);
    commitSample();
    report.first = publish();
    report.sameResize = resize(1, 1, 1);
    report.samePublish = publish();
    report.rejectedResize = resize(0, 1, 1);
    report.rejectedPublish = publish();
    report.idle = publish();
    setInvert(true);
    report.inverted = publish();
    patchBlit();
    const putsBeforeFail = puts;
    setInvert(false);
    failNext = true;
    report.failed = publish();
    report.failedPuts = puts - putsBeforeFail;
    const putsBeforeRetry = puts;
    report.retried = publish();
    report.retriedPuts = puts - putsBeforeRetry;
    const settled = report.retried.events[0] ? report.retried.events[0].generation : 0;
    setInvert(true);
    failNext = true;
    report.staleFail = publish();
    setInvert(false);
    report.newer = publish();
    report.settledGeneration = settled;
  } catch (caught) {
    report.error = String(caught);
  }
  self.postMessage(report);
};
`;

const ALLOC_WORKER = `
const counts = { clamped: 0, image: 0 };

function watch(name, key) {
  const original = globalThis[name];
  globalThis[name] = new Proxy(original, {
    construct(target, args, newTarget) {
      counts[key] += 1;
      return Reflect.construct(target, args, newTarget);
    },
  });
}

watch("Uint8ClampedArray", "clamped");
watch("ImageData", "image");

let api = null;
let memory = null;

function snap() {
  return { clamped: counts.clamped, image: counts.image };
}

function sample40() {
  const bytes = new Uint8Array(4);
  new DataView(bytes.buffer).setFloat32(0, 40, true);
  return bytes;
}

function commitSample() {
  const bytes = sample40();
  const ptr = api.alloc(4);
  const ticket = api.bulk_ticket();
  new Uint8Array(memory.buffer, ptr, 4).set(bytes);
  if (api.commit_frame(ticket, 4, 1, 1, 1, 1) !== 1) {
    throw new Error("sample commit was refused");
  }
}

function setInvert(enabled) {
  const packed = new Uint8Array([9, 0, 1, 0, 1, 0, 0, 0, enabled ? 1 : 0]);
  const ptr = api.alloc(packed.byteLength);
  const ticket = api.bulk_ticket();
  new Uint8Array(memory.buffer, ptr, packed.byteLength).set(packed);
  if (api.apply_commands(ticket, packed.byteLength) !== 1) {
    throw new Error("invert command was refused");
  }
}

self.onmessage = async () => {
  const report = {};
  try {
    const glue = await import("./ocelli_wasm.js");
    await glue.default();
    api = glue;
    memory = glue.__wasm.memory;
    const canvas = new OffscreenCanvas(1, 1);
    api.bindCpuCanvas(canvas, 1, 1, 1);
    const afterBind = snap();
    if (afterBind.image < 1) {
      throw new Error("ImageData counter did not see bind");
    }
    commitSample();
    api.publishCpuCanvas();
    setInvert(true);
    api.publishCpuCanvas();
    api.publishCpuCanvas();
    const afterPublish = snap();
    api.resizeCpuCanvas(1, 1, 1);
    const afterSame = snap();
    let rejectedError = "";
    try {
      api.resizeCpuCanvas(0, 1, 1);
    } catch (caught) {
      rejectedError = String(caught);
    }
    const afterRejected = snap();
    api.resizeCpuCanvas(2, 1, 1);
    const afterResize = snap();
    api.publishCpuCanvas();
    const afterResizePublish = snap();
    const beforeProbe = counts.clamped;
    new Uint8ClampedArray(4);
    report.counterSeesClamped = counts.clamped === beforeProbe + 1;
    report.afterBind = afterBind;
    report.afterPublish = afterPublish;
    report.afterSame = afterSame;
    report.rejectedError = rejectedError;
    report.afterRejected = afterRejected;
    report.afterResize = afterResize;
    report.afterResizePublish = afterResizePublish;
    const context = canvas.getContext("2d");
    report.pixels = Array.from(context.getImageData(0, 0, 1, 1).data);
    report.width = canvas.width;
    report.height = canvas.height;
  } catch (caught) {
    report.error = String(caught);
  }
  self.postMessage(report);
};
`;

const COLOR_WORKER = `
const counts = { clamped: 0, image: 0 };
let puts = 0;
let failNext = false;

function watch(name, key) {
  const original = globalThis[name];
  globalThis[name] = new Proxy(original, {
    construct(target, args, newTarget) {
      counts[key] += 1;
      return Reflect.construct(target, args, newTarget);
    },
  });
}

watch("Uint8ClampedArray", "clamped");
watch("ImageData", "image");

let api = null;
let memory = null;
let canvas = null;

function u32(bytes, offset) {
  return new DataView(bytes.buffer, bytes.byteOffset, bytes.byteLength).getUint32(offset, true);
}

function drain() {
  const bytes = api.drain_events();
  const count = bytes.byteLength >= 16 ? u32(bytes, 0) : 0;
  const events = [];
  for (let index = 0; index < count; index += 1) {
    const at = 16 + index * 48;
    events.push({
      kind: u32(bytes, at),
      generation: u32(bytes, at + 16),
      width: u32(bytes, at + 20),
      height: u32(bytes, at + 24),
      flags: u32(bytes, at + 28),
    });
  }
  return events;
}

function stateFlags() {
  const state = api.copy_state();
  if (state.byteLength !== 96) {
    throw new Error("state copy was " + state.byteLength);
  }
  return u32(state, 80);
}

function readPixels() {
  const context = canvas.getContext("2d");
  const image = context.getImageData(0, 0, canvas.width, canvas.height);
  return Array.from(image.data);
}

function around(body) {
  const image = counts.image;
  const clamped = counts.clamped;
  const putsBefore = puts;
  let error = "";
  try {
    body();
  } catch (caught) {
    error = String(caught);
  }
  return {
    error,
    imageDelta: counts.image - image,
    clampedDelta: counts.clamped - clamped,
    putDelta: puts - putsBefore,
    pixels: readPixels(),
    events: drain(),
    flags: stateFlags(),
    width: canvas.width,
    height: canvas.height,
  };
}

function commitRgb() {
  const bytes = new Uint8Array(24);
  const view = new DataView(bytes.buffer);
  [10, 20, 30, 40, 50, 60].forEach((value, index) => {
    view.setFloat32(index * 4, value, true);
  });
  const ptr = api.alloc(24);
  const ticket = api.bulk_ticket();
  if (ptr === 0 || ticket === 0) {
    throw new Error("colour allocation was refused");
  }
  new Uint8Array(memory.buffer, ptr, 24).set(bytes);
  if (api.commit_frame(ticket, 24, 1, 2, 1, 1) !== 1) {
    throw new Error("colour commit was refused");
  }
}

function setInvert(enabled) {
  const packed = new Uint8Array([9, 0, 1, 0, 1, 0, 0, 0, enabled ? 1 : 0]);
  const ptr = api.alloc(packed.byteLength);
  const ticket = api.bulk_ticket();
  if (ptr === 0 || ticket === 0) {
    throw new Error("command allocation was refused");
  }
  new Uint8Array(memory.buffer, ptr, packed.byteLength).set(packed);
  if (api.apply_commands(ticket, packed.byteLength) !== 1) {
    throw new Error("invert command was refused");
  }
}

function patchBlit() {
  const prototype = globalThis.OffscreenCanvasRenderingContext2D.prototype;
  const original = prototype.putImageData;
  prototype.putImageData = function patchedPut(...args) {
    puts += 1;
    if (failNext) {
      failNext = false;
      throw new Error("injected blit failure");
    }
    return original.apply(this, args);
  };
}

self.onmessage = async () => {
  const report = { htmlContext: typeof CanvasRenderingContext2D };
  try {
    const glue = await import("./ocelli_wasm.js");
    await glue.default();
    api = glue;
    memory = glue.__wasm.memory;
    patchBlit();
    canvas = new OffscreenCanvas(2, 1);
    report.bind = around(() => {
      api.bindCpuCanvas(canvas, 2, 1, 1);
    });
    commitRgb();
    // Commit requests generation 1. Colour settle does not bump.
    // Each interaction calls begin. The failed interaction still begins.
    // SetInvert begins again. The retry begins once more. Settle uses
    // the generation that retry left, which is 5.
    report.full = around(() => {
      api.publishCpuColor(1);
    });
    report.interaction = around(() => {
      api.publishCpuInteraction();
    });
    failNext = true;
    report.failed = around(() => {
      api.publishCpuInteraction();
    });
    setInvert(true);
    report.retry = around(() => {
      api.publishCpuInteraction();
    });
    report.settled = around(() => {
      api.publishCpuColor(1);
    });
    report.idle = around(() => {
      api.publishCpuColor(1);
    });
    report.sameResize = around(() => {
      api.resizeCpuCanvas(2, 1, 1);
    });
    report.rejectedResize = around(() => {
      api.resizeCpuCanvas(0, 1, 1);
    });
    report.unavailable = around(() => {
      api.publishCpuColor(0);
    });
  } catch (caught) {
    report.error = String(caught);
  }
  self.postMessage(report);
};
`;

const MODE_WORKER = `
let api = null;
let memory = null;
let canvas = null;

function u32(bytes, offset) {
  return new DataView(bytes.buffer, bytes.byteOffset, bytes.byteLength).getUint32(offset, true);
}

function drain() {
  const bytes = api.drain_events();
  const count = bytes.byteLength >= 16 ? u32(bytes, 0) : 0;
  const events = [];
  for (let index = 0; index < count; index += 1) {
    const at = 16 + index * 48;
    events.push({
      kind: u32(bytes, at),
      generation: u32(bytes, at + 16),
      width: u32(bytes, at + 20),
      height: u32(bytes, at + 24),
      flags: u32(bytes, at + 28),
    });
  }
  return events;
}

function stateFlags() {
  const state = api.copy_state();
  if (state.byteLength !== 96) {
    throw new Error("state copy was " + state.byteLength);
  }
  return u32(state, 80);
}

function readPixels() {
  const context = canvas.getContext("2d");
  const image = context.getImageData(0, 0, canvas.width, canvas.height);
  return Array.from(image.data);
}

function capture(body) {
  let error = "";
  try {
    body();
  } catch (caught) {
    error = String(caught);
  }
  return {
    error,
    pixels: readPixels(),
    events: drain(),
    flags: stateFlags(),
    width: canvas.width,
    height: canvas.height,
  };
}

function observe() {
  return capture(() => {});
}

function commitSample() {
  const bytes = new Uint8Array(4);
  new DataView(bytes.buffer).setFloat32(0, 40, true);
  const ptr = api.alloc(4);
  const ticket = api.bulk_ticket();
  if (ptr === 0 || ticket === 0) {
    throw new Error("sample allocation was refused");
  }
  new Uint8Array(memory.buffer, ptr, 4).set(bytes);
  if (api.commit_frame(ticket, 4, 1, 1, 1, 1) !== 1) {
    throw new Error("sample commit was refused");
  }
}

function commitRgb() {
  const bytes = new Uint8Array(24);
  const view = new DataView(bytes.buffer);
  [10, 20, 30, 40, 50, 60].forEach((value, index) => {
    view.setFloat32(index * 4, value, true);
  });
  const ptr = api.alloc(24);
  const ticket = api.bulk_ticket();
  if (ptr === 0 || ticket === 0) {
    throw new Error("colour allocation was refused");
  }
  new Uint8Array(memory.buffer, ptr, 24).set(bytes);
  if (api.commit_frame(ticket, 24, 1, 2, 1, 1) !== 1) {
    throw new Error("colour commit was refused");
  }
}

function setInvert(enabled) {
  const packed = new Uint8Array([9, 0, 1, 0, 1, 0, 0, 0, enabled ? 1 : 0]);
  const ptr = api.alloc(packed.byteLength);
  const ticket = api.bulk_ticket();
  if (ptr === 0 || ticket === 0) {
    throw new Error("command allocation was refused");
  }
  new Uint8Array(memory.buffer, ptr, packed.byteLength).set(packed);
  if (api.apply_commands(ticket, packed.byteLength) !== 1) {
    throw new Error("invert command was refused");
  }
}

self.onmessage = async (event) => {
  const scenario = event.data && event.data.scenario;
  const report = { scenario, htmlContext: typeof CanvasRenderingContext2D };
  try {
    const glue = await import("./ocelli_wasm.js");
    await glue.default();
    api = glue;
    memory = glue.__wasm.memory;
    const mono = scenario === "settled-mono" || scenario === "invalidated-mono";
    const rgb = scenario === "settled-rgb" || scenario === "invalidated-rgb";
    if (!mono && !rgb) {
      throw new Error("unknown mode scenario");
    }
    if (mono) {
      canvas = new OffscreenCanvas(1, 1);
      api.bindCpuCanvas(canvas, 1, 1, 1);
      commitSample();
      report.settled = capture(() => {
        api.publishCpuCanvas();
      });
      if (scenario === "invalidated-mono") {
        setInvert(true);
        report.invalidated = observe();
      }
      report.rejected = capture(() => {
        api.publishCpuColor(1);
      });
      report.interaction = capture(() => {
        api.publishCpuInteraction();
      });
      report.recovered = capture(() => {
        api.publishCpuCanvas();
      });
    } else {
      canvas = new OffscreenCanvas(2, 1);
      api.bindCpuCanvas(canvas, 2, 1, 1);
      commitRgb();
      report.settled = capture(() => {
        api.publishCpuColor(1);
      });
      if (scenario === "invalidated-rgb") {
        setInvert(true);
        report.invalidated = observe();
      }
      report.rejected = capture(() => {
        api.publishCpuCanvas();
      });
      report.interaction = capture(() => {
        api.publishCpuInteraction();
      });
      report.recovered = capture(() => {
        api.publishCpuColor(1);
      });
    }
  } catch (caught) {
    report.error = String(caught);
  }
  self.postMessage(report);
};
`;

function contentType(name) {
  if (name.endsWith(".wasm")) {
    return "application/wasm";
  }
  if (name.endsWith(".js")) {
    return "text/javascript; charset=utf-8";
  }
  return "application/octet-stream";
}

function listen(server) {
  return new Promise((resolve, reject) => {
    server.once("error", reject);
    server.listen(0, "127.0.0.1", () => resolve());
  });
}

function sameCount(left, right) {
  return left.clamped === right.clamped && left.image === right.image;
}

function assertMode(report, scenario) {
  assert.equal(report.error, undefined);
  assert.equal(report.htmlContext, "undefined");
  assert.equal(report.scenario, scenario);
}

function assertPresented(step, pixels, generation, width, height, flags) {
  assert.equal(step.error, "");
  assert.deepEqual(step.pixels, pixels);
  assert.equal(step.events.length, 1);
  assert.equal(step.events[0].kind, 6);
  assert.equal(step.events[0].generation, generation);
  assert.equal(step.events[0].width, width);
  assert.equal(step.events[0].height, height);
  assert.equal(step.events[0].flags, flags);
  assert.equal(step.flags, flags);
}

function assertRefused(step, error, pixels, flags) {
  assert.equal(step.error, error);
  assert.deepEqual(step.pixels, pixels);
  assert.deepEqual(step.events, []);
  assert.equal(step.flags, flags);
}

function assertHeld(step, pixels, flags) {
  assert.equal(step.error, "");
  assert.deepEqual(step.pixels, pixels);
  assert.deepEqual(step.events, []);
  assert.equal(step.flags, flags);
}

const server = createServer(async (request, response) => {
  const url = new globalThis.URL(request.url ?? "/", "http://127.0.0.1");
  if (url.pathname === "/harness.html") {
    response.writeHead(200, { "content-type": "text/html; charset=utf-8" });
    response.end(HARNESS);
    return;
  }
  if (url.pathname === "/worker.js") {
    response.writeHead(200, { "content-type": "text/javascript; charset=utf-8" });
    response.end(WORKER);
    return;
  }
  if (url.pathname === "/alloc-worker.js") {
    response.writeHead(200, { "content-type": "text/javascript; charset=utf-8" });
    response.end(ALLOC_WORKER);
    return;
  }
  if (url.pathname === "/color-worker.js") {
    response.writeHead(200, { "content-type": "text/javascript; charset=utf-8" });
    response.end(COLOR_WORKER);
    return;
  }
  if (url.pathname === "/mode-worker.js") {
    response.writeHead(200, { "content-type": "text/javascript; charset=utf-8" });
    response.end(MODE_WORKER);
    return;
  }
  const name = url.pathname.slice(1);
  if (name.length === 0 || name.includes("/") || name.includes("..")) {
    response.writeHead(404);
    response.end();
    return;
  }
  try {
    const bytes = await readFile(join(PKG, name));
    response.writeHead(200, { "content-type": contentType(name) });
    response.end(bytes);
  } catch {
    response.writeHead(404);
    response.end();
  }
});

await listen(server);
const address = server.address();
const port = address && typeof address === "object" ? address.port : 0;

const browser = await chromium.launch({
  headless: true,
  args: ["--disable-gpu", "--force-device-scale-factor=1"],
});

function ask(page, workerUrl, message = { op: "run" }) {
  return page.evaluate(
    async ({ url, message }) => {
      const worker = new globalThis.Worker(url, { type: "module" });
      return new Promise((resolve, reject) => {
        const timer = globalThis.setTimeout(() => reject(new Error("worker timed out")), 55000);
        worker.addEventListener(
          "message",
          (event) => {
            globalThis.clearTimeout(timer);
            resolve(event.data);
          },
          { once: true },
        );
        worker.postMessage(message);
      });
    },
    { url: workerUrl, message },
  );
}

try {
  const page = await browser.newPage();
  page.setDefaultTimeout(60_000);
  await page.goto(`http://127.0.0.1:${port}/harness.html`);
  const happy = await ask(page, "/worker.js");
  const alloc = await ask(page, "/alloc-worker.js");
  const color = await ask(page, "/color-worker.js");
  const settledMono = await ask(page, "/mode-worker.js", { scenario: "settled-mono" });
  const invalidatedMono = await ask(page, "/mode-worker.js", {
    scenario: "invalidated-mono",
  });
  const settledRgb = await ask(page, "/mode-worker.js", { scenario: "settled-rgb" });
  const invalidatedRgb = await ask(page, "/mode-worker.js", {
    scenario: "invalidated-rgb",
  });
  globalThis.console.log(
    JSON.stringify({
      happy,
      alloc,
      color,
      settledMono,
      invalidatedMono,
      settledRgb,
      invalidatedRgb,
    }),
  );

  assert.equal(happy.error, undefined);
  assert.equal(happy.htmlContext, "undefined");
  assert.deepEqual(happy.first.pixels, [255, 255, 255, 255]);
  assert.equal(happy.first.error, "");
  assert.equal(happy.first.events.length, 1);
  assert.equal(happy.first.events[0].kind, 6);
  assert.equal(happy.first.events[0].generation, 1);
  assert.equal(happy.first.events[0].width, 1);
  assert.equal(happy.first.events[0].height, 1);
  assert.equal(happy.first.flags & 8, 0);

  assert.equal(happy.sameResize.error, "");
  assert.equal(happy.sameResize.width, 1);
  assert.equal(happy.sameResize.height, 1);
  assert.deepEqual(happy.sameResize.pixels, [255, 255, 255, 255]);
  assert.deepEqual(happy.sameResize.events, []);
  assert.equal(happy.sameResize.flags & 8, 0);
  assert.equal(happy.samePublish.error, "");
  assert.deepEqual(happy.samePublish.pixels, [255, 255, 255, 255]);
  assert.deepEqual(happy.samePublish.events, []);
  assert.equal(happy.samePublish.flags & 8, 0);

  assert.equal(
    happy.rejectedResize.error,
    "cpu raster extent is outside the pre-sized canvas",
  );
  assert.equal(happy.rejectedResize.width, 1);
  assert.equal(happy.rejectedResize.height, 1);
  assert.deepEqual(happy.rejectedResize.pixels, [255, 255, 255, 255]);
  assert.deepEqual(happy.rejectedResize.events, []);
  assert.equal(happy.rejectedResize.flags & 8, 0);
  assert.equal(happy.rejectedPublish.error, "");
  assert.deepEqual(happy.rejectedPublish.pixels, [255, 255, 255, 255]);
  assert.deepEqual(happy.rejectedPublish.events, []);
  assert.equal(happy.rejectedPublish.flags & 8, 0);

  assert.equal(happy.idle.error, "");
  assert.deepEqual(happy.idle.events, []);
  assert.deepEqual(happy.idle.pixels, [255, 255, 255, 255]);

  assert.equal(happy.inverted.error, "");
  assert.deepEqual(happy.inverted.pixels, [0, 0, 0, 255]);
  assert.equal(happy.inverted.events.length, 1);
  assert.equal(happy.inverted.events[0].kind, 6);
  assert.equal(happy.inverted.events[0].generation, 2);
  assert.equal(happy.inverted.flags & 8, 0);
  assert.equal(happy.inverted.flags & 1, 1);

  // putImageData throws an Error object. JsValue::as_string does not read
  // Error.message, so the product reports the stable fallback.
  assert.equal(happy.failed.error, "canvas blit failed");
  assert.equal(happy.failedPuts, 1);
  assert.deepEqual(happy.failed.events, []);
  assert.equal(happy.failed.flags & 8, 8);
  assert.deepEqual(happy.failed.pixels, [0, 0, 0, 255]);

  assert.equal(happy.retried.error, "");
  assert.equal(happy.retriedPuts, 1);
  assert.equal(happy.retried.events.length, 1);
  assert.deepEqual(happy.retried.pixels, [255, 255, 255, 255]);
  assert.equal(happy.retried.flags & 8, 0);
  assert.equal(happy.retried.events[0].kind, 6);
  assert.equal(happy.retried.events[0].generation, 3);

  assert.equal(happy.staleFail.error, "canvas blit failed");
  assert.deepEqual(happy.staleFail.events, []);
  assert.equal(happy.newer.error, "");
  assert.equal(happy.newer.events.length, 1);
  assert.deepEqual(happy.newer.pixels, [255, 255, 255, 255]);
  assert.equal(happy.newer.events[0].generation, happy.settledGeneration + 2);
  assert.equal(happy.newer.flags & 8, 0);

  assert.equal(alloc.error, undefined);
  assert.equal(alloc.counterSeesClamped, true);
  assert.equal(sameCount(alloc.afterBind, alloc.afterPublish), true);
  assert.equal(sameCount(alloc.afterPublish, alloc.afterSame), true);
  assert.equal(
    alloc.rejectedError,
    "cpu raster extent is outside the pre-sized canvas",
  );
  assert.equal(sameCount(alloc.afterPublish, alloc.afterRejected), true);
  assert.ok(alloc.afterResize.image > alloc.afterRejected.image);
  assert.equal(sameCount(alloc.afterResize, alloc.afterResizePublish), true);
  assert.deepEqual(alloc.pixels, [0, 0, 0, 255]);
  assert.equal(alloc.width, 2);
  assert.equal(alloc.height, 1);

  // Hand mapping, not a value read back from a first canvas run.
  // Full device samples source columns 0 and 1. The reduced 1 by 1 sample
  // is source column 1, then replicated across both device pixels.
  const fullRgba = [10, 20, 30, 255, 40, 50, 60, 255];
  const reducedRgba = [40, 50, 60, 255, 40, 50, 60, 255];
  assert.equal(color.error, undefined);
  assert.equal(color.htmlContext, "undefined");
  assert.equal(color.bind.error, "");
  assert.equal(color.bind.imageDelta, 1);
  assert.equal(color.bind.width, 2);
  assert.equal(color.bind.height, 1);

  assert.equal(color.full.error, "");
  assert.deepEqual(color.full.pixels, fullRgba);
  assert.equal(color.full.imageDelta, 0);
  assert.equal(color.full.clampedDelta, 0);
  assert.equal(color.full.putDelta, 1);
  assert.equal(color.full.events.length, 1);
  assert.equal(color.full.events[0].kind, 6);
  assert.equal(color.full.events[0].generation, 1);
  assert.equal(color.full.events[0].width, 2);
  assert.equal(color.full.events[0].height, 1);
  assert.equal(color.full.events[0].flags, 0);
  assert.equal(color.full.flags, 0);
  assert.equal(color.full.width, 2);
  assert.equal(color.full.height, 1);

  assert.equal(color.interaction.error, "");
  assert.deepEqual(color.interaction.pixels, reducedRgba);
  assert.equal(color.interaction.imageDelta, 0);
  assert.equal(color.interaction.clampedDelta, 0);
  assert.equal(color.interaction.putDelta, 1);
  assert.equal(color.interaction.events.length, 1);
  assert.equal(color.interaction.events[0].kind, 6);
  assert.equal(color.interaction.events[0].generation, 2);
  assert.equal(color.interaction.events[0].width, 1);
  assert.equal(color.interaction.events[0].height, 1);
  assert.equal(color.interaction.events[0].flags, 8);
  assert.equal(color.interaction.flags, 8);
  assert.equal(color.interaction.width, 2);
  assert.equal(color.interaction.height, 1);

  assert.equal(color.failed.error, "canvas blit failed");
  assert.deepEqual(color.failed.pixels, reducedRgba);
  assert.equal(color.failed.imageDelta, 0);
  assert.equal(color.failed.clampedDelta, 0);
  assert.equal(color.failed.putDelta, 1);
  assert.deepEqual(color.failed.events, []);
  assert.equal(color.failed.flags, 8);
  assert.equal(color.failed.width, 2);
  assert.equal(color.failed.height, 1);

  assert.equal(color.retry.error, "");
  assert.deepEqual(color.retry.pixels, reducedRgba);
  assert.equal(color.retry.imageDelta, 0);
  assert.equal(color.retry.clampedDelta, 0);
  assert.equal(color.retry.putDelta, 1);
  assert.equal(color.retry.events.length, 1);
  assert.equal(color.retry.events[0].kind, 6);
  assert.equal(color.retry.events[0].generation, 5);
  assert.equal(color.retry.events[0].width, 1);
  assert.equal(color.retry.events[0].height, 1);
  assert.equal(color.retry.events[0].flags, 9);
  assert.equal(color.retry.flags, 9);
  assert.equal(color.retry.width, 2);
  assert.equal(color.retry.height, 1);

  assert.equal(color.settled.error, "");
  assert.deepEqual(color.settled.pixels, fullRgba);
  assert.equal(color.settled.imageDelta, 0);
  assert.equal(color.settled.clampedDelta, 0);
  assert.equal(color.settled.putDelta, 1);
  assert.equal(color.settled.events.length, 1);
  assert.equal(color.settled.events[0].kind, 6);
  assert.equal(color.settled.events[0].generation, 5);
  assert.equal(color.settled.events[0].width, 2);
  assert.equal(color.settled.events[0].height, 1);
  assert.equal(color.settled.events[0].flags, 1);
  assert.equal(color.settled.flags, 1);

  assert.equal(color.idle.error, "");
  assert.deepEqual(color.idle.pixels, fullRgba);
  assert.equal(color.idle.imageDelta, 0);
  assert.equal(color.idle.clampedDelta, 0);
  assert.equal(color.idle.putDelta, 0);
  assert.deepEqual(color.idle.events, []);
  assert.equal(color.idle.flags, 1);

  assert.equal(color.sameResize.error, "");
  assert.deepEqual(color.sameResize.pixels, fullRgba);
  assert.equal(color.sameResize.imageDelta, 0);
  assert.equal(color.sameResize.clampedDelta, 0);
  assert.equal(color.sameResize.putDelta, 0);
  assert.deepEqual(color.sameResize.events, []);
  assert.equal(color.sameResize.flags, 1);
  assert.equal(color.sameResize.width, 2);
  assert.equal(color.sameResize.height, 1);

  assert.equal(
    color.rejectedResize.error,
    "cpu raster extent is outside the pre-sized canvas",
  );
  assert.deepEqual(color.rejectedResize.pixels, fullRgba);
  assert.equal(color.rejectedResize.imageDelta, 0);
  assert.equal(color.rejectedResize.clampedDelta, 0);
  assert.equal(color.rejectedResize.putDelta, 0);
  assert.deepEqual(color.rejectedResize.events, []);
  assert.equal(color.rejectedResize.flags, 1);
  assert.equal(color.rejectedResize.width, 2);
  assert.equal(color.rejectedResize.height, 1);

  assert.equal(color.unavailable.error, "cpu raster colour space is unavailable");
  assert.deepEqual(color.unavailable.pixels, fullRgba);
  assert.equal(color.unavailable.imageDelta, 0);
  assert.equal(color.unavailable.clampedDelta, 0);
  assert.equal(color.unavailable.putDelta, 0);
  assert.deepEqual(color.unavailable.events, []);
  assert.equal(color.unavailable.flags, 1);
  assert.equal(color.unavailable.width, 2);
  assert.equal(color.unavailable.height, 1);

  const lengthError = "cpu raster source length does not match the grid";
  const white = [255, 255, 255, 255];
  const black = [0, 0, 0, 255];
  assertMode(settledMono, "settled-mono");
  assertPresented(settledMono.settled, white, 1, 1, 1, 0);
  assertRefused(settledMono.rejected, lengthError, white, 0);
  assertPresented(settledMono.interaction, white, 2, 1, 1, 8);
  assert.equal(settledMono.interaction.width, 1);
  assert.equal(settledMono.interaction.height, 1);
  assertPresented(settledMono.recovered, white, 2, 1, 1, 0);

  assertMode(invalidatedMono, "invalidated-mono");
  assertPresented(invalidatedMono.settled, white, 1, 1, 1, 0);
  assertHeld(invalidatedMono.invalidated, white, 9);
  assertRefused(invalidatedMono.rejected, lengthError, white, 9);
  assertPresented(invalidatedMono.interaction, black, 3, 1, 1, 9);
  assertPresented(invalidatedMono.recovered, black, 3, 1, 1, 1);

  assertMode(settledRgb, "settled-rgb");
  assertPresented(settledRgb.settled, fullRgba, 1, 2, 1, 0);
  assert.equal(settledRgb.settled.width, 2);
  assert.equal(settledRgb.settled.height, 1);
  assertRefused(settledRgb.rejected, lengthError, fullRgba, 0);
  assert.equal(settledRgb.rejected.width, 2);
  assert.equal(settledRgb.rejected.height, 1);
  assertPresented(settledRgb.interaction, reducedRgba, 2, 1, 1, 8);
  assert.equal(settledRgb.interaction.width, 2);
  assert.equal(settledRgb.interaction.height, 1);
  assertPresented(settledRgb.recovered, fullRgba, 2, 2, 1, 0);
  assert.equal(settledRgb.recovered.width, 2);
  assert.equal(settledRgb.recovered.height, 1);

  assertMode(invalidatedRgb, "invalidated-rgb");
  assertPresented(invalidatedRgb.settled, fullRgba, 1, 2, 1, 0);
  assertHeld(invalidatedRgb.invalidated, fullRgba, 9);
  assertRefused(invalidatedRgb.rejected, lengthError, fullRgba, 9);
  assertPresented(invalidatedRgb.interaction, reducedRgba, 3, 1, 1, 9);
  assert.equal(invalidatedRgb.interaction.width, 2);
  assert.equal(invalidatedRgb.interaction.height, 1);
  assertPresented(invalidatedRgb.recovered, fullRgba, 3, 2, 1, 1);
  assert.equal(invalidatedRgb.recovered.width, 2);
  assert.equal(invalidatedRgb.recovered.height, 1);
} finally {
  await browser.close();
  await new Promise((resolve) => server.close(resolve));
}
