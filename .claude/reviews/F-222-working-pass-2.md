# F-222 review, pass 2

**Reviewed**: base `c775931`, uncommitted working diff of `crates/ocelli-volume/src/levels.rs` and `crates/ocelli-volume/tests/pyramid.rs`. The `git diff` sha256 is `be6c970359b35f99c23fdcd93b599bf9181cb92c6cc012df76ad498b8b7aefbb`. The untracked `tools/oracle/out` symlink and `.claude/reviews/` are excluded. Worktree `/Users/atulsharma/Documents/projects/tensorbee/ocelli-wt-f-222`, branch `work/f-222-xai`.
**Author / reviewer**: an external coding agent implemented the diff. The sprint lead (Claude) made the pass-1 comment fixes. The reviewer is a separate Claude subagent that authored none of the design, the code or the fixes, and that did not write pass 1. Identities are as the caller stated them.
**Result**: 0 defects, 0 smells, 4 nitpicks

## Acceptance challenged

- **Tree identity against pass 1**: I rebuilt pass 1's `pyramid.rs` by reversing the two comment edits in a scratch copy. That copy was then swapped in, hashed and restored with `cmp`. The rebuilt `git diff` hashes to `7ae8d6ce1d9bee8af0e48e3e3cde4795cd5754d53fe6ddd7438957c65c50537e`, which matches pass 1's recorded prefix `7ae8d6ce1d9bee8a`. So the reviewed tree is pass 1's tree plus exactly these two edits:
  - AC6, one line becomes two: "F-119 decision 2, kept by F-222 decision 9, computes every level from level 0."
  - AC9, the two-line sample sentence becomes "The inputs are `f64` coordinates under F-119 decision 12, not samples."
  
  `levels.rs` is unchanged since pass 1, so pass 1's arithmetic and mutation evidence carries over to it.
- **D1 closed**: F-119 plan line 151 is decision 2, "Every level is computed from level 0, never from the level above it." F-222 plan decision 9 reads "Every level is still computed from level 0 (decision 2)". The citation is correct. AC6's volume is `VoxelKind::F32`, built by `filled([4, 1, 1], ..., VoxelKind::F32, ...)`, so the "Samples are IEEE 754 binary32 words" line that remains on AC6 is true for that test.
- **D2 closed**: F-119 plan line 161 is decision 12, "one mapping from a level voxel to level 0 continuous index ... `u = (c + 0.5) * D / d - 0.5`". That is the affine coordinate formula, and F-222 decision 8 names it as "the affine formula of decision 12". AC9's inputs are `f64` coordinates passed to `level_voxel_to_level0`. Both volumes are `VoxelKind::U8`, so "not samples" is true. The next sentence, "The operator's answer of 2026-11-05 settles the sample mean.", matches the answer quoted at `docs/sprints/CURRENT_SPRINT.md:73`.
- **Replayed the original trigger**: `grep -n "Samples are IEEE" pyramid.rs` matches exactly six lines (1110, 1144, 1178, 1218, 1255 and 1286), one per test from AC1 to AC6. I checked the fixture of each of the six, and every one builds `VoxelKind::F32`. No copy is left on a non-F32 test.

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|--------------------------|---------------------------|------------------------------|----------------------|
| `cargo fmt --all --check` | working tree `be6c9703` | now | 0 |
| `cargo clippy -p ocelli-volume --all-targets -- -D warnings` | same tree, worktree target dir | now | 0 |
| `cargo test -p ocelli-volume` | same tree | now | 0. pyramid has 34 passed, and all 16 suites are green |
| `python3 scripts/prose_check.py` | same tree | now | 0, clean over 829 files |
| `python3 scripts/source_provenance_check.py` | same tree | now | 0, clean over 1648 files |
| Rebuild pass 1's diff and hash it | scratch copy with the two edits reversed, swapped in and restored | now | `7ae8d6ce...`, which matches pass 1. After the restore, `cmp` is identical and the diff hash is `be6c9703...` again |
| M3, divide by `full[0]*full[1]*full[2]` instead of the finite weight, `--test pyramid` | `levels.rs` byte copy, restored with `cmp` | now | Red: AC1, AC2, AC5 and AC6. 4 failed, the same tests as pass 1 |
| M4, clearing compares `0x8000_0002`, `--test pyramid` | `levels.rs` byte copy, restored with `cmp` | now | Red: AC4. 1 failed, the same test as pass 1 |
| `grep -rn -E "non_finite_floats_keep_the_current_bits\|decision is open" crates/` | same tree | now | exit 1, no match (AC10, code half) |
| Pass 1's M1, M2 and M5 to M11 | pass 1's tree, `levels.rs` byte-identical to this one | prior receipt, pass 1 | carried over, and not re-run |

## Defects

None.

## Smells

None.

## Nitpicks

- **N1, carried from pass 1 and unchanged**: the `!any` early return is an equivalent mutant today, because `narrow_f32` already writes `0x7fc0_0000` for a 0/0. This does not block.
- **N2, carried from pass 1 and unchanged**: the `0x8000_0000` literal in `mean_f32` has no name, while `CANONICAL_NAN_BITS` beside it does.
- **N3, carried from pass 1 and unchanged**: AC3's doc comment, "are the same empty box", reads as though the three volumes were one box.
- **N4, new**: plan procedure step 4 asks for a doc comment on each of AC1 to AC6 and AC9 that cites the operator's answer. After the D1 fix, AC6's comment no longer names the answer, although its expected `1.0` and `3.0` hold only under the re-weighting that the answer introduced. Its first line, "under re-weighting", only implies the source. The comment is true as written. Adding "The operator's answer of 2026-11-05 divides by the finite weight." would put the plan's citation back. This does not block.

## Verified clean

Every factual sentence in the doc comments of the F-222 diff was checked against the plan, F-119's plan, `CURRENT_SPRINT.md` and the fixture's own construction:

- **`levels.rs` doc comments**:
  - The `downsample_level` and `generate_pyramid` sentences match `mean_f32`.
  - The `CANONICAL_NAN_BITS` comment is correct. It cites F-222 decision 4, which is the canonical quiet NaN `0x7fc0_0000`.
  - The inline comments are correct. Decision 1 is the finiteness test before widening. Decision 4 is the empty-box NaN. Decision 6 is normalising negative zero after narrowing.
- **AC1**:
  - F32. The operator's answer leaves non-finite samples out of the mean, which matches the quote.
  - 28/7 = 4.0 = `0x4080_0000`, and 28/8 = 3.5.
- **AC2**:
  - F32. The claim that infinity is non-finite matches decision 1.
  - Both expected words are correct.
  - "infinity or a NaN" is correct: the pair gives `+inf` and the wide case gives `-inf + inf`.
- **AC3**:
  - F32. "keeps that box as NaN" matches the answer.
  - `0x7f80_0001` is signalling, and `0xffc0_0000` and `0x7fff_ffff` are NaN.
- **AC4**:
  - F32. The zero-sign rule matches the answer.
  - The `-2^-150` tie and the `0x8000_0001` control match pass 1's derivation.
- **AC5**:
  - F32. The "finite weight" sentence matches the answer. F-119 decision 1 is the overlap `w(c, p)`, which matches F-119 plan line 150.
  - The weights (3, 2), (1, 3, 1) and (2, 3) and the words 10, 32.5 and 40 are correct.
  - Dividing by the full weight 5 gives 6, 26 and 16, as stated.
- **AC6**:
  - F32, and decisions 2 and 9 are verified above.
  - The level 1 children 1.0 and 4.0, the level 2 value 9/3 = 3.0 = `0x4040_0000`, and 2.5 from averaging level 1 are recomputed and correct.
- **AC9**: decisions 12 and 8 are verified above. The claim that level 0 returns the input unchanged matches the bit-identical asserts, and the claim that the coarser NaN payload is not pinned matches `is_nan()` only.
- **No new prose outside the diff**: the plan's decision numbers 1 to 9 and the ACs are unchanged from pass 1.

## Limitations and handoff

- Only native aarch64-darwin was executed. AC11's claim of identical bytes on every target is argued from the code, not observed. `bin/ocelli.sh native` and `gate --sprint` were not run.
- I re-ran only two mutations, M3 and M4. The rest are pass 1's receipts on the same `levels.rs`.
- `docs/lld/volumes.md:152`, "is an open operator question", remains for `/complete-feature`, as the plan states.
- This pass is clean, so the review gate for the working diff is met. It is not completion: verification receipts, the LLD edit and integration remain with the sprint lead.
