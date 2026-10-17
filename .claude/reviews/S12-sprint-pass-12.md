# S12 whole-sprint review, pass 12

**Reviewed**: staged tree `cd465340b92e4a9cd0ec37e48ece5fb3f4de949d` on `sprint/s12`. HEAD is `0ba66c7e5c7b78e6930d2dd1e81c373e40b814ae`. The sprint diff is `git diff 4976c929b8a7c0c081f6aeb2216dd5414571377c`. Untracked `session-manager-plugin.pkg` was not staged and is not part of this review.
**Result**: 5 defects, 0 smells, 0 nitpicks

## Defects

### D1, Labelmap edge projection does not wait for F-052 and F-054

**Where**: `docs/lld/render-graph.md:52`
**What**: The closing sentence says the blend modes other than composite wait for F-052 and F-054.
**Why it is wrong**: `BlendMode` has five variants. Only `Composite` has a shader body. F-054 is the thick slab for composite, MIP, MinIP and average, and it depends on F-052. `LabelmapEdgeProjection` is F-067, which depends on F-064. The enum comment on `Mip` names F-052 and F-054. The comment on `LabelmapEdgeProjection` does not.
**Evidence**: `docs/sprints/BACKLOG.md:201` is F-054, "Thick slab with composite, MIP, MinIP and average blend modes", status pending, depends on F-052. `docs/sprints/BACKLOG.md:224` is F-067, "Segmentation intersection and labelmap edge-projection blend", status pending, depends on F-064. `crates/ocelli-render/src/graph.rs:78` through `graph.rs:88` declares `Composite`, `Mip`, `MinIp`, `Average` and `LabelmapEdgeProjection`. `COMPILED_BLEND` at `graph.rs:323` is `BlendMode::Composite`.

### D2, Encoded bytes do not still arrive with a story

**Where**: `docs/lld/cache.md:148`
**What**: The tier paragraph says encoded bytes and GPU textures still arrive with their own stories.
**Why it is wrong**: HLD section 8 says encoded bytes are transient and dropped as soon as a frame decodes. The only backlog row that names encoded bytes is F-031, and that story is done. It left `CacheTier::Encoded` as a discriminant. No production `Lru::new` uses it. F-035 is "Zero-copy handoff strategy from decode worker to renderer". F-040 and F-043 are the texture upload stories, so the GPU-texture half of the sentence has stories and the encoded-byte half does not.
**Evidence**: `docs/hld/06-memory-and-cache.md:20`. `rg -n encoded docs/sprints/BACKLOG.md` hits only the done F-031 title at line 157. `docs/sprints/BACKLOG.md:161` is F-035, pending. `docs/sprints/BACKLOG.md:166` is F-040, "Texture upload path: decode straight to write_texture", pending. `docs/sprints/BACKLOG.md:169` is F-043, pending, depends on F-040. Production `Lru::new` sites are `crates/ocelli-cache/src/image.rs:228` (`CacheTier::Decoded`), `crates/ocelli-volume/src/lib.rs:626` (`CacheTier::Decoded`) and `crates/ocelli-cache/src/brick.rs:363` (`CacheTier::Gpu`). `CacheTier::Encoded` appears in test `Lru::new` calls inside `lru.rs` and `telemetry.rs`.

### D3, The F-038 floor count does not match the files it names

**Where**: `docs/sprints/AS_BUILT.md:3335`
**What**: The tests line says `crates/ocelli-render/tests/graph.rs` and `graph_voi.rs` contain 5 unit and fixture rows in the floor and 7 device rows under the gpu gate.
**Why it is wrong**: Those two files contain 2 floor tests and 7 ignored device tests. The other 3 floor tests live in `crates/ocelli-render/src/graph.rs`, which the sentence does not name. One of the 7 ignored tests is `linear_and_linear_exact_separate_at_the_window_centre`, and its comment states LINEAR 0.50125 against LINEAR_EXACT 0.5.
**Evidence**: `tests/graph.rs` floor tests are `a_new_viewport_starts_fully_dirty_and_marks_coalesce` at line 19 and `pipeline_key_distinguishes_each_field` at line 36. Its ignored tests are at lines 223, 266, 298, 385 and 479. `tests/graph_voi.rs` ignores both tests at lines 192 and 213, including `linear_and_linear_exact_separate_at_the_window_centre`. `src/graph.rs` floor tests are `pipeline_lookup_never_returns_another_tiers_pipeline` at line 826, `removal_clears_the_slot_and_reregistration_reuses_it_fully_dirty` at line 857 and `registry_refuses_beyond_capacity_and_starts_fully_dirty` at line 881. None of those three carries `#[ignore]`.

### D4, Nine green cache findings are an eight-item list

**Where**: `docs/sprints/AS_BUILT.md:3286` and `docs/sprints/SPRINT_TRACKER.md:90`
**What**: The F-031 entry says nine tests exist because of what they found, then says nine left the suite green and names the list. The tracker says those became nine new tests.
**Why it is wrong**: The colon list is eight findings, and each one is one test. `Lru::is_empty` is `self.entries.is_empty()` at `crates/ocelli-cache/src/lru.rs:225`. The zero-byte test is the one that separates `used == 0` from `is_empty`.
**Evidence**: The eight tests are `the_evicted_vector_is_least_recently_used_first`, `remove_releases_the_bytes_admitted_and_not_the_bytes_reported_now`, `eviction_releases_the_bytes_admitted_and_not_the_bytes_reported_now`, `replacing_a_key_releases_the_bytes_it_was_admitted_at`, `a_refused_insert_does_not_change_the_eviction_order`, `a_tier_with_a_budget_admits_an_entry_of_no_bytes`, `replacing_a_live_key_at_a_full_budget_evicts_nothing` and `a_frame_is_rows_by_columns_by_samples_by_bytes_per_sample`. The samples fixture sets `samples_per_pixel: 3` in `crates/ocelli-cache/tests/budget.rs:141`.

### D5, A bench value containing `-h` skips the install refusal and still runs

**Where**: `bin/ocelli.sh:591`
**What**: The bench comment says `--help`, `-h` and `--list` do not load a runner, and every other invocation stops when `tools/bench/node_modules` is absent, before a runner starts.
**Why it is wrong**: The case arm is a substring match on `" $*"`. `parseArgs` sets help only for the flags `--help` and `-h`, and list only for the flag `--list`. After the shell shifts `bench` off, `bench --why "see -h now"` matches `*" -h "*` and `parseArgs` leaves both help and list false. `main` then calls `runSubjects`. The same split happens for a value containing ` --help ` or ` --list `. The three named flags themselves return before `runSubjects`.
**Evidence**:

```text
case " $* " in *" --help "*|*" -h "*|*" --list "*) EXEMPT ;; *) STOP ;; esac
--help                         EXEMPT
-h                             EXEMPT
--list                         EXEMPT
(empty)                        STOP
render.first_frame             STOP
--why "see -h now"             EXEMPT
--why "try --help please"      EXEMPT
--why "see --list"             EXEMPT

parseArgs(["--help"])                 help=true  list=false
parseArgs(["-h"])                     help=true  list=false
parseArgs(["--list"])                 help=false list=true
parseArgs(["--why","see -h now"])     help=false list=false why="see -h now"
parseArgs(["--why","try --help please"]) help=false list=false
parseArgs(["--why","see --list"])     help=false list=false
```

`tools/bench/run.mjs:244` returns on `options.help` before `loadRegistry`. `run.mjs:251` returns on `options.list` before `runSubjects`. `run.mjs:264` is the `runSubjects` call. `node tools/bench/run.mjs -h` and `node tools/bench/run.mjs --help` both exited 0. `node tools/bench/run.mjs --list` printed registry rows and did not start a runner. The `--why` form was not executed, because that path enters `runSubjects`.

## Smells

None.

## Nitpicks

None.

## Verified clean

`git write-tree` at the start and after the restored mutation was `cd465340b92e4a9cd0ec37e48ece5fb3f4de949d`. HEAD stayed `0ba66c7e5c7b78e6930d2dd1e81c373e40b814ae` on `sprint/s12`. `git diff -- crates/ocelli-volume/src/lib.rs` was empty after the restore. Untracked `session-manager-plugin.pkg` was left in place.

The bench comment names `--help`, `-h` and `--list`, and the case arm contains all three. `parseArgs` treats `--help` and `-h` as help. `wasm_cold_start.mjs:237` imports playwright inside `run`. No other file under `tools/bench/src/runners` imports it.

`docs/lld/cache.md` now heads the section "The caches and their tiers". Production caches are not one `Lru` per tier. `ImageCache` and `VolumeCache` are `CacheTier::Decoded`. `BrickCache` is `CacheTier::Gpu`. No production cache is `CacheTier::Encoded`. `DecodedFrame` and `Brick` are the two value types in `ocelli-cache`. `Brick::bytes` is `self.samples.len()` at `brick.rs:346`. `Volume` implements `Budgeted` in `ocelli-volume` and keeps its own decoded `Lru`. `crates/ocelli-cache/Cargo.toml` has no `wgpu` dependency. `GpuSlot.target` is a `wgpu::TextureView` at `graph.rs:246`, and `probe.rs:716` creates a `wgpu::Texture`. Those types live in `ocelli-render`.

The F-032 tests line names `events_account_for_the_bytes_the_cache_stopped_holding`, `ring_event_is_48_bytes_at_section_17_3_offsets` and `raw_payload_starts_at_byte_16`. All three exist in `crates/ocelli-cache/tests/image_cache.rs`. The property test sits inside `proptest!`. The line no longer states a crate-wide total.

`docs/lld/render-graph.md` says a lookup for any other pass, blend and tier combination reports unavailable rather than aliasing composite. `compile` returns one key, stack plus composite at the graph tier, at `graph.rs:424`. `pipeline` looks that key up by full equality at `graph.rs:457`. `pipeline_lookup_never_returns_another_tiers_pipeline` expects `None` for the same pass and blend at tier B. The doc comment on `pipeline_compiled` says the composite stack body applies at any tier and every other key is unavailable. The function returns true only for `PassKind::Stack` and `BlendMode::Composite`, and it ignores tier. `tick` calls `pipeline`, and `ctx.queue().submit` appears once at `graph.rs:811`. There is no `wgpu::Surface` under `crates/ocelli-render`. The LLD says F-039 acquires no surface, F-040 uploads no texture, and decision D7 stays open.

`docs/lld/benchmarks.md` says a named story is measurable only at in-progress or done, a row that names no story is measurable too, and pending, archived and superseded rows stay unavailable. A done story must have a runner, and so must a null story. In-progress may have or lack one. `scripts/bench_check.py` and `tools/bench/src/registry.mjs` carry the same five statuses, with pending, archived and superseded refused. `check_baseline` refuses a baseline only when the story is not null and the status is not measurable. `state.mjs` returns `NO_SUBJECT` only when `subjectExists` is false. A null `subject_story` sets `subjectExists` true at `registry.mjs:180`. `tools/bench/tests/paths_test.mjs` does not say there is one measurable subject. The bench gate arm names the twelve `*_test.mjs` files in `tools/bench/tests`. The historical sentence that said five while the arm named six was left as past.

`render.first_frame` names F-101. F-101 is pending and the row has no runner. The note does not say a runner was refused because F-038 is done. F-038 is done. Done rows `decode.frame` (F-023), the three transfer-syntax rows (F-027, F-026, F-028) and `tier.startup_microbenchmark` (F-004) each have a runner. The F-X003 rows are pending and have no runner. `wasm.cold_start` names no story and has a runner. `CHANGELOG.md` says a recorded number for a pending, archived or superseded story is refused by the bench gate. `INCREASE_MEANS` declares `ms`, `fraction_of_one_vcpu`, `changed_frames_per_second` and `pixels_per_second`. Those are the four units in `tools/bench/subjects.json`.

`docs/lld/oracle.md` lists the `.mjs` modules without a count, including `metadata-sources`. `tools/oracle/src` has those thirteen modules. The tests row says twelve `node:test` suites plus the fault runner. `tools/oracle/tests` has twelve `*_test.mjs` files plus `faults.mjs`. `check_sidecars.py` loads `metadata-truth.json` and iterates `document["entries"]` at line 113. `docs/lld/comparator.md` says twenty-nine catalogue entries. `Mutation {` entries in `tools/oracle/src/mutations.rs` are 29. `MeasuredReformat` is the first passing volume reformat and the comment says it is not pinned to a declared divergence. On this corpus the comment names `volume__real__mr_eay131__AXIAL` because a `BTreeSet` sorts `real` before `synthetic`. `volume-truth.json` has a non-null `referenceDivergence` on `volume__synthetic__ct_series_nonuniform` and `volume__real__mr_eay131`. The uniform synthetic subject and `volume__real__ct_cmb_mml` are null. Comparator prose does not say there is one divergent subject.

The F-036 close says brick origin voxel 192, 128, 576 is byte offset 302121344 on the 512 by 512 by 600 sixteen-bit fixture, and places voxel 200, 130, 599 in brick 3, 2, 9. The same layout gives voxel 200, 130, 599 byte offset 314182032, and that number is not in the close. The 500 by 300 by 130 eight-bit last brick is extent 116, 44, 2 and 10208 bytes. `GpuContext::recover` opens the retained adapter and assigns `*self = rebuilt` at `gpu.rs:317`. `ResolvedAdapter::open` uses `self.resolution.caps` and does not call `resolve`. The gpu gate comment says no count is written there, and the command is `cargo test -p ocelli-render -- --ignored --test-threads=1`.

Cache kinds are evicted 1, displaced 2, refused 3, pressure 4 and budget changed 5. `TierCounters::record` increments evicted and refused and ignores a displacement. F-033 spacing is the median of `position_vector().dot(slice_normal())` at `ocelli-volume/src/lib.rs:433`. Replacing that dot with `.z` made `an_oblique_series_sorts_by_projection_and_not_by_ipp_z` fail at `tests/assembly.rs:352` with `assertion failed: volume.is_ok()`. The line was restored by editing that expression back. The shear cosine at line 460 was left as `.dot(normal)`.

Tier rows are present for F-032, F-033, F-034, F-036 and F-038. D-25 is one row in `docs/hld/DEVIATIONS.md`. `SlicePresence` is `words: Vec<u64>` at `ocelli-volume/src/lib.rs:221`. F-033 names 14 tests in `assembly.rs`, F-034 names 12 in `telemetry.rs`, and F-036 names 22 in `bricks.rs` plus 2 in `brick_width.rs`. Those `#[test]` counts match. `CURRENT_SPRINT.md` status was not reviewed. Added lines in the sprint diff, excluding review and plan files, contain none of the leftover phrases "fourteen", "three `Lru`", "38 tests", "five beyond composite" or "the one subject".
