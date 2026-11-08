// F223 consumer boundaries. These fixtures name fake inputs and clocks only.
import assert from "node:assert/strict";
import test from "node:test";
import { spawnSync } from "node:child_process";
import { createRequire } from "node:module";
import { createHash } from "node:crypto";
import { mono16Part10, filledPixels } from "../src/viewer_work.mjs";
import { fileURLToPath, pathToFileURL } from "node:url";
import { mkdtemp, rm, writeFile, readFile, mkdir, readdir } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { readyDuration, writePair, runComparison, collectTrialSamples, readHeaders, safeColourOutputs, publishComparison, prepareInputs } from "../src/comparison_run.mjs";

test("page-ready joins clock origins rather than subtracting cross-context now", () => {
  // Controller origin 1000 + now 30 starts at1030. Browser origin2000 +
  // now40 ends at2040. Plain40-30 would incorrectly report10 instead1010.
  assert.equal(readyDuration(1030, { timeOrigin: 2000, now: 40 }), 1010);
  assert.throws(() => readyDuration(1030, { timeOrigin: 0, now: 40 }), /clock/);
  assert.throws(() => readyDuration(1030, { timeOrigin: NaN, now: 40 }), /clock/);
});

test("comparison pair writes both prepared bodies and no next-file residue", async () => {
  const dir = await mkdtemp(join(tmpdir(), "comparison-pair-"));
  try {
    const json = join(dir, "result.json"), md = join(dir, "result.md");
    await writeFile(json, "old-json"); await writeFile(md, "old-md");
    await writePair(json, md, "new-json", "new-md");
    assert.equal(await readFile(json, "utf8"), "new-json");
    assert.equal(await readFile(md, "utf8"), "new-md");
    await assert.rejects(readFile(`${json}.next`), { code: "ENOENT" });
    await assert.rejects(readFile(`${md}.next`), { code: "ENOENT" });
  } finally { await rm(dir, { recursive: true, force: true }); }
});

test("missing and empty pseudo input refuse before launching either browser", async () => {
  let launches = 0;
  const services = { launch: () => { launches += 1; throw new Error("launcher must not run"); } };
  await assert.rejects(runComparison({ pseudo: null, environment: {}, build: false }, services), /--pseudo/);
  const dir = await mkdtemp(join(tmpdir(), "comparison-empty-"));
  try { await assert.rejects(runComparison({ pseudo: dir, environment: {}, out: join(dir, "out"), build: false }, services), /Part 10/); }
  finally { await rm(dir, { recursive: true, force: true }); }
  assert.equal(launches, 0);
});

test("publication refuses injected measurement services", async () => {
  await assert.rejects(runComparison({ pseudo: "/synthetic-control", publish: true }, { launch: () => {} }), /injected/);
});

test("failed pixel verdict records no trial and cleans its exclusive lock", async () => {
  const dir = await mkdtemp(join(tmpdir(), "comparison-pixels-"));
  let trials = 0;
  try {
    await mkdir(join(dir, "input"));
    const verdict = { within_one_lsb_fraction: 1, count_over_max: 0, max_abs_diff: 1, signed_mean_bias_informative: 1, informative_fraction: 1, low_information: false, passes: false };
    const result = await runComparison({ pseudo: join(dir, "input"), out: join(dir, "out"), build: false }, {
      prepare: async () => ({ denySet: {}, selections: {}, directories: {}, identity: "a".repeat(64) }),
      build: async () => ({}),
      exclusive: async () => {},
      checkPixels: async () => ({ verdict, metadata: [{ version: "control", renderer: "control", defaults: {} }, { version: "control", renderer: "control", defaults: {} }] }),
      measure: async () => { trials += 1; },
    });
    assert.equal(result.state, "refused_pixels");
    assert.deepEqual(result.record.metrics, {});
    assert.equal(trials, 0);
    await assert.rejects(readFile(join(dir, "out", "lock")), { code: "ENOENT" });
  } finally { await rm(dir, { recursive: true, force: true }); }
});

// N9 is PER SIDE, so18 actual observation calls must alternate.
test("trial consumer alternates nine actual observations per side",async()=>{
 const order=[];const got=await collectTrialSamples(9,"ocelli",async(side,index)=>{order.push([side,index]);return{value:index+1};});
 assert.deepEqual(order,Array.from({length:18},(_,i)=>[i%2===0?"ocelli":"cornerstone",Math.floor(i/2)]));
 assert.deepEqual(got.samples.ocelli,[1,2,3,4,5,6,7,8,9]);assert.deepEqual(got.samples.cornerstone,[1,2,3,4,5,6,7,8,9]);
 const flipped=[];await collectTrialSamples(1,"cornerstone",async side=>{flipped.push(side);return{value:1};});assert.deepEqual(flipped,["cornerstone","ocelli"]);
 await assert.rejects(collectTrialSamples(0.5,"ocelli",async()=>({value:1})),/whole/);
});
test("a rejected trial remains absent and never reuses its planned window index",async()=>{
 const indices=[];const got=await collectTrialSamples(3,"ocelli",async(side,index)=>{if(side==="ocelli")indices.push(index);if(side==="ocelli"&&index===0)throw new Error("fixture failure");return{value:index+1};});
 assert.deepEqual(indices,[0,1,2]);assert.deepEqual(got.samples.ocelli,[2,3]);assert.equal(got.trials.filter(x=>x.error).length,1);
});

test("SIGTERM closes owned actors and infrastructure and removes temporary state",async()=>{
 const dir=await mkdtemp(join(tmpdir(),"comparison-abort-"));
 try{const child=spawnSync(process.execPath,[fileURLToPath(new URL("./fixtures/comparison_signal_child.mjs",import.meta.url)),dir],{encoding:"utf8",timeout:10000});
 assert.equal(child.status,143,child.stderr);const receipt=JSON.parse(child.stdout.trim());assert.equal(receipt.temporary_directory_remains,false);
 assert.equal(await readFile(join(dir,"actor-closed"),"utf8"),"yes");assert.equal(await readFile(join(dir,"infrastructure-closed"),"utf8"),"yes");await assert.rejects(readFile(join(dir,"out","lock")),{code:"ENOENT"});
 }finally{await rm(dir,{recursive:true,force:true});}
});


test("controlled selectors remain public while generated target headers and basenames are denied", async () => {
  const dir = await mkdtemp(join(tmpdir(), "comparison-synthetic-privacy-"));
  try {
    const target = join(dir, "colour"); await mkdir(target);
    const path = join(target, "uncontrolled-sensitive-token.dcm");
    await writeFile(path, mono16Part10({ rows: 2, columns: 2, pixels: filledPixels(4, 50), instanceNumber: "1",
      sopClass: "1.2.840.10008.5.1.4.1.1.2", sopUid: "2.25.223001", seriesUid: "2.25.223002", modality: "CT" }));
    const parser = createRequire(new URL("../../oracle/package.json", import.meta.url))("dicom-parser");
    const deny = { names: [], ids: [], dates: [], institutions: [], uids: [], paths: [] };
    await readHeaders([path], "pseudo", target, parser, deny);
    const good = safeColourOutputs({ caseId: "rgb8", format: "colour-comparison" }, "colour calibration-ct", deny);
    assert.match(good.digest, /^[0-9a-f]{64}$/);
    assert.equal(good.json.endsWith("\n"), true);
    assert.throws(() => safeColourOutputs({ note: "SYNTHETIC" }, "colour", deny), /denied/);
    assert.throws(() => safeColourOutputs({ note: "uncontrolled-sensitive-token.dcm" }, "colour", deny), /denied/);
    assert.throws(() => safeColourOutputs({ caseId: "rgb8" }, "2.25.223002", deny), /denied/);
  } finally { await rm(dir, { recursive: true, force: true }); }
});

test("supplemental branch cannot reach colour trials before successful CT calibration", async () => {
  const dir = await mkdtemp(join(tmpdir(), "comparison-colour-calibration-")); let measured = 0;
  try {
    const result = await runComparison({ pseudo: dir, out: join(dir, "out"), colourCase: "rgb8", build: false }, {
      colourInput: async () => ({ calibrationDirectory: dir, targetDirectory: dir }),
      prepare: async () => ({ denySet: {}, selections: {}, directories: {}, identity: "a".repeat(64) }),
      build: async () => ({}), exclusive: async () => {},
      checkPixels: async () => ({ verdict: { passes: true, low_information: true, within_one_lsb_fraction: 1, count_over_max: 0, max_abs_diff: 1, signed_mean_bias_informative: 0, informative_fraction: 0.01 }, metadata: [
        { version: "control", renderer: "control", defaults: {} }, { version: "control", renderer: "control", defaults: {} }] }),
      measureColour: async () => { measured += 1; },
    });
    assert.equal(result.state, "refused_pixels"); assert.equal(measured, 0);
  } finally { await rm(dir, { recursive: true, force: true }); }
});


test("tracked publication eligibility is checked after cleanup before either replacement", async () => {
  const dir = await mkdtemp(join(tmpdir(), "comparison-publication-"));
  try {
    const json = join(dir, "cornerstone-comparison.json"), md = join(dir, "cornerstone-comparison.md");
    await writeFile(json, "old-json"); await writeFile(md, "old-md");
    const pendingPublication = { json: "new-json\n", text: "new-markdown" };
    for (const outcome of [{ state: "measured", cleanupFailed: true, pendingPublication },
      { state: "invalid", pendingPublication }, { state: "measured", pendingPublication: { ...pendingPublication, extra: 1 } },
      { state: "measured", pendingPublication: { json: 1, text: "x" } }, { state: "measured" }]) {
      await assert.rejects(publishComparison(outcome, dir), /publication/);
      assert.equal(await readFile(json, "utf8"), "old-json"); assert.equal(await readFile(md, "utf8"), "old-md");
    }
    await publishComparison({ state: "measured", pendingPublication }, dir);
    assert.equal(await readFile(json, "utf8"), "new-json\n"); assert.equal(await readFile(md, "utf8"), "new-markdown");
  } finally { await rm(dir, { recursive: true, force: true }); }
});


test("actual controller publishes only after successful owned cleanup", async () => {
  const dir = await mkdtemp(join(tmpdir(), "comparison-publication-order-"));
  try {
    const sourceUrl = new URL("../src/comparison_run.mjs", import.meta.url);
    const source = await readFile(sourceUrl, "utf8");
    const guard = "if (options.publish && Object.keys(services).length)";
    const destination = 'destination = repoPath("docs", "benchmarks")';
    assert.equal(source.split(guard).length, 2, "exactly the injected-service guard is bypassed in this test copy");
    assert.equal(source.split(destination).length, 2, "only the publication destination is redirected");
    const publication = join(dir, "publication"); await mkdir(publication);
    // Relocate static imports to the original modules. Apart from that
    // mechanical relocation, only these two expressions change. Actual
    // executeComparison -> finally -> runComparison control flow is retained.
    const copy = source.replace(guard, "if (false)")
      .replace(destination, "destination = " + JSON.stringify(publication))
      .replace(/from "(\.[^"]+)"/g, (_match, name) => 'from "' + new URL(name, sourceUrl).href + '"');
    const path = join(dir, "comparison_run.mjs"); await writeFile(path, copy);
    const { runComparison: copiedController } = await import(pathToFileURL(path).href);
    const json = join(publication, "cornerstone-comparison.json");
    const markdown = join(publication, "cornerstone-comparison.md");
    const prior = ["prior-json\n", "prior-markdown\n"];
    const verdict = { within_one_lsb_fraction: 1, count_over_max: 0, max_abs_diff: 0,
      signed_mean_bias_informative: 0, informative_fraction: 1, low_information: false, passes: true };
    const cell = { state: "measured", n: 9, median: 5, p25: 3, p75: 7, min: 1, max: 9 };
    for (const failClose of [true, false]) {
      await writeFile(json, prior[0]); await writeFile(markdown, prior[1]);
      const atClose = [];
      const operation = copiedController({ pseudo: dir, out: join(dir, "out"), publish: true }, {
        prepare: async () => ({ denySet: {}, selections: {}, directories: {}, identity: "a".repeat(64) }),
        exclusive: async () => {},
        build: async () => ({ close: async () => {
          atClose.push([await readFile(json, "utf8"), await readFile(markdown, "utf8")]);
          if (failClose) throw new Error("synthetic cleanup failure");
        } }),
        checkPixels: async () => ({ verdict, metadata: [
          { version: "fixture", renderer: "fixture", defaults: { serverCacheBytes: 268435456, pageGreyCacheBytes: 67108864, decodePermits: 2 } },
          { version: "fixture", renderer: "fixture", defaults: { maxCacheSizeBytes: 3221225472, decodeWorkers: 8 } }] }),
        measure: async () => ({ metrics: { open: {
          "synthetic-ct/cold": { ocelli: cell, cornerstone: cell },
          "synthetic-ct/warm": { ocelli: cell, cornerstone: cell } } }, raw: [] }),
      });
      if (failClose) {
        await assert.rejects(operation, /cleanup/);
        assert.equal(await readFile(json, "utf8"), prior[0]);
        assert.equal(await readFile(markdown, "utf8"), prior[1]);
      } else {
        const result = await operation; assert.equal(result.state, "measured");
        const replaced = JSON.parse(await readFile(json, "utf8"));
        assert.equal(replaced.metrics.open["synthetic-ct/cold"].ocelli.median, 5);
        assert.notEqual(await readFile(markdown, "utf8"), prior[1]);
      }
      assert.deepEqual(atClose, [prior], "both prior files must survive until close has completed");
    }
  } finally { await rm(dir, { recursive: true, force: true }); }
});


test("prepared subsets contain exact independent regular files while full input stays untouched", async () => {
  const temp = await mkdtemp(join(tmpdir(), "comparison-prepared-files-"));
  const input = join(temp, "input"), prepared = join(temp, "prepared");
  const hash = (bytes) => createHash("sha256").update(bytes).digest("hex");
  try {
    await mkdir(input); await mkdir(prepared);
    for (const index of [0, 1]) {
      await writeFile(join(input, `input-${index}.dcm`), mono16Part10({ rows: 512, columns: 512,
        pixels: filledPixels(512 * 512, index + 50), instanceNumber: String(index + 1),
        sopClass: "1.2.840.10008.5.1.4.1.1.2", sopUid: `2.25.223401${index}`,
        seriesUid: "2.25.223400", modality: "CT" }));
    }
    await writeFile(join(input, "auxiliary.txt"), "own synthetic auxiliary input");
    const parser = createRequire(new URL("../../oracle/package.json", import.meta.url))("dicom-parser");
    const inputs = await prepareInputs(input, prepared, parser, false);
    assert.equal(inputs.submitted_file_count, 3);
    assert.equal(inputs.directories["pseudo-full"], input, "full root still submits every original file");
    const selected = inputs.selections["pseudo-ct"];
    assert.equal(selected.count, 2);
    const entries = await readdir(inputs.directories["pseudo-ct"], { withFileTypes: true });
    assert.equal(entries.length, selected.files.length);
    assert(entries.every((entry) => entry.isFile() && !entry.isSymbolicLink()));
    assert.deepEqual(entries.map((entry) => entry.name).sort(), ["000000.dcm", "000001.dcm"]);
    for (let index = 0; index < selected.files.length; index += 1) {
      const source = selected.files[index];
      const copy = join(inputs.directories["pseudo-ct"], `${String(index).padStart(6, "0")}.dcm`);
      const sourceBytes = await readFile(source), copyBytes = await readFile(copy);
      assert.deepEqual(copyBytes, sourceBytes);
      assert.equal(hash(copyBytes), hash(sourceBytes));
      copyBytes[copyBytes.length - 1] ^= 1;
      await writeFile(copy, copyBytes);
      assert.equal(hash(await readFile(source)), hash(sourceBytes), "prepared files never alias source storage writes");
    }
    assert.equal((await readdir(input)).length, 3);
  } finally { await rm(temp, { recursive: true, force: true }); }
});

// A disposable copy substitutes OS/browser transports only. The actual
// measureAll -> memorySession -> cpuBaseline -> cpuSnapshot control flow and
// CPU-accounting predicates stay intact, including on non-macOS test hosts.
test("actual memory orchestration establishes only a bounded pre-action CPU baseline", async () => {
  const temp = await mkdtemp(join(tmpdir(), "comparison-cpu-coherence-"));
  const descriptor = Object.getOwnPropertyDescriptor(process, "platform");
  const sourceUrl = new URL("../src/comparison_run.mjs", import.meta.url);
  const fixture = { states: [], mode: "stable", active: null };
  try {
    Object.defineProperty(process, "platform", { ...descriptor, value: "darwin" });
    fixture.exec = async (file, args) => {
      assert.equal(file, "ps");
      const state = fixture.active;
      if (args.includes("pid=,ppid=")) {
        const child = fixture.mode === "changed_after" && state.navigated ? "" : "11 10\n";
        return { stdout: `10 1\n${child}20 1\n`, stderr: "" };
      }
      assert(args.includes("pid=,time="));
      const before = !state.navigated;
      if (before) state.baselineReads += 1; else state.endReads += 1;
      assert(state.baselineReads <= 4 && state.endReads <= 4, "fixture must not permit an unbounded retry");
      if (fixture.mode === "foreign_error" && before) throw new Error("fixture transport refused");
      const missing = before
        ? fixture.mode === "persistent" || (fixture.mode === "race_then_stable" && state.baselineReads === 1)
        : fixture.mode === "missing_after" || fixture.mode === "changed_after";
      const value = before ? "00:01.00" : "00:02.00";
      const root = fixture.mode === "invalid_counter" && before ? "invalid" : value;
      return { stdout: `10 ${root}\n${missing ? "" : `11 ${value}\n`}20 ${value}\n`, stderr: "" };
    };
    fixture.launchSide = async (side, context) => {
      const state = { side, navigated: false, actions: 0, baselineReads: 0, endReads: 0, samplerStarts: 0, samplerStops: 0, closed: false };
      fixture.states.push(state); fixture.active = state;
      const browser = { version: () => "fixture-browser", newBrowserCDPSession: async () => ({
        send: async () => ({ processInfo: fixture.mode === "changed_after" && state.navigated
          ? [{ type: "browser", id: 10 }] : [{ type: "browser", id: 10 }, { type: "renderer", id: 11 }] }),
        detach: async () => {},
      }) };
      const actor = { side, browser, server: side === "ocelli" ? { pid: 20 } : null,
        nativeBudgetBytes: side === "ocelli" ? 256 * 1024 * 1024 : null,
        page: { evaluate: async () => "fixture-renderer" },
        navigate: async () => { state.navigated = true; state.actions += 1; },
        close: async () => { state.closed = true; context.actors.delete(actor); },
      };
      context.actors.add(actor); return actor;
    };
    fixture.openFolder = async () => { fixture.active.actions += 1; return { open: 1, first_image: 1 }; };
    fixture.scroll = async () => { fixture.active.actions += 1; return { keys: 150, blank: 0 }; };
    fixture.windowStep = async () => { fixture.active.actions += 1; return 1; };
    fixture.startProcessSampler = () => {
      const state = fixture.active; state.samplerStarts += 1;
      return { stop: async () => { state.samplerStops += 1; return {
        figures: { sampled_peak: 30, sum_of_peaks: 40 },
        samples: [[[10, { footprint: 10, peak: 10 }]]], sample_times_ms: [1], infrastructure: [],
      }; } };
    };
    globalThis.__cpuCoherenceFixture = fixture;
    let source = await readFile(sourceUrl, "utf8");
    assert.equal(source.split("const exec = promisify(execFile);").length, 2);
    source = source.replace("const exec = promisify(execFile);", "const exec = globalThis.__cpuCoherenceFixture.exec;");
    for (const [prefix, name] of [["export async function ", "launchSide"], ["export async function ", "openFolder"],
      ["export async function ", "scroll"], ["export async function ", "windowStep"], ["export function ", "startProcessSampler"]]) {
      assert.equal(source.split(prefix + name + "(").length, 2);
      source = source.replace(prefix + name + "(", prefix + "unusedTransport_" + name + "(");
    }
    source = source.replace(/from "(\.[^"]+)"/g, (_match, name) => 'from "' + new URL(name, sourceUrl).href + '"');
    source += "\nconst { launchSide, openFolder, scroll, windowStep, startProcessSampler } = globalThis.__cpuCoherenceFixture;\n";
    const copy = join(temp, "controller.mjs"); await writeFile(copy, source);
    const { measureAll: actualMeasureAll, measureColourMemorySession: actualColourSession } = await import(pathToFileURL(copy).href);
    const inputs = { selections: {}, directories: { "pseudo-full": "full", "pseudo-ct": "ct", "pseudo-dx": "dx" } };
    const rule = { cold: { state: "yes" }, warm: { state: "no", reason: "functional fixture" } };
    const definition = { metrics: [{ id: "peak_memory", n: 1, workloads: [], ...rule }, { id: "session_cpu", n: 1, workloads: [], ...rule }] };
    for (const mode of ["stable", "race_then_stable", "persistent", "invalid_counter", "foreign_error", "missing_after", "changed_after"]) {
      fixture.mode = mode; fixture.states = [];
      const context = { actors: new Set() };
      const result = await actualMeasureAll(context, inputs, definition);
      const acted = !["persistent", "invalid_counter", "foreign_error"].includes(mode);
      const memoryValid = ["stable", "race_then_stable", "changed_after"].includes(mode);
      const cpuValid = ["stable", "race_then_stable"].includes(mode);
      for (const side of ["ocelli", "cornerstone"]) {
        assert.equal(result.metrics.peak_memory["session/cold"][side].state, memoryValid ? "measured" : "invalid", mode);
        assert.equal(result.metrics.session_cpu["session/cold"][side].state, cpuValid ? "measured" : "invalid", mode);
        if (cpuValid) assert.equal(result.metrics.session_cpu["session/cold"][side].median, side === "ocelli" ? 3 : 2);
      }
      for (const state of fixture.states) {
        assert.equal(state.baselineReads, mode === "race_then_stable" ? 2 : mode === "persistent" ? 3 : 1, mode);
        assert.equal(state.endReads, acted ? 1 : 0, "post-action counters are never retried");
        assert.equal(state.navigated, acted, mode); assert.equal(state.actions > 0, acted, mode);
        assert.equal(state.samplerStarts, acted ? 1 : 0, mode); assert.equal(state.samplerStops, acted ? 1 : 0, mode);
        assert.equal(state.closed, true);
      }
      assert.equal(context.actors.size, 0);
    }
    // The supplemental consumer shares the same setup-only policy.
    fixture.mode = "race_then_stable"; fixture.states = [];
    const context = { actors: new Set() };
    const colour = await actualColourSession("ocelli", { directory: "colour", caseSpec: { serverCacheMiB: 256 }, context });
    assert.equal(colour.cpu.state, "measured"); assert.equal(colour.cpu.seconds, 3);
    assert.equal(fixture.states[0].baselineReads, 2); assert.equal(fixture.states[0].endReads, 1);
    assert.equal(fixture.states[0].closed, true); assert.equal(context.actors.size, 0);
    for (const mode of ["persistent", "missing_after", "invalid_counter"]) {
      fixture.mode = mode; fixture.states = [];
      const context = { actors: new Set() };
      await assert.rejects(actualColourSession("ocelli", { directory: "colour", caseSpec: { serverCacheMiB: 256 }, context }));
      const state = fixture.states[0];
      assert.equal(state.baselineReads, mode === "persistent" ? 3 : 1);
      assert.equal(state.endReads, mode === "missing_after" ? 1 : 0);
      assert.equal(state.actions > 0, mode === "missing_after");
      assert.equal(state.closed, true); assert.equal(context.actors.size, 0);
    }
  } finally {
    Object.defineProperty(process, "platform", descriptor);
    delete globalThis.__cpuCoherenceFixture;
    await rm(temp, { recursive: true, force: true });
  }
});

test("environment refusals retain both sides only after successful private cleanup", async () => {
  const dir = await mkdtemp(join(tmpdir(), "comparison-environment-refusal-"));
  const info = [
    { version: "fixture-browser-A", renderer: "fixture-renderer-A", defaults: {} },
    { version: "fixture-browser-B", renderer: "fixture-renderer-B", defaults: {} },
  ];
  try {
    for (const mode of ["mismatch", "headed_software", "cleanup_failure", "privacy_failure"]) {
      const out = join(dir, mode); let trials = 0, checks = 0, closed = false;
      const metadata = mode === "privacy_failure" ? [{ ...info[0], renderer: "denied-fixture-value" }, info[1]] : info;
      const operation = runComparison({ pseudo: dir, out, build: false }, {
        prepare: async () => ({ denySet: mode === "privacy_failure" ? { names: ["denied-fixture-value"] } : {}, selections: {}, directories: {}, identity: "a".repeat(64) }),
        exclusive: async () => {},
        build: async () => ({ close: async () => { closed = true; if (mode === "cleanup_failure") throw new Error("fixture cleanup failure"); } }),
        checkPixels: async (context) => {
          checks += 1;
          assert.equal(context.headed, mode === "headed_software" && checks === 2);
          return mode === "headed_software" ? { needsHeaded: true, metadata } : { environmentRefusal: "environment_mismatch", metadata };
        },
        measure: async () => { trials += 1; throw new Error("no trial is permitted"); },
      });
      await assert.rejects(operation, (error) => {
        assert.equal(closed, true, "owned cleanup finishes before the public refusal");
        assert.equal(error.exitCode, mode === "cleanup_failure" ? 1 : 2);
        return true;
      });
      assert.equal(trials, 0); assert.equal(checks, mode === "headed_software" ? 2 : 1);
      await assert.rejects(readFile(join(out, "lock")), { code: "ENOENT" });
      if (["cleanup_failure", "privacy_failure"].includes(mode)) {
        await assert.rejects(readFile(join(out, "comparison.json")), { code: "ENOENT" });
        await assert.rejects(readFile(join(out, "comparison.txt")), { code: "ENOENT" });
      } else {
        const record = JSON.parse(await readFile(join(out, "comparison.json"), "utf8"));
        const text = await readFile(join(out, "comparison.txt"), "utf8");
        assert.equal(record.state, "refused_environment"); assert.deepEqual(record.metrics, {});
        assert.equal(Object.hasOwn(record, "pair_verdict"), false);
        assert.equal(record.headed, mode === "headed_software");
        assert.deepEqual(Object.values(record.versions), info.flatMap((side) => [side.version, side.renderer]));
        for (const value of Object.values(record.versions)) assert(text.includes(value));
        assert(text.includes("before pixel comparison or timings"));
      }
    }
  } finally { await rm(dir, { recursive: true, force: true }); }
});

test("refused and invalid publication keeps ignored evidence without replacing tracked results", async () => {
  const dir = await mkdtemp(join(tmpdir(), "comparison-refused-publication-"));
  try {
    const sourceUrl = new URL("../src/comparison_run.mjs", import.meta.url);
    const source = await readFile(sourceUrl, "utf8");
    const guard = "if (options.publish && Object.keys(services).length)";
    const destination = 'destination = repoPath("docs", "benchmarks")';
    assert.equal(source.split(guard).length, 2); assert.equal(source.split(destination).length, 2);
    const publication = join(dir, "publication"); await mkdir(publication);
    const copy = source.replace(guard, "if (false)")
      .replace(destination, "destination = " + JSON.stringify(publication))
      .replace(/from "(\.[^"]+)"/g, (_match, name) => 'from "' + new URL(name, sourceUrl).href + '"');
    const path = join(dir, "comparison_run.mjs"); await writeFile(path, copy);
    const { runComparison: controller } = await import(pathToFileURL(path).href);
    const json = join(publication, "cornerstone-comparison.json"), markdown = join(publication, "cornerstone-comparison.md");
    const verdict = { within_one_lsb_fraction: 1, count_over_max: 0, max_abs_diff: 0, signed_mean_bias_informative: 0, informative_fraction: 1, low_information: false, passes: true };
    const metadata = [{ version: "fixture-A", renderer: "renderer-A", defaults: {} }, { version: "fixture-B", renderer: "renderer-B", defaults: {} }];
    for (const environmentRefused of [true, false]) {
      await writeFile(json, "prior-json\n"); await writeFile(markdown, "prior-markdown\n");
      const out = join(dir, environmentRefused ? "environment" : "invalid");
      let closed = false, trials = 0;
      const invalid = { state: "invalid", n: 0, reason: "fixture missing trial" };
      await assert.rejects(controller({ pseudo: dir, out, publish: true }, {
        prepare: async () => ({ denySet: {}, selections: {}, directories: {}, identity: "a".repeat(64) }),
        exclusive: async () => {},
        build: async () => ({ close: async () => { closed = true; } }),
        checkPixels: async () => environmentRefused ? { environmentRefusal: "environment_mismatch", metadata } : { verdict, metadata },
        measure: async () => { trials += 1; return { metrics: { open: { "pseudo-ct/cold": { ocelli: invalid, cornerstone: invalid } } }, raw: [{ side: "cornerstone", error: "fixture failed trial" }] }; },
      }), (error) => { assert.equal(closed, true); assert.equal(error.exitCode, environmentRefused ? 2 : 1); return true; });
      assert.equal(trials, environmentRefused ? 0 : 1);
      const record = JSON.parse(await readFile(join(out, "comparison.json"), "utf8"));
      assert.equal(record.state, environmentRefused ? "refused_environment" : "invalid");
      if (!environmentRefused) assert.equal(record.trials[0].error, "fixture failed trial");
      assert.equal(await readFile(json, "utf8"), "prior-json\n"); assert.equal(await readFile(markdown, "utf8"), "prior-markdown\n");
      await readFile(join(out, "comparison.txt"));
      await assert.rejects(readFile(join(out, "lock")), { code: "ENOENT" });
    }
    // Pre-launch flag refusals never take ownership of a prior ignored record.
    const out = join(dir, "prelaunch"); await mkdir(out); await writeFile(join(out, "comparison.json"), "prior-ignored\n");
    await assert.rejects(runComparison({ pseudo: dir, out, publish: true }, { prepare: async () => {} }), /injected/);
    assert.equal(await readFile(join(out, "comparison.json"), "utf8"), "prior-ignored\n");
  } finally { await rm(dir, { recursive: true, force: true }); }
});

test("actual orchestration shares the workload start side across cold and warm", async () => {
  const dir = await mkdtemp(join(tmpdir(), "comparison-workload-order-"));
  const sourceUrl = new URL("../src/comparison_run.mjs", import.meta.url);
  const fixture = { opens: [], launches: 0, closes: 0 };
  try {
    fixture.launchSide = async (side, context) => {
      fixture.launches += 1;
      const actor = { side, page: { close: async () => {} }, newPage: async () => {},
        close: async () => { fixture.closes += 1; context.actors.delete(actor); } };
      context.actors.add(actor); return actor;
    };
    fixture.openFolder = async (actor, directory) => {
      fixture.opens.push([directory, actor.side]); return { open: 1, first_image: 1 };
    };
    globalThis.__workloadOrderFixture = fixture;
    let source = await readFile(sourceUrl, "utf8");
    // Substitute only actual browser transports. Scheduling and the real
    // measureAll -> collectTrialSamples caller chain remain unchanged.
    for (const name of ["launchSide", "openFolder"]) {
      const declaration = "export async function " + name + "(";
      assert.equal(source.split(declaration).length, 2);
      source = source.replace(declaration, "export async function unusedTransport_" + name + "(");
    }
    source = source.replace(/from "(\.[^"]+)"/g, (_match, name) => 'from "' + new URL(name, sourceUrl).href + '"');
    source += "\nconst { launchSide, openFolder } = globalThis.__workloadOrderFixture;\n";
    const path = join(dir, "controller.mjs"); await writeFile(path, source);
    const { measureAll } = await import(pathToFileURL(path).href);
    const context = { actors: new Set() };
    const inputs = { selections: { "pseudo-full": { state: "unavailable", reason: "fixture missing workload" } },
      directories: { "synthetic-ct": "synthetic-ct", "synthetic-dx": "synthetic-dx", "pseudo-ct": "pseudo-ct" } };
    const definition = { metrics: [
      { id: "first_image", n: 9, workloads: ["synthetic-ct", "synthetic-dx", "pseudo-full"], cold: { state: "yes" }, warm: { state: "yes" } },
      { id: "open", n: 9, workloads: ["pseudo-ct"], cold: { state: "yes" }, warm: { state: "yes" } },
    ] };
    const result = await measureAll(context, inputs, definition);
    const expectedOpens = [];
    for (const [metric, workload, start] of [["first_image", "synthetic-ct", "ocelli"], ["first_image", "synthetic-dx", "cornerstone"], ["open", "pseudo-ct", "cornerstone"]]) {
      const other = start === "ocelli" ? "cornerstone" : "ocelli";
      const expectedSides = Array.from({ length: 18 }, (_, index) => index % 2 === 0 ? start : other);
      for (const condition of ["cold", "warm"]) {
        const row = result.raw.find((row) => row.metric === metric && row.workload === workload && row.condition === condition);
        assert.equal(row.start_side, start, `${workload}/${condition}`);
        assert.deepEqual(row.trials.map((trial) => trial.side), expectedSides);
        if (condition === "warm") expectedOpens.push([workload, "ocelli"], [workload, "cornerstone"]);
        expectedOpens.push(...expectedSides.map((side) => [workload, side]));
        for (const side of ["ocelli", "cornerstone"]) {
          assert.equal(result.metrics[metric][`${workload}/${condition}`][side].state, "measured");
          assert.equal(result.metrics[metric][`${workload}/${condition}`][side].n, 9);
        }
      }
    }
    assert.deepEqual(fixture.opens, expectedOpens, "actual observations follow the declared per-workload order");
    for (const condition of ["cold", "warm"]) for (const side of ["ocelli", "cornerstone"]) {
      assert.equal(result.metrics.first_image[`pseudo-full/${condition}`][side].state, "unavailable");
    }
    assert.equal(result.raw.some((row) => row.workload === "pseudo-full"), false);
    assert.equal(context.actors.size, 0); assert.equal(fixture.launches, fixture.closes);
  } finally {
    delete globalThis.__workloadOrderFixture;
    await rm(dir, { recursive: true, force: true });
  }
});
