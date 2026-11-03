# F-113 review, pass 3

**Reviewed**: Staged index tree `c210a6c7034bf112217a40f66fee7553bb9af3df` on `sprint/s40`. HEAD is `b7a9992d403d0ff1defed8c367522750c3a66d78`. The review is `git diff --cached` only. `git write-tree` printed that hash before review and again after every mutation and the probe were restored. `git diff` was empty after the restore. This file is untracked.

**Author / reviewer**: staged implementation / 01a1a0fa-ed27-76b2-9fb5-204ae311f92a / independent pass 3. This reviewer did not write the change.

**Result**: 0 defects, 0 smells, 0 nitpicks

## Acceptance challenged

AC1 to AC7 from `.claude/plans/F-113-design.md`. HLD section 8 names three tiers and says encoded bytes are dropped after decode. It does not give a 4D byte formula. The product and the sample limits are the approved local plan. They are not an HLD formula. No staged file adds a deviation.

Python computed the hand products without calling `profile_load`.

- `512 * 512 * 300 * 1 * 1 * 2` is `157286400`.
- `64 * 64 * 8 * 4 * 1 * 2` is `262144`.
- `64 * 64 * 8 * 4 * 3 * 2` is `786432`.
- `65536 * 65536` is `4294967296`, which is `2^32`.
- `4294967296 < 2^32` is false. `4294967296 <= 2^32` is true. `2^32 - 1 < 2^32` is true.

Changing `product < limit` to `product <= limit` made `a_65536_square_fits_a_wide_address_only` fail. The panic was `fits_address_bits(SQUARE, 32).is_none()` at `profile.rs:270`. Exit 101. The operator was restored.

Changing GPU `used` to `spec.gpu_bytes` made `three_samples_are_786432_and_an_over_budget_gpu_figure_is_unused` fail with left `10` and right `0` at `profile.rs:322`. Exit 101. The admitted arm was restored.

Refusing every product above `u32::MAX` before `fits_address_bits` made that same square test fail with left `0` and right `4294967296`. The other six profile tests stayed green. `demand` supplied the `0` because `profile_load` returned nothing. The direct helper asserts had already passed, so this failure is the 64-bit `profile_load` path. The edit was restored.

Replacing `usize::try_from(product).ok()?` with `unwrap_or(0)` left all seven profile tests green. That edit does not change a result once `fits_address_bits` has accepted the product. A 64-bit address holds every `u64`. A narrower address has already refused every product at or above two to the power of its width. The pass 2 hole was the missing guard. The guard is now `fits_address_bits`, and the `<=` mutation makes it fail.

A throwaway probe, then removed, loaded `8 * 8 * 2 * 2 * 1 * 1`. That product is `256`. A decoded budget of `0` kept `decoded_bytes` at `256`, set decoded used to `0`, and did not admit. A GPU budget of `0` with a figure of `0` did not admit, and GPU used was `0`. Encoded used was `0` and the encoded budget stayed `9`. Columns `0`, rows `0`, slices `0`, samples `0` and `4`, and byte widths `0` and `3` returned nothing. Tier C with a GPU figure of `4` returned nothing. Tier C with a figure of `0` and a budget of `256` reported GPU used `0`, admitted the decoded product, and set `gpu_admitted` true. That flag is `admits(256, 0)`, which is the one comparison. It is not a second GPU byte count.

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|---|---|---|---|
| `git write-tree` and `git rev-parse HEAD` | Index before review | Executed now | Exit 0. Tree `c210a6c7034bf112217a40f66fee7553bb9af3df`. HEAD `b7a9992d403d0ff1defed8c367522750c3a66d78`. |
| Python products and the cache F-ID header | Staged text, not `profile_load` | Executed now | `157286400`, `262144`, `786432`, and `4294967296`. The header and the `cache.md` cell are the same string. |
| `a_65536_square_fits_a_wide_address_only` after `product < limit` became `product <= limit` | Mutated worktree, index not updated | Executed now | Exit 101. Panic at `profile.rs:270`. `fits_address_bits(SQUARE, 32).is_none()` failed. |
| `three_samples_are_786432_and_an_over_budget_gpu_figure_is_unused` after GPU used became `spec.gpu_bytes` | Mutated worktree, index not updated | Executed now | Exit 101. Left `10` and right `0` at `profile.rs:322`. |
| `profile::` after an early `None` for `product > u32::MAX` | Mutated worktree, index not updated | Executed now | Exit 101. 6 passed. The square test left `0` and right `4294967296`. |
| `profile::` after `try_from` became `unwrap_or(0)` | Mutated worktree, index not updated | Executed now | Exit 0. 7 passed. |
| Probe `reviewer_probe_narrow_factors_and_zero_budget` | Temporary test, then removed | Executed now | Exit 0. 1 passed. |
| `bin/ocelli.sh clippy ocelli-cache` | Restored staged sources, all targets, `-D warnings` | Executed now | Exit 0. Cargo printed `Checking ocelli-cache`. |
| `bin/ocelli.sh test ocelli-cache profile::` | Restored staged tree | Executed now | Exit 0. 7 passed in the library. Integration filters ran 0 tests. |
| `bin/ocelli.sh test ocelli-cache --lib admit` | Restored staged `lru.rs` and `profile.rs` | Executed now | Exit 0. 8 passed, including the budget-boundary test and the zero-budget cache test. |
| `python3 scripts/prose_check.py --staged` | 9 staged prose paths. `CHANGELOG.md` is outside that checker. | Executed now | Exit 0. |
| `git restore --worktree` on `profile.rs`, then `git diff` and `git write-tree` | Restored from the index | Executed now | No worktree diff. Tree still `c210a6c7034bf112217a40f66fee7553bb9af3df`. |

Environment: rustc 1.97.1. No wasm-pack was started. The floor gate and the corpus gate were not run. No compiler was running when this pass started.

## Defects

None.

## Smells

None.

## Nitpicks

None.

## Verified clean

Pass 1 D1 is closed. `bin/ocelli.sh clippy ocelli-cache` is `--all-targets` and `-D warnings`. It printed `Checking ocelli-cache` and exited 0. The test module has no `as` cast, no `unwrap`, no `expect` and no `panic!`. The factors of 1 are named consts.

Pass 1 D2 is closed. `docs/sprints/AS_BUILT.md` says a change to `bytes < budget` made `would_admit` false at the budget, the hand-written expected value is true, and insert refuses because it calls `would_admit`. This pass did not repeat that operator edit. `Lru::insert` still calls `would_admit` at `lru.rs:160`. `would_admit` calls `admits` at `lru.rs:286`. The restored boundary test passed.

Pass 1 N1 is closed. `CHANGELOG.md` says a tier C load with a non-zero GPU figure returns nothing. A zero figure returns a profile and GPU used is 0.

Pass 2 D1 is closed. `fits_address_bits` refuses a product at or above two to the power of a width below 64, and a width of 64 or more returns the product. `65536 * 65536` is `4294967296`. The `<=` mutation made the 32-bit helper assert fail. The `u32::MAX` cap made the 64-bit `profile_load` assert fail. The note in `AS_BUILT.md` names pass 2, reviewer `01a1a0e8-d38d-7d51-8286-d9c3267cf336`, and the products `4294967296` and `786432`. Those products match the Python results and the restored tests.

- AC1 and AC2. The literals `157286400` and `262144` are asserted against both a handwritten const product and `decoded_bytes`.
- AC3. Encoded used is the constant 0. The encoded tier is `CacheTier::Encoded`. The function builds no volume buffer.
- AC4. A decoded budget one byte under the product is not admitted. `decoded_bytes` stays `157286400`. Decoded used stays 0.
- AC5. `times` 0, samples 2, bytes per sample 4, and the `u32::MAX` product return nothing. `checked_mul` is what the overflow assert executes. The zero check in `checked_product` also refused columns, rows and slices of 0 in the probe.
- AC6. Tier C with `gpu_bytes` 1 returns nothing. Tier C with 0 reports GPU used 0. The probe kept the decoded product at `256` for that zero figure.
- AC7. `admits` is `budget > 0 && bytes <= budget` at `lru.rs:123`. A search of `ocelli-cache` found that comparison once. `would_admit` calls it. `insert` calls `would_admit`. `profile_load` calls `admits` for decoded bytes and for the GPU figure. The boundary test and the zero-budget cache test passed.
- Samples 3 are `786432`. The GPU figure `10` against budget `9` keeps `gpu_requested` at `10` and GPU used at `0`. The unconditional used arm produced left `10` and right `0`.
- Bytes per sample of 1 are in the square product and in the probe product `256`. A hardcoded width of 2 would miss both.
- A zero budget admits nothing. The probe showed that for both the decoded tier and the GPU tier. A byte count equal to the budget is admitted by the large test, the 4D test and the probe.
- `profile.rs` does not name `std`. `lib.rs` keeps `#![cfg_attr(not(test), no_std)]`. No `unsafe` in the new file. No `wasm-bindgen` in `ocelli-cache`. No patient bytes are read. No JavaScript wrapper is in the staged diff.
- `docs/lld/cache.md` says a 65536 by 65536 frame is `4294967296` bytes, that it fits a 64-bit address and does not fit a 32-bit address, that encoded used is 0, and that a GPU figure above its budget leaves GPU used at 0. The source and the mutations agree. The F-ID header equals the last cell of the `cache.md` row in `docs/lld/README.md`.
- Tiers A and B are `tier_c` false and take the caller GPU figure, with used set to 0 when `admits` is false. Tier C reports GPU used 0 or returns nothing. It does not publish a different decoded product.
- Seven `#[test]` functions are in `profile.rs` after the probe was removed.
- `docs/sprints/SPRINT_TRACKER.md` records an admission-boundary mutation and does not claim a feature-gate receipt. The AS_BUILT verification paragraph is the same house template as F-112. It points at a later ledger entry. It is not a receipt for this pass.
- `python3 scripts/prose_check.py --staged` exited 0 over 9 files.
- Restored profile tests: 7 passed, exit 0. Restored admission filter: 8 passed, exit 0.

## Limitations and handoff

This host ran the `usize::BITS >= 64` arm of the square test. The `else` arm, which expects `profile_load` to return nothing, was not executed. No wasm32 run was started. No wasm-pack run was started.

`usize::try_from` returning `Err` was not observed. It cannot return `Err` for a product that `fits_address_bits` has accepted. Replacing it with `unwrap_or(0)` left the seven tests green for that reason. The edit was restored.

The tier C test's zero-GPU asserts match the zeros inside `demand`. The large-volume test uses the same input and checks `decoded_bytes` against `157286400`, so a forced `None` still fails the suite. The committed tests do not read `gpu_admitted` on tier C. The removed probe did. With a positive budget and a zero figure that flag is true, and GPU used is 0.

The floor gate and the corpus gate were not run. This pass does not record a feature profile. The comparison operator inside `admits` was not mutated again. No code change is waiting on this pass.
