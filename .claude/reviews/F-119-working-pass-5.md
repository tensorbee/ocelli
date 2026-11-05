# F-119 review, pass 5

**Reviewed**: `git diff 12f93e9..HEAD` on `work/f-119-xai`, HEAD `1126c43f245cd01e2717b69098aeaea8a147e4d8`, tree `bbc894ac9612deb147793665949b569b4ecd66a3`. HEAD is a merge of `sprint/s46` at `12f93e9` over `19b9f87`, which holds the implementation through fix 4. The diff touches only `crates/ocelli-volume`. The worktree had no tracked change at start or end. Blob hashes, pinned at start and identical at end after every mutation was restored:

| Path | Blob |
|------|------|
| `crates/ocelli-volume/src/levels.rs` | `7c763380fa0207681d997952030b3d68d035b874` |
| `crates/ocelli-volume/tests/pyramid.rs` | `3b16b8404fcbf8150659f881ac9a16c8dc133f30` |
| `crates/ocelli-volume/src/lib.rs` | `450443a6cbaed247039d809d818d0dc465630839` |
| `crates/ocelli-volume/src/ortho.rs` | `97d1fa94939b404834de191fc4d856284d3e6cd8` |
| `crates/ocelli-cache/src/brick.rs` (not in the diff) | `cd9e4a714dccc20c1c7d26179a2c8847b2d613e6` |

`lib.rs`, `ortho.rs` and `brick.rs` match the pass 4 pins. `levels.rs` and `pyramid.rs` changed in fix 4. Those blobs match the ones the fix 4 log reports.

**Author / reviewer**: an external coding agent wrote the implementation and all four remediations (the latest log is `the implementer log`). A fresh independent Claude reviewer wrote this pass. It wrote none of the reviewed code and none of passes 1 to 4. This file is the only file this pass wrote in the worktree. Independent cases ran in a scratch crate outside the worktree, with path dependencies on it.
**Result**: 0 defects, 0 smells, 3 nitpicks

F-119, pass 5: 0 defects, 0 smells, 3 nitpicks. Pass 4 was 1 defect, 0 smells, 5 nitpicks. Pass 4's D1, N4 and N5 are closed. N1 to N3 are carried, and none of them blocks.

## Prior findings

| Finding | Status | Evidence now |
|---------|--------|--------------|
| Pass 4 D1, f64 accumulation unwatched | **Closed** | P1 and P5 replayed, and both turn `fixture_float_mean` red. See Evidence |
| Pass 4 N4, `# Errors` omit `LevelAbsent` | **Closed** | `levels.rs:99` and `:135-136`. `an_empty_level_axis_returns_level_absent` (`pyramid.rs:1089`) passes |
| Pass 4 N5, non-finite test should name the open question | **Closed** | `pyramid.rs:1101-1106` names `docs/sprints/CURRENT_SPRINT.md` and "Questions for the operator". That heading is at line 57 of the file at HEAD, and the item at line 59 is the non-finite F32 question. Both say "before F-120" |
| Pass 4 N1 to N3 | Carried as N1 to N3 | Unchanged |

The non-finite F32 behaviour is the operator question recorded in `CURRENT_SPRINT.md`. Under this pass's brief it is not a finding. The pinning test was checked and is correct. Each of its assertions matches IEEE propagation through `narrow_f32` and the `level_centre` shortcut, and mutation Q2 below turns it red.

## Acceptance challenged

### D1's expected bits, recomputed by hand

I derived these from the IEEE 754 binary32 layout, not from the implementation or the fix log.

- `5592406.0`. `2^22 = 4194304 <= 5592406 < 2^23`, so the biased exponent is `22 + 127 = 149`. The mantissa field is `(5592406 - 4194304) * 2^(23-22) = 1398102 * 2 = 2796204`. The bits are `149 * 2^23 + 2796204 = 1249902592 + 2796204 =` **1252698796**. The mutant value `5592405.5` gives mantissa `1398101.5 * 2 = 2796203` and bits 1252698795.
- `13981015.0`. `2^23 <= 13981015 < 2^24`, so the biased exponent is 150. The mantissa is `13981015 - 8388608 = 5592407`. The bits are `150 * 2^23 + 5592407 = 1258291200 + 5592407 =` **1263883607**. `13981016` gives 1263883608.
- Case (a) under P1. The fold is x fastest. `0 + 16777216` is exact. `16777216 + 1 = 16777217` sits at the midpoint of the ulp-2 spacing in `[2^24, 2^25)` and goes to even, `16777216`, and the second `+ 1` does the same. `16777216 / 3 = 5592405.33`, and at ulp 0.5 that narrows to `5592405.5`. Red.
- Case (b) under P5. `41943046` lies in `[2^25, 2^26)` at ulp 4. Its neighbours are `41943044` (mantissa 2097153, odd) and `41943048` (mantissa 2097154, even), and it is their midpoint, so it goes to `41943048`. `41943048 / 3 = 13981016` exactly. Red.
- Cross terms. Under P5, case (a)'s sum of `16777218` is representable in f32, so (a) passes and (b) is the watcher. Under P1, case (b)'s running sum is `33554436`, then `41943044`. `/ 3` gives `13981014.67`, which narrows to `13981015`, so (b) passes and (a) is the watcher. Each case is needed and each mutant has its own red test.
- In f64 `16777218 / 3` is exactly `5592406`, and `41943046 / 3 = 13981015.33` narrows to `13981015` at ulp 1. Both match the test.

### Independent discriminating cases, scratch crate `review-p5/xcheck`

None of the expected values below was read from the implementation.

- **U16 odd axis**. `[3,1,1]` with `0, 2, 65535` to `[2,1,1]`. With D = 3 and d = 2, child 0 covers `[0,6)` and weighs parent 0 by 2 and parent 1 by 1, giving `2/3 = 0.67`, which rounds to **1**. Child 1 is `(1*2 + 2*65535) / 3 = 131072/3 = 43690.67`, which rounds to **43691**. Truncation would give `[0, 43690]` and a 2x box with a short edge cell would give `[1, 65535]`. The test passed with `[1, 43691]`. That rejects both wrong implementations at a sixteen-bit extreme, which no suite case pairs with an odd axis.
- **Refusal and then a valid operation, U16, through `place`**. `[4,1,2]` gives level 1 `[35, 55]`. A short `place(1, 3 bytes)` is refused and the whole `Volume` compares equal to the clone taken before it, pyramid included. A valid `place(1, ..)` then leaves `levels.len() == 1` and the store at 16 bytes, which is level 0 at width 2. A fresh `downsample_level` gives `[55, 75]` from the new level 0. AC15 uses U8 only. This case shows that `place`'s truncation keeps all of a two-byte level 0.

## Evidence

All commands ran now with `CARGO_TARGET_DIR=/Users/atulsharma/Documents/projects/tensorbee/ocelli/target`, on aarch64-apple-darwin.

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|--------------------------|---------------------------|------------------------------|----------------------|
| `cargo fmt --all --check` | HEAD tree, pinned blobs | now | 0, no output |
| `cargo clippy -p ocelli-volume -p ocelli-cache --all-targets -- -D warnings` | same | now | 0 |
| `cargo test -p ocelli-volume -p ocelli-cache` | same | now | 0. pyramid 28, volume lib 9, levels 5, assembly 14, cache lib 30, bricks 23, every other suite green |
| `cargo test -p ocelli-render --test progressive` | same | now | 6 passed, 3 ignored. The ignored tests need an adapter, so there is no GPU evidence |
| `cargo build -p ocelli-volume --target wasm32-unknown-unknown` | same | now | Finished, 0 |
| `bin/ocelli.sh gate nostd` | same | now | ALL GREEN, "7 no_std crate(s) reach no std feature" |
| `python3 scripts/source_provenance_check.py` | same | now | OK, 1625 files |
| `python3 scripts/deviation_check.py` | same | now | OK, 27 deviations |
| `python3 scripts/prose_check.py` | tracked files, which excludes `.rs` | now | OK, 813 files |
| Grep of all 1734 added lines for U+2014, U+2013, comment semicolons, numeric ` as `, `unsafe`, `#[allow`, `.unwrap()`, `.expect(`, `panic!` and `unreachable!` | `git diff 12f93e9..HEAD` | now | No match |
| `BrickGrid::new` call sites, and `64` or `128` literals in added `src` lines | same | now | `levels.rs:85` and `lib.rs:546`, both pre-existing. No literal |
| Manifest and lock diff | same | now | Empty. No new dependency |
| Callers of the four new methods outside `crates/ocelli-volume` | `crates/**/*.rs` | now | None. That matches Q10 |
| 6 mutations, exact-string replace with the count asserted at 1, restored from a scratchpad backup, blob hash asserted after each | `review-p5/mut.py` | now | See the table |
| Scratch crate `review-p5/xcheck`, 2 cases | path deps on the worktree | now | 2 passed |

Mutations. P1 and P5 replay pass 4. Q1 to Q4 are this pass's own. I checked them against passes 1 to 4 and the five implementer logs and found them in none.

| # | Mutation | Result |
|---|----------|--------|
| P1 | `mean_f32` narrows the running sum to f32 after every term | **red**, `fixture_float_mean` at `pyramid.rs:356`, left 1252698795, right 1252698796 |
| P5 | `mean_f32` narrows the sum to f32 before dividing | **red**, `fixture_float_mean` at `pyramid.rs:368`, left 1263883608, right 1263883607 |
| Q1 | `VoxelStore::truncate` sizes level 0 with `VoxelKind::U8` width, not the store's kind | red, `regenerating_is_idempotent`. The suite's `place` cases are U8 only. The xcheck U16 case covers that route |
| Q2 | `levels.rs` `read_f32` decodes big-endian | red, `fixture_float_mean`, `coarse_level_brick_keeps_its_sample_kind` and `non_finite_floats_keep_the_current_bits`, so the pin is a live guard |
| Q3 | `generate_pyramid` returns `levels.len() - 1` | red, 6 tests including AC8 and AC9 |
| Q4 | `axis_weight` takes `min` of the two lower bounds, not `max` | red, the `offset_bounds` unit test, and 13 pyramid tests when that suite runs alone |

## Defects

None.

## Smells

None.

## Nitpicks

### N1, carried from pass 1 N1

The plan's "Ownership, commit and recovery" row says it tests a valid-prefix outcome after a mid-pyramid failure. No such test exists, and the case cannot be reached after the completeness check. Delete the sentence at `/complete-feature`.

### N2, carried from pass 1 N3

`Volume.data` and `VoxelStore::slice_mut` are `pub`, so `volume.data.slice_mut(n)` writes level 0 without decision 10's drop. This surface existed before F-119.

### N3, carried from pass 2 N3

At `levels.rs:256`, "Level 0, where the two dims match, is identity" describes one case of a per-axis rule. "An axis whose two dims match is identity" would be exact.

## Verified clean

- **Arithmetic against the plan**:
  - Decision 1's filter: `parent_span`, `axis_weight`, `weight_product` and `weight_total`. Q4 is red, and the xcheck odd U16 case matches a hand overlap.
  - Decision 4's rounding: `(2S + W) div (2W)`, and its mirror for `S < 0`, in checked `i128` and narrowed by `try_from`.
  - Decision 5: f64 weights from `u32`, an f64 sum and divisor, and one `narrow_f32` at the end. P1 and P5 are red.
  - Decision 6's `axis_halved`, with a `u64` numerator and the guard for `k >= 32`.
  - Decision 7's stop rule through `brick_grid_at`. Q3 is red.
  - Decision 12's `level_centre`.
- **Casts and overflow**: no numeric `as` in added lines. Every widening is `from` and every narrowing is `try_from` or a checked op. `base_len` saturates in the same way the code it replaced did. `byte_len` and `stored_offset` are checked.
- **Refusal ordering and state**:
  - `downsample_level` checks completeness, then `level(0)`, then `coarser`, then computes and appends.
  - `generate_pyramid` checks completeness before it truncates.
  - On an empty axis, both calls return `LevelAbsent { index: 0 }` with the volume unchanged. The doc lines at `levels.rs:99` and `:135-136` are true.
  - `place` truncates only after a successful copy, and a refused `place` leaves the whole `Volume` equal (xcheck).
- **Structure**:
  - No `unsafe`.
  - No new dependency.
  - No new cache type. AC13 keys both levels in one `BrickCache`.
  - `BrickGrid::new` is still the only edge chooser.
  - No new trait or generic beyond the private `fold_parents` closure.
  - No `wasm-bindgen`.
  - Nothing outside `ocelli-volume` changed.
- **wasm32**: the build is clean.
- **Prose**: no em dash, en dash or comment semicolon in added lines. The fix 4 doc comments are true:
  - `fixture_float_mean`'s arithmetic, recomputed above. Its "16777216 / 3 = 5592405.5" is the f32 quotient, and P1 produces it.
  - `an_empty_level_axis_returns_level_absent`.
  - The N5 pointer in `non_finite_floats_keep_the_current_bits`.
- **Always-in-scope docs**: `README.md`, `examples/folder-viewer/src/use.html` and `more.html` mention no pyramid, downsample or level generation, so nothing there went stale.
- **Consumers**: nothing outside the crate calls the new methods. `progressive` (render) and `tests/progressive.rs`, `levels.rs` and `ready_bricks.rs` (volume) are green.

## Limitations and handoff

- There is no GPU evidence. The three adapter tests in `ocelli-render` `progressive` were ignored.
- There is no build for a target without `std`. The evidence is the wasm32 compile plus the dependency-feature gate.
- `bin/ocelli.sh native` was not rerun. Fix 4 touched only test code and doc comments in `levels.rs`.
- The xcheck cases are not in the suite. Adding the U16 `place` case would make Q1 red through `place` as well as through `generate_pyramid`. This is optional, because both routes share one `truncate`.
- `docs/lld/volumes.md` and `docs/lld/cache.md` are still owed at `/complete-feature`. N1 is fixed there too.
- The operator question on non-finite F32 samples is still open in `CURRENT_SPRINT.md` and does not block F-119.
- This is a clean bounded review of the F-119 diff at the pinned tree. It is not acceptance, a `/verify` receipt or sprint readiness. Next owner: `/complete-feature F-119`.
