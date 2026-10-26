// The benchmark command is a guard as well as an instrument. These cases are
// written from its documented command contract and the registry rule that a
// pending subject cannot have a runner. Every refusal in run.mjs must be
// reached directly so the census can count the file as watched.

import { Buffer } from "node:buffer";
import assert from "node:assert/strict";
import test from "node:test";
import { mkdtemp, readFile, writeFile, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { pathToFileURL } from "node:url";

import * as command from "../run.mjs";
import { runnerPath, repoPath } from "../src/paths.mjs";
import { cpuFraction, runBrowserSession } from "../src/browser_session.mjs";

const ARGUMENT_REFUSALS = [
  [["--out"], /--out needs a value/],
  [["--tolerance", "zero"], /takes a fraction above 0 and at most 1/],
  [["--not-an-option"], /unknown argument --not-an-option/],
  [["--accept"], /--accept needs --story and --why/],
  [["--tolerance", "0.25"], /--tolerance needs --tolerance-why/],
  [["--tolerance-why", "measured"],
    /--tolerance-why only means something with --tolerance/],
  [["--accept", "--story", "F-000", "--why", "probe", "--no-build"],
    /--accept refuses --no-build/],
  [["--subject", "wasm.cold_start"],
    /--subject only means something with --accept/],
];

for (const [argv, expected] of ARGUMENT_REFUSALS) {
  test(`parseArgs refuses ${argv.join(" ")}`, () => {
    assert.throws(() => command.parseArgs(argv), expected);
  });
}

test("subject execution refuses a runner for a pending subject", async () => {
  assert.equal(
    typeof command.runSubjects,
    "function",
    "runSubjects must be the exported production seam, not a test wrapper",
  );
  await assert.rejects(
    command.runSubjects([
      {
        subject: {
          id: "wasm.cold_start",
          subject_story: "F-999",
        },
        subjectExists: false,
        storyStatus: "pending",
      },
    ], { out: "unused", build: false }),
    /subject story F-999 is pending rather than in-progress or done/,
  );
});

const PRESENTED_RUNNERS = [
  "render.first_frame",
  "interaction.window_level_drag",
  "interaction.scroll",
  "session.cpu_interactive",
  "session.cpu_idle",
  "cine.frame_change_rate",
];

test("registered presentation subjects expose permanent runners", async () => {
  for (const id of PRESENTED_RUNNERS) {
    const loaded = await import(pathToFileURL(runnerPath(id)).href);
    assert.equal(loaded.id, id);
    assert.equal(typeof loaded.run, "function");
  }
});

test("CPU acceptance subjects require the representative host configuration", async () => {
  const prior = process.env.OCELLI_CPU_BASELINE_HOST;
  delete process.env.OCELLI_CPU_BASELINE_HOST;
  try {
    for (const subject of ["session.cpu_interactive", "session.cpu_idle", "cine.frame_change_rate"]) {
      await assert.rejects(() => runBrowserSession(subject, { build: false }), /representative GPU-less host measurement is unavailable/);
    }
  } finally {
    if (prior !== undefined) process.env.OCELLI_CPU_BASELINE_HOST = prior;
  }
});

test("session configuration refuses undefined subjects and invalid windows before a build", async () => {
  await assert.rejects(() => runBrowserSession("unknown"), /unknown session measurement/);
  await assert.rejects(() => runBrowserSession("render.first_frame", { iterations: 0 }), /iterations or duration/);
  await assert.rejects(() => runBrowserSession("render.first_frame", { durationMs: 0 }), /iterations or duration/);
  await assert.rejects(() => runBrowserSession("render.first_frame", { workerTransform: text => text }), /diagnostic mode/);
});

test("CPU acceptance refuses an unrelated host attestation", async () => {
  const directory = await mkdtemp(join(tmpdir(), "ocelli-host-"));
  const previous = process.env.OCELLI_CPU_BASELINE_HOST;
  try {
    const config = join(directory, "host.json");
    await writeFile(config, JSON.stringify({ representativeGpuLess: true, incumbent: "synthetic control", host: {} }));
    process.env.OCELLI_CPU_BASELINE_HOST = config;
    await assert.rejects(() => runBrowserSession("session.cpu_idle", { build: false }), /physical host/);
  } finally {
    if (previous === undefined) delete process.env.OCELLI_CPU_BASELINE_HOST;
    else process.env.OCELLI_CPU_BASELINE_HOST = previous;
    await rm(directory, { recursive: true });
  }
});

test("CPU fraction sums dedicated browser processes and rejects invalid intervals", () => {
  const before = [{ id: 1, cpuTime: 1 }, { id: 2, cpuTime: 4 }];
  const after = [{ id: 1, cpuTime: 1.25 }, { id: 2, cpuTime: 4.75 }];
  assert.equal(cpuFraction(before, after, 2), 0.5);
  assert.throws(() => cpuFraction(before, [{ id: 1, cpuTime: 2 }, { id: 1, cpuTime: 2 }], 2), /counter is invalid/);
  assert.throws(() => cpuFraction(before, after, 0), /wall interval/);
  assert.throws(() => cpuFraction(before, [{ id: 1, cpuTime: 2 }], 2), /process set changed/);
  assert.throws(() => cpuFraction(before, [{ id: 1, cpuTime: 0 }, { id: 2, cpuTime: 5 }], 2), /counter is invalid/);
});

test("release worker instrument observes a first frame and a real pointer batch", {
  skip: process.env.OCELLI_BENCH_BROWSER !== "1",
}, async () => {
  const first = await runBrowserSession("render.first_frame", { iterations: 2 });
  assert.ok(first.value > 0);
  assert.equal(first.detail.tier, "C");
  assert.equal(first.detail.profile, "release");
  const drag = await runBrowserSession("interaction.window_level_drag", { build: false, iterations: 2 });
  assert.ok(drag.value > 0);
  const scroll = await runBrowserSession("interaction.scroll", { build: false, iterations: 2 });
  assert.ok(scroll.value > 0);
  const cine = await runBrowserSession("cine.frame_change_rate", { build: false, diagnostic: true, durationMs: 250 });
  assert.ok(cine.value > 0);
  assert.ok(cine.detail.observation.changedFrames > 1);
  assert.equal(cine.detail.representativeHostAcceptance, false);
  const idle = await runBrowserSession("session.cpu_idle", { build: false, diagnostic: true, durationMs: 250 });
  assert.equal(idle.detail.observation.changedFrames, 0);
  assert.equal(idle.detail.observation.scheduled, false);
  const interactive = await runBrowserSession("session.cpu_interactive", { build: false, diagnostic: true, durationMs: 250 });
  assert.ok(interactive.detail.observation.iterations > 1);
  assert.ok(Number.isFinite(interactive.value));
  const changed = repoPath("packages", "core", "dist", "bulk.js");
  const original = await readFile(changed);
  try {
    await writeFile(changed, Buffer.concat([original, Buffer.from("\n// altered served bytes\n")]));
    await assert.rejects(() => runBrowserSession("render.first_frame", { build: false }), /differs from its receipt/);
  } finally { await writeFile(changed, original); }
  const workerSource = await readFile(repoPath("tools/bench/page/session-worker.mjs"), "utf8");
  function mutateWorker(needle, replacement) {
    assert.ok(workerSource.includes(needle), "controlled mutation must apply");
    return source => {
      assert.equal(source, workerSource, "the served worker must be the reviewed source");
      return source.replace(needle, replacement);
    };
  }
  await assert.rejects(() => runBrowserSession("render.first_frame", {
    build: false, diagnostic: true,
    workerTransform: mutateWorker("return api.drain_events();", "api.drain_events(); return new Uint8Array();"),
  }), /no presented-frame kind 6 event/);
  await assert.rejects(() => runBrowserSession("cine.frame_change_rate", {
    build: false, diagnostic: true,
    workerTransform: mutateWorker('const white = canvas.getContext("2d").getImageData(64, 64, 1, 1).data[0];', 'const white = black;'),
  }), /cine frames did not change the actual canvas/);
});
