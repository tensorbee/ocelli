# F-227 review, pass 1

**Reviewed**: library half only. Staged tree `e50fe64f4d1f1340e47c4775ca479c7b05e39113` on HEAD `4458d5fec5b2150ba06b5eb03cf8a192100e155e`, branch `work/f-227-xai`, worktree `ocelli-wt-f-227`. Paths: `crates/ocelli-pixel/src/auto_window.rs` (new, sha1 `f895f7b1`), `crates/ocelli-pixel/src/lib.rs` (sha1 `d90ec5ff`), `crates/ocelli-pixel/tests/auto_window.rs` (new, sha1 `4a4618ba`). No unstaged changes to tracked files. Untracked `tools/oracle/out` ignored as instructed. Plan `.claude/plans/F-227-design.md` (approved). The viewer half, AC10 to AC18, is out of scope.
**Author / reviewer**: implementation by GLM-5.3 in opencode (its own report, not a generated trailer, is the only attribution evidence). Review by Claude Opus 5.5 as an independent subagent, which wrote none of the reviewed code and none of the plan.
**Result**: 4 defects, 1 smell, 5 nitpicks

## Acceptance challenged

- **AC1 to AC8, every asserted number re-derived** by an independent numpy `float32` script (`f227_derive.py` in the session scratchpad) that implements the plan's rules directly: per-frame stride `N.div_ceil(2^20 / K)` from index 0, per-frame modality `s * slope + intercept` in `float32`, stored 0 and the inclusive padding range left out per frame, nearest rank at `ceil(n/100)` and `ceil(99n/100)` over a full sort, LINEAR_EXACT `width = high - low`, `centre = low + width / 2`, constant `width 1`, background `m_max + 0.5`. Every window, count and grey the tests assert agrees with it, including AC5's delicate rank boundary (rank 2623 is the last 1, rank 259,623 is the last 99, so an off-by-one in either rank moves `low` to 2 or `high` to 100).
- **The order statistics against a full sort**, 3,000 randomised pools of 1 to 16 declared frames, each frame with its own slope (0.37 to 2) and intercept (0, -1024, 3.5), random padding in either declared order and signed stored values. `counted`, `low` and `high` agree bit for bit with a `total_cmp` sort, and every background case returns the largest left-out value. This rejects a wrong `select_nth_unstable_by` index into the tail.
- **AC6** re-derived by hand and by script for M = 1, 2, 16, 17, 175, 267, and first, last, strictly increasing and `min(M, 16)` long for every M from 1 to 4,999.
- **AC9** transform equality, refusal set and recovery, and the plan's claim that the window is `VoiTransform::new(.., LinearExact, ..)`. `AutoWindow::transform` is a single call to `VoiTransform::new`, and the module contains no VOI, rescale or display formula of its own, so the LUT chain still exists once.
- **Plan item 8's claim that `[low, high]` maps exactly onto `[ymin, ymax]`, and item 9's claim that the background window makes every sample `ymin`**, challenged in `f32` through the real `VoiTransform::apply`. Both fail on reachable inputs, D1.

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|--------------------------|---------------------------|------------------------------|----------------------|
| `cargo fmt --all --check` | staged tree above | now | 0 |
| `cargo clippy -p ocelli-pixel --all-targets -- -D warnings` | same | now | 0, no warnings |
| `cargo test -p ocelli-pixel` | same | now | 0, `auto_window` 11 passed, 80 tests over 10 binaries |
| `bin/ocelli.sh gate nostd` | same | now | ALL GREEN |
| `python3 scripts/prose_check.py` | same | now | 0, clean over 864 files |
| `python3 scripts/source_provenance_check.py` | same | now | 0, clean over 1708 files |
| `python3 scripts/guard_census.py` | same | now | 0 |
| `f227_derive.py`, numpy 2.5.2 `float32` | plan rules only | now | all AC1 to AC8 values agree with the tests |
| probe crate `f227probe`, bins `main`, `sweep`, `pct`, `one`, `diff` | path dependency on the worktree crates, scratch target under `target/f227-scratch` (deleted) | now | findings D1, D3, D4, N1 and the 3,000-case differential |
| 26 source mutations, `cargo test -p ocelli-pixel --test auto_window` per mutation, file restored from a byte copy after each run | `auto_window.rs` sha1 `f895f7b1` | now | table below, hashes re-checked OK afterwards |
| implementer's 8 mutations | GLM report | prior receipt, not re-run except where they coincide with mine | not relied on |

Mutations run by this reviewer:

| Mutation | Result |
|----------|--------|
| M1 floor the ranks, `max(1)` | red: `fixture_ranks_round_up`, `fixture_nearest_rank_is_exact`, `fixture_series_pool_excludes_per_frame`, `fixture_constant_background_signed_and_two_values` |
| M2 centre `low + width / 2 + 0.5` | red: 10 of 11 |
| M3 width `high - low + 1` | red: 10 of 11 |
| M4 padding low end exclusive | red: `fixture_series_pool_excludes_per_frame`, `fixture_constant_background_signed_and_two_values` |
| M5 padding high end exclusive | red: the same two |
| M6 padding bounds used in declared order, no min and max | **survived** |
| M7 stride read from index `stride - 1` | red: `fixture_cap_is_split_across_frames` |
| M8 whole cap per frame | red: `fixture_cap_is_split_across_frames` |
| M9 constant width 2 | red: `auto_window_pool_refusals`, `fixture_constant_background_signed_and_two_values` |
| M10 no non-finite refusal | red: `auto_window_pool_refusals` |
| M11 no truncate on refusal | red: `auto_window_pool_refusals` |
| M12 a refused frame increments `added` | red: `auto_window_pool_refusals` |
| M13 a refused frame commits its left-out maximum | **survived** |
| M14 stored zero tested as `to_bits() == 0`, sign not masked | **survived** |
| M15 transform built as LINEAR | red: 7 of 11 |
| M16 background centre `m_max` | red: `fixture_constant_background_signed_and_two_values` |
| M17 background from the smallest left-out value | red: the same |
| M18 series indices rounded | red: `fixture_series_sample_spacing` |
| M19 `TooManyFrames` at `>` | red: `auto_window_pool_refusals` |
| M20 second selection index off by one | red: 10 of 11 |
| M21 exclusion applied before the stride | red: `fixture_constant_background_signed_and_two_values` |
| M22 non-finite refused only for counted samples | **survived** |
| M23 padding validity skipped | red: `auto_window_pool_refusals` |
| M24 `EmptyFrame` skipped | red: `auto_window_pool_refusals` |
| M25 `NonFiniteWindow` check removed | **survived** |
| M13 first attempt, by reference | did not compile, replaced by the M13 above |

## Defects

### D1, the window's f32 edges do not reproduce the predicate the plan relies on, so a background frame can draw mid grey

**Where**: `crates/ocelli-pixel/src/auto_window.rs:296-311` (background), `:326-331` (percentile), and the claims at `:105-107`, `:117-123` and `:297-299`.
**What**: the window is built with one `f32` addition and never checked against LINEAR_EXACT's own `f32` edges, `c - w/2` and `c + w/2` in `lut.rs:651`. When the rounding of `m_max + 0.5`, or of `low + width / 2`, lands on the wrong side, a value the plan says is `ymin` is drawn far above it.
- Background. An all-padding frame of stored 32768, slope 256, padding 32768 gives `m_max = 8388608`. `centre = 8388608 + 0.5` rounds to `8388608`, the lower edge is `8388607.5`, and the frame draws **grey 128, not black**. An exhaustive sweep over every finite `f32` `m_max` finds 65 values that do not draw grey 0. The worst are `8388608` (grey 128), `4194303.75` (grey 64) and `2097151.625` (grey 32). `4194303.75` is under `2^22`, so the plan's own bound, "Exact while `|m_max| < 2^22`", is false too, and the code comment's "beyond which f32 cannot hold the half" is false in the other direction, because every integer up to `2^23` holds the half.
- Percentile. Two counted values one `f32` step apart, `0.001` and `0.0010000002`, give a window that draws the 99th percentile **grey 0**. The integer pair `[8388608, 8388609]` draws its low value grey 128. 492 of 1,210 two-value probes at widths of 1 to 5 ulp fail one edge.
- `AutoWindow::low` and `high` for `Background` are documented as "the lower window edge" and "the upper window edge". At `m_max = 8388608` the lower edge the transform uses is `8388607.5`, not `low`.
**Why it is wrong**: PS3.3 C.11.2.1.3.2 sends `x <= c - w/2` to `ymin`. Plan item 8 rests on `[low, high]` mapping onto `[ymin, ymax]` and item 9 on every left-out sample being `ymin`, and both hold only in real arithmetic. A blank end slice drawn mid grey is the quietly wrong pixel CLAUDE.md names.
**Evidence**: probe bins `sweep` (exhaustive over finite `f32`, same operations as `apply_window`'s LINEAR_EXACT branch), `one` and `pct`, all through the real `auto_window`, `AutoWindow::transform` and `VoiTransform::apply` except `sweep`, which reproduces the arithmetic so it could cover four billion inputs.
**Scope and regression**: every consumer of `AutoWindow`, which today is only the tests, and the viewer half once it lands. The realistic triggers need modality values near `2^22` to `2^23` or a percentile width of a few ulp, so ordinary CT, MR and PET are not affected, but the doc and the plan state the property without a bound that holds. The plan carries the same flaw, so the fix needs the designer as well as the implementer: either choose the edges so the `f32` predicate holds (for example step `centre` up with `f32::next_up` until `centre - 0.5 >= m_max`, and check `centre - width / 2 <= low` and `centre + width / 2 >= high` for a percentile window), or refuse with `NonFiniteWindow` or a new variant when it cannot. Smallest regression: `auto_window(&[Stored(32768.0)], slope 256, padding 32768..32768)` must draw `Modality(8388608.0)` at grey 0, and `auto_window(&[Stored(0.001), Stored(next_up(0.001))])` must draw the high value at 255 or refuse.

### D2, three false claims in the fixture doc comments

**Where**: `crates/ocelli-pixel/tests/auto_window.rs:344`, `:322-323`, `:213`.
**What**:
1. `fixture_ranks_round_up` says "Flooring the ranks would give 1 and 99, width 97". Floor ranks for n = 101 are `101 / 100 = 1` and `9999 / 100 = 99`, so the values are 1 and 99 and the width is **98**, the same width as the correct window. What actually tells them apart is the centre (51 against 50) and `low`.
2. `fixture_nearest_rank_is_exact` says "Under interpolation the window would move and the centre grey would not be 128". For stored 1 to 150 the linear-interpolation percentiles are 2.49 and 148.51 (numpy), so the centre is still **75.5** and modality 75.5 is still grey 128. The test does catch interpolation, through the width (146.02 against 147), not through the centre.
3. `fixture_stored_zero_is_left_out_not_modality_zero` calls the frame "signed CT-like". Its stored values are 0 and 1000 to 1099 through the unsigned helper `stored`, and the plan's item 3 calls this case "An unsigned CT with intercept -1024".
**Why it is wrong**: microscope section 3, a false prose claim is a defect. These comments are the hand computation HLD 27.2 R2 asks for, and a reader checking a fixture against its comment is misled about which assertion carries the discrimination.
**Evidence**: `f227_derive.py` prints `floor ranks for n=101: 1 99 -> width 98` and `interpolated (numpy linear) p1 p99 [2.49 148.51]`.
**Scope and regression**: test prose only, the assertions are right. Delete or correct the three sentences.

### D3, `AutoWindowBasis::Constant` is documented as "Every counted value equal", and it is not

**Where**: `crates/ocelli-pixel/src/auto_window.rs:102-104`.
**What**: the branch at `:327-331` takes `Constant` when the 1st and 99th percentiles are equal, not when every counted value is. 200 samples of stored 7 and one of stored 1000 give `basis: Constant, counted: 201, centre 7, width 1`, with a counted value of 1000.
**Why it is wrong**: a false claim in a public doc comment. The behaviour follows the plan's rule ("`high - low` not greater than zero"), so only the sentence is wrong, but a viewer label or LLD sentence written from it would say "constant image" about an image that is not.
**Evidence**: probe bin `main`, line `outlier: AutoWindow { center: 7.0, width: 1.0, low: 7.0, high: 7.0, counted: 201, frames: 1, basis: Constant }`.
**Scope and regression**: say "The 1st and 99th percentiles are equal". A fixture with one outlier would pin it.

### D4, `StoredRange`'s `PartialEq` doc claims bit equality is value equality, and for public `f32` fields it is not

**Where**: `crates/ocelli-pixel/src/auto_window.rs:71-79`.
**What**: the comment says "Bit equality is value equality for the bounds, the comparison `lut.rs` documents for the descriptor's input field". `lut.rs:163-169` justifies that equivalence only because NaN and negative zero are unreachable for its private field. `StoredRange` has public fields, so both are reachable. `StoredRange { 0.0, 0.0 } == StoredRange { -0.0, -0.0 }` is `false` although `padding_contains` treats them identically, and a NaN range compares equal to itself.
**Why it is wrong**: a false prose claim, and the cited justification does not transfer to this type.
**Evidence**: probe bin `main` prints `StoredRange 0 == -0: false` and `StoredRange NaN == NaN: true`.
**Scope and regression**: no consumer compares ranges today. Either state the actual relation (bitwise equality of the two bounds), or drop the impl, since nothing in the tree uses it and the plan's derive was never possible.

## Smells

### S1, five documented behaviours that no test pins, each shown by a surviving mutation

**Where**: `crates/ocelli-pixel/src/auto_window.rs:61-62, 85-86` (either-order padding), `:230-231, 249, 266` (a refused frame leaves the left-out maximum unchanged), `:256` (stored negative zero is stored zero), `:229-230, 251-255` (any sampled value, left out or counted, that is not finite is refused), `:282-283, 332-334` (`NonFiniteWindow`).
**What**: M6, M13, M14, M22 and M25 above all leave the suite green. Each removes a behaviour the doc comments or the plan state:
- M6: "The bounds are honoured in either declared order". Every fixture uses a one-point range. The viewer will convert F-210's `PaddingRange { value, limit }` straight into `low` and `high`, so a regression here would quietly stop excluding a padding range declared as `-2000 .. -3000`.
- M13: "A refused frame leaves the pool exactly as it was". The recovery test checks `values` and `added` but not `left_out_max`. A refused frame with a left-out sample of 3e38 before its overflowing sample would make a later all-background pool return centre 3e38 under M13.
- M14: the sign mask the plan calls out in Approach step 1. No test has `Stored(-0.0)`.
- M22: plan item 9 refuses "a non-finite modality value in any sampled frame". The only overflow fixture's samples are counted ones. Under M22 a left-out infinite sample reaches `left_out_max` and `finish` returns `Ok` with an infinite background centre, because the background branch has no finiteness check of its own.
- M25: `NonFiniteWindow` is reachable, `auto_window(&[Stored(-3.0e38), Stored(3.0e38)])` returns it, and no test asserts it.
**Why it matters**: microscope section 4, a guard nothing executes is the class that survives a green suite and is counted as coverage. Each has a specific future failure, listed above.
**Scope and regression**: five small cases in `tests/auto_window.rs`, each citing PS3.3 C.7.5.1.1.2 or C.11.1 and seen red under the corresponding mutation.

## Nitpicks

- **N1** `series_sample` at `:54` computes `k * (images - 1)` unchecked, so it panics with "attempt to multiply with overflow" in a debug build for `images > usize::MAX / 15`, which on wasm32 is about 286 million images. No real series reaches it. `k * q + k * r / (K - 1)` with `(q, r)` the quotient and remainder of `(M - 1) / (K - 1)` is overflow-free and gives the same indices.
- **N2** `sample_budget`'s doc, "The highest sample each declared frame may contribute", reads as a value. It is the largest number of samples.
- **N3** `AutoWindowPool::new(1)` reserves `2^20` `f32` (4 MiB) before seeing the frame, so every per-image window, including each thumbnail the viewer half plans, allocates 4 MiB however small the frame. Reserving `min(budget, len)` in `add_frame` would keep the cap.
- **N4** `new`'s doc says the fixed split "makes a mixed series with skipped frames sample the same values as an unmixed one". It samples fewer values. What it guarantees is that each contributing frame is read at the same stride.
- **N5** AC8's row says the all-zero plus all-padding pool gives "every grey 0". The test asserts its centre and width but not the grey of -2000 or 0.

## Verified clean

- No `as` cast in either file. Every integer step is `usize` with `div_ceil` or `/` on positive divisors. The rank product `99 * counted` cannot overflow because `counted <= 2^20`: per frame `ceil(N / ceil(N / b)) <= b`, frames are capped at the declared count by `TooManyFrames`, and the declared count times `2^20 / K` is at most `2^20`. Checked with a 3 times `2^20` frame in a 16-frame pool (65,536 counted) and a `2^20 + 1` frame alone (524,289 counted).
- Stride is at least 1 because `stored` is non-empty and the budget is at least 65,536. Reading starts at index 0 (M7 red).
- `select_nth_unstable_by(rank_low - 1)` then `rank_high - 1 - rank_low` into the right tail is the exact nearest-rank element, by the 3,000-case differential and by M20.
- `total_cmp` only ever sees finite values, since non-finite modality values are refused before they are pushed. Negative zero sorts below positive zero, which can only produce a `Constant` window centred on `-0.0`, harmless under LINEAR_EXACT.
- An infinite width or centre is refused with `NonFiniteWindow`, observed with ±3e38.
- Padding inclusive at both ends (M4, M5 red), applied to its own frame only (AC4), stored-space and after the stride (M21 red). Reversed bounds behave correctly in the code (probe: `[-5, -3]` declared as `-3 .. -5` leaves 2 counted).
- `AutoWindow::transform` equals `VoiTransform::new(None, &[c], &[w], 0, LinearExact, ymin, ymax)`, and its documented errors match `VoiTransform::new`'s branches at `lut.rs:345-357`.
- The test helper `assert_grey` uses `floor(y + 0.5)` clamped, the same rounding `display_index` in `crates/ocelli-render/src/colour.rs` performs for finite values in range.
- `StoredRange` either-order reading matches `ocelli_dicom::PaddingRange::contains` at `crates/ocelli-dicom/src/frame_open.rs:163-168`.
- `open_frame`, `WindowInput` and the three user documents are untouched, and none of the new names appears in `README.md`, `use.html`, `more.html` or `docs/lld`, which is consistent with the plan's split (they are the viewer half's).
- The implementer's report: the 80-test count, the check outputs and the decisions list match the tree. Its remark that the background addition "stays exact to 2^23" is true for integers only, see D1.
- The tree is unchanged by this review: the three file hashes re-checked OK after every mutation batch, `git status` shows the same staged set, and the scratch target directory was deleted.

## Limitations and handoff

- Not exercised: the viewer half and AC10 to AC18, any GPU tier, the oracle, and the operator study. The library half is CPU only and has no tier path of its own.
- The D1 sweep reproduces `apply_window`'s LINEAR_EXACT arithmetic rather than calling it for four billion inputs. The three concrete triggers were confirmed through the real `VoiTransform::apply`.
- D1 is also a plan defect (items 8 and 9 state an exactness the `f32` arithmetic does not have), so the remedy goes to the designer first and then to the implementer.
- Next owner: implementation remediates D1 to D4 and S1, then a fresh independent pass 2 reviews the changed material and replays the D1 triggers. This report is a bounded review of the library half, not completed acceptance of F-227.
