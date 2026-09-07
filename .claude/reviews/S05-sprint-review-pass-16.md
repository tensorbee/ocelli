# S05 sprint review, pass 16

**Reviewed**: complete remediated sprint diff
`7c5e29c..884425e2865bc2e9ba2db3522acfe6b52096f24a`
**Reviewer**: independent review agent, no implementation changes
**Result**: 0 defects, 1 smell, 0 nitpicks

## Defects

None.

## Smells

### S1, No acceptance probe distinguishes all forced partitions from any forced partition

**Where**: `scripts/guards/catalogue.py:1462-1518,1646-1685,1988-2043`

The pass 15 implementation correctly refuses `monochromeFrame: false` only
when every nonempty spatial partition forces monochrome RGB. The standing
probes do not preserve the universal part of that condition.

Both full-frame forced-extreme probes have no background. The new
opposite-regional-extremes probe has a forced image and a forced background.
The mixed-extremes acceptance probe again has no background. An incorrect
implementation that checked only whether the image partition was forced, or
that refused when any partition was forced, would therefore produce every
current expected result and leave `guards-deep` green.

The missing control is a false-flag record with a forced image and a non-forced
nonempty background. I exercised a one-row, three-column case. Its one-pixel
image had `[[255, 1]]` in every lane. Its two-pixel background had
`[[-255, 1], [255, 1]]` in every lane. The current reader correctly accepted
the otherwise green 99-record report:

```text
{'reportSha256': '53fe34dfeb7658b25e275c1b370acedd329a6f1ef2c466133b8d18a83d1fd318',
 'claimedVerdictViews': 71, 'verdict': 'pass'}
```

This report is realizable. Keep the image lanes at `+255`. In the two
background pixels, assign red signs `[-255, +255]`, green signs
`[+255, -255]`, and blue signs `[-255, +255]`. Each lane retains the reported
histogram, while each background pixel is coloured on both sides. A standing
acceptance probe for this case would fail if the region loop at
`scripts/verify_ledger.py:610-625` were narrowed to image or changed from all
partitions to any partition.

## Nitpicks

None.

## Pass 15 regional characterization audit

- The producer defines `monochromeFrame` over both complete rendered frames at
  `tools/oracle/src/attribution.rs:751-754`, using the per-pixel RGB equality
  check at `tools/oracle/src/frame.rs:540-550`. The validator applies its false
  direction only to the three-lane class-two shape at
  `scripts/verify_ledger.py:609-630`, which is the only shape where a false
  flag can occur in a green report.
- The exact forced-value argument is correct. For a signed byte difference
  `d = candidate - reference`, `+255` uniquely gives `(0, 255)` and `-255`
  uniquely gives `(255, 0)`. Every other difference admits at least two
  reference values. Equal non-extreme lane differences can therefore use
  different reference values to make both RGB pixels coloured.
- Unequal lane histograms cannot force monochrome frames because monochrome
  source pixels have equal signed differences in all three lanes. Equal
  histograms containing both extreme signs also do not force monochrome. With
  at least two pixels, the signs may be permuted between lanes at one pixel,
  exactly as the acceptance probe at `scripts/guards/catalogue.py:1646-1685`
  demonstrates.
- A single partition forces both frames monochrome exactly when all three lane
  histograms are the same uniform `+255` or uniform `-255` distribution. The
  complete frame is forced exactly when every nonempty image or background
  partition has that property. The implementation initializes the conjunction
  at line 613 and clears it when any partition fails at lines 614-625.
- Empty background is handled correctly by omitting it from the spatial region
  list at lines 610-612. A nonempty all-zero background makes a false flag
  realizable because a zero-difference pixel can use different equal channel
  values. Even an uninformative zero pixel can choose different clipped
  extremes across RGB lanes, consistent with the producer definition at
  `tools/oracle/src/frame.rs:599-604,670-688`.
- Opposite forced signs across image and background now refuse with
  `non-monochrome frame is impossible from forced regional RGB extremes`.
  Uniform same-sign forced partitions also refuse. A mixed-sign partition, a
  zero-difference background, and a non-extreme difference each remain
  accepted, so the repair introduced no measured false refusal.
- I independently enumerated 46,375 three-lane histogram triples for partition
  sizes one through three using differences `-255`, `-254`, `0`, `254`, and
  `255`. Exact per-pixel lane assignment and the implemented uniform-extreme
  predicate disagreed in zero cases. The six forced cases were the two extreme
  signs at each of the three partition sizes.

## Spatial support, schema, and producer consistency

- The frame loop computes signed lane differences, informative membership, and
  exact regional touched indices together at
  `tools/oracle/src/frame.rs:638-708`. The validator checks full-image and
  background composition before the false-flag rule at
  `scripts/verify_ledger.py:549-600`, so the regional histograms cannot drift
  independently from the full histogram.
- Image support remains capped by `informativePixels` at
  `scripts/verify_ledger.py:693-699`. True monochrome RGB support remains capped
  by one shared lane in the same common regional checker. Calls at lines
  707-710 apply that calculation to both image and background.
- I repeated the direct spatial enumeration through frames and image
  rectangles up to 4 by 4. All 3,162,596 region, touched-set, and three-lane
  count cases agreed with the image and rectangular-hole capacity calculation.
- The report contract already publishes `monochromeFrame` as a required
  boolean, and the Rust serializer emits the producer-owned value at
  `tools/oracle/src/report.rs:519-525`. Pass 15 required no schema change. The
  LLD at `docs/lld/comparator.md:304-327` now describes the same regional
  condition implemented by the validator.
- The remediation adds a fixed two-region by three-channel scan to ledger
  validation. It changes no render-loop work, allocation, tolerance, DICOM
  attribute handling, colour conversion, or LUT arithmetic. Python integer
  arithmetic retains the existing overflow-safe spatial products.

## Whole-sprint closure and measured verification

- Passes 1 through 14 remain closed at their recorded boundaries. In
  particular, malformed green evidence, contradictory summaries and
  histograms, impossible informative subsets, invalid dimensions, regional
  support, the background rectangle hole, true monochrome lane identity, and
  full-frame reformat constraints continue to refuse.
- The current 99-record report with exact regional indices remains accepted
  with 71 claimed verdict views. Commit `884425e2865bc2e9ba2db3522acfe6b52096f24a`
  carries the recorded comparison digest
  `738ecac020ca5ae02101fa6206fb88b2e5da77dd70b991107508aff603cbb6a0`.
- `bin/ocelli.sh test ocelli-oracle` passed 145 tests across the library,
  comparator binary, and integration suites.
- `bin/ocelli.sh gate guards-deep` passed 376 refusal probes, 50 acceptance
  probes, and 48 controls. Its census found 790 claimed refusal sites and zero
  watched by nothing.
- `bin/ocelli.sh gate quirks quirk-mutations ci guards` passed. The mutation
  gate drove three controlled mutations and six fixture-binding mutations red,
  CI proved all 26 floor gates on pull request and push, and the floor guard
  harness remained green.
- Commit `884425e2865bc2e9ba2db3522acfe6b52096f24a` passed
  `check-commit --require-corpus` with a matching tree.
  `git diff --check 7c5e29c..884425e2865bc2e9ba2db3522acfe6b52096f24a`
  passed.
- No defect or nitpick was found. S1 is the only remaining smell.
