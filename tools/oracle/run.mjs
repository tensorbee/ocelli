#!/usr/bin/env node
// The differential harness's reference half: render every corpus row through
// the pinned cornerstone3D in headless Chromium and emit reference pixels or a
// precise failure. F-010, HLD section 11, decision D7.
//
// It does not compare anything. Comparison is F-011.
//
// THE DEFECT THIS FILE EXISTS TO PREVENT, from docs/sprints/CURRENT_SPRINT.md:
//
//   "A headless page can start, load a test runner and exit successfully
//    without decoding every corpus row, presenting a frame or reading back the
//    rendered pixels."
//
// So there are four boundaries and each one has an assertion that fails
// loudly:
//
//   1. REACHED     every row in the manifest was attempted, counted before the
//                  run starts and checked after it ends.
//   2. DECODED     cornerstone3D's image load resolved. A rejection is a
//                  failure unless `unsupported.json` names it.
//   3. PRESENTED   cornerstone3D's own IMAGE_RENDERED event fired, with a
//                  timeout. Never a fixed sleep.
//   4. READ BACK   the pixels came back, at the declared size, and are not a
//                  single value. A blank canvas reads back perfectly and
//                  hashes stably, which is why it needs its own check.
//
// Each of the four is observed red by `--inject`, see `tests/faults.mjs`.
//
// Nothing under `out/` is ever committed. A reference frame of a real corpus
// row is a rendered picture of patient data and every real row in
// `corpus/manifest.tsv` carries `burned-in-unchecked`.

import { spawn } from "node:child_process";
import { createRequire } from "node:module";
import { readFile, writeFile } from "node:fs/promises";
import { existsSync } from "node:fs";
import { join, resolve } from "node:path";
import { hostname, platform, release, arch } from "node:os";

import { chromium } from "playwright";

import { buildPage, codecPackages, PAGE_DIST } from "./build-page.mjs";
import {
  CORPUS_DATA,
  digestOf,
  digestOfManifest,
  readManifest,
  rowId,
} from "./src/manifest.mjs";
import {
  canvasScale,
  readRenderParams,
  resolveRenderParams,
  RENDER_PARAMS_PATH,
} from "./src/params.mjs";
import { isEntryPoint, oraclePath, repoPath } from "./src/paths.mjs";
import { installedVersion } from "./src/pins.mjs";
import { serveDirectory } from "./src/server.mjs";
import {
  discardOutput,
  isInside,
  openOutput,
  prepareOutput,
  writeRow,
} from "./src/output.mjs";
import {
  claimedRows,
  entryFor,
  readUnsupported,
  UNSUPPORTED_PATH,
} from "./src/unsupported.mjs";
import {
  FAULTS,
  faultedBytes,
  faultedParams,
  pageFaultName,
  skipsRow,
} from "./src/faults.mjs";
// The self test is the runner, not the catalogue. It re-enters this file
// through `spawn`, so there is no module cycle.
import { runSelfTest } from "./tests/faults.mjs";

const require = createRequire(import.meta.url);

/** Pinned exactly, for section 15.2's reason applied to the reference. */
export const PINNED = {
  "@cornerstonejs/core": "5.8.2",
  "@cornerstonejs/tools": "5.8.2",
  "@cornerstonejs/dicom-image-loader": "5.8.2",
  "@cornerstonejs/metadata": "5.8.2",
  "@cornerstonejs/utils": "5.8.2",
  // NOT part of the renderer, and pinned for a reason of its own. This is what
  // reads the sidecar's `attributes` block straight from the bytes, and HLD
  // section 11 makes that block load-bearing output "because a wrong rescale
  // slope can still produce a plausible image". A drifting metadata reader is
  // the same problem as a drifting renderer.
  "dicom-parser": "1.8.21",
  // cornerstone3D v5 renders through vtk.js, so of everything here this is the
  // package whose drift would move the most reference pixels.
  "@kitware/vtk.js": "36.4.1",
  "gl-matrix": "3.4.3",
  playwright: "1.62.1",
  // Not part of the reference. It is what turns the reference into a page a
  // browser can load, and a bundler that changed the module graph between two
  // runs would move the output for a reason nobody could see.
  esbuild: "0.28.2",
};

/**
 * The four codec packages.
 *
 * Nothing imports them by name at run time. `build-page.mjs` resolves them to
 * copy their `.wasm` binaries into the page, and those binaries decide the
 * decoded pixels for every JPEG, JPEG-LS, JPEG 2000 and HTJ2K row. Leaving them
 * to npm's hoisting out of `@cornerstonejs/dicom-image-loader`'s subtree would
 * make the decoders a phantom dependency, which is the one kind of dependency
 * hoisting can stop providing without warning.
 */
export const PINNED_CODECS = {
  "@cornerstonejs/codec-charls": "1.2.5",
  "@cornerstonejs/codec-libjpeg-turbo-8bit": "1.2.4",
  "@cornerstonejs/codec-openjpeg": "1.3.2",
  "@cornerstonejs/codec-openjph": "2.4.9",
};

/**
 * Peer dependencies of `@cornerstonejs/tools`, which this harness never loads.
 *
 * Pinned like everything else. They are listed apart only because they are
 * needed by nothing the reference render executes, and a later reader deleting
 * `tools` should delete these with it.
 */
export const PINNED_TOOLS_PEERS = {
  "d3-array": "3.2.4",
  "d3-interpolate": "3.0.1",
};

export const DEFAULT_OUT = oraclePath("out");

/** How long one row may take to present a frame. A timeout is a failure. */
const RENDER_TIMEOUT_MS = 30_000;

// ---------------------------------------------------------------------------
// Arguments
// ---------------------------------------------------------------------------

export function parseArgs(argv) {
  const options = {
    out: DEFAULT_OUT,
    rows: null,
    once: false,
    inject: null,
    unit: true,
    metadataCheck: true,
    selfTest: true,
    reportUnsupported: false,
  };
  // A flag whose value went missing would otherwise become `undefined` and
  // surface much later as an unrelated error, which is the shape of failure
  // this harness exists to refuse.
  const value = (index, flag) => {
    if (index >= argv.length || argv[index] === "" || argv[index].startsWith("--")) {
      throw new Error(`${flag} needs a value. Try --help.`);
    }
    return argv[index];
  };

  for (let index = 0; index < argv.length; index += 1) {
    const arg = argv[index];
    switch (arg) {
      case "--out":
        // Resolved, not taken verbatim. `DEFAULT_OUT` is absolute, and the
        // guard below compares the two. `--out tools/oracle/out` from the
        // repository root names the same directory and would not have
        // compared equal, so the guard would not fire and a one-row run would
        // replace the full corpus render that F-011 reads.
        options.out = resolve(value(++index, arg));
        break;
      case "--rows":
        options.rows = value(++index, arg);
        break;
      case "--once":
        options.once = true;
        break;
      case "--inject":
        options.inject = value(++index, arg);
        // `Object.hasOwn`, not a truthiness test. `FAULTS["constructor"]`
        // resolves through Object.prototype and is truthy, so a plain lookup
        // accepted a fault name that injects nothing, and the run then
        // rendered normally and reported green having broken nothing.
        if (!Object.hasOwn(FAULTS, options.inject)) {
          throw new Error(
            `unknown fault ${JSON.stringify(options.inject)}. The named ` +
              `faults are ${Object.keys(FAULTS).join(", ")}.`,
          );
        }
        break;
      case "--no-unit":
        options.unit = false;
        break;
      case "--no-metadata-check":
        options.metadataCheck = false;
        break;
      case "--no-self-test":
        options.selfTest = false;
        break;
      case "--report-unsupported":
        options.reportUnsupported = true;
        break;
      case "--help":
      case "-h":
        options.help = true;
        break;
      default:
        throw new Error(`unknown argument ${arg}. Try --help.`);
    }
  }
  if (options.inject) {
    // An injected run is deliberately broken output. It never touches `out/`,
    // never runs the checks that would then also fail, and never recurses into
    // the self test that spawned it.
    options.once = true;
    options.unit = false;
    options.metadataCheck = false;
    options.selfTest = false;
  }
  // A partial run must name its own directory. Otherwise `--rows syntax/`
  // would leave the canonical output holding eighteen frames, and F-011 would
  // read a subset of the corpus as if it were the corpus. `run.json` records
  // `partial`, but a record of a trap is not the same as not setting one.
  //
  // `sameDirectory` and not a string comparison, because three spellings name
  // the canonical directory and only one of them is caught by comparing
  // strings: the relative one, which `resolve` fixes, the case-only variant on
  // a case-insensitive filesystem, which `realpathSync` does NOT fix on macOS,
  // and a symlink. The device and inode pair is the filesystem's own answer.
  // `isInside`, which asks `sameDirectory` about every ancestor, so the case
  // and symlink handling covers a SUBDIRECTORY too. A string prefix test would
  // have caught `<out>/sub` and missed `<OUT>/sub` and
  // `<symlink-to-out>/sub`, which is the comparison the paragraph above says
  // is not enough, made one level up.
  if (options.rows && isInside(options.out, DEFAULT_OUT)) {
    throw new Error(
      `--rows selects a subset of the corpus, so --out must name a directory ` +
        `outside ${DEFAULT_OUT}. Writing a subset there would leave F-011 ` +
        `reading part of the corpus as if it were all of it. The relative ` +
        `spelling, the absolute one, a case-only variant, a symlink and a ` +
        `subdirectory all name that output and all are refused.`,
    );
  }
  return options;
}

const USAGE = `bin/ocelli.sh oracle [options]

  --out <dir>            where reference output goes (default tools/oracle/out)
  --rows <substring>     only rows whose manifest path contains this. Requires
                         --out, because a subset must not become the output
  --once                 one pass, so determinism is not measured
  --inject <fault>       run one named fault and EXPECT the run to fail:
                         ${Object.keys(FAULTS).join(", ")}
  --report-unsupported   print candidate unsupported.json entries and stop.
                         Checks the pins, runs the unit suites, rebuilds the
                         page and re-hashes every corpus row on the way, so it
                         is not free. It verifies NO BOUNDARY, writes no
                         reference output, and exits 2 so it can never be
                         mistaken for a passing gate
  --no-unit              skip the pure unit tests
  --no-metadata-check    skip the pydicom cross-read of the sidecars
  --no-self-test         skip the fault injection self test

Every --no-* flag is a development aid. The oracle gate passes none of them,
so a run that skipped a check cannot be recorded as one that passed it.
`;

// ---------------------------------------------------------------------------
// Step 1, the pins
// ---------------------------------------------------------------------------

/**
 * Every pinned package is installed at exactly its pin.
 *
 * `bin/ocelli.sh` refuses when `node_modules` is absent. Present but at the
 * wrong version is the case this covers, and it is a refusal rather than a
 * warning: an oracle that drifts is not an oracle, and reference output from
 * an unpinned reference is not comparable with reference output from a pinned
 * one.
 */
function checkPins() {
  const installed = {};
  const problems = [];
  const pins = { ...PINNED, ...PINNED_CODECS, ...PINNED_TOOLS_PEERS };

  for (const [name, pin] of Object.entries(pins)) {
    let version;
    try {
      version = installedVersion(require, name);
    } catch (error) {
      problems.push(`${name} is not installed (${String(error?.message ?? error)})`);
      continue;
    }
    installed[name] = version;
    if (version !== pin) {
      problems.push(`${name} is installed at ${version}, and run.mjs pins ${pin}`);
    }
  }

  // The codecs are named in two places: pinned by version here, and copied
  // into the page by `build-page.mjs`, which names them as subpath specifiers.
  // One copied and not pinned decides pixels nobody can reproduce. One pinned
  // and not copied is a pin on something the page never loads.
  const copied = codecPackages();
  const pinnedCodecs = Object.keys(PINNED_CODECS).sort();
  for (const name of copied) {
    if (!Object.hasOwn(PINNED_CODECS, name)) {
      problems.push(
        `build-page.mjs copies a wasm binary from ${name} and run.mjs pins no ` +
          `version for it. A decoder nobody pinned decides pixels nobody can ` +
          `reproduce.`,
      );
    }
  }
  for (const name of pinnedCodecs) {
    if (!copied.includes(name)) {
      problems.push(
        `run.mjs pins ${name} and build-page.mjs copies nothing from it, so ` +
          `the pin is on a package the page never loads.`,
      );
    }
  }

  // Two lists that could disagree, cross-checked so they cannot. The maps above
  // are what the run enforces, `package.json` is what `npm ci` installs, and a
  // pin recorded in one place and not the other is a pin that does not hold.
  const manifest = require("./package.json");
  const declared = manifest.dependencies ?? {};
  const devDeclared = manifest.devDependencies ?? {};
  for (const name of Object.keys(declared)) {
    if (Object.hasOwn(devDeclared, name)) {
      problems.push(
        `tools/oracle/package.json declares ${name} in both dependencies and ` +
          `devDependencies, so which version applies is npm's decision rather ` +
          `than this file's`,
      );
    }
  }
  const allDeclared = { ...declared, ...devDeclared };
  for (const [name, pin] of Object.entries(pins)) {
    if (allDeclared[name] !== pin) {
      problems.push(
        `run.mjs pins ${name} at ${pin} and tools/oracle/package.json ` +
          `declares ${allDeclared[name] ?? "nothing"}. A pin recorded in one ` +
          `place and not the other is a pin that does not hold.`,
      );
    }
  }
  for (const name of Object.keys(allDeclared)) {
    if (!Object.hasOwn(pins, name)) {
      problems.push(
        `tools/oracle/package.json declares ${name} and run.mjs checks no ` +
          `version for it. Every dependency of the reference is pinned in both ` +
          `places or in neither.`,
      );
    }
  }

  if (problems.length > 0) {
    throw new Error(
      `the reference stack is not at its pinned versions:\n  ` +
        problems.join("\n  ") +
        `\nRun \`npm ci\` in tools/oracle. Deviation D-11 records why the pin ` +
        `is 5.8.2 and not Appendix B's v5.8.9.`,
    );
  }
  return installed;
}

// ---------------------------------------------------------------------------
// Step 2, the pure unit tests
// ---------------------------------------------------------------------------

function run(command, args, options = {}) {
  return new Promise((resolveRun) => {
    const child = spawn(command, args, { stdio: "inherit", ...options });
    child.on("error", (error) => resolveRun({ code: 127, error }));
    child.on("close", (code) => resolveRun({ code: code ?? 1 }));
  });
}

function capture(command, args, options = {}) {
  return new Promise((resolveRun) => {
    const child = spawn(command, args, { ...options, stdio: ["ignore", "pipe", "pipe"] });
    // Decode as a stream. Coercing each Buffer independently would mangle a
    // multibyte character that landed across a chunk boundary, and this is how
    // the sidecar cross-read's output reaches the terminal.
    child.stdout.setEncoding("utf8");
    child.stderr.setEncoding("utf8");
    let stdout = "";
    let stderr = "";
    child.stdout.on("data", (chunk) => (stdout += chunk));
    child.stderr.on("data", (chunk) => (stderr += chunk));
    child.on("error", (error) => resolveRun({ code: 127, stdout, stderr: String(error) }));
    child.on("close", (code) => resolveRun({ code: code ?? 1, stdout, stderr }));
  });
}

async function runUnitTests() {
  const result = await run(
    process.execPath,
    [
      "--test",
      "tests/args_test.mjs",
      "tests/manifest_test.mjs",
      "tests/output_test.mjs",
      "tests/params_test.mjs",
      "tests/paths_test.mjs",
      "tests/pins_test.mjs",
      "tests/registration_test.mjs",
      "tests/server_test.mjs",
      "tests/sidecar_test.mjs",
      "tests/unsupported_test.mjs",
    ],
    { cwd: oraclePath() },
  );
  if (result.code !== 0) {
    throw new Error("the oracle's unit tests failed, see above");
  }
}

// ---------------------------------------------------------------------------
// The browser
// ---------------------------------------------------------------------------

/**
 * Software rasterisation, forced.
 *
 * The plan's decision 2: the reference is held still so it is the same
 * artefact on every machine. `--use-angle=swiftshader` selects ANGLE's
 * software backend and `--enable-unsafe-swiftshader` is what allows WebGL to
 * use it rather than falling back to no context at all. The resulting adapter
 * string is read from the page and asserted below, so this is a claim the run
 * checks rather than a flag nobody reads.
 */
const CHROMIUM_ARGS = [
  "--use-gl=angle",
  "--use-angle=swiftshader",
  "--enable-unsafe-swiftshader",
  "--force-device-scale-factor=1",
  "--disable-lcd-text",
  "--hide-scrollbars",
];

async function openPage(origin, params) {
  const browser = await chromium.launch({ args: CHROMIUM_ARGS });
  const context = await browser.newContext({
    deviceScaleFactor: 1,
    viewport: {
      width: params.canvas.width + 64,
      height: params.canvas.height + 64,
    },
  });
  const page = await context.newPage();

  const console_ = [];
  page.on("console", (message) => {
    console_.push(`[${message.type()}] ${message.text()}`);
  });
  page.on("pageerror", (error) => {
    console_.push(`[pageerror] ${String(error?.message ?? error)}`);
  });

  await page.goto(`${origin}/index.html`, { waitUntil: "load" });
  await page.waitForFunction("window.__oracleLoaded === true", null, {
    timeout: 60_000,
  });

  // `globalThis` and not `window`, deliberately. This arrow function is
  // written in a node file and executed in the browser, and naming the ambient
  // object the way both agree on keeps `no-undef` able to catch a node-side
  // file that reaches for a browser global by mistake.
  const environment = await page.evaluate(
    (setup) => globalThis.__oracle.ready(setup),
    {
      canvas: params.canvas,
      background: params.background,
      wasmBasePath: `${origin}/wasm/`,
    },
  );

  return {
    page,
    environment,
    console: console_,
    close: async () => {
      await context.close();
      await browser.close();
    },
  };
}

/**
 * The reference must be the software rasteriser it was launched as.
 *
 * A machine that quietly gave the page a hardware adapter would produce
 * frames that are correct and not the reference. D-07's rule generalised:
 * report unavailable, never quietly produce a different result.
 */
function assertReferenceEnvironment(environment) {
  const { rendering } = environment;
  if (!rendering.webgl2) {
    throw new Error(
      `the page has no WebGL2 context (renderer ${JSON.stringify(rendering.renderer)}). ` +
        `cornerstone3D v5 renders through vtk.js on WebGL2 and would have ` +
        `fallen back to its CPU path, which is a different reference.`,
    );
  }
  if (!rendering.softwareRasterizer) {
    throw new Error(
      `the adapter is ${JSON.stringify(rendering.renderer)}, which ` +
        `cornerstone3D does not recognise as a software rasteriser. The ` +
        `reference is rendered on SwiftShader so it is the same artefact on ` +
        `every machine.`,
    );
  }
  if (rendering.useCPURendering) {
    throw new Error(
      `cornerstone3D chose its CPU rendering path (adapter ` +
        `${JSON.stringify(rendering.renderer)}). Those are different pixels ` +
        `from the GPU path this reference is meant to be.`,
    );
  }
  if (rendering.devicePixelRatio !== 1) {
    throw new Error(
      `devicePixelRatio is ${rendering.devicePixelRatio}, so the viewport ` +
        `canvas would not be the size render-params.json declares`,
    );
  }
}

// ---------------------------------------------------------------------------
// One pass over the corpus
// ---------------------------------------------------------------------------

async function renderPass(session, rows, spec, options, pass) {
  const results = new Map();
  for (const row of rows) {
    const faulted = options.inject !== null && row === rows[0];
    if (faulted && skipsRow(options.inject)) {
      // Boundary one, injected: a row the loop never reaches. Nothing else
      // about this run is wrong, which is the point.
      continue;
    }

    const file = join(CORPUS_DATA, row.path);
    let bytes;
    try {
      bytes = await readFile(file);
    } catch (error) {
      throw new Error(
        `corpus row ${row.path} is not present under corpus/data ` +
          `(${String(error?.message ?? error)}). Populate the corpus with ` +
          `\`uv run scripts/populate_corpus.py\`. A missing row is a row the ` +
          `oracle would otherwise report as covered.`,
      );
    }
    const actual = digestOf(bytes);
    if (actual !== row.sha256) {
      throw new Error(
        `corpus row ${row.path} hashes ${actual} and the manifest says ` +
          `${row.sha256}. Reference output is only meaningful against the ` +
          `corpus the manifest describes.`,
      );
    }

    if (faulted) {
      bytes = faultedBytes(options.inject, bytes);
    }

    const params = faulted
      ? faultedParams(options.inject, resolveRenderParams(spec, row))
      : resolveRenderParams(spec, row);
    const request = {
      id: rowId(row.path),
      bytesBase64: Buffer.from(bytes).toString("base64"),
      params,
      timeoutMs: RENDER_TIMEOUT_MS,
      includePixels: pass === 1,
      fault: faulted ? pageFaultName(options.inject) : null,
    };

    const result = await session.page.evaluate(
      (payload) => globalThis.__oracle.render(payload),
      request,
    );
    results.set(row.path, { row, params, result });
    process.stdout.write(
      `  ${result.ok ? "ok  " : "FAIL"} ${row.path}` +
        (result.ok ? ` ${result.frame.sha256.slice(0, 12)}` : ` (${result.boundary})`) +
        "\n",
    );
  }
  return results;
}

// ---------------------------------------------------------------------------
// Output
// ---------------------------------------------------------------------------

// ---------------------------------------------------------------------------
// The sidecar metadata cross-read
// ---------------------------------------------------------------------------

/**
 * Which interpreters may run the sidecar cross-read, in order.
 *
 * `scripts/corpus_tests.py` stops at the same place, and the stop is the
 * important half. The candidate lists differ in their last entry, `python3`
 * here and `sys.executable` there. An explicit `$OCELLI_PYTHON` is
 * authoritative: if it cannot import pydicom, no fallback is tried, because
 * running a different interpreter from the one the operator asked for and
 * reporting success is its own quiet failure. Only the implicit candidates
 * fall through to each other.
 */
function pythonCandidates() {
  if (process.env.OCELLI_PYTHON) {
    return { explicit: true, candidates: [process.env.OCELLI_PYTHON] };
  }
  const candidates = [];
  const venv = repoPath(".venv", "bin", "python");
  if (existsSync(venv)) {
    candidates.push(venv);
  }
  candidates.push("python3");
  return { explicit: false, candidates };
}

async function checkSidecarMetadata(outDir, partial) {
  const script = oraclePath("check_sidecars.py");
  const { explicit, candidates } = pythonCandidates();
  let last = null;
  for (const interpreter of candidates) {
    // The self test first, and it doubles as the interpreter probe: it needs
    // pydicom to import, so exit 3 still means "not this one". It exercises
    // the redaction and comparison helpers, which otherwise run only on a
    // mismatch and are therefore guards no gate run ever watches.
    const probe = await capture(interpreter, [script, "--self-test"], {
      cwd: repoPath(),
    });
    if ((probe.code === 127 || probe.code === 3) && !explicit) {
      last = { interpreter, ...probe };
      continue;
    }
    if (probe.code === 127 || probe.code === 3) {
      process.stderr.write(probe.stderr);
      throw new Error(
        `OCELLI_PYTHON names ${interpreter}, which cannot run the sidecar ` +
          `cross-read. An explicit interpreter is authoritative and nothing ` +
          `else is tried, because answering with a different interpreter from ` +
          `the one that was asked for is the quiet failure this check exists ` +
          `to close.`,
      );
    }
    process.stdout.write(probe.stdout);
    if (probe.code !== 0) {
      process.stderr.write(probe.stderr);
      throw new Error(
        `check_sidecars.py's own self test failed under ${interpreter}. Its ` +
          `redaction or its comparison is broken, so what the real check says ` +
          `about the sidecars cannot be relied on either.`,
      );
    }

    // A partial run legitimately produces fewer sidecars than the manifest has
    // rows, so the completeness half of the cross-read does not apply to it.
    // Every per-sidecar comparison still does.
    const args = [script, "--out", outDir, ...(partial ? ["--partial"] : [])];
    const result = await capture(interpreter, args, { cwd: repoPath() });
    process.stdout.write(result.stdout);
    if (result.code !== 0) {
      process.stderr.write(result.stderr);
      throw new Error(
        `the sidecar metadata cross-read failed under ${interpreter}. The ` +
          `sidecar's DICOM attributes disagree with pydicom's reading of the ` +
          `same file, which would make F-011 chase a pixel difference that is ` +
          `really a metadata bug.`,
      );
    }
    return interpreter;
  }
  throw new Error(
    `no Python interpreter with pydicom was found for the sidecar metadata ` +
      `cross-read (tried ${candidates.join(", ")}). Run ` +
      `\`uv sync --locked\`, or set OCELLI_PYTHON. A skip is not a pass.` +
      (last ? `\nLast attempt said: ${last.stderr.trim()}` : ""),
  );
}

// ---------------------------------------------------------------------------
// Main
// ---------------------------------------------------------------------------

export async function runOracle(argv) {
  const options = parseArgs(argv);
  if (options.help) {
    process.stdout.write(USAGE);
    return 0;
  }

  // FIRST, before the pins, before the unit tests, before anything else that
  // can fail. Every one of those aborts before a frame exists, and the
  // previous run's output sitting beside a red gate is the thing this ordering
  // exists to prevent. `--report-unsupported` is exempt: it answers a
  // different question, exits 2, and has no business deleting the last good
  // run.
  if (!options.inject && !options.reportUnsupported) {
    await prepareOutput(options.out);
  }

  const installed = checkPins();
  if (options.unit) {
    await runUnitTests();
  }

  const allRows = await readManifest();
  const rows = options.rows
    ? allRows.filter((row) => row.path.includes(options.rows))
    : allRows;
  if (rows.length === 0) {
    throw new Error(
      `no manifest row matches --rows ${JSON.stringify(options.rows)}`,
    );
  }
  const spec = readRenderParams();
  const unsupported = readUnsupported();
  if (unsupported.cornerstone3DVersion !== installed["@cornerstonejs/core"]) {
    throw new Error(
      `unsupported.json records what cornerstone3D ` +
        `${unsupported.cornerstone3DVersion} could not decode and the ` +
        `installed version is ${installed["@cornerstonejs/core"]}. What one ` +
        `version cannot do is not a fact about another.`,
    );
  }

  await buildPage();

  const baseParams = resolveRenderParams(spec, rows[0]);
  const server = await serveDirectory(PAGE_DIST);
  let session;
  const started = Date.now();
  let passes;
  try {
    session = await openPage(server.origin, baseParams);
    assertReferenceEnvironment(session.environment);
    process.stdout.write(
      `cornerstone3D ${installed["@cornerstonejs/core"]} on ` +
        `${session.environment.rendering.renderer}, ${rows.length} row(s)\n`,
    );

    passes = [await renderPass(session, rows, spec, options, 1)];
    if (!options.once) {
      process.stdout.write("second pass, for determinism\n");
      passes.push(await renderPass(session, rows, spec, options, 2));
    }
  } finally {
    if (session) {
      await session.close();
    }
    await server.close();
  }

  return await report({
    options,
    rows,
    spec,
    passes,
    installed,
    environment: session.environment,
    consoleLines: session.console,
    unsupported,
    elapsedMs: Date.now() - started,
  });
}

async function report(context) {
  const { options, rows, spec, passes, installed, environment, unsupported,
    consoleLines, elapsedMs } = context;
  const first = passes[0];
  const problems = [];

  // ---- Boundary one, reached ------------------------------------------
  const expected = rows.map((row) => row.path);
  const missing = expected.filter((path) => !first.has(path));
  if (missing.length > 0) {
    problems.push(
      `reached: ${missing.length} of ${expected.length} manifest row(s) were ` +
        `never attempted: ${missing.join(", ")}. A row that was never ` +
        `attempted is a failure, not an absence.`,
    );
  }

  // ---- Boundaries two to four, per row ---------------------------------
  const counts = { applicable: expected.length, reached: first.size, decoded: 0, presented: 0, readBack: 0, unsupported: 0 };
  const observedUnsupported = [];
  const accountedFor = new Set();

  for (const [path, entry] of first) {
    const { row, result } = entry;
    if (result.stage?.decoded) counts.decoded += 1;
    if (result.stage?.presented) counts.presented += 1;
    if (result.stage?.readBack) counts.readBack += 1;
    if (result.ok) {
      continue;
    }
    const failure = { boundary: result.boundary, error: result.error };
    const named = entryFor(unsupported, row, failure);
    if (named) {
      counts.unsupported += 1;
      accountedFor.add(path);
      observedUnsupported.push({
        path,
        transferSyntax: row.transferSyntax,
        feature: named.feature,
        ...failure,
        why: named.why,
      });
      continue;
    }
    observedUnsupported.push({ path, transferSyntax: row.transferSyntax, ...failure });
    problems.push(
      `${failure.boundary}: ${path} (${row.transferSyntax}) failed and ` +
        `unsupported.json does not account for it: ${failure.error}`,
    );
  }

  // A claim that is no longer true is as misleading as a missing one.
  for (const path of claimedRows(unsupported)) {
    if (!first.has(path)) {
      continue;
    }
    if (!accountedFor.has(path)) {
      problems.push(
        `unsupported.json claims ${path} cannot be rendered and it was. ` +
          `Remove the entry: a stale claim reads as a known limit and hides a ` +
          `coverage gain.`,
      );
    }
  }

  // ---- Frames that passed every boundary and still say little ----------
  //
  // Not a failure. A frame saturated by the window the file itself declares is
  // the frame that file asks for. But an oracle whose reference frame is 96%
  // black and white cannot show a divergence in the values that clipped, and
  // "the row was covered" is not the same claim as "the row was measured". So
  // they are counted and named here rather than left for F-011 to discover.
  const threshold = spec.informationFloor?.extremeFractionWarnAbove;
  if (typeof threshold !== "number") {
    // Defaulting it to 1 would switch the whole check off and say nothing, and
    // a guard that silently stops running is worse than one that was never
    // written.
    throw new Error(
      `render-params.json declares no informationFloor.extremeFractionWarnAbove, ` +
        `so the check that names frames too saturated to compare would not run.`,
    );
  }
  const lowInformation = [];
  for (const [path, entry] of first) {
    if (!entry.result.ok) {
      continue;
    }
    const stats = entry.result.frame.statistics;
    const extreme = stats.blackFraction + stats.whiteFraction;
    if (extreme > threshold) {
      lowInformation.push({
        path,
        extremeFraction: Number(extreme.toFixed(6)),
        blackFraction: Number(stats.blackFraction.toFixed(6)),
        whiteFraction: Number(stats.whiteFraction.toFixed(6)),
        voi: entry.result.voi,
      });
    }
  }

  // ---- Frames fitted DOWN into the canvas -------------------------------
  //
  // Also not a failure, and also not something to leave for F-011 to notice.
  // The canvas is 512 by 512 and most corpus frames are smaller, so the camera
  // magnifies them. Two real rows are larger in both dimensions and are fitted
  // down under NEAREST, which discards source pixels. A per-modality tolerance
  // written against a magnified frame does not automatically hold for a
  // decimated one.
  const downsampled = [];
  for (const [path, entry] of first) {
    if (!entry.result.ok) {
      continue;
    }
    const plane = entry.result.cornerstoneMetadata?.imagePlaneModule ?? {};
    const scale = canvasScale({
      parallelScale: entry.result.camera.parallelScale,
      canvasHeight: entry.params.canvas.height,
      rowPixelSpacing: plane.rowPixelSpacing,
      columnPixelSpacing: plane.columnPixelSpacing,
    });
    if (scale.vertical < 1 || scale.horizontal < 1) {
      downsampled.push({
        path,
        sourceRows: entry.result.image.rows,
        sourceColumns: entry.result.image.columns,
        canvas: {
          width: entry.params.canvas.width,
          height: entry.params.canvas.height,
        },
        rowPixelSpacing: plane.rowPixelSpacing ?? null,
        columnPixelSpacing: plane.columnPixelSpacing ?? null,
        parallelScale: entry.result.camera.parallelScale,
        canvasPixelsPerSourcePixel: {
          vertical: Number(scale.vertical.toFixed(6)),
          horizontal: Number(scale.horizontal.toFixed(6)),
        },
      });
    }
  }

  // An injected run exists to be RED. Reaching the end with nothing wrong is
  // the fault having failed to fire, and it is the one outcome an injection
  // must never report as success. `runSelfTest` checks the boundary and the
  // reason, this checks the far cruder thing that the run failed at all.
  if (options.inject !== null && problems.length === 0) {
    problems.push(
      `the injected fault ${JSON.stringify(options.inject)} did not break ` +
        `anything. An injection that leaves the run green has proved nothing ` +
        `about the guard it was aimed at.`,
    );
  }

  // The four boundaries in one line. Every applicable row either produced a
  // frame or is accounted for by unsupported.json, and nothing else is a
  // legitimate outcome. Each failing row has already pushed its own problem, so
  // this is the accounting cross-check on top: it catches a counter that
  // stopped counting.
  if (counts.readBack + counts.unsupported !== counts.applicable) {
    problems.push(
      `accounting: ${counts.applicable} row(s) applicable, ` +
        `${counts.readBack} read back and ${counts.unsupported} accounted for ` +
        `by unsupported.json, which does not add up. Every row is one or the ` +
        `other.`,
    );
  }

  if (options.reportUnsupported) {
    process.stdout.write(
      `${JSON.stringify({ candidates: observedUnsupported }, null, 2)}\n`,
    );
    // Exit 2, never 0. This mode renders the corpus and then reports what
    // failed. It checks no boundary and writes nothing, and a mode that
    // answers a different question must not be able to read as a passing gate.
    // That is the same rule `bin/ocelli.sh` applies to a skipped gate.
    process.stderr.write(
      `\nREPORT ONLY: nothing was verified and no reference output was ` +
        `written. Write the reason for each candidate by hand into ` +
        `tools/oracle/unsupported.json, then run the gate.\n`,
    );
    return 2;
  }

  // ---- Determinism -----------------------------------------------------
  let determinism = { passes: passes.length, measured: false, matched: null, mismatches: [] };
  if (passes.length > 1) {
    const mismatches = [];
    for (const [path, entry] of first) {
      const second = passes[1].get(path);
      if (!second) {
        mismatches.push({ path, reason: "the second pass never attempted it" });
        continue;
      }
      if (entry.result.ok !== second.result.ok) {
        mismatches.push({
          path,
          reason: `pass one ${entry.result.ok ? "succeeded" : "failed"} and ` +
            `pass two ${second.result.ok ? "succeeded" : "failed"}`,
        });
        continue;
      }
      // A row that fails in both passes still has to fail the SAME WAY. Two
      // failures at different boundaries are two different behaviours, and
      // comparing only the boolean would call that pair deterministic.
      if (!entry.result.ok && entry.result.boundary !== second.result.boundary) {
        mismatches.push({
          path,
          reason: `failed at ${entry.result.boundary} then at ` +
            `${second.result.boundary}`,
        });
        continue;
      }
      if (entry.result.ok && entry.result.frame.sha256 !== second.result.frame.sha256) {
        mismatches.push({
          path,
          reason: `${entry.result.frame.sha256} then ${second.result.frame.sha256}`,
        });
      }
    }
    determinism = {
      passes: passes.length,
      measured: true,
      matched: mismatches.length === 0,
      mismatches,
    };
    if (mismatches.length > 0) {
      problems.push(
        `determinism: ${mismatches.length} row(s) rendered differently on the ` +
          `second pass of the same browser: ` +
          mismatches.map((m) => `${m.path} (${m.reason})`).join(", "),
      );
    }
  }

  // ---- The run record --------------------------------------------------
  const runRecord = {
    story: "F-010",
    // No `ok` field and no `problems` field. This file is written ONLY by a
    // run that passed every boundary, so its existence is the claim, and a
    // field that could only ever say `true` or `[]` would be one more thing to
    // keep true.
    //
    // `checks` is the opposite case and is why it is here. The `--no-*` flags
    // are development aids, the oracle gate passes none of them, and a record
    // that did not say which checks ran would look identical either way.
    checks: {
      unitTests: options.unit,
      sidecarCrossRead: options.metadataCheck,
      faultSelfTest: options.selfTest,
      determinismPasses: passes.length,
    },
    partial: options.rows !== null,
    inject: options.inject,
    elapsedMs,
    manifestSha256: await digestOfManifest(),
    renderParamsSha256: digestOf(await readFile(RENDER_PARAMS_PATH)),
    unsupportedSha256: digestOf(await readFile(UNSUPPORTED_PATH)),
    packages: installed,
    page: environment,
    host: { platform: platform(), release: release(), arch: arch(), hostname: hostname(), node: process.version },
    boundaries: counts,
    determinism,
    unsupportedObserved: observedUnsupported,
    lowInformation: {
      extremeFractionWarnAbove: threshold,
      rows: lowInformation,
    },
    downsampled,
    rows: [...first].map(([path, entry]) => ({
      path,
      id: rowId(path),
      ok: entry.result.ok,
      boundary: entry.result.boundary,
      error: entry.result.error ?? null,
      sha256: entry.result.ok ? entry.result.frame.sha256 : null,
    })),
  };

  // ---- Output ----------------------------------------------------------
  //
  // Written only by a run with nothing wrong with it SO FAR. The directory was
  // emptied by `prepareOutput` before the first row was read, so a run that
  // aborted anywhere before here left nothing behind, and the two checks that
  // come AFTER the write discard it again if either goes red. The invariant is
  // one sentence: this directory holds the output of one complete run that
  // passed every boundary, or it holds nothing.
  if (!options.inject && problems.length === 0) {
    // Recorded rather than thrown, like the two checks below it. `writeRow`'s
    // frame-integrity guard is DESIGNED to throw, and a throw here would go
    // straight past the single discard path and leave a half-written directory
    // with no run.json, which the next run then refuses as foreign.
    try {
      await openOutput(options.out);
      for (const [, entry] of first) {
        if (entry.result.ok) {
          await writeRow(options.out, entry, environment, installed);
        }
      }
      await writeFile(
        join(options.out, "run.json"),
        `${JSON.stringify(runRecord, null, 2)}\n`,
      );
      await writeFile(
        join(options.out, "console.log"),
        `${consoleLines.join("\n")}\n`,
      );
    } catch (error) {
      problems.push(String(error?.message ?? error));
    }
  }

  // ---- The sidecar metadata cross-read ---------------------------------
  //
  // It reads the sidecars, so it cannot run before they are written. A failure
  // here is recorded as a problem rather than thrown, so that it reaches the
  // one place below that discards the output.
  if (problems.length === 0 && options.metadataCheck) {
    try {
      await checkSidecarMetadata(options.out, options.rows !== null);
    } catch (error) {
      problems.push(String(error?.message ?? error));
    }
  }

  // ---- The fault injection self test ------------------------------------
  //
  // Also after the write, because it spawns whole runs of this driver and they
  // are slow. Its failures reach the same place.
  if (problems.length === 0 && options.selfTest) {
    const selfTest = await runSelfTest();
    problems.push(...selfTest.problems);
  }

  process.stdout.write(
    `\nboundaries  applicable ${counts.applicable}, reached ${counts.reached}, ` +
      `decoded ${counts.decoded}, presented ${counts.presented}, ` +
      `read back ${counts.readBack}, unsupported ${counts.unsupported}\n`,
  );
  if (determinism.measured) {
    process.stdout.write(
      `determinism two passes on this browser build, ` +
        `${determinism.matched ? "identical" : "DIFFERENT"}\n`,
    );
  }
  if (downsampled.length > 0) {
    process.stdout.write(
      `NOTE  ${downsampled.length} row(s) are larger than the declared canvas ` +
        `and were fitted down under NEAREST, so the reference frame is a ` +
        `decimation of the source. See downsampled in run.json.\n`,
    );
  }
  if (lowInformation.length > 0) {
    process.stdout.write(
      `NOTE  ${lowInformation.length} frame(s) are over ` +
        `${threshold * 100}% black and white together, so a divergence in the ` +
        `clipped values would not show in a pixel diff. Covered is not the ` +
        `same as measured. See lowInformation in run.json.\n`,
    );
  }

  if (problems.length > 0) {
    // The one place a failure is reported, and the one place the output is
    // discarded. Every route into here goes through it, including the two
    // checks that run after the frames were written.
    if (!options.inject) {
      await discardOutput(options.out);
    }
    process.stderr.write("\nFAIL: oracle\n");
    for (const problem of problems) {
      process.stderr.write(`  ${problem}\n`);
    }
    if (!options.inject) {
      process.stderr.write(
        `  ${options.out} is empty. A failed run leaves no reference output, ` +
          `because the previous run's frames beside a red gate would read as ` +
          `current.\n`,
      );
    }
    if (consoleLines.length > 0) {
      process.stderr.write(
        `  last page console lines:\n` +
          consoleLines
            .slice(-8)
            .map((line) => `    ${line}`)
            .join("\n") +
          "\n",
      );
    }
    return 1;
  }

  process.stdout.write(
    `OK: ${counts.readBack} reference frame(s) in ${options.out}\n`,
  );
  return 0;
}

if (isEntryPoint(import.meta.url)) {
  try {
    process.exitCode = await runOracle(process.argv.slice(2));
  } catch (error) {
    process.stderr.write(`\nFAIL: oracle\n  ${String(error?.message ?? error)}\n`);
    process.exitCode = 1;
  }
}
