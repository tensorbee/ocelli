// F-245, part 2. A synthetic DICOMweb upstream is browsed from the viewer.
//   setup    an upstream serving QIDO-RS JSON and WADO-RS multipart, started
//            on a free port, with the viewer pointed at it by URL
//   refused  an instance whose stored SOP Instance UID contradicts the
//            requested instance is refused by the native identity check
//   local    the local synthetic image, an SR document read back through the
//            load path, marks, key images, checks and cache stay unchanged
//            throughout
//
// The remote path resolves no tier and adds no GPU path. The tier-independent
// observable asserted here is that no canvas appears anywhere on the page
// through the remote stages and the fixed unavailable status holds in the one
// browser this suite runs, while the local render stays byte-identical. All
// three tiers answer the same, per the plan's boundary section and
// docs/lld/feature-availability.md. The served More matrix row is snapshotted
// against overstated claims.
//
// The buffers are wholly synthetic Part 10 files written by this script, carry
// no patient attributes, and live only in memory. It starts the viewer with
// OCELLI_PORT and stops it by its PID, through tools/bench's pinned Playwright.
// bin/ocelli.sh does not yet register this suite under viewer-browser.
// Registration there is pending in another story's shared slice, so this
// script is run directly, after aspect.mjs.

import { Buffer } from "node:buffer";
import { mkdtempSync, rmSync } from "node:fs";
import { createServer } from "node:net";
import { join } from "node:path";
import { tmpdir } from "node:os";
import { fileURLToPath } from "node:url";

import { ensureViewerBinary, longEl, shortEl, startViewer } from "../../../../tools/bench/src/viewer_work.mjs";
import { loadPlaywright, stopByPid } from "./aspect.mjs";

// Synthetic DICOMweb upstream for browser tests.
// Serves QIDO-RS JSON search endpoints and a single-instance WADO-RS multipart retrieval.
// Every payload is synthetic and supplied entirely by the caller. Test-only, never production code.

import http from "node:http";

async function startSyntheticDicomweb({ origin, study, series, instances }) {
  const requests = [];
  const byUid = new Map();
  for (const instance of instances) {
    byUid.set(instance.uid, instance);
  }

  const server = http.createServer((req, res) => {
    const path = req.url ?? "/";
    const accept = req.headers.accept ?? "";
    requests.push({ method: req.method ?? "GET", path, accept });

    const setCors = () => {
      res.setHeader("Access-Control-Allow-Origin", origin);
    };

    if (req.method === "OPTIONS") {
      setCors();
      res.setHeader("Access-Control-Allow-Methods", "GET");
      res.setHeader("Access-Control-Allow-Headers", "accept");
      res.writeHead(204);
      res.end();
      return;
    }

    if (req.method !== "GET") {
      setCors();
      res.writeHead(404);
      res.end();
      return;
    }

    const [pathOnly] = path.split("?");
    const base = "/dicom-web/studies";

    if (pathOnly === base) {
      setCors();
      const body = JSON.stringify([
        { "0020000D": { vr: "UI", Value: [study] } },
      ]);
      res.writeHead(200, { "Content-Type": "application/dicom+json" });
      res.end(body);
      return;
    }

    if (pathOnly === `${base}/${study}/series`) {
      setCors();
      const body = JSON.stringify([
        {
          "0020000E": { vr: "UI", Value: [series] },
          "0020000D": { vr: "UI", Value: [study] },
        },
      ]);
      res.writeHead(200, { "Content-Type": "application/dicom+json" });
      res.end(body);
      return;
    }

    if (pathOnly === `${base}/${study}/series/${series}/instances`) {
      setCors();
      const body = JSON.stringify(
        instances.map((instance) => ({
          "00080018": { vr: "UI", Value: [instance.uid] },
          "0020000E": { vr: "UI", Value: [series] },
          "0020000D": { vr: "UI", Value: [study] },
        })),
      );
      res.writeHead(200, { "Content-Type": "application/dicom+json" });
      res.end(body);
      return;
    }

    const instancePrefix = `${base}/${study}/series/${series}/instances/`;
    if (pathOnly.startsWith(instancePrefix)) {
      const uid = decodeURIComponent(pathOnly.slice(instancePrefix.length));
      const instance = byUid.get(uid);
      if (instance !== undefined) {
        setCors();
        const contentType = `multipart/related; type="application/dicom"; boundary=${BOUNDARY}`;
        const part10 = Buffer.isBuffer(instance.part10) ? instance.part10 : Buffer.from(instance.part10);
        const body = Buffer.concat([
          Buffer.from(
            `--${BOUNDARY}\r\nContent-Type: application/dicom\r\nContent-Location: ${pathOnly}\r\nContent-Length: ${part10.length}\r\n\r\n`,
          ),
          part10,
          Buffer.from(`\r\n--${BOUNDARY}--`),
        ]);
        res.writeHead(200, {
          "Content-Type": contentType,
          "Content-Length": String(body.length),
        });
        res.end(body);
        return;
      }
    }

    setCors();
    res.writeHead(404);
    res.end();
  });

  await new Promise((resolve, reject) => {
    server.once("error", reject);
    server.listen(0, "127.0.0.1", () => {
      server.removeListener("error", reject);
      resolve(undefined);
    });
  });

  const address = server.address();
  const port = typeof address === "object" && address !== null ? address.port : 0;
  const url = `http://127.0.0.1:${port}/dicom-web`;

  async function close() {
    await new Promise((resolve, reject) => {
      server.close((error) => {
        if (error !== undefined && error !== null) {
          reject(error);
        } else {
          resolve(undefined);
        }
      });
    });
  }

  return { url, requests, close };
}

const BOUNDARY = "synthetic-dicomweb-boundary-f245";

function u16(value) {
  const buffer = Buffer.alloc(2);
  buffer.writeUInt16LE(value, 0);
  return buffer;
}

function u32(value) {
  const buffer = Buffer.alloc(4);
  buffer.writeUInt32LE(value, 0);
  return buffer;
}

function ui(value) {
  const padded = value.length % 2 === 0 ? value : `${value}\0`;
  return Buffer.from(padded, "ascii");
}

function cs(value) {
  const padded = value.length % 2 === 0 ? value : `${value} `;
  return Buffer.from(padded, "ascii");
}

function buildPart10(sopInstance, sopClass) {
  const metaElements = Buffer.concat([
    longEl(0x0002, 0x0001, "OB", Buffer.from([0x00, 0x01])),
    shortEl(0x0002, 0x0002, "UI", ui(sopClass)),
    shortEl(0x0002, 0x0003, "UI", ui(sopInstance)),
    shortEl(0x0002, 0x0010, "UI", ui("1.2.840.10008.1.2.1")),
    shortEl(0x0002, 0x0012, "UI", ui("2.25.216")),
  ]);
  const meta = Buffer.concat([
    shortEl(0x0002, 0x0000, "UL", u32(metaElements.length)),
    metaElements,
  ]);
  const dataset = Buffer.concat([
    shortEl(0x0008, 0x0016, "UI", ui(sopClass)),
    shortEl(0x0008, 0x0018, "UI", ui(sopInstance)),
    shortEl(0x0008, 0x0060, "CS", cs("OT")),
    shortEl(0x0020, 0x000d, "UI", ui("2.25.245001")),
    shortEl(0x0020, 0x000e, "UI", ui("2.25.245002")),
    shortEl(0x0028, 0x0002, "US", u16(1)),
    shortEl(0x0028, 0x0004, "CS", cs("MONOCHROME2")),
    shortEl(0x0028, 0x0010, "US", u16(2)),
    shortEl(0x0028, 0x0011, "US", u16(2)),
    shortEl(0x0028, 0x0100, "US", u16(16)),
    shortEl(0x0028, 0x0101, "US", u16(16)),
    shortEl(0x0028, 0x0102, "US", u16(15)),
    shortEl(0x0028, 0x0103, "US", u16(0)),
    longEl(0x7fe0, 0x0010, "OW", Buffer.alloc(8)),
  ]);
  return Buffer.concat([
    Buffer.alloc(128),
    Buffer.from("DICM", "ascii"),
    meta,
    dataset,
  ]);
}

async function freePort() {
  return new Promise((resolve, reject) => {
    const server = createServer();
    server.once("error", reject);
    server.listen(0, "127.0.0.1", () => {
      const address = server.address();
      const port = typeof address === "object" && address !== null ? address.port : 0;
      server.close(() => resolve(port));
    });
  });
}

async function readLocal(page) {
  return page.evaluate(async () => {
    const canvases = Array.from(document.querySelectorAll("#wrap canvas"));
    const visible = canvases.filter((canvas) => canvas.getClientRects().length > 0);
    const canvas = visible[0];
    const context = canvas.getContext("2d");
    const data = context.getImageData(0, 0, canvas.width, canvas.height).data;
    const copy = data.buffer.slice(data.byteOffset, data.byteOffset + data.byteLength);
    const digest = await crypto.subtle.digest("SHA-256", copy);
    const pixels = Array.from(new Uint8Array(digest), (byte) => byte.toString(16).padStart(2, "0")).join("");
    const wrapBox = document.querySelector("#wrap").getBoundingClientRect();
    const marks = Array.from(notes.values()).reduce((sum, list) => sum + list.length, 0);
    return {
      status: document.querySelector("#status")?.textContent ?? "",
      series: document.querySelectorAll("#series-list > *").length,
      marks,
      keys: keyImages.size,
      documents: sessionDocuments.size,
      cacheSize: cache.size,
      cacheGeneration: cacheGeneration,
      checksToken: checks.token,
      checksReports: checks.reports.size,
      canvases: document.querySelectorAll("canvas").length,
      wrap: { width: Math.round(wrapBox.width), height: Math.round(wrapBox.height) },
      pixels,
    };
  });
}

function sameLocal(before, after) {
  return before.status === after.status && before.series === after.series && before.marks === after.marks && before.keys === after.keys && before.documents === after.documents && before.cacheSize === after.cacheSize && before.cacheGeneration === after.cacheGeneration && before.checksToken === after.checksToken && before.checksReports === after.checksReports && before.canvases === after.canvases && before.wrap.width === after.wrap.width && before.wrap.height === after.wrap.height && before.pixels === after.pixels;
}

async function client(page, x, y) {
  return page.evaluate(([px, py]) => {
    const at = imageToClient(activeViewport(), { x: px, y: py });
    return [at.x, at.y];
  }, [x, y]);
}

async function drag(page, from, to) {
  const [x0, y0] = await client(page, from[0], from[1]);
  const [x1, y1] = await client(page, to[0], to[1]);
  await page.mouse.move(x0, y0);
  await page.mouse.down();
  await page.mouse.move((x0 + x1) / 2, (y0 + y1) / 2, { steps: 6 });
  await page.mouse.move(x1, y1, { steps: 6 });
  await page.mouse.up();
}

async function main() {
  const { chromium } = await loadPlaywright();
  const binary = await ensureViewerBinary("release", true);
  const port = await freePort();
  const viewerOrigin = `http://127.0.0.1:${port}`;
  const study = "2.25.245001";
  const series = "2.25.245002";
  const instanceA = "2.25.245003";
  const instanceB = "2.25.245004";
  const bytesA = buildPart10("2.25.245003", "1.2.840.10008.5.1.4.1.1.7");
  const bytesB = buildPart10("2.25.245099", "1.2.840.10008.5.1.4.1.1.7");
  const identityRanges = [];
  for (const bytes of [bytesA, bytesB]) {
    for (const uid of [ui("2.25.245003"), ui("2.25.245099")]) {
      let at = bytes.indexOf(uid);
      while (at >= 0) {
        identityRanges.push([at, at + uid.length]);
        at = bytes.indexOf(uid, at + 1);
      }
    }
  }
  if (bytesA.length !== bytesB.length) {
    throw new Error("the synthetic buffers differ in length");
  }
  for (let offset = 0; offset < bytesA.length; offset += 1) {
    if (bytesA[offset] !== bytesB[offset] && !identityRanges.some(([start, end]) => offset >= start && offset < end)) {
      throw new Error("the synthetic buffers differ outside the SOP Instance UID values");
    }
  }
  const upstream = await startSyntheticDicomweb({
    origin: viewerOrigin,
    study,
    series,
    instances: [
      { uid: instanceA, part10: bytesA },
      { uid: instanceB, part10: bytesB },
    ],
  });
  const server = await startViewer(binary, {
    OCELLI_PORT: String(port),
    OCELLI_VIEWER_STUDIES_SERVICE_URL: upstream.url,
    OCELLI_VIEWER_WADO_URI_URL: `${upstream.url}/wado`,
  });
  const browser = await chromium.launch();
  const downloads = mkdtempSync(join(tmpdir(), "f245-dicomweb-docs-"));
  const pageRequests = [];
  const problems = [];
  try {
    const page = await browser.newPage({ acceptDownloads: true });
    page.on("request", (request) => {
      const pathname = new URL(request.url()).pathname;
      pageRequests.push(`${request.method()} ${pathname}`);
    });

    await page.goto(viewerOrigin);
    await page.click("#source-synthetic");
    await page.waitForFunction(() => document.querySelectorAll("#series-list > *").length > 0);
    await page.waitForFunction(() => !(document.querySelector("#status")?.textContent ?? "").startsWith("Opening"));
    await page.waitForFunction(() => Boolean(activeViewport() && activeViewport().imageWidth && activeViewport().scale));
    await page.click("#tool-rect");
    await drag(page, [10, 10], [20, 20]);
    await page.click("#key-image");

    await page.waitForFunction(() => !checks.busy && checks.due === null && checks.reports.size > 0);

    // An SR document is loaded through the real read path before the remote
    // flow, so its preservation through remote stages is a counted claim and
    // not a zero against zero.
    const saved = [];
    const saving = [];
    page.on("download", (download) => {
      const path = join(downloads, download.suggestedFilename());
      saved.push(path);
      saving.push(download.saveAs(path).then(() => path));
    });
    await page.click("#save-marks");
    await page.waitForFunction(() => (document.getElementById("status").textContent || "").startsWith("Saved"));
    for (let wait = 0; wait < 50 && saved.length < 2; wait += 1) await page.waitForTimeout(100);
    if (saved.length < 2) {
      problems.push(`the save produced ${saved.length} documents, 2 expected`);
    }
    const paths = await Promise.all(saving);
    await page.locator("#marks-file").setInputFiles(paths);
    await page.waitForFunction(() => (document.getElementById("status").textContent || "").startsWith("Read "));

    const local = await readLocal(page);
    if (local.marks < 1 || local.keys < 1) {
      problems.push("the first local snapshot lacks a mark or a key image");
    }
    if (local.documents < 1) {
      problems.push("the first local snapshot has no loaded document");
    }
    if (local.canvases === 0) {
      problems.push("the page showed no canvas at the first local snapshot");
    }

    await page.waitForSelector("#source-dicomweb", { state: "visible" });
    await page.click("#source-dicomweb");
    await page.locator("#remote-list button", { hasText: study }).first().click();
    await page.locator("#remote-list button", { hasText: series }).first().click();
    const remoteOpen = await readLocal(page);
    if (!sameLocal(local, remoteOpen)) {
      problems.push("opening the remote panel changed the local image");
    }
    await page.locator("#remote-list button", { hasText: instanceB }).first().click();
    await page.waitForFunction(
      (prefix) => (document.querySelector("#remote-state")?.textContent ?? "").startsWith(prefix),
      "The DICOMweb request was refused.",
    );
    const afterRefusal = await readLocal(page);
    if (!sameLocal(local, afterRefusal)) {
      problems.push("the refused instance changed the local image");
    }

    await page.locator("button", { hasText: "Back to studies" }).first().click();
    await page.locator("#remote-list button", { hasText: study }).first().click();
    await page.locator("#remote-list button", { hasText: series }).first().click();
    const heldUrl = `${upstream.url}/studies/${study}/series/${series}/instances/${instanceA}`;
    let releaseFetch;
    const fetchHeld = new Promise((resolve) => {
      releaseFetch = resolve;
    });
    let handlerCount = 0;
    let handlerDone;
    const handlerSettled = new Promise((resolve) => {
      handlerDone = resolve;
    });
    await page.route(heldUrl, async (route) => {
      handlerCount += 1;
      await fetchHeld;
      const continued = route.continue();
      handlerDone(continued);
      await continued;
    });
    await page.locator("#remote-list button", { hasText: instanceA }).first().click();
    await page.waitForFunction(
      (expected) => (document.querySelector("#remote-state")?.textContent ?? "") === expected,
      "Fetching the instance.",
    );
    const pendingLocal = await readLocal(page);
    if (!sameLocal(local, pendingLocal)) {
      problems.push("the pending fetch changed the local state");
    }
    if (handlerCount !== 1) {
      problems.push(`the route handler count at the pending snapshot was ${handlerCount}, expected 1`);
    }
    const pendingInstanceARequest = upstream.requests.find(
      (request) => request.method === "GET" && request.path.split("?")[0] === `/dicom-web/studies/${study}/series/${series}/instances/${instanceA}`
    );
    if (pendingInstanceARequest) {
      problems.push("the upstream already served instance A at the pending snapshot");
    }
    releaseFetch();
    await handlerSettled;
    await page.unroute(heldUrl);
    await page.waitForFunction(
      (expected) => (document.querySelector("#remote-state")?.textContent ?? "") === expected,
      "The remote instance is fetched and validated. Remote image display is not available yet. The local image is unchanged.",
    );
    const afterFetch = await readLocal(page);
    if (!sameLocal(local, afterFetch)) {
      problems.push("the fetched instance changed the local image");
    }
    const shownAfterFetch = await page.evaluate(() => document.querySelectorAll("canvas").length);
    if (shownAfterFetch !== local.canvases) {
      problems.push(`the page held ${shownAfterFetch} canvases after the validated fetch, ${local.canvases} before, so a display appeared`);
    }

    const got = (method, path) => upstream.requests.some((request) => request.method === method && request.path.split('?')[0] === path);
    if (!got("GET", "/dicom-web/studies")) problems.push("the upstream saw no studies request");
    if (!got("GET", `/dicom-web/studies/${study}/series`)) problems.push("the upstream saw no series list request");
    if (!got("GET", `/dicom-web/studies/${study}/series/${series}/instances`)) problems.push("the upstream saw no instance list request");
    if (!got("GET", `/dicom-web/studies/${study}/series/${series}/instances/${instanceA}`)) problems.push("the upstream saw no instance A request");
    if (!got("GET", `/dicom-web/studies/${study}/series/${series}/instances/${instanceB}`)) problems.push("the upstream saw no instance B request");
    const qidoCount = pageRequests.filter((request) => request === "POST /dicweb/qido").length;
    if (qidoCount < 3) problems.push(`the page made ${qidoCount} POST /dicweb/qido requests, at least 3 expected`);
    const instanceCount = pageRequests.filter((request) => request === "POST /dicweb/instance").length;
    if (instanceCount < 2) problems.push(`the page made ${instanceCount} POST /dicweb/instance requests, at least 2 expected`);

    // The served More matrix row is a snapshot. It must state the actual
    // partial status and must not claim remote display.
    const more = await page.evaluate(async () => {
      const response = await fetch("/more", { cache: "no-store" });
      return response.ok ? response.text() : "";
    });
    const row = '<td><span class="viewer-state part">Partial</span>The viewer searches studies, series and instances over DICOMweb and fetches one instance the native backend validates, remote image display is not available yet, and local folders open as before</td>';
    if (!more.includes(row)) {
      problems.push("the served More matrix does not carry the truthful DICOMweb sources row");
    }

    if (problems.length) {
      for (const problem of problems) console.error(problem);
      process.exitCode = 1;
      return;
    }
    console.log("ok");
  } finally {
    await browser.close();
    await stopByPid(server.pid);
    await upstream.close();
    rmSync(downloads, { recursive: true, force: true });
  }
}

if (process.argv[1] === fileURLToPath(import.meta.url)) {
  main().catch((error) => {
    console.error(error);
    process.exitCode = 1;
  });
}