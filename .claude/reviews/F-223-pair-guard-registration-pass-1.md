# Pair-process refusal registration peer review

**Scope:** current bench.comparison-runtime catalogue entry, its single budget advance and the generated runbook lines. Author is the separate F244/Codex participant with root-owned census recording. Reviewer is Codex participant f225_review, who authored none of this registration or its runtime/Node controls. Own concurrency repair is excluded. No canonical or index write and no runtime command was performed for this review.

**Result:**0defects,0smells,0nitpicks.

Independent deep comparison of guard-budget-before-pair-process-site.json with current ci/guard-probe-budget.json found exactly entry_sites.bench.comparison-runtime:51to52. Every other field, including all51constant digests,36gates,88entry records, uncovered0and wall-clock settings, is unchanged. This is a measured one-site advance, not an unchecked ratchet reset.

The new site is readPairVerdict's explicit closed invalid-verdict refusal in tools/bench/src/comparison_run.mjs. The named Guard now declares malformed comparator verdicts and incoherent exit/output channels. covered_by still names the existing comparison_run_test.mjs and comparison_page_test.mjs. The former is registered in the bench gate at bin/ocelli.sh238, and contains three new permanent controls, not a detached manual-only helper:

- Closed seven-field parser cases cover malformed/unknown/duplicate keys, scalar domains and contradictory status.
- Actual disposable child comparators exercise valid exit0/1, including expected pixel refusal and low information, plus foreign stdout/stderr/signals and contradictory exit channels.
- The actual pixelPrecheck→child comparator→executeComparison→cleanup→runComparison path is retained, with only four browser/transport providers substituted. It proves valid exit1 produces a retained private refused_pixels report, zero timing calls, both actors and infrastructure closed, and the lock removed.

The recorded20-test restored suite is actual exit0. The old direct-exec integration mutant is actual1. Foreign-stderr and duplicate-key mutants are actual1at their intended missing-rejection assertions. These support real refusal/consumer coverage. They do not claim browser pixel parity or clinical-study acceptance. The runbook correctly keeps tiny real-release-comparator RGBA observations as separately recorded explicit checks outside the floor and preserves the pre-existing native-I/O and equipped-timing limitations.

The fresh record-own-exit wrapper directly captured guard_census.py --record exit0/.362830s. The earlier recording command's standalone status was not retained, so no verdict is inferred from a later proof. The next census actually failed1/.376414s specifically because the generated runbook was stale. Render-runbook then returned0/.102720s. Final census returned0/1.000812s. The generated diff is exactly two replaced lines: the corresponding refusal table row and its coverage paragraph. No adjacent hand prose, gate rule, uncovered ceiling, tolerance or other budget was relaxed.

Source-review read correction: one initial lookup used src/comparison_run_test.mjs and failed because that nonexistent path is wrong. The review then followed the Guard's actual covered_by path tools/bench/tests/comparison_run_test.mjs. That lookup was not a test execution or passing check.

Root may stage/verify the coherent three-path registration. This bounded review does not make the earlier failed CI green, certify the current final tree, approve the separate concurrency repair or complete S46.
