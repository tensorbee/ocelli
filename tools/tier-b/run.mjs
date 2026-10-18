// F-042. Section 18.3 on headless Chromium WebGL2.
//
// Playwright is the oracle's 1.62.1. This script does not add a dependency.
// The Chromium launch does not pass the oracle's SwiftShader flags. A
// software rasteriser that resolves tier C is not a tier B run.
//
// Exit 0: drew, tier B, within 0.001 of the hand rows and of LutChain.
// Exit 1: drew a different number, or decidedBy is Benchmark.
// Exit 2: the stack pass is unavailable and drew nothing.
// Exit 3: no adapter, demoted, crash, or the tier is not B.

import { createRequire } from "node:module";
import { createServer } from "node:http";
import { readFileSync, existsSync, writeFileSync } from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const here = path.dirname(fileURLToPath(import.meta.url));
const repo = path.resolve(here, "../..");
const outDir = path.join(repo, "crates/ocelli-wasm/target/tier-b-probe");
const require = createRequire(path.join(repo, "tools/oracle/package.json"));
const { chromium } = require("playwright");

const LINEAR = [0, 127.819, 255, 63.91];
const LINEAR_EXACT = [0, 127.5, 255, 63.75];
const TOLERANCE = 0.001;

const PAGE = `<!doctype html>
<meta charset="utf-8">
<body>
<script type="module">
import init, { tierBSection183 } from "./ocelli_wasm.js";
try {
  await init();
  const canvas = new OffscreenCanvas(64, 64);
  const text = await tierBSection183(canvas);
  document.body.dataset.result = text;
} catch (error) {
  document.body.dataset.result = JSON.stringify({
    status: "crash",
    reason: String(error && error.stack ? error.stack : error),
  });
}
</script>
</body>
`;

function close(actual, expected) {
  return Math.abs(actual - expected) < TOLERANCE;
}

function rowClose(actual, expected, label) {
  if (!Array.isArray(actual) || actual.length !== expected.length) {
    return `${label} length`;
  }
  for (let index = 0; index < expected.length; index += 1) {
    if (!close(actual[index], expected[index])) {
      return `${label}[${index}] was ${actual[index]} wanted ${expected[index]}`;
    }
  }
  return "";
}

function compare(report) {
  const problems = [
    rowClose(report.linear, LINEAR, "linear"),
    rowClose(report.exact, LINEAR_EXACT, "exact"),
    rowClose(report.linear, report.cpuLinear, "linear-cpu"),
    rowClose(report.exact, report.cpuExact, "exact-cpu"),
  ].filter((item) => item !== "");
  const low = Array.isArray(report.exact) ? report.exact[0] : Number.NaN;
  if (!close(low, 0)) {
    problems.push(`LINEAR_EXACT(-160) was ${low} wanted 0`);
  }
  const centreLinear = Array.isArray(report.linear) ? report.linear[1] : Number.NaN;
  const centreExact = Array.isArray(report.exact) ? report.exact[1] : Number.NaN;
  const gap = Math.abs(centreLinear - centreExact);
  if (!(gap > 0.2)) {
    problems.push(`centre gap was ${gap}`);
  }
  if (report.format === "Rgba8Unorm") {
    problems.push("stored format is Rgba8Unorm");
  }
  return problems;
}

function contentType(file) {
  if (file.endsWith(".wasm")) {
    return "application/wasm";
  }
  if (file.endsWith(".js")) {
    return "text/javascript";
  }
  return "text/html";
}

function serve(root) {
  return new Promise((resolve) => {
    const server = createServer((request, response) => {
      const url = new URL(request.url, "http://127.0.0.1");
      const relative = url.pathname === "/" ? "index.html" : url.pathname.slice(1);
      const file = path.normalize(path.join(root, relative));
      if (!file.startsWith(root) || !existsSync(file)) {
        response.writeHead(404);
        response.end("missing");
        return;
      }
      response.writeHead(200, { "content-type": contentType(file) });
      response.end(readFileSync(file));
    });
    server.listen(0, "127.0.0.1", () => {
      const address = server.address();
      resolve({ server, port: address.port });
    });
  });
}

const moduleJs = path.join(outDir, "ocelli_wasm.js");
if (!existsSync(moduleJs)) {
  console.error(`missing ${moduleJs}`);
  console.error("build with: wasm-pack build crates/ocelli-wasm --target web --out-dir target/tier-b-probe -- --features tier-b-probe");
  process.exit(3);
}

writeFileSync(path.join(outDir, "index.html"), PAGE);

const { server, port } = await serve(outDir);
const browser = await chromium.launch({
  headless: true,
  args: [
    "--use-gl=angle",
    "--use-angle=metal",
    "--ignore-gpu-blocklist",
    "--enable-webgl",
    "--enable-webgl2",
  ],
});

let text = "";
try {
  const page = await browser.newPage();
  page.on("console", (message) => {
    console.log(`console ${message.type()} ${message.text()}`);
  });
  page.on("pageerror", (error) => {
    console.log(`pageerror ${error}`);
  });
  await page.goto(`http://127.0.0.1:${port}/`, { waitUntil: "load", timeout: 120000 });
  await page.waitForFunction(
    () => document.body && document.body.dataset && document.body.dataset.result,
    null,
    { timeout: 120000 },
  );
  text = await page.locator("body").getAttribute("data-result");
} catch (error) {
  console.error(`webgl probe did not finish: ${error}`);
  await browser.close();
  server.close();
  process.exit(3);
}
await browser.close();
server.close();

console.log(text);
let report;
try {
  report = JSON.parse(text);
} catch (error) {
  console.error(`probe returned text that is not JSON: ${error}`);
  process.exit(3);
}

if (report.status === "crash") {
  console.error(`probe crashed: ${report.reason}`);
  process.exit(3);
}
if (report.status === "unavailable") {
  console.error(`tier B stack pass unavailable: ${report.reason}`);
  process.exit(2);
}
if (report.status !== "drew" || report.tier !== "B") {
  console.error(`not a tier B run: status ${report.status} tier ${report.tier} ${report.reason}`);
  process.exit(3);
}
if (report.decidedBy === "Benchmark") {
  console.error("AutoFinish completion was recorded as a fill-rate benchmark");
  process.exit(1);
}
const problems = compare(report);
if (problems.length !== 0) {
  console.error(problems.join("\n"));
  process.exit(1);
}
console.log(`tier B section 18.3 matched within ${TOLERANCE} format ${report.format}`);
process.exit(0);
