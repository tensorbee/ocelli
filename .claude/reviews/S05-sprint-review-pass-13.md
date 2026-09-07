# S05 sprint review, pass 13

**Reviewed**: complete remediated sprint diff `7c5e29c..a215924`
**Reviewer**: independent agent, did not write the sprint implementation or pass 12 remediation
**Result**: 2 defects, 1 smell, 0 nitpicks

## Defects

### D1, Image touched extent can exceed the informative pixel set

**Where**: `scripts/verify_ledger.py:576-600,627-678` and
`tools/oracle/src/frame.rs:654-686,691-706`

**What**: Every image pixel carrying a nonzero lane difference is necessarily
informative. The union of differing image pixels must therefore fit within
`informativePixels`. The regional feasibility calculation instead caps that
union only by the Cartesian touched cells and the sum of per-lane difference
counts.

Starting from the serializer-owned green fixture, I added a class-two RGB
record with a 2 by 2 full-frame image. Each lane had one `+1` difference and
three zero differences. Each informative histogram contained its required one
`+1` sample, and `informativePixels` was one. The image touched arrays named
rows `[0, 1]` and columns `[0, 1]`. All summaries, regions, dimensions, verdicts
and aggregate hashes were otherwise consistent. The evidence reader accepted
it:

```text
one informative pixel but two rows and columns touched:
ACCEPT {'claimedVerdictViews': 1, 'verdict': 'pass'}
```

**Why it is wrong**: With one informative pixel, the three lane differences
must all occupy that same pixel. One pixel can touch only one row and one
column. Covering the reported two rows and two columns needs at least two
distinct differing pixels. The validator computes that minimum correctly, but
then compares it with `sum(channel_differences) == 3` rather than also capping
the image union by `informativePixels`. The Rust producer cannot emit this
record.

### D2, Monochrome RGB lane supports are treated as independently placeable

**Where**: `scripts/verify_ledger.py:602-608,627-678` and
`tools/oracle/src/attribution.rs:751-754,790-793`

**What**: The pass 12 remediation correctly requires identical RGB channel
reports when `monochromeFrame` is true. The spatial feasibility calculation
still sums their difference counts as if the three lanes could differ at
different pixels.

I isolated this from D1 by using the same 2 by 2 class-two record with
`monochromeFrame: true`, four informative pixels, and identical full, image and
informative channel objects. Each lane had the same one `+1` sample and three
zero samples. The report again claimed touched rows `[0, 1]` and columns
`[0, 1]`. The evidence reader accepted it:

```text
monochrome RGB supports disagree with touched extent:
ACCEPT {'claimedVerdictViews': 1, 'verdict': 'pass'}
```

**Why it is wrong**: True monochrome frames have equal red, green and blue
values at each pixel on both sides. Their signed differences therefore agree
pixel by pixel, not merely as three equal aggregate histograms. One nonzero
sample in each lane is the same one differing pixel, so the touched extent can
cover only one row and one column. For a monochrome RGB record, regional union
capacity must use the common lane support rather than the sum of three equal
lane counts.

## Smells

### S1, No standing acceptance case exercises nonzero background feasibility

**Where**: `scripts/guards/catalogue.py:1395-1445,1762-1801` and
`tools/oracle/src/frame.rs:938-975`

The new Python edge-cover calculation for the rectangular image hole is
exercised only by refusal mutations. The Rust unit proves that the producer
emits exact regional indices for one nontrivial background, but it does not
pass that serialization through the Python evidence reader. The genuine green
comparison is an identity run, so its touched arrays are empty.

This leaves the most complex new acceptance branch without a persistent
positive example. A valid report with simultaneous image and background
differences should be accepted as a standing control. It should include touched
background rows and columns that cross the image's row and column ranges while
placing every differing cell outside the image rectangle. That would protect
against a future false refusal in the complement matching formula.

## Nitpicks

None.

## Spatial feasibility audit

- I exhaustively compared the image and background feasibility calculation
  with direct cell enumeration for frames and image rectangles up to 4 by 4,
  all touched row and column subsets, and three lane-difference counts. Across
  3,162,596 cases there were zero mismatches in the calculation's declared
  inputs.
- The background allowed-cell formula correctly removes the image-shaped hole.
  Its isolated-index conditions and maximum-matching formula produce the exact
  minimum edge-cover size for the tested complement graphs.
- D1 and D2 are constraints outside those declared inputs. The first adds the
  common informative-set capacity. The second adds pixelwise support equality
  implied by `monochromeFrame`.
- Touched index arrays are required, sorted, unique and within the frame. Image
  indices are also restricted to the reported image rectangle. Their global
  unions reproduce the scalar touched counts exactly.
- `imageX`, `imageY`, frame dimensions and image dimensions flow from the Rust
  pixel loop through `FrameDifference`, `ViewStatistics`, JSON, the closed
  contract and Python validation without spelling or coordinate drift.
- A volume reformat is now required to have origin zero, full-frame image
  dimensions and no background channel reports. This matches the producer's
  unconditional `Rect::full(width, height)` path and introduces no false
  refusal.
- When `monochromeFrame` is true, all populated RGB region channel objects are
  required equal. The aggregate rule itself is correct. D2 concerns the
  remaining spatial implication of that flag.
- Sparse signed histograms, exact summary derivation, image and background
  composition, informative-bin inclusion, numeric ranges and arithmetic bounds
  remain closed. Index processing is linearithmic in the JSON input size, and
  Python integer arithmetic avoids product overflow.

## Prior-finding closure and verification

- All three pass 12 reproductions now refuse: the impossible L-shaped
  background, unequal monochrome lanes and a partial volume-reformat image.
- Pass 11's region-confinement case now refuses through the exact touched
  arrays. Both pass 10 findings and every earlier sparse-histogram, contract,
  attribution, path, coverage, quirk and provenance repair remain closed.
- The current 99-record report with origins and regional touched arrays is
  accepted unchanged with 71 claimed verdict views.
- `bin/ocelli.sh test ocelli-oracle` passed 145 tests across the library,
  comparator binary and integration suites.
- `bin/ocelli.sh gate guards-deep` passed 418 registered probes. Its census
  found 789 claimed refusal sites and zero watched by nothing.
- Commit `a215924` passed `check-commit --require-corpus` with a matching tree.
  `git diff --check 7c5e29c..a215924` passed.
- No additional defect, smell or nitpick was found outside D1, D2 and S1.
