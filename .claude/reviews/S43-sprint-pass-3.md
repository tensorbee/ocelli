# S43 review, pass 3

**Reviewed**: staged tree `7961c5d32a1ee1ec6da110828324be5c2ec504b3` on `sprint/s43`. That is HEAD `fa655ed` plus the pass 1 and pass 2 remediation.
**Author / reviewer**: implementation by the sprint host. Review by Claude Code, read only, with no shell.
**Result**: 0 defects, 0 smells, 3 nitpicks

## Acceptance challenged

- **Pass 2 D1, volumes LLD and the F-197 count.** Fixed. `docs/lld/volumes.md` says the stored step is the longest, equal lengths keep the earlier step, a later step must agree with the first within 1e-6 mm, and the longest step at or under 1e-6 mm is `OrthogonalStack`. In `prepare`, the longest-step test uses strict `>`, so a tie keeps the earlier step. Disagreement is still measured against the first step. The bound check reads the longest step. `LATERAL_BOUND_MM` is still `1e-6`. `tests/tilted.rs` has eleven `#[test]` attributes, and the F-197 AS_BUILT line says eleven.
- **Pass 2 D2, key-object omissions.** Fixed. `docs/lld/key-object.md` splits the list by writer. The secondary-capture writer stores Instance Number 1, and the LLD says it does. The key-object dataset has no Instance Number, Series Number, Content Date, Content Time or evidence sequence, and the LLD lists those omissions. Neither writer gained a tag. The sentence does not claim to be a complete inventory.
- **Pass 2 S1, F-202 allocation.** Fixed. `allocation.json` has F-202, phase P1, 4 weeks, an empty milestone and an empty sprint. The story text matches the backlog row. It is not implemented. M19 stays 11 stories and 43 weeks. Planned F-IDs stay 191 because F-202 has no sprint.
- **Initialiser-throw sentence.** The AS_BUILT sprint-review paragraph puts the throw in the initialiser and names `packages/core/src/load-retry.test.ts`. That test throws synchronously, expects false, then expects a second load to succeed.
- **Phase 1 row count.** The backlog sentence now says F-202 is a Phase 1 row in that table. It does not say it is the only one.

## Evidence

| Check | Source | Executed now / prior receipt | Observation |
|---|---|---|---|
| `gate --sprint` | tree 7961c5d3 | prior receipt | ALL GREEN, 31 gates |
| compare gate | candidate 2bf5c039, reference e117f11b | prior receipt | 102 views, 71 pass, 0 fail, 31 unmeasured, verdict pass |
| `gen_sprint_plan.py --check` | tree 7961c5d3 | prior receipt | exit 0, 191 planned F-IDs |
| `backlog_check.py` | tree 7961c5d3 | prior receipt | exit 0, 225 F-IDs, 147 done |
| allocation one-liner | tree 7961c5d3 | prior receipt | P1 M19 is 11 stories and 43 weeks. P1 with a blank milestone is 1 story and 4 weeks |
| `prepare` | `crates/ocelli-volume/src/lib.rs` | read by the reviewer | strict `>` for the longest step, agreement against the first step |
| test count | `crates/ocelli-volume/tests/tilted.rs` | read by the reviewer | 11 tests |
| F-202 outside docs | repository grep | read by the reviewer | no implementation |

## Defects

None.

## Smells

None.

## Nitpicks

1. The longest-step sentence does not say the agreement check runs first. A longest step of exactly 1e-6 mm can still be `InconsistentShear` when the steps disagree by more than 1e-6 mm. `Volume::assemble` still accepts that stack. The same ordering gap was in the pre-remediation wording.
2. "The one-liner counts it with the other unscheduled rows" is loose. The one-liner keys on phase and milestone, so F-202 prints on its own `P1` blank-milestone line.
3. The F-198 "Tests added" line still lists the feature-commit tests. The sprint-review paragraph names `load-retry.test.ts`, and that sentence is true.

## Verified clean

- **Tolerances.** `LATERAL_BOUND_MM` is `1e-6`. The duplicate bound and the 0.5 percent spacing factor are unchanged.
- **Ledger agreement.** The F-202 story text matches. F-198's weeks and the M19 totals did not move.
- **Prose.** The new sentences have no em dash and no prose semicolon.
- **`unsafe` and wasm-bindgen.** The remediation adds neither.

## Limitations and handoff

The reviewer ran nothing. The gate, compare and script results are host receipts on `7961c5d3`. PS3.3 was not opened. Pass 1 nitpicks, the production `/@fs` URL and the empty failed-load tier line were not reopened. This pass meets the microscope stop condition.
