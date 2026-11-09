# F-239 JPEG class-two observation repair, pass 1

**Authors:** local GLM wrote the test delta. Codex participant glm_codec_merge applied formatting and two fallible test-vector accesses and proposed the explicit test policy. Raw model and separate controller alternatives are retained.
**Reviewer:** Codex root, who wrote neither the test delta nor the initial policy proposal.
**Result:** 0 defects, 0 smells, 0 nitpicks in that bounded proposal. Root's subsequent adoption wording and added CI observation step receive separate peer review.

The exact reviewed proposal is private patch `1806fc26e0b163cbe283cc4d5380217470e6f521bb4fda485a3efeca41d0129e`. Test source SHA256 is `7a84a4eea1dfae8d1475952ff352eff17c56eb317b9efbf49d9bd626ce8d9015`. Shipping code, features, fixture bytes, lossless assertions, monochrome tolerances and the existing statistics helper are unchanged.

The root accepts the explicit D-16 test policy. The two lossy-colour decodes now report measured per-channel differences as unmeasured observations. They no longer require one architecture's full aggregate vector on every target. This is a reviewed assertion change, not a new quality threshold or a quality pass. Existing local and Ubuntu vectors remain bound to source 8fdee914 and their actual receipts. The pinned decoder can select distinct scalar and SSSE3 implementations. That source fact does not establish which operation caused the observed differences.

The root independently counted both fixed hand vectors. Differences `(1,-2)`, `(-2,0)` and `(0,5)` give the three stated signed sums, counts, maxima and percentiles. With 1000 samples, 998 zero differences followed by 1 and 255 have nearest-rank 99.9 percentile 1 and maximum 255. Malformed lengths still refuse. These controls validate reporter arithmetic, not JPEG pixel quality.

Actual fresh-target JPEG tests pass 13 cases and registry tests pass 19. The root inspected all six recorded mutant failure assertions: percentile boundary, signed direction, malformed length, RGB declaration, sample layout and an exact lossless sample. Each exits 101 at its intended assertion. Restored JPEG tests pass 13. An additional arbitrary valid lossy RGB-byte corruption still passes these observation tests while changing the report. That demonstrated limitation is retained, not called decoder correctness.

Primary records are `/private/tmp/ocelli-s46-codex/jpeg-ci-class-two-repair/FINAL-HANDOFF.json`, `final-permanent-mutants.json`, `lossy-colour-mutation-limitation.json` and their named direct logs. The raw model's formatting and indexing lint failures and the controller's initially invalid reused-target receipt remain recorded. Fresh separate-target evidence replaces that invalid source binding. No Linux rerun or successful overall CI result is claimed here. The final staged feature profile remains required.
