# F-119 review, pass 3

**Reviewed**: uncommitted worktree diff on `work/f-119-xai` at base `19c7ed361b19091a2fac50acf1cf57e9db30471d`. Blob hashes pinned at start and identical at end, after every mutation was reverted:

| Path | Blob |
|------|------|
| `crates/ocelli-volume/src/levels.rs` | `a86c39bfd20fd919ef1754992afd43796ebe19e2` |
| `crates/ocelli-volume/src/lib.rs` | `450443a6cbaed247039d809d818d0dc465630839` |
| `crates/ocelli-volume/src/ortho.rs` | `97d1fa94939b404834de191fc4d856284d3e6cd8` |
| `crates/ocelli-volume/tests/pyramid.rs` (untracked) | `d71658bec5c0843075dea75328e6438cc7840421` |
| `crates/ocelli-cache/src/brick.rs` (not in the diff) | `cd9e4a714dccc20c1c7d26179a2c8847b2d613e6`, equal to the index blob, so the implementer's restore holds |
| `.claude/reviews/F-119-working-pass-1.md` and `-pass-2.md` (not reviewed material) | `7f4e8ba7`, `7f9e6e4f` |

`git diff --name-only` lists only the three `crates/ocelli-volume/src` files, and the only other untracked paths are `tests/pyramid.rs` and the two earlier reviews. No `Cargo.toml` or `Cargo.lock` change.

**Author / reviewer**: an external coding agent wrote the implementation and both remediations (logs `the implementer log`, `-fix1.log`, `-fix2.log`). A fresh independent Claude reviewer wrote this pass and wrote none of the reviewed code or the earlier passes. This file is the only file this pass wrote in the worktree. Mutations were applied in place and restored from a scratchpad backup. Replay cases ran in a scratch crate outside the worktree with path dependencies on it.
**Result**: 3 defects, 0 smells, 4 nitpicks

F-119, pass 3: 3 defects, 0 smells, 4 nitpicks. Pass 2 was 3 defects, 0 smells, 3 nitpicks. All three pass-2 defects are closed. The three new defects are new findings of the same class as every earlier defect, an entry point whose guarded behaviour the suite does not watch.

## Prior findings

| Finding | Status | Evidence now |
|---------|--------|--------------|
| Pass 2 D1, F32 filter watched only in 1-D | **Closed** | F4 (drop `wz`), F5 (drop `fz`), F6 (drop `wy`) each make `fixture_float_mean` red. Self-audit drop `wx` and drop `fx` in the divisor are also red |
| Pass 2 D2, stop rule and ceil watched only on x | **Closed** | F3 (`grid[0] == 1`) red in two tests, F3xy red in `fixture_pyramid_levels_follow_brick_grid`, F7 (floor z) red in two tests |
| Pass 2 D3, `BrickCut` doc false | **Closed** | `lib.rs:262` now reads "refused a brick outside a stored level's grid", which is what `cut_level_brick(1, [2,0,0])` and the level 0 path both produce |
| Pass 2 N1 (pass 1 N1), plan prefix-on-failure sentence | Carried, N1 | Unchanged |
| Pass 2 N2 (pass 1 N3), `VoxelStore::slice_mut` bypass | Carried, N2 | Unchanged |
| Pass 2 N3, `level_centre` header narrower than the shortcut | Carried, N3 | `levels.rs:251` unchanged, as the fix log says |

## Acceptance challenged

### New fixtures recomputed by hand

Every value below is derived from plan decision 1 (child `c` covers `[cD, (c+1)D)` against parent `[pd, (p+1)d)`), decision 4 and decision 6, with layout index `(z*dy + y)*dx + x`. None was read from the implementation.

- **F32 `[2,4,4]`, value `index + 1`, to `[1,2,2]`, voxel 0.** Index is `8z + 2y + x`. x: D=2, d=1, parents 0 and 1 with weight 1 each. y: D=4, d=2, child `[0,4)` against parents `[0,2)` and `[2,4)`, weight 2 each. z: the same, weight 2 each. Parents are indices 0, 1, 2, 3, 8, 9, 10, 11, values 1, 2, 3, 4, 9, 10, 11, 12, sum 52. Every weight product is `1*2*2 = 4`, the divisor `2*4*4 = 32`, so `208 / 32 = 6.5`. Drop `wz` gives `104 / 32 = 3.25` and drop `fz` gives `208 / 8 = 26`, which match the fix log's left values. Because `wx = 1` here, drop `wx` stays 6.5, which is why the second case exists.
- **F32 `[4,6,8]` to `[2,3,4]`, the 15.5 assertion.** Index is `24z + 4y + x`. x: D=4, d=2, parents 0 and 1, weight 2. y: D=6, d=3, parents 0 and 1, weight 3. z: D=8, d=4, parents 0 and 1, weight 4. Indices 0, 1, 4, 5, 24, 25, 28, 29, values 1, 2, 5, 6, 25, 26, 29, 30, sum 124. Weight 24 each, divisor 192, `2976 / 192 = 15.5`. Drop `wx` gives `124*12/192 = 7.75`, which matches the log.
- **U8 `[4,4,129]`.** `BrickGrid::new` gives edge 128 for `EightBit`, grid `[ceil(4/128), ceil(4/128), ceil(129/128)] = [1,1,2]`. Level 1 is `[ceil(4/2), ceil(4/2), ceil(129/2)] = [2,2,65]`, grid `[1,1,1]`, so generation stops and returns 2. Floor z gives 64.
- **`fixture_y_and_z_overlaps_differ`, U8 `[1,5,3]` to `[1,3,2]`.** The test source sets `samples = [1..=15]` placed slice by slice, so `v(y,z) = 5z + y + 1`. y weights (D=5, d=3): c0 `p0:3, p1:2`, c1 `p1:1, p2:3, p3:1`, c2 `p3:2, p4:3`. z weights (D=3, d=2): c0 `p0:2, p1:1`, c1 `p1:1, p2:2`. Divisor `1*5*3 = 15`. Output order is y fastest within each z.
  - `(y0,z0)`: z0 row `3*1 + 2*2 = 7`, times 2 is 14. z1 row `3*6 + 2*7 = 32`, times 1. Total 46, `46/15 = 3.07`, rounds to **3**.
  - `(y1,z0)`: z0 `2 + 3*3 + 4 = 15`, times 2 is 30. z1 `7 + 3*8 + 9 = 40`. Total 70, `4.67`, **5**.
  - `(y2,z0)`: z0 `2*4 + 3*5 = 23`, times 2 is 46. z1 `2*9 + 3*10 = 48`. Total 94, `6.27`, **6**.
  - `(y0,z1)`: z1 `32`, times 1. z2 `3*11 + 2*12 = 57`, times 2 is 114. Total 146, `9.73`, **10**.
  - `(y1,z1)`: z1 `40`. z2 `12 + 3*13 + 14 = 65`, times 2 is 130. Total 170, `11.33`, **11**.
  - `(y2,z1)`: z1 `48`. z2 `2*14 + 3*15 = 73`, times 2 is 146. Total 194, `12.93`, **13**.
  - `[3, 5, 6, 10, 11, 13]` matches. No child is a tie, so this fixture does not watch the rounding mode, and AC1, AC2 and AC4 do. Truncation would give `[3, 4, 6, 9, 11, 12]`, so it does watch the weights.
- **`fixture_each_axis_keeps_ceil_stop_and_brick_bounds`, U8 `[3,129,5]`.** Grid `[1, ceil(129/128), 1] = [1,2,1]`. Level 1 `[ceil(3/2), ceil(129/2), ceil(5/2)] = [2,65,3]`, grid `[1,1,1]`, count 2. The one brick at `[0,0,0]` has extent `[2,65,3]`, and brick 1 on each axis is past `grid = 1`, so `BrickCut { axis, value: 1 }`.
- **Spacing `[8,6,5]` to `[2,3,2]` at unit spacing.** `1*8/2 = 4`, `1*6/3 = 2`, `1*5/2 = 2.5`, all exact in `f64`, three distinct ratios.
- **`stored_offset([2,3,4], 1, 2, 1, 1)`.** `((1*3 + 2)*2 + 1)*1 = 11`. With the y bound removed, `y = 3` gives `((0*3 + 3)*2 + 0) = 6`, which is the log's `Some(6)`.

### Composition and refusal cases, run through the public API in the scratch crate

All four pass on the reviewed tree.

- **U16 level 1 brick keeps its sample kind.** U16 `[130,4,1]` with value `300x`. `generate_pyramid` returns 3 (grids `[3,1,1]`, `[2,1,1]`, `[1,1,1]`). `cut_level_brick(1, [0,0,0])` has sample `U16`, extent `[64,2,1]`, 256 bytes, and voxel 1 is `(600 + 900)/2 = 750`, bytes `EE 02`. An I16 `[2,1,1]` of `-3, -3` cuts level 1 as sample `I16`, bytes `FD FF`.
- **Level 0 through `cut_level_brick` on an incomplete volume.** `[4,4,4]` U8 with only slice 0 placed. `cut_level_brick(0, [0,0,0])` is `BrickNotReady { brick: [0,0,0] }`, equal to `cut_ready_brick`.
- **Incomplete level 0 with a caller level present.** Slices 1 to 3 placed, then `append_level([2,2,2], [200; 8])`. `generate_pyramid` is `SliceNotPresent { index: 0 }` and the volume equals its clone.
- **Zero-dimension probe.** `[0,4,4]` and `[4,4,0]` U8. `generate_pyramid` returns `Err(LevelShape)` with `levels.len()` 1. See N4.

## Evidence

All commands ran now. `CARGO_TARGET_DIR=/Users/atulsharma/Documents/projects/tensorbee/ocelli/target`, rustc 1.97.1, aarch64-apple-darwin.

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|--------------------------|---------------------------|------------------------------|----------------------|
| `cargo fmt --all --check` | worktree, pinned hashes | now | 0, no output |
| `cargo clippy -p ocelli-volume -p ocelli-cache --all-targets -- -D warnings` | same | now | 0 |
| `cargo test -p ocelli-volume -p ocelli-cache` | same | now | 0. pyramid 20 passed, `ocelli-volume` lib 3 (including `stored_offset_refuses_each_axis`), levels 5, every other volume and cache suite green |
| `cargo test -p ocelli-render --test progressive` | same | now | 0. 6 passed, 3 ignored. The ignored ones need an adapter, so there is no GPU evidence |
| `cargo build -p ocelli-volume --target wasm32-unknown-unknown` | same | now | Finished. A no_std compile of the crate's own code under `#![cfg_attr(not(test), no_std)]` |
| `bin/ocelli.sh gate nostd` | same | now | ALL GREEN, "7 no_std crate(s) reach no std feature". It checks dependency features only |
| `python3 scripts/source_provenance_check.py` | same | now | 0, 1616 files |
| `python3 scripts/deviation_check.py` | same | now | 0, 27 deviations |
| Comment grep for `;`, em dash and en dash over added `//` lines in the `src` diff and all of `tests/pyramid.rs` | same | now | No match |
| Grep of added lines for ` as <type>`, `unsafe`, and `64` or `128` literals, plus `pyramid.rs` for `as` and `unsafe` | same | now | No match. `BrickGrid::new` is called only at `levels.rs:85` and `lib.rs:546`, both pre-existing |
| Scratch replay crate, 4 cases | path deps on the worktree | now | 4 passed on the tree. Each of the three discriminating cases fails under its mutant, see below |
| 26 mutations of `levels.rs` and `lib.rs`, each run against `--test pyramid --test levels --lib` | in place, restored from backup, hashes rechecked | now | see the table |

Mutations. F3 to F7 replay pass 2. SA1 to SA8 replay rows of the implementer's self-audit, chosen for the rows that were green and then got a fixture. N1 to N6 are this pass's own and appear in no earlier report or the self-audit.

| # | Mutation | Result |
|---|----------|--------|
| F3 | Stop rule `grid[0] == 1` | red, `fixture_pyramid_levels_follow_brick_grid` and `fixture_each_axis_keeps_ceil_stop_and_brick_bounds` |
| F3xy | Stop rule tests x and y | red, `fixture_pyramid_levels_follow_brick_grid` |
| F4 | Float product drops `wz` | red, `fixture_float_mean` |
| F5 | Float divisor drops `fz` | red, `fixture_float_mean` |
| F6 | Float product drops `wy` | red, `fixture_float_mean` |
| F7 | `pyramid_dims` floors z | red, both pyramid fixtures |
| SA1 | Float product drops `wx` | red, `fixture_float_mean` |
| SA2 | `axis_weight` for y with `full` and `coarse` swapped | red, 10 tests including `fixture_y_and_z_overlaps_differ` |
| SA3 | `pyramid_dims` floors y | red, `fixture_each_axis_keeps_ceil_stop_and_brick_bounds` |
| SA4 | Stop rule omits y | red, `fixture_each_axis_keeps_ceil_stop_and_brick_bounds` |
| SA5 | `level_centre` for y with dims swapped | red, `fixture_level_voxel_centres` |
| SA6 | `stored_offset` drops the y bound | red, `stored_offset_refuses_each_axis` |
| SA7 | `scaled` reads the x child dim for y | red, AC1 and `fixture_spacing_is_the_dimension_ratio` |
| SA8 | Float divisor drops `fx` | red, `fixture_float_mean` |
| N1 | `cut_level_brick` passes `BrickSample::U8` instead of `BrickSample::from(self.voxel)` | **GREEN, 20 passed**. Red against the scratch U16 case |
| N2 | `cut_level_brick` drops the `index == 0` delegation to `cut_ready_brick` | **GREEN, 20 passed**. Red against the scratch incomplete case |
| N3 | `generate_pyramid` truncates levels above 0 before `ensure_complete` | **GREEN, 20 passed**. Red against the scratch caller-level case |
| N4 | `place` drops levels only when `levels.len() > 2` | red, `placing_level_zero_drops_stale_levels` |
| N5 | Positive rounding numerator `2S + W - 1` (ties down) | red, 4 tests including AC1 and AC4 |
| N6 | Negative rounding numerator `-2S + W - 1` (ties toward zero) | red, `fixture_signed_tie_rounds_away_from_zero` |

## Defects

### D1, the sample kind of a coarse-level brick is unwatched

**Where**: `crates/ocelli-volume/src/levels.rs:167`, watched by nothing. Every `cut_level_brick` call in `tests/pyramid.rs` (AC9, AC13, `fixture_each_axis_keeps_ceil_stop_and_brick_bounds`) is on a `U8` volume.
**What**: Mutation N1 hard-codes `BrickSample::U8` and the suite stays green. Under the mutant a U16 or I16 level-1 brick is cut at one byte per voxel. The extent is right, the byte count is half, the bytes are the low and high halves of different voxels, and the sample tag says unsigned eight-bit. An I16 brick tagged U16 by a similar slip would turn `-3` into `65533`. That is a quietly wrong voxel at every coarse level of every CT series, which is the class `BrickSample::from` exists to prevent (`lib.rs:82-86`, PS3.3 C.7.6.3).
**Why it is wrong**: Plan consumer map, `cut_level_brick` provider `BrickSample::from(VoxelKind)`. Microscope section 2, a test that passes both ways is a defect.
**Evidence**: Scratch case `u16_level_brick_keeps_sample_kind` passes on the tree and fails under N1. Its expected values are derived above.
**Scope and regression**: Test gap only, production code correct. No consumer outside the crate calls `cut_level_brick` yet (Q10), and F-122 will be the first. Smallest fix: a U16 (or I16) `cut_level_brick(1, ..)` case asserting `sample` and two sample bytes, such as the `[130,4,1]` value `300x` case, voxel 1 is 750, `EE 02`.

### D2, the level 0 delegation of `cut_level_brick` is unwatched

**Where**: `crates/ocelli-volume/src/levels.rs:162-164`. The only level 0 call is AC13 (`pyramid.rs:624-626`), on a complete volume.
**What**: Mutation N2 deletes the `index == 0` branch so level 0 goes through `Brick::cut` on `level_bytes(0)` with no readiness check, and the suite stays green. On a complete volume the two paths give identical bricks, so AC13's equality cannot tell them apart. Under the mutant, `cut_level_brick(0, b)` on a volume with an absent slice cuts the cleared zeros that `ready_bricks` exists to refuse.
**Why it is wrong**: Plan decision 13, "Level 0 delegates to `cut_ready_brick`, so readiness keeps one predicate", and the method's own doc at `levels.rs:152-158`, "Level 0 is [`Volume::cut_ready_brick`]" and "level 0's own refusals". Microscope section 2.
**Evidence**: Scratch case `level0_cut_on_incomplete_is_not_ready`, `[4,4,4]` U8 with only slice 0 placed, expects `BrickNotReady { brick: [0,0,0] }`. Passes on the tree, fails under N2.
**Scope and regression**: Test gap only. Smallest fix: that case added to `level_bricks_share_one_cache` or as its own test.

### D3, the refusal-before-truncation order of `generate_pyramid` is unwatched

**Where**: `crates/ocelli-volume/src/levels.rs:133-137`. `incomplete_level_zero_is_refused` (`pyramid.rs:506-544`) has only level 0, so the truncation is a no-op in both of its volumes.
**What**: Mutation N3 moves `ensure_complete` after the truncation, and the suite stays green. Under the mutant an incomplete volume carrying levels above 0 (a caller's F-143 level, or a pyramid generated before a progressive re-assembly) loses them while `generate_pyramid` reports a refusal. Pass 2 replayed this case in its scratch crate, which is why the ordering is correct, but it never became a suite test.
**Why it is wrong**: The method's doc at `levels.rs:129-130`, "`SliceNotPresent` when level 0 is incomplete, with the volume unchanged", and the plan state table row "Level 0 with slice `s` absent, `generate_pyramid`, levels and store length unchanged". Microscope "Check every mutation route's refusal policy ... and state preservation".
**Evidence**: Scratch case `incomplete_with_caller_level_keeps_it` passes on the tree and fails under N3.
**Scope and regression**: Test gap only. Smallest fix: in `incomplete_level_zero_is_refused`, `append_level` a coarser level before the `before` clone (a `[2,2,2]` buffer of 200 works because `append_level` does not check presence), then the existing `assert_eq!(volume, before)` watches it.

## Smells

None.

## Nitpicks

### N1, carried from pass 1 N1

The plan's "Ownership, commit and recovery" row says it tests a valid-prefix outcome after a mid-pyramid failure. No test exists, and passes 1 and 2 judged the case unreachable after the completeness check. Delete the sentence at `/complete-feature`.

### N2, carried from pass 1 N3

`Volume.data` and `VoxelStore::slice_mut` are `pub`, so `volume.data.slice_mut(n)` writes level 0 without the decision 10 drop. Pre-existing surface. Its only caller is still `place`.

### N3, carried from pass 2 N3

`levels.rs:251`, "Level 0, where the two dims match, is identity", describes one case of a per-axis rule. "An axis whose two dims match is identity" would be exact.

### N4, a zero-dimension level 0 makes `generate_pyramid` return an undocumented `LevelShape`

`Volume`'s fields are public, so `[0,4,4]` is constructible. `ensure_complete` passes (for `dz = 0` it checks nothing), the grid is `[0,1,1]`, which is not `[1,1,1]`, `pyramid_dims` gives `[1,2,2]`, and `coarser` refuses it. The result is `Err(LevelShape)` with the volume unchanged. The doc's `# Errors` lists only `SliceNotPresent`. Nothing is lost and nothing quietly wrong is produced, so this is preference. Either add "[`VolumeError::LevelShape`] for a level 0 with a zero dimension" or return 1 for an empty level 0.

## Verified clean

- **Filter against decision 1**: `parent_span`, `axis_weight`, `weight_product`, `weight_total` and the float equivalents. Every per-axis term was mutated in some pass and is now red (pass 2 I1, I2, F2 and this pass's F4 to F6, SA1, SA2, SA8).
- **Rounding against decision 4**: both branches, ties away from zero, `i128`, no clamp, `try_from` narrowing. N5 and N6 red.
- **Dims and stop rule against decisions 6 and 7**: ceil on every axis and the full-grid stop comparison, red per axis (F3, F3xy, F7, SA3, SA4). The stop rule reads `brick_grid_at`, so `BrickGrid::new` remains the only edge chooser. No 64 or 128 literal in added code.
- **Spacing**: `scaled` is called only from `append_level` (`levels.rs:66`). SA7 red.
- **Level geometry against decision 12**: SA5 red, and the equal-dims shortcut is exact per axis.
- **Casts and overflow**: no `as` and no `unsafe` in the diff or the test file. `axis_halved` uses `checked_shl` and `u64`. `stored_offset` is checked `u64` then `try_from`. The `i128` sum bound is under 2^114. Float axis weights are at most `min(D, d)`, so `weight_f64`'s `try_from` cannot fail on valid dims. `fold_parents` increments stop at the inclusive end, so `px += 1` cannot overflow.
- **State**: `place` truncates only after `copy_from_slice` succeeds (N4 and pass 1 M10 red). `downsample_level` refuses before it allocates. `generate_pyramid` refuses before it truncates (true on the tree, watched only by the scratch case, D3).
- **One cache**: no new cache type. AC13 keys levels 0 and 1 in one `BrickCache`.
- **Dependencies and boundary**: no manifest or lock change, no wasm-bindgen, no GPU or render-loop code, wasm32 build clean. No change outside `crates/ocelli-volume`, and `brick.rs` is the index blob.
- **Doc comments in the diff**: module header, `Level`, `levels`, `truncate`, the `place` paragraph, the `BrickCut` variant, the four new method docs, the `round_ties_away` and `axis_weight` comments, and the new test doc comments all match observed behaviour, apart from N3 and N4. The `stored_offset_refuses_each_axis` doc arithmetic is correct.
- **Consumers**: nothing outside `ocelli-volume` calls the four new methods or `truncate` (grep of `crates`, `examples`, `packages`), which matches Q10. `sync_ready_bricks` is unchanged and progressive is green.

## Limitations and handoff

- No GPU evidence. The three ignored progressive tests did not run.
- No build against a target without `std`. The evidence is the wasm32 no_std compile plus the dependency-feature gate.
- NaN and infinite F32 input and non-finite `level_voxel_to_level0` input are not exercised, and the plan specifies nothing for them.
- `bin/ocelli.sh native` was not rerun this pass. Pass 2 ran it at an earlier tree, and this remediation touched only test code and one unit test in `levels.rs`.
- The `docs/lld/volumes.md` and `docs/lld/cache.md` content pass 2 listed is still owed at `/complete-feature`.
- Next action: the implementer adds the three regression cases for D1 to D3 (each is a few lines, and the scratch versions above are the expected values). Then a fresh independent pass 4 is required. This pass is not acceptance or sprint readiness.
