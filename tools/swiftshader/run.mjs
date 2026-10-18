// F-X002 layer 3. Headless Chrome with SwiftShader.
//
// This run is not the oracle and not lavapipe. It records the renderer
// string and the resolved tier. It does not compare pixels and it does not
// open the corpus.
//
// Exit 0: the renderer string contains SwiftShader and the tier is C.
// Exit 1: the renderer string contains SwiftShader and the tier is not C.
// Exit 3: the string is absent, the page crashed, or the browser did not finish.

import { createRequire } from "node:module";
import { spawnSync } from "node:child_process";
import { createServer } from "node:http";
import { readFileSync, existsSync, writeFileSync } from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const here = path.dirname(fileURLToPath(import.meta.url));
const repo = path.resolve(here, "../..");
const outDir = path.join(repo, "crates/ocelli-wasm/target/swiftshader-probe");
const require = createRequire(path.join(repo, "tools/oracle/package.json"));
const { chromium } = require("playwright");

console.log("This run is not the oracle and not lavapipe.");

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
    renderer: "",
    tier: "",
    reason: String(error && error.stack ? error.stack : error),
  });
}
</script>
</body>
`;

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

const build = spawnSync(
  "wasm-pack",
  [
    "build",
    "crates/ocelli-wasm",
    "--target",
    "web",
    "--out-dir",
    "target/swiftshader-probe",
    "--",
    "--features",
    "tier-b-probe",
  ],
  { cwd: repo, stdio: "inherit" },
);
if (build.status !== 0) {
  console.error("wasm-pack did not finish");
  process.exit(3);
}

const moduleJs = path.join(outDir, "ocelli_wasm.js");
if (!existsSync(moduleJs)) {
  console.error(`missing ${moduleJs}`);
  process.exit(3);
}

writeFileSync(path.join(outDir, "index.html"), PAGE);

const { server, port } = await serve(outDir);
const browser = await chromium.launch({
  headless: true,
  args: [
    "--use-gl=angle",
    "--use-angle=swiftshader",
    "--enable-unsafe-swiftshader",
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
  console.error(`swiftshader session did not finish: ${error}`);
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

const renderer = typeof report.renderer === "string" ? report.renderer : "";
const tier = typeof report.tier === "string" ? report.tier : "";
console.log(`renderer: ${renderer}`);
console.log(`tier: ${tier}`);

if (!renderer.includes("SwiftShader")) {
  console.error("renderer string does not contain SwiftShader");
  process.exit(3);
}
if (tier === "C") {
  process.exit(0);
}
console.error(`SwiftShader resolved tier ${tier}`);
process.exit(1);
