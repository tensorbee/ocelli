# F-119 review, pass 4

**Reviewed**: uncommitted worktree diff on `work/f-119-xai` at base `19c7ed361b19091a2fac50acf1cf57e9db30471d`. Blob hashes pinned at start and identical at end, after every mutation was restored from a scratchpad backup:

| Path | Blob |
|------|------|
| `crates/ocelli-volume/src/levels.rs` | `48268a4c078df07d3e2a2eaf4cdf8ce35ebec36a` |
| `crates/ocelli-volume/src/lib.rs` | `450443a6cbaed247039d809d818d0dc465630839` |
| `crates/ocelli-volume/src/ortho.rs` | `97d1fa94939b404834de191fc4d856284d3e6cd8` |
| `crates/ocelli-volume/tests/pyramid.rs` (untracked) | `be8f637e52cce10d38d47cbbe83dba5ce17283f6` |
| `crates/ocelli-cache/src/brick.rs` (not in the diff) | `cd9e4a714dccc20c1c7d26179a2c8847b2d613e6` |

`git diff --name-only` lists only the three `crates/ocelli-volume/src` files. The other untracked paths are `tests/pyramid.rs`, the three earlier reviews and this file. No `Cargo.toml` or `Cargo.lock` change.

**Author / reviewer**: an external coding agent wrote the implementation and all three remediations (the latest log is `the implementer log`). A fresh independent Claude reviewer wrote this pass. It wrote none of the reviewed code and none of passes 1 to 3. This file is the only file this pass wrote in the worktree. Replay cases ran in a scratch crate outside the worktree, with path dependencies on it.
**Result**: 1 defect, 0 smells, 5 nitpicks

F-119, pass 4: 1 defect, 0 smells, 5 nitpicks. Pass 3 was 3 defects, 0 smells, 4 nitpicks. All three pass-3 defects are closed. The new defect is the same class as every earlier one: a plan decision that has a plausible wrong implementation and no test watching it. This time it is decision 5's f64 accumulation.

## Prior findings

| Finding | Status | Evidence now |
|---------|--------|--------------|
| Pass 3 D1, coarse-level sample kind unwatched | **Closed** | N1 (`BrickSample::U8`) makes `coarse_level_brick_keeps_its_sample_kind` red, 26 passed and 1 failed |
| Pass 3 D2, level 0 delegation unwatched | **Closed** | N2 (delete the `index == 0` branch) makes `level_zero_cut_on_an_absent_slice_is_not_ready` red |
| Pass 3 D3, refusal before truncation unwatched | **Closed** | N3 (`ensure_complete` after truncation) makes `incomplete_level_zero_is_refused` red |
| Pass 3 N1, plan prefix-on-failure sentence | Carried, N1 | Unchanged |
| Pass 3 N2, `VoxelStore::slice_mut` bypass | Carried, N2 | Unchanged. `slice_mut`'s only caller is still `place` |
| Pass 3 N3, `level_centre` header | Carried, N3 | `levels.rs:253` unchanged |
| Pass 3 N4, zero-dimension `LevelShape` undocumented | **Closed** | `levels.rs:132-133` documents it. `a_zero_dimension_returns_level_shape` covers x, y and z. A scratch probe confirms `downsample_level([1,2,2])` on `[0,4,4]` is also `LevelShape`, which "not strictly coarser" already covers |

## Acceptance challenged

### Pass 3 D1's expected values, recomputed by hand

The plan's decision 1, with layout `(z*dy + y)*dx + x`. None of these values were read from the implementation.

- U16 `[130,4,1]`, value `300x`. Edge 64 for `SixteenBit`. Level 0 grid is `[ceil(130/64), 1, 1] = [3,1,1]`. Level 1 is `[65,2,1]`, grid `[2,1,1]`. Level 2 is `[33,1,1]`, grid `[1,1,1]`, so `generate_pyramid` returns **3**.
- Level 1 voxel `(1,0,0)`. x: D=130, d=65. The child covers `[130,260)`, against parent 2 at `[130,195)` and parent 3 at `[195,260)`, weight 65 each. y: D=4, d=2. The child covers `[0,4)`, against parents `[0,2)` and `[2,4)`, weight 2 each. z weight 1. The sum is `65*2*(600+600) + 65*2*(900+900) = 390000` and the divisor is `130*4*1 = 520`, giving exactly **750**, which is `0x02EE` and `EE 02` little-endian. The largest stored value is `300*129 = 38700`, which fits `u16`.
- Brick `[0,0,0]` at edge 64 has extent `[min(64,65), 2, 1] = [64,2,1]`, or 256 bytes. Voxel `(1,0,0)` is at byte 2, so `samples[2..4]` is the right slot.
- I16 `-3, -3`. The sum is -6 over 2, which is -3 exactly, `FD FF`. F32 `1.0, 3.0` gives `2.0`.
- D2's coordinate case. U8 `[200,1,1]` at edge 128 has grid 2. Brick 1 has origin 128 and extent `200 - 128 = 72`, and values 128 to 199. It is the same on y and z.

Every value matches the test.

### The six green sweep rows, judged independently

`Volume`'s fields are public (`dims`, `levels`, `data`, `present`, `voxel`), so for each row I asked whether a field-built state can tell the mutant from the tree.

| Row | Independent judgement | Verdict |
|-----|-----------------------|---------|
| `ds-source-is-whole-store` | **The fix log's reason is wrong. Its conclusion is right.** A `Level` that disagrees with the store *is* constructible, because `levels` is public. The mutant still cannot be observed. `source` is indexed only through `stored_offset(full, ..)`, which is below `source_len`, so both versions read the same bytes. When `levels[0].dims` claims more than the store holds, the tree refuses at `.get(..source_len)` and the mutant refuses at the first parent read past the end, because every parent is covered by some child. Both refusals are `LevelShape`, and no state changes before either one | Equivalent mutant, acceptable |
| `gp-always-truncate` | Observable through the public fields. Scratch probe: append a level, then `levels.truncate(1)` by hand. The tree's `generate_pyramid` returns `Ok(1)` and leaves the store at 6 bytes, and the mutant would leave 4. The only effect is 2 orphan bytes that `Budgeted` counts. Nothing reads them, and no method-built state reaches it | Defensive. Harmless both ways |
| `place-always-drop` | The same state and the same reasoning | Defensive. Harmless both ways |
| `span-drop-start-gt-end` | `end = ((c+1)D - 1) div d >= cD div d` whenever `D >= 1`. `D = 0` fails `checked_sub` first | Unreachable, defensive |
| `weight-f64-overflow-becomes-zero` | One axis weight is the overlap of a length-D interval and a length-d interval, so it is at most `min(D, d) <= u32::MAX` | Unreachable, defensive |
| `read-stored-f32-becomes-zero` | `write_voxel` sends `F32` to `mean_f32`, and `read_stored` is reached only from `mean_i128`. `f32-arm-uses-integer-mean` watches the dispatch | Unreachable, defensive |

None of the six is a reachable defect.

### The NaN and infinity plan gap

**Classification: a plan gap to raise with the operator. It does not block this story.**

- **Not reachable in production today.** `describe` (`lib.rs:1061-1074`) maps only 8-bit unsigned, 16-bit signed and 16-bit unsigned stored containers to a `VoxelKind`, and refuses 32-bit. So `construct` and `assemble` never build an `F32` volume, and under Q10 nothing in production calls `generate_pyramid`. A non-finite coarse voxel cannot reach a render on any tier in this sprint.
- **The pinned behaviour is IEEE propagation, not a substitution.** A NaN or an infinity never becomes a finite number. A finite value becoming a different finite value is the quietly wrong pixel CLAUDE.md warns about, and that does not happen here. The scratch probe, F32 `[256,1,1]` with NaN at x=0, produces one NaN voxel at each of 3 levels. The poisoned footprint in level 0 units doubles per level on each axis, which is 8 times the volume per level in 3D. An infinity stays an infinity. `+inf` with `-inf` gives the canonical NaN.
- **The pin is a characterisation test, not a specification test.** `non_finite_floats_keep_the_current_bits` honestly says "The bits below are the current behaviour". CLAUDE.md says tests derive from the specification or the oracle, and here neither says anything. Pinning is acceptable as a tripwire only if the open question is recorded somewhere durable, so that a later reader does not take the test as the decision. See N5.
- **The decision needed**, before F-120 or before any story makes an `F32` volume constructible (Float Pixel Data, or a derived parametric map):
  - (a) Propagate. This is current behaviour, and the renderer then has to treat NaN as "no value" on all three tiers.
  - (b) Exclude non-finite samples from the mean and renormalise by the finite weight. A child with no finite parent stays NaN.
  - (c) Refuse generation with an error for a level 0 that contains a non-finite sample, which is the "report unavailable" rule applied to the pyramid.
  - The same question covers what `level_voxel_to_level0` does with a non-finite input.
  - Signed zero belongs in the same decision. P6 below shows that a mean of `-0.0` samples is written as `+0.0`, because the fold starts at `0.0`. The plan says nothing about it, and the two are numerically equal.

## Evidence

All commands ran now. `CARGO_TARGET_DIR=/Users/atulsharma/Documents/projects/tensorbee/ocelli/target`, aarch64-apple-darwin.

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|--------------------------|---------------------------|------------------------------|----------------------|
| `cargo fmt --all --check` | worktree, pinned hashes | now | 0 |
| `cargo clippy -p ocelli-volume -p ocelli-cache --all-targets -- -D warnings` | same | now | 0 |
| `cargo test -p ocelli-volume -p ocelli-cache` | same | now | 0. pyramid 27, volume lib 9, levels 5, every other volume and cache suite green |
| `cargo test -p ocelli-render --test progressive` | same | now | 6 passed, 3 ignored. The ignored tests need an adapter, so there is no GPU evidence |
| `cargo build -p ocelli-volume --target wasm32-unknown-unknown` | same | now | Finished |
| `bin/ocelli.sh gate nostd` | same | now | ALL GREEN, "7 no_std crate(s) reach no std feature" |
| `python3 scripts/source_provenance_check.py` | same | now | 0, 1616 files |
| `python3 scripts/deviation_check.py` | same | now | 0, 27 deviations |
| Grep of added `src` lines and all of `tests/pyramid.rs` for comment semicolons, U+2014 and U+2013, ` as <numeric>`, `unsafe`, `#[allow`, `unwrap()` and `expect(` | same | now | No match |
| `BrickGrid::new` call sites in `ocelli-volume` | same | now | `levels.rs:85` and `lib.rs:546`, both pre-existing. No 64 or 128 literal in added `src` code |
| 10 mutations through `cargo test -p ocelli-volume --test pyramid --test levels --lib` | exact-string replace, restored from backup, hash asserted after each | now | see the table |
| Scratch crate, `tests/p4.rs` (5 cases) and `tests/narrow.rs` | path deps on the worktree, release build | now | all pass on the tree. The two F32 cases fail under P1 and P5 respectively |
| `narrow_f32` against hardware `f64 as f32`, 20 million inputs | function body copied by `sed` from `ortho.rs:434-489` into the scratch crate | now | 0 mismatches. The inputs cover random bits, the f32 normal range, the f32 subnormal and underflow range, and exact ties at the 29-bit shift |

Mutations. N1 to N3 replay pass 3. P1 to P7 are this pass's own and appear in no earlier report and none of the fix log's 94 rows.

| # | Mutation | Result |
|---|----------|--------|
| N1 | `cut_level_brick` passes `BrickSample::U8` | red, `coarse_level_brick_keeps_its_sample_kind` |
| N2 | `cut_level_brick` loses the `index == 0` delegation | red, `level_zero_cut_on_an_absent_slice_is_not_ready` |
| N3 | `generate_pyramid` truncates before `ensure_complete` | red, `incomplete_level_zero_is_refused` |
| P1 | `mean_f32` narrows the running sum to f32 after every term, which is an f32 accumulator | **GREEN, 27 passed**. Red against scratch `f32_sum_is_not_accumulated_in_f32` |
| P2 | `brick_grid_at` drops `grid.level = index` | red, `levels.rs` `a_missing_level_is_not_level_zero` and `appended_sixteen_bit_level_keeps_the_brick_edge` |
| P3 | `level_voxel_to_level0` reads "full" from `level(index)`, so every level is identity | red, `fixture_level_voxel_centres` |
| P4 | `cut_level_brick` falls back to `cut_ready_brick` when the level is absent | red, `a_missing_generated_level_is_not_level_zero` |
| P5 | `mean_f32` narrows the sum to f32 before dividing | **GREEN, 27 passed**. Red against scratch `f32_sum_is_not_narrowed_before_division` |
| P6 | The F32 fold starts at `-0.0` | GREEN. It changes only the sign of an all-negative-zero mean, which the plan does not specify. See the plan gap |
| P7 | `place` drops `levels` above 0 but keeps their store bytes | red, `placing_level_zero_drops_stale_levels` |

## Defects

### D1, decision 5's f64 accumulation is unwatched

**Where**: `crates/ocelli-volume/src/levels.rs:377-385` (`mean_f32`), watched by nothing. `fixture_float_mean` (`tests/pyramid.rs:277`) and every other F32 case use small integer-valued samples, whose sums are exact in f32.
**What**: The plan's decision 5 is "the mean in `f64`", and approach step 1 is "`F32` sums `f64`". Two plausible wrong implementations pass the whole suite:
- P1 accumulates in f32. That is the natural `let mut sum = 0.0_f32`.
- P5 narrows the sum to f32 before dividing.

Both write a different f32 for some inputs. A wrong voxel value that is still a finite number is a quietly wrong voxel.
**Why it is wrong**: Plan decision 5 and approach step 1. Microscope section 2: if the code were wrong, the test must fail.
**Evidence**: Hand-computed cases, run in the scratch crate through `downsample_level`:
- F32 `[3,1,1]` of `16777216.0, 1.0, 1.0` to `[1,1,1]`. Each weight is 1 and the divisor is 3. The exact sum is 16777218, and `16777218 / 3 = 5592406`, which f32 represents. Under P1, `2^24 + 1` narrows ties-to-even back to `2^24` twice, and `16777216 / 3 = 5592405.33` narrows to `5592405.5`, because the f32 ulp in `[2^22, 2^23)` is 0.5. The tree gives `5592406.0` and P1 gives bits `1252698795` against the expected `1252698796`.
- F32 `[3,1,1]` of `33554432.0, 5.0, 8388609.0`. The sum is 41943046, and `41943046 / 3 = 13981015.33` narrows to `13981015` (ulp 1). Under P5 the sum narrows first, at ulp 4 in `[2^25, 2^26)`. 41943046 is the tie between 41943044 (mantissa odd) and 41943048 (mantissa even), so it goes to 41943048, and `/ 3 = 13981016`. The tree gives `13981015.0` and P5 gives bits `1263883608` against `1263883607`.
- Both cases pass on the tree, and each fails under its mutant.

**Scope and regression**: Test gap only. The production code is correct. No production path builds an `F32` volume today (`describe`, `lib.rs:1061-1074`) and nothing calls generation (Q10), so nothing is wrong on HEAD. Smallest fix: add those two cases to `fixture_float_mean`, compared by bits, with the arithmetic above in the doc comment. The first catches P1 and the second catches P5.

## Smells

None.

## Nitpicks

### N1, carried from pass 1 N1

The plan's "Ownership, commit and recovery" row says it tests a valid-prefix outcome after a mid-pyramid failure. No such test exists, and the case is unreachable after the completeness check. Delete the sentence at `/complete-feature`.

### N2, carried from pass 1 N3

`Volume.data` and `VoxelStore::slice_mut` are `pub`, so `volume.data.slice_mut(n)` writes level 0 without decision 10's drop. This surface existed before F-119.

### N3, carried from pass 2 N3

`levels.rs:253`, "Level 0, where the two dims match, is identity", describes one case of a per-axis rule. "An axis whose two dims match is identity" would be exact.

### N4, `downsample_level` and `generate_pyramid` do not document `LevelAbsent`

`levels.rs:95-99` and `:128-133`. When `levels` is empty, which is possible because the field is public, both return `LevelAbsent { index: 0 }` from `self.level(0)?`, after `ensure_complete` passes. `append_level` documents the same case at `:56`. One line in each `# Errors` section would make the three consistent.

### N5, the non-finite characterisation test should name the open question

`tests/pyramid.rs:1051-1063`. The doc says the bits are current behaviour, which is honest. It does not say that the behaviour is undecided or where the decision will be recorded. Once the plan gap above has a recorded home (an open question in the LLD at `/complete-feature`, or a backlog row), cite it in the doc, so that the test reads as a tripwire and not as the specification.

## Verified clean

- **Filter against decision 1**: `parent_span`, `axis_weight`, `weight_product`, `weight_total` and the float equivalents. Recomputed from the plan for D1's U16 case, and every per-axis term is red under some mutation in passes 2 to 4 or the fix log.
- **Rounding against decision 4**: `(2S + W) div (2W)` is `floor(S/W + 1/2)` for `S >= 0`, which rounds a tie up, away from zero. The negative branch mirrors it. Both are in `i128` and the result is narrowed by `try_from`. The sum is bounded by `max|v| * Dx*Dy*Dz`, which stays below 2^112.
- **Float narrowing**: `narrow_f32` agrees with hardware ties-to-even on 20 million inputs, including subnormals and exact ties. It is now on a pixel path, so this is the first direct check of it from the pyramid side.
- **Dims, stop rule and edge**: `axis_halved` is `max(1, ceil(D / 2^k))` with a `u64` numerator and a guard for `k >= 32`. The stop rule reads `brick_grid_at`, so `BrickGrid::new` is still the only edge chooser. P2 confirms that `grid.level` is watched.
- **Spacing**: `scaled` is called only from `append_level` (`levels.rs:66`). The module header's "Spacing is written only by `append_level`" is true.
- **Level geometry against decision 12**: P3 is red. The per-axis equal-dims shortcut is exact.
- **Refusal order and state**:
  - `downsample_level` checks completeness, then shape, then allocates and appends. After `coarser` passes, `append_level` cannot refuse, because the buffer length equals `byte_len(dims)` by construction.
  - `generate_pyramid` checks completeness before truncating (N3 red). A zero-dimension level 0 can carry no level above 0, because `coarser` refuses every child of a zero axis, so its `LevelShape` comes back with nothing truncated.
  - `place` truncates only after `copy_from_slice` succeeds, and P7 confirms the store bytes are dropped as well as the entries.
  - `SlicePresence` has no clear, so once level 0 is complete it cannot become incomplete again through a method.
- **Missing levels**: P4 is red. No path substitutes level 0 for an absent index.
- **Casts, unsafe, dependencies and boundary**: no numeric `as`, no `unsafe`, no `#[allow]`, no manifest or lock change, no wasm-bindgen, no render-loop code. The wasm32 build is clean. Nothing outside `crates/ocelli-volume` changed, and `brick.rs` matches the index blob.
- **One cache**: no new cache type. AC13 keys levels 0 and 1 in one `BrickCache`.
- **Prose**: no em dash, no en dash and no comment semicolon in added lines. These doc comments match observed behaviour: the `Level`, `levels` and `truncate` docs, the `place` paragraph, `BrickCut`, the four method docs, and the new test docs for D1 to D3 and N4. The exceptions are N3 to N5.
- **Consumers**: nothing outside `ocelli-volume` calls the new methods. `progressive` is green, and `sync_ready_bricks` is unchanged.

## Limitations and handoff

- There is no GPU evidence. The three ignored progressive tests did not run.
- There is no build for a target without `std`. The evidence is the wasm32 compile plus the dependency-feature gate.
- `bin/ocelli.sh native` was not rerun. This remediation changed test code and one `#[cfg(test)]` module.
- `docs/lld/volumes.md` and `docs/lld/cache.md` are still owed at `/complete-feature`. The NaN, infinity and signed-zero question should be recorded there as an open item, see N5.
- Operator question, which does not block F-119: choose (a), (b) or (c) for non-finite F32 samples in the pyramid, before F-120 or before an `F32` volume becomes constructible.
- Next action: the implementer adds D1's two F32 cases to `fixture_float_mean`. Then a fresh independent pass 5 reviews that change. This pass is not acceptance or sprint readiness.
