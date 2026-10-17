# S12 whole-sprint review, pass 10

**Reviewed**: staged tree `2f591556f439d717ac63f8add36d1e2c4af63670` on `sprint/s12`. HEAD is `0ba66c7e5c7b78e6930d2dd1e81c373e40b814ae`. The sprint diff is `git diff 4976c929b8a7c0c081f6aeb2216dd5414571377c`. Untracked `session-manager-plugin.pkg` was not staged and is not part of this review.
**Result**: 6 defects, 0 smells, 0 nitpicks

## Defects

### D1, a test still says the basename rule hides the one measurable subject

**Where**: `tools/bench/tests/paths_test.mjs:81-90`

**What**: The comment says a basename rule that drifted from the file on disk turns the one measurable subject in this tree into `unavailable / no_runner`. The assertion message says `run.mjs` would report the only measurable subject as having no runner.

**Why it is wrong**: Six subjects are measurable and each has a runner. Every subject id contains a dot, so a basename that keeps the dots misses every one of those runners, not one.

**Evidence**: `node tools/bench/run.mjs --list` exited 0. It prints `subject exists today` for `wasm.cold_start` and `done` for `decode.frame` (F-023), `decode.transfer_syntax.htj2k` (F-027), `decode.transfer_syntax.jpeg2000` (F-026), `decode.transfer_syntax.jpegls` (F-028) and `tier.startup_microbenchmark` (F-004). `ls tools/bench/src/runners` has the matching six files. The other six rows are `pending` and have no runner. `resolveSubjects` sets `subjectExists` true for a null story and for an in-progress or done story.

The opening comments in this file and in `docs/lld/benchmarks.md` were rewritten and do not make this claim. They are not this defect.

### D2, measurable is said to be only in-progress or done

**Where**: `docs/lld/benchmarks.md:352-353`

**What**: The bullet says a subject is measurable only at `in-progress` or `done`.

**Why it is wrong**: `wasm.cold_start` has `subject_story` null, `--list` prints `subject exists today`, and `wasm_cold_start.mjs` exists. `tools/bench/src/registry.mjs` sets `subjectExists` true for a null story and for an in-progress or done story.

**Evidence**: The same file, at the refusal list and again under the four states, says a row that names no story must have a runner. This bullet does not. `MEASURABLE_STATUS` has no null row because a null story never reaches it.

### D3, five blend modes beyond composite

**Where**: `docs/lld/render-graph.md:52-53`

**What**: The sentence says the five blend modes beyond composite wait for F-052 and F-054.

**Why it is wrong**: `BlendMode` has five variants in total. Four are not composite.

**Evidence**: `crates/ocelli-render/src/graph.rs` defines `Composite`, `Mip`, `MinIp`, `Average` and `LabelmapEdgeProjection`. `docs/hld/B-parity-surface.md` counts five blend modes and lists composite among them. The code comment says those five are what the parity surface counts, and only composite has a shader body.

### D4, the reformat mutation is said to land on the one divergent subject

**Where**: `docs/lld/comparator.md:526-528`

**What**: The sentence says `plus-three-on-one-pixel-of-a-reformat` lands on the one subject carrying a declared divergence only because `real` sorts before `synthetic`.

**Why it is wrong**: Two subjects carry a non-null `referenceDivergence`. The mutation target is the first passing volume reformat, and the runner comment says that target is not pinned to a declared divergence.

**Evidence**: `tools/oracle/volume-truth.json` has a non-null `referenceDivergence` on `volume__synthetic__ct_series_nonuniform` and on `volume__real__mr_eay131`. `Target::MeasuredReformat` in `tools/oracle/src/mutations.rs` selects the first `VolumeReformat` whose outcome is `Pass`. The comment there says today's resolution is `volume__real__mr_eay131__AXIAL` because `real` sorts before `synthetic`, and that nothing should assume the target is a declared divergence.

### D5, the oracle source directory is said to hold twelve modules

**Where**: `docs/lld/oracle.md:60`

**What**: The layout row says `tools/oracle/src/` holds twelve `.mjs` modules and lists twelve names.

**Why it is wrong**: The directory holds thirteen `.mjs` files. `metadata-sources.mjs` is tracked and is not in the list.

**Evidence**: `ls tools/oracle/src/*.mjs` printed thirteen names. The sprint base tree lists the same thirteen. The named list is paths, manifest, params, voi, geometry, volume, sidecar, output, pins, server, unsupported and faults.

### D6, metadata truth is said to name eleven rows

**Where**: `docs/lld/oracle.md:688-689`

**What**: The sentence says `metadata-truth.json` covers eleven named synthetic and syntax rows.

**Why it is wrong**: `entries` has fourteen keys, and the sidecar checker walks every entry.

**Evidence**: A Python read of `tools/oracle/metadata-truth.json` printed fourteen entry keys: `synthetic/ct_unsigned_16.dcm`, `synthetic/ct_signed_12in16_left.dcm`, `synthetic/ct_signed_12in16_right.dcm`, `synthetic/ct_sigmoid_width_half.dcm`, `synthetic/cr_monochrome1.dcm`, `synthetic/sc_rgb_interleaved.dcm`, `synthetic/sc_rgb_planar.dcm`, `synthetic/mr_nonsquare_spacing.dcm`, `synthetic/ct_multiframe_perframe.dcm`, `syntax/jpeg_baseline_rgb8.dcm`, `syntax/reference_mono12.dcm`, `synthetic/sc_palette_color.dcm`, `synthetic/sc_palette_color_16.dcm` and `syntax/jpegls_lossless_rgb8.dcm`. `tools/oracle/check_sidecars.py` iterates `document["entries"]`.

## Smells

None.

## Nitpicks

None.

## Verified clean

`git write-tree` at the start, and again after the spacing line was restored, returned `2f591556f439d717ac63f8add36d1e2c4af63670`. `git diff -- crates/ocelli-volume/src/lib.rs` is empty. The sprint gate, the oracle and the bench measurement were not run.

**Pass 9 D1 rewrite.** `docs/lld/benchmarks.md` and the opening comment of `tools/bench/tests/paths_test.mjs` say a `runnerBasename` that does not replace dots makes `existsSync` false for every subject whose id contains a dot, and that a subject which exists and whose runner is then missed reports `unavailable / no_runner`. Every id in `subjects.json` contains a dot. `runnerBasename` replaces every dot. The six runner files match six of those ids. `subjectState` returns `unavailable` and `no_runner` when the subject exists and `hasRunner` is false. Not refiled. The later test comment is D1 above.

**Pass 9 D2 rewrite.** The bench test-file count is not written as a current number. The paragraph says the count reached five and then six in S03, that `[a-z_]` cannot match a digit and therefore drops `decode_htj2k_test.mjs` and `decode_jpeg2000_test.mjs`, and that the digit-allowing grep and `ls tools/bench/tests/*.mjs` are the two lists to compare. The grep exited 0 and printed the same twelve names as `ls`. The `[a-z_]` grep exited 0 and printed ten names, omitting those two files. Not refiled. The F-006 correction table names those same two commands. "It named six when `828037e` added `paths_test.mjs`" is past. The F-006 entry that still says the arm names five files is the text that table corrects. The entry was not filed.

**Bench rule.** `check_runners` refuses a runner for pending, archived and superseded, requires one for a null story and for a done story, and allows an in-progress story to have or lack one. `check_baseline` refuses a baseline when the story is not null and the status is not measurable, so a null story may have one. `tools/bench/tests/run_test.mjs` expects the pending refusal text. `--list` does not print runner state.

**Oracle frame.** `RenderGraph::tick` submits once, at `graph.rs:811`, into `slot.target`. A search for `create_surface` and `wgpu::Surface` under `crates/ocelli-render` prints nothing. The graph header, `docs/lld/render-graph.md`, `docs/sprints/CURRENT_SPRINT.md` and the F-038 close say no surface is acquired and that decision D7 stays open because the oracle has no Ocelli frame to judge. They do not name F-039, F-040 or F-X021 as the story that closes D7.

**Brick offset.** The F-036 close says voxel 200, 130, 599 is in brick 3, 2, 9, and that brick's origin, voxel 192, 128, 576, is at byte offset 302121344. `((576 * 512 + 128) * 512 + 192) * 2` is 302121344. The voxel's own offset is 314182032 and is not the number in the close. The eight-bit extent 116 by 44 by 2 is 10208 bytes.

**Recovery.** `GpuContext::recover` calls `adapter.open()` and assigns `*self = rebuilt`, which drops the old device handle. `ResolvedAdapter::open` passes `self.resolution.caps` into `GpuContext::new` and does not resolve the tier again. `install_recovered` sets `self.tier` from `recovered.caps.tier`. The comment says recovery does not resolve a tier.

**Gpu gate.** `bin/ocelli.sh` runs `cargo test -p ocelli-render -- --ignored --test-threads=1`. The comment writes no count of ignored tests.

**Budgeted values.** Production `impl Budgeted` is `DecodedFrame`, `Brick` and `Volume`. `GpuTexture`, `DecodedSeries`, `Block` and `Shifting` are test doubles.

**Cache kinds.** Kinds are 1 evicted, 2 displaced, 3 refused, 4 pressure and 5 budget changed. `TierCounters::record` walks `admission.evicted` and does not read `displaced`.

**F-033 spacing.** Inter-slice spacing is the median of `position_vector().dot(slice_normal())`. `PixelSpacing::new` takes row spacing then column spacing. The volume stores `column_mm`, `row_mm`, then the median. Direction column 1 is the column cosine, and it is scaled by `spacing[1]`, which is `row_mm`, which is `PixelSpacing[0]`. The fixture `[0.5, 1.0]` is asserted as `[1.0, 0.5, 2.0]`, and the column step is `(0.0, 0.5, 0.0)`. One mutation replaced `position_vector().dot(normal)` with `position_vector().z`. `bin/ocelli.sh test ocelli-volume --test assembly an_oblique_series_sorts_by_projection_and_not_by_ipp_z -- --exact` failed at `assembly.rs:352` because `volume.is_ok()` was false. The test command's status was 101. The line was restored.

**Tiers.** The five AS_BUILT entries name A, B and C. F-032 and F-034 are n/a on all three. F-033 is n/a, n/a and full. F-036 is full on all three for host arithmetic, with the 128 edge inside the tier B 256 floor and no upload. F-038 is full on A, full by construction and never exercised on B, and unavailable on C. `RenderGraph::new` returns `TierCUnavailable` when the tier is `Cpu`.

**D-25.** `docs/hld/DEVIATIONS.md` has one `| D-25 |` row. `SlicePresence` stores `words: Vec<u64>`. A search for `bitvec` in `Cargo.toml` files prints nothing.

**LLD headers.** Every `docs/lld/*.md` F-ID header was compared with its row in `docs/lld/README.md`, joining the wrapped continuation in `oracle.md`. The comparison printed zero mismatches.

**Builders.** A search for `no volume builder` and `future volume builder` prints nothing. `Volume::assemble` is in `crates/ocelli-volume/src/lib.rs`.

**F-096.** `git grep -n F-096 -- bin ci crates scripts tools .githooks` prints six historical lines, in `bin/ocelli.sh`, `crates/ocelli-wasm/Cargo.toml`, `scripts/bench_check.py`, `tools/bench/src/registry.mjs`, and two blocks of `tools/bench/subjects.json`.

**P0 notes.** `allocation.json` has the `P0 KILL CRITERION` note on F-027 and on F-028 and on no other story.

**Status column.** Not filed. The backlog rows for F-032, F-033, F-034, F-036 and F-038 are `done`.

**Not claimed done.** The sprint still says the M2 release, deviation D-22, F-X011, ICC, multi-component JPEG-LS and the unscheduled LINEAR range were carried in and not taken. F-X011 is pending. D-22 is one row. `ci/wasm-size-budget.json` still records 16388 bytes. ICC is still named unimplemented in `docs/lld/pixel-pipeline.md`. The codecs LLD still says multi-component JPEG-LS decoding is owed.

**Fourteen keys.** Three pass kinds times five blend modes, minus the composite stack, is fourteen at the graph's one tier. `compile` stores that one key. Not filed. D3 is the count beyond composite.

**Boundary.** A search for `wasm_bindgen` and `unsafe` over `crates/ocelli-cache`, `crates/ocelli-volume` and `crates/ocelli-render/src/graph.rs` prints nothing.

**Other search hits, not filed.** `state.mjs` documents `NO_SUBJECT` as a story that has not landed. The reason is also used for archived and superseded, which the file header and `state_test.mjs` name. `feature-availability.md` lists work that has not landed as something that contract does not build. `state_test.mjs` says the gate refuses a runner and a baseline for a story that is not in progress or done, which is the non-null branch. The eleven volume-fault rows in `docs/lld/oracle.md` match the fault table from `volume-member-unparseable` through `wrong-reformat-canvas-size`. `CLAUDE.md`'s eleven shared crates is `ls crates` minus `ocelli-native` and `ocelli-wasm`, and that difference is eleven.
