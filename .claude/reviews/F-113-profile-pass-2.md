# F-113 review, pass 2

**Reviewed**: Staged index tree `97b9229d6321c769ca9828ed963ae0516d98ba77` on `sprint/s40`. HEAD is `b7a9992d403d0ff1defed8c367522750c3a66d78`. The review is `git diff --cached` only. `git write-tree` printed that hash before review and again after every mutation and the probe were restored. Unstaged and untracked paths were not reviewed. This file is unstaged.

**Author / reviewer**: staged implementation / 01a1a0e8-d38d-7d51-8286-d9c3267cf336 / independent pass 2. This reviewer did not write the change.

**Result**: 1 defect, 0 smells, 0 nitpicks

## Acceptance challenged

AC1 to AC7 from `.claude/plans/F-113-design.md`. HLD section 8 names three tiers and says encoded bytes are dropped after decode. It does not give a 4D byte formula. The product and the sample limits are the approved local plan. They are not an HLD formula. No staged file adds a deviation.

Independent checks, not copies of the function:

- Python: `512 * 512 * 300 * 1 * 1 * 2` is `157286400`. `64 * 64 * 8 * 4 * 1 * 2` is `262144`. `64 * 64 * 8 * 4 * 3 * 2` is `786432`. `65536 * 65536` is `4294967296`.
- Dropping `spec.columns` from the product made `a_large_volume_is_157286400_decoded_bytes` fail with left `307200` and right `157286400`. The const assert against `157286400` had already passed. `demand` did not hide the wrong product.
- Dropping `spec.times` made the 4D test fail with left `65536` and right `262144`. The same edit made `times` 0 return a profile, so that refusal is reached.
- Replacing `checked_mul` with `wrapping_mul` made the `u32::MAX` overflow assert fail at `profile.rs:241`. The earlier asserts in that test still passed.
- Forcing `profile_load` to return nothing made the large, 4D and one-byte-under tests fail with left `0` and the hand literals. That is `demand` substituting a zero profile. The tier C test stayed green because its remaining asserts expect zeros.
- Changing `bytes <= budget` to `bytes < budget` made the large and 4D tests fail on `decoded_admitted`. The boundary test, with one temporary tuple assert, failed with left `(false, false, false)` and right `(true, true, true)`. That expected triple is the equal-budget case. `would_admit`, `refused.is_none()` and `contains_key` were all false. Insert refuses the boundary because it calls `would_admit`.

A throwaway probe, then removed, accepted samples 3 at `786432`, accepted one byte per sample at `393216`, refused GPU bytes 10 against budget 9 with used 0 and `gpu_requested` 10 while `decoded_bytes` stayed `393216`, refused a zero decoded budget without clearing `decoded_bytes`, admitted a byte count equal to the budget, and refused zero columns, rows, slices, samples 0 and 4, and byte widths 0 and 3. On this host, `65536 * 65536 * 1 * 1 * 1 * 1` returned a profile with `decoded_bytes` `4294967296`. Tier C with a zero GPU figure returned a profile whose decoded product matched the same load off tier C, and GPU used was 0.

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|---|---|---|---|
| `git write-tree` and `git rev-parse HEAD` | Index before review | Executed now | Exit 0. Tree `97b9229d6321c769ca9828ed963ae0516d98ba77`. HEAD `b7a9992d403d0ff1defed8c367522750c3a66d78`. |
| `bin/ocelli.sh clippy ocelli-cache` after `touch` on `profile.rs` and `lru.rs` | Staged sources, all targets, `-D warnings` | Executed now | Exit 0. Cargo printed `Checking ocelli-cache`. |
| `bin/ocelli.sh test ocelli-cache --lib profile` | Restored staged `profile.rs` | Executed now | Exit 0. 5 passed. |
| `bin/ocelli.sh test ocelli-cache --lib would_admit_agrees_with_insert_at_the_budget_boundary` | Restored staged `lru.rs` | Executed now | Exit 0. 1 passed. |
| Same boundary test after `bytes <= budget` became `bytes < budget`, assert widened to the triple | Mutated worktree, index not updated | Executed now | Exit 101. Left `(false, false, false)` and right `(true, true, true)`. |
| Profile tests with the same `<` operator and the committed asserts | Mutated `lru.rs` only | Executed now | Exit 101. Large and 4D failed on `decoded_admitted`. |
| Profile tests after `try_from(product).ok()?` became `unwrap_or(0)` | Mutated `profile.rs` | Executed now | Exit 0. 5 passed. |
| Profile tests after an added `None` for `product > u64::from(u32::MAX)` | Mutated `profile.rs` | Executed now | Exit 0. 5 passed. |
| `a_large_volume_is_157286400_decoded_bytes` with `spec.columns` omitted | Mutated `profile.rs` | Executed now | Exit 101. Left `307200` and right `157286400`. |
| Profile tests with `spec.times` omitted | Mutated `profile.rs` | Executed now | Exit 101. 4D left `65536` and right `262144`. `times` 0 was no longer `None`. |
| Overflow test with `wrapping_mul` | Mutated `profile.rs` | Executed now | Exit 101. Panic at `profile.rs:241`. |
| Profile tests with `profile_load` forced to return nothing | Mutated `profile.rs` | Executed now | Exit 101. Large, 4D and one-byte-under left `0`. The tier C test passed. |
| Profile tests with GPU `used` always `spec.gpu_bytes` | Mutated `profile.rs` | Executed now | Exit 0. 5 passed. |
| Probe `reviewer_probe_factors_and_gpu_refusal` | Temporary test, then removed | Executed now | Exit 0. 1 passed. |
| `python3 scripts/prose_check.py --staged` | 8 staged prose paths | Executed now | Exit 0. |
| Python compare of the cache F-ID header with the last cell of the `cache.md` row | Staged text | Executed now | The strings are equal and both include F-113. |
| `git restore --worktree` on `lru.rs` and `profile.rs`, then `git diff` and `git write-tree` | Restored from the index | Executed now | No worktree diff. Tree still `97b9229d6321c769ca9828ed963ae0516d98ba77`. `rg` shows `bytes <= budget`. |

Environment: rustc 1.97.1. No wasm-pack was started. The floor gate and the corpus gate were not run. No compiler was running when this pass started.

## Defects

### D1, the usize fit check is not guarded

**Where**: `crates/ocelli-cache/src/profile.rs:84`. The overflow test is `profile.rs:237` through `profile.rs:241`.

**What**: `usize::try_from(product).ok()?` is the refusal for a product that does not fit in `usize`. Replacing it with `usize::try_from(product).unwrap_or(0)` left all five profile tests green. Inserting a `None` return for `product > u64::from(u32::MAX)` ahead of that line also left all five green. The second edit is wrong on this host. `4294967296` fits in `usize` here, and the removed probe observed a profile with that `decoded_bytes`. The committed suite never builds it.

**Why it is wrong**: The approved plan says a product that does not fit in `usize` returns nothing, and a product that does fit is the decoded byte count. Microscope section 4. The overflow test returns nothing from `checked_mul` of three `u32::MAX` dimensions before `try_from` runs. `u32::MAX` squared still fits in `u64` (`18446744065119617025`). The third factor overflows `u64`. That guards `checked_mul`. It does not guard `try_from`. On wasm32, `usize` is 32 bits, so `65536 * 65536` does not fit. On this host the same product does fit. Neither outcome is locked by the five tests.

**Evidence**: `bin/ocelli.sh test ocelli-cache --lib profile` exited 0 with 5 passed after each edit. The edits were restored. `git write-tree` stayed `97b9229d6321c769ca9828ed963ae0516d98ba77`. `try_from` returning `Err` was not observed. Every `u64` fits in `usize` on this 64-bit host, so that arm cannot run here.

**Scope and regression**: `profile_load` is the only producer of the product. Later cache callers would take the wrong `Option`. Present on the staged tree. The smallest regression is a committed load of 65536 by 65536 by 1 by 1 by 1 by 1 that expects a profile and `decoded_bytes` `4294967296` on this host, plus a 32-bit `usize` case that expects nothing. A native test cannot make `try_from` return `Err`. Uncertainty: no wasm32 run was started.

## Smells

None.

## Nitpicks

None.

## Verified clean

Pass 1 D1 is closed. `bin/ocelli.sh clippy ocelli-cache` is `--all-targets` and `-D warnings`. After `touch` on `profile.rs` and `lru.rs` it printed `Checking ocelli-cache` and exited 0. The test module has no `as` cast, no `unwrap`, no `expect` and no `panic!`. The factors of 1 are named consts, not a literal multiply by 1.

Pass 1 D2 is closed. `docs/sprints/AS_BUILT.md` line 4669 says a change to `bytes < budget` made `would_admit` false at the budget, the hand-written expected value is true, and insert refuses because it calls `would_admit`. The mutation above produced that result. The sentence is not the earlier false claim that insert still admitted the boundary. The same note says this tree checks the product with `usize::try_from` and states the refusal. Line 84 is `usize::try_from(product).ok()?`, which is that refusal in the source. The sentence does not say a test observed `Err`.

Pass 1 N1 is closed. `CHANGELOG.md` line 13 says a tier C load with a non-zero GPU figure returns nothing. A zero figure returns a profile and GPU used is 0. The probe and `tier_c_rejects_a_nonzero_gpu_figure_and_reports_zero_otherwise` agree with that split.

- AC1 and AC2. The literals `157286400` and `262144` are asserted against both a handwritten const product and `decoded_bytes`.
- AC3. Encoded used is the constant 0. The encoded tier is `CacheTier::Encoded`. The function builds no volume buffer.
- AC4. A decoded budget one byte under the product is not admitted. `decoded_bytes` stays `157286400`. Decoded used stays 0.
- AC5. `times` 0, samples 2, bytes per sample 4, and the `u32::MAX` product return nothing. `checked_mul` is what the overflow assert executes.
- AC6. Tier C with `gpu_bytes` 1 returns nothing. Tier C with 0 reports GPU used 0. The decoded product does not change between tier C and not tier C in the probe.
- AC7. `admits` is `budget > 0 && bytes <= budget` at `lru.rs:123`. `would_admit` calls it at line 286. `insert` calls `would_admit` at line 160. `profile_load` calls `admits` for decoded bytes and for the GPU figure. A search of `ocelli-cache` found that comparison once.
- Zero budget. The probe kept `decoded_bytes` at `262144`, set decoded used to 0, and set `gpu_admitted` false for GPU bytes 0 against budget 0.
- Equal budget. The 4D test and the strict-less mutation both show a byte count equal to the budget is admitted only while the operator is `<=`.
- Samples 3 and byte width 1 are accepted. Widths other than 1 or 2, and samples other than 1 or 3, return nothing in the probe. The committed test already refuses samples 2 and width 4.
- `profile.rs` does not name `std`. `lib.rs` keeps `#![cfg_attr(not(test), no_std)]`. No `unsafe` in the new file. No `wasm-bindgen` in `ocelli-cache`. No patient bytes are read.
- `docs/lld/cache.md` header and the last cell of the `cache.md` row in `docs/lld/README.md` are the same string and both include F-113.
- Tiers A and B are `tier_c` false and take the caller GPU figure. Tier C reports GPU used 0 or returns nothing. It does not publish a different decoded product.
- No JavaScript wrapper is in the staged diff. No deviation file is in the staged diff.
- `docs/sprints/SPRINT_TRACKER.md` records an admission-boundary mutation and does not claim a feature-gate receipt. The AS_BUILT verification paragraph is the same house template as F-112. It points at a later ledger entry. It is not a receipt for this pass.
- `python3 scripts/prose_check.py --staged` exited 0 over 8 files.
- Restored profile tests: 5 passed, exit 0. Restored boundary test: 1 passed, exit 0.

## Limitations and handoff

`try_from` returning `Err` was not exercised. It cannot return `Err` for any `u64` on this host. No wasm32 test was started. No wasm-pack run was started.

The five committed tests stay green if GPU `used` is always `spec.gpu_bytes`. That edit was made, the profile tests exited 0, and the edit was restored. The current arm is still the `gpu_admitted` check. The removed probe is what shows an over-budget figure keeps used at 0 while `decoded_bytes` stays the product. A later edit of that else arm would not turn the committed suite red. That gap was a pass 1 limitation and it is still open. It is not a second defect here because the probed behavior matches the plan.

The committed suite does not call samples 3. The probe did.

The tier C test's zero-GPU asserts match the zeros inside `demand`. Forcing every call to return nothing left that test green. The large-volume test uses the same input and went red, so the suite still fails.

The floor gate and the corpus gate were not run. This pass does not record a feature profile. The next owner is a regression that locks a product above `u32::MAX` on this host, and a 32-bit `usize` refusal if one can be executed, then a new independent pass.
