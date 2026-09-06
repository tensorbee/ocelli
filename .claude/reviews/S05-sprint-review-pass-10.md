# S05 sprint review, pass 10

**Reviewed**: complete remediated sprint diff `7c5e29c..07fc9e8`
**Reviewer**: independent agent, did not write the sprint implementation or pass 9 remediation
**Result**: 2 defects, 0 smells, 0 nitpicks

## Defects

### D1, An empty informative region bypasses the nonzero-bin invariant

**Where**: `scripts/verify_ledger.py:475-477,507-527`,
`tools/oracle/src/frame.rs:634-674`, `scripts/guards/catalogue.py:1285-1295`
and `docs/lld/comparator.md:300-306`

**What**: The pass 9 remediation checks that every nonzero image bin appears
unchanged in the informative histogram only inside
`if regions["informative"]`. When `informativePixels` is zero, the required
informative array is empty and the entire check is skipped.

Starting from the serializer-owned green contract fixture, I added a declared
green class-two record with one image pixel in every lane. Its full and image
histograms were `[[1, 1]]`, while `informative` was empty and
`informativePixels` was zero. All summaries and aggregate hashes were kept
consistent. The production evidence reader accepted it:

```text
zero informative region despite nonzero image differences:
ACCEPT {'claimedVerdictViews': 1, 'verdict': 'pass'}
```

**Why it is wrong**: A nonzero lane difference means the two values are not
equal, so `clipped_to_the_same_extreme()` is necessarily false. The producer
marks that pixel informative and adds every compared lane to the informative
region. Therefore any nonzero image bin makes an empty informative region
impossible. The new standing probe covers omission only when the informative
array is populated, so it misses the bypass. This contradicts the LLD statement
that every nonzero image bin must appear unchanged in the informative
histogram.

### D2, Touched dimensions need not fit any frame with the reported pixel count

**Where**: `scripts/verify_ledger.py:462-479,528-538` and
`tools/oracle/src/frame.rs:605-630,634-685`

**What**: The verifier bounds touched rows and columns independently by the
full-frame pixel count and difference counts. It never proves that the full
pixel count has a width and height capable of containing that many rows and
columns.

I constructed a green three-lane record with seven full and image pixels. Two
differing pixels account exactly for two touched rows and two touched columns,
and the shared informative selection and every channel histogram are otherwise
consistent. The production evidence reader accepted it:

```text
prime-sized frame with impossible touched dimensions:
ACCEPT {'claimedVerdictViews': 1, 'verdict': 'pass'}
```

Every rectangular seven-pixel frame is either 1 by 7 or 7 by 1. No such frame
can touch both two rows and two columns. The producer obtains `pixels` from
`width * height` and sizes its two touched arrays directly from those same
dimensions, so it cannot emit this record.

**Why it is wrong**: The ledger attests a report whose scalar bounds are
individually plausible but whose geometry cannot exist. It must either carry
and validate the producer dimensions or prove that some factor pair of the
reported full pixel count can accommodate both touched counts. The same
geometry must remain compatible with the image rectangle and region totals.

## Smells

None.

## Nitpicks

None.

## Sparse signed-histogram audit

- The pass 9 nonempty-region repair is correct for both monochrome and RGB.
  Any nonzero lane makes the common pixel informative, so the corresponding
  signed bin must be copied exactly. Once those bins and the shared informative
  pixel count agree, each lane's omitted zero count is also fixed to
  `imagePixels - informativePixels`.
- Signed differences are strictly increasing integers from -255 through 255.
  Counts are positive and bounded by `u32`, totals match the channel pixel
  count, and the exact signed sum must fit the producer's `i32` conversion.
  Python's integer accumulation has no overflow path here.
- Maximum, four coarse buckets, fractions, percentile and signed mean are all
  recomputed from the sparse histogram. The division operands are within the
  exact integer ranges used by the Rust producer, and the exact float equality
  did not produce a false refusal in the genuine report.
- Full histograms are the exact coalesced sum of image and background. When the
  background is empty, the entire full and image channel objects must match.
  I found no remaining composition or maximum drift in those regions.
- The contract closes the channel object around `signedHistogram`, the Rust
  serializer ratchet derives that key from production output, and standing
  probes cover malformed arrays and pairs, ordering, range, count, total,
  summary, overflow and composition refusals. D1 is the uncovered empty-array
  branch.
- The current genuine sparse report has 99 records, including five three-lane
  records and two zero-informative records. It is accepted unchanged by the
  current evidence reader.

## Prior-finding closure and verification

- Pass 9 D1 is closed for every populated informative region, but D1 above
  shows that its zero-pixel boundary remains open.
- Both pass 8 defects remain closed by exact sparse histograms. The pass 8
  contract-helper smell and pass 7 production-state smell remain closed.
- The pass 7 tail, percentile and full-maximum cases, pass 6 path and state
  cases, pass 5 contract and attribution cases, and all earlier report-shape,
  coverage, quirk and provenance repairs remain closed under their standing
  tests and probes.
- `bin/ocelli.sh test ocelli-oracle` passed 144 tests across the library,
  comparator binary and integration suites.
- `bin/ocelli.sh gate guards-deep` passed 404 registered probes. The focused
  `quirks`, `quirk-mutations`, `ci` and `guards` gates also passed.
- Commit `07fc9e8` passed `check-commit --require-corpus` with a matching tree,
  and `git diff --check 7c5e29c..07fc9e8` passed.
- No additional defect, smell or nitpick was found outside D1 and D2.
