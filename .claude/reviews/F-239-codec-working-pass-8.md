# F-239 review, pass 8, two merge repairs

**Reviewed**: staged tree `c7f7166b969c4d897dcc301acda19e20598502f4` on `work/f-239-codec`, HEAD `2414d639c274ac649e46f414417c792fe20e5f63`, merge still uncommitted. This bounded pass reviews only the post-GLM digest-row restoration and the inherited oracle prose correction. The earlier broad merge review is pass 7, not repeated here.
**Author / reviewer**: coordinator Codex participant `glm_codec_merge` restored the digest row, root Codex corrected the LLD sentence. Independent review by Codex participant `/root/f244_review`, which authored neither repair, the GLM merge, the prior report or the AC18 receipt.
**Result**: 0 defects, 0 smells, 0 nitpicks

## Acceptance challenged

- Pass 7 D1: the repaired JPEG Lossless SV1 digest row must equal both actual merge sources, not merely have a plausible length.
- The repair must change only that row in the submitted GLM digest table.
- The new LLD sentence must report the existing F-243 AC18 receipt accurately without claiming an oracle run on this merge tree.

## Evidence

Read-only equality checks used `git show` for the exact staged blobs and named source revisions. The check observations and command logs are under `/private/tmp/ocelli-f239-review-p8`.

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|--------------------------|---------------------------|------------------------------|----------------------|
| `git write-tree` | Actual index | now | 0, exact tree above |
| Complete keyed-row equality | Staged `corpus/decode-digests.tsv` against `2414d639` and `fd11a482` | now | Python exits 0, one matching key in each source, repaired complete row identical to both |
| Old versus repaired table | Submitted GLM tree `2c9462ee3527ce467719832f532c1b33390974d7` against staged table | now | Old file digest has 63 hex characters, repaired value has 64, replacing that one old row produces the staged file byte for byte |
| AC18 receipt equality and statement check | `fd11a482:.claude/plans/F-243-design.md`, current staged plan and LLD | now | Python exits 0, receipt sections equal and support the revised sentence |
| Expectation-file membership | Staged `tools/oracle/compare-expectations.json` | now | Neither banded JPEG-LS row has an entry, matching the receipt |
| `git diff --cached 2c9462ee3527ce467719832f532c1b33390974d7 --check` | Repair delta | now | 0 |
| `git diff --cached fe8030b4e30186dead78ecdbdd8524acf9aaed5b --check` | Later prose and report delta | now | 0 |
| `git diff --exit-code` | Actual unstaged worktree | now | 0 |
| Whole-merge `git diff --cached --check` | Merge against current HEAD | now | 2, trailing tabs in inherited manifest rows whose ninth field is empty, outside these repairs |
| Broad union checks and focused test suites | Pass 7's separately recorded repaired tree | prior receipt | Not rerun in this bounded pass |

The whole-merge whitespace diagnostic was inspected, not hidden. The affected manifest rows retain nine fields, like the header, with an empty final field. Neither reviewed repair changes `corpus/manifest.tsv`. No field was removed to silence that diagnostic.

## Defects

None found in these two repairs.

## Smells

None found in these two repairs.

## Nitpicks

None added by these two repairs. Pass 7's provenance-scope nitpick is unchanged and was not re-reviewed.

## Verified clean

- The complete `syntax/jpeg_lossless_p14_sv1.dcm` row, including both hashes, is byte-identical to the row in each actual merge source. This independently approves the coordinator's one-row repair. It does not rely on the coordinator's approval of their own change.
- Only one row changes between the submitted GLM digest table and the staged repaired table. The rest of the TSV is byte-identical.
- The LLD now attributes the oracle observation to F-243's AC18. The receipt in the staged plan is identical to the receipt in `fd11a482` and says its run was at `15d4817e`: 105 views, 74 pass, zero fail, 31 unmeasured, zero absent and 29 mutations detected. It explicitly says both banded rows passed without a qualifier and neither requires an expectation entry. These numbers are the historical receipt's, not measurements made during this review.
- The revised sentence correctly describes that receipt and removes the inherited false statement that the rows had not been rendered. It does not claim current-merge oracle verification.

## Limitations and handoff

- No browser, GPU, oracle, benchmark, wasm, cross-target, crate test, broad merge-union check, floor or sprint gate was run. The reviewed source delta is one restored metadata row and two lines of attributed prose.
- The broad merge coverage and tests remain pass 7's evidence. A clean pass on these repairs does not replace verification on the final integration tree or close external acceptance such as F-243 AC13 timing.
- No source remediation, staging or commit was performed by this reviewer. Root owns merge completion, remaining acceptance and final verification.
