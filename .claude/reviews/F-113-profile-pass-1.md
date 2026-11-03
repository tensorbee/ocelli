# F-113 review, pass 1

**Reviewed**: Staged index tree `489a2a85fb6f65f47ec5181a51b8bfacd7521009` on `sprint/s40`. HEAD is `b7a9992d403d0ff1defed8c367522750c3a66d78`. The review is `git diff --cached` only. `git write-tree` matched that hash at the start and again after the admission edit and the probe were restored. The worktree matches the index. This file is unstaged.

**Author / reviewer**: staged implementation / 01a1a0d7-dff8-70a3-b37b-b3bbd0604969 / independent pass 1. This reviewer did not write the change.

**Result**: 2 defects, 0 smells, 1 nitpick

## Acceptance challenged

AC1 to AC7 were checked against `.claude/plans/F-113-design.md` and HLD section 8. Section 8 names three tiers and says encoded bytes are dropped after decode. It does not give a 4D byte formula. The product and the sample limits are the approved local plan. They are not an HLD formula. No staged file adds a deviation.

The committed tests lock these cases.

- `512 * 512 * 300 * 1 * 1 * 2` equals the literal `157286400`. `profile_load` is then compared with that literal, not with its own return.
- `64 * 64 * 8 * 4 * 1 * 2` equals the literal `262144`. The function result is compared with that literal.
- Encoded used is 0.
- A decoded budget one byte under the product keeps `decoded_bytes` at `157286400` and sets decoded used to 0.
- `times` 0, samples 2, bytes per sample 4, and a product that overflows `u64` return nothing.
- Tier C with `gpu_bytes` 1 returns nothing. Tier C with 0 reports GPU used 0.
- `would_admit` is compared with the hardcoded boundary table, not with a second copy of the rule.

An independent probe was added, run, and removed. It is not in the staged tree. It checked samples 3 (`786432`), bytes per sample 1 (`393216`), a GPU figure of 10 against a budget of 9, a decoded budget of 0, zero columns, rows and slices, samples 0 and 4, byte widths 0 and 3, and a product of `4294967296` on this 64-bit host.

Changing `bytes <= budget` to `bytes < budget` made `would_admit_agrees_with_insert_at_the_budget_boundary` fail with `left: false` and `right: true`. A second look at the same mutation compared `(would_admit, refused.is_none())` and got `(false, false)` against `(true, true)`. Insert refused the boundary as well. The operator was restored. The restored test passed.

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|---|---|---|---|
| `git write-tree` and `git rev-parse HEAD` | Index before review | Executed now | Exit 0. Tree `489a2a85fb6f65f47ec5181a51b8bfacd7521009`. HEAD `b7a9992d403d0ff1defed8c367522750c3a66d78`. |
| `bin/ocelli.sh test ocelli-cache --lib -- profile` | Staged `profile.rs` | Executed now | Exit 0. 5 passed. |
| `bin/ocelli.sh test ocelli-cache --lib -- would_admit_agrees_with_insert_at_the_budget_boundary` | Staged `lru.rs` | Executed now | Exit 0. 1 passed. |
| Same boundary test after `bytes <= budget` became `bytes < budget` | Mutated worktree, index not updated | Executed now | Exit 101. Panic at `lru.rs:658`. `left: false` and `right: true`. |
| Same mutation with the assert reading `would_admit` and `refused.is_none()` | Same mutated worktree | Executed now | Exit 101. `left: (false, false)` and `right: (true, true)`. |
| Probe `reviewer_probe_factors_and_gpu_refusal` | Temporary test, then removed | Executed now | Exit 0. 1 passed. |
| `bin/ocelli.sh clippy ocelli-cache` | Staged sources, all targets, `-D warnings` | Executed now | Exit 101. 11 errors in `profile.rs` tests. |
| `cargo clippy -p ocelli-cache --lib -- -D warnings` | Restored non-test library | Executed now | Exit 0. |
| Restored boundary test | Index content | Executed now | Exit 0. 1 passed. |
| `python3 scripts/prose_check.py --staged` | 7 staged markdown paths | Executed now | Exit 0. |
| Python compare of the cache F-ID header with the last cell of the `cache.md` row | Staged text | Executed now | The strings are equal and both include F-113. |
| `git restore --worktree` on `lru.rs` and `profile.rs`, then `git diff` and `git write-tree` | Restored from the index | Executed now | No worktree diff. Tree still `489a2a85fb6f65f47ec5181a51b8bfacd7521009`. |

Environment: rustc 1.97.1. Clippy reported rustc 1.97.0 in its lint links. No wasm-pack was started. The floor gate and the corpus gate were not run.

## Defects

### D1, profile tests fail the clippy gate

**Where**: `crates/ocelli-cache/src/profile.rs:126`, `:128`, `:149`, `:162`, `:185`, `:213` and `:214`

**What**: `bin/ocelli.sh clippy ocelli-cache` exits 101. The test module trips `identity_op` on the written factors of 1, `unwrap_used` on four `unwrap` calls, and `cast_possible_truncation` on four `LARGE as usize` casts. The library target without tests exits 0.

**Why it is wrong**: `bin/ocelli.sh clippy` runs `--all-targets` with `-D warnings`. Workspace `Cargo.toml` denies `cast_possible_truncation` under HLD section 27.1 and denies `unwrap_used` under the section 23 panic policy. `157286400` fits in `usize` on wasm32 and on this host, but the `as` cast is the form that lint rejects. The five unit tests still pass, so the green suite does not see this.

**Evidence**: The clippy command printed 11 errors and `could not compile ocelli-cache (lib test)`. `cargo clippy -p ocelli-cache --lib -- -D warnings` exited 0 after the temporary test was removed.

**Scope and regression**: `profile_load` itself has no `as` cast and no `unwrap`. The floor clippy gate cannot pass on this tree. The smallest check is `bin/ocelli.sh clippy ocelli-cache`. Present on the staged tree.

### D2, the close note says insert still admitted the boundary

**Where**: `docs/sprints/AS_BUILT.md:4669`

**What**: The note says changing `bytes <= budget` to `bytes < budget` made `would_admit` false where insert still admitted the boundary. Under that edit, insert refuses the same boundary. `Lru::insert` calls `would_admit`, and `would_admit` calls `admits`, so the two cannot diverge.

**Why it is wrong**: The sentence is a false claim in the delivery record. The first assertion compares `would_admit` with the literal `true` in the table. That literal is not insert's result.

**Evidence**: With only the operator changed, the existing test panicked at `lru.rs:658` with `left: false` and `right: true`. Exit 101. With the same operator, the pair `(would_admit, refused.is_none())` was `(false, false)` against `(true, true)`. The source was restored. `git write-tree` stayed `489a2a85fb6f65f47ec5181a51b8bfacd7521009`. The restored boundary test passed.

**Scope and regression**: The admission function is not wrong. The note is. A later reader would look for a second comparison that this tree does not have. Present on the staged tree.

## Smells

None.

## Nitpicks

### N1, the changelog drops non-zero

**Where**: `CHANGELOG.md:13`

**What**: The new line says a tier C load with a GPU figure returns nothing. A zero figure on tier C returns a profile and reports GPU used 0. `docs/lld/cache.md` and the AS_BUILT tier row say non-zero.

**Why it does not block**: The function matches the plan. This is the only loose sentence. The LLD states the rule the code implements.

## Verified clean

- AC1. Decoded bytes for 512 by 512 by 300 by 1, one sample and two bytes, are `157286400`. The test multiplies those factors in a `u64` const, asserts that const against the literal, and asserts `decoded_bytes` against the same literal.
- AC2. The 64 by 64 by 8 by 4 case includes `times` as 4 and is `262144`. Dropping `times` would not meet that literal.
- AC3. Encoded used is the constant 0. The encoded tier is `CacheTier::Encoded`. The function does not build a volume buffer.
- AC4. Budget `157286399` is not admitted. `decoded_bytes` stays `157286400`. Decoded used stays 0. Encoded used stays 0.
- AC5. `times` 0, samples 2, bytes per sample 4, and columns, rows and slices of `u32::MAX` with `times` 2 return `None`. The product uses `checked_mul` on `u64`. `usize::try_from` is the second refusal, after that.
- AC6. Tier C with `gpu_bytes` 1 returns `None`. Tier C with 0 reports GPU used 0. Off tier C, the 4D test reports GPU used `262144` when that figure fits. The probe reported GPU used 0 and kept `gpu_requested` at 10 when the figure was over the GPU budget.
- AC7. `admits` is `budget > 0 && bytes <= budget` at `lru.rs:122`. `would_admit` calls it. `insert` calls `would_admit`. `profile_load` calls `admits` for the decoded tier and for the GPU tier. A search of `ocelli-cache` found no second `budget > 0 && bytes <= budget`.
- Samples other than 1 or 3 return `None`. The committed test uses 2. The probe also refused 0 and 4, and accepted 3 with the product multiplied by 3. Bytes per sample other than 1 or 2 return `None`. The probe accepted 1 and refused 0 and 3.
- A zero decoded budget returns a profile, keeps the product in `decoded_bytes`, and sets decoded used to 0. That is admission, not `None`.
- `profile.rs` does not name `std`. `lib.rs` still has `#![cfg_attr(not(test), no_std)]`. The non-test library passed clippy.
- The plan has `## LLD impact` naming `docs/lld/cache.md`. The F-ID header in that file equals the last cell of the cache row in `docs/lld/README.md`. Both include F-113.
- Tiers A and B take the caller GPU figure. Tier C reports GPU used 0 and refuses a non-zero figure. No deviation was added.
- `docs/sprints/AS_BUILT.md` uses the house verification paragraph. It does not claim a feature-gate receipt. `docs/sprints/SPRINT_TRACKER.md` records an admission-boundary mutation and does not claim staged feature verification.
- `python3 scripts/prose_check.py --staged` exited 0.

## Limitations and handoff

The committed suite does not call `profile_load` with samples 3 or with a GPU figure above its budget. The probe did, and it was removed. A later edit to the GPU `used` arm would not turn the five committed tests red.

`usize::try_from` was not observed returning `None`. On this 64-bit host, `65536 * 65536` is `4294967296` and the function returns `Some`. A wasm32 run was not started. No wasm-pack run was started.

The floor gate and the corpus gate were not run. This pass does not record a feature profile. The next owner is a repair of the clippy errors and of the AS_BUILT sentence, then a new independent pass.
