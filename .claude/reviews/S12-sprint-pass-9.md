# S12 whole-sprint review, pass 9

**Reviewed**: staged tree `d328fb7aa8ad9cc3424cfc140e6d6e5ecc0418c2` on `sprint/s12`. HEAD is `0ba66c7e5c7b78e6930d2dd1e81c373e40b814ae`. The sprint diff is `git diff 4976c929b8a7c0c081f6aeb2216dd5414571377c`. Untracked `session-manager-plugin.pkg` was not staged and is not part of this review.
**Result**: 2 defects, 0 smells, 0 nitpicks

## Defects

### D1, a runner-path bug is still said to hide the one subject that has a runner

**Where**: `docs/lld/benchmarks.md:474-476` and `tools/bench/tests/paths_test.mjs:9-11`

**What**: Both say a `runnerBasename` that stops replacing dots with underscores makes `existsSync` false for the one subject that has a runner. The benchmarks sentence is present tense. The opening clause about the S03 eighth pass does not put the count in the past.

**Why it is wrong**: Six runner files exist on this tree. Every one of those subject ids contains a dot, so the same bug misses all six, not one.

**Evidence**: `ls tools/bench/src/runners` lists `decode_frame.mjs`, `decode_transfer_syntax_htj2k.mjs`, `decode_transfer_syntax_jpeg2000.mjs`, `decode_transfer_syntax_jpegls.mjs`, `tier_startup_microbenchmark.mjs` and `wasm_cold_start.mjs`.

### D2, the bench test-file count is still six, and the two commands are said to agree

**Where**: `docs/lld/benchmarks.md:491-499`

**What**: The paragraph leads with six as the count of files the bench arm names. It then says `grep -o 'tools/bench/tests/[a-z_]*\.mjs' bin/ocelli.sh | sort -u | wc -l` and `ls tools/bench/tests/*.mjs | wc -l` agree today. The clause that the count reached six in the S03 eighth pass is past tense and is not this defect.

**Why it is wrong**: The arm lists twelve files and the directory holds twelve. The cited grep prints ten. Its pattern cannot match a digit, so it drops `decode_htj2k_test.mjs` and `decode_jpeg2000_test.mjs`. Six is neither result, and the two commands do not agree.

**Evidence**: The grep exited 0 and printed ten names. `ls tools/bench/tests/*.mjs` exited 0 and printed twelve names. `bin/ocelli.sh` lines 154 to 165 name all twelve.

## Smells

None.

## Nitpicks

None.

## Verified clean

`git write-tree` at the start, and again after the spacing line was restored, returned `d328fb7aa8ad9cc3424cfc140e6d6e5ecc0418c2`. `git diff -- crates/ocelli-volume/src/lib.rs` is empty. The sprint gate, the oracle and the bench measurement were not run. `node tools/bench/run.mjs --list` exited 0. It prints a blocking story and a backlog status. It does not print runner or measured state.

**Pass 8 D1.** `tools/bench/src/record.mjs` and `tools/bench/tests/record_test.mjs` say not every registry row is a duration. They do not say seven of eleven. `INCREASE_MEANS` has four units. `subjects.json` has twelve rows and those same four units: `ms` on eight, `fraction_of_one_vcpu` on two, `changed_frames_per_second` on one, and `pixels_per_second` on one. A node read of the file printed that. Not refiled.

**Pass 8 D2.** The `bench` arm says `--help` and `--list` do not load a runner, that every other invocation stops when `tools/bench/node_modules` is absent, and that `wasm_cold_start.mjs` imports playwright inside `run` while the decode runners and `tier_startup_microbenchmark.mjs` spawn a process and do not. `run.mjs` returns from `--help` before `loadRegistry`, and from `--list` before `runSubjects`. The import at `wasm_cold_start.mjs:237` is inside `run`. The other five runners import `spawn` and do not import playwright. The cold-start header says this file is the cold-start half. It does not say it is the one subject with a real subject today, and it does not say cold start has never been measured. Not refiled.

**Pass 8 D3.** The `render.first_frame` note says the row named F-038 until F-038 landed, that F-038 presents no frame and builds no ring, that the row now names F-101, and that a pending, archived or superseded story is refused a runner while a done story must have one. `subject_story` is F-101. The backlog row for F-038 is `done`. `RenderGraph::tick` renders into a caller-supplied texture view, and `ocelli-render` builds no event ring. `check_runners` refuses a runner for those three statuses and requires one when the story is done. Not refiled. `--list` printed `F-101 (pending)`. The note is not required to state that status.

**Pass 8 D4.** `CHANGELOG.md` says a recorded number for a pending, archived or superseded story is refused by the bench gate. `check_baseline` refuses when the story is not null and the status is not measurable. The past clause that most subjects had nothing to measure when the harness landed was not refiled. The `bench.registry` catalogue row and `docs/runbooks/guard-verification.md` say a duplicate subject id, a `subject_story` that is neither null nor a string, and a named story absent from the allocation or the backlog status table. `parseRegistry` throws on the first two. `resolveSubjects` throws on the third. The registry header says the module throws when a row cannot be labelled and sets `subjectExists` true for a null story and for an in-progress or done story. That matches `MEASURABLE_STATUS`. The catalogue row for `tools/bench/run.mjs`, and the matching runbook row, say a runner is refused when the story is not in progress or done. `runSubjects` throws that text when a runner exists and `subjectExists` is false. `run_test.mjs` expects the pending wording. Not refiled.

**Other search hits, not filed.** The changelog and the F-006 close use past tense for subjects that had nothing to measure, including nine of eleven when F-006 landed. The baseline `why` and the cold-start section say cold start had never been measured, which is the record of the first figure. `state.mjs` documents `NO_SUBJECT` as a story that has not landed, and `registry_test.mjs` uses the same phrase for a mutation, and both then state that measurable means in-progress or done. `bench_check.py` limits "nothing to measure yet" to pending, archived and superseded. `feature-availability.md` lists work that has not landed as something that contract does not build. `state_test.mjs` says the gate refuses a runner and a baseline for a story that is not in progress or done, which is the non-null branch of `check_runners` and `check_baseline`.

**Oracle frame.** `RenderGraph::tick` submits once, at `graph.rs:811`, into `slot.target`. A search for `create_surface` and `wgpu::Surface` under `crates/ocelli-render` prints nothing. The graph header, `docs/lld/render-graph.md`, `docs/sprints/CURRENT_SPRINT.md` and the F-038 close say no surface is acquired and that decision D7 stays open because the oracle has no Ocelli frame to judge. They do not name F-039, F-040 or F-X021 as the story that closes D7. `docs/lld/comparator.md` names F-X021 as activation of the candidate record.

**Brick offset.** The F-036 close says voxel 200, 130, 599 is in brick 3, 2, 9, and that brick's origin, voxel 192, 128, 576, is at byte offset 302121344. `((576 * 512 + 128) * 512 + 192) * 2` is that offset. The eight-bit extent 116 by 44 by 2 is 10208 bytes. The close attaches 302121344 to the origin, not to the voxel. `bricks.rs` asserts the same numbers.

**Recovery.** `GpuContext::recover` calls `adapter.open()` and assigns `*self = rebuilt`, which drops the old device handle. `ResolvedAdapter::open` passes `self.resolution.caps` into `GpuContext::new` and does not resolve the tier again. `install_recovered` sets `self.tier` from `recovered.caps.tier`. The comment says recovery does not resolve a tier.

**Gpu gate.** `bin/ocelli.sh` runs `cargo test -p ocelli-render -- --ignored --test-threads=1`. The comment writes no count of ignored tests.

**Budgeted values.** Production `impl Budgeted` is `DecodedFrame`, `Brick` and `Volume`. `GpuTexture` is a test double. A search for `no texture type exists` prints nothing.

**Cache kinds.** Kinds are 1 evicted, 2 displaced, 3 refused, 4 pressure and 5 budget changed. `TierCounters::record` walks `admission.evicted` and does not read `displaced`. The F-034 close says a displacement is ignored.

**F-033 spacing.** Inter-slice spacing is the median of `position_vector().dot(slice_normal())`. `PixelSpacing::new` takes row spacing then column spacing. The volume stores `column_mm`, `row_mm`, then the median. Direction column 1 is the column cosine, and it is scaled by `spacing[1]`, which is `row_mm`, which is `PixelSpacing[0]`. The fixture `[0.5, 1.0]` is asserted as `[1.0, 0.5, 2.0]`, and the column step is `(0.0, 0.5, 0.0)`. One mutation replaced `position_vector().dot(normal)` with `position_vector().z`. `bin/ocelli.sh test ocelli-volume --test assembly an_oblique_series_sorts_by_projection_and_not_by_ipp_z -- --exact` failed at `assembly.rs:352` because `volume.is_ok()` was false. The test command's status was 101. The line was restored.

**Tiers.** The five AS_BUILT entries name A, B and C. F-032 and F-034 are n/a on all three. F-033 is n/a, n/a and full. F-036 is full on all three for host arithmetic, with the 128 edge inside the tier B 256 floor and no upload. F-038 is full on A, full by construction and never exercised on B, and unavailable on C. `RenderGraph::new` returns `TierCUnavailable` when the tier is `Cpu`.

**D-25.** `docs/hld/DEVIATIONS.md` has one `| D-25 |` row. `SlicePresence` stores `words: Vec<u64>`. A search for `bitvec` in `Cargo.toml` files prints nothing.

**LLD headers.** Every `docs/lld/*.md` F-ID header was compared with its row in `docs/lld/README.md`, joining the wrapped continuation in `oracle.md`. Twenty files, twenty rows, zero mismatches.

**Builders.** A search for `no volume builder` and `future volume builder` prints nothing. `Volume::assemble` is in `crates/ocelli-volume/src/lib.rs`.

**F-096.** `git grep -n F-096 -- bin ci crates scripts tools .githooks` prints six historical lines, in `bin/ocelli.sh`, `crates/ocelli-wasm/Cargo.toml`, `scripts/bench_check.py`, `tools/bench/src/registry.mjs`, and two blocks of `tools/bench/subjects.json`.

**P0 notes.** `allocation.json` has the `P0 KILL CRITERION` note on F-027 and on F-028 and on no other story.

**Status column.** Not filed. The backlog rows for F-032, F-033, F-034, F-036 and F-038 are `done`.

**Not claimed done.** The sprint still says the M2 release, deviation D-22, F-X011, ICC, multi-component JPEG-LS and the unscheduled LINEAR range were carried in and not taken.

**Boundary.** A search for `wasm_bindgen` and `unsafe` over `crates/ocelli-cache`, `crates/ocelli-volume` and `crates/ocelli-render/src/graph.rs` prints nothing. The graph does not reference `ImageCache`, `BrickCache` or `VolumeCache`.
