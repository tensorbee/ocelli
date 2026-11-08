// Controlled supplemental contract and lifecycle. No browser, large DICOM or timing run.
import assert from "node:assert/strict";
import { mkdtemp, writeFile, mkdir, readFile, rm, symlink, realpath } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { createHash } from "node:crypto";
import { Buffer } from "node:buffer";
import test from "node:test";
import { COLOUR_CASE_IDS, COLOUR_WINDOWS, colourCaseSpec, readColourComparisonInput,
  collectPreparedColourComparison, measureColourComparison } from "../src/comparison_colour.mjs";

const hash = value => createHash("sha256").update(value).digest("hex");
const proof = () => ({ host: { platform: "fictional", release: "1", arch: "test", cpu_model: "fixture", cpu_count: 4, memory_bytes: 1073741824 },
  startedAt: "2026-11-08T00:00:00.000Z", source: { head: "1".repeat(40), index_tree: "2".repeat(40), worktree_matches_index: true,
    binary_sha256: "3".repeat(64), comparator_sha256: "4".repeat(64), page_sha256: "5".repeat(64) }, pixelPrecheck: { verdict: { passes: true, low_information: false },
  input_identity: "own-ct-calibration-digest", browser: { version: "151" },
  versions: { cornerstone: "5.8.2" }, defaults: { ocelli_cache_mib: 256 } } });
const prepared = (id = "rgb8") => ({ spec: colourCaseSpec(id), targetDirectory: "/private/test-only-target",
  calibrationDirectory: "/private/test-only-calibration", publicIdentity: {
    manifest_sha256: "a".repeat(64), dataset_sha256: "b".repeat(64), series_sha256: "c".repeat(64), profile: colourCaseSpec(id).profileKind === null ? null :
      { kind: colourCaseSpec(id).profileKind, sha256: "d".repeat(64), bytes: 560 } } });

function fakeTools(change = {}) {
  const calls = []; const closed = [];
  const tools = {
    async measureColourTrial(side, options) {
      calls.push({ kind: "trial", side, ...options });
      return { open_ms: side === "ocelli" ? 10 : 20, first_image_ms: 5,
        browser_version: "151", renderer: side === "ocelli" ? "native CPU" : "WebGL",
        native_budget_bytes: side === "ocelli" ? options.caseSpec.serverCacheMiB * 1048576 : null };
    },
    async launchSide(side, context) {
      calls.push({ kind: "launch", side, context });
      return { side, nativeBudgetBytes: side === "ocelli" ? context.colourCase.serverCacheMiB * 1048576 : null,
        async close() { closed.push(side); } };
    },
    async openFolder(actor, directory) { calls.push({ kind: "open", side: actor.side, directory }); return { open: 1, first_image: 1 }; },
    async windowStep(actor, center, width) { calls.push({ kind: "window", side: actor.side, center, width }); return 3; },
    async measureColourMemorySession(side, options) {
      calls.push({ kind: "memory", side, ...options });
      return { figures: { sampled_peak: 1000, sum_of_peaks: 1500 },
        samples: [[[12, { footprint: 1000, peak: 1500 }]]], sample_times_ms: [12],
        infrastructure: [[[99, { footprint: 100, peak: 200 }]]],
        cpu: { state: "measured", seconds: 2 }, before: [[12, 1]], after: [[12, 3]],
        native_budget_bytes: side === "ocelli" ? options.caseSpec.serverCacheMiB * 1048576 : null };
    }, ...change,
  };
  return { tools, calls, closed };
}

test("the five closed cases and window protocol match the approved extension", () => {
  assert.deepEqual(COLOUR_CASE_IDS, ["rgb8", "ybr422", "palette16", "icc8-gamma22", "icc16-table"]);
  assert.deepEqual(COLOUR_CASE_IDS.map(id => { const s = colourCaseSpec(id); return [s.rows, s.columns, s.frames, s.serverCacheMiB]; }),
    [[512, 512, 200, 256], [512, 512, 200, 256], [512, 512, 200, 256], [8192, 8192, 1, 256], [8192, 8192, 1, 512]]);
  assert.deepEqual(COLOUR_WINDOWS, Array.from({ length: 9 }, (_, at) => ({ center: 100 + at, width: 200 })));
  for (const id of ["constructor", "__proto__", "RGB8", ""]) assert.throws(() => colourCaseSpec(id), /unknown controlled/);
});

test("no filesystem or instrument runs without the unchanged informative CT precheck", async () => {
  for (const context of [{}, { pixelPrecheck: { verdict: { passes: true } } },
    { pixelPrecheck: { verdict: { passes: false, low_information: false } } },
    { pixelPrecheck: { verdict: { passes: true, low_information: true } } }]) {
    const fake = fakeTools();
    await assert.rejects(measureColourComparison({ caseId: "rgb8", context, directory: "/does-not-exist", tools: fake.tools }), /CT pixel calibration/);
    assert.equal(fake.calls.length, 0);
  }
  for (const verdict of [{ passes: false, low_information: false }, { passes: true, low_information: true }]) {
    const context = proof(); context.pixelPrecheck.verdict = verdict; const fake = fakeTools();
    await assert.rejects(collectPreparedColourComparison(prepared(), context, fake.tools), /CT pixel calibration/);
    assert.equal(fake.calls.length, 0);
  }
  const context = proof(); delete context.pixelPrecheck.input_identity;
  await assert.rejects(collectPreparedColourComparison(prepared(), context, fakeTools().tools), /lacks input_identity/);
});


test("actual host and staged artifact identity are required before any trial", async () => {
  for (const edit of [context => { delete context.host; }, context => { context.host.platform = ""; },
    context => { context.host.cpu_count = 0; }, context => { context.source.head = "short"; }, context => { context.source.worktree_matches_index = false; },
    context => { context.source.binary_sha256 = "short"; }, context => { context.host.extra = true; },
    context => { context.startedAt = "not a date"; }]) {
    const context = proof(); edit(context); const fake = fakeTools();
    await assert.rejects(collectPreparedColourComparison(prepared(), context, fake.tools), /host|source|artifact|start/);
    assert.equal(fake.calls.length, 0);
  }
});

test("all declared trials alternate, windows use retained actors and full memory receipts survive", async () => {
  const fake = fakeTools(); const { record, text } = await collectPreparedColourComparison(prepared(), proof(), fake.tools);
  assert.equal(record.format, "ocelli-controlled-colour-comparison-1");
  assert.equal(record.trials.length, 36); assert.equal(record.window_steps.length, 18); assert.equal(record.memory.length, 6);
  for (const warm of [false, true]) {
    const rows = record.trials.filter(row => row.warm === warm);
    assert.deepEqual(rows.slice(0, 4).map(row => row.side), ["ocelli", "cornerstone", "cornerstone", "ocelli"]);
    for (const side of ["ocelli", "cornerstone"]) assert.equal(rows.filter(row => row.side === side).length, 9);
  }
  assert.deepEqual(record.window_steps.slice(0, 4).map(row => [row.side, row.center, row.width]),
    [["ocelli", 100, 200], ["cornerstone", 100, 200], ["cornerstone", 101, 200], ["ocelli", 101, 200]]);
  assert.deepEqual(fake.calls.filter(call => call.kind === "window").map(call => [call.side, call.center, call.width]),
    record.window_steps.map(row => [row.side, row.center, row.width]));
  assert.deepEqual(fake.closed.sort(), ["cornerstone", "ocelli"]);
  assert.equal(record.memory[0].samples[0][0][1].footprint, 1000);
  assert.deepEqual(record.memory[0].before, [[12, 1]]);
  assert.equal(record.summary.ocelli.open_cold_ms.median, 10);
  assert.equal(record.summary.cornerstone.open_cold_ms.median, 20);
  assert.equal(record.summary.ocelli.sampled_peak_bytes.median, 1000);
  assert.equal(record.colour_pixel_score, "not-scored");
  assert(!JSON.stringify(record).includes("/private/test-only"));
  assert(!text.includes("undefined"));
  assert.match(text, /No baseline or published five-workload record is changed/);
});

test("ICC16 is explicitly charged512MiB and records different ICC semantics", async () => {
  const fake = fakeTools(); const { record } = await collectPreparedColourComparison(prepared("icc16-table"), proof(), fake.tools);
  assert(record.trials.filter(row => row.side === "ocelli").every(row => row.native_budget_bytes === 536870912));
  assert(record.memory.filter(row => row.side === "ocelli").every(row => row.native_budget_bytes === 536870912));
  assert.match(record.profile_semantics, /Ocelli applies ICC, cornerstone draws device RGB/);
  assert(fake.calls.filter(call => call.kind === "memory").every(call => call.caseSpec.serverCacheMiB === 512));
});

test("invalid measurements, wrong actual budgets and browser changes refuse a record", async () => {
  for (const override of [{ open_ms: NaN }, { first_image_ms: -1 }, { native_budget_bytes: 1 }, { renderer: "" }]) {
    const fake = fakeTools(); const original = fake.tools.measureColourTrial;
    fake.tools.measureColourTrial = async (...args) => ({ ...await original(...args), ...override });
    await assert.rejects(collectPreparedColourComparison(prepared(), proof(), fake.tools), /measurement|budget|browser or renderer/);
  }
  const cs = fakeTools(); const csOriginal = cs.tools.measureColourTrial;
  cs.tools.measureColourTrial = async (...args) => ({ ...await csOriginal(...args), ...(args[0] === "cornerstone" ? { native_budget_bytes: 1 } : {}) });
  await assert.rejects(collectPreparedColourComparison(prepared(), proof(), cs.tools), /budget/);
  const fake = fakeTools(); const original = fake.tools.measureColourTrial;
  fake.tools.measureColourTrial = async (side, options) => ({ ...await original(side, options), browser_version: side === "ocelli" ? "151" : "150" });
  await assert.rejects(collectPreparedColourComparison(prepared(), proof(), fake.tools), /different browser builds/);
});

test("both actors close after a window refusal and after synchronous cleanup failure", async () => {
  const fake = fakeTools({ async windowStep() { throw new Error("draw refused"); } });
  await assert.rejects(collectPreparedColourComparison(prepared(), proof(), fake.tools), /draw refused/);
  assert.deepEqual(fake.closed.sort(), ["cornerstone", "ocelli"]);
  const second = fakeTools(); const launch = second.tools.launchSide;
  second.tools.launchSide = async (...args) => { const actor = await launch(...args); if (args[0] === "ocelli") actor.close = () => { second.closed.push("ocelli"); throw new Error("close failed"); }; return actor; };
  await assert.rejects(collectPreparedColourComparison(prepared(), proof(), second.tools), /close failed/);
  assert.deepEqual(second.closed.sort(), ["cornerstone", "ocelli"]);
  second.tools.windowStep = async () => { throw new Error("primary draw refused"); };
  await assert.rejects(collectPreparedColourComparison(prepared(), proof(), second.tools), /primary draw refused/);
});

test("no invented memory figure or silently missing process evidence is accepted", async () => {
  for (const change of [{ samples: [] }, { sample_times_ms: [] }, { sample_times_ms: [-1] }, { samples: [[]] },
    { samples: [[[0, { footprint: 1000, peak: 1500 }]]] },
    { samples: [[[12, { footprint: 0, peak: 1500 }]]] }, { samples: [[[12, { footprint: 1000, peak: 999 }]]] },
    { before: [[0, 1]] }, { before: [[12, 1], [12, 2]] },
    { samples: [[[12, { footprint: 1000, peak: 1500 }]], [[12, { footprint: 1000, peak: 1500 }]]],
      sample_times_ms: [5, 4], infrastructure: [[[99, { footprint: 100, peak: 200 }]], [[99, { footprint: 100, peak: 200 }]]] }, { before: [] }, { before: [[12, NaN]] }, { infrastructure: [] },
    { before: null }, { figures: { sampled_peak: 999, sum_of_peaks: 1500 } }, { cpu: { state: "invalid" } }, { figures: { sampled_peak: NaN, sum_of_peaks: 2 } }]) {
    const fake = fakeTools(); const memory = fake.tools.measureColourMemorySession;
    fake.tools.measureColourMemorySession = async (...args) => ({ ...await memory(...args), ...change });
    await assert.rejects(collectPreparedColourComparison(prepared(), proof(), fake.tools), /memory|CPU|process receipts/);
  }
});

async function manifestFixture(caseId = "rgb8") {
  const spec = colourCaseSpec(caseId);
  const root = await mkdtemp(join(tmpdir(), "ocelli-controlled-input-"));
  await mkdir(join(root, "colour")); await mkdir(join(root, "calibration-ct"));
  async function entry(path, content) { await writeFile(join(root, path), content); return { path, bytes: Buffer.byteLength(content), sha256: hash(content) }; }
  const targetFiles = [];
  for (let index = 0; index < spec.frames; index += 1) targetFiles.push(await entry(`colour/${String(index).padStart(6, "0")}.dcm`, `test identity bytes ${index}`));
  const manifest = { schemaVersion: 1, generator: "scripts/corpus_synth.py:comparison-colour-v1", caseId,
    rows: spec.rows, columns: spec.columns, frames: spec.frames, measurementSize: true, photometric: spec.photometric, samplesPerPixel: spec.samplesPerPixel,
    bitsAllocated: spec.bitsAllocated, bitsStored: spec.bitsStored, seriesUid: "2.25.123", pixelPattern: "position-frame-v1", serverCacheMiB: spec.serverCacheMiB,
    profile: prepared(caseId).publicIdentity.profile, calibrationDirectory: "calibration-ct", calibrationFiles: [await entry("calibration-ct/000000.dcm", "test CT identity")],
    targetDirectory: "colour", targetFiles };
  const path = join(root, "colour-comparison-input.json");
  const save = () => writeFile(path, JSON.stringify(manifest));
  await save(); return { root, manifest, path, save };
}

test("manifest identities bind every input and never substitute the calibration folder", async () => {
  const fixture = await manifestFixture();
  try {
    const input = await readColourComparisonInput(fixture.root, "rgb8");
    assert.equal(input.targetDirectory, join(await realpath(fixture.root), "colour"));
    assert.equal(input.calibrationDirectory, join(await realpath(fixture.root), "calibration-ct"));
    assert.match(input.publicIdentity.dataset_sha256, /^[0-9a-f]{64}$/);
    const original = input.publicIdentity.dataset_sha256;
    await writeFile(join(fixture.root, fixture.manifest.targetFiles[0].path), "edited identity");
    await assert.rejects(readColourComparisonInput(fixture.root, "rgb8"), /declared bytes/);
    assert.match(original, /^[0-9a-f]{64}$/);
  } finally { await rm(fixture.root, { recursive: true, force: true }); }
});

test("small test dimensions, unknown schema fields, path escapes and inventory additions refuse", async () => {
  const fixture = await manifestFixture();
  try {
    for (const edit of [{ measurementSize: false }, { rows: 6 }, { extra: 1 }, { targetDirectory: "calibration-ct" }]) {
      const original = await readFile(fixture.path);
      await writeFile(fixture.path, JSON.stringify({ ...fixture.manifest, ...edit }));
      await assert.rejects(readColourComparisonInput(fixture.root, "rgb8"), /declaration|case|unknown fields/);
      await writeFile(fixture.path, original);
    }
    const original = fixture.manifest.targetFiles[0].path;
    fixture.manifest.targetFiles[0].path = "../escape.dcm"; await fixture.save();
    await assert.rejects(readColourComparisonInput(fixture.root, "rgb8"), /identity is invalid/);
    fixture.manifest.targetFiles[0].path = original; await fixture.save();
    await writeFile(join(fixture.root, "colour/extra.dcm"), "extra");
    await assert.rejects(readColourComparisonInput(fixture.root, "rgb8"), /inventory differs/);
    await rm(join(fixture.root, "colour/extra.dcm"));
    const replacement = join(fixture.root, "foreign"); await mkdir(replacement);
    await rm(join(fixture.root, "colour"), { recursive: true }); await symlink(replacement, join(fixture.root, "colour"));
    await assert.rejects(readColourComparisonInput(fixture.root, "rgb8"), /plain directory/);
  } finally { await rm(fixture.root, { recursive: true, force: true }); }
});


test("prepared definitions and profile manifests refuse substitutions", async () => {
  for (const edit of [input => { input.spec.rows = 6; }, input => { input.publicIdentity.dataset_sha256 = "short"; },
    input => { input.publicIdentity.profile = { kind: "not-declared" }; }]) {
    const input = prepared(); edit(input); const fake = fakeTools();
    await assert.rejects(collectPreparedColourComparison(input, proof(), fake.tools), /prepared/);
    assert.equal(fake.calls.length, 0);
  }
  for (const caseId of ["rgb8", "icc8-gamma22", "icc16-table"]) {
    const fixture = await manifestFixture(caseId);
    try {
      const input = await readColourComparisonInput(fixture.root, caseId);
      assert.deepEqual(input.publicIdentity.profile, fixture.manifest.profile);
      const original = fixture.manifest.profile;
      fixture.manifest.profile = caseId === "rgb8" ? { kind: "gamma22" } : { ...original, bytes: 559 };
      await fixture.save();
      await assert.rejects(readColourComparisonInput(fixture.root, caseId), /profile/);
      fixture.manifest.profile = original;
      fixture.manifest.targetFiles.pop(); await fixture.save();
      await assert.rejects(readColourComparisonInput(fixture.root, caseId), /file count/);
    } finally { await rm(fixture.root, { recursive: true, force: true }); }
  }
});
