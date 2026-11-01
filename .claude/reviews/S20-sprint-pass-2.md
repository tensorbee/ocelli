# S20 whole-sprint review, pass 2

**Reviewed**: `sprint/s20` at `d198eed9ddd84425ba4781854b476a5ee1a480ca`. `git rev-parse HEAD^{tree}` is `80852061458a85dde9a17619a4422fe760626b73`, the pass 1 tree. `git write-tree` is `e10452f6842ae42c32ccb5cabb910f106c7bb741`, the index, which adds the staged pass 1 review and this remediation. This review file is outside that tree. The remediation is the uncommitted diff of `scripts/guards/catalogue.py`, `scripts/lint_policy_check.py` and `docs/runbooks/guard-verification.md`. Pass 1 is not reopened.
**Author / reviewer**: independent. This reviewer did not write the delta.
**Result**: 0 defects, 0 smells, 0 nitpicks

## Acceptance challenged

Expecting `2 `#[path]` module(s) followed` proves the planted module was followed. It does not excuse a skip.

`python3 scripts/lint_policy_check.py` on this tree exits 0. The OK line carries `1 `#[path]` module(s) followed`, 188 `.rs` files and 14 workspace members. That line does not contain the new fragment. `tools/oracle/src/produce.rs` is the only `#[path]` in the walked set, and it names `produce_current.rs`. The four `#[path]` attributes under `vendor/ritk-codecs-0.6.0` are not in the walk. `Cargo.toml` excludes that crate, and `cargo metadata --no-deps` reports 14 members.

Each accept probe plants one module and nothing else. `_a_crate_root` resolves to `crates/ocelli-cache/src/lib.rs`. The plain probe appends `#[path = "..."]`. The `cfg_attr` probe appends `#[cfg_attr(all(), path = "...")]`. Both write `probe-shared/shared.rs`, which is outside every member, so the member glob does not already see it. The planted source is `pub fn probe(x: i64) -> i64 { x + 1 }`, which has no further `#[path]`. In a `git ls-files` sandbox, each plant moves the OK line to `2 `#[path]` module(s) followed` and the scanned set from 188 files to 189. The fragment occurs once. The old fragment `1 `#[path]` module(s) followed` is not a substring of that line.

A skip was then forced in the same sandbox. After the plant, `MODULE_PATH` was narrowed to `(produce_current\.rs)`, so the guard can still follow `produce` and cannot see the planted path. Both shapes stay at exit 0. The OK line stays `1 `#[path]` module(s) followed`. The scanned set stays 188, so the planted file was not read. `2 `#[path]` module(s) followed` is absent. `scripts/guard_probe.py` treats a missing fragment as a probe failure (`probe.expect not in output`). The old fragment is still present, which is why an expect of 1 stayed green when the plant was skipped, and why 2 does not.

The harness matches by substring. A rendering of 12, 22 or 32 also contains `2 `#[path]` module(s) followed`. The measured plant count is 2. A skipped plant stays at 1, and 1 is not one of those collisions. The same matcher is what the expect of 1 already used. It does not excuse this skip.

The runbook regeneration changed only those two cells. `git diff HEAD --numstat` is `2 2` for `docs/runbooks/guard-verification.md`. The lines are row 411, `lint-policy.clean-module-outside-the-member`, and row 414, `lint-policy.clean-cfg-attr-module-path`. Both replace the carried fragment `1 `#[path]` module(s) followed` with `2 `#[path]` module(s) followed`. No other row, limit or known-defect sentence moves. `python3 scripts/guard_census.py --check-runbook` exited 0, so the file matches a render of the catalogue.

No new defect or smell in the delta. The `member_sources` docstring now says this repository follows one `#[path]` module and no `include!`, which is the measured OK line (`0 `include!`(s) followed`). The two probe notes say `produce` is the first followed module and the plant is the second, which is the sandbox count above.

## Evidence

| Check | Identity | Executed now or prior receipt | Observation |
|---|---|---|---|
| `git rev-parse HEAD` and `git write-tree` | `sprint/s20` | executed now, before this file | commit `d198eed9ddd84425ba4781854b476a5ee1a480ca`, HEAD tree `80852061458a85dde9a17619a4422fe760626b73`, index tree `e10452f6842ae42c32ccb5cabb910f106c7bb741` |
| `git diff HEAD --numstat` | the three remediation files | executed now | runbook `2 2`, catalogue `7 4`, lint-policy `5 4` |
| `python3 scripts/lint_policy_check.py` | unmutated worktree | executed now | exit 0, 188 `.rs` files, 14 members, `1 `#[path]` module(s) followed`, new fragment absent |
| Plain plant in a `git ls-files` sandbox | `_a_clean_module_outside_the_member` | executed now | exit 0, 189 `.rs` files, fragment present once |
| `cfg_attr` plant in a fresh sandbox | `_a_clean_cfg_attr_module_outside_the_member` | executed now | exit 0, 189 `.rs` files, fragment present once |
| Both plants with `MODULE_PATH` narrowed to `produce_current.rs` | skip of the planted path only | executed now | exit 0, 188 `.rs` files, line stays `1 `#[path]` module(s) followed`, new fragment absent |
| Substring collision of the fragment | renderings 0 through 39 | executed now | hits at 2, 12, 22 and 32 only |
| `python3 scripts/guard_census.py --check-runbook` | generated probe table | executed now | exit 0 |
| Sprint gate | `bin/ocelli.sh gate --sprint` | not re-run | out of scope for this pass |

## Defects

None.

## Smells

None.

## Nitpicks

None.
