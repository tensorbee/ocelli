# F-119 review, pass 1

**Reviewed**: uncommitted worktree diff on `work/f-119-xai` at base `19c7ed3`. Changed files are `crates/ocelli-volume/src/levels.rs`, `src/lib.rs` and `src/ortho.rs`, plus the untracked `crates/ocelli-volume/tests/pyramid.rs`. Hashes before review: `levels.rs` b4d3f148, `lib.rs` 17d7d7ba. Both match after every mutation was reverted.
**Author / reviewer**: an external coding agent wrote it (log `the implementer log`). An independent Claude reviewer reviewed it and wrote none of it.
**Result**: 2 defects, 0 smells, 3 nitpicks

## Acceptance challenged

I recomputed every fixture by hand from plan decisions 1, 4, 6, 7 and 12. I did not read the expected values off the implementation.

- AC1. `[4, 2, 2]`, index `z*8 + y*4 + x`. Child 0 sums `0+1+2+3+4+5+6+7 = 28`. Each x weight is 2 and each y and z weight is 1, so the weighted sum is 56 and W = `4*2*2` = 16, giving 3.5, which rounds to 4. Child 1 sums `5*4 = 20`, giving 40/16 = 2.5, which rounds to 3. Spacing is `1*4/2 = 2`, `2*2/1 = 4` and `3*2/1 = 6`, all exact. The fixture's numbers match.
- AC2. -28/8 is -3.5, which rounds to -4. That is 0xFFFC, so the bytes are `FC FF`. Matches.
- AC3. D=5, d=3, scaled intervals child `[5c, 5c+5)` and parent `[3p, 3p+3)`. c0 has weights 3 and 2, giving 70/5 = 14. c1 has 1, 3 and 1, giving 150/5 = 30. c2 has 2 and 3, giving 230/5 = 46. Spacing is computed as `1.0*5.0/3.0`, the same operation order as `scaled`. Matches.
- AC4. 1/2 rounds to 1 and 0/2 is 0. 1/4 is 0.25, which rounds to 0. A cascade would give 1. Matches.
- AC5. For `-32768` in the negative branch, `(2*262144 + 8) div 16` is 32768, negated to -32768, bytes `00 80`. The i128 headroom bound is 2^16 times DxDyDz (at most 2^96) times 2, which is under 2^127. Matches.
- AC6. 2.5 and -1.25 are exact. Matches.
- AC7. `0.5*6/3 = 1`, `0.75*4/2 = 1.5`, `2.5*3/2 = 3.75`, all exact. Matches.
- AC8. `BrickWidth::from(U16)` is SixteenBit, edge 64. Level 0 `[130,64,64]` has grid `[3,1,1]`. `ceil(130/2) = 65` gives `[65,32,32]` and grid `[2,1,1]`. `ceil(130/4) = 33` gives `[33,16,16]` and grid `[1,1,1]`, which stops the loop, so the count is 3. U8 has edge 128: level 0 grid `[2,1,1]`, level 1 grid `[1,1,1]`, count 2. Matches.
- AC10. `130*64*64*2 = 1064960`, `65*32*32*2 = 133120`, `33*16*16*2 = 16896`. Matches.
- AC16. `0.5*4/2 - 0.5 = 0.5`, `1.5*2 - 0.5 = 2.5` and `2.5*5/3 - 0.5`. This agrees with decision 1's footprint: the centre of child c in edge coordinates is `(c+0.5)D/d`, minus the half-voxel offset of a centre-indexed grid. Matches.

These are my own discriminating cases. I wrote them in a temporary test file, ran them and deleted the file. All passed.

- I16 `[3,1,1]` values `-5, 2, -1` to `[2,1,1]`. D=3 and d=2 give weights c0 `(2,1)` and c1 `(1,2)` with W=3. c0 is -8/3 = -2.67, which rounds to -3 (`FD FF`). c1 is 0/3 = 0.
- F32 `1, 2, 4` to `[2,1,1]`. 4/3 and 10/3 match `f32` division by bits.
- U16 `[1,3,1]` values `65535, 0, 65535` to `[1,2,1]`. Each child is `2*65535/3 = 43690` (`AA AA`), and y spacing is 1.5. This covers a 1-voxel axis and an odd axis together.

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|--------------------------|---------------------------|------------------------------|----------------------|
| `cargo fmt --all --check` | worktree | now | 0, no output |
| `cargo clippy -p ocelli-volume -p ocelli-cache --all-targets -- -D warnings` | worktree | now | 0 |
| `cargo test -p ocelli-volume -p ocelli-cache` | worktree | now | 0. pyramid 16 passed, levels 5, every other volume and cache suite green |
| `cargo test -p ocelli-render --test progressive` | worktree | now | 0. 6 passed, 3 ignored (they need a GPU adapter, so no GPU evidence) |
| `bin/ocelli.sh gate nostd` | worktree | now | 0. "7 no_std crate(s) reach no std feature". The gate checks dependency features and does not build for a no_std target |
| `python3 scripts/prose_check.py` | tracked files only | now | 0, 805 files. `in_scope` excludes `.rs`, and `scan` flags code semicolons, so I scanned the comment lines of every changed `.rs` file by grep for `;` and em or en dashes. None found |
| `python3 scripts/source_provenance_check.py` | worktree | now | 0, 1616 files |
| `python3 scripts/deviation_check.py` | worktree | now | 0, 27 deviations |
| 10 independent mutations plus 3 discriminating cases | `levels.rs`, `lib.rs` | now, each reverted, hashes re-checked | see below |
| `bin/ocelli.sh native` | n/a | not run, by me or by the implementer | not exercised |

Independent mutations, none of them on the implementer's list:

| # | Mutation | Result |
|---|----------|--------|
| M1 | `ensure_complete` loop `index < count` changed to `index + 1 < count`, so the last slice is not checked | **GREEN, 16 passed** |
| M2 | `generate_pyramid` drops the initial truncation of levels above 0 | **GREEN, 16 passed** |
| M3 | Integer divisor from child dims instead of level 0 dims | red, 8 failed |
| M4 | Child dims floor instead of ceil | red, AC8 and AC10 |
| M5 | I16 read as `u16` | red, AC2 and AC5 |
| M6 | `cut_level_brick` uses the level 0 grid | red, AC13 |
| M7 | Parent span end drops the `- 1` | red, 13 failed |
| M8 | Float divisor from child dims | red, AC6 |
| M9 | `place` drops `levels` but not the store bytes | red, AC15 |
| M10 | `place` drops levels before the size refusal | red, AC15 |

## Defects

### D1, AC11 does not watch the last slice of the completeness guard

**Where**: `crates/ocelli-volume/tests/pyramid.rs:400-420`, guarding `crates/ocelli-volume/src/levels.rs:238-248`
**What**: Mutation M1 changes the loop so it never checks slice `count - 1`, and the suite stays green. Under the mutant, a volume whose last slice never landed downsamples the cleared zeros into every coarse level. That is a quietly wrong voxel at the bottom edge of every level. The production code is correct today, but nothing would catch a regression to the off-by-one.
**Why it is wrong**: microscope section 2 says a test that passes both ways is a defect. Plan decision 9 and the AC11 row reject "Averaging the cleared slice as zeros", and the only absent slice the test uses is interior slice 2.
**Evidence**: M1 left all 16 tests passing. My temporary case placed slices 0, 1 and 2 of a `[4,4,4]` U8 volume and expected `downsample_level([2,2,2])` to return `Err(SliceNotPresent { index: 3 })`. It passed on the real code and failed under M1.
**Scope and regression**: the gap is in the test only. Smallest fix: add a last-slice-absent case, index `dz - 1`, to `incomplete_level_zero_is_refused`, for both `generate_pyramid` and `downsample_level`.

### D2, nothing distinguishes "replace levels above 0" (Q8) from "keep them"

**Where**: `crates/ocelli-volume/tests/pyramid.rs:484-501` (AC14), guarding `crates/ocelli-volume/src/levels.rs:133-136`
**What**: Mutation M2 deletes the truncation in `generate_pyramid`, and AC14 stays green. The loop starts from the last stored level. After a first generation that level is already one brick, so the mutant breaks out immediately and leaves the store byte-identical, which is the same result idempotence expects. AC14 rejects "appending a second pyramid behind the first", but this implementation cannot produce that wrong result even when the truncation is gone, so the fixture has no discriminating power over the line it is meant to cover.
**Why it is wrong**: plan Q8(a), decision 11 and the state-table row "Pyramid present, `generate_pyramid` again, levels above 0 replaced" all require replacement. Under the mutant, a level appended by a caller through `append_level`, such as F-143's levels, survives generation with bytes that are not a downsample of level 0. That is the sprint's first named defect class.
**Evidence**: M2 left all 16 tests passing. My temporary case built a `[130,64,64]` U8 volume of constant 7, appended `[65,32,32]` of constant 200 through `append_level`, then called `generate_pyramid`. On the real code it returns 2 and level 1 is all 7s. Under M2 it failed with `left: 200, right: 7`.
**Scope and regression**: the gap is in the test only. Smallest fix: add that case, a caller-appended level with distinct bytes, then `generate_pyramid` and an assertion that every level above 0 equals the downsample.

## Smells

None.

## Nitpicks

### N1, the plan's prefix-on-failure test does not exist

Plan, "Ownership, commit and recovery", says "The plan still tests that the volume is left with a prefix of valid levels, never a gap". No test does that. The implementer's admitted gap, that a later level failing after a valid prefix would keep the prefix, is the behaviour the plan itself sanctions. It is not a refusal-policy violation, since the stated refusal contract, "volume unchanged", applies only to `SliceNotPresent`, which is checked before truncation (`levels.rs:133`). I also judge it unreachable. After the completeness check, `pyramid_dims` is strictly coarser on some axis while the previous grid is not `[1,1,1]`, because an axis longer than the edge (at least 64) halves. `byte_len` of a smaller grid cannot overflow when level 0's did not. The i128 bound holds, and the `try_from` narrowings receive in-range means. Only allocation failure remains, and that aborts. It cannot be tested without fault injection, so the fix is to delete the plan's sentence rather than add a test.

### N2, the "is identity" claim holds only for integer indices

The `level_centre` doc at `levels.rs:250` says "Level 0, where the two dims match, is identity". That holds for integer voxel indices. I checked `c` from 0 to 4999 against several values of D and every result was exact. For fractional input it is off by 1 ULP about 10% of the time, for example `c = 119.25118477159562`, `D = 1319` gives `119.25118477159563`. The public method takes `[f64; 3]`, and F-120 may pass continuous positions to it. The error is harmless in size but the claim is false as written. Either return `voxel` unchanged for index 0 or qualify the comment to integer centres.

### N3, a public write path bypasses the level drop

Two public members of the existing API reach level 0 without going through `place`: `Volume.data` is a `pub` field and `VoxelStore::slice_mut` is `pub`. `volume.data.slice_mut(n)` therefore writes level 0 without dropping levels above 0, so decision 10 holds only for `place`. Today the only caller of `slice_mut` is `place` (`lib.rs:645`). This story did not create the surface. Narrowing `slice_mut` to `pub(crate)` would close it in a later story, if nothing outside the crate needs it.

## Verified clean

- Filter: `parent_span` and `axis_weight` match decision 1 exactly. Child weights sum to D, the 3D total is DxDyDz, and M3, M7 and M8 confirm the total is what the tests watch.
- Rounding: matches decision 4 for both signs. `sum >= 0` against `> 0` is an equivalent mutant, because 0 rounds to 0 on either branch. i128 accumulation and `try_from` narrowing are used, and there is no clamp.
- Source: every level is computed from the level 0 prefix (`get(..source_len)`), never from the previous level.
- Signedness: I16 is read with `i16::from_le_bytes`, the stored container. `place` stores the decoded container, so Bits Stored sign extension is upstream and is not reapplied, which is consistent with `present_slice_f32`'s contract. M5 goes red.
- Float: the sum is in f64, weights go through `u32` into `f64` (exact, each axis weight is at most d), and the result is narrowed by the existing `narrow_f32`. NaN propagates to a canonical NaN through `narrow_f32`, and the plan specifies nothing for NaN.
- Casts: `levels.rs` and the `lib.rs` diff contain no `as` cast and no `unsafe`. `Cargo.toml` and `Cargo.lock` are unchanged.
- Dims: `axis_halved` is `ceil(dim / 2^k)` with `max(1)`, and its overflow paths use checked arithmetic. M4 goes red.
- Stop rule: it uses `brick_grid_at`, which calls `BrickGrid::new`. There is no 64 or 128 literal in `ocelli-volume/src`, so `BrickGrid::new` is still the only edge chooser.
- Spacing: `scaled` is called only from `append_level` (`levels.rs:66`), and the generator never writes spacing.
- `place`: refusals return before mutation, and success drops the store bytes and the level entries. M9 and M10 go red.
- One cache: no new cache type, and AC13 keys level 0 and level 1 in one `BrickCache`.
- `no_std`: the crate is `#![cfg_attr(not(test), no_std)]`, new code uses `alloc::vec`, and there is no float intrinsic beyond arithmetic. The nostd gate passes.
- Consumers: outside the crate the store is reachable only through `slice(n)` and `cut_ready_brick`, both bounded to level 0. `sync_ready_bricks` is unchanged, and progressive is green.
- User docs: `more.html` line 71 says volume rendering is not built by the example image, and neither it, `use.html` nor `README.md` mentions levels or pyramids. No statement becomes false. `docs/lld/volumes.md:127` still says "Pyramid generation is F-119", which `/complete-feature` owns.
- Doc comments and the implementer's AC table: every row maps to a named test, and every test exists and passes. Rows AC11 and AC14 are not fully discriminating, see D1 and D2.

## Limitations and handoff

- `bin/ocelli.sh native` (the cross-target build) was not run by me or by the implementer. The nostd gate does not build the crate for a no_std target.
- There is no GPU evidence. The three ignored progressive tests did not run.
- Performance was not measured, and the plan makes no performance claim.
- Next action: the implementer adds the two discriminating test cases for D1 and D2, considers N1 to N3, and then a fresh independent pass 2 is required.
