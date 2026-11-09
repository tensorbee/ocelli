# F-225 AC35 evidence-stage amendment review, pass 1

**Author:** Codex participant f244_review prepared the independent counterfactual runs and plan appendix. The checker and original fixtures were written by earlier implementers.
**Reviewer:** Codex root, who wrote neither the checker, fixture expectations nor the proposed appendix.
**Result:** 0 defects, 0 smells, 0 nitpicks in this bounded evidence amendment.

The original pre-code progress receipt was not recovered by a bounded search. Its chronology remains unverified. The lead accepts fresh independent counterfactual execution for AC35's fixture-discrimination requirement, while preserving that historical limit. This changes the evidence stage only. It does not claim the original test-first event or change any expected finding, fixture, algorithm, tolerance or other acceptance requirement.

The root inspected the proposed patch, source and fixture identities, nine direct receipts and their actual failure assertions, plus the baseline and restored suite receipts. The five empty-checker cases fail at the named AS-FORMAT, VALUE-LENGTH, TYPE1-EMPTY, DA-FORMAT and UI-FORMAT assertions. The four separate faults fail at the specified byte-count, leap-year, last-value-pad and synthetic-secret checks. Every fault exits 101 with one semantic test failure. No compile or setup failure is counted as discrimination.

Original and restored full conformance suites pass 42 tests, direct exit 0. Restored checker SHA256 is `6ee2d61dea53791065ede025c6de191ad735feba2be6f301a8b6185f56a9ad44`. The root confirmed the acceptance identities against the actual retained receipts and the unchanged candidate checker and fixture. No product or test source changed in the candidate.

Primary records are `/private/tmp/ocelli-s46-codex/f225-ac35-counterfactual/fresh-results.json`, its named direct logs, `mutations.json`, `restoration.json` and the adjacent source identities. This review supplies no new browser, benchmark, CI or completion result. Other F-225 acceptance and final-tree verification remain separate.
