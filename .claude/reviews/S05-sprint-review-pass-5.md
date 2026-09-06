# S05 sprint review, pass 5

**Reviewed**: complete remediated sprint diff `7c5e29c..0368cfd`
**Reviewer**: independent agent, did not write the sprint implementation or pass 4 remediation
**Result**: 3 defects, 1 smell, 0 nitpicks

## Defects

### D1, Candidate evidence accepts two spellings of the same input directory

**Where**: `scripts/verify_ledger.py:422-425` and
`tools/oracle/src/bin/ocelli-compare.rs:152-176`

**What**: `comparison_evidence()` checks only that the two serialized path
strings differ. A report with `reference: "tools/oracle/out"` and
`candidate: "./tools/oracle/out"` is accepted even though both names resolve
to the same directory. A symlink gives the same result.

**Why it is wrong**: The explicit candidate gate defines independence by
resolved directory identity. Its command path canonicalizes both inputs and
refuses equality before reading or writing a report. The ledger is intended to
attest evidence produced by that command, but this accepted report cannot be
produced by it. Raw string inequality weakens the independence boundary into a
presentation check.

**Evidence**: Starting with the genuine 99-record report described below, I
changed only `candidate` from its distinct temporary directory to
`./tools/oracle/out`, while `reference` remained `tools/oracle/out`.
`comparison_evidence()` returned a passing attestation. The comparator's
standing `gate_refuses_two_spellings_of_the_same_directory` test passed in the
same tree, demonstrating the disagreement between producer and verifier.

### D2, Green record validation accepts combinations the comparator cannot emit

**Where**: `scripts/verify_ledger.py:310-379` and
`tools/oracle/src/attribution.rs:753-788,819-995`

**What**: The record validator checks each enum and a small subset of their
relationships, but it does not reconstruct the attribution ladder. Controlled
mutations accepted all of these green records:

- A record with only the `weak` qualifier and the `decimated` rung.
- A `mono16` record with `monochromeFrame: false`.
- A `colour-or-us` record changed from class-two `unmeasured` to an unqualified
  measured `pass`, with the summaries and qualifier histogram updated.
- A `mono16` weak record changed to `unstated-threshold` and the `class-two`
  rung, with the qualifier histogram updated.

**Why it is wrong**: The producer refuses a non-monochrome class-one frame.
It assigns `unstated-threshold` and the `class-two` rung only to class two, and
it never emits a measured pass for class two because HLD section 25.1 states no
threshold for it. The weak and decimated overlays also add the qualifier that
names their rung. These are facts the verifier has enough published fields to
check. Accepting contrary combinations lets an edited report relabel a judged
view as unmeasured or turn a deliberately unmeasured class-two view into a
claimed pass while retaining a green ledger attestation.

**Evidence**: Each mutation above was applied independently to the genuine
report. The changed outcome case also recomputed `pass`, `unmeasured`,
`claimedVerdictViews`, `coverage.unmeasured` and `qualifiers`, so the acceptance
was not caused by an unrelated stale summary. All four calls to
`comparison_evidence()` succeeded.

### D3, Green statistics can contradict their histogram and the written tolerance

**Where**: `scripts/verify_ledger.py:208-289,359-362`,
`tools/oracle/src/report.rs:201-251` and
`tools/oracle/src/attribution.rs:1032-1117`

**What**: The validator totals the four coarse histogram buckets and recomputes
two fractions, but it accepts statistics which are impossible or red under the
published predicate. Independent mutations accepted all of these cases:

- `maxAbsDiff: 255` with every pixel counted at zero.
- `percentile999AbsDiff: 255` when `maxAbsDiff` remained zero.
- A channel signed mean of 256 display codes.
- Top-level `signedMeanDiff: 1.0` with `biasPasses: true`.
- One pixel moved consistently into `countOverTwo`, with `maxAbsDiff: 3`, both
  fractions and the channel mean recomputed, while `predicatePasses` remained
  true.
- Informative and top-level signed means both changed consistently to 0.2 while
  `biasPasses` remained true.
- Region and histogram counts changed consistently to `2^32`, although the
  producer refuses counts above `u32` before computing these fractions.

**Why it is wrong**: HLD section 25.1 requires zero monochrome pixels over two
display codes and a signed informative mean within 0.1. The fifth and sixth
mutations are therefore red by the exact production rules, not merely unusual
data. `ChannelReport::of()` also derives maximum, percentile, counts,
fractions and mean from one `ChannelStats`, so the first three and seventh
forms cannot be serialized by the producer. Checking only that the boolean
flags are true attests the claim instead of checking the evidence published to
support it.

**Evidence**: The semantic matrix made 12 independently controlled impossible
or contradictory reports pass, including the four record and one path cases
above. Seven adjacent controls were correctly refused: an unknown nested
channel field, a per-view hash change without its aggregate update, a duplicate
record identifier, records out of serializer order, a duplicate nested JSON
key, nested `NaN`, and a nested overflow float. The failures are therefore at
specific semantic relationships, not at loading the genuine report or reaching
the deep validator.

## Smells

### S1, The report contract has two hand-maintained implementations and a third hand-maintained fixture

**Where**: `scripts/verify_ledger.py:65-114,208-507`,
`scripts/guards/catalogue.py:861-937` and
`tools/oracle/src/report.rs:30-400,650-685`

The Python verifier duplicates the Rust serializer's complete object schemas,
enum vocabularies, qualifier order, run-hash encoding and parts of the outcome
and tolerance semantics. The guard catalogue then duplicates a complete green
report by hand. Of those verifier constants, only the unrelated
`CORPUS_STATES` set is in the declared-constant ratchet at
`scripts/guards/catalogue.py:9647-9649`.

This increases both the number of cases a reader must reconcile and the number
of places required to answer what a valid report is, contrary to the repository
structural rule. D2 and D3 are present correctness drift today. A future Rust
field, enum or semantic change can also falsely reject a genuine report, or can
be copied into the green Python fixture while omitting the corresponding
negative rule. The 99-record happy-path test proves today's common shape but
does not bind the implementations. One mechanically checked contract authority
is needed for the schema and vocabularies, with cross-language generated-report
tests for the remaining semantic verification.

## Nitpicks

None.

## Verified clean

- A genuine report was generated by `bin/ocelli.sh compare gate` over two
  distinct current output directories. It contained 99 ordered records, 71
  passes, 28 unmeasured views, zero failures and zero absent views.
  `comparison_evidence()` accepted it and reproduced both aggregate run hashes.
  This found no false rejection of the current genuine report shape.
- Unknown members were injected independently into all 13 relevant locations:
  root, coverage, qualifier vocabulary, aggregate hashes, record, record hashes,
  statistics, each of the four channel-region shapes, parameter divergence and
  geometry divergence. All 13 were refused. Nine independent unknown enum or
  algorithm values were also refused for kind, class, outcome, side, rung,
  qualifier, both hash algorithm sites and parameter attribution side.
- The pass 4 boundaries remain closed for missing, null and empty records,
  duplicate identifiers, record ordering, outcome summaries, view and claimed
  counts, qualifier histograms, exact nested schemas, malformed hashes,
  aggregate run-hash reproduction, duplicate keys and non-finite JSON numbers.
  The remaining semantic gaps are D1 through D3 above.
- The pass 3 coverage matrix remains closed. Every coverage member is required
  as a non-negative integer, top-level and coverage unmeasured counts must
  agree, both absent counts must be zero, and record-derived outcome counts must
  match the summaries.
- The pass 2 problem-array repair remains closed. `problems`,
  `coverageProblems` and `absorbedDivergences` must be present empty arrays in a
  green report. The quirk registry's 26 focused contract tests passed, including
  the test that injects unknown fields at every schema object.
- All six pass 1 remediations remain effective. Every sprint commit carries a
  matching verification tree and both provenance trailers. A mixed input and
  pixel failure remains a refusal. The comparator refuses output equal to,
  below or containing an input and refuses two spellings of one input. The live
  quirk regression is bound to the attribution mutation. The CI checker names
  four floor exclusions and proved all 26 floor gates on its declared events.
- `bin/ocelli.sh gate quirks quirk-mutations ci` exited 0. The quirk gate ran 35
  tests. All 3 controlled mutations and all 6 fixture-binding mutations went
  red for their fixed signatures. The CI checker proved all 26 floor gates and
  the non-GPU excluded gates on pull requests, pushes and manual dispatches.
- `bin/ocelli.sh gate guards` exited 0. Its census found 743 claimed refusals in
  65 files, 332 registered probes, and 10 harness properties. The current
  candidate-report negative probes all went red, subject to the semantic gaps
  reported above.
- `bin/ocelli.sh gate --floor` completed green across all 26 floor gates. This
  includes formatting, clippy, the Rust and Python suites, content, provenance,
  prose and CI equivalence. `bin/ocelli.sh gate corpus` separately verified all
  92 manifest rows with zero missing or mismatched files.
- All eight commits in `7c5e29c..0368cfd` passed
  `scripts/verify_ledger.py check-commit --require-corpus`. The rewritten design
  commit still carries both required trailers and a matching tree.
- `git diff --check 7c5e29c..0368cfd` passed. The 44-file sprint diff has 5,292
  insertions and 448 deletions. It adds no `unsafe` use, keeps `wasm-bindgen`
  inside `ocelli-wasm`, changes no tolerance, and adds no render-loop or
  tier-specific execution path. No unrelated defect, smell or nitpick was found
  outside D1 through S1.
