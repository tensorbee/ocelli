# S05 sprint review, pass 6

**Reviewed**: complete remediated sprint diff `7c5e29c..6bc09f2`
**Reviewer**: independent agent, did not write the sprint implementation or pass 5 remediation
**Result**: 2 defects, 2 smells, 0 nitpicks

## Defects

### D1, Green evidence still accepts measurements the producer cannot derive

**Where**: `scripts/verify_ledger.py:195-250,253-400,470-505`,
`tools/oracle/src/frame.rs:101-107,250-325,386-394` and
`tools/oracle/src/attribution.rs:1032-1117`

**What**: The pass 5 remediation checks many necessary relationships, but it
does not establish that the exact signed mean and percentile are attainable
from the published buckets. It also permits a measured class-one pass with no
informative pixels. Starting from the serializer-owned green contract or the
genuine report, all four independent mutations below were accepted:

```text
one absolute-one pixel with unattainable zero signed sum: ACCEPT
negative-zero derived signed means: ACCEPT
one over-two pixel with percentile below its exact maximum: ACCEPT
mono16 measured pass with no informative pixels: ACCEPT
```

The first report has one pixel in `countAtOne`, zero in every other bucket and
`signedMeanDiff: 0.0` in all three regions. Its only possible signed sum is 1 or
-1, never zero. The second replaces all derived zero signed means with `-0.0`.
The producer obtains zero from integer zero divided by a positive count, so it
emits positive zero. The third has one pixel in `countOverTwo`, maximum 255 and
the 99.9th percentile reported as 3. With one pixel the percentile rank is the
only pixel, so it must equal the maximum, 255. The fourth removes the
informative channel and sets the informative count and fraction to zero while
leaving an unqualified `mono16` pass.

**Why it is wrong**: `ChannelReport::of()` derives all of these values from one
integer histogram and signed integer sum. The verifier's current checks prove
only that `signed_mean * pixels` is an integer within `i32`, that its magnitude
does not exceed `maxAbsDiff`, and that a percentile in the over-two bucket lies
between 3 and the maximum. Those conditions are necessary but not sufficient.
At minimum, the exact one-code and two-code bucket counts bound and constrain
the parity of the signed sum. When the percentile rank consumes the sole
over-two pixel, the exact percentile equals the maximum.

For the fourth mutation, production has only two paths. An empty informative
region that was not declared low-information is refused by
`NoBiasDenominator`. A declared low-information record receives `weak` and is
`unmeasured`. It cannot be a measured pass. Accepting any of these mutations
allows edited evidence that the comparator cannot produce to receive a green
ledger attestation.

### D2, Relative report paths make validation depend on the ledger caller's directory

**Where**: `scripts/verify_ledger.py:527-535,560-563` and
`bin/ocelli.sh:10-12,540-570`

**What**: `_resolved_directory()` calls `Path(text).resolve(strict=True)`.
Python therefore resolves a relative path against the ledger process's current
working directory. The comparator entry point first changes to the repository
root and legitimately records paths such as `tools/oracle/out`, but the ledger
otherwise finds its repository, git tree and ledger file through its own
absolute `ROOT`.

**Evidence**: The unchanged genuine 99-record report was passed by absolute
pathname to the same absolute `verify_ledger.py` module. From the repository
root it was accepted. After changing only the caller's working directory to
`/private/tmp`, it was refused because the reference was looked up as
`/private/tmp/tools/oracle/out`:

```text
genuine 99-record report from repository root: ACCEPT
same genuine report from /private/tmp: REFUSE reference directory cannot be resolved
```

**Why it is wrong**: Caller location is not report evidence. The script already
has a canonical repository root and the documented comparator wrapper emits
relative paths from that root. A genuine report must not become malformed
because the same ledger command is launched through an absolute pathname from
another directory. This is the false-rejection half of the canonical path
repair.

## Smells

### S1, Fourteen new refusal branches have no targeted standing probe

**Where**: `scripts/verify_ledger.py:246-249,315-399,500-503,527-535` and
`scripts/guards/catalogue.py:1105-1195,7037`

The remediation added 21 distinct refusal messages and 11 semantic mutations.
Seven new messages have a direct mutation. Fourteen appear only at their
implementation site and nowhere in a standing test or probe:

- signed mean not pixel-derived
- full histogram, mean and maximum not equal to image plus background
- full region not equal to image when background is empty
- informative histogram and maximum exceeding image
- whole-image informative statistics disagreeing with image
- touched counts disagreeing with presence or number of differing pixels
- top signed mean disagreeing with its source region
- weak attribution whose informative fraction is not low
- an input path that cannot resolve or resolves to a non-directory

The guard census remains green because all refusal sites belong to the one
ledger guard entry, not because each branch was driven red. This is material in
this verifier. The unprobed exact signed-sum branch is adjacent to D1 and did
not catch it, and the unprobed path-resolution branch is the one producing D2's
false rejection. The report validation needs one controlled mutation per
semantic rule, with a green control that reaches the same branch family.

### S2, The contract closes most duplication but leaves two stale-authority routes

**Where**: `tools/oracle/report-contract.json`,
`tools/oracle/src/report.rs:772-932`,
`tools/oracle/src/attribution.rs:836-975` and
`scripts/verify_ledger.py:66-97,421-505`

The new contract materially resolves pass 5 S1 for serializer object keys,
typed enum labels, tolerance constants, hash domains and the single green
fixture. The Rust ratchet derives those from production and the Python verifier
consumes the same file.

Two parts remain hand-maintained. First, production rungs are string literals
in attribution and metadata. The Rust ratchet compares the contract to a second
hard-coded list of the same ten strings, rather than to a production type, so a
new emitted rung need not fail the ratchet. Second, exact class, outcome,
qualifier and rung relationships are Python logic. The Rust ratchet constructs
only the unqualified `mono16` pass used by `greenReport`. It does not serialize
each green unmeasured state and present those records to the Python verifier.
The current one-off 99-record run covers today's corpus combinations, but no
tracked cross-language test makes that protection persist after a producer
change.

The authority file's own containers are also open. In a disposable copy I
added an unused top-level `unusedAuthority` member and a duplicate `version`
member. `tracked_report_contract_matches_the_serializer` passed in both cases.
Both Rust and Python parse the contract with last-member-wins JSON behavior.
These additions do not change today's runtime result, but they let stale or
ambiguous authority-looking declarations accumulate without any consumer.
Closing the contract's own schema and deriving rungs from a production type
would make the one-authority claim exact.

## Nitpicks

None.

## Verified clean

- Every pass 5 reproduction is now refused: canonical input aliases, weak with
  a decimated rung, non-monochrome class one, class two changed to pass,
  class-one use of the class-two rung, maximum and percentile contradictions,
  a signed mean beyond the display maximum, predicate and bias contradictions,
  and counts above the producer's `u32` conversion limit.
- The unchanged genuine report contains 99 ordered records, 71 passes, 28
  unmeasured views, zero failures and zero absent views. It is accepted from the
  repository root. Both aggregate run hashes reproduce, so the remediation did
  not reject the current genuine report shape under its canonical invocation.
- The contract ratchet passed in the current tree. It checks the whole green
  fixture against `RunReport::to_json()`, all seven serialized object key sets,
  five typed vocabularies, four semantic constants, class channel counts,
  green-unmeasured qualifier vocabulary and both hash algorithms.
- The pass 4 deep boundaries remain closed for exact nested schemas, duplicate
  keys, non-finite numbers, record order and identifiers, outcome counts,
  qualifier histograms, per-view hashes and aggregate hash reconstruction.
- The pass 3 coverage repair and pass 2 array repair remain closed. All four
  coverage counts are required non-negative integers, both unmeasured copies
  agree, both absent copies are zero, and the three problem arrays are present
  and empty in green evidence.
- All six pass 1 remediations remain effective. All nine sprint commits carry
  both provenance trailers and matching trees. Mixed invalid input retains
  refusal precedence. Output cannot equal, contain or sit below either input.
  The F-014 regression stays bound to the live mutation, and CI proves all 26
  floor gates on its declared events.
- `bin/ocelli.sh test ocelli-oracle` passed 82 library tests, 9 comparator
  argument tests, 49 integration and fixture tests, and doc tests. This includes
  the new serializer-to-contract ratchet.
- `bin/ocelli.sh gate quirks quirk-mutations ci` exited 0. The quirk gate ran
  35 tests. All 3 controlled mutations and all 6 fixture-binding mutations went
  red at their fixed signatures. No F-014 defect or stale sprint-document claim
  was found.
- `bin/ocelli.sh gate guards` exited 0. Its census found 764 claimed refusals in
  65 files and 343 registered probes, with 435 refusals assigned to probed
  entries, 297 assigned to standing-test entries, 32 declared out of scope and
  zero watched by nothing. S1 explains why this entry-level result does not
  establish each new ledger branch.
- `bin/ocelli.sh gate corpus` verified all 92 manifest rows with zero missing or
  mismatched files. All nine commits in `7c5e29c..6bc09f2` passed
  `scripts/verify_ledger.py check-commit --require-corpus`.
- The independent `bin/ocelli.sh gate --floor` run reached the `panic` gate and
  was prevented by this sandbox from installing `wasm-opt`, reporting
  `Operation not permitted`. The Rust workspace tests and the subsequent
  deterministic gates shown above passed. The commit's recorded sprint profile
  covers every gate, including `panic`, but this review does not relabel the
  sandbox refusal as an independent pass.
- `git diff --check 7c5e29c..6bc09f2` passed. The 47-file sprint diff has 6,105
  insertions and 451 deletions. It adds no `unsafe`, confines `wasm-bindgen` to
  `ocelli-wasm`, changes no tolerance, and adds no render-loop or tier-specific
  execution path. No unrelated defect, smell or nitpick was found outside D1
  through S2.
