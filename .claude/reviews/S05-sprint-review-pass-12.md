# S05 sprint review, pass 12

**Reviewed**: complete remediated sprint diff `7c5e29c..523efff`
**Reviewer**: independent agent, did not write the sprint implementation or pass 11 remediation
**Result**: 3 defects, 0 smells, 0 nitpicks

## Defects

### D1, The regional touched bound accepts an impossible background layout

**Where**: `scripts/verify_ledger.py:552-604`,
`tools/oracle/src/frame.rs:638-687` and
`scripts/guards/catalogue.py:1322-1338,1656`

**What**: The pass 11 remediation computes independent upper bounds for
background rows and columns. A background is not generally a rectangle. It is
the complement of the image rectangle, so its available rows and columns
cannot always be combined as a Cartesian block.

Starting from the serializer-owned green contract fixture, I constructed this
exact class-one pass:

```text
frame:       3 rows by 3 columns
image:       2 rows by 2 columns, signedHistogram=[[0,4]]
background:  5 pixels, signedHistogram=[[-1,2],[0,1],[1,2]]
full:        signedHistogram=[[-1,2],[0,5],[1,2]]
informative: signedHistogram=[[0,4]]
touched:     2 rows by 2 columns
```

All four background differences are balanced, all class-one bounds pass, and
every scalar, region, dimension, composition and hash check agrees. The
production evidence reader accepted it:

```text
{'reportSha256': '23564eda8dcd31a956d20c32ba08099fc94e688eed28f129aa8b4504e39e897c',
 'claimedVerdictViews': 1, 'verdict': 'pass'}
```

No placement of a 2 by 2 image inside a 3 by 3 frame leaves a 2 by 2 block in
the background. Four distinct background pixels touching exactly two rows and
two columns would have to fill such a block. The Rust producer therefore
cannot emit these touched counts.

**Why it is wrong**: Lines 584-600 prove separate row and column maxima, but
not that one set of differing background pixels can attain both maxima at the
same time. The pass 11 probe covers an unchanged background and therefore does
not exercise this complement geometry. The ledger still attests a spatially
impossible report. A complete check needs a joint image and complement
feasibility rule, or producer evidence that preserves enough spatial shape to
validate the reported pair.

### D2, A monochrome frame may carry three different channel distributions

**Where**: `scripts/verify_ledger.py:417-430,649-668`,
`tools/oracle/src/attribution.rs:751-754,790-793` and
`tools/oracle/src/report.rs:408-415`

**What**: Rust sets `monochromeFrame` only when both frames have red equal to
green equal to blue at every pixel. For a class-two record, that makes the
signed difference identical in all three compared lanes at every pixel. The
full, image, background and informative histograms must therefore be identical
across the three channel entries.

I changed only channel 1 of the genuine monochrome class-two record
`real__us_cmb_crc__00000001`. Its image and informative distributions gained
one `-1` and one `+1`, with the same balanced change composed into the full
distribution. Channels 0 and 2 remained all zero. All derived summaries,
touched counts, region composition and run hashes remained consistent, and
`monochromeFrame` remained true. The evidence reader accepted the 99-record
report:

```text
{'reportSha256': '6027ab4a0f0ba1eefaa68c532000def8c4b423f6bb12cf3e4eecf799160c2570',
 'claimedVerdictViews': 71, 'verdict': 'pass'}
```

**Why it is wrong**: No pair of monochrome frames can differ in only one colour
lane. The verifier checks the flag's type and requires it for `mono16`, but it
never validates what a true flag means for a three-channel record. This leaves
a producer-impossible relationship among already serialized fields, with no
standing refusal probe.

### D3, A volume reformat may claim a partial image rectangle

**Where**: `scripts/verify_ledger.py:417,649-698` and
`tools/oracle/src/attribution.rs:632-655,658-690`

**What**: Rust defines every volume-reformat image rectangle as the whole
frame. The verifier validates `kind` separately, then calls `_statistics()`
without passing it. It therefore accepts a volume record whose image dimensions
are smaller than the frame and whose background is nonempty.

I changed the genuine record
`volume__synthetic__ct_series_uniform__AXIAL` from a 512 by 512 image to a 256
by 512 image, split its unchanged full histogram evenly between image and
background, and left its empty informative region and weak unmeasured state
consistent. The evidence reader accepted it:

```text
{'reportSha256': '22a32ce9bee2e5343053177abb57361e379c881c88ffe2c49820381ea15f7ac2',
 'claimedVerdictViews': 71, 'verdict': 'pass'}
```

**Why it is wrong**: `rectangles()` unconditionally returns
`Rect::full(width, height)` for this kind. The accepted dimensions and
background can never be emitted by the producer. The validator must couple
`kind` to the dimension and background rules, with a probe for a partial
volume-reformat image.

## Smells

None.

## Nitpicks

None.

## Pass 11 bound and data-format audit

- The regional formula is a safe upper bound for producer output. A region can
  touch no more rows or columns than its extent, and no more than its number of
  differing pixels. Summed lane differences are at least the number of
  distinct differing pixels. Combining the two region bounds cannot be below
  the producer's union. I found no false refusal in this bound.
- The genuine 99-record dimensional report is accepted after changing only
  the operation and candidate directory needed by the evidence command. It
  retains 71 claimed verdict views. The exact pass 11 reproduction now refuses
  with `touched counts contradict region geometry`.
- Dimension values are positive `u32` integers, their products are checked
  against accepted `u32` pixel totals, and Python multiplication cannot
  overflow. The new arithmetic adds no material denial-of-service path.
- Sparse signed histograms remain ordered and bounded at 511 entries per
  channel. Counts, absolute buckets, maximum, percentile, fractions and signed
  mean are derived from the same distribution under the producer's count and
  signed-sum limits. Full composition and informative nonzero-bin selection
  remain closed.
- Rust threads frame and image dimensions from `FrameDifference` through
  `ViewStatistics` to JSON. The tracked contract requires the same fields, and
  the serializer contract test closes the object key set. I found no field
  spelling, numeric-type, contract, hash-domain or serialization drift.
- D1 is beyond the one-row image probe. D2 and D3 have no matching semantic
  probes in the catalogue. The existing malformed histogram, informative,
  dimension, composition and hash probes remain effective for their declared
  boundaries.

## Prior-finding closure and verification

- Pass 11 D1 is closed at its exact reproduction without refusing the genuine
  report. D1 above is a different simultaneous row-and-column condition.
- Both pass 10 findings, pass 9 informative omission, pass 8 exact-distribution
  findings and helper smell, pass 7 tail and production-state findings, pass 6
  path and contract findings, and all earlier report-shape, coverage, quirk and
  provenance repairs remain closed.
- `bin/ocelli.sh test ocelli-oracle` passed 144 tests across the library,
  comparator binary and integration suites.
- `bin/ocelli.sh gate guards-deep` passed 360 refusal probes, 48 acceptance
  probes and 48 controls. Its census found 783 claimed refusal sites and zero
  watched by nothing.
- Commit `523efff` passed `check-commit --require-corpus` with a matching tree.
  `git diff --check 7c5e29c..523efff` passed.
- No additional defect, smell or nitpick was found outside D1 through D3.
