# S12 whole-sprint review, pass 6

**Reviewed**: staged tree `07831fb1fa438e95310165ad0da449f23a7c6bce` on `sprint/s12`. HEAD is `0ba66c7e5c7b78e6930d2dd1e81c373e40b814ae`. The sprint diff is `git diff 4976c929b8a7c0c081f6aeb2216dd5414571377c`. The remediation since HEAD is `git diff HEAD`, which matches `git diff --cached` on the tracked paths.
**Result**: 4 defects, 0 smells, 0 nitpicks

## Defects

### D1, a story that is not done is still described as unmeasurable

**Where**: `tools/bench/subjects.json:5`, `tools/bench/tests/state_test.mjs:1-2`, `tools/bench/tests/registry_test.mjs:257-258`, and `docs/sprints/AS_BUILT.md:986-988`

**What**: The registry says a row whose `subject_story` is not `done` is reported unavailable, and that `scripts/bench_check.py` refuses both a runner and a baseline entry for it. The state-test header says a subject whose story is not done resolves to unavailable and never to a number. The registry test says the gate refuses a runner and a baseline on `status != "done"`. The F-006 note says the guard refuses a runner for a story that is not done and does not demand one for a story that is.

**Why it is wrong**: `MEASURABLE_STATUS` is true for `in-progress` and for `done`. `check_runners` allows a runner for those two statuses. It refuses a runner for `pending`, `archived` and `superseded`. It refuses a missing runner when the row names no story, and when the story is `done`. `check_baseline` allows an entry for a null story and for an in-progress or done story. `wasm.cold_start` names no story, has `tools/bench/src/runners/wasm_cold_start.mjs`, and has a baseline entry. The same state test that carries the header asserts that an in-progress story with a runner is `measured`. The registry test's own table sets `in-progress` to true. The remediation rewrote the subjects.json sentence and deleted the "most rows, decision D7" half. It kept the not-done rule. The F-006 sentence is present tense about the guard. It is not the close-time count of subjects that had nothing to measure.

**Evidence**: `scripts/bench_check.py:82-88` and `scripts/bench_check.py:223-248`. `scripts/tests/test_bench_check.py:171-177` expects no problem for an in-progress story that has both a runner and a baseline entry. `scripts/tests/test_bench_check.py:186-191` expects a problem when a done story has no runner. `tools/bench/tests/state_test.mjs:95-100` expects `measured`. `node tools/bench/run.mjs --list` exited 0 and printed a blocking story and a backlog status for each row. It did not print runner or measured state. The done rows were F-023, F-027, F-026, F-028 and F-004. `wasm.cold_start` printed `subject exists today`. The union of baseline subject keys is `decode.frame`, `decode.transfer_syntax.htj2k`, `decode.transfer_syntax.jpeg2000`, `decode.transfer_syntax.jpegls` and `wasm.cold_start`.

### D2, the status line still says no pixel, LUT or geometry code exists

**Where**: `README.md:9-10`

**What**: The status paragraph says no pixel, LUT or geometry port code has been written, and that this is decision D7 holding.

**Why it is wrong**: The LUT chain is in `ocelli-pixel`. `Volume::assemble` derives geometry from the image plane. `RenderGraph::tick` draws into a caller-supplied texture view. `CLAUDE.md` records that this same sentence was already false by S09. Decision D7 still means the oracle has no Ocelli frame to judge. It does not mean those crates were not written. The next sentence in the README only withdraws the older "no imaging code" wording.

**Evidence**: `README.md:9-14`. `CLAUDE.md` states that the sentence used to say no pixel, LUT or geometry port code had been written, and that by S09 every word of that was false. `crates/ocelli-volume/src/lib.rs:425-433` projects slice position. `crates/ocelli-render/src/graph.rs:811` submits one encoder.

### D3, recovery is documented as re-resolving the tier

**Where**: `crates/ocelli-render/src/graph.rs:693-694`

**What**: The comment on `install_recovered` says the rebuilt tier is copied because a fallback adapter may resolve differently from the device that was lost.

**Why it is wrong**: `GpuContext::recover` calls `open` on the `ResolvedAdapter` it is given. `ResolvedAdapter::open` passes `self.resolution.caps` into `GpuContext::new` and does not resolve the tier again. The recovery test says the caps match because they are copied from the same `ResolvedAdapter`, and it asserts `recovered.caps == caps_before`. `docs/lld/gpu-ownership.md:290-298` says those caps cannot differ and that the tier must not be re-resolved, because HLD section 7 resolves it once at startup. The three `recover` call sites pass that same adapter. A fallback adapter is a startup signal in tier resolution. It is not a step in `recover`.

**Evidence**: `crates/ocelli-render/src/probe.rs:320-352`, `crates/ocelli-render/src/gpu.rs:306` and `crates/ocelli-render/src/gpu.rs:505-526`, `crates/ocelli-render/tests/device.rs:184` and `crates/ocelli-render/tests/device.rs:213`.

### D4, a texture type is still said not to exist

**Where**: `docs/sprints/AS_BUILT.md:3254-3255`

**What**: The F-031 close says a `Budgeted` implementation for a texture that no texture type exists to be is a `bytes()` nobody can check.

**Why it is wrong**: A single-line search misses it because `no` and `texture type` are on different lines. Read as a sentence, it says no texture type exists. `crates/ocelli-render/src/probe.rs:716` creates a `wgpu::Texture`. `GpuSlot` at `crates/ocelli-render/src/graph.rs:245-246` stores a `wgpu::TextureView`. F-031 created no production `Budgeted` value, and a GPU texture is still not one of those values. That is a different claim from the type being absent.

**Evidence**: the lines above. `rg` for `no texture type` and `has no texture type` prints nothing, which is why this sentence survived the previous search.

## Smells

None.

## Nitpicks

None.

## Verified clean

Checked on this tree. Pass 5 was a list of claims, not evidence. `git write-tree` at the start, and again after the spacing line was restored, returned `07831fb1fa438e95310165ad0da449f23a7c6bce`. `git diff -- crates/ocelli-volume/src/lib.rs` is empty. The sprint gate, the oracle and the bench were not run. `node tools/bench/run.mjs --list` was run. It does not measure.

**Pass 5 D1, the sentences that are gone.** `scripts/bench_check.py`, `tools/bench/run.mjs`, `ci/bench-baseline.json`, `docs/lld/benchmarks.md`, `tools/bench/subjects.json` and `AGENTS.md` no longer say that most subjects have nothing to measure because there is no decoder, no renderer, no worker and no boundary, and they no longer call that decision D7. The baseline says a row that names no story exists today, and that a row whose story is in progress or done exists today. `check_baseline` refuses an entry only when the story is not null and the status is not measurable. The docstring lists three refusals and does not put a second count beside that list. Twelve subjects are registered. Six have a subject today: `wasm.cold_start` plus F-023, F-026, F-027, F-028 and F-004, all `done` except the null story. Six runner files exist for those six. Five baseline entries exist across two host classes, and their union is the five keys named under D1. `tier.startup_microbenchmark` has a runner and no baseline entry, which the gate allows. The six pending rows are three on F-101 and three on F-X003. F-X003 is CPU raster and is `pending`. Half is not most, and the rewritten openings no longer say most. `CHANGELOG.md` still says most subjects had nothing to measure when the harness landed. That verb is past, and it was not refiled.

**Pass 5 D2.** `docs/lld/benchmarks.md:142-145` says a missing runner fails the gate when the row names no story and when the story is done, that an in-progress story may lack one, and that a pending, archived or superseded story must not have one. That matches `check_runners`. The layout row at line 70 says the same. `docs/lld/benchmarks.md:326-333` includes those cases and does not say a `no_runner` row is ignored. The sentence at lines 97-98 names the refusal `run_test.mjs` reaches. That test refuses a pending runner, and the expected text is `rather than in-progress or done`. It does not say every story that is not done is refused.

**Pass 5 D3, the recovery paragraph.** `docs/lld/gpu-ownership.md:285-288` says `Recovered` carries `Caps` and, when a graph registered them, `GraphResources`. It says that value does not carry a canvas or a texture. It does not say the tree has no texture. `GraphResources` is `pipelines_restored: u32` and `cache_generation: u64` at `crates/ocelli-render/src/gpu.rs:84-88`. `Recovered` is `caps` and `graph: Option<GraphResources>` at lines 111-116. The exception is D4.

**Oracle frame.** `rg` for `until F-040`, `until that lands`, `stays open for F-039`, `no port code exists` and `no frame exists` prints nothing. `crates/ocelli-render/src/graph.rs:7-10` and `docs/lld/render-graph.md:49-50` say no surface is acquired, no texture is uploaded, and the oracle still has no Ocelli frame to judge. They name presentation as F-039 and upload as F-040. They do not name either story, or F-X021, as the close of decision D7. `docs/sprints/BACKLOG.md` has F-X021 pending, the candidate-render gate, depending on F-012 and F-052. `docs/lld/comparator.md:1297` says no Ocelli producer exists and names F-X021 as the activation of that requirement. The bullet at `comparator.md:1268` is under what the comparator does not build. `rg` for `create_surface` and `wgpu::Surface` under `crates/ocelli-render` finds nothing. `RenderGraph::tick` calls `queue().submit` once, at `graph.rs:811`. The other `queue().submit` calls are tests and the recovery probe in `gpu.rs`.

**Brick offset.** The F-036 close says voxel 200, 130, 599 is in brick 3, 2, 9, and that brick's origin, voxel 192, 128, 576, is at byte offset 302121344. Independent of the comment: `200 // 64`, `130 // 64` and `599 // 64` are 3, 2 and 9. Times 64 the origin is 192, 128, 576. Section 19's `((z * dy + y) * dx + x) * 2` with dimensions 512, 512, 600 is 302121344 for that origin. The eight-bit extent 116 by 44 by 2 is 10208.

**Recovery, apart from D3.** `recover` copies the registration, bumps `cache_generation`, and assigns the new context at `gpu.rs:317`, which drops the old device handle. The comment at lines 302-305 says the copied registration is the count and the generation, the caller reinstalls the pipelines, and the old handle is dropped at the assignment. `install_recovered` at `graph.rs:696-713` calls `compile`, replaces `self.pipelines`, and sets every per-viewport slot to `None`.

**Gpu gate.** The arm is `cargo test -p ocelli-render -- --ignored --test-threads=1` at `bin/ocelli.sh:344`. The comment names no test file and writes no count. `docs/lld/gpu-ownership.md:304-306` says the gate is every `#[ignore]`d test in the crate and points at the list command.

**Value types.** `impl Budgeted` for production values is `DecodedFrame` at `crates/ocelli-cache/src/image.rs:90`, `Brick` at `crates/ocelli-cache/src/brick.rs:344`, and `Volume` at `crates/ocelli-volume/src/lib.rs:554`. `Block`, `Shifting`, `GpuTexture` and `DecodedSeries` are test doubles. `CLAUDE.md` names the three production types and says an encoded byte run and a GPU texture are not among them.

**LLD headers.** Every `docs/lld/*.md` F-ID header was compared with its row in `docs/lld/README.md`, joining the wrapped continuation in `oracle.md`. Twenty files, twenty rows, zero mismatches. `oracle.md` and `comparator.md` changed date only. Their F-ID lists match the index.

**Cache kinds.** Kinds are 1 evicted, 2 displaced, 3 refused, 4 pressure and 5 budget changed, at `crates/ocelli-cache/src/image.rs:130-141`. `ImageCache::insert` pushes one `CacheEvent` per `Admission` field. `BrickCache::insert` returns `Admission` and builds no `CacheEvent`, at `brick.rs:368`. `TierCounters::record` walks `admission.evicted` and does not read `displaced`. A displacement test in `lru.rs` asserts `evicted` is empty when `displaced` is set.

**F-033 spacing.** Inter-slice spacing is the median of `position_vector().dot(slice_normal())` at `crates/ocelli-volume/src/lib.rs:425-493`. The crate does not read `SpacingBetweenSlices` or `SliceThickness` as values. `PixelSpacing` input `[0.5, 1.0]` is stored as spacing `[1.0, 0.5, 2.0]`, and `row_spacing_multiplies_the_column_direction_cosine` expects the column step `(0.0, 0.5, 0.0)`, so `PixelSpacing[0]` scales the column direction. One mutation replaced `position_vector().dot(normal)` with `position_vector().z`. `bin/ocelli.sh test ocelli-volume --test assembly an_oblique_series_sorts_by_projection_and_not_by_ipp_z -- --exact` failed at `assembly.rs:352` because `volume.is_ok()` was false. The test command's status was 101. The line was edited back.

**Tiers.** The five AS_BUILT entries name A, B and C. F-032 and F-034 are n/a on all three. F-033 is n/a, n/a and full. F-036 is full on all three for host arithmetic, with the 128 edge inside the tier B 256 floor and no upload. F-038 is full on A, full by construction and never exercised on B, and unavailable on C. `RenderGraph::new` returns `TierCUnavailable` when the tier is `Cpu`. An uncompiled blend is skipped in `tick` and reported unavailable. None of those paths substitutes a different numeric result.

**D-25.** `docs/hld/DEVIATIONS.md` has one `| D-25 |` row. `Volume::present` is `SlicePresence` over `words: Vec<u64>` at `crates/ocelli-volume/src/lib.rs:221-222`. `rg bitvec` over `Cargo.toml` finds nothing.

**Builders.** `rg` for `no volume builder` and `future volume builder` prints nothing.

**F-096.** `git grep -n F-096 -- bin ci crates scripts tools .githooks` prints the six historical sites the benchmark LLD names. A raw `grep -rn` on this machine also prints ignored `tools/bench/out/run.json` and a `scripts/__pycache__` file. Those are not in the staged tree.

**P0 notes.** `allocation.json` has the `P0 KILL CRITERION` note on F-027 and on F-028 and on no other story. F-006's separate note is `Numbers feed the P0 kill criteria`.

**Status column.** `docs/sprints/CURRENT_SPRINT.md:18-19` says the column is hand-typed and that the backlog is the authority. Not filed.

**Not claimed done.** The sprint still says the M2 release, deviation D-22, F-X011, ICC, multi-component JPEG-LS and the unscheduled LINEAR range were carried in and not taken.

**Boundary.** `rg wasm_bindgen` and `rg unsafe` over `crates/ocelli-cache`, `crates/ocelli-volume` and `crates/ocelli-render/src/graph.rs` find nothing. The graph does not reference `ImageCache`, `BrickCache` or `VolumeCache`.
