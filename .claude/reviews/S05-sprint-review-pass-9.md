# S05 sprint review, pass 9

**Reviewed**: complete remediated sprint diff `7c5e29c..65a69c2`
**Reviewer**: independent agent, did not write the sprint implementation or pass 8 remediation
**Result**: 1 defect, 0 smells, 0 nitpicks

## Defects

### D1, Informative histograms may omit differences that the producer must include

**Where**: `scripts/verify_ledger.py:507-518`,
`tools/oracle/src/frame.rs:587-591,634-673`,
`scripts/guards/catalogue.py:1568-1569` and
`docs/sprints/AS_BUILT.md:1940-1945`

**What**: The producer marks a pixel informative whenever any compared lane is
not equal at the same clipped extreme. A nonzero difference necessarily meets
that condition. It then adds every compared lane at that pixel to the
informative region. Therefore every nonzero signed-histogram entry in each
image channel must appear with exactly the same count in the corresponding
informative channel.

The verifier checks only that each informative count is no greater than the
image count at the same difference. That is necessary but not sufficient. It
allows a mandatory nonzero image sample to be omitted and another zero sample
to take its place.

Starting from `greenReport` in the tracked contract, I constructed a one-record
class-one report with these exact values:

```text
full and image:  pixels=1000, signedHistogram=[[0,999],[1,1]]
informative:     pixels=1,    signedHistogram=[[0,1]]
rowsTouched=1, columnsTouched=1
predicatePasses=true, biasPasses=true, signedMeanDiff=0.0
```

Every channel summary, fraction, maximum, percentile, region total, touched
count and hash was kept consistent with those distributions. Directly calling
the production evidence reader accepted it:

```text
{'reportSha256': '49366fc4306ef0bab9f9b01697c2fcd5ce965aa3ef7f60173d13f785546e6594',
 'claimedVerdictViews': 1, 'verdict': 'pass'}
```

No frame can produce that report. The `+1` sample is necessarily informative.
With `informativePixels` equal to one, the producer's informative histogram
must be `[[1,1]]`, its signed mean must be `1.0`, and the class-one bias bound
must fail. The forged histogram replaces that sample with a zero, reports a
zero mean, and turns the evidence green.

**Why it is wrong**: The ledger accepts evidence that contradicts the exact
pixel-selection rule used by its producer, and the contradiction can change a
failed class-one verdict into a pass. The AS_BUILT claim that distribution
composition is checked exactly across regions is therefore false for the
informative region. The two current informative probes cover an excess count
and excess maximum, but not omission of a mandatory nonzero sample. Validation
must require equality between image and informative counts for every nonzero
signed difference, with a standing refusal probe for that boundary.

## Smells

None.

## Nitpicks

None.

## Adversarial sparse-histogram audit

- The sparse channel shape is closed and bounded. Differences are strictly
  increasing integers from -255 through 255, so a channel has at most 511
  entries. Counts are positive `u32` values, totals must equal the channel
  pixel count, and the derived signed sum must fit the producer's `i32`
  conversion range. I found no sparse-array overflow or traversal-amplification
  path.
- Published buckets, fractions, maximum, percentile and signed mean are derived
  exactly from one signed distribution. The Python arithmetic matches the Rust
  producer because all accepted count and signed-sum operands are exactly
  representable in `f64`. I found no false refusal at the numeric boundaries.
- Full-frame histograms are checked as the exact coalesced sum of image and
  background histograms. Whole-image equality follows when background is
  empty. The defect above is the remaining cross-region composition gap.
- The contract requires `signedHistogram`, the Rust contract test compares the
  declared channel keys with serializer output, and the producer emits sorted
  nonzero pairs. The contract loader still refuses unknown and duplicate keys.
  I found no schema, vocabulary, hash-domain or serializer-drift defect.
- The catalogue directly probes malformed arrays, empty arrays, malformed
  pairs, invalid and unordered differences, invalid and zero counts, total and
  summary contradictions, producer-range overflow, maximum, percentile,
  signed mean, full composition, and informative excess. It has no probe for
  the mandatory-informative rule identified in D1.

## Prior-finding closure and verification

- Both pass 8 defects are closed. Sparse signed histograms make the signed mean
  an exact producer value and make percentile and signed sum share one exact
  distribution.
- Pass 8's smell is closed. Standing green-state probes obtain both their count
  and run-hash algorithm from `tools/oracle/report-contract.json`.
- The pass 7 tail, percentile and full-maximum cases, pass 6 path and state
  cases, pass 5 contract and attribution cases, and all earlier report-shape,
  coverage, quirk and provenance repairs remain closed under the current
  standing probes and tests.
- `bin/ocelli.sh test ocelli-oracle` passed 144 tests across the crate, binary
  and integration suites.
- `bin/ocelli.sh gate guards-deep` passed 355 refusal probes, 48 acceptance
  probes and 48 controls. `bin/ocelli.sh gate quirks quirk-mutations ci guards`
  also passed, including all nine live quirk mutations.
- `scripts/verify_ledger.py check-commit --require-corpus` passed separately for
  all 12 sprint commits from `2f0bcde` through `65a69c2`, with matching trees
  and corpus evidence. `git diff --check 7c5e29c..65a69c2` passed.
