# S12 whole-sprint review, pass 5

**Reviewed**: staged tree `2a2363cccfea087a39b4999305f2b52fbbd85c19` on `sprint/s12`. HEAD is `0ba66c7e5c7b78e6930d2dd1e81c373e40b814ae`. The sprint diff is `git diff 4976c929b8a7c0c081f6aeb2216dd5414571377c`. The remediation since HEAD is `git diff HEAD`, which matches `git diff --cached` on the tracked paths.
**Result**: 3 defects, 0 smells, 0 nitpicks

## Defects

### D1, the benchmark split is still denied outside the rewritten opening

**Where**: `scripts/bench_check.py:16-18`, `tools/bench/run.mjs:5-7`, and `ci/bench-baseline.json:8`

**What**: The gate module says most registry rows have nothing to measure because there is no decoder, no renderer, no worker and no boundary, and that this is decision D7. The driver says most subjects have nothing to measure because D7 puts the instruments before the port code. The baseline file says most rows have no subject in this tree.

**Why it is wrong**: That is the pass 4 claim, moved rather than removed. Twelve subjects are registered. Six have a subject today. `wasm.cold_start` exists today, and F-023, F-026, F-027, F-028 and F-004 are `done`. Six are pending, three on F-101 and three on F-X003. Six runner files exist, including JPEG 2000, HTJ2K, JPEG-LS and the fill-rate runner. Five baseline subjects carry a number. Half is not most. Decoders exist, and three transfer-syntax subjects have recorded numbers. `RenderGraph::tick` draws into a caller-supplied texture view. F-X003 is CPU raster, not the absence of a decoder. The rewrite deleted this cause from `docs/lld/benchmarks.md` and `tools/bench/subjects.json` and left it in these three files.

**Evidence**:

```text
$ node tools/bench/run.mjs --list
EXIT:0
  wasm.cold_start                      subject exists today
  decode.frame                         F-023 (done)
  decode.transfer_syntax.htj2k         F-027 (done)
  decode.transfer_syntax.jpeg2000      F-026 (done)
  decode.transfer_syntax.jpegls        F-028 (done)
  render.first_frame                   F-101 (pending)
  interaction.window_level_drag        F-101 (pending)
  interaction.scroll                   F-101 (pending)
  session.cpu_interactive              F-X003 (pending)
  session.cpu_idle                     F-X003 (pending)
  cine.frame_change_rate               F-X003 (pending)
  tier.startup_microbenchmark          F-004 (done)
```

`ls tools/bench/src/runners` prints `decode_frame.mjs`, `decode_transfer_syntax_htj2k.mjs`, `decode_transfer_syntax_jpeg2000.mjs`, `decode_transfer_syntax_jpegls.mjs`, `tier_startup_microbenchmark.mjs` and `wasm_cold_start.mjs`. The union of subject keys in `ci/bench-baseline.json` is `decode.frame`, `decode.transfer_syntax.htj2k`, `decode.transfer_syntax.jpeg2000`, `decode.transfer_syntax.jpegls` and `wasm.cold_start`. The bench was not executed.

### D2, a done subject with no runner does fail the gate

**Where**: `docs/lld/benchmarks.md:141-146`

**What**: The paragraph says a `no_runner` row does not fail the gate. It says requiring a runner the moment a story is marked done would fail the CI floor. It says a done story whose runner has not been written is not a case the gate requires a runner for.

**Why it is wrong**: `scripts/bench_check.py` `check_runners` appends a problem when the backlog status is `done` and the runner file is absent. `scripts/tests/test_bench_check.py` `test_a_done_story_with_no_runner_is_refused` asserts that problem. The same module's item 3 says a done subject with no runner is refused. The LLD says the gate does not do the thing the gate does. This remediation shortened the paragraph and kept the false rule.

**Evidence**: A direct call of `check_runners` with `decode.frame` at status `done` and an empty runner set printed:

```text
decode.frame is done and has no runner at tools/bench/src/runners/decode_frame.mjs. A completed subject requires its permanent instrument; completion cannot stand in for evidence.
```

The assertion is `scripts/tests/test_bench_check.py:186-191`. The refusing branch is `scripts/bench_check.py:245-250`.

### D3, a texture is in this tree

**Where**: `docs/lld/gpu-ownership.md:285-288`

**What**: Under "What recovery does not restore", the file says a canvas and a texture are not in this tree. The next sentence says F-039 and F-040 extend this struct.

**Why it is wrong**: Everywhere else in this repository, "in this tree" means the source tree. `crates/ocelli-render/src/probe.rs:716` creates a `wgpu::Texture`. `GpuSlot` at `crates/ocelli-render/src/graph.rs:245-246` stores a `wgpu::TextureView`. The fragment entry at `graph.rs:214` declares `texture_2d<f32>`. The following sentence uses "this struct" for `Recovered`, so "this tree" is not that struct. `Recovered` holding neither a canvas nor a texture does not make a texture absent from the tree. Pass 4 paraphrased this sentence as "not types in that struct". The file does not say that. The explicit "this workspace has no texture type" clause was removed from the paragraph above, and this denial was left.

**Evidence**: The lines above, plus `rg -n "in this tree"` which hits this sentence and the source-tree uses in `docs/lld/oracle.md`, `ci/bench-baseline.json`, `tools/bench/run.mjs`, `tools/bench/src/registry.mjs`, `tools/oracle/src/pins.mjs` and `crates/ocelli-codec/tests/htj2k.rs`.

## Smells

None.

## Nitpicks

None.

## Verified clean

Checked on this tree. Pass 4 was a list of claims, not evidence. `git write-tree` at the start, and again after the spacing line was restored, returned `2a2363cccfea087a39b4999305f2b52fbbd85c19`. `git diff -- crates/ocelli-volume/src/lib.rs` is empty. The sprint gate, the oracle and the bench were not run.

**Pass 4 D1, the parts that are gone.** `docs/lld/benchmarks.md` no longer says two of eleven subjects have a runner, no longer says eight are blocked because there is no worker and no boundary, and no longer has the table titled "The state of every subject at merge". That table's HTJ2K and JPEG-LS cells, its omission of JPEG 2000, its fill-rate `no_runner` cell, and its `render.first_frame` chain through F-037, F-038, F-039 and F-040 are gone. The later sentence that the three tier-C rows and the startup subject have no runner is gone. The startup sentence now says the runner invokes `measures_a_fill_rate_on_this_machine` and reads the pixel rate that test prints. `tools/bench/src/runners/tier_startup_microbenchmark.mjs:19` spawns that test, and `crates/ocelli-render/src/probe.rs:1281` prints `pixels per second`. `tools/bench/subjects.json` no longer says the first-frame row is blocked on F-037, F-038, F-039 and F-040. Its note is the historical account that `subject_story` named F-038 until that story landed, and the field is `F-101`. `--list` prints a blocking story and a backlog status. It does not print runner or measured state. The three tier-C rows are F-X003 `pending` and have no runner file and no baseline key. `docs/sprints/allocation.json` has the P0 kill-criterion note on F-027 and on F-028 and on no other story. The F-006 close in `docs/sprints/AS_BUILT.md` and the CHANGELOG line that most subjects had nothing to measure when the harness landed are close-time history, not this present-tense denial.

**Pass 4 D2.** `rg` for `no volume builder` and `future volume builder` prints nothing. `tools/oracle/src/volume.mjs:22-24`, `tools/oracle/volume-truth.json:11` and `docs/lld/oracle.md:976-978` no longer say a volume builder is absent. They say no HLD section states a refusal policy. A search of `docs/hld/` for a non-uniform spacing policy, `SpacingBetweenSlices` and a 0.5 percent threshold still finds nothing, so that narrower claim was not reopened. `Volume::assemble` is `crates/ocelli-volume/src/lib.rs:398` and returns `NonUniformSpacing` at lines 485-490. `docs/lld/oracle.md:905` now says the spacing rule matters to the volume builder, not to a future one. `docs/sprints/BACKLOG.md` has F-033 `done`.

**Pass 4 D3, the explicit denial.** `rg` for `no texture, buffer`, `no texture type` and `has no texture type` prints nothing. The F-037 close no longer says this workspace has no texture, buffer, pipeline or viewport type. The F-041 close no longer says there is no texture type yet for a stored value to arrive through. The exception is D3 above.

**Oracle frame.** `rg` for `until F-040`, `until that lands`, `stays open for F-039`, `no port code exists` and `no frame exists` prints nothing. `crates/ocelli-render/src/graph.rs:8-10` and `docs/lld/render-graph.md:49-50` say the oracle still has no Ocelli frame to judge and do not name F-040, F-039 or F-X021 as the close of decision D7. Presentation is named as F-039 and upload as F-040. `docs/sprints/BACKLOG.md:212` is F-X021, the candidate-render gate, `pending`, depending on F-012 and F-052. `docs/lld/comparator.md:1297` says no Ocelli producer exists and names F-X021. That producer is the candidate record, not the graph. `rg` for `create_surface` and `wgpu::Surface` under `crates/ocelli-render` finds nothing. `RenderGraph::tick` calls `queue().submit` once, at `graph.rs:811`. The other `queue().submit` calls are tests and the recovery probe in `gpu.rs`.

**Brick offset.** The F-036 close says voxel 200, 130, 599 is in brick 3, 2, 9, and that brick's origin, voxel 192, 128, 576, is at byte offset 302121344. `crates/ocelli-cache/tests/bricks.rs:26` asserts the brick. Lines 32-40 assert the origin and `byte_offset` of that origin. Independent of the comment: `200 // 64`, `130 // 64` and `599 // 64` are 3, 2 and 9. Times 64 the origin is 192, 128, 576. Section 19's `((z * dy + y) * dx + x) * 2` with dimensions 512, 512, 600 is 302121344 for the origin. The eight-bit extent 116 by 44 by 2 is 10208, which `bricks.rs:57-58` asserts as `samples.len()` and `bytes()`.

**Recovery.** `Recovered` is `caps` and `graph: Option<GraphResources>` at `crates/ocelli-render/src/gpu.rs:111-116`. `GraphResources` is `pipelines_restored: u32` and `cache_generation: u64` at lines 84-88. `recover` copies that registration, bumps `cache_generation`, and assigns the new context at line 317, which drops the old device handle. The comment at lines 299-305 says the copied registration is the count and the generation, the caller reinstalls the pipelines, and the old device handle is dropped at the assignment. `install_recovered` at `graph.rs:688-713` calls `compile`, replaces `self.pipelines`, and sets every per-viewport slot to `None`.

**Gpu gate.** The arm is `cargo test -p ocelli-render -- --ignored --test-threads=1` at `bin/ocelli.sh:344`. The comment names no test file and writes no count. `docs/lld/gpu-ownership.md:304-306` says the gate is every `#[ignore]`d test in the crate and points at the list command.

**Value types.** `impl Budgeted` for production values is `DecodedFrame` at `crates/ocelli-cache/src/image.rs:90`, `Brick` at `crates/ocelli-cache/src/brick.rs:344`, and `Volume` at `crates/ocelli-volume/src/lib.rs:554`. `Block`, `Shifting`, `GpuTexture` and `DecodedSeries` are test doubles. `CLAUDE.md` names the three production types and says an encoded byte run and a GPU texture are not among them.

**LLD headers.** A comparison of every `docs/lld/*.md` F-ID header with its row in `docs/lld/README.md` found no mismatch. Twenty files, twenty rows, zero mismatches. `oracle.md` and `comparator.md` changed date only. Their F-ID lists match the index, which does not store the date.

**Cache kinds.** Kinds are 1 evicted, 2 displaced, 3 refused, 4 pressure and 5 budget changed, at `crates/ocelli-cache/src/image.rs:130-141`. `ImageCache::insert` pushes one `CacheEvent` per `Admission` field. `BrickCache::insert` returns `Admission` and builds no `CacheEvent`, at `brick.rs:368-369`. `TierCounters::record` walks `admission.evicted` and does not read `displaced`. `Lru` keeps `displaced` as `Option` and `evicted` as a separate vector. A displacement test in `lru.rs` asserts `evicted` is empty when `displaced` is set.

**F-033 spacing.** Inter-slice spacing is the median of `position_vector().dot(slice_normal())` at `crates/ocelli-volume/src/lib.rs:425-493`. The crate does not read `SpacingBetweenSlices` or `SliceThickness` as values. `PixelSpacing` input `[0.5, 1.0]` is stored as spacing `[1.0, 0.5, 2.0]`, and `row_spacing_multiplies_the_column_direction_cosine` expects the column step `(0.0, 0.5, 0.0)`, so `PixelSpacing[0]` scales the column direction. One mutation replaced `position_vector().dot(normal)` with `position_vector().z`. `bin/ocelli.sh test ocelli-volume --test assembly an_oblique_series_sorts_by_projection_and_not_by_ipp_z -- --exact` failed at `assembly.rs:352` because `volume.is_ok()` was false. The test command's status was 101. The line was edited back.

**Tiers.** The five AS_BUILT entries name A, B and C. F-032 and F-034 are n/a on all three. F-033 is n/a, n/a and full. F-036 is full on all three for host arithmetic, with the 128 edge inside the tier B 256 floor and no upload. F-038 is full on A, full by construction and never exercised on B, and unavailable on C. `RenderGraph::new` returns `TierCUnavailable` when the tier is `Cpu`. An uncompiled blend is skipped in `tick` and reported unavailable. None of those paths substitutes a different numeric result.

**D-25.** `docs/hld/DEVIATIONS.md` has one `| D-25 |` row. `Volume::present` is `SlicePresence` over `words: Vec<u64>` at `crates/ocelli-volume/src/lib.rs:221-222`. `rg bitvec` over `Cargo.toml` finds nothing.

**Status column.** `docs/sprints/CURRENT_SPRINT.md:18-19` says the column is hand-typed and that the backlog is the authority. Not filed.

**Not claimed done.** The sprint still says the M2 release, deviation D-22, F-X011, ICC, multi-component JPEG-LS and the unscheduled LINEAR range were carried in and not taken.

**Boundary.** `rg wasm_bindgen` and `rg unsafe` over `crates/ocelli-cache`, `crates/ocelli-volume` and `crates/ocelli-render/src/graph.rs` find nothing. The graph does not reference `ImageCache`, `BrickCache` or `VolumeCache`.
