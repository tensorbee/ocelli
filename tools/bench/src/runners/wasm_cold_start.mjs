// `wasm.cold_start`, the one subject with a real subject today.
//
// Appendix A gate A4 has two halves. F-002 measured the first, recording
// 14,104 bytes in `ci/wasm-size-budget.json` on 2026-09-04, re-baselined by
// F-005 to 16,388. **The second half, cold start, has never been measured at
// all.** This is that half.
//
// It is not a fabricated workload. It is the actual release artefact doing the
// actual thing a browser does at session start, and nothing here substitutes a
// proxy for a subject that does not exist.
//
// **The number is tiny and it carries the same caveat docs/lld/build-targets.md
// puts on the size number.** A module with four exported functions, no wgpu and
// no Naga tells you nothing about the cold start of a feature-complete module.
// A4 stays open. The first recorded figure is a baseline for regression
// detection during build-out, not an answer to the gate, and re-baselining is
// expected repeatedly with each one naming its design plan.

import { spawn } from "node:child_process";
import { createServer } from "node:http";
import { createReadStream } from "node:fs";
import { copyFile, mkdir, readFile, rm, stat } from "node:fs/promises";
import { createHash } from "node:crypto";
import { extname, join, resolve, sep } from "node:path";

import { chromium } from "playwright";

import { benchPath, repoPath } from "../paths.mjs";

export const id = "wasm.cold_start";

/**
 * Iterations, of which the first is discarded.
 *
 * SIXTEEN, MEASURED, not a round number picked for looking careful. At six the
 * median of the five kept iterations swung between 2.2 and 2.9 ms across ten
 * consecutive runs on this machine, which is 26 per cent and would force a
 * tolerance so wide it caught nothing. At sixteen the median of the fifteen
 * kept iterations sat between 2.3 and 2.5 ms across eleven consecutive runs,
 * a spread of about 4 per cent, on a machine carrying a load average near six
 * because another sprint worker was running. The whole subject costs about two
 * seconds either way, so the tighter instrument is free.
 */
const ITERATIONS = 16;

/** How long one page may take to load and measure. A timeout is a failure. */
const MEASURE_TIMEOUT_MS = 30_000;

const CONTENT_TYPES = {
  ".html": "text/html; charset=utf-8",
  ".js": "text/javascript; charset=utf-8",
  ".mjs": "text/javascript; charset=utf-8",
  ".wasm": "application/wasm",
};

/** What the page needs from `crates/ocelli-wasm/pkg`: the glue and the module. */
const ARTEFACT_FILES = ["ocelli_wasm.js", "ocelli_wasm_bg.wasm"];

/**
 * Where a request path lands on disk, or `null` if it lands outside `base`.
 *
 * The containment check is made against the RESULT of `join`, never against the
 * request string, because a check that inspects the request and then normalises
 * it is checking something other than the path that gets opened.
 */
export function resolveServedPath(base, pathname) {
  const root = resolve(base);
  let decoded;
  try {
    decoded = decodeURIComponent(pathname);
  } catch {
    return null;
  }
  const candidate = resolve(
    join(root, decoded === "/" ? "/index.html" : decoded),
  );
  if (candidate !== root && !candidate.startsWith(root + sep)) {
    return null;
  }
  return candidate;
}

/** Serve `root` on loopback, on an ephemeral port. */
async function serveDirectory(root) {
  const base = resolve(root);
  const server = createServer((request, response) => {
    const requested = new URL(request.url, "http://127.0.0.1");
    const candidate = resolveServedPath(base, requested.pathname);
    if (candidate === null) {
      response.writeHead(403).end("outside the served directory");
      return;
    }
    stat(candidate)
      .then((info) => {
        if (!info.isFile()) {
          response.writeHead(404).end("not a file");
          return;
        }
        response.writeHead(200, {
          "content-type":
            CONTENT_TYPES[extname(candidate)] ?? "application/octet-stream",
          "content-length": info.size,
          "cache-control": "no-store",
        });
        createReadStream(candidate).pipe(response);
      })
      .catch(() => {
        response.writeHead(404).end("not found");
      });
  });
  await new Promise((done, fail) => {
    server.once("error", fail);
    server.listen(0, "127.0.0.1", done);
  });
  const address = server.address();
  return {
    origin: `http://127.0.0.1:${address.port}`,
    close: () => new Promise((done) => server.close(() => done())),
  };
}

/** The workspace version, from the one place it is declared. */
export function workspaceVersion(cargoToml) {
  const section = /\[workspace\.package\]([\s\S]*?)(?:\n\[|$)/.exec(cargoToml);
  if (!section) {
    throw new Error("Cargo.toml has no [workspace.package] section");
  }
  const match = /^\s*version\s*=\s*"([^"]+)"/m.exec(section[1]);
  if (!match) {
    throw new Error("[workspace.package] declares no version");
  }
  return match[1];
}

/**
 * A duration at the precision the clock actually has.
 *
 * THIS IS A ROUNDING DECISION and it is stated rather than buried, because HLD
 * 27.3 asks a human to check every one. `performance.now()` in this Chromium is
 * quantised to 0.1 ms, so every reading is a multiple of 0.1. Subtracting two
 * of them in binary floating point turns 2.2 into 2.2000000029802322, and
 * recording that would claim thirteen digits of precision the clock does not
 * have. Rounding to four decimal places is a thousand times finer than the
 * quantum, so it cannot move a figure from one side of a tolerance to the
 * other. It removes the arithmetic's artefact and nothing else.
 */
export function atClockPrecision(ms) {
  return Math.round(ms * 1e4) / 1e4;
}

/** The median of a list of numbers. An even count averages the middle two. */
export function median(values) {
  if (values.length === 0) {
    throw new Error("no values to take a median of");
  }
  const sorted = [...values].sort((a, b) => a - b);
  const middle = Math.floor(sorted.length / 2);
  return sorted.length % 2 === 1
    ? sorted[middle]
    : (sorted[middle - 1] + sorted[middle]) / 2;
}

/** Build the release artefact, through the same command the size gate uses. */
async function buildArtefact() {
  await new Promise((done, fail) => {
    const child = spawn(repoPath("bin", "ocelli.sh"), ["wasm"], {
      cwd: repoPath(),
      stdio: ["ignore", "inherit", "inherit"],
    });
    child.on("error", fail);
    child.on("exit", (code) => {
      if (code === 0) {
        done();
        return;
      }
      fail(new Error(`bin/ocelli.sh wasm exited ${code}`));
    });
  });
}

/**
 * Assemble the served directory: the page, plus a copy of the built artefact.
 *
 * A copy and not a symlink into `crates/ocelli-wasm/pkg`, so that the digest
 * recorded below is the digest of the bytes the browser actually fetched.
 */
async function assemblePage(outDir) {
  const pageDir = join(outDir, "page");
  await rm(pageDir, { recursive: true, force: true });
  await mkdir(pageDir, { recursive: true });
  for (const name of ["index.html", "app.mjs"]) {
    await copyFile(benchPath("page", name), join(pageDir, name));
  }
  const pkg = repoPath("crates", "ocelli-wasm", "pkg");
  for (const name of ARTEFACT_FILES) {
    try {
      await copyFile(join(pkg, name), join(pageDir, name));
    } catch (error) {
      throw new Error(
        `crates/ocelli-wasm/pkg/${name} is not there. This runner measures ` +
          `the RELEASE artefact and refuses to measure anything else. Run ` +
          `bin/ocelli.sh wasm, or drop --no-build. (${String(error?.message ?? error)})`,
      );
    }
  }
  const wasmBytes = await readFile(join(pageDir, "ocelli_wasm_bg.wasm"));
  return {
    pageDir,
    artefact: {
      bytes: wasmBytes.byteLength,
      sha256: createHash("sha256").update(wasmBytes).digest("hex"),
    },
  };
}

/**
 * Run the subject.
 *
 * @param {{outDir: string, build: boolean}} options
 * @returns {Promise<{value: number, detail: object,
 *                    instrument: {chromium: string}}>}
 */
export async function run({ outDir, build = true }) {
  if (build) {
    await buildArtefact();
  }
  const { pageDir, artefact } = await assemblePage(outDir);
  const expectedVersion = workspaceVersion(
    await readFile(repoPath("Cargo.toml"), "utf8"),
  );

  const server = await serveDirectory(pageDir);
  const iterations = [];
  let chromiumVersion = null;
  try {
    for (let index = 0; index < ITERATIONS; index += 1) {
      // A FRESH BROWSER per iteration, not a fresh page and not a fresh
      // context. A compiled WebAssembly module is cached by the browser
      // process, so a second page in the same process would report a compile
      // phase that measured a cache lookup. Launching is slower and the
      // slowness is not in the measurement.
      const browser = await chromium.launch();
      chromiumVersion = browser.version();
      try {
        const page = await browser.newPage();
        await page.goto(server.origin, {
          waitUntil: "load",
          timeout: MEASURE_TIMEOUT_MS,
        });
        // The handle, not a fixed sleep and not the load event alone. A page
        // that loaded and defined nothing is indistinguishable from one that
        // measured nothing, and `app.mjs` sets the handle as its last act.
        await page.waitForFunction(
          () => globalThis.__bench !== undefined,
          undefined,
          { timeout: MEASURE_TIMEOUT_MS },
        );
        const measurement = await page.evaluate(
          async () => globalThis.__bench.measure(),
        );
        if (measurement.version !== expectedVersion) {
          throw new Error(
            `ocelli_version() returned ${JSON.stringify(measurement.version)} ` +
              `and Cargo.toml declares ${JSON.stringify(expectedVersion)}. ` +
              `The page measured something other than this tree's artefact.`,
          );
        }
        if (measurement.artefact_bytes !== artefact.bytes) {
          throw new Error(
            `the page fetched ${measurement.artefact_bytes} bytes and the ` +
              `served copy is ${artefact.bytes}`,
          );
        }
        if (!(measurement.total > 0)) {
          throw new Error(
            `the page reported a total of ${measurement.total} ms. A cold ` +
              `start that took no time did not happen.`,
          );
        }
        iterations.push(measurement);
      } finally {
        await browser.close();
      }
    }
  } finally {
    await server.close();
  }

  // The first is discarded. It carries the operating system's page-cache miss
  // on a freshly built artefact and the browser's own first-launch cost, and
  // neither is what this subject is defined to measure.
  // Round each observation first, then take the median, for the total and for
  // every phase alike. Doing it the other way round for one of them would leave
  // two orders of operation in one function for no reason, and while the
  // difference is immaterial at four decimal places against a 0.1 ms quantum,
  // "immaterial" is a judgement a reader should not have to make twice.
  const kept = iterations.slice(1);
  const totals = kept.map((one) => atClockPrecision(one.total));
  const phases = {};
  for (const name of Object.keys(kept[0].phases)) {
    phases[name] = median(
      kept.map((one) => atClockPrecision(one.phases[name])),
    );
  }
  return {
    value: atClockPrecision(median(totals)),
    instrument: { chromium: chromiumVersion },
    detail: {
      statistic: "median",
      iterations_run: ITERATIONS,
      iterations_kept: kept.length,
      discarded: "the first iteration, page-cache miss and first browser launch",
      observed_range_ms: [Math.min(...totals), Math.max(...totals)],
      totals_ms: totals,
      phases_median_ms: phases,
      clock_quantum_ms: 0.1,
      clock:
        "performance.now() in the page. Chromium quantises it to 0.1 ms, " +
        "which is a large fraction of a figure this small, and every " +
        "duration here is rounded to four decimal places to drop the binary " +
        "floating point artefact of subtracting two such readings.",
      artefact,
      profile: "release",
      version: expectedVersion,
      caveat:
        "This module holds four exported functions, no wgpu and no Naga. " +
        "Appendix A gate A4 estimates 3 to 8 MB uncompressed with Naga " +
        "dominating, so this figure has no bearing on A4 and A4 stays open. " +
        "It is a regression baseline for the build-out phase.",
      not_measured:
        "A user's true cold start, which includes the network fetch of an " +
        "uncached artefact over a real link and a browser process that was " +
        "not just launched. The artefact is served from loopback here, so " +
        "the fetch phase is a lower bound and nothing else.",
    },
  };
}

export default { id, run };
