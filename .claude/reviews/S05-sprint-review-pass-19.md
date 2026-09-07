# S05 sprint review, pass 19

**Reviewed**: complete remediated sprint diff
`7c5e29c..067082061aa4d6ffedb1ea23c86a9a6fa24bffab`
**Reviewer**: independent review agent, no implementation changes
**Result**: 0 defects, 0 smells, 0 nitpicks

## Defects

None.

## Smells

None.

## Nitpicks

None.

## Pass 18 remediation audit

- The catalogue now has acceptance controls at both adjacent signed
  boundaries. Uniform `+254` and `-254` distributions over all three RGB lanes
  are accepted, while the pre-existing uniform `+255` and `-255` cases are
  refused. Broadening the forced set to include either adjacent value makes
  the corresponding acceptance control fail.
- The adjacent reports are producer-realizable. For `+254`, reference RGB
  `[0, 0, 1]` and candidate RGB `[254, 254, 255]` produce the recorded lane
  distributions and both frames contain a coloured pixel. Reversing those
  frames realizes `-254`. An exhaustive byte-pair check found two possible
  pairs per lane at each adjacent value and one possible pair per lane at each
  signed endpoint.
- Both new adjacent reports have two records, two views, one pass, one
  unmeasured view, zero fail, zero absent, one claimed verdict view, one
  coverage-unmeasured view and one `unstated-threshold` qualifier. The
  validator accepted the generated documents.
- I independently recomputed both aggregate hashes after serializer sorting.
  The `+254` report is
  `4bb2888aed605ed70f780deb6d7bce28d0f66571d834ed82a687e66b0252f2d2`.
  The `-254` report is
  `fb42c7edb48f6c45c113311110c52841dc800874b83b1ac84f23ac24e79f733c`.
  Stored and recomputed reference and candidate values matched in both
  reports.
- The catalogue now also refuses uniform two-pixel `+255` and `-255`
  distributions. The reports describe a one-row, two-column image, mark both
  columns touched, put two samples in every image, informative and full lane,
  and keep the regional summaries exact. Black reference with white candidate
  realizes the positive distributions. White reference with black candidate
  realizes the negative distributions. Every source frame is necessarily
  monochrome, so the false `monochromeFrame` value is the deliberate and
  correct refusal target.
- Both multipixel reports retain internally correct
  `sha256-rgba8-run-v1` aggregates. Stored and independently recomputed values
  matched at
  `311c307457058b6cfefff79bdeb407dfb6056185b81937159771e4d998a9de3b`
  on both sides. Restricting the validator to forced regions of one pixel
  makes both new refusal probes miss their declared reason and fail the
  harness.
- The generated runbook carries the multipixel refusals at rows 234 and 235
  and the adjacent acceptance controls at rows 254 and 255. The delivery note
  in `AS_BUILT.md` describes the same numeric and multiplicity boundaries.

## Predicate and partition mutation audit

- I tested six independent validator mutations in disposable clones. The
  catalogue rejected a forced set widened to `+254` and `-254`, a forced set
  limited to one-pixel regions, a positive-only endpoint check, an
  any-partition refusal, an image-only composition and a full-histogram
  composition. Each mutation changed at least one expected result or missed
  the declared refusal reason.
- The complete two-partition state matrix remains preserved. Forced image with
  mixed background, mixed image with forced background and both partitions
  mixed accept. Both partitions forced with the same sign or opposite signs
  refuse. The no-background controls preserve each signed endpoint and the
  mixed endpoint state.
- Lane equality is still required independently of the endpoint value.
  Existing histogram shape, signed range, count, summary, composition,
  channel disagreement and touched-geometry probes remain present and green.
  The report contract continues to refuse unknown, duplicate, missing and
  invalid schema states.
- I repeated the direct spatial enumeration through frames and image
  rectangles up to 4 by 4. All 3,162,596 region, touched-set and
  three-lane-count cases agreed with the validator's forced-region predicate.
  The adjacent byte-pair witness also confirms the strict numeric boundary in
  both directions.

## Whole-sprint closure and measured verification

- I reviewed all 66 changed files in the sprint diff, comprising 10,142
  insertions and 500 deletions, and audited the 18 prior sprint review records.
  The identity, input-safety and verdict remediations remain closed. The
  schema, derivation, authority, informative-region and spatial-feasibility
  remediations remain closed. The monochrome direction, regional composition,
  exact numeric boundary and multiplicity remediations now have matching
  standing evidence.
- `tools/oracle/src/frame.rs` still computes candidate minus reference per RGB
  lane, partitions image and background in the producer loop, records exact
  touched indices and excludes alpha. `tools/oracle/src/attribution.rs` sets
  `monochromeFrame` only when both complete frames have equal RGB lanes at
  every pixel. The schema, serializer and ledger validator retain the same
  meaning.
- The current 99-view comparison report passed an independent ledger read. It
  contains 71 claimed pass views, 28 unmeasured views, zero fail and zero
  absent, with digest
  `738ecac020ca5ae02101fa6206fb88b2e5da77dd70b991107508aff603cbb6a0`.
- `bin/ocelli.sh test ocelli-oracle` passed 145 tests across the library,
  comparator binary and integration suites.
- The deep guard profile passed 379 refusal probes, 55 acceptance probes and
  48 controls. The census found 790 claimed refusal sites, 434 catalogue
  probes and zero sites watched by nothing.
- The quirk gate passed 35 tests. The quirk-mutation gate drove three
  controlled mutations and six fixture-binding mutations red. The CI gate
  proved all 26 floor gates on pull request and push.
- Commit `067082061aa4d6ffedb1ea23c86a9a6fa24bffab` passed
  `check-commit --require-corpus` with corpus pass and a matching tree.
  `git diff --check
  7c5e29c..067082061aa4d6ffedb1ea23c86a9a6fa24bffab` also passed.
- No defect, smell or nitpick was found.
