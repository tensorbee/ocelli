# F-119 review, pass 2

**Reviewed**: uncommitted worktree diff on `work/f-119-xai` at base `19c7ed361b19091a2fac50acf1cf57e9db30471d`. Blob hashes pinned at start and identical at end, after every mutation was reverted:

| Path | Blob |
|------|------|
| `crates/ocelli-volume/src/levels.rs` | `652fa694288b5b6a36ff31e770c48b60316c194d` |
| `crates/ocelli-volume/src/lib.rs` | `1a410578f1cf000b224165e0c6bd867353634fe4` |
| `crates/ocelli-volume/src/ortho.rs` | `97d1fa94939b404834de191fc4d856284d3e6cd8` |
| `crates/ocelli-volume/tests/pyramid.rs` (untracked) | `048baa1715f3689b77f39c68b61be1370bdf8560` |
| `.claude/reviews/F-119-working-pass-1.md` (untracked, not reviewed material) | `7f4e8ba74b467ac8d10d20e775e0ae7f5fcfb75a` |

**Author / reviewer**: an external coding agent wrote the implementation and the pass-1 remediation. A fresh independent Claude reviewer wrote this pass. It did not write pass 1 or any of the reviewed code. This file is the only file this pass wrote in the worktree. Replay and fixture-candidate tests ran in a scratch crate outside the worktree.
**Result**: 3 defects, 0 smells, 3 nitpicks

F-119, pass 2: 3 defects, 0 smells, 3 nitpicks. Pass 1 was 2 defects, 0 smells, 3 nitpicks. Both pass-1 defects and N2 are closed. All three new defects are new findings.

## Prior findings

| Pass-1 finding | Status | Evidence now |
|----------------|--------|--------------|
| D1, last slice of the completeness guard unwatched | **Closed** | Mutation `index + 1 < count` makes `incomplete_level_zero_is_refused` red, 16 passed and 1 failed |
| D2, replacement of caller-appended levels unwatched | **Closed** | Deleting the truncation in `generate_pyramid` makes `generate_pyramid_replaces_caller_appended_levels` red, 16 passed and 1 failed |
| N1, the plan's prefix-on-failure sentence has no test | Carried, see N1 | Unchanged |
| N2, `level_centre` not bit-identical for fractional input | **Closed** | Deleting the equal-dims shortcut makes `fixture_level_voxel_centres` red |
| N3, `VoxelStore::slice_mut` is a public write path that skips the level drop | Carried, see N2 | Unchanged, and `slice_mut`'s only caller is still `place` (`lib.rs:645`) |

## Acceptance challenged

I replayed each pass-1 trigger and one adjacent case through the public API in a scratch crate that has path dependencies on the worktree crates. All 8 cases pass on the reviewed tree:

- **D1 replay**: `[4,4,4]` U8 with slices 0 to 2 placed. `generate_pyramid` and `downsample_level([2,2,2])` both return `SliceNotPresent { index: 3 }`, and the volume equals its clone.
- **D1 adjacent, first slice absent while a caller level is present**: slices 1 to 3 placed, then `append_level([2,2,2], [200; 8])`. Both entry points return `SliceNotPresent { index: 0 }`, and the volume, caller level included, equals its clone. So the refusal comes before the truncation.
- **D2 replay plus adjacent**: U16 `[130,64,64]` constant 1000, then three caller levels: `[65,32,32]` of `0xAB`, `[10,10,10]` of `0xCD` (a shape the pyramid never makes) and `[5,5,5]`. `generate_pyramid` returns 3, level 2 is `[33,16,16]`, every level above 0 is 1000, the store length is exactly the sum of the three generated spans, and `level(3)` is `LevelAbsent`.
- **N2 adjacent, an axis of dimension 1 at every level, on a non-zero level**: U8 `[1,300,1]` gives levels `[1,300,1]`, `[1,150,1]` and `[1,75,1]`. `level_voxel_to_level0(2, [119.25118477159562, 3, 119.25118477159562])` returns the x and z input bits unchanged and y `13.5`. By hand, `(3 + 0.5) * 300 / 75 - 0.5 = 13.5`, and child 3 covers level 0 `[12, 16)` under decision 1, whose centre in the plan's centre-indexed convention is 13.5.
- **The shortcut is per axis and cannot mask another axis's ratio**: U8 `[6,1,1]` `1..6` to `[3,1,1]` gives `[2,4,6]` (1.5, 3.5 and 5.5 round away). `level_voxel_to_level0(1, [2, 0.25, 0])` is `[4.5, 0.25, 0]`. `level_centre` takes its own axis's `full` and `coarse`, so the shortcut fires only where that axis's dims are equal. On that axis `(c + 0.5) * D / D - 0.5 = c` exactly, so the shortcut is decision 12's formula with the rounding error removed. A mutant that hands axis y the x dims gives y `0.333` in AC16's odd case, which is red. **The shortcut is mathematically correct.**
- **Fresh signed case**: I16 `[3,1,1]` `-3, 4, -1` to `[2,1,1]`, where D=3, d=2 and W=3. c0 is `(2*-3 + 4)/3 = -0.667`, which rounds to -1. c1 is `(4 + 2*-1)/3 = 0.667`, which rounds to 1. Separately, `{-1, 0}` is -0.5, which rounds away to -1.
- **Place, then regenerate**: U8 `[130,64,64]` of 7 generates 2 levels. `place(0, [9; ..])` drops level 1 and the store returns to level 0's length. Regenerating puts 8 at level 1 voxel `[0,0,0]` (the mean of z0 = 9 and z1 = 7) and 7 at the first voxel of level 1 slice 1.
- **Multi-brick coarse level**: U16 `[130,64,64]` with value = x. Level 1 grid is `[2,1,1]`. `cut_level_brick(1, [1,0,0])` has extent `[1,32,32]` and first sample 129, which is the mean of 128 and 129, 128.5, rounded away. `[2,0,0]` is refused and `cut_level_brick(3, ..)` is `LevelAbsent { index: 3 }`.

Every AC1 to AC16 expected value was re-derived by pass 1 from plan decisions 1, 4, 6, 7 and 12. I spot-checked AC1, AC3, AC8 and AC16 again and they agree.

## Evidence

All commands ran now. `CARGO_TARGET_DIR=/Users/atulsharma/Documents/projects/tensorbee/ocelli/target`, rustc 1.97.1, aarch64-apple-darwin.

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|--------------------------|---------------------------|------------------------------|----------------------|
| `cargo fmt --all --check` | worktree, pinned hashes | now | 0, no output |
| `cargo clippy -p ocelli-volume -p ocelli-cache --all-targets -- -D warnings` | same | now | 0 |
| `cargo test -p ocelli-volume -p ocelli-cache` | same | now | 0. pyramid 17 passed, levels 5, every other volume and cache suite green |
| `cargo test -p ocelli-render --test progressive` | same | now | 0. 6 passed, 3 ignored. The ignored ones need an adapter, so there is no GPU evidence |
| `bin/ocelli.sh gate nostd` | same | now | ALL GREEN, "7 no_std crate(s) reach no std feature". This gate checks dependency features only |
| `cargo build -p ocelli-volume --target wasm32-unknown-unknown` | same | now | 0. The lib is `#![cfg_attr(not(test), no_std)]`, so this is a no_std compile of the crate's own code. No bare-metal target (`thumbv*`, `*-none`) is installed, so a no_std-std-library build was not possible |
| `bin/ocelli.sh native` | same | now | 0. Step 2 checks the eleven shared crates, `ocelli-volume` included, for wasm32. Declared dependencies resolve identically on both targets |
| `python3 scripts/source_provenance_check.py` | same | now | 0, 1616 files |
| `python3 scripts/deviation_check.py` | same | now | 0, 27 deviations |
| Comment prose grep for `;`, em dash and en dash over added `//` lines in `src` and all of `tests/pyramid.rs` | same | now | No match. `prose_check.py` skips `.rs` |
| Grep for ` as `, `unsafe`, and `64` or `128` literals in `levels.rs`, plus diff of every `Cargo.toml` and `Cargo.lock` | same | now | None. No dependency change |
| Scratch replay crate, 8 cases | path deps on the worktree | now | 8 passed |
| 12 mutations of `levels.rs`, run against `--test pyramid` | backup restore and `cmp`, hash rechecked | now | see the table |

Mutations. R1 to R3 replay pass 1, and the rest are this pass's own:

| # | Mutation | Result |
|---|----------|--------|
| R1 | `ensure_complete`: `index + 1 < count` | red, `incomplete_level_zero_is_refused` |
| R2 | `generate_pyramid` without the truncation | red, `generate_pyramid_replaces_caller_appended_levels` |
| R3 | `level_centre` without the equal-dims shortcut | red, `fixture_level_voxel_centres` |
| F1 | The negative branch of `round_ties_away` uses the positive formula | red, AC2 and AC5 |
| F2 | `parent_span` start rounds up instead of down | red, AC3 and AC8 |
| F3 | Stop rule `grid.grid[0] == 1` only | **GREEN, 17 passed** |
| F3xy | Stop rule tests x and y only | not run against the suite. Red against D2's proposed fixture |
| F4 | Float weight product drops `wz` | **GREEN, 17 passed** |
| F5 | Float divisor drops `fz` | **GREEN, 17 passed** |
| F6 | Float weight product drops `wy` | **GREEN, 17 passed** |
| F7 | `pyramid_dims` floors the z axis | **GREEN, 17 passed** |
| I1 | Integer `weight_product` drops `wz` | red, AC8 and decision 11 test |
| I2 | Integer `weight_total` drops `dz` | red, 7 failed |

## Defects

### D1, the F32 filter is watched only on a one-dimensional input

**Where**: `crates/ocelli-volume/src/levels.rs:369-384` (`mean_f32`, its own weight product at line 377 and its own divisor at line 382), watched only by `crates/ocelli-volume/tests/pyramid.rs:265-292` (AC6)
**What**: The float path does not reuse the integer `weight_product` and `weight_total`. It computes `wx * wy * wz` and `fx * fy * fz` in `f64` itself. AC6's only fixtures are `[2,1,1]` to `[1,1,1]`, where every y and z weight and dimension is 1. Dropping `wy` (F6), `wz` (F4) or `fz` (F5) leaves the suite green. Under F5, any F32 volume with more than one slice gets every coarse voxel scaled by `Dz`. For example, `[2,4,4]` values 1 to 32 to `[1,2,2]` gives voxel 0 as 26.0 instead of 6.5. That is a quietly wrong voxel on the F32 kind, which the integer fixtures cannot catch because the code is separate.
**Why it is wrong**: Plan decision 1 (`sum(w * v) / (Dx * Dy * Dz)`) and decision 5. Microscope section 2: a test that passes both ways is a defect. CLAUDE.md requires every pixel-arithmetic function to have a hand-computed fixture.
**Evidence**: F4, F5 and F6 left all 17 tests passing. My candidate fixture in the scratch crate passes on the reviewed tree and fails under each of F4, F5 and F6. It is F32 `[2,4,4]` with value `index + 1`, x fastest, downsampled to `[1,2,2]`. Child 0 covers parents 1, 2, 3, 4, 9, 10, 11 and 12, which sum to 52. Each has weight `1*2*2 = 4` and the total is `2*4*4 = 32`, so the result is 208 / 32 = 6.5 by bits.
**Scope and regression**: Test gap only, and the production code is correct today. Smallest fix: add that `[2,4,4]` to `[1,2,2]` F32 case to `fixture_float_mean`.

### D2, the stop rule and the ceil dims rule are watched only on the x axis

**Where**: `crates/ocelli-volume/src/levels.rs:143` (stop comparison) and `levels.rs:262-269` (`pyramid_dims`), watched only by `crates/ocelli-volume/tests/pyramid.rs:320-360` (AC8)
**What**: AC8's level 0 is `[130,64,64]`, so x is the only axis with more than one brick and the only odd axis. A stop rule that tests only `grid[0]` (F3) leaves the suite green. Under F3, a tall z stack such as U16 `[64,64,600]` (grid `[1,1,10]`) returns 1 and gets no pyramid. A z axis floored instead of ceiled (F7) also stays green, and it drops the last plane of an odd stack. Decision 6 says that is the reason for ceil.
**Why it is wrong**: Plan decisions 6 and 7 apply to every axis. Microscope section 2.
**Evidence**: F3 and F7 left all 17 tests passing. My candidate fixture in the scratch crate fails under F3, F3xy and F7, and passes on the reviewed tree. It is U8 `[4,4,129]`, constant 3, edge 128 from `BrickGrid::new`, so grid `[1,1,2]`. `generate_pyramid` returns 2 and level 1 is `[2,2,65]`.
**Scope and regression**: Test gap only. Smallest fix: add that case to `fixture_pyramid_levels_follow_brick_grid`, asserting the count and the level 1 dims.

### D3, the `BrickCut` variant doc is false now that a level above 0 is cut

**Where**: `crates/ocelli-volume/src/lib.rs:262`, made false by `crates/ocelli-volume/src/levels.rs:165-168`
**What**: The doc says "[`ocelli_cache::Brick::cut`] refused the level 0 prefix." `cut_level_brick(index > 0, ..)` maps a level-`k` refusal through `brick_cut_error` into the same variant. Trigger: U16 `[130,64,64]` after `generate_pyramid`, then `cut_level_brick(1, [2,0,0])`. This returns `BrickCut`, a refusal of level 1's grid, not of the level 0 prefix.
**Why it is wrong**: Microscope severity table: a false claim in prose is a defect. The new method's own doc (`levels.rs:159-160`) already describes it correctly as "outside that level's grid".
**Scope and regression**: Rustdoc only. Smallest fix: delete "the level 0 prefix", for example "[`ocelli_cache::Brick::cut`] refused a brick of a stored level."

## Smells

None.

## Nitpicks

### N1, carried from pass 1 N1

The plan's "Ownership, commit and recovery" row says the plan tests a valid-prefix outcome after a mid-pyramid failure. No such test exists. I agree with pass 1 that the case is unreachable after the completeness check. The fix is to drop that sentence from the plan rather than add a test.

### N2, carried from pass 1 N3

`Volume.data` is `pub` and `VoxelStore::slice_mut` is `pub` (`lib.rs:431`), so `volume.data.slice_mut(n)` writes level 0 without the decision 10 drop. This surface predates the story. Narrowing it is a later story's call.

### N3, the `level_centre` header comment is narrower than the shortcut

`levels.rs:251` says "Level 0, where the two dims match, is identity". The shortcut fires on any axis whose dims match at any level, for example axis x of `[1,300,1]` at level 2. The comment is true but describes one case of a per-axis rule. It could read "An axis whose two dims match is identity."

## Verified clean

- **Filter arithmetic against plan decision 1**: `parent_span` covers `c*D div d` to `((c+1)*D - 1) div d`. `axis_weight` is the integer overlap of `[cD, (c+1)D)` and `[pd, (p+1)d)`. Integer weights are summed in `u128` and `i128` and divided by `Dx*Dy*Dz`. F2, I1 and I2 are red.
- **Rounding against decision 4**: both branches match. F1 is red. `sum >= 0` against `> 0` is equivalent, as pass 1 noted.
- **Overflow and casts**: `levels.rs` and the `lib.rs` diff contain no `as` and no `unsafe`. The sample magnitude is at most 2^16 and the weight total at most 2^96, so `2S + W` is under 2^114, inside `i128`. Each float axis weight is at most `d`, which fits in `u32`, so the `weight_f64` `try_from` cannot fail on valid dims. `axis_halved` uses `checked_shl`, a `u64` numerator and `try_from`. `stored_offset` is checked `u64` arithmetic and then `try_from`. `base_len` keeps `VoxelStore::new`'s saturating semantics unchanged.
- **Refusal and state preservation**: `ensure_complete` runs before the truncation in `generate_pyramid` (replay: first slice absent with a caller level present, volume unchanged). `downsample_level` refuses before it allocates. `place` truncates only after `copy_from_slice` succeeds (AC15).
- **Zero dims**: `Volume` fields are public, so a zero dim is constructible. Level 0 cannot have a level above it then, because `coarser` needs `1 <= next <= limit`, so the post-truncation `LevelShape` path discards nothing.
- **Termination**: every loop iteration appends a level strictly coarser than level 0, and any axis longer than the edge halves, so the stop grid is reached.
- **One cache, one edge chooser**: there is no new cache type, and AC13 keys levels 0 and 1 in one `BrickCache`. `levels.rs` has no 64 or 128 literal, and the stop rule reads `brick_grid_at`, which calls `BrickGrid::new`.
- **One spacing formula**: `scaled` is called only from `append_level`.
- **no_std**: the new code uses `alloc::vec` and only arithmetic on `f64`. The wasm32 lib build and `bin/ocelli.sh native` both exit 0.
- **Consumers**: outside `ocelli-volume`, nothing calls the four new methods or `truncate`, which matches Q10. `sync_ready_bricks` is unchanged and progressive is green.
- **User docs**: `README.md`, `use.html` and `more.html` do not mention pyramids, levels or downsampling. `more.html:71` says the image does not build volume rendering. No user-facing statement becomes false.
- **Doc comments in the diff**: the `Level` doc, the `levels` field doc, the `truncate` doc, the `place` paragraph and the four new method docs all match the observed behaviour. The exception is D3, which is an unchanged line made false.

## Limitations and handoff

- There is no GPU evidence. The three ignored progressive tests did not run.
- A no_std build against a target with no `std` was not possible, because no such target is installed. The evidence is the wasm32 no_std lib compile plus the dependency-feature gate.
- NaN and infinite F32 inputs and non-finite `level_voxel_to_level0` inputs are not exercised, and the plan specifies nothing for them.
- **What `docs/lld/volumes.md` must say at `/complete-feature`**: line 22's `Level` row must stop saying "caller-supplied" only, because later entries are now caller bytes or a downsample of level 0. Lines 125 to 128 must replace "Pyramid generation is F-119" with what this story built: the exact-overlap box filter from level 0 in the stored domain, ties-away rounding, `max(1, ceil(D0 / 2^k))` dims, the stop after the first one-brick level, `level_voxel_to_level0`'s `(c + 0.5) * D0 / dk - 0.5`, `place` dropping every level above 0, `generate_pyramid` replacing caller levels, and `cut_level_brick`. `docs/lld/cache.md` gains the plan's one line that a level above 0 is a `BrickKey.level` in the same `BrickCache`.
- Next action: the implementer adds the D1 and D2 fixtures, fixes the D3 doc line and considers N3. Then a fresh independent pass 3 is required. This pass is not acceptance or sprint readiness.
