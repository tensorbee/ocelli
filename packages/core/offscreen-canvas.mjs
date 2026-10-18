/**
 * F-039. A worker OffscreenCanvas bitmap matches device_extent.
 *
 * CSS 100 by 50 at dpr 2 is 200 by 100. The CSS size itself would be 100 by
 * 50, and that answer fails this file. A resize of the same box at dpr 1.25
 * is 125 by 63, because 50 * 1.25 is 62.5 and half away from zero rounds to
 * 63.
 *
 * Playwright is the copy pinned at 1.62.1 in tools/oracle. Chromium is launched
 * with that harness's software-rasteriser arguments. Run after
 * `bin/ocelli.sh wasm`, which writes crates/ocelli-wasm/pkg.
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
const PKG = join(ROOT, "crates", "ocelli-wasm", "pkg");

const CHROMIUM_ARGS = [
  "--use-gl=angle",
  "--use-angle=swiftshader",
  "--enable-unsafe-swiftshader",
  "--force-device-scale-factor=1",
  "--disable-lcd-text",
  "--hide-scrollbars",
];

const HARNESS = `<!DOCTYPE html>
<meta charset="utf-8">
<title>ocelli offscreen canvas</title>
`;

const WORKER = `
import init, { attachOffscreenCanvas, resizeOffscreenCanvas } from "./ocelli_wasm.js";

const ready = init();
let canvas = null;

self.onmessage = async (event) => {
  const data = event.data;
  let error = "";
  try {
    await ready;
    if (data.canvas) {
      canvas = data.canvas;
    }
    if (data.op === "attach") {
      await attachOffscreenCanvas(0, canvas, data.cssWidth, data.cssHeight, data.dpr);
    } else {
      await resizeOffscreenCanvas(0, canvas, data.cssWidth, data.cssHeight, data.dpr);
    }
  } catch (caught) {
    error = String(caught);
  }
  self.postMessage({
    width: canvas ? canvas.width : 0,
    height: canvas ? canvas.height : 0,
    dpr: data.dpr,
    error,
  });
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

const server = createServer(async (request, response) => {
  const url = new URL(request.url ?? "/", "http://127.0.0.1");
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
  args: CHROMIUM_ARGS,
});

try {
  const page = await browser.newPage();
  page.setDefaultTimeout(60_000);
  page.setDefaultNavigationTimeout(60_000);
  await page.goto(`http://127.0.0.1:${port}/harness.html`);
  const report = await page.evaluate(async () => {
    const worker = new Worker("/worker.js", { type: "module" });
    const next = () =>
      new Promise((resolve, reject) => {
        const timer = setTimeout(() => reject(new Error("worker timed out")), 55000);
        worker.addEventListener(
          "message",
          (event) => {
            clearTimeout(timer);
            resolve(event.data);
          },
          { once: true },
        );
      });
    const element = document.createElement("canvas");
    const offscreen = element.transferControlToOffscreen();
    worker.postMessage(
      { op: "attach", canvas: offscreen, cssWidth: 100, cssHeight: 50, dpr: 2 },
      [offscreen],
    );
    const attached = await next();
    worker.postMessage({ op: "resize", cssWidth: 100, cssHeight: 50, dpr: 1.25 });
    const resized = await next();
    return { attached, resized };
  });
  console.log(JSON.stringify(report));
  assert.equal(report.attached.dpr, 2);
  assert.equal(report.attached.width, 200);
  assert.equal(report.attached.height, 100);
  assert.notEqual(report.attached.width, 100);
  assert.notEqual(report.attached.height, 50);
  assert.equal(report.resized.dpr, 1.25);
  assert.equal(report.resized.width, 125);
  assert.equal(report.resized.height, 63);
} finally {
  await browser.close();
  await new Promise((resolve) => server.close(resolve));
}
