# S12 whole-sprint review, pass 11

**Reviewed**: staged tree `3e6b14d689252212f1efd7f7a1da45265dcf7d2e` on `sprint/s12`. HEAD is `0ba66c7e5c7b78e6930d2dd1e81c373e40b814ae`. The sprint diff is `git diff 4976c929b8a7c0c081f6aeb2216dd5414571377c`. Untracked `session-manager-plugin.pkg` was not staged and is not part of this review.
**Result**: 4 defects, 0 smells, 0 nitpicks

## Defects
### D1, `bench -h` does not stop when the install is absent
**Where**: `bin/ocelli.sh:590`
**What**: The bench comment says `--help` and `--list` do not load a runner, and that every other invocation stops when `tools/bench/node_modules` is absent, before a runner starts.
**Why it is wrong**: `bench -h` is another invocation. The case arm exempts it with `--help` and `--list`, and `parseArgs` in `tools/bench/run.mjs` treats `-h` as help and returns before `runSubjects`. It does not stop, and it does not load a runner. The comment's "every other" is therefore false.
**Evidence**: The comment at `bin/ocelli.sh:590` names only `` `--help` `` and `` `--list` ``. The arm below it is `*" --help "*|*" -h "*|*" --list "*)`. A stand-in of that arm prints `exempt` for `-h` and `stops` for `--compare`:

```text
$ set -- -h; case " $* " in *" --help "*|*" -h "*|*" --list "*) echo exempt;; *) echo stops;; esac
exempt
$ set -- --compare; case " $* " in *" --help "*|*" -h "*|*" --list "*) echo exempt;; *) echo stops;; esac
stops
```

### D2, Three tiers are not three production Lrus
**Where**: `docs/lld/cache.md:140` and `crates/ocelli-cache/src/tier.rs:9`
**What**: Both say the three tiers hold three different value types, and that three tiers is three `Lru` values.
**Why it is wrong**: The production constructors are not one Lru per tier. `ImageCache` and `VolumeCache` are both `CacheTier::Decoded`. `BrickCache` is `CacheTier::Gpu`. No `src` file constructs an Encoded Lru. `cache.md` then says `Volume` keeps a second decoded-tier Lru, which contradicts the sentence above it. Encoded has no production value type. Decoded has two.
**Evidence**: `Lru::new` outside tests is only these three sites.

```text
crates/ocelli-cache/src/image.rs:228    Lru::new(CacheTier::Decoded, budget)
crates/ocelli-volume/src/lib.rs:626     Lru::new(CacheTier::Decoded, budget)
crates/ocelli-cache/src/brick.rs:363    Lru::new(CacheTier::Gpu, budget)
```

`CacheTier::Encoded` in `src` is the wire discriminant in `telemetry.rs` and test-only `Lru::new` calls inside `lru.rs`.

### D3, The F-032 test census does not match the crate
**Where**: `docs/sprints/AS_BUILT.md:3319`
**What**: The entry says `image_cache.rs` has 11 tests, of which 9 are unit or fixture tests and 1 is a property test, plus layout assertions, and that the focused suite is 38 tests in the crate.
**Why it is wrong**: The file has 11 `#[test]` functions. Ten are ordinary tests and one is the proptest `events_account_for_the_bytes_the_cache_stopped_holding`. Nine plus one is 10, not 11. Two of the ordinary tests are layout checks, `ring_event_is_48_bytes_at_section_17_3_offsets` and `raw_payload_starts_at_byte_16`, so they are not a single unnamed remainder inside the nine. The crate now has 72 `#[test]` functions, not 38. Thirty-eight is the count before `telemetry.rs` (12) and `bricks.rs` (22): 20 in `lru.rs`, 11 in `image_cache.rs`, 4 in `budget.rs`, 2 in `invariants.rs` and 1 in `lib.rs`.
**Evidence**:

```text
$ rg -c '#\[test\]' crates/ocelli-cache --glob '*.rs'
crates/ocelli-cache/src/lru.rs:20
crates/ocelli-cache/src/lib.rs:1
crates/ocelli-cache/tests/bricks.rs:22
crates/ocelli-cache/tests/invariants.rs:2
crates/ocelli-cache/tests/image_cache.rs:11
crates/ocelli-cache/tests/budget.rs:4
crates/ocelli-cache/tests/telemetry.rs:12
```

### D4, Fourteen is not the pass, blend and tier remainder
**Where**: `docs/lld/render-graph.md:21`
**What**: Only the composite stack pipeline has a shader body, and the other fourteen pass, blend and tier combinations are keys that report unavailable rather than aliasing composite.
**Why it is wrong**: A `PipelineKey` is pass, blend and tier. `PassKind` has 3 variants, `BlendMode` has 5 and `Tier` has 3, so the product is 45. `RenderGraph::pipeline_compiled` is true for `Stack` and `Composite` at every tier, which is 3 keys, so 42 keys have no body. One graph compiles a single pipeline for its own tier. Lookup of composite stack at another tier misses as well (`pipeline_lookup_never_returns_another_tiers_pipeline`), so the keys that do not alias composite are not fourteen. Fourteen is only 3 times 5 minus 1, which drops tier out of the product the sentence names.
**Evidence**: `BlendMode` at `crates/ocelli-render/src/graph.rs:78` is `Composite`, `Mip`, `MinIp`, `Average`, `LabelmapEdgeProjection`. `PassKind` at line 93 is `Stack`, `VolumeRaycast`, `SegOverlay`. `Tier` at `crates/ocelli-render/src/caps.rs:24` is `A`, `B`, `Cpu`. `pipeline_compiled` at `graph.rs:330` compares pass and blend and ignores tier. `compile` pushes one key.

## Smells
None.

## Nitpicks
None.

## Verified clean
`git write-tree` is `3e6b14d689252212f1efd7f7a1da45265dcf7d2e`. After the spacing mutation below, `git diff -- crates/ocelli-volume/src/lib.rs` was empty and the index hash was unchanged.

The six pass-10 rewrites match the code.

- `tools/bench/tests/paths_test.mjs` and `docs/lld/benchmarks.md` say a basename that does not replace dots makes `existsSync` false for every subject whose id contains a dot. All 12 ids in `tools/bench/subjects.json` contain a dot. Six runners live under `tools/bench/src/runners`. The assertion names `wasm.cold_start` only. Neither site says there is one measurable subject.
- `docs/lld/benchmarks.md` says a named story is measurable only at in-progress or done, and that a row naming no story is measurable too. `MEASURABLE_STATUS` in `tools/bench/src/registry.mjs` and `scripts/bench_check.py` is that five-status table. `resolveSubjects` sets `subjectExists` true for a null story. `check_baseline` refuses an entry only when the story is not null and the status is not measurable. `wasm.cold_start` has `subject_story` null and a baseline entry. `tier.startup_microbenchmark` is F-004 done, has `tier_startup_microbenchmark.mjs`, and has no baseline entry. `node tools/bench/run.mjs --list` prints each blocking story and its backlog status, including `subject exists today` for the null row.
- `docs/lld/render-graph.md` no longer says five blend modes beyond composite. `BlendMode` has five variants in total. Only composite has a shader body. The blend modes other than composite wait for F-052 and F-054. The fourteen in the same paragraph is D4, not a leftover "five beyond".
- `docs/lld/comparator.md` and the `MeasuredReformat` comment in `tools/oracle/src/mutations.rs` say the mutation lands on the first volume reformat whose outcome is `Pass`, and that it is not pinned to a declared divergence. On this corpus that id is `volume__real__mr_eay131__AXIAL` because records come from a `BTreeSet` and `real` sorts before `synthetic`. `volume-truth.json` has a non-null `referenceDivergence` on both `volume__synthetic__ct_series_nonuniform` and `volume__real__mr_eay131`. The `attribution.rs` comment says the same thing and does not say there is one divergent subject.
- `docs/lld/oracle.md` lists the reference `.mjs` modules as paths, manifest, params, voi, geometry, volume, sidecar, output, pins, server, unsupported, faults and metadata-sources. `ls tools/oracle/src/*.mjs` is those 13 files and no others. The tests row says twelve `node:test` suites plus the fault runner. There are twelve `*_test.mjs` files and `tests/faults.mjs`.
- `metadata-truth.json` has 14 keys under `entries`. The oracle sentence no longer says eleven. `check_sidecars.py` iterates `document["entries"]`, and every one of those 14 rows contributes at least one `/attributes/` field that the checker then compares.

Also re-executed and left standing:

- `tools/bench/src/record.mjs` says not every registry row is a duration. `INCREASE_MEANS` has four units. The registry has `ms` on eight rows, `fraction_of_one_vcpu` on two, `changed_frames_per_second` on one and `pixels_per_second` on one.
- `tools/bench/subjects.json` names F-101 on `render.first_frame` and says a pending story is refused a runner. F-101 is pending. F-038 is done and the note does not say a runner was refused because F-038 is done.
- CHANGELOG says a recorded number for a pending, archived or superseded story is refused. That is `check_baseline`.
- `bench.registry` in `scripts/guards/catalogue.py` refuses a duplicate subject id, a `subject_story` that is neither null nor a string, and a named story absent from the allocation or the backlog status table. The `registry.mjs` header permits a runner during in-progress, requires one when done, refuses one for every other status, and sets `subjectExists` true for a null story and for an in-progress or done story. `run.mjs` refuses a runner only when `subjectExists` is false, so a null story is not refused. The catalogue `bench.runner` sentence and the generated runbook row match that non-null branch.
- `RenderGraph::tick` submits once into the caller-supplied view. `create_surface` and `wgpu::Surface` do not appear under `crates/ocelli-render`. D7 stays open because the oracle has no Ocelli frame. The rewritten sentences do not name F-040, F-039 or F-X021 as the story that closes it.
- Brick arithmetic matches the tests and the F-036 entry. Voxel (200, 130, 599) is brick (3, 2, 9). That brick's origin (192, 128, 576) is byte offset 302121344 at two bytes per voxel. The voxel itself is 314182032, and the entry attaches 302121344 to the origin. The eight-bit 500 by 300 by 130 last brick is extent (116, 44, 2) and 10208 bytes. Edge is 128 for eight-bit and 64 for sixteen-bit and float. 600 is not a multiple of 64. The 512 by 512 by 600 origin offset fits in `u32`. The wrap pin is dims 65536 by 65536 by 2 and byte offset 4294967296.
- `GpuContext::recover` opens the adapter before it replaces the context, copies the graph registration, bumps `cache_generation`, assigns the new context, and returns `Recovered`. `ResolvedAdapter::open` passes `self.resolution.caps` and does not resolve the tier again. `install_recovered` sets `self.tier` from the recovered caps and sets every per-viewport slot to `None`.
- The gpu arm is `cargo test -p ocelli-render -- --ignored --test-threads=1` and writes no count. `docs/lld/gpu-ownership.md` says the gate is every ignored test in the crate.
- Production `Budgeted` implementations are `DecodedFrame`, `Brick` and `Volume`. `Block`, `Shifting`, `GpuTexture` and `DecodedSeries` are test doubles. An encoded byte run is not a production value. `probe.rs` creates a `wgpu::Texture` and `GpuSlot` stores a `wgpu::TextureView`.
- Cache kinds are 1 evicted, 2 displaced, 3 refused, 4 pressure and 5 budget changed. `BrickCache::insert` returns `Admission` and builds no `CacheEvent`. `TierCounters::record` walks `admission.evicted` and does not read `displaced`. `ImageCache` and `VolumeCache` are separate Lrus on `CacheTier::Decoded`. `BrickCache` is `CacheTier::Gpu`.
- F-033 spacing is the median of `position_vector().dot(slice_normal())`. The crate does not read `SpacingBetweenSlices` or `SliceThickness`. `PixelSpacing::new` takes row spacing then column spacing. The volume stores column spacing, row spacing, then the median. Fixture `[0.5, 1.0]` is asserted as `[1.0, 0.5, 2.0]` with column step `(0.0, 0.5, 0.0)`. Replacing the dot with `.z` made `an_oblique_series_sorts_by_projection_and_not_by_ipp_z` fail at `volume.is_ok()`. The line was restored.
- Tier rows in the five entries match the code. F-032 and F-034 are n/a on A, B and C. F-033 is n/a, n/a and full, with no tier branch in `ocelli-volume`. F-036 is full on all three, host arithmetic, a 128 edge inside the tier B 256 floor, and no upload. F-038 is full on A, full by construction and never exercised on B, and unavailable on C when the tier is Cpu. An uncompiled blend returns `Unavailable` and does not substitute another numeric result.
- D-25 is one row. `Volume::present` is `SlicePresence` over `words: Vec<u64>`. No `Cargo.toml` depends on `bitvec`.
- LLD F-ID headers match `docs/lld/README.md`, including the files whose dates moved to 2026-10-17. A date-only change does not move the README row.
- `git grep -n F-096 -- bin ci crates scripts tools .githooks` is the six historical sites named in `docs/lld/benchmarks.md`. P0 kill-criterion notes in `docs/sprints/allocation.json` are F-027 and F-028. The F-006 note is the older "Numbers feed the P0 kill criteria" line, not a third kill-criterion story.
- `state.mjs` `NO_SUBJECT` and the non-null branch in `state_test.mjs` do not treat a null story as refused. `docs/lld/feature-availability.md` "work that has not landed" is a different contract.
- The gpu gate comment, the oracle frame wording, and the carried M2, D-22, F-X011, ICC, multi-component JPEG-LS and unscheduled LINEAR notes were not filed. The hand-typed status column in `CURRENT_SPRINT.md` was not filed.
