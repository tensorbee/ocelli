# S05 sprint review, pass 18

**Reviewed**: complete remediated sprint diff
`7c5e29c..1433f80f35ccccab46ae159e46f6d1e23e2d3e58`
**Reviewer**: independent review agent, no implementation changes
**Result**: 0 defects, 1 smell, 0 nitpicks

## Defects

None.

## Smells

### S1, The exact forced-extreme boundary is not preserved by standing probes

**Where**: `scripts/guards/catalogue.py:1462-1531,1659-1761,2058-2158`,
`scripts/verify_ledger.py:609-630` and
`docs/sprints/AS_BUILT.md:2006-2010`

The pass 17 probes complete the forced-or-mixed state matrix, but every forced
partition in the standing catalogue has exactly one pixel and every
non-forced case uses both signed endpoints. The suite therefore preserves the
regional Boolean operation without preserving the exact numeric predicate
that feeds it.

I made two independent changes to the validator in disposable clones of the
reviewed commit. First, I broadened `forced_extremes` to include uniform
`+254` and `-254` histograms. Second, I made the validator recognize a forced
partition only when `region_pixels == 1`. Both wrong implementations passed
the complete floor guard gate:

```text
OK: 328 refusal probe(s) drove 31 guard(s) red for their declared reason,
39 accept probe(s) green, 0 known defect(s) still open,
45 control(s) green
```

The first mutation creates a false refusal. I constructed a one-row report
whose image and background each contain one pixel and whose three lane
histograms are all `[[254, 1]]`. The full histograms are `[[254, 2]]`.
The current validator accepted the report. The broadened mutant refused it as
`non-monochrome frame is impossible from forced regional RGB extremes`.

That report is producer-realizable. In each partition, reference pixel
`[0, 1, 0]` and candidate pixel `[254, 255, 254]` give a `+254` difference in
every lane while both frames remain coloured. The false `monochromeFrame`
value is therefore true evidence, not merely a schema-valid construction.

The second mutation creates a false acceptance. A report with two image
pixels and two background pixels, all uniformly `+255` in all three lanes,
must be refused when `monochromeFrame` is false. The current validator refused
it. The count-one mutant accepted it and recorded the tree. Every source pixel
is forced to black and every candidate pixel is forced to white, so no lane
assignment can make either frame coloured.

Add standing acceptance controls at the adjacent `+254` and `-254` boundaries
and forced refusal controls with a regional count greater than one. Those
controls should preserve both signs. The complete partition state matrix can
then compose a mechanically preserved regional predicate rather than one
whose value and multiplicity boundaries remain code-inspection claims.

## Nitpicks

None.

## Pass 17 remediation audit

- The two-partition state matrix is now symmetric. Forced image with mixed
  background, mixed image with forced background, and both mixed all accept.
  Both-forced combinations refuse with the same sign and with opposite signs.
  The existing
  no-background controls refuse both forced signs and accept mixed signs.
- An image-only or background-omitting implementation fails the forced-image,
  mixed-background control. A background-only or image-omitting implementation
  fails the mixed-image, forced-background control. An any-partition refusal
  fails both asymmetric acceptance rows. A combined-full-histogram rewrite
  fails the opposite-sign forced refusal. A positive-only or negative-only
  check fails one of the no-background refusals. A same-sign-only check fails
  the opposite-sign forced refusal. None of those mutations survives.
- The new reports are producer-realizable. The mixed partition uses reference
  pixels `[255, 0, 255]`, `[0, 255, 0]` and candidate pixels
  `[0, 255, 0]`, `[255, 0, 255]`. Every lane then has one `-255` and one
  `+255` while both frames contain coloured pixels. A forced `+255` partition
  uses black reference and white candidate pixels. Reusing the mixed pair in
  both regions realizes the both-mixed report.
- Each generic acceptance report has two records, two views, one pass, one
  unmeasured view, zero fail, zero absent, one claimed verdict view, one
  coverage-unmeasured view, and one `unstated-threshold` qualifier. The
  validator accepted all three generated documents.
- I independently recomputed both sides of each `sha256-rgba8-run-v1`
  aggregate after record sorting. Forced-image with mixed-background is
  `b0b98f8bed44b572acd2876c40b2c73809bf20a8ec78499cae11dacdb822319f`.
  Mixed-image with forced-background is
  `680cbfe4aa9000903f8b02a7f6575cb4d47b895d337216793aa311c57bdefbe1`.
  Both-mixed is
  `b62568ae2cf14a40f245ba9114dba2b8ee78c10ce5f97ce695cb7325a9b863a9`.
  Stored and recomputed reference and candidate values matched in every case.
- Dimensions, regions and touched indices agree. The helper places the image
  in the first contiguous columns of a one-row frame and the background in the
  remaining columns. Every sample differs, every image sample is informative,
  and the regional histograms sum exactly to the full histogram.
- The generated runbook carries the four refusal rows at 232 through 235 and
  the four relevant acceptance rows at 248 through 251. The guard census sees
  all 430 catalogue probes and reports zero refusal sites watched by nothing.

## Producer, schema and implementation consistency

- `tools/oracle/src/frame.rs:638-708` computes signed differences as candidate
  minus reference in one pass, partitions them into image and background, and
  records the exact touched indices. `tools/oracle/src/attribution.rs:753-754`
  sets `monochromeFrame` only when both frames have no coloured pixel.
- `tools/oracle/src/report.rs:414-421,445-526` defines and serializes the flag
  with the regional statistics. `tools/oracle/report-contract.json:44-74`
  declares the same fields. The tracked-contract serializer test passed.
- `scripts/verify_ledger.py:609-630` is correct. It includes the image and each
  nonempty background, recognizes only a lane-identical uniform `+255` or
  `-255` histogram as forced, and refuses a false flag only when every included
  partition is forced. No current false acceptance or false refusal was found.
- I enumerated 46,375 three-lane histogram triples for partition sizes one
  through three over differences `-255`, `-254`, `0`, `254`, and `255`.
  Exact lane assignment and the implemented predicate disagreed in zero cases.
  The six forced cases were the two endpoints at each of the three sizes.
- I repeated the direct spatial enumeration through frames and image
  rectangles up to 4 by 4. All 3,162,596 region, touched-set, and
  three-lane-count cases agreed with the image and rectangular-hole capacity
  calculation.

## Whole-sprint closure and measured verification

- I reviewed all 65 changed files in the sprint diff, comprising 9,916
  insertions and 500 deletions, and audited the 17 prior sprint review records.
  The identity, input-safety and verdict defects from passes 1 through 4 remain
  closed. The schema, derivation and authority defects from passes 5 through 8
  remain closed. The informative and spatial feasibility defects from passes
  9 through 13 remain closed. The monochrome false-direction defects from
  passes 14 and 15 and the partition-composition smells from passes 16 and 17
  remain closed at their recorded boundaries.
- The current sprint evidence documents the 99-view comparison. The ledger
  records 71 claimed verdict views with pass verdict and digest
  `738ecac020ca5ae02101fa6206fb88b2e5da77dd70b991107508aff603cbb6a0`.
- `bin/ocelli.sh test ocelli-oracle` passed 145 tests across the library,
  comparator binary and integration suites.
- The deep guard profile passed 377 refusal probes, 53 acceptance probes and
  48 controls. Its census found 790 claimed refusal sites, 430 catalogue
  probes and zero sites watched by nothing.
- The quirk, quirk-mutation and CI gates passed. The quirk-mutation gate drove
  three controlled mutations and six fixture-binding mutations red. CI proved
  all 26 floor gates on pull request and push.
- Commit `1433f80f35ccccab46ae159e46f6d1e23e2d3e58` passed
  `check-commit --require-corpus` with corpus pass and matching tree
  `257e910c6956bd565663aa0b674cf31bfc705674`.
- `git diff --check
  7c5e29c..1433f80f35ccccab46ae159e46f6d1e23e2d3e58` passed.
- No defect or nitpick was found. S1 is the only remaining smell.
