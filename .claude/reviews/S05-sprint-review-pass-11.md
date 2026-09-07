# S05 sprint review, pass 11

**Reviewed**: complete remediated sprint diff `7c5e29c..558277f`
**Reviewer**: independent agent, did not write the sprint implementation or pass 10 remediation
**Result**: 1 defect, 0 smells, 0 nitpicks

## Defects

### D1, Touched counts are not constrained by the region containing the differences

**Where**: `scripts/verify_ledger.py:477-501,547-557` and
`tools/oracle/src/frame.rs:634-683,705-712`

**What**: The pass 10 remediation proves that the frame and image dimensions
produce their reported pixel counts, that the image rectangle fits inside the
frame, and that touched rows and columns do not exceed the full-frame
dimensions. The remaining touched-count proof uses only the per-lane full
histogram counts. It does not account for whether the differences are confined
to the image rectangle or the background.

Starting from the serializer-owned green fixture, I constructed this exact
measured monochrome pass:

```text
frame:       2 rows by 2 columns, 4 pixels
image:       1 row by 2 columns, 2 pixels
image hist:  [[-1, 1], [1, 1]]
background:  [[0, 2]]
full hist:   [[-1, 1], [0, 2], [1, 1]]
informative: [[-1, 1], [1, 1]]
touched:     2 rows, 2 columns
```

The two opposite differences give exact zero informative bias. The class-one
predicate, every derived channel field, all region totals, all four dimension
products and both aggregate hashes were consistent. The production evidence
reader accepted the report:

```text
differences confined to one image row but two rows touched:
ACCEPT {'claimedVerdictViews': 1, 'verdict': 'pass'}
```

**Why it is wrong**: The background contains no difference, so every touched
pixel must lie in the one-row image rectangle. The producer marks touched rows
from the same pixel loop that partitions image and background. It can therefore
emit exactly one touched row for this distribution, never two. Summing
per-lane difference counts permits two and loses the spatial constraint.

The validator must couple touched counts to the region histograms and their
dimensions. At minimum, when one region has no differences, touched rows and
columns cannot exceed the other region's row and column extent. A complete
coexistence proof may need the image rectangle position or equivalent
region-specific spatial evidence. This boundary also needs a standing refusal
probe.

## Smells

None.

## Nitpicks

None.

## Data-format and implementation consistency

- Rust carries `frame_rows`, `frame_columns`, `image_rows` and `image_columns`
  from the actual frame and rectangle through `FrameDifference`,
  `ViewStatistics` and JSON serialization. The contract declares the matching
  four camel-case fields, and the serializer ratchet closes the statistics
  object around them.
- The validator requires every dimension to be a positive `u32`, checks both
  pixel-count products exactly, and requires the image extents to fit within
  the frame extents. Python integer multiplication cannot overflow. These
  checks reject the prime-sized pass 10 reproduction and do not narrow any
  report the Rust producer can serialize.
- The pass 10 empty-informative case is closed. The nonzero-bin comparison now
  runs for every image channel even when the informative array is empty. The
  exact reproduction is refused with `informative signed histogram omits image
  differences`.
- Sparse signed histograms remain closed, sorted and bounded from -255 through
  255. Counts, totals, summaries, maximum, percentile and signed mean are
  derived from the same distribution, with the producer's `u32` count and
  `i32` signed-sum limits enforced.
- Full histograms remain the exact sum of image and background. Informative
  nonzero bins match image bins for monochrome and all three RGB lanes. Shared
  informative totals make each lane's omitted zero count consistent with one
  common pixel selection.
- The tracked contract fixture includes the four new fields. The catalogue
  adjusts earlier multi-pixel mutations to valid dimensions and directly
  probes zero-informative differences, invalid dimension products and touched
  counts beyond the frame. D1 is a distinct region-placement boundary not
  covered by those probes.
- The LLD and AS_BUILT descriptions agree with the implemented fields and the
  exact frame-level checks. No stale field name, type, bound or serializer
  path was found.

## Prior-finding closure and verification

- Both pass 10 findings are closed at their exact reproductions. D1 above is a
  separate coexistence condition exposed after dimensions made region geometry
  explicit.
- The pass 9 informative omission, pass 8 exact distribution and helper drift,
  pass 7 tail and production state, pass 6 path and contract, and all earlier
  report-shape, coverage, quirk and provenance repairs remain closed.
- The genuine dimensional report contains 99 records, 71 measured passes, 28
  unmeasured records, zero failures and zero absent records. The current
  evidence reader accepts it with 71 claimed verdict views.
- `bin/ocelli.sh test ocelli-oracle` passed 144 tests across the library,
  comparator binary and integration suites.
- `bin/ocelli.sh gate guards-deep` passed 407 registered probes. Its census
  found 782 claimed refusals and zero watched by nothing.
- Commit `558277f` passed `check-commit --require-corpus` with a matching tree.
  `git diff --check 7c5e29c..558277f` passed.
- No additional defect, smell or nitpick was found outside D1.
