// F-228 AC13 to AC16. Run explicitly after building @ocelli/core.
// This uses the release viewer, the pinned Chromium and synthetic corpus rows.
import assert from "node:assert/strict";
import { createHash } from "node:crypto";
import { copyFileSync, existsSync, mkdirSync, mkdtempSync, readFileSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";

import { ensureViewerBinary, startViewer } from "../../tools/bench/src/viewer_work.mjs";
import { loadPlaywright, stopByPid } from "./tests/browser/aspect.mjs";

const ROOT = fileURLToPath(new URL("../../", import.meta.url));
const ROWS = ["j2k_lossless", "htj2k_lossless", "htj2k_lossless_rpcl", "jpeg_lossless_p14", "jpeg_lossless_p14_sv1", "jpegls_lossless", "rle_lossless"];
const ROUTES = [["/", "/index.html"], ["/use", "/use.html"], ["/more", "/more.html"]];
const hash = (text) => `'sha256-${createHash("sha256").update(text, "utf8").digest("base64")}'`;

// Parse inert HTML, including template subtrees. Runtime style properties are
// deliberately outside this check of the server's static source.
async function inspectHTML(page, html) {
  assert(!html.includes("\r"), "static HTML contains a carriage return");
  return page.evaluate((source) => {
    const doc = new DOMParser().parseFromString(source, "text/html");
    const scripts = [];
    const styles = [];
    const bad = [];
    let importmaps = 0;
    function visit(root) {
      for (const node of root.querySelectorAll("*")) {
        for (const attribute of node.attributes) {
          if (attribute.name === "style" || /^on[a-z]/i.test(attribute.name)) bad.push(`${node.localName}[${attribute.name}]`);
        }
        if (node.localName === "script" && !node.hasAttribute("src")) {
          scripts.push(node.textContent);
          if (node.getAttribute("type") === "importmap") importmaps += 1;
        }
        if (node.localName === "style") styles.push(node.textContent);
        if (node.localName === "template") visit(node.content);
      }
    }
    visit(doc);
    return { scripts, styles, bad, importmaps };
  }, html);
}

export function assertPolicy(header, elements) {
  assert(header, "HTML response has no Content-Security-Policy header");
  const parsed = new Map();
  for (const directive of header.split(";")) {
    const [name, ...sources] = directive.trim().split(/\s+/);
    if (!name) continue;
    assert(!parsed.has(name), `duplicate policy directive ${name}`);
    assert.equal(new Set(sources).size, sources.length, `duplicate source in ${name}`);
    parsed.set(name, sources.sort());
  }
  const expected = new Map([
    ["default-src", ["'none'"]],
    ["script-src", ["'self'", ...elements.scripts.map(hash)]],
    ["style-src", ["'self'", ...elements.styles.map(hash)]],
    ["img-src", ["'self'", "data:"]],
    ["connect-src", ["'self'"]],
    ["base-uri", ["'none'"]],
    ["form-action", ["'none'"]],
    ["frame-ancestors", ["'none'"]],
  ]);
  assert.deepEqual([...parsed.keys()].sort(), [...expected.keys()].sort(), "policy directive set");
  for (const [name, sources] of expected) assert.deepEqual(parsed.get(name), [...new Set(sources)].sort(), name);
  assert(!header.includes("wasm-unsafe-eval"), "folder policy allows wasm compilation");
}

async function monitoredPage(browser) {
  const page = await browser.newPage({ viewport: { width: 1400, height: 900 }, acceptDownloads: true });
  page.setDefaultTimeout(30000);
  const errors = [];
  const requests = new Set();
  page.on("pageerror", (error) => errors.push(error.message));
  page.on("console", (message) => {
    if (/content security policy|violates.*(?:script-src|style-src|connect-src)|refused to (?:execute|load|apply)/i.test(message.text())) errors.push(message.text());
  });
  page.on("request", (request) => requests.add(new URL(request.url()).pathname));
  await page.addInitScript(() => {
    window.__cspViolations = [];
    window.__cspPaints = [];
    document.addEventListener("securitypolicyviolation", (event) => {
      window.__cspViolations.push({ directive: event.effectiveDirective, uri: event.blockedURI });
    });
    const put = CanvasRenderingContext2D.prototype.putImageData;
    CanvasRenderingContext2D.prototype.putImageData = function (...args) {
      const result = put.apply(this, args);
      if (this.canvas.matches(".cell.active canvas.view")) {
        window.__cspPaints.push({ generation: currentGeneration, width: this.canvas.width, height: this.canvas.height });
      }
      return result;
    };
  });
  return {
    page, requests,
    async clean(label) {
      assert.deepEqual(errors, [], `${label}: CSP console or page errors`);
      assert.deepEqual(await page.evaluate(() => window.__cspViolations), [], `${label}: policy violations`);
    },
  };
}

async function dragImage(page, from, to) {
  const coordinates = await page.evaluate(([a, b]) => [a, b].map(([x, y]) => {
    const point = imageToClient(activeViewport(), { x, y });
    return [point.x, point.y];
  }), [from, to]);
  await page.mouse.move(...coordinates[0]);
  await page.mouse.down();
  await page.mouse.move(...coordinates[1], { steps: 12 });
  await page.mouse.up();
}

async function openImage(page, folder, filename) {
  const generation = await page.evaluate(() => currentGeneration);
  await page.locator("#folder").setInputFiles(folder);
  await page.waitForFunction(({ previous, name }) => {
    const vp = activeViewport();
    const image = currentImage(vp);
    return currentGeneration > previous && image && vp.scale > 0 && image.file.name === name &&
      window.__cspPaints.some((paint) => paint.generation === currentGeneration && paint.width === vp.imageWidth && paint.height === vp.imageHeight);
  }, { previous: generation, name: filename });
  return page.evaluate(() => {
    const canvas = document.querySelector(".cell.active canvas.view");
    return { width: canvas.width, height: canvas.height, rgba: Array.from(canvas.getContext("2d").getImageData(0, 0, canvas.width, canvas.height).data) };
  });
}

async function synthetic(page) {
  await page.locator("#source-synthetic").click();
  await page.waitForFunction(() => document.getElementById("status").textContent.startsWith("Synthetic study, not a patient. 1 series.") && activeViewport().imageWidth && activeViewport().scale);
}

export async function probe(url, chromium, scratch) {
  const browser = await chromium.launch({ headless: true });
  const passed = [];
  try {
    const parser = await browser.newPage();
    const hygieneControl = await inspectHTML(parser, '<template><div STYLE = "color:red" OnClick = "noop()"></div></template><svg onload="noop()"></svg>');
    assert.deepEqual(hygieneControl.bad.sort(), ["div[onclick]", "div[style]", "svg[onload]"], "inert parser detects varied attribute grammar and template contents");
    const pages = new Map();
    for (const pair of ROUTES) {
      for (const route of pair) {
        const response = await fetch(new URL(route, url));
        assert.equal(response.status, 200, route);
        const html = await response.text();
        const elements = await inspectHTML(parser, html);
        assert.deepEqual(elements.bad, [], `${route}: static style or event attributes`);
        assertPolicy(response.headers.get("content-security-policy"), elements);
        if (route === "/") assert.equal(elements.importmaps, 1, "main page import map is hashed");
        pages.set(route, html);
      }
      assert.equal(pages.get(pair[0]), pages.get(pair[1]), `${pair[0]} alias source`);
    }
    await parser.close();
    for (const [route] of ROUTES) {
      const observed = await monitoredPage(browser);
      try {
        await observed.page.goto(new URL(route, url).href);
        if (route === "/") await observed.page.waitForFunction(() => typeof showIndex === "function");
        else assert((await observed.page.locator("body").innerText()).trim().length > 0, `${route}: page content`);
        await observed.clean(route);
      } finally { await observed.page.close(); }
    }
    passed.push("pages: six exact headers and static DOM hygiene, three pages loaded");

    const tampered = await browser.newPage();
    try {
      await tampered.addInitScript(() => {
        window.__blocked = [];
        document.addEventListener("securitypolicyviolation", (event) => window.__blocked.push({ directive: event.effectiveDirective, uri: event.blockedURI }));
      });
      await tampered.route(new URL("/", url).href, async (route) => {
        const response = await route.fetch();
        const source = await response.text();
        let changed = 0;
        const body = source.replace(/(<script\b[^>]*>)([\s\S]*?)(<\/script\s*>)/gi, (whole, open, text, close) => {
          if (/\bsrc\s*=|\btype\s*=\s*["']importmap["']/i.test(open)) return whole;
          changed += 1;
          return `${open} ${text}${close}`;
        });
        assert.equal(changed, 1, "one main script changed, original response header retained");
        await route.fulfill({ response, body });
      });
      await tampered.goto(url);
      await tampered.waitForFunction(() => window.__blocked.some((event) => event.directive === "script-src-elem" && event.uri === "inline"));
      assert.equal(await tampered.evaluate(() => typeof showIndex), "undefined", "tampered main script did not boot");
      passed.push("tampered: changed inline script blocked under original header");
    } finally { await tampered.close(); }

    const observed = await monitoredPage(browser);
    const { page } = observed;
    try {
      await page.goto(url);
      const inputs = new Map();
      for (const row of ["explicit_vr_le", ...ROWS]) {
        const source = join(ROOT, "corpus/data/syntax", `${row}.dcm`);
        assert(existsSync(source), `compressed: required synthetic corpus row absent: ${row}, AC14 not run`);
        const directory = join(scratch, row);
        mkdirSync(directory);
        copyFileSync(source, join(directory, `${row}.dcm`));
        inputs.set(row, directory);
      }
      const native = await openImage(page, inputs.get("explicit_vr_le"), "explicit_vr_le.dcm");
      assert(native.width > 1 && native.height > 1 && new Set(native.rgba.filter((_, at) => at % 4 === 0)).size > 1, "native control has actual varied pixels");
      for (const row of ROWS) {
        const decoded = await openImage(page, inputs.get(row), `${row}.dcm`);
        assert.deepEqual(decoded, native, `${row}: actual backing-canvas RGBA differs from native`);
        await observed.clean(row);
      }
      passed.push(`compressed: ${ROWS.length} lossless rows equal native backing-canvas RGBA`);

      for (const library of ["/lib/ocelli-core/index.js", "/lib/dcmjs.js"]) {
        const response = await fetch(new URL(library, url));
        assert(response.ok, `library absent at ${library}, AC15 not run`);
      }
      await openImage(page, inputs.get("explicit_vr_le"), "explicit_vr_le.dcm");
      const before = await page.evaluate(() => ({ center: dragWindow(activeViewport()).center, width: dragWindow(activeViewport()).width }));
      const paintsBeforeRegion = await page.evaluate(() => window.__cspPaints.length);
      await page.locator("#tool-wlregion").click();
      await dragImage(page, [native.width / 4, native.height / 4], [native.width * 3 / 4, native.height * 3 / 4]);
      await page.waitForFunction((beforePaints) => activeViewport().overrideWindow?.from === "region" && window.__cspPaints.length > beforePaints, paintsBeforeRegion);
      const region = await page.evaluate(() => ({ center: activeViewport().overrideWindow.center, width: activeViewport().overrideWindow.width }));
      assert.notDeepEqual(region, before, "region import actually changes the window");
      const regionPixels = await page.evaluate(() => {
        const canvas = document.querySelector(".cell.active canvas.view");
        return Array.from(canvas.getContext("2d").getImageData(0, 0, canvas.width, canvas.height).data);
      });
      assert.notDeepEqual(regionPixels, native.rgba, "region window changed the painted pixels");
      assert(observed.requests.has("/lib/ocelli-core/index.js"), "region did not import core through /lib");
      await observed.clean("region");
      passed.push("region: import-map library set a region window");

      await page.reload();
      await synthetic(page);
      await page.locator("#tool-rect").click();
      await dragImage(page, [20, 20], [40, 40]);
      await page.waitForFunction(() => [...notes.values()].flat().length === 1);
      const original = await page.evaluate(() => [...notes.values()].flat().map((note) => ({ type: note.type, points: note.points.map(({ x, y }) => ({ x, y })) })));
      const downloading = page.waitForEvent("download");
      await page.locator("#save-marks").click();
      const download = await downloading;
      const saved = join(scratch, "marks.dcm");
      await download.saveAs(saved);
      assert.equal(await download.failure(), null, "SR download completed");
      const bytes = readFileSync(saved);
      assert(bytes.length > 132 && bytes.subarray(128, 132).toString("ascii") === "DICM", "download is a DICOM Part10 document");
      await page.waitForFunction(() => document.getElementById("status").textContent.startsWith("Saved 1 mark and 0 key images in 1 file."));
      await observed.clean("save");
      await page.reload();
      await synthetic(page);
      assert.equal(await page.evaluate(() => [...notes.values()].flat().length), 0, "fresh page before SR read");
      await page.locator("#marks-file").setInputFiles(saved);
      await page.waitForFunction(() => document.getElementById("status").textContent.startsWith("Read 1 document: 1 mark and 0 key images placed."));
      const loaded = await page.evaluate(() => [...notes.values()].flat().map((note) => ({ type: note.type, points: note.points.map(({ x, y }) => ({ x, y })) })));
      assert.equal(loaded.length, original.length, "SR mark count");
      assert.equal(loaded[0].type, original[0].type, "SR region type");
      assert.equal(loaded[0].points.length, original[0].points.length, "SR point count");
      // SCOORD is FL. These integer control coordinates must round-trip to
      // within 1e-4 image pixels, independent of canvas scaling.
      loaded[0].points.forEach((point, at) => {
        for (const axis of ["x", "y"]) assert(Math.abs(point[axis] - original[0].points[at][axis]) < 1e-4, `SR ${axis} coordinate`);
      });
      await observed.clean("SR load");
      passed.push("save: actual TID1500 download loaded back with matching rectangle");
    } finally { await page.close(); }
    passed.push("no-wasm-unsafe-eval: all positive cases used the exact folder policy");
    return { chromium: browser.version(), passed };
  } finally { await browser.close(); }
}

async function main() {
  const scratch = mkdtempSync(join(tmpdir(), "ocelli-folder-csp-"));
  let server;
  try {
    const { chromium } = await loadPlaywright();
    const binary = await ensureViewerBinary("release", true);
    server = await startViewer(binary);
    const result = await probe(server.url, chromium, scratch);
    for (const sentence of result.passed) console.log(`PASS ${sentence}`);
    console.log(`Chromium ${result.chromium}`);
  } finally {
    if (server) await stopByPid(server.pid);
    rmSync(scratch, { recursive: true, force: true });
  }
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
  main().catch((error) => { console.error(error.message); process.exitCode = 1; });
}
