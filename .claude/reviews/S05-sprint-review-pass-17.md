# S05 sprint review, pass 17

**Reviewed**: complete remediated sprint diff
`7c5e29c..d8b49dc322b9673d2411d97bf299ae9e1e8e03e9`
**Reviewer**: independent review agent, no implementation changes
**Result**: 0 defects, 1 smell, 0 nitpicks

## Defects

None.

## Smells

### S1, The two-partition acceptance matrix remains asymmetric

**Where**: `scripts/guards/catalogue.py:1646-1733,2081-2097` and
`docs/sprints/AS_BUILT.md:2000-2004`

The pass 16 acceptance probe correctly distinguishes a regional conjunction
from an image-only or any-partition refusal. It does not preserve the symmetric
case where the image is mixed and the nonempty background is forced.

I changed the validator in a disposable clone to prefer the background region
whenever it exists, falling back to the image only when there is no background.
That incorrect implementation rejects a valid mixed-image, forced-background
record, but all current floor probes still passed:

```text
OK: 327 refusal probe(s) drove 31 guard(s) red for their declared reason,
37 accept probe(s) green, 0 known defect(s) still open,
45 control(s) green
```

The mutation survives because the no-background forced and mixed probes still
read the image. The opposite-regional-extremes refusal still sees a forced
background. The new forced-image, mixed-background control still sees its
mixed background and accepts. No existing probe asks the validator to accept
when only the image partition proves that the frame can be non-monochrome.

I constructed that missing one-row, three-column report. Its two-pixel image
has `[[-255, 1], [255, 1]]` in every RGB lane. Its one-pixel background has
`[[255, 1]]` in every lane. The full histogram is therefore
`[[-255, 1], [255, 2]]`. The current reader correctly accepted it:

```text
{'reportSha256': 'c7883ec9c0a466b02b559dde351bd130df9a3d01f2a0bbe46aacf18c7b3eaf1d',
 'claimedVerdictViews': 1, 'verdict': 'pass'}
```

The report is producer-realizable. Use image lane differences
`[-255, +255, -255]` at the first pixel and
`[+255, -255, +255]` at the second. Use `[+255, +255, +255]` in the
background. The reference pixels are `[255, 0, 255]`, `[0, 255, 0]`, and
`[0, 0, 0]`. The candidate pixels are `[0, 255, 0]`, `[255, 0, 255]`, and
`[255, 255, 255]`. Both frames contain coloured image pixels while every lane
retains the reported regional histogram.

The two-region truth table also lacks a both-mixed acceptance control and a
same-sign, both-forced refusal. The existing opposite-sign forced refusal and
forced-image acceptance do not preserve those rows. A symmetric acceptance
probe is the immediate missing boundary. Covering all four forced or mixed
partition combinations, with both sign relations in the forced row, would make
the regional conjunction a mechanical claim rather than a code-inspection
claim.

## Nitpicks

None.

## Pass 16 remediation audit

- The new forced-image, mixed-background report is producer-realizable. An
  independent reconstruction produced reference pixels `[0, 0, 0]`,
  `[255, 0, 255]`, `[0, 255, 0]` and candidate pixels `[255, 255, 255]`,
  `[0, 255, 0]`, `[255, 0, 255]`. Each lane has one `-255` and two `+255`
  full-frame differences, while the image has one forced `+255` pixel and the
  background has one difference of each sign.
- Its summary is internally consistent. It has two records, one pass, one
  unmeasured view, one claimed verdict view, and one `unstated-threshold`
  qualifier. The coverage count is one unmeasured view. Both aggregate hashes
  recompute to
  `b0b98f8bed44b572acd2876c40b2c73809bf20a8ec78499cae11dacdb822319f`
  after the new record is sorted into the run.
- The probe fails under an image-only check and under an any-partition check,
  so it closes pass 16 S1 exactly as stated. S1 in this pass is the untested
  symmetric direction, demonstrated by a different surviving mutation.
- The implementation at `scripts/verify_ledger.py:610-630` is correct. It
  includes image and every nonempty background region, proves each partition
  forced only for one common uniform extreme across all RGB lanes, and refuses
  a false flag only when all included partitions are forced.

## Enumeration, spatial constraints, and prior closure

- I independently enumerated 46,375 three-lane histogram triples for partition
  sizes one through three using differences `-255`, `-254`, `0`, `254`, and
  `255`. Exact lane assignment and the implemented predicate disagreed in zero
  cases. The six forced cases were the two extremes at each partition size.
- Combining the per-partition result over the four forced or mixed states for
  image and background also produced zero implementation mismatches. The
  whole frame is forced exactly when both nonempty partitions are forced.
- The current reader refuses a same-sign forced image and background. It
  accepts both mixed partitions and the symmetric mixed-image,
  forced-background report. These checks found no current false acceptance or
  false refusal.
- I repeated the direct spatial enumeration through frames and image
  rectangles up to 4 by 4. All 3,162,596 region, touched-set, and
  three-lane-count cases agreed with the image and rectangular-hole capacity
  calculation.
- The pass 13 informative-union and monochrome-shared-support reproductions
  still refuse with `image touched indices contradict region geometry`.
  Passes 1 through 15 otherwise remain closed at their recorded boundaries.

## Measured verification

- The current 99-record report remains accepted with 71 claimed verdict views
  and digest
  `738ecac020ca5ae02101fa6206fb88b2e5da77dd70b991107508aff603cbb6a0`.
- `bin/ocelli.sh test ocelli-oracle` passed 145 tests across the library,
  comparator binary, and integration suites.
- The deep guard profile passed 376 refusal probes, 51 acceptance probes, and
  48 controls. Its census found 790 claimed refusal sites, 427 probes, and
  zero sites watched by nothing.
- The quirk, quirk-mutation, CI, and floor guard checks passed. The quirk
  mutation gate drove three controlled mutations and six fixture-binding
  mutations red. CI proved all 26 floor gates on pull request and push.
- Commit `d8b49dc322b9673d2411d97bf299ae9e1e8e03e9` passed
  `check-commit --require-corpus` with a matching tree.
- `git diff --check
  7c5e29c..d8b49dc322b9673d2411d97bf299ae9e1e8e03e9` passed.
- No defect or nitpick was found. S1 is the only remaining smell.
