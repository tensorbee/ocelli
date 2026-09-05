// The real release module loads under headless Chromium and ocelli_version()
// returns the workspace version, with a duration attached.
//
// This is the only suite here that needs a browser, so it is NOT in the `bench`
// floor gate. `bin/ocelli.sh gate bench` asserts the instrument's integrity and
// never a duration, and this asserts that the instrument can actually take one.
// Run it with `npm run test:browser` from tools/bench, or get it for free from
// `bin/ocelli.sh bench`, which fails if the subject does not measure.
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

test("the real release artefact cold starts, and the module is this tree's",
  { timeout: 600_000 }, async () => {
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
