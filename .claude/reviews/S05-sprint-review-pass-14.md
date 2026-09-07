# S05 sprint review, pass 14

**Reviewed**: complete remediated sprint diff `7c5e29c..df929ca`
**Reviewer**: independent review agent, no implementation changes
**Result**: 1 defect, 1 smell, 0 nitpicks

## Defects

### D1, A producer-impossible false `monochromeFrame` claim is accepted

**Where**: `tools/oracle/src/frame.rs:540-550`,
`tools/oracle/src/attribution.rs:751-754`,
`tools/oracle/src/report.rs:413-421`, and
`scripts/verify_ledger.py:602-608,733-752,782-792`

**What**: The producer defines `monochromeFrame` as true exactly when every
reference and candidate pixel has equal red, green and blue lanes. The evidence
reader checks the spatial and distribution consequences only when the flag is
true. A class-two record whose flag is false is accepted without checking that
any source frames could make it false.

I changed one current class-two record to a one-pixel full-frame image with
`monochromeFrame: false`. Each of its three full, image, and informative channel
reports had the exact signed histogram `[[255, 1]]`. The report had one
informative pixel, row and column index `[0]`, consistent summaries, and the
unchanged valid run hashes. The current evidence reader accepted the otherwise
green 99-record report:

```text
{'reportSha256': 'b60498576aa781d9f5414fba4dddb592d5f423259efd0c6391d46967daf27f85',
 'claimedVerdictViews': 71, 'verdict': 'pass'}
```

**Why it is wrong**: The Rust producer computes each signed difference as
`candidate - reference` over `u8` lanes at `tools/oracle/src/frame.rs:658-664`.
The only pair with difference `+255` is reference `0`, candidate `255`.
Applying that fact independently to red, green, and blue forces the sole
reference pixel to `[0, 0, 0]` and the sole candidate pixel to
`[255, 255, 255]`. Both frames are monochrome, so the producer at
`tools/oracle/src/attribution.rs:753-754` must publish true. The accepted false
claim cannot be emitted by the producer.

The evidence boundary needs to validate both truth values of the published
fact. A verifier-checkable non-monochrome witness is the direct option. At
minimum, the one-pixel forced-extreme case above needs a refusal probe so the
false direction cannot remain unchecked.

## Smells

### S1, The monochrome shared-support regression does not exercise background

**Where**: `scripts/guards/catalogue.py:1462-1493,1855-1856,1890-1897`

The implementation is correct for both regions. The cap at
`scripts/verify_ledger.py:675-676` is inside the common regional checker, and
that checker is called for image and background at lines 685-687. The standing
`monochrome-touched-support` probe nevertheless constructs a 2 by 2 full-frame
image with no background and expects an image-region refusal. The new nonzero
background acceptance probe starts from the one-channel `mono16` green fixture,
so it does not exercise shared RGB lane support either.

A three-channel monochrome record with a nonempty background should make the
background require two differing pixels while each equal lane reports only one.
That case should refuse. It would preserve the stated both-region property
against a later image-only conditional without depending on inspection of the
current generic function.

## Nitpicks

None.

## Pass 13 remediation audit

- The image touched union is now capped by `informativePixels` at
  `scripts/verify_ledger.py:672-674`. This is necessary because every nonzero
  image difference enters the informative set in
  `tools/oracle/src/frame.rs:670-688`. It is sufficient alongside the existing
  per-lane and cell bounds because the remaining informative pixels may carry
  zero difference. The pass 13 one-informative-pixel reproduction now refuses.
- A true monochrome RGB record is first required to have identical channel
  reports at `scripts/verify_ledger.py:602-608`. Its union capacity is then
  bounded by one lane at lines 675-676. Since all three lane differences are
  equal pixel by pixel, `max(channel_differences)` is the exact common support
  size. The same calculation runs for image and background. The pass 13
  summed-lane reproduction now refuses.
- The rectangular-hole calculation remains exact. The background allowed-cell
  count at lines 649-658, isolated-index checks at lines 659-664, and maximum
  matching at lines 665-669 give the minimum edge cover used at line 671. I
  repeated the prior direct enumeration through frames and image rectangles up
  to 4 by 4. All 3,162,596 region, touched-set, and three-lane-count cases agreed
  with the calculation.
- The nonzero-background acceptance probe is realizable. Set every reference
  sample to 100 and use the following candidate-minus-reference differences,
  with the top-left 2 by 2 cells as the image:

  ```text
  -1  0  +1
   0 +1   0
  +1  0  +1
  ```

  The image histogram is `[-1: 1, 0: 2, +1: 1]`. Its touched rows and columns
  are both `[0, 1]`. The five background cells have histogram
  `[0: 2, +1: 3]`, touched rows `[0, 2]`, and touched columns `[0, 2]`.
  Their sum is the probe's full histogram `[-1: 1, 0: 4, +1: 4]`. All four
  image samples are informative because value 100 is not a display extreme,
  including the two equal pairs. This realizes every field constructed at
  `scripts/guards/catalogue.py:1530-1553`.

## Producer, contract, and validator consistency

- Frame and image dimensions, image origin, and four regional touched-index
  arrays are produced from the same pixel loop at
  `tools/oracle/src/frame.rs:638-708`. The validator bounds the origin and
  dimensions at `scripts/verify_ledger.py:453-510`, confines image indices at
  lines 523-543, and reconstructs the global scalar touched counts from the
  exact regional unions at lines 544-548.
- Full equals image plus background per signed lane histogram. Every nonzero
  image bin is present unchanged in the informative histogram. Image and
  background supports occupy disjoint cells, so their regional feasibility
  checks compose without an extra cross-region placement constraint.
- The sparse signed histograms have at most 511 possible `u8` difference keys
  per lane. Pixel counts and indices remain producer-bounded, while Python
  integer products avoid overflow. The added checks are linear in the report
  arrays apart from the existing sorting and do not affect the Rust render
  loop.
- Contract keys and serializer spellings remain aligned. The serializer emits
  `monochromeFrame` at `tools/oracle/src/report.rs:519-525`, but D1 shows that
  the validator still enforces only one direction of that field's semantics.

## Prior-finding closure and verification

- All three pass 13 findings are closed in the current implementation. D1 in
  this pass is a separate false-value consistency gap exposed while checking
  false accepts across the full S05 evidence boundary.
- The pass 13 informative-cap and monochrome-cap reproductions both refuse with
  `image touched indices contradict region geometry`.
- The current 99-record report with origins and regional touched arrays is
  accepted unchanged with 71 claimed verdict views.
- `bin/ocelli.sh test ocelli-oracle` passed 145 tests across the library,
  comparator binary, and integration suites.
- `bin/ocelli.sh gate guards-deep` passed 372 refusal probes, 49 acceptance
  probes, and 48 controls. Its census found 789 claimed refusal sites and zero
  watched by nothing.
- Commit `df929ca` passed `check-commit --require-corpus` with a matching tree.
  `git diff --check 7c5e29c..df929ca` passed.
- No additional defect, smell, or nitpick was found outside D1 and S1.
