// Four pure suites over the cold-start runner, and one that needs a browser.
//
// **Only the last test needs one**, and until the S03 review's fourth pass the
// whole file was outside the `bench` gate on the strength of it. The cause was
// mechanical rather than a judgement: `wasm_cold_start.mjs` imported
// `playwright` at module scope, so this file could not be LOADED without a
// playwright install, and `workspaceVersion`, `median`, `atClockPrecision`,
// which is this story's one stated rounding decision, and `resolveServedPath`,
// which refuses `/..`, `/%2e%2e/`, a malformed escape and a sibling directory
// sharing a prefix, were watched by nothing. The runner now imports playwright
// inside `run()`, and the four pure tests are in the floor.
//
// The browser test is opted into with `OCELLI_BENCH_BROWSER=1`, which
// `npm run test:browser` in tools/bench sets. It builds the release artefact
// and launches sixteen Chromiums, so it is not something a floor gate can run,
// and `bin/ocelli.sh bench` exercises the same path anyway by failing if the
// subject does not measure.
//
// It builds the artefact rather than reusing whatever is in
// crates/ocelli-wasm/pkg, for the reason the runner's refusal gives: a figure
// taken against an artefact this run did not build is a figure about whatever
// happened to be lying there.

import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import { mkdtemp, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import test from "node:test";

import {
  atClockPrecision,
  median,
  resolveServedPath,
  run,
  workspaceVersion,
} from "../src/runners/wasm_cold_start.mjs";
import { repoPath } from "../src/paths.mjs";

test("the workspace version comes from [workspace.package]", () => {
  assert.equal(
    workspaceVersion('[package]\nversion = "9.9.9"\n\n' +
      '[workspace.package]\nversion = "0.2.1"\nedition = "2024"\n'),
    "0.2.1",
    "the version was read from the wrong section",
  );
  assert.throws(() => workspaceVersion('[package]\nversion = "1.0.0"\n'),
    /no \[workspace\.package\] section/);
  assert.throws(() => workspaceVersion("[workspace.package]\nedition = \"2024\"\n"),
    /declares no version/);
});

test("the median is the middle, and an even count averages the two", () => {
  assert.equal(median([3, 1, 2]), 2);
  assert.equal(median([4, 1, 3, 2]), 2.5);
  assert.throws(() => median([]), /no values/);
});

test("a duration is recorded at the precision the clock has", () => {
  assert.equal(atClockPrecision(2.2000000029802322), 2.2);
  assert.equal(atClockPrecision(0.7000000029802322), 0.7);
  // A thousand times finer than the 0.1 ms quantum, so it cannot move a figure
  // across a tolerance boundary.
  assert.equal(atClockPrecision(2.00005), 2.0001);
});

test("the served path refusal fires on a walk out of the directory", () => {
  const base = "/tmp/page";
  assert.equal(resolveServedPath(base, "/"), "/tmp/page/index.html");
  assert.equal(resolveServedPath(base, "/app.mjs"), "/tmp/page/app.mjs");
  assert.equal(resolveServedPath(base, "/../secrets"), null);
  assert.equal(resolveServedPath(base, "/%2e%2e/secrets"), null);
  assert.equal(resolveServedPath(base, "/%zz"), null,
    "a malformed percent escape reached the filesystem");
  assert.equal(resolveServedPath(base, "/../page-other/x"), null,
    "a sibling directory sharing a prefix was served");
});

/**
 * Why the one browser test is opted into rather than detected.
 *
 * Detecting a playwright install would run it on every developer machine that
 * happens to have one, including inside `bin/ocelli.sh gate bench`, and the
 * test builds a release wasm artefact and launches sixteen browsers. An
 * explicit variable makes the cost a choice, and the skip message names the
 * variable so nobody has to find this comment.
 */
const BROWSER_TEST_SKIP = process.env.OCELLI_BENCH_BROWSER === "1"
  ? false
  : "needs a browser and a release wasm build. Set OCELLI_BENCH_BROWSER=1, " +
    "or run `npm run test:browser` in tools/bench";

test("the real release artefact cold starts, and the module is this tree's",
  { timeout: 600_000, skip: BROWSER_TEST_SKIP }, async () => {
    const outDir = await mkdtemp(join(tmpdir(), "ocelli-bench-"));
    try {
      const result = await run({ outDir, build: true });
      const expected = workspaceVersion(
        await readFile(repoPath("Cargo.toml"), "utf8"),
      );

      assert.ok(result.value > 0,
        "a cold start that took no time did not happen");
      assert.equal(result.detail.version, expected);
      assert.equal(result.detail.profile, "release");
      assert.equal(result.detail.iterations_kept,
        result.detail.iterations_run - 1);
      assert.equal(result.detail.artefact.bytes > 0, true);
      assert.match(result.detail.artefact.sha256, /^[0-9a-f]{64}$/);
      assert.match(result.instrument.chromium, /^\d+\./);

      // Every phase is present and none is negative. A phase missing from the
      // page would otherwise leave the total describing fewer steps than the
      // definition names, which is the measurement quietly getting easier.
      const phases = result.detail.phases_median_ms;
      assert.deepEqual(Object.keys(phases), [
        "glue_module_script", "fetch", "compile", "instantiate", "first_call",
      ]);
      for (const [name, duration] of Object.entries(phases)) {
        assert.ok(duration >= 0, `${name} reported ${duration} ms`);
      }

      // The recorded figure is one of the observations, not something derived
      // out of range of them.
      const [low, high] = result.detail.observed_range_ms;
      assert.ok(result.value >= low && result.value <= high,
        `the median ${result.value} sits outside the observed range ` +
          `${low} to ${high}`);
    } finally {
      await rm(outDir, { recursive: true, force: true });
    }
  });
