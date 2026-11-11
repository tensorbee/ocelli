/**
 * F-228, the built example viewer under the embedding policy, measured.
 *
 * The policy cases run on the invented origins http://ocelli.test and
 * http://dicomweb.test through page.route, so they bind no port and start no
 * server. The unmodified-dist positive runs on a real plain static HTTP
 * server instead: it serves only files under the emitted dist, answers every
 * ordinary well-formed, decodable unknown path, including any /@fs reach, with
 * a plain 404, and never maps a
 * repository path, never intercepts a request and never prepends worker
 * bytes. That is F-266's genuine positive; the policy cases remain the
 * F-228 evidence and keep their own instrumentation.
 *
 * Each case is one page load under one variation of the policy, and each
 * case asserts its own named failure, not only the absence of success.
 * Chromium is the acceptance browser, plan item 10. Firefox and WebKit are
 * not run here and are not claimed.
 *
 * The policy exists once, in src/csp-policy.ts. This probe imports it under
 * Node 24's type stripping and never carries a copy. Each variation header
 * names its change. The blob cases are the exception: the legacy failure is
 * `blob:` allowed while 'wasm-unsafe-eval' is absent, so the relaxed page
 * varies the two together, and its positive control restores
 * 'wasm-unsafe-eval' while `blob:` stays allowed.
 *
 * The policy cases' served worker chunk carries a small collector prelude.
 * A dedicated worker's securitypolicyviolation events stay in the worker,
 * because addInitScript does not run there, and Chromium does not forward
 * the browser's own CSP console text out of a worker, so the violation
 * listener is the channel that carries that evidence. The worker script's
 * own console calls do reach page.on("console") whatever the method, and
 * the collector wraps every console method the worker global exposes, so
 * the strict case asserts the full collected message set from the page and
 * the worker empty. The prelude is probe instrumentation on the policy
 * cases and is not part of the unmodified positive, which serves the real
 * dist bytes unchanged.
 *
 * Build first:
 *
 *     bin/ocelli.sh wasm
 *     npm run build -w @ocelli/example-viewer
 *
 * Run:
 *
 *     node examples/viewer-react/csp-probe.mjs
 */

import { existsSync, readFileSync, statSync } from "node:fs";
import { createHash } from "node:crypto";
import { Buffer } from "node:buffer";
import { createRequire } from "node:module";
import { basename, dirname, isAbsolute, join, relative } from "node:path";
import { fileURLToPath } from "node:url";
import assert from "node:assert/strict";
import { createServer } from "node:http";

const require = createRequire(import.meta.url);
const { chromium } = require("../../tools/oracle/node_modules/playwright");

const HERE = dirname(fileURLToPath(import.meta.url));
const ROOT = join(HERE, "..", "..");
const DIST = join(ROOT, "examples", "viewer-react", "dist");
const PKG = join(ROOT, "crates", "ocelli-wasm", "pkg");
const PAGE = "http://ocelli.test";
const DICOMWEB = "http://dicomweb.test";
const INSTANCE = join(ROOT, "corpus", "data", "syntax", "rle_lossless.dcm");

let policyHeader;
try {
  ({ policyHeader } = await import("./src/csp-policy.ts"));
} catch {
  globalThis.console.error(
    "csp-probe: Node 24 type stripping could not read src/csp-policy.ts. Run the probe with Node 24 or newer, which imports TypeScript directly.",
  );
  globalThis.process.exit(1);
}

for (const needed of [
  join(DIST, "index.html"),
  join(DIST, "assets"),
  join(PKG, "ocelli_wasm.js"),
  join(PKG, "ocelli_wasm_bg.wasm"),
]) {
  if (!existsSync(needed)) {
    globalThis.console.error(
      `csp-probe: ${needed} is absent. Build the release wasm package and the example first: bin/ocelli.sh wasm, then npm run build -w @ocelli/example-viewer.`,
    );
    globalThis.process.exit(1);
  }
}

/** Remove one named source from a header, refusing if it was not there. */
function without(header, source) {
  const removed = header.replace(` ${source}`, "");
  if (removed === header) {
    throw new Error(
      `the probe could not remove ${source} from the header it starts from`,
    );
  }
  return removed;
}

/** Replace one directive's sources, refusing if the directive is absent. */
function withDirective(header, directive, sources) {
  let changed = false;
  const next = header
    .split("; ")
    .map((part) => {
      if (part.startsWith(`${directive} `)) {
        changed = true;
        return `${directive} ${sources}`;
      }
      return part;
    })
    .join("; ");
  if (!changed) {
    throw new Error(
      `the probe could not find ${directive} in the header it starts from`,
    );
  }
  return next;
}

/** Remove one directive entirely, refusing if the directive is absent. */
function withoutDirective(header, directive) {
  let changed = false;
  const next = header
    .split("; ")
    .filter((part) => {
      if (part.startsWith(`${directive} `)) {
        changed = true;
        return false;
      }
      return true;
    })
    .join("; ");
  if (!changed) {
    throw new Error(
      `the probe could not find ${directive} in the header it starts from`,
    );
  }
  return next;
}

const STRICT = policyHeader();
const WITHOUT_WASM_EVAL = without(STRICT, "'wasm-unsafe-eval'");
const NO_WORKER = withDirective(STRICT, "worker-src", "'none'");
const NO_CONNECT = withDirective(STRICT, "connect-src", "'none'");
const NO_SCRIPT = withDirective(STRICT, "script-src", "'none'");
// The pin removed but nothing refused: worker-src falls back to script-src.
const NO_WORKER_PIN = withoutDirective(STRICT, "worker-src");
// The blob worker's positive control: construction and compile both allowed.
const BLOB_ALLOWED = withDirective(STRICT, "worker-src", "'self' blob:");
const RELAXED_BLOB = withDirective(
  WITHOUT_WASM_EVAL,
  "worker-src",
  "'self' blob:",
);
const DICOMWEB_ALLOWED = policyHeader([DICOMWEB]);

const MIME = {
  html: "text/html",
  js: "text/javascript",
  css: "text/css",
  wasm: "application/wasm",
  svg: "image/svg+xml",
  json: "application/json",
  map: "application/json",
};

function contentType(path) {
  const extension = path.slice(path.lastIndexOf(".") + 1).toLowerCase();
  return MIME[extension] ?? "application/octet-stream";
}

/**
 * Map a request on the page origin to a file on this machine. /@fs reaches
 * files under the repository root only. Returns null for anything else.
 */
function fileUnderRoot(url) {
  const parsed = new globalThis.URL(url);
  if (parsed.origin !== PAGE) {
    return null;
  }
  const path = parsed.pathname;
  let file = null;
  if (path === "/" || path === "/index.html") {
    file = join(DIST, "index.html");
  } else if (path.startsWith("/assets/")) {
    file = join(DIST, "assets", basename(path));
  } else if (path.startsWith("/@fs/")) {
    // Strip "/@fs" only, so the absolute path keeps its leading slash.
    const requested = decodeURIComponent(path.slice("/@fs".length));
    const inRoot = relative(ROOT, requested);
    if (inRoot === "" || inRoot.startsWith("..") || isAbsolute(inRoot)) {
      return { refused: true };
    }
    file = requested;
  }
  if (file === null || !existsSync(file)) {
    return null;
  }
  return { file, type: contentType(file) };
}

/**
 * One WADO-RS instance response, CORS headers included, wrapping the bytes
 * of the corpus row in the multipart content type packages/core accepts.
 */
function dicomwebResponse(url) {
  const parsed = new globalThis.URL(url);
  if (parsed.origin !== DICOMWEB) {
    return null;
  }
  const boundary = "ocelliprobe7ae0d1fe";
  const body = Buffer.concat([
    Buffer.from(`--${boundary}\r\nContent-Type: application/dicom\r\n\r\n`, "utf8"),
    readFileSync(INSTANCE),
    Buffer.from(`\r\n--${boundary}--\r\n`, "utf8"),
  ]);
  return {
    status: 200,
    contentType: `multipart/related; type="application/dicom"; boundary=${boundary}`,
    headers: { "Access-Control-Allow-Origin": "*" },
    body,
  };
}

const BLOB_PAGE = `<!doctype html>
<html lang="en">
<head>
<meta charset="utf-8">
<title>blob worker control</title>
<script src="/__probe/blob-script.js" type="module"></script>
</head>
<body><p id="out">pending</p></body>
</html>
`;

const BLOB_SCRIPT = `const violations = [];
const out = document.getElementById("out");
document.addEventListener("securitypolicyviolation", (event) => {
  violations.push(event.effectiveDirective + " " + event.blockedURI);
});
const WORKER_SOURCE = [
  "self.onmessage = async () => {",
  "  try {",
  "    await WebAssembly.compile(new Uint8Array([0, 97, 115, 109, 1, 0, 0, 0]));",
  '    self.postMessage("compiled");',
  "  } catch (error) {",
  '    self.postMessage("error: " + error.name);',
  "  }",
  "};",
].join("\\n");
try {
  const url = URL.createObjectURL(new Blob([WORKER_SOURCE], { type: "text/javascript" }));
  const worker = new Worker(url, { type: "module" });
  worker.onmessage = (event) => {
    out.textContent = "result: " + event.data + " | " + violations.join(" | ");
  };
  worker.onerror = () => {
    out.textContent = "worker-error | " + violations.join(" | ");
  };
  worker.postMessage("go");
} catch (error) {
  // A refused construction throws in some engines. The violation event is
  // queued as a task, so let it land before writing.
  setTimeout(() => {
    out.textContent = "refused: " + error.name + " | " + violations.join(" | ");
  }, 200);
}
setTimeout(() => {
  if (out.textContent === "pending") {
    out.textContent = "timeout | " + violations.join(" | ");
  }
}, 10000);
`;

/**
 * Instrumentation served ahead of the worker chunk. A dedicated worker's
 * securitypolicyviolation events stay in the worker, and the browser's own
 * CSP console text from a worker reaches neither page.on("console") nor the
 * collector, so the violation listener is what records the refusals where
 * the worker.evaluate of workerProbe can read them. The console wrapper
 * records every console method call made in the worker. It changes no
 * policy: the header on the response is unchanged.
 */
const WORKER_PRELUDE = `;
self.__probeCsp = { violations: [], console: [] };
self.addEventListener("securitypolicyviolation", (event) => {
  self.__probeCsp.violations.push(
    event.effectiveDirective + " " + event.blockedURI,
  );
});
for (const probeMethod of Object.getOwnPropertyNames(console)) {
  if (typeof console[probeMethod] !== "function") {
    continue;
  }
  try {
    const probeOriginal = console[probeMethod].bind(console);
    console[probeMethod] = (...parts) => {
      self.__probeCsp.console.push(
        probeMethod + ": " + parts.map((part) => String(part)).join(" "),
      );
      probeOriginal(...parts);
    };
  } catch {
  }
}
`;

function respond(url) {
  const parsed = new globalThis.URL(url);
  if (parsed.origin === DICOMWEB) {
    return dicomwebResponse(parsed);
  }
  if (parsed.origin === PAGE) {
    if (parsed.pathname === "/__probe/blob-page.html") {
      return { status: 200, contentType: "text/html", body: BLOB_PAGE };
    }
    if (parsed.pathname === "/__probe/blob-script.js") {
      return { status: 200, contentType: "text/javascript", body: BLOB_SCRIPT };
    }
  }
  const local = fileUnderRoot(url);
  if (local === null) {
    return null;
  }
  if (local.refused === true) {
    return {
      status: 403,
      contentType: "text/plain",
      body: "csp-probe: outside the repository root",
    };
  }
  const body = readFileSync(local.file);
  if (isWorkerChunk(url)) {
    return {
      status: 200,
      contentType: local.type,
      body: Buffer.concat([Buffer.from(WORKER_PRELUDE, "utf8"), body]),
    };
  }
  return { status: 200, contentType: local.type, body };
}

function serveWith(headerFor, routed) {
  return async (route) => {
    const url = route.request().url();
    const header = headerFor(url);
    const response = respond(url);
    if (routed !== undefined) {
      routed.push({ url, csp: header });
    }
    const headers =
      header === null ? (response?.headers ?? {}) : {
        ...(response?.headers ?? {}),
        "Content-Security-Policy": header,
      };
    if (response === null) {
      return route.fulfill({
        status: 404,
        contentType: "text/plain",
        headers,
        body: "csp-probe: not found",
      });
    }
    return route.fulfill({ ...response, headers });
  };
}

function isWorkerChunk(url) {
  return /\/assets\/core-worker-.+\.js$/.test(new globalThis.URL(url).pathname);
}

/**
 * The real plain static server for the unmodified positive. It serves files
 * under the emitted dist only, answers every ordinary well-formed, decodable
 * unknown path with a plain 404, and keeps a log of every request with its
 * response status and the sha256
 * of the served bytes. There is no repository or /@fs mapping, no fallback
 * to index.html, no request interception and no worker prefix.
 */
const STATIC_MIME = {
  ".html": "text/html",
  ".js": "text/javascript",
  ".css": "text/css",
  ".wasm": "application/wasm",
  ".svg": "image/svg+xml",
  ".json": "application/json",
  ".map": "application/json",
};

const staticLog = [];

function startStaticServer() {
  return new Promise((resolve) => {
    const server = createServer((req, res) => {
      const url = new globalThis.URL(req.url ?? "/", "http://static.test");
      let path = decodeURIComponent(url.pathname);
      if (path === "/") {
        path = "/index.html";
      }
      const file = join(DIST, path);
      // The header is recorded at writeHead time. Reading it back with
      // res.getHeader in the same tick does not reflect what writeHead was
      // passed, which is the observation bug the first policied run exposed.
      const policy = STRICT;
      const finish = (status, type, bytes) => {
        // The genuine positive carries the existing STRICT policy. The
        // document and every emitted asset are served with the same header
        // the policy cases measure, so the positive proves the policy's
        // allowed path, not an unpolicied load. The bodies stay the exact
        // unmodified dist bytes.
        res.writeHead(status, type === null
          ? { "Content-Security-Policy": policy }
          : {
              "Content-Type": type,
              "Content-Security-Policy": policy,
            });
        res.end(bytes);
        // requestPath is the URL as requested (`/` for the document); served
        // is the dist-relative path that answered it. The two differ for the
        // document, so the assertion keys on the resolved name.
        staticLog.push({
          url: req.url ?? "/",
          served: path,
          status,
          policy,
          digest: bytes === undefined ? null : createHash("sha256").update(bytes).digest("hex"),
        });
      };
      if (
        !file.startsWith(DIST) || !existsSync(file) ||
        !file.startsWith(`${DIST}/`)
      ) {
        finish(404, "text/plain", Buffer.from("csp-probe: not found"));
        return;
      }
      if (!statSync(file).isFile()) {
        finish(404, "text/plain", Buffer.from("csp-probe: not found"));
        return;
      }
      const type = STATIC_MIME[file.slice(file.lastIndexOf("."))] ?? null;
      finish(200, type, readFileSync(file));
    });
    server.listen(0, "127.0.0.1", () => {
      resolve({
        base: `http://127.0.0.1:${server.address().port}`,
        close: () => server.close(),
      });
    });
  });
}

/**
 * The unmodified positive over real plain HTTP. Waits for the settled
 * post-status state, not the initial not-built render, and records what the
 * server actually served.
 */
async function realServerPositive(browser) {
  const { base, close } = await startStaticServer();
  try {
    const context = await browser.newContext();
    const page = await context.newPage();
    const errors = [];
    page.on("pageerror", (error) => errors.push(error.message));
    await page.goto(`${base}/`, { waitUntil: "load", timeout: 30000 });
    // The header reads "core not built" before the worker posts anything.
    // The settled state is the header after the status message or a tier
    // paragraph, whichever lands. A fixed early read races the worker.
    await page.waitForFunction(
      () => {
        const header =
          document.querySelector("header span")?.textContent ?? "";
        if (header === "core ready") {
          return true;
        }
        return [...document.querySelectorAll("p")].some((p) =>
          (p.textContent ?? "").startsWith("Tier "),
        );
      },
      undefined,
      { timeout: 30000 },
    );
    const state = await page.evaluate(() => {
      const paragraphs = [...document.querySelectorAll("p")].map(
        (p) => p.textContent ?? "",
      );
      return {
        header: document.querySelector("header span")?.textContent ?? null,
        tier: paragraphs.find((text) => text.startsWith("Tier "))?.slice(5) ??
          null,
      };
    });
    await context.close();
    // What the server actually served for the document, worker, glue and
    // wasm, with the header policy each response carried.
    const served = staticLog.filter((entry) =>
      /core-worker-|ocelli_wasm|index\.html|index-/.test(new globalThis.URL(entry.url, base).pathname));
    const absolute = staticLog.filter((entry) =>
      new globalThis.URL(entry.url, base).pathname.startsWith("/@fs"));
    // Every 200 response carried the strict policy, on the document, the
    // worker chunk and the assets alike. The 404s carry it too, but the
    // assertion is about what the run was served.
    const served200 = staticLog.filter((entry) => entry.status === 200);
    assert.equal(
      served200.length,
      5,
      `the run served the document, the entry chunk, the worker chunk, the glue chunk and the module asset, got ${served200.length} responses: ${served200.map((s) => s.url).join(", ")}`,
    );
    // Independent actual received-header observation: finite awaited GETs of
    // the exact recorded URLs, reading the received response headers, not the
    // server's staticLog.policy and not getHeader on the server side. These
    // requests append to staticLog but every later assertion uses the arrays
    // captured above, so the original browser's five 200 responses stay the
    // counted snapshot.
    for (const entry of served200) {
      const response = await fetch(`${base}${entry.url}`);
      const observed = response.headers.get("content-security-policy");
      await response.arrayBuffer();
      assert.equal(
        observed,
        STRICT,
        `the response actually received for ${entry.url} carries the strict policy, got ${observed}`,
      );
    }
    // Permanent actual awaited GET of an existing directory: before this
    // repair readFileSync threw on it instead of answering the fixed 404.
    const directory = await fetch(`${base}/assets/`);
    const directoryBody = await directory.text();
    assert.equal(
      directory.status,
      404,
      `GET /assets/ answers 404, got ${directory.status}`,
    );
    assert.equal(
      directoryBody,
      "csp-probe: not found",
      `GET /assets/ answers the fixed not-found text, got ${directoryBody}`,
    );
    // Served-body digests bound to the actual emitted dist files, not to
    // raw pkg inputs: the glue chunk is the bundler's rewrite, so the
    // binding is to the file at the emitted name.
    const servedAssets = served200.filter((entry) =>
      /\/assets\//.test(new globalThis.URL(entry.url, base).pathname));
    for (const entry of servedAssets) {
      const name = new globalThis.URL(entry.url, base).pathname.slice("/assets/".length);
      const emitted = createHash("sha256")
        .update(readFileSync(join(DIST, "assets", name)))
        .digest("hex");
      assert.equal(
        entry.digest,
        emitted,
        `the served bytes of ${name} are the emitted dist file's bytes`,
      );
    }
    const documentEntry = served200.find((entry) => entry.served === "/index.html");
    assert.equal(
      state.header,
      "core ready",
      "the unmodified dist must reach core ready on plain static HTTP",
    );
    assert.notEqual(state.tier, null, "the resolved tier is shown");
    assert.deepEqual(
      errors,
      [],
      "no page error fires on the unmodified dist",
    );
    assert.equal(
      absolute.length,
      0,
      `no /@fs request reaches the server, got ${absolute.map((a) => a.url).join(", ")}`,
    );
    const workerEntries = served.filter((entry) =>
      /core-worker-/.test(entry.url));
    assert.equal(workerEntries.length, 1, "the worker chunk was served once");
    assert.equal(workerEntries[0].status, 200, "the worker chunk answers 200");
    const glueEntries = served.filter((entry) =>
      /ocelli_wasm-[^/]*\.js$/.test(entry.url));
    assert.equal(
      glueEntries.length,
      1,
      "the emitted glue chunk was requested from the worker and served",
    );
    assert.equal(glueEntries[0].status, 200, "the glue chunk answers 200");
    const wasmEntries = served.filter((entry) =>
      /ocelli_wasm_bg-[^/]*\.wasm$/.test(entry.url));
    assert.equal(
      wasmEntries.length,
      1,
      "the emitted module asset was requested and served",
    );
    assert.equal(wasmEntries[0].status, 200, "the module asset answers 200");
    const inputGlue = createHash("sha256")
      .update(readFileSync(join(PKG, "ocelli_wasm.js")))
      .digest("hex");
    const inputWasm = createHash("sha256")
      .update(readFileSync(join(PKG, "ocelli_wasm_bg.wasm")))
      .digest("hex");
    assert.equal(
      wasmEntries[0].digest,
      inputWasm,
      "the served module bytes are byte-identical to the release pkg input",
    );
    // The glue chunk is Vite's bundled rewrite of the pkg glue, so it is NOT
    // byte-identical to the input. The identities that are claimed are: the
    // module asset identical to its pkg input, and the chunk names recorded.
    assert.notEqual(
      glueEntries[0].digest,
      inputGlue,
      "the glue chunk is the bundler's rewrite, not a passthrough of the input file",
    );
    assert.match(
      workerEntries[0].url,
      /core-worker-/,
      "the served worker is the emitted hashed chunk",
    );
    assert.notEqual(
      documentEntry,
      undefined,
      "the document response is in the served log",
    );
    const documentEmitted = createHash("sha256")
      .update(readFileSync(join(DIST, "index.html")))
      .digest("hex");
    assert.equal(
      documentEntry.digest,
      documentEmitted,
      "the served document bytes are the emitted index.html's bytes",
    );
    return {
      detail: `tier ${state.tier}, strict policy on every 200 response, worker/glue/wasm all 200 over real plain HTTP with served bytes equal to the emitted dist files, no /@fs request, module asset identical to the pkg input (${inputWasm.slice(0, 12)}...), glue chunk ${glueEntries[0].digest.slice(0, 12)}..., worker chunk ${workerEntries[0].digest.slice(0, 12)}...`,
    };
  } finally {
    close();
  }
}

const failures = [];
const notRun = [];
let passed = 0;
function report(name, outcome, detail) {
  globalThis.console.log(
    `${name.padEnd(20)} ${outcome.padEnd(8)} ${detail ?? ""}`.trimEnd(),
  );
  if (outcome === "FAIL") {
    failures.push(name);
  } else if (outcome === "NOT RUN") {
    notRun.push(name);
  } else if (outcome === "PASS") {
    passed += 1;
  }
}

function firstLine(text) {
  return text.split("\n")[0];
}

/** Route every response through the case's header and record violations. */
async function prepare(page, headerFor, routed) {
  await page.unrouteAll();
  await page.route("**/*", serveWith(headerFor, routed));
  await page.addInitScript(() => {
    if (!window.__violationListenerInstalled) {
      window.__violationListenerInstalled = true;
      window.__violations = [];
      document.addEventListener("securitypolicyviolation", (event) => {
        window.__violations.push(`${event.effectiveDirective} ${event.blockedURI}`);
      });
    }
  });
}

/**
 * Read the collector prelude out of every dedicated worker. One entry per
 * worker, null for a worker that did not carry the collector.
 */
async function workerProbe(page, settleMs = 300) {
  if (settleMs > 0) {
    await new Promise((resolve) => {
      globalThis.setTimeout(resolve, settleMs);
    });
  }
  const states = [];
  for (const worker of page.workers()) {
    try {
      // globalThis is the worker's own global, where the prelude collected.
      states.push(await worker.evaluate(() => globalThis.__probeCsp ?? null));
    } catch (error) {
      states.push({ error: String(error) });
    }
  }
  return states;
}

/** Load the example and read the state the header span and panels show. */
async function open(page, headerFor, routed) {
  await prepare(page, headerFor, routed);
  await page.goto(`${PAGE}/`, { waitUntil: "load" });
  // The header span reads "core not built" before the worker posts anything,
  // so the wait holds until a status arrived or the worker was refused.
  await page.waitForFunction(
    () => {
      const span = document.querySelector("header span");
      if (span === null || !/core (ready|not built)/.test(span?.textContent ?? "")) {
        return false;
      }
      const settled = [...document.querySelectorAll("p")].some((p) =>
        (p.textContent ?? "").startsWith("Tier "),
      );
      return settled || (window.__violations ?? []).length > 0;
    },
    undefined,
    { timeout: 30000 },
  );
  return page.evaluate(() => {
    const paragraphs = [...document.querySelectorAll("p")].map(
      (p) => p.textContent ?? "",
    );
    const canvas = document.querySelector('canvas[aria-label="Worker canvas"]');
    let pixel = null;
    if (canvas !== null) {
      const reader = document.createElement("canvas");
      reader.width = 1;
      reader.height = 1;
      const ctx = reader.getContext("2d");
      ctx.drawImage(canvas, 0, 0);
      pixel = Array.from(ctx.getImageData(0, 0, 1, 1).data).join(" ");
    }
    return {
      header: document.querySelector("header span")?.textContent ?? null,
      published: paragraphs.includes("Canvas published"),
      notPublished: paragraphs.includes("Canvas not published"),
      tier: paragraphs.find((text) => text.startsWith("Tier "))?.slice(5) ?? null,
      pixel,
      violations: window.__violations ?? [],
    };
  });
}

async function measureDocuments(page) {
  await page.getByRole("button", { name: "Synthetic documents" }).click();
  await page.getByText("Pixels 0, 1, 2, 3").waitFor({ timeout: 15000 });
  await page.getByRole("button", { name: "Show measurement" }).click();
  await page.getByText(/^TID 1500 /).waitFor({ timeout: 15000 });
  return page.evaluate(() => {
    const paragraphs = [...document.querySelectorAll("p")].map(
      (p) => p.textContent ?? "",
    );
    return {
      pixels: paragraphs.find((text) => text.startsWith("Pixels ")) ?? null,
      tid: paragraphs.find((text) => text.startsWith("TID 1500 ")) ?? null,
    };
  });
}

async function blobOutcome(page) {
  await page.goto(`${PAGE}/__probe/blob-page.html`, { waitUntil: "load" });
  await page.waitForFunction(
    () => document.getElementById("out")?.textContent !== "pending",
    undefined,
    { timeout: 20000 },
  );
  return page.evaluate(() => document.getElementById("out")?.textContent ?? "");
}

async function runCase(browser, name, body) {
  const context = await browser.newContext();
  const page = await context.newPage();
  const messages = [];
  const routed = [];
  page.on("console", (message) =>
    messages.push(`${message.type()}: ${message.text()}`),
  );
  page.on("pageerror", (error) => messages.push(`pageerror: ${error.message}`));
  let outcome;
  try {
    outcome = await body(page, messages, routed);
  } catch (error) {
    report(name, "FAIL", firstLine(error instanceof Error ? error.message : String(error)));
    await context.close();
    return null;
  }
  await context.close();
  report(name, "PASS", outcome.detail);
  return outcome.value ?? true;
}

const browser = await chromium.launch();
try {
  globalThis.console.log(`csp-probe, Chromium ${browser.version()}`);
  globalThis.console.log(`policy ${STRICT}`);

  // The unmodified positive runs first and gates the comparisons: without a
  // genuine core-ready over real plain HTTP there is no baseline.
  let realDist;
  try {
    realDist = await realServerPositive(browser);
    report("real-dist", "PASS", realDist.detail);
  } catch (error) {
    realDist = null;
    report("real-dist", "FAIL", firstLine(error instanceof Error ? error.message : String(error)));
  }

  const baseline = await runCase(
    browser,
    "control",
    async (page) => {
      const state = await open(page, () => null);
      assert.equal(
        state.header,
        "core ready",
        "the control run sends no header, so the core must load",
      );
      assert.equal(state.published, true, "the control publishes the canvas");
      const documents = await measureDocuments(page);
      assert.equal(
        documents.pixels,
        "Pixels 0, 1, 2, 3",
        "the control writes its synthetic documents",
      );
      assert.notEqual(documents.tid, null, "the control shows its TID 1500 line");
      return {
        detail: `tier ${state.tier}, pixel ${state.pixel}`,
        value: { tier: state.tier, pixel: state.pixel, tid: documents.tid },
      };
    },
  );
  if (baseline === null) {
    globalThis.console.error(
      "csp-probe: the control run failed, so the comparisons have no baseline",
    );
  } else if (realDist === null) {
    // The policy comparisons still run, but the unmet genuine positive is a
    // named failure the exit code already carries.
    globalThis.console.error(
      "csp-probe: the unmodified-dist positive failed, the policy comparisons have no real-server baseline",
    );
  } else {
    await runCase(browser, "strict", async (page, messages) => {
      const state = await open(page, () => STRICT);
      assert.equal(
        state.header,
        "core ready",
        "the strict header must not stop the worker and wasm path",
      );
      assert.equal(state.published, true, "the canvas publishes under the strict header");
      assert.equal(
        state.tier,
        baseline.tier,
        "the resolved tier equals the no-header control",
      );
      assert.equal(
        state.pixel,
        baseline.pixel,
        "the worker canvas pixel equals the control",
      );
      assert.deepEqual(
        messages,
        [],
        "the full set of console messages collected from the page and the worker is not empty under the strict header",
      );
      assert.deepEqual(
        state.violations,
        [],
        "no securitypolicyviolation fires under the strict header",
      );
      const workers = await workerProbe(page);
      assert.equal(
        workers.length,
        1,
        "the example runs exactly one dedicated worker, so the collector observed it",
      );
      assert.ok(workers[0], "the worker chunk carried the probe collector");
      assert.deepEqual(
        workers[0].violations,
        [],
        "no securitypolicyviolation fires in the worker under the strict header",
      );
      assert.deepEqual(
        workers[0].console,
        [],
        "the full set of console messages collected in the worker is not empty under the strict header",
      );
      return { detail: `tier ${state.tier}, pixel ${state.pixel}, the full console message set from the page and the worker empty, every console method wrapped` };
    });

    await runCase(browser, "no-wasm", async (page, messages, routed) => {
      const state = await open(page, () => WITHOUT_WASM_EVAL, routed);
      assert.equal(
        state.header,
        "core not built",
        "without 'wasm-unsafe-eval' the compile is refused",
      );
      assert.equal(
        state.notPublished,
        true,
        "the canvas is not published without 'wasm-unsafe-eval'",
      );
      const wasm = routed.filter((entry) => /ocelli_wasm_bg.*\.wasm/.test(entry.url));
      assert.equal(
        wasm.length,
        1,
        "the module bytes were fetched, connect-src did not refuse them",
      );
      assert.equal(
        wasm[0].csp,
        WITHOUT_WASM_EVAL,
        "the module response carried the header without 'wasm-unsafe-eval'",
      );
      assert.deepEqual(
        state.violations,
        [],
        "no violation fires in the page, the compile is refused in the worker",
      );
      const [worker] = await workerProbe(page, 0);
      assert.ok(worker, "the worker chunk carried the probe collector");
      const refused = worker.violations.filter(
        (text) => text.startsWith("script-src") && text.includes("wasm"),
      );
      assert.equal(
        refused.length,
        1,
        "the worker named the refused compile in its own violation",
      );
      assert.deepEqual(
        worker.violations.filter((text) => !text.startsWith("script-src")),
        [],
        "no refusal other than the wasm compile fired in the worker",
      );
      assert.notEqual(
        worker.console.length,
        0,
        "the worker logged its failed load",
      );
      return {
        detail: `core not built, the module was fetched and served, the worker named the refused compile (${refused[0]}) and logged ${worker.console[0]}`,
      };
    });

    await runCase(browser, "no-worker", async (page) => {
      const state = await open(page, () => NO_WORKER);
      assert.equal(
        state.header,
        "core not built",
        "worker-src 'none' refuses the module worker",
      );
      const named = state.violations.filter((text) =>
        text.startsWith("worker-src"),
      );
      assert.notEqual(
        named.length,
        0,
        "a worker-src violation names the refused worker",
      );
      return { detail: `core not built, violation ${named[0]}` };
    });

    await runCase(browser, "worker-fallback", async (page) => {
      const state = await open(page, () => NO_WORKER_PIN);
      assert.equal(
        state.header,
        "core ready",
        "worker-src absent falls back to script-src, which still allows the module worker",
      );
      assert.equal(
        state.published,
        true,
        "the canvas publishes without the worker-src pin",
      );
      return {
        detail: "core ready, worker-src absent fell back to script-src 'self'",
      };
    });

    await runCase(browser, "no-connect", async (page, messages, routed) => {
      // The import-only worker loads its glue through a script module
      // import, which script-src 'self' allows. connect-src 'none' still
      // refuses the wasm fetch inside that glue, so the page stays core not
      // built, no module bytes reach the server, and the worker's own
      // connect-src violation names the refused wasm fetch.
      const state = await open(page, () => NO_CONNECT, routed);
      assert.equal(
        state.header,
        "core not built",
        "connect-src 'none' refuses the wasm fetch inside the glue import",
      );
      assert.equal(
        state.notPublished,
        true,
        "the canvas is not published under connect-src 'none'",
      );
      assert.deepEqual(
        routed.filter((entry) => entry.url.includes("ocelli_wasm_bg")),
        [],
        "no module bytes were fetched, connect-src refused the fetch inside the glue",
      );
      const [worker] = await workerProbe(page, 0);
      assert.ok(worker, "the worker chunk carried the probe collector");
      const refused = worker.violations.filter(
        (text) => text.startsWith("connect-src") && text.includes("wasm"),
      );
      assert.equal(
        refused.length,
        1,
        "the worker's own connect-src violation names the refused wasm fetch",
      );
      return { detail: `core not built, ${refused[0]}` };
    });

    await runCase(browser, "no-script", async (page) => {
      await prepare(page, () => NO_SCRIPT);
      await page.goto(`${PAGE}/`, { waitUntil: "load" });
      const mounted = await page.evaluate(
        () => document.querySelector("#root")?.children.length ?? -1,
      );
      assert.equal(mounted, 0, "React does not mount, #root is empty");
      const named = (await page.evaluate(() => window.__violations ?? [])).filter(
        (text) => text.startsWith("script-src"),
      );
      assert.notEqual(
        named.length,
        0,
        "a script-src violation names the refused bundle",
      );
      return { detail: "React did not mount, script-src refused the bundle" };
    });

    await runCase(browser, "worker-strict", async (page, messages, routed) => {
      const state = await open(
        page,
        (url) => (isWorkerChunk(url) ? WITHOUT_WASM_EVAL : STRICT),
        routed,
      );
      assert.equal(
        state.header,
        "core not built",
        "the worker's own response policy governs its wasm compile",
      );
      const chunk = routed.filter((entry) => isWorkerChunk(entry.url));
      assert.equal(chunk.length, 1, "the worker chunk was served once");
      assert.equal(
        chunk[0].csp,
        WITHOUT_WASM_EVAL,
        "the worker response carried the header without 'wasm-unsafe-eval'",
      );
      const documentResponse = routed.filter(
        (entry) => new globalThis.URL(entry.url).pathname === "/",
      );
      assert.equal(
        documentResponse[0].csp,
        STRICT,
        "the page response carried the full header, 'wasm-unsafe-eval' included",
      );
      const wasm = routed.filter((entry) => /ocelli_wasm_bg.*\.wasm/.test(entry.url));
      assert.equal(wasm.length, 1, "the module bytes were still fetched");
      assert.equal(
        wasm[0].csp,
        STRICT,
        "the module's own response even carried 'wasm-unsafe-eval', the worker response is what governs the compile",
      );
      return {
        detail: "core not built, the worker response carried no 'wasm-unsafe-eval' while the page's and the module's did",
      };
    });

    await runCase(browser, "page-strict", async (page) => {
      const state = await open(
        page,
        (url) => {
          const path = new globalThis.URL(url).pathname;
          return path === "/" || path === "/index.html" ? WITHOUT_WASM_EVAL : STRICT;
        },
      );
      assert.equal(
        state.header,
        "core ready",
        "the page's policy does not reach the worker",
      );
      return {
        detail:
          "core ready, the page document alone went without 'wasm-unsafe-eval' and the worker compiled anyway",
      };
    });

    await runCase(browser, "blob-control", async (page) => {
      const refused = await (async () => {
        await prepare(page, () => STRICT);
        return blobOutcome(page);
      })();
      assert.ok(
        !refused.includes("compiled"),
        `the blob worker must not compile under the strict page, got ${refused}`,
      );
      assert.ok(
        refused.includes("worker-src"),
        `the refusal names worker-src, got ${refused}`,
      );
      assert.ok(
        refused.includes("blob"),
        `the refusal names the blob scheme, got ${refused}`,
      );
      const compiled = await (async () => {
        await prepare(page, () => RELAXED_BLOB);
        return blobOutcome(page);
      })();
      assert.ok(
        compiled.includes("CompileError"),
        `the blob worker's compile fails without 'wasm-unsafe-eval', got ${compiled}`,
      );
      const allowed = await (async () => {
        await prepare(page, () => BLOB_ALLOWED);
        return blobOutcome(page);
      })();
      assert.ok(
        allowed.startsWith("result: compiled"),
        `the same blob worker with the same module compiles when the policy allows it, got ${allowed}`,
      );
      assert.ok(
        !allowed.includes("worker-src"),
        `the allowed page raises no worker-src violation, got ${allowed}`,
      );
      return { detail: `strict page: ${refused}, relaxed page: ${compiled}, allowed page: ${allowed}` };
    });

    await runCase(browser, "documents", async (page) => {
      await open(page, () => STRICT);
      const documents = await measureDocuments(page);
      assert.equal(
        documents.pixels,
        "Pixels 0, 1, 2, 3",
        "the capture shows its four bytes under the strict header",
      );
      assert.equal(
        documents.tid,
        baseline.tid,
        "the TID 1500 line equals the control's",
      );
      return { detail: `${documents.pixels}, ${documents.tid}` };
    });

    if (existsSync(INSTANCE)) {
      const fillDicomweb = async (page) => {
        await page.getByLabel("DICOMweb base URL").fill(DICOMWEB);
        await page.getByLabel("Study UID").fill("2.25.1961");
        await page.getByLabel("Series UID").fill("2.25.1962");
        await page.getByLabel("Instance UID").fill("2.25.1963");
        await page.getByRole("button", { name: "Fetch instance" }).click();
      };
      await runCase(browser, "dicomweb", async (page) => {
        const state = await open(page, () => DICOMWEB_ALLOWED);
        assert.equal(
          state.header,
          "core ready",
          "the listed origin still loads the core",
        );
        await fillDicomweb(page);
        await page
          .getByText("The instance bytes were handed to the worker.")
          .waitFor({ timeout: 20000 });
        return { detail: "the instance bytes were handed to the worker" };
      });
      await runCase(browser, "dicomweb-unlisted", async (page) => {
        const state = await open(page, () => STRICT);
        assert.equal(
          state.header,
          "core ready",
          "the core still loads for the unlisted-origin case",
        );
        await fillDicomweb(page);
        await page
          .getByText("The instance request was refused.")
          .waitFor({ timeout: 20000 });
        // The blocked fetch rejects before the violation event lands, so
        // poll briefly. The assertion below is the check, not the poll.
        try {
          await page.waitForFunction(
            () => (window.__violations ?? []).some((text) => text.startsWith("connect-src")),
            undefined,
            { timeout: 5000 },
          );
        } catch {
          // The read below still asserts.
        }
        const violations = await page.evaluate(() => window.__violations ?? []);
        const named = violations.filter((text) => text.startsWith("connect-src"));
        assert.notEqual(
          named.length,
          0,
          "the refusal names connect-src for the unlisted origin",
        );
        return { detail: "the instance request was refused, connect-src named" };
      });
    } else {
      report("dicomweb", "NOT RUN", `${relative(ROOT, INSTANCE)} is absent`);
      report("dicomweb-unlisted", "NOT RUN", `${relative(ROOT, INSTANCE)} is absent`);
    }
  }
} finally {
  await browser.close();
}

globalThis.console.log(
  `csp-probe, ${passed} passed, ${failures.length} failed, ${notRun.length} not run`,
);
globalThis.process.exitCode = failures.length === 0 ? 0 : 1;
