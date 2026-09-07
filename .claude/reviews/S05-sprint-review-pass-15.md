# S05 sprint review, pass 15

**Reviewed**: complete remediated sprint diff
`7c5e29c..4172bfdda8ca8e94136a1966790e8e62a8c7d177`
**Reviewer**: independent review agent, no implementation changes
**Result**: 1 defect, 0 smells, 0 nitpicks

## Defects

### D1, Opposite forced extremes across image and background still admit an impossible false `monochromeFrame`

**Where**: `scripts/verify_ledger.py:609-623`,
`scripts/guards/catalogue.py:1462-1489,1617-1657`,
`docs/lld/comparator.md:322-325`, and
`docs/sprints/AS_BUILT.md:1985-1991`

**What**: The pass 14 remediation checks the false direction only when each
full-frame RGB histogram is one uniform `+255` or `-255` bin. Image and
background are separate spatial partitions whose signed histograms are also
reported exactly. Each partition can force monochrome pixels even when their
opposite signs combine into a mixed full-frame histogram.

I constructed a one-row, two-column class-two record with a one-pixel image
and a one-pixel background. All three RGB lanes reported `[[255, 1]]` in the
image, `[[-255, 1]]` in the background, and therefore
`[[-255, 1], [255, 1]]` in the full frame. The informative histogram was the
one image `+255`, and the regional touched indices named the two real cells.
The record set `monochromeFrame` to false. The current evidence reader accepted
the otherwise green report:

```text
{'reportSha256': '1c8e5c51b484c37458ec22aa8ea1396dc8dfd2097c76955e929a70df37347fbd',
 'claimedVerdictViews': 1, 'verdict': 'pass'}
```

**Why it is wrong**: A signed byte difference of `+255` uniquely fixes the
reference and candidate values to 0 and 255. A difference of `-255` uniquely
fixes them to 255 and 0. The image pixel is therefore forced to reference
`[0, 0, 0]` and candidate `[255, 255, 255]`. The background pixel is forced to
reference `[255, 255, 255]` and candidate `[0, 0, 0]`. Both source frames are
monochrome at every pixel, so the producer at
`tools/oracle/src/attribution.rs:751-754` must publish true.

The condition at `scripts/verify_ledger.py:617-619` misses this because the
full histogram contains both signs and is not one of its two `forced_extremes`
values. The exactly decidable condition is regional. A false flag is impossible
when every nonempty spatial partition, image and background, has all three
lanes concentrated in one common forced extreme. The partitions may choose
different extremes. A standing refusal probe needs this opposite-sign,
nonempty-background case.

## Smells

None.

## Nitpicks

None.

## Pass 14 remediation audit

- The uniform positive and uniform negative forced-extreme probes are valid.
  Their one-pixel reports uniquely determine monochrome source frames, and the
  current reader refuses both.
- The mixed-extreme acceptance probe is also valid. With two pixels and one of
  each sign per lane, the signs can be assigned differently between RGB lanes
  at a pixel. That realizes a non-monochrome frame while preserving every
  reported lane histogram. The current reader accepts it as intended.
- The background shared-support probe closes pass 14 S1. It constructs a
  nonempty two-cell background whose touched columns require two support cells
  while a true monochrome frame provides only one common differing support
  cell. The generic regional calculation refuses it, and `guards-deep` drives
  the probe.
- The implementation and documentation overstate the false-direction closure
  as exact. It is exact for one full-frame uniform sign, but D1 shows that
  exact regional evidence proves another producer-impossible class.

## Prior-finding closure and measured verification

- The pass 13 informative-union and monochrome-shared-support reproductions
  both still refuse with `image touched indices contradict region geometry`.
- I repeated the regional geometry enumeration through frames and image
  rectangles up to 4 by 4. All 3,162,596 region, touched-set, and
  three-lane-count cases agreed with the implemented capacity calculation.
- I separately enumerated 50,400 aggregate `+255` and `-255` lane-count cases
  for image sizes 1 through 4 and background sizes 0 through 4. The current
  full-histogram predicate produced no false refusals and missed 32 impossible
  false flags. Every miss was an image forced to one sign and a nonempty
  background forced to the opposite sign, which is D1.
- The current 99-record report remains accepted with 71 claimed verdict views
  and report digest
  `738ecac020ca5ae02101fa6206fb88b2e5da77dd70b991107508aff603cbb6a0`.
- `bin/ocelli.sh test ocelli-oracle` passed 145 tests across the library,
  comparator binary, and integration suites.
- `bin/ocelli.sh gate guards-deep` passed with 790 claimed refusal sites, 425
  probes, and zero sites watched by nothing.
- `bin/ocelli.sh gate quirks quirk-mutations ci guards` passed. The quirk
  mutation gate drove three controlled mutations and six fixture-binding
  mutations red.
- Commit `4172bfdda8ca8e94136a1966790e8e62a8c7d177` passed
  `check-commit --require-corpus` with a matching tree.
- `git diff --check
  7c5e29c..4172bfdda8ca8e94136a1966790e8e62a8c7d177` passed.
- No additional defect, smell, or nitpick was found outside D1.
