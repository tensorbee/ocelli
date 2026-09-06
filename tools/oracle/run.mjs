#!/usr/bin/env node
// The differential harness's reference half: render every corpus row through
// the pinned cornerstone3D in headless Chromium and emit reference pixels or a
// precise failure. F-010, HLD section 11, decision D7.
//
// It does not compare anything against Ocelli. Comparison is F-011.
//
// F-X007 adds a SECOND pass over the same corpus. Where the stack pass renders
// one frame of one instance, the volume pass assembles the four series
// directories declared in `volume-params.json` into volumes and renders three
// orthogonal reformats of each, so the twenty synthetic spacing rows are asked
// something about their geometry rather than only about their pixels. It runs
// in its own page, after the stack page has been closed, so nothing it does can
// move a stack frame.
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
// The volume pass adds four more, and the first of them exists because a
// volume has a degeneracy the stack path has no analogue for:
//
//   5. VOLUME LOADED      every member parsed, the volume built, its slice
//                         count is the member count, its sorted image ids are a
//                         permutation of the members with no repeats, and its z
//                         profile is the ramp `scripts/corpus_synth.py` wrote.
//                         A volume assembled from the wrong slices, or from one
//                         slice ten times, renders a perfectly plausible frame
//                         that hashes stably.
//   6. VOLUME GEOMETRY    the harness's OWN reading of the series geometry, from
//                         the files and never from a cornerstone3D module,
//                         matches `volume-truth.json` within HLD 25.1's 1e-6 mm,
//                         and the reference either agrees with it or diverges in
//                         a way that file declares.
//   7. REFORMAT PRESENTED cornerstone3D's own IMAGE_RENDERED fired for the
//                         orientation, with a timeout. Never a fixed sleep.
//   8. REFORMAT READ BACK the reformat came back, at the declared size, not one
//                         value and not still the sentinel.
//
// Each of the eight is observed red by `--inject`, see `tests/faults.mjs`.
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
import {
  VOLUME_PARAMS_PATH,
  VOLUME_TRUTH_PATH,
  checkSubjectsAgainstManifest,
  checkZProfile,
  compareGeometry,
  comparePairs,
  frameIdFor,
  measureSubject,
  readVolumeParams,
  readVolumeTruth,
  selectSubjects,
  validateVolumeTruth,
} from "./src/volume.mjs";
import { isEntryPoint, oraclePath, repoPath } from "./src/paths.mjs";
import { installedVersion } from "./src/pins.mjs";
import { serveDirectory } from "./src/server.mjs";
import {
  discardOutput,
  isInside,
  openOutput,
  prepareOutput,
  writeRow,
  writeVolumeFrame,
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
  faultedMemberBytes,
  faultedParams,
  faultedVolumeRequest,
  faultedVolumeResult,
  pageFaultName,
  pageVolumeFaultName,
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
      "tests/geometry_test.mjs",
      "tests/manifest_test.mjs",
      "tests/output_test.mjs",
      "tests/params_test.mjs",
      "tests/paths_test.mjs",
      "tests/pins_test.mjs",
      "tests/registration_test.mjs",
      "tests/server_test.mjs",
      "tests/sidecar_test.mjs",
      "tests/unsupported_test.mjs",
      "tests/volume_test.mjs",
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

/**
 * The two pages, and what each is called in the browser.
 *
 * Two documents in two browser contexts, not two viewports in one. The volume
 * pass could probably have shared the stack page's rendering engine, since
 * cornerstone3D 5.8.2 defaults to a seven-context pool, and "probably" is not a
 * property worth spending F-011's schedule on while it is reading the stack
 * frames this story must not move.
 */
const STACK_PAGE = {
  document: "index.html",
  loadedFlag: "__oracleLoaded",
  api: "__oracle",
};
const VOLUME_PAGE = {
  document: "volume.html",
  loadedFlag: "__oracleVolumeLoaded",
  api: "__oracleVolume",
};

async function openPage(origin, params, which) {
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
    console_.push(`[${which.document}] [${message.type()}] ${message.text()}`);
  });
  page.on("pageerror", (error) => {
    console_.push(`[${which.document}] [pageerror] ${String(error?.message ?? error)}`);
  });

  await page.goto(`${origin}/${which.document}`, { waitUntil: "load" });
  await page.waitForFunction(`window.${which.loadedFlag} === true`, null, {
    timeout: 60_000,
  });

  // `globalThis` and not `window`, deliberately. This arrow function is
  // written in a node file and executed in the browser, and naming the ambient
  // object the way both agree on keeps `no-undef` able to catch a node-side
  // file that reaches for a browser global by mistake.
  const environment = await page.evaluate(
    ([api, setup]) => globalThis[api].ready(setup),
    [
      which.api,
      {
        canvas: params.canvas,
        background: params.background,
        wasmBasePath: `${origin}/wasm/`,
      },
    ],
  );

  return {
    page,
    api: which.api,
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
// One pass over the volume subjects
// ---------------------------------------------------------------------------

/**
 * Assemble and reformat every attempted subject, once.
 *
 * The member ATTRIBUTES come from the stack pass, not from a second reader.
 * `page/app.mjs` already read each file's Image Plane module straight from the
 * bytes with `dicom-parser`, `check_sidecars.py` already cross-reads that
 * against pydicom, and a second reader in the volume page would be a second
 * copy of the one thing this harness most needs to be single. The BYTES are
 * re-read from the corpus, because the page needs files to hand cornerstone3D
 * and the stack pass did not keep them.
 */
async function volumePass(session, subjects, context, options, pass) {
  const { stackResults, rowsByPath, spec, volumeParams, volumeTruth } = context;
  const results = new Map();

  for (const subject of subjects) {
    // Resolved from the subject's OWN first member and not from the run's
    // first row. `canvas` and `background` cannot be set by a rule so they are
    // the same either way, and `interpolation`, `camera` and the VOI policy
    // can be, so a subject must be given the parameters its own rows resolve.
    const params = resolveRenderParams(spec, rowsByPath.get(subject.members[0]));
    const members = [];
    for (const path of subject.members) {
      // Already digest-checked against the manifest by `renderPass`, which
      // every member went through: a subject is attempted only when all of its
      // members are in this run's selection, and every selected row is
      // rendered as a stack first.
      const bytes = await readFile(join(CORPUS_DATA, path));
      const faulted =
        options.inject !== null && path === subject.members[0];
      const sent = faulted ? faultedMemberBytes(options.inject, bytes) : bytes;
      const stack = stackResults.get(path);
      members.push({
        path,
        id: rowId(path),
        sha256: digestOf(bytes),
        stackSidecar: `${rowId(path)}.json`,
        // The reading `page/app.mjs` took during the stack pass. Null only if
        // that row failed before it, which the stack boundaries already refuse.
        attributes: stack?.result?.attributes ?? null,
        bytesBase64: Buffer.from(sent).toString("base64"),
      });
    }

    const truth = volumeTruth.subjects?.[subject.id] ?? null;
    let request = {
      id: subject.id,
      seriesDirectory: subject.seriesDirectory,
      members,
      orientations: volumeParams.orientations,
      params,
      blendMode: volumeParams.blendMode,
      slabThicknessMm: volumeParams.slabThicknessMm,
      cameraMode: volumeParams.cameraMode,
      loadTimeoutMs: volumeParams.loadTimeoutMs,
      renderTimeoutMs: RENDER_TIMEOUT_MS,
      zProfileVoxel: truth?.zProfile?.voxel ?? null,
      includePixels: pass === 1,
      fault: options.inject !== null ? pageVolumeFaultName(options.inject) : null,
    };
    if (options.inject !== null) {
      request = faultedVolumeRequest(options.inject, request);
    }

    const pageResult = await session.page.evaluate(
      ([api, payload]) => globalThis[api].renderSubject(payload),
      [session.api, request],
    );
    // The members travel back with the result so that a fault aimed at the
    // geometry boundary has one object to perturb, and so the sidecar's member
    // block and the geometry are computed from the same list.
    let record = { ...pageResult, members };
    if (options.inject !== null) {
      record = faultedVolumeResult(options.inject, record);
    }

    results.set(subject.id, { subject, params, request, truth, result: record });
    const digests = (record.frames ?? [])
      .map((frame) => `${frame.orientation} ${frame.frame.sha256.slice(0, 12)}`)
      .join("  ");
    process.stdout.write(
      `  ${record.ok ? "ok  " : "FAIL"} ${subject.id} ` +
        `(${subject.members.length} member(s))` +
        (record.ok ? `  ${digests}` : ` (${record.boundary})`) +
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

  // The two committed volume declarations, read and cross-checked before a
  // browser starts. Both are strict in both directions: every declared member
  // is a manifest row, every manifest row carrying the `series` category token
  // belongs to exactly one subject, and every subject has a truth entry.
  const volumeParams = readVolumeParams();
  const volumeTruth = validateVolumeTruth(readVolumeTruth(), volumeParams);
  if (volumeTruth.cornerstone3DVersion !== installed["@cornerstonejs/core"]) {
    throw new Error(
      `volume-truth.json records what cornerstone3D ` +
        `${volumeTruth.cornerstone3DVersion} does with these series and the ` +
        `installed version is ${installed["@cornerstonejs/core"]}. What one ` +
        `version does is not a fact about another.`,
    );
  }
  const manifestProblems = checkSubjectsAgainstManifest(volumeParams, allRows);
  if (manifestProblems.length > 0) {
    throw new Error(
      `the volume subject declaration and the corpus disagree:\n  ` +
        manifestProblems.join("\n  "),
    );
  }
  const selected = selectSubjects(
    volumeParams,
    new Set(rows.map((row) => row.path)),
  );
  if (selected.problems.length > 0) {
    throw new Error(selected.problems.join("\n  "));
  }

  await buildPage();

  const baseParams = resolveRenderParams(spec, rows[0]);
  const server = await serveDirectory(PAGE_DIST);
  let session;
  let volumeSession = null;
  const started = Date.now();
  let passes;
  let volumePasses = null;
  try {
    session = await openPage(server.origin, baseParams, STACK_PAGE);
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

    // The stack page is CLOSED before the volume page opens. Sequential and
    // not concurrent, so the stack render is provably the same program it was
    // and the two passes cannot contend for one browser's decode worker.
    await session.close();
    session.closed = true;

    if (selected.attempted.length > 0) {
      const volumeContext = {
        stackResults: passes[0],
        rowsByPath: new Map(rows.map((row) => [row.path, row])),
        spec,
        volumeParams,
        volumeTruth,
      };
      volumeSession = await openPage(server.origin, baseParams, VOLUME_PAGE);
      assertReferenceEnvironment(volumeSession.environment);
      process.stdout.write(
        `volume pass, ${selected.attempted.length} subject(s), ` +
          `${volumeParams.orientations.join(", ")}\n`,
      );
      volumePasses = [
        await volumePass(volumeSession, selected.attempted, volumeContext, options, 1),
      ];
      if (!options.once) {
        process.stdout.write("second volume pass, for determinism\n");
        volumePasses.push(
          await volumePass(volumeSession, selected.attempted, volumeContext, options, 2),
        );
      }
    }
  } finally {
    if (session && !session.closed) {
      await session.close();
    }
    if (volumeSession) {
      await volumeSession.close();
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
    consoleLines: [...session.console, ...(volumeSession?.console ?? [])],
    unsupported,
    volumeParams,
    volumeTruth,
    volumeSubjects: selected.attempted,
    volumePasses,
    volumeEnvironment: volumeSession?.environment ?? null,
    elapsedMs: Date.now() - started,
  });
}

async function report(context) {
  const { options, rows, spec, passes, installed, environment, unsupported,
    consoleLines, elapsedMs, volumeParams, volumeTruth, volumeSubjects,
    volumePasses, volumeEnvironment } = context;
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
        kind: "stack",
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

  // ---- The volume pass -------------------------------------------------
  //
  // Four more boundaries. The geometry one is here rather than in the page,
  // because only the driver holds `volume-truth.json` and because the
  // measurement is taken from the FILES by `src/geometry.mjs`, which never
  // reads a cornerstone3D module. A harness that transcribed the reference's
  // own answer could not show the reference getting a series wrong, and on the
  // non-uniform subject it does.
  const volumeCounts = {
    volumesApplicable: volumeSubjects.length,
    volumesBuilt: 0,
    volumesRefused: 0,
    volumesRefusedAsDeclared: 0,
    // ALL THREE MEAN ACHIEVED, and they mean it the way the stack half's
    // `readBack` does: a reformat counted here is one that survived every
    // boundary and is a file in `out/`.
    //
    // They read the declared total once, and it was a defect. The volume
    // pass's boundaries do not run in the order they are listed:
    // `volume-geometry` is the DRIVER's and runs LAST, after the page has
    // presented and read back every orientation. So a subject refused there
    // has already rendered three reformats that are not reference output, and
    // adding them here made one `boundaries` object use `readBack` to mean
    // "achieved and written" for stacks and "attempted and discarded" for
    // reformats. `readBack: 89` equals the files on disk, so
    // `reformatsReadBack` has to as well.
    //
    // Nothing is lost by that. What a refused subject reached is recorded per
    // subject in `volumes[]`, which is where a fact about one subject belongs.
    reformatsPresented: 0,
    reformatsReadBack: 0,
    reformatsWritten: 0,
    // The DECLARED total, under a name that says so. Four subjects at three
    // orientations is twelve, whatever happens to any of them.
    reformatsDeclared:
      volumeSubjects.length * volumeParams.orientations.length,
  };
  const volumeRecords = [];
  const volumeFrames = {};
  const volumeWritable = [];
  const volumeFirst = volumePasses ? volumePasses[0] : new Map();

  for (const subject of volumeSubjects) {
    const entry = volumeFirst.get(subject.id);
    if (!entry) {
      problems.push(
        `volume-loaded: subject ${subject.id} was never attempted. A subject ` +
          `that was never attempted is a failure, not an absence.`,
      );
      volumeCounts.volumesRefused += 1;
      continue;
    }
    const { result, truth } = entry;
    // What the page reached for THIS subject, recorded on the subject rather
    // than added to a total. A refused subject's own record then says how far
    // it got, and the totals stay a statement about the reference output.
    const reached = {
      reformatsPresented: result.stage?.reformats ?? 0,
      reformatsReadBack: (result.frames ?? []).length,
    };

    // `volume-truth.json`'s per-subject `expectedRefusal`, which is the volume
    // pass's `unsupported.json`. A series that CANNOT be one volume is
    // declared and named with its reason rather than silently dropped, and the
    // claim is strict in both directions: an undeclared refusal fails the run
    // and a declared refusal that does not occur fails it too.
    const expected = truth?.expectedRefusal ?? null;
    const accountedFor = (boundary, error) =>
      expected !== null &&
      expected.boundary === boundary &&
      String(error ?? "").includes(expected.errorContains);

    if (!result.ok) {
      volumeCounts.volumesRefused += 1;
      const named = accountedFor(result.boundary, result.error);
      if (named) {
        volumeCounts.volumesRefusedAsDeclared += 1;
      } else {
        problems.push(
          `${result.boundary}: volume subject ${subject.id} failed and ` +
            `volume-truth.json does not account for it: ${result.error}`,
        );
      }
      volumeRecords.push({
        id: subject.id,
        seriesDirectory: subject.seriesDirectory,
        memberCount: subject.members.length,
        outcome: named ? "refused-as-declared" : "refused",
        boundary: result.boundary,
        error: result.error,
        expectedRefusal: named ? expected : null,
        ...reached,
        frames: [],
      });
      continue;
    }

    // The harness's own geometry. `measureSubject` refuses a member with no
    // Image Plane attributes, a series whose members disagree about the
    // orientation or the pixel spacing, direction cosines that are not unit
    // and perpendicular, and two members at one position. Each of those is a
    // series that cannot be one volume, and each has a unit test.
    let measured;
    try {
      measured = measureSubject(
        result.members.map((member) => ({
          path: member.path,
          imagePositionPatient: member.attributes?.imagePositionPatient ?? null,
          imageOrientationPatient: member.attributes?.imageOrientationPatient ?? null,
          pixelSpacing: member.attributes?.pixelSpacing ?? null,
        })),
      );
    } catch (error) {
      volumeCounts.volumesRefused += 1;
      const message = String(error?.message ?? error);
      const named = accountedFor("volume-geometry", message);
      if (named) {
        volumeCounts.volumesRefusedAsDeclared += 1;
      } else {
        problems.push(`volume-geometry: ${subject.id}: ${message}`);
      }
      volumeRecords.push({
        id: subject.id,
        seriesDirectory: subject.seriesDirectory,
        memberCount: subject.members.length,
        outcome: named ? "refused-as-declared" : "refused",
        boundary: "volume-geometry",
        error: message,
        expectedRefusal: named ? expected : null,
        // This subject DID render its reformats. The page presented and read
        // back three, and the driver then refused it, so none of the three is
        // reference output and none is in the totals. Recorded here so that
        // work is visible rather than erased.
        ...reached,
        frames: [],
      });
      continue;
    }

    // Past every refusal, so this subject is a volume. Counted here rather than
    // at the build, so `volumesBuilt` and `volumesRefused` are exclusive and
    // the accounting identity below means something.
    volumeCounts.volumesBuilt += 1;
    volumeCounts.reformatsPresented += reached.reformatsPresented;
    volumeCounts.reformatsReadBack += reached.reformatsReadBack;
    if (expected !== null) {
      problems.push(
        `volume-truth.json declares that ${subject.id} is refused at ` +
          `${expected.boundary} and it was not. Remove the entry: a stale ` +
          `claim reads as a known limit and hides a subject that became a ` +
          `volume.`,
      );
    }

    const comparison = compareGeometry({
      subjectId: subject.id,
      measured,
      referenceGeometry: result.referenceGeometry,
      truth,
      toleranceMm: volumeTruth.toleranceMm,
    });
    for (const problem of comparison.problems) {
      problems.push(`volume-geometry: ${problem}`);
    }

    // The z profile, where the subject's content declares one. The volume's
    // version of the stack page's identity check, and the only one that reads
    // the assembled voxels.
    if (truth?.zProfile) {
      if (result.zProfile === null) {
        problems.push(
          `volume-loaded: ${subject.id}: volume-truth.json declares a z ` +
            `profile and the page sampled none`,
        );
      } else {
        for (const problem of checkZProfile(
          subject.id,
          result.zProfile.values,
          truth.zProfile,
        )) {
          problems.push(`volume-loaded: ${problem}`);
        }
      }
    }

    volumeFrames[subject.id] = Object.fromEntries(
      result.frames.map((frame) => [frame.orientation, frame.frame.sha256]),
    );
    volumeRecords.push({
      id: subject.id,
      seriesDirectory: subject.seriesDirectory,
      memberCount: subject.members.length,
      outcome: "built",
      boundary: null,
      error: null,
      referenceAgreesWithTruth: comparison.referenceAgreesWithTruth,
      referenceDivergence: comparison.referenceDivergence,
      voiMembersAgree: result.voiMembersAgree,
      ...reached,
      referenceGeometry: result.referenceGeometry,
      measuredGeometry: {
        normal: measured.normal,
        gapsMm: measured.gapsMm,
        meanGapMm: measured.meanGapMm,
        medianGapMm: measured.medianGapMm,
        minGapMm: measured.minGapMm,
        maxGapMm: measured.maxGapMm,
        maxDeviationFromMeanMm: measured.maxDeviationFromMeanMm,
        uniform: comparison.uniform,
        toleranceMm: comparison.truth.toleranceMm,
        voxelAxes: measured.voxelAxes,
      },
      zProfile: result.zProfile,
      frames: result.frames.map((frame) => ({
        id: frameIdFor(subject.id, frame.orientation),
        orientation: frame.orientation,
        sha256: frame.frame.sha256,
      })),
    });
    volumeWritable.push({ entry, measured, comparison });
    volumeCounts.reformatsWritten += result.frames.length;

    // A saturated reformat is the same fact as a saturated stack frame and is
    // counted the same way, under the same declared threshold.
    for (const frame of result.frames) {
      const stats = frame.frame.statistics;
      const extreme = stats.blackFraction + stats.whiteFraction;
      if (extreme > threshold) {
        lowInformation.push({
          kind: "volume-reformat",
          id: frameIdFor(subject.id, frame.orientation),
          extremeFraction: Number(extreme.toFixed(6)),
          blackFraction: Number(stats.blackFraction.toFixed(6)),
          whiteFraction: Number(stats.whiteFraction.toFixed(6)),
          voi: result.voi,
        });
      }
    }
  }

  // The falsifiable prediction, checked rather than assumed. cornerstone3D
  // 5.8.2 takes the mean gap from the endpoints alone, so the uniform and the
  // non-uniform subjects hand it the same grid, and it is predicted to render
  // them to bit-identical reformats. If it ever stops averaging, this goes red
  // and names the orientation.
  //
  // Only pairs whose BOTH subjects this run attempted. A `--rows` selection
  // that picks up one side of a pair and not the other has not falsified
  // anything, and failing it would make every partial run red for a reason
  // that is about the selection rather than about the reference. A pair whose
  // subjects WERE attempted and produced no frames still fails, which is the
  // case that matters.
  const attemptedIds = new Set(volumeSubjects.map((subject) => subject.id));
  const applicablePairs = (volumeTruth.framePairs ?? []).filter(
    (pair) => attemptedIds.has(pair.left) && attemptedIds.has(pair.right),
  );
  const pairs = comparePairs(applicablePairs, volumeFrames);
  problems.push(...pairs.problems);

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

  // Every applicable subject either became a volume or was refused, and
  // nothing else is a legitimate outcome. Each refused subject has already
  // pushed its own problem unless volume-truth.json accounts for it, so this
  // is the accounting cross-check on top: it catches a counter that stopped
  // counting.
  if (volumeCounts.volumesBuilt + volumeCounts.volumesRefused !==
      volumeCounts.volumesApplicable) {
    problems.push(
      `accounting: ${volumeCounts.volumesApplicable} volume subject(s) ` +
        `applicable, ${volumeCounts.volumesBuilt} built and ` +
        `${volumeCounts.volumesRefused} refused, which does not add up. Every ` +
        `subject is one or the other.`,
    );
  }
  // The identity over the WHOLE TRIO, not over one counter of it.
  //
  // Written for `reformatsWritten` alone at first, and that is exactly why the
  // defect it was meant to catch survived: `reformatsPresented` and
  // `reformatsReadBack` were counting the declared total while the identity
  // watched only the third number. A counter with no identity on it is a
  // counter nothing can contradict.
  //
  // All three describe the same set of frames, the ones that survived every
  // boundary and reached `out/`, so all three are the same number and that
  // number is fixed by the subjects that became volumes.
  const expectedReformats =
    volumeCounts.volumesBuilt * volumeParams.orientations.length;
  for (const [name, count] of [
    ["reformatsPresented", volumeCounts.reformatsPresented],
    ["reformatsReadBack", volumeCounts.reformatsReadBack],
    ["reformatsWritten", volumeCounts.reformatsWritten],
  ]) {
    if (count !== expectedReformats) {
      problems.push(
        `accounting: ${volumeCounts.volumesBuilt} volume(s) built at ` +
          `${volumeParams.orientations.length} orientation(s) each is ` +
          `${expectedReformats} reference reformat(s), and ${name} is ` +
          `${count}. Every one of the three counts the frames that survived ` +
          `every boundary and reached the output directory, so a difference ` +
          `is one of them counting something else.`,
      );
    }
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
        mismatches.push({ kind: "stack", path, reason: "the second pass never attempted it" });
        continue;
      }
      if (entry.result.ok !== second.result.ok) {
        mismatches.push({
          kind: "stack",
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
          kind: "stack",
          path,
          reason: `failed at ${entry.result.boundary} then at ` +
            `${second.result.boundary}`,
        });
        continue;
      }
      if (entry.result.ok && entry.result.frame.sha256 !== second.result.frame.sha256) {
        mismatches.push({
          kind: "stack",
          path,
          reason: `${entry.result.frame.sha256} then ${second.result.frame.sha256}`,
        });
      }
    }

    // The volume pass is measured the same way and reported in the same list,
    // with `id` where a stack entry uses `path`, because a volume frame is not
    // a corpus row. A subject that failed both times still has to fail the
    // SAME WAY, for the reason the stack half gives.
    if (volumePasses && volumePasses.length > 1) {
      for (const [id, entry] of volumeFirst) {
        const second = volumePasses[1].get(id);
        if (!second) {
          mismatches.push({ kind: "volume-reformat", id, reason: "the second pass never attempted it" });
          continue;
        }
        if (entry.result.ok !== second.result.ok) {
          mismatches.push({
            kind: "volume-reformat",
            id,
            reason: `pass one ${entry.result.ok ? "succeeded" : "failed"} and ` +
              `pass two ${second.result.ok ? "succeeded" : "failed"}`,
          });
          continue;
        }
        if (!entry.result.ok) {
          if (entry.result.boundary !== second.result.boundary) {
            mismatches.push({
              kind: "volume-reformat",
              id,
              reason: `failed at ${entry.result.boundary} then at ${second.result.boundary}`,
            });
          }
          continue;
        }
        for (const frame of entry.result.frames) {
          const other = second.result.frames.find(
            (candidate) => candidate.orientation === frame.orientation,
          );
          if (!other) {
            mismatches.push({
              kind: "volume-reformat",
              id: frameIdFor(id, frame.orientation),
              reason: "the second pass produced no frame for this orientation",
            });
            continue;
          }
          if (frame.frame.sha256 !== other.frame.sha256) {
            mismatches.push({
              kind: "volume-reformat",
              id: frameIdFor(id, frame.orientation),
              reason: `${frame.frame.sha256} then ${other.frame.sha256}`,
            });
          }
        }
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
        `determinism: ${mismatches.length} frame(s) rendered differently on the ` +
          `second pass of the same browser: ` +
          mismatches.map((m) => `${m.path ?? m.id} (${m.reason})`).join(", "),
      );
    }
  }

  // ---- The run record --------------------------------------------------
  const runRecord = {
    story: "F-010, F-X007",
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
      // False only when this run's row selection contained no complete volume
      // subject. A run that skipped the volume pass must not be
      // indistinguishable from one that did it.
      volumePass: volumePasses !== null,
    },
    partial: options.rows !== null,
    inject: options.inject,
    elapsedMs,
    manifestSha256: await digestOfManifest(),
    renderParamsSha256: digestOf(await readFile(RENDER_PARAMS_PATH)),
    unsupportedSha256: digestOf(await readFile(UNSUPPORTED_PATH)),
    volumeParamsSha256: digestOf(await readFile(VOLUME_PARAMS_PATH)),
    volumeTruthSha256: digestOf(await readFile(VOLUME_TRUTH_PATH)),
    packages: installed,
    page: environment,
    volumePage: volumeEnvironment,
    host: { platform: platform(), release: release(), arch: arch(), hostname: hostname(), node: process.version },
    boundaries: { ...counts, ...volumeCounts },
    determinism,
    unsupportedObserved: observedUnsupported,
    lowInformation: {
      extremeFractionWarnAbove: threshold,
      rows: lowInformation,
    },
    downsampled,
    // STACK ONLY, and it stays that way. The accounting identity
    // `readBack + unsupported === applicable` is asserted over this array, and
    // mixing volume frames into it would break an assertion that currently
    // means something exact. Every entry carries `kind` so F-011 dispatches on
    // the field rather than on the array it came out of.
    rows: [...first].map(([path, entry]) => ({
      kind: "stack",
      path,
      id: rowId(path),
      ok: entry.result.ok,
      boundary: entry.result.boundary,
      error: entry.result.error ?? null,
      sha256: entry.result.ok ? entry.result.frame.sha256 : null,
    })),
    volumes: volumeRecords,
    framePairs: pairs.records,
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
      for (const { entry, measured, comparison } of volumeWritable) {
        for (const frame of entry.result.frames) {
          await writeVolumeFrame(options.out, entry, frame, {
            measured,
            comparison,
            volumeParams,
            environment: volumeEnvironment,
            installed,
          });
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
  //
  // **The try is the point and it was missing.** `runSelfTest` returns its
  // findings in `problems`, and it can also THROW, from its own `mkdtemp`, the
  // driver spawn or the `rm`. An uncaught throw here reaches the entry point's
  // catch, which prints `FAIL: oracle` and exits 1 WITHOUT reaching
  // `discardOutput` below, leaving a complete-looking output directory behind
  // a red run. That is the state this file's own guarantee says cannot exist,
  // and `checkSidecarMetadata` above was already wrapped for exactly this
  // reason while this was not. The S03 review's tenth pass measured the
  // asymmetry.
  if (problems.length === 0 && options.selfTest) {
    try {
      const selfTest = await runSelfTest();
      problems.push(...selfTest.problems);
    } catch (error) {
      problems.push(
        `the fault injection self test threw rather than reporting: ` +
          `${String(error?.message ?? error)}`,
      );
    }
  }

  process.stdout.write(
    `\nboundaries  applicable ${counts.applicable}, reached ${counts.reached}, ` +
      `decoded ${counts.decoded}, presented ${counts.presented}, ` +
      `read back ${counts.readBack}, unsupported ${counts.unsupported}\n`,
  );
  if (volumePasses !== null) {
    process.stdout.write(
      `volumes     applicable ${volumeCounts.volumesApplicable}, built ` +
        `${volumeCounts.volumesBuilt}, refused ${volumeCounts.volumesRefused} ` +
        `(${volumeCounts.volumesRefusedAsDeclared} accounted for by ` +
        `volume-truth.json), reformats declared ` +
        `${volumeCounts.reformatsDeclared}, presented ` +
        `${volumeCounts.reformatsPresented}, read back ` +
        `${volumeCounts.reformatsReadBack}, written ` +
        `${volumeCounts.reformatsWritten}\n`,
    );
    for (const pair of pairs.records) {
      const identical = pair.orientations.filter((entry) => entry.identical).length;
      process.stdout.write(
        `frame pair  ${pair.left} against ${pair.right}: ${identical} of ` +
          `${pair.orientations.length} orientation(s) identical, expected ` +
          `${pair.expect}\n`,
      );
    }
  } else {
    process.stdout.write(
      `volumes     none, this run's row selection contains no complete volume ` +
        `subject. run.json records checks.volumePass false.\n`,
    );
  }
  if (determinism.measured) {
    process.stdout.write(
      `determinism two passes on this browser build, ` +
        `${determinism.matched ? "identical" : "DIFFERENT"}\n`,
    );
  }
  const disagreeing = volumeRecords.filter(
    (record) => record.voiMembersAgree === false,
  );
  if (disagreeing.length > 0) {
    process.stdout.write(
      `NOTE  ${disagreeing.length} volume subject(s) have members that resolve ` +
        `DIFFERENT windows, and one volume viewport carries one voiRange, so ` +
        `the frame was rendered with the first member's. Every member's own ` +
        `window is in the sidecar under voiMembers. See volumes in run.json.\n`,
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
    `OK: ${counts.readBack} stack and ${volumeCounts.reformatsWritten} volume ` +
      `reformat reference frame(s) in ${options.out}\n`,
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
