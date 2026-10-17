# S12 whole-sprint review, pass 7

**Reviewed**: staged tree `52cc7f252189548c17a0c4f45c97e8f72f88759a` on `sprint/s12`. HEAD is `0ba66c7e5c7b78e6930d2dd1e81c373e40b814ae`. The sprint diff is `git diff 4976c929b8a7c0c081f6aeb2216dd5414571377c`. The remediation since HEAD is `git diff HEAD`, which matches `git diff --cached` on the tracked paths.
**Result**: 2 defects, 0 smells, 0 nitpicks

## Defects

### D1, a row that is not in progress or done is still described as unmeasurable

**Where**: `scripts/bench_check.py:16` and `scripts/bench_check.py:27-30`, `scripts/bench_check.py:429-432`, `docs/lld/benchmarks.md:32-35`, `docs/lld/benchmarks.md:97-98`, `docs/lld/benchmarks.md:136`, and `docs/sprints/AS_BUILT.md:986-988`

**What**: The bench gate's module text says a row whose story is not `in-progress` or `done` has nothing to measure yet. It says the gate refuses a runner for that row, and a baseline entry for such a subject. The failure text says a subject whose story has not landed is never given a runner and is never given a number. The benchmark LLD's opening list says the same two refusals. Its state table says a refused number is a number for a subject whose story has not landed. The same LLD says `run_test.mjs` reaches the refusal of a runner whose subject story is not done. The F-006 note says the guard refuses a runner unless the story is in progress or done.

**Why it is wrong**: `check_runners` requires a runner when `subject_story` is null. It allows a runner when the status is `in-progress` or `done`. It refuses a runner for `pending`, `archived` and `superseded`. It refuses a missing runner when the story is `done`, and when the row names no story. `check_baseline` allows an entry for a null story and for an in-progress or done story. It refuses an entry only when the story is not null and the status is not measurable. `wasm.cold_start` names no story, has a runner, and has a baseline entry. An in-progress story may have both. A null story is not a subject that does not exist. `run_test.mjs` refuses one pending runner. The expected text is `rather than in-progress or done`, not a refusal of every story that is not done. Later sentences in `docs/lld/benchmarks.md` state the narrower rule. They do not make the broader sentences true.

**Evidence**: `scripts/bench_check.py:223-248` and `scripts/bench_check.py:288-297`. `scripts/tests/test_bench_check.py:158-177` expects a problem for a subject that exists today and has no runner, and expects no problem for an in-progress story that has a runner and a baseline entry. `tools/bench/tests/run_test.mjs:31-49` expects the pending wording. `node tools/bench/run.mjs --list` exited 0. It printed a blocking story and a backlog status for each of 12 rows. It did not print runner or measured state. `wasm.cold_start` printed `subject exists today`. The done rows were F-023, F-027, F-026, F-028 and F-004. Six runner files exist under `tools/bench/src/runners/`. The union of baseline subject keys is `decode.frame`, `decode.transfer_syntax.htj2k`, `decode.transfer_syntax.jpeg2000`, `decode.transfer_syntax.jpegls` and `wasm.cold_start`. `tier.startup_microbenchmark` has a runner and no baseline entry.

### D2, the F-006 close still speaks of today's bench inventory in the present

**Where**: `docs/sprints/AS_BUILT.md:897-902`

**What**: The past clause says nine of the eleven subjects had nothing to measure when F-006 landed. The next sentences say all nine are blocked on a story that has not landed, that this is decision D7 holding, and that `tier.startup_microbenchmark` reports a done story and still has no runner.

**Why it is wrong**: `are` and `still has` are about this tree. `tier.startup_microbenchmark` has `tools/bench/src/runners/tier_startup_microbenchmark.mjs`. F-004, F-023, F-026, F-027 and F-028 are `done` in `docs/sprints/BACKLOG.md`, and each of those rows has a runner. `--list` shows six rows with a subject today and six pending rows. That is not nine blocked rows, and it is not a done story with no runner. Decision D7 is that the oracle has no Ocelli frame to judge. It is not a claim that those rows are still blocked.

**Evidence**: the runner list and the `--list` output named under D1. The past clause in the same paragraph, and `CHANGELOG.md` saying most subjects had nothing to measure when the harness landed, were not refiled.

## Smells

None.

## Nitpicks

None.

## Verified clean

Checked on this tree. Pass 6 was a list of claims, not evidence. `git write-tree` at the start, and again after the spacing line was restored, returned `52cc7f252189548c17a0c4f45c97e8f72f88759a`. `git diff -- crates/ocelli-volume/src/lib.rs` is empty. The sprint gate, the oracle and the bench were not run. `node tools/bench/run.mjs --list` was run. It does not measure.

**Pass 6 D1, the sentences that now match the gate.** `tools/bench/subjects.json` says a pending, archived or superseded story is refused a runner and a baseline entry, that an in-progress story may have both, that a done story must have a runner, and that a row that names no story must have a runner. `tools/bench/tests/state_test.mjs` says those three statuses resolve to `unavailable`, and that an in-progress or done story may resolve to `measured`. `tools/bench/tests/registry_test.mjs` says measurable means in-progress or done, and that any other status is refused a runner and a baseline entry. `ci/bench-baseline.json` says a row that names no story exists today, and that a row whose story is in progress or done exists today. `docs/lld/benchmarks.md:142-145` says a missing runner fails the gate when the row names no story and when the story is done, that an in-progress story may lack one, and that a pending, archived or superseded story must not have one. `docs/lld/benchmarks.md:329-332` says the same and adds the baseline half. `docs/lld/benchmarks.md:70` says a null story must have a runner, a done story must have one, and an in-progress story may lack one. Those sentences are not D1. The openings and the F-006 note above them are. Twelve subjects are registered. Six runner files exist. Five baseline keys exist across two host classes. The six pending rows are three on F-101 and three on F-X003. F-X003 is `pending`. No new written count of subjects, runners or baseline keys was found in the remediation.

**Pass 6 D2.** `README.md:9-12` no longer says no pixel, LUT or geometry port code has been written. It says decision D7 holds in the narrow sense that the oracle has no Ocelli frame to judge. `CLAUDE.md` records that the old sentence was false by S09, and it names `DecodedFrame`, `Brick` and `Volume` as the value types.

**Pass 6 D3.** `install_recovered` says `recovered.caps` is the tier `open` copied from the same adapter, and that recovery does not resolve a tier. `GpuContext::recover` calls `open` on the given `ResolvedAdapter`. `ResolvedAdapter::open` passes `self.resolution.caps` into `GpuContext::new` and does not resolve the tier again. The three `recover` call sites pass that same adapter. The recovery test says the caps match because they are copied from the same `ResolvedAdapter`, and it asserts `recovered.caps == caps_before`.

**Pass 6 D4.** The F-031 close no longer says no texture type exists. It says F-031 created no value type, that `DecodedFrame`, `Brick` and `Volume` landed later, and that an encoded byte run and a GPU texture are still not production `Budgeted` values. `probe.rs` creates a `wgpu::Texture`. `GpuSlot` stores a `wgpu::TextureView`. `impl Budgeted` for production values is `DecodedFrame`, `Brick` and `Volume`. `Block`, `Shifting`, `GpuTexture` and `DecodedSeries` are test doubles.

**Oracle frame.** `rg` for `until F-040`, `until that lands`, `stays open for F-039`, `no port code exists` and `no frame exists` prints nothing. `crates/ocelli-render/src/graph.rs:7-10` and `docs/lld/render-graph.md:49-50` say no surface is acquired, no texture is uploaded, and the oracle still has no Ocelli frame to judge. They name presentation as F-039 and upload as F-040. They do not name either story, or F-X021, as the close of decision D7. `docs/sprints/CURRENT_SPRINT.md:70-72` says the same and does not say D7 waits for F-040. The F-038 note that a presented-frame ring event does not exist until F-039, F-040 and F-101 land is the end of `render.first_frame`, not a close of D7. `docs/lld/comparator.md:1274` names F-X021 as activation of the candidate record. `comparator.md:1297` says no Ocelli producer exists and names F-X021 as activation of that requirement. The bullet at `comparator.md:1268` is under what the comparator does not build. `rg` for `create_surface` and `wgpu::Surface` under `crates/ocelli-render` finds nothing. `RenderGraph::tick` calls `queue().submit` once, at `graph.rs:811`. The other `queue().submit` calls are tests and the recovery probe in `gpu.rs`.

**Brick offset.** The F-036 close says voxel 200, 130, 599 is in brick 3, 2, 9, and that brick's origin, voxel 192, 128, 576, is at byte offset 302121344. Independent of the comment: `200 // 64`, `130 // 64` and `599 // 64` are 3, 2 and 9. Times 64 the origin is 192, 128, 576. `((z * dy + y) * dx + x) * 2` with dimensions 512, 512, 600 is 302121344 for that origin. The eight-bit extent 116 by 44 by 2 is 10208. `checked_linear` is that product in `u64`.

**Recovery.** `recover` copies the registration, bumps `cache_generation`, and assigns the new context at `gpu.rs:317`, which drops the old device handle. `GraphResources` is `pipelines_restored: u32` and `cache_generation: u64`. `Recovered` is `caps` and `graph: Option<GraphResources>`. The comment says the copied registration is the count and the generation, the caller reinstalls the pipelines, and the old handle is dropped at the assignment. `install_recovered` calls `compile`, replaces `self.pipelines`, and sets every per-viewport slot to `None`. `docs/lld/gpu-ownership.md:285-288` says `Recovered` carries `Caps` and, when a graph registered them, `GraphResources`. It says that value does not carry a canvas or a texture. It does not say the tree has no texture.

**Gpu gate.** The arm is `cargo test -p ocelli-render -- --ignored --test-threads=1` at `bin/ocelli.sh:344`. The comment names no test file as the set and writes no count. It quotes the design plan's old `--test device --test voi_shader` spelling as the list the arm is not. `docs/lld/gpu-ownership.md:304-306` says the gate is every `#[ignore]`d test in the crate and points at the list command.

**Cache kinds.** Kinds are 1 evicted, 2 displaced, 3 refused, 4 pressure and 5 budget changed, at `crates/ocelli-cache/src/image.rs:130-141`. `ImageCache::insert` pushes one `CacheEvent` per `Admission` field. `BrickCache::insert` returns `Admission` and builds no `CacheEvent`, at `brick.rs:368`. `TierCounters::record` walks `admission.evicted` and does not read `displaced`. A displacement test in `lru.rs` asserts `evicted` is empty when `displaced` is set.

**F-033 spacing.** Inter-slice spacing is the median of `position_vector().dot(slice_normal())` at `crates/ocelli-volume/src/lib.rs:425-493`. The crate does not read `SpacingBetweenSlices` or `SliceThickness` as values. `PixelSpacing` input `[0.5, 1.0]` is stored as spacing `[1.0, 0.5, 2.0]`, and `row_spacing_multiplies_the_column_direction_cosine` expects the column step `(0.0, 0.5, 0.0)`, so `PixelSpacing[0]` scales the column direction. One mutation replaced `position_vector().dot(normal)` with `position_vector().z`. `bin/ocelli.sh test ocelli-volume --test assembly an_oblique_series_sorts_by_projection_and_not_by_ipp_z -- --exact` failed at `assembly.rs:352` because `volume.is_ok()` was false. The test command's status was 101. The line was edited back.

**Tiers.** The five AS_BUILT entries name A, B and C. F-032 and F-034 are n/a on all three. F-033 is n/a, n/a and full. F-036 is full on all three for host arithmetic, with the 128 edge inside the tier B 256 floor and no upload. F-038 is full on A, full by construction and never exercised on B, and unavailable on C. `RenderGraph::new` returns `TierCUnavailable` when the tier is `Cpu`. An uncompiled blend is skipped in `tick` and reported unavailable. None of those paths substitutes a different numeric result.

**D-25.** `docs/hld/DEVIATIONS.md` has one `| D-25 |` row. `Volume::present` is `SlicePresence` over `words: Vec<u64>` at `crates/ocelli-volume/src/lib.rs:221-222` and `lib.rs:362`. `rg bitvec` over `Cargo.toml` finds nothing.

**LLD headers.** Every `docs/lld/*.md` F-ID header was compared with its row in `docs/lld/README.md`, joining the wrapped continuation in `oracle.md`. Twenty files, twenty rows, zero mismatches. `gpu-ownership.md` and `benchmarks.md` now include F-038. `render-graph.md` is F-038 only. Those match the index.

**F-096.** `git grep -n F-096 -- bin ci crates scripts tools .githooks` prints six historical sites. They are `bin/ocelli.sh`, `crates/ocelli-wasm/Cargo.toml`, `scripts/bench_check.py`, `tools/bench/src/registry.mjs`, and two blocks of `tools/bench/subjects.json`.

**P0 notes.** `allocation.json` has the `P0 KILL CRITERION` note on F-027 and on F-028 and on no other story. F-006's separate note is `Numbers feed the P0 kill criteria`.

**Builders.** `rg` for `no volume builder` and `future volume builder` prints nothing.

**Status column.** `docs/sprints/CURRENT_SPRINT.md:18-19` says the column is hand-typed and that the backlog is the authority. Not filed. The backlog rows for F-032, F-033, F-034, F-036 and F-038 are `done`.

**Not claimed done.** The sprint still says the M2 release, deviation D-22, F-X011, ICC, multi-component JPEG-LS and the unscheduled LINEAR range were carried in and not taken.

**Boundary.** `rg wasm_bindgen` and `rg unsafe` over `crates/ocelli-cache`, `crates/ocelli-volume` and `crates/ocelli-render/src/graph.rs` find nothing. The graph does not reference `ImageCache`, `BrickCache` or `VolumeCache`.
