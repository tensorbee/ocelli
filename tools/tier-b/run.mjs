// F-042 and F-235. Section 18.3 and the two stage-one pairs on headless
// Chromium WebGL2.
//
// Playwright is the oracle's 1.62.1. This script does not add a dependency.
// The Chromium launch does not pass the oracle's SwiftShader flags. A
// software rasteriser that resolves tier C is not a tier B run.
//
// The build runs here with `tier-b-probe`, a cargo feature that is off by
// default, so the shipped module and its size budget do not carry it.
//
// Exit 0: drew, tier B, within 0.001 of the hand rows, of LutChain, and of
// the pinned stage-one displays 0.0, 255.0 and 127.5.
// Exit 1: drew a different number, or decidedBy is Benchmark.
// Exit 2: the stack pass is unavailable and drew nothing.
// Exit 3: no adapter, demoted, crash, or the tier is not B.

import { createRequire } from "node:module";
import { createServer } from "node:http";
import { spawnSync } from "node:child_process";
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
const WINDOW_ENDS = [0, 255];
const CANCELLATION = 127.5;
const TOLERANCE = 0.001;

const build = spawnSync(
  "wasm-pack",
  [
    "build",
    "crates/ocelli-wasm",
    "--target",
    "web",
    "--out-dir",
    "target/tier-b-probe",
    "--",
    "--features",
    "tier-b-probe",
  ],
  { stdio: "inherit", cwd: repo },
);
if (build.status !== 0) {
  console.error(`wasm-pack build failed with status ${build.status}`);
  process.exit(build.status ?? 3);
}

const PAGE = `<!doctype html>
<meta charset="utf-8">
<body>
<script type="module">
import init, { tierBSection183, tierBStageOnePairs } from "./ocelli_wasm.js";
try {
  await init();
  const canvas = new OffscreenCanvas(64, 64);
  const section = await tierBSection183(canvas);
  const pairs = await tierBStageOnePairs(new OffscreenCanvas(64, 64));
  document.body.dataset.result = JSON.stringify({ section: JSON.parse(section), pairs: JSON.parse(pairs) });
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
    rowClose(report.section.linear, LINEAR, "linear"),
    rowClose(report.section.exact, LINEAR_EXACT, "exact"),
    rowClose(report.section.linear, report.section.cpuLinear, "linear-cpu"),
    rowClose(report.section.exact, report.section.cpuExact, "exact-cpu"),
    rowClose(report.pairs.windowEnds, WINDOW_ENDS, "window-ends"),
  ].filter((item) => item !== "");
  const low = Array.isArray(report.section.exact) ? report.section.exact[0] : Number.NaN;
  if (!close(low, 0)) {
    problems.push(`LINEAR_EXACT(-160) was ${low} wanted 0`);
  }
  const centreLinear = Array.isArray(report.section.linear) ? report.section.linear[1] : Number.NaN;
  const centreExact = Array.isArray(report.section.exact) ? report.section.exact[1] : Number.NaN;
  const gap = Math.abs(centreLinear - centreExact);
  if (!(gap > 0.2)) {
    problems.push(`centre gap was ${gap}`);
  }
  const cancellation = report.pairs.cancellation;
  if (!close(cancellation, CANCELLATION)) {
    problems.push(`stage one of 56893 was ${cancellation} wanted ${CANCELLATION}`);
  }
  if (report.section.format === "Rgba8Unorm") {
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
const section = report.section ?? {};
const pairs = report.pairs ?? {};
if (section.status === "crash" || pairs.status === "crash") {
  console.error(`probe crashed: ${section.reason ?? pairs.reason}`);
  process.exit(3);
}
if (section.status === "unavailable" || pairs.status === "unavailable") {
  console.error(`tier B pass unavailable: ${section.reason ?? pairs.reason}`);
  process.exit(2);
}
if (
  section.status !== "drew" ||
  section.tier !== "B" ||
  pairs.status !== "drew" ||
  pairs.tier !== "B"
) {
  console.error(
    `not a tier B run: section ${section.status}/${section.tier} pairs ${pairs.status}/${pairs.tier} ${section.reason ?? pairs.reason}`,
  );
  process.exit(3);
}
if (section.decidedBy === "Benchmark" || pairs.decidedBy === "Benchmark") {
  console.error("AutoFinish completion was recorded as a fill-rate benchmark");
  process.exit(1);
}
const problems = compare(report);
if (problems.length !== 0) {
  console.error(problems.join("\n"));
  process.exit(1);
}
console.log(
  `tier B section 18.3 and the stage-one pairs matched within ${TOLERANCE} format ${section.format}`,
);
process.exit(0);
