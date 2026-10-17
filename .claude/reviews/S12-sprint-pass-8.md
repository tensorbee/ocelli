# S12 whole-sprint review, pass 8

**Reviewed**: staged tree `a4e6d845a9db59b24485bbe77265ef86b5d304c1` on `sprint/s12`. HEAD is `0ba66c7e5c7b78e6930d2dd1e81c373e40b814ae`. The sprint diff is `git diff 4976c929b8a7c0c081f6aeb2216dd5414571377c`. The remediation since HEAD matches `git diff --cached` on the tracked paths. Untracked `session-manager-plugin.pkg` was not staged and is not part of this review.
**Result**: 4 defects, 0 smells, 0 nitpicks

## Defects

### D1, the bench registry is still counted as eleven rows and seven durations

**Where**: `tools/bench/src/record.mjs:45` and `tools/bench/tests/record_test.mjs:115`

**What**: The record module says seven of the eleven registry rows are durations and four are not. The record test says four of the eleven registry rows are not durations. Both sentences are present tense.

**Why it is wrong**: `tools/bench/subjects.json` has twelve subjects. Eight use `ms`. Four do not: `session.cpu_interactive` and `session.cpu_idle` use `fraction_of_one_vcpu`, `cine.frame_change_rate` uses `changed_frames_per_second`, and `tier.startup_microbenchmark` uses `pixels_per_second`. Four non-durations is still the size of that subset. Eleven rows is not, and seven durations is not. `INCREASE_MEANS` declaring four units is a different claim and is still true.

**Evidence**: `node tools/bench/run.mjs --list` exited 0 and printed twelve rows. A count of `subjects.json` printed `subjects 12` and `units {'ms': 8, 'fraction_of_one_vcpu': 2, 'changed_frames_per_second': 1, 'pixels_per_second': 1}`.

### D2, the bench command still says one subject exists today

**Where**: `bin/ocelli.sh:591-594`

**What**: The `bench` arm says the one subject that has a subject today is timed in a browser, and that a missing playwright install would make the harness report its single measurable subject as a runner failure.

**Why it is wrong**: Six rows have a subject on this tree, and six runner files exist. `wasm.cold_start` names no story. The done rows are F-023, F-026, F-027, F-028 and F-004. Only `tools/bench/src/runners/wasm_cold_start.mjs` imports `playwright`, and it does so inside `run`. The decode runners and `tier_startup_microbenchmark.mjs` spawn a process and do not import it. A missing browser is not the failure of one measurable subject.

**Evidence**: the `--list` output named under D1, `ls tools/bench/src/runners`, and the import lines in those six runner files. `runSubjects` still runs every row whose subject exists and whose runner file exists.

### D3, landing F-038 is still described as the gate refusing a runner

**Where**: `tools/bench/subjects.json:92`

**What**: The `render.first_frame` note says `subject_story` named F-038 until F-038 landed, when the bench gate correctly refused a runner for a measurement whose end point does not exist yet.

**Why it is wrong**: `check_runners` requires a runner when the story is `done`. It refuses a runner only when the status is `pending`, `archived` or `superseded`. F-038 is `done`. A row that still named F-038 and had no runner would fail because a done story must have one. It would not fail because a runner was present. `render.first_frame` now names F-101. `--list` printed that story as `pending`. The pending status is what makes a runner a refusal. F-038 landing is the opposite transition.

**Evidence**: `scripts/bench_check.py:225-250`. `docs/sprints/BACKLOG.md` has F-038 as `done`. `--list` printed `render.first_frame` as `F-101 (pending)` and printed no runner state, which this command does not print. No `render_first_frame.mjs` is in `tools/bench/src/runners`.

### D4, "has not landed" is still the present-tense bench refusal

**Where**: `CHANGELOG.md:181-182`, `scripts/guards/catalogue.py:10758-10759`, `docs/runbooks/guard-verification.md:682`, and `tools/bench/src/registry.mjs:11-12`

**What**: The changelog says a recorded number for a subject whose story has not landed is refused by the bench gate. The guard catalogue, and the same sentence in the runbook, say `tools/bench/src/registry.mjs` refuses a registry entry with no subject story and a runner for a subject whose story has not landed. The registry module says no record can ever carry a number beside a story that has not landed.

**Why it is wrong**: A baseline entry is refused only when the story is not null and the status is not measurable. Measurable is in-progress or done. An in-progress story may have a number. `resolveSubjects` sets `subjectExists` from that same table, so an in-progress story can carry a value. `registry.mjs` does not read runner files. `tools/bench/tests/registry_test.mjs` does not drive a runner. The runner refusal that does exist is `run.mjs`, and `run_test.mjs` expects one pending row with the text `pending rather than in-progress or done`. A null `subject_story` is also accepted. `wasm.cold_start` is that row, so "a registry entry with no subject story" is not what the parser refuses.

**Evidence**: `scripts/bench_check.py:84-90` and `scripts/bench_check.py:290-299`. `tools/bench/src/registry.mjs:179-208`. `tools/bench/src/state.mjs:61-70`. `tools/bench/tests/run_test.mjs:31-49`. The past clause in `CHANGELOG.md:183-184`, that most subjects had nothing to measure when the harness landed, was not refiled.

## Smells

None.

## Nitpicks

None.

## Verified clean

`git write-tree` at the start, and again after the spacing line was restored, returned `a4e6d845a9db59b24485bbe77265ef86b5d304c1`. `git diff -- crates/ocelli-volume/src/lib.rs` is empty. The sprint gate, the oracle and the bench measurement were not run. `node tools/bench/run.mjs --list` was run. It exited 0. It prints a blocking story and a backlog status. It does not print runner or measured state and it does not take the GPU.

**Pass 7 D1, the rewritten sentences.** `scripts/bench_check.py` says a pending, archived or superseded story has nothing to measure yet, that a row naming no story exists today, and that the gate refuses a runner and a baseline for those three statuses, plus a missing runner on a done story and on a row that names no story. The failure print says the same and adds that a row naming no story must have a runner. `docs/lld/benchmarks.md` opens with those three refusals. Its state table says a refused number is a number for a pending, archived or superseded subject. It says `run_test.mjs` reaches the refusal of one pending runner and that the message requires the story to be in progress or done. The later paragraph says a row naming no story has a runner, a done story has a runner, and a story which is not in progress or done has neither a runner nor a baseline entry. `docs/sprints/AS_BUILT.md` says the guard refuses a runner for a pending, archived or superseded story, and that a done story and a row that names no story must have one. `tools/bench/subjects.json` says the same split. `MEASURABLE_STATUS` is pending false, in-progress true, done true, archived false and superseded false, in both `scripts/bench_check.py` and `tools/bench/src/registry.mjs`. `check_runners` requires a runner for a null story, allows one for in-progress, requires one for done, and refuses one otherwise. `check_baseline` allows an entry for a null story and for an in-progress or done story. Those sentences were not refiled. The catalogue row for `tools/bench/run.mjs`, and the matching runbook row, say a runner is refused when the story is not in progress or done. That matches `run.mjs`. It was not refiled.

**Pass 7 D2.** The F-006 close still says nine of the eleven subjects had nothing to measure when that story landed. That clause is past tense and was not refiled. The sentences after it point at `bin/ocelli.sh bench --list` and do not say the nine are still blocked, and they do not say `tier.startup_microbenchmark` has no runner. That subject has `tools/bench/src/runners/tier_startup_microbenchmark.mjs`. The nearby "lists the things this project will ever measure" sentence names the registry's job. It is not a count of this tree.

**Oracle frame.** `rg` for `until F-040`, `until that lands`, `stays open for F-039`, `no port code exists` and `no frame exists` prints nothing. `crates/ocelli-render/src/graph.rs` submits once, at `graph.rs:811`, into `slot.target`, which is a caller-supplied `wgpu::TextureView`. `rg` for `create_surface` and `wgpu::Surface` under `crates/ocelli-render` prints nothing. The graph header, `docs/lld/render-graph.md`, `docs/sprints/CURRENT_SPRINT.md` and the F-041 close say no surface is acquired and that decision D7 stays open because the oracle has no Ocelli frame to judge. They do not name F-039, F-040 or F-X021 as the story that closes D7. `docs/lld/comparator.md` names F-X021 as activation of the candidate record. The F-038 note that a presented-frame ring event does not exist until F-039, F-040 and F-101 land is the end of `render.first_frame`, not a close of D7.

**Brick offset.** The F-036 close says voxel 200, 130, 599 is in brick 3, 2, 9, and that brick's origin, voxel 192, 128, 576, is at byte offset 302121344. Independent arithmetic gives the same origin offset, `((576 * 512 + 128) * 512 + 192) * 2`. The voxel itself is 314182032. The eight-bit extent 116 by 44 by 2 is 10208 bytes. The close attaches 302121344 to the origin, not to the voxel.

**Recovery.** `GpuContext::recover` copies `self.graph`, bumps `cache_generation`, calls `adapter.open()`, and assigns `*self = rebuilt`, which drops the old device handle. `ResolvedAdapter::open` passes `self.resolution.caps` into `GpuContext::new` and does not resolve the tier again. `install_recovered` sets `self.tier` from `recovered.caps.tier`. The comment says recovery does not resolve a tier. `rg` for a fallback adapter resolving a different tier in that path prints nothing.

**Gpu gate.** `bin/ocelli.sh` runs `cargo test -p ocelli-render -- --ignored --test-threads=1`. The comment writes no count of ignored tests. It quotes `--test device --test voi_shader` as the design plan's old spelling, and it says that spelling is not the set. `docs/lld/gpu-ownership.md` says the gate is every `#[ignore]`d test in the crate and points at the list command.

**Budgeted values.** Production `impl Budgeted` is `DecodedFrame`, `Brick` and `Volume`. `Block`, `Shifting`, `GpuTexture` and `DecodedSeries` are test doubles. `probe.rs` creates a `wgpu::Texture`. `GpuSlot` stores a `wgpu::TextureView`. `rg` for `no texture type exists` prints nothing.

**Cache kinds.** Kinds are 1 evicted, 2 displaced, 3 refused, 4 pressure and 5 budget changed. `BrickCache::insert` returns `Admission` and builds no `CacheEvent`. `TierCounters::record` walks `admission.evicted` and does not read `displaced`. The F-034 close says a displacement is ignored. No sentence was found that says a displacement inflates `evicted`.

**F-033 spacing.** Inter-slice spacing is the median of `position_vector().dot(slice_normal())`. The crate does not read `SpacingBetweenSlices` or `SliceThickness` as values. `PixelSpacing::new` takes row spacing then column spacing, and the volume stores column spacing, row spacing, then the median. The fixture `[0.5, 1.0]` is asserted as `[1.0, 0.5, 2.0]`, and the column step is `(0.0, 0.5, 0.0)`, so `PixelSpacing[0]` scales the column cosine. One mutation replaced `position_vector().dot(normal)` with `position_vector().z`. `bin/ocelli.sh test ocelli-volume --test assembly an_oblique_series_sorts_by_projection_and_not_by_ipp_z -- --exact` failed at `assembly.rs:352` because `volume.is_ok()` was false. The test command's status was 101. The line was restored.

**Tiers.** The five AS_BUILT entries name A, B and C. F-032 and F-034 are n/a on all three. F-033 is n/a, n/a and full. F-036 is full on all three for host arithmetic, with the 128 edge inside the tier B 256 floor and no upload. F-038 is full on A, full by construction and never exercised on B, and unavailable on C. `RenderGraph::new` returns `TierCUnavailable` when the tier is `Cpu`. An uncompiled blend is skipped in `tick` and reported unavailable.

**D-25.** `docs/hld/DEVIATIONS.md` has one `| D-25 |` row. `Volume::present` is `SlicePresence` over `words: Vec<u64>`. `rg bitvec` over `Cargo.toml` and the crate manifests prints nothing.

**LLD headers.** Every `docs/lld/*.md` F-ID header was compared with its row in `docs/lld/README.md`, joining the wrapped continuation in `oracle.md`. Twenty files, twenty rows, zero mismatches.

**Builders.** `rg` for `no volume builder` and `future volume builder` prints nothing. `Volume::assemble` is in `crates/ocelli-volume/src/lib.rs`.

**F-096.** `git grep -n F-096 -- bin ci crates scripts tools .githooks` prints six historical lines, in `bin/ocelli.sh`, `crates/ocelli-wasm/Cargo.toml`, `scripts/bench_check.py`, `tools/bench/src/registry.mjs`, and two blocks of `tools/bench/subjects.json`.

**P0 notes.** `allocation.json` has the `P0 KILL CRITERION` note on F-027 and on F-028 and on no other story. F-006's separate note is `Numbers feed the P0 kill criteria`.

**Status column.** Not filed. The backlog rows for F-032, F-033, F-034, F-036 and F-038 are `done`.

**Not claimed done.** The sprint still says the M2 release, deviation D-22, F-X011, ICC, multi-component JPEG-LS and the unscheduled LINEAR range were carried in and not taken.

**Boundary.** `rg wasm_bindgen` and `rg unsafe` over `crates/ocelli-cache`, `crates/ocelli-volume` and `crates/ocelli-render/src/graph.rs` print nothing. The graph does not reference `ImageCache`, `BrickCache` or `VolumeCache`.

**Baseline keys observed, not prescribed.** The union across the two host classes is `decode.frame`, `decode.transfer_syntax.htj2k`, `decode.transfer_syntax.jpeg2000`, `decode.transfer_syntax.jpegls` and `wasm.cold_start`. `tier.startup_microbenchmark` has a runner and no baseline entry, which the gate allows.
