# F-010 review, pass 5

**Reviewed**: the fully staged index on `work/f-010-claude`, 44 files,
+7980/-41, after four rounds of remediation
**Result**: 3 defects, 1 smell, 5 nitpicks

Seventeen mutations run, every one restored and confirmed by digest. Ten of the
eleven required gate commands run and reported.

## Defects

### D1, the LLD's layout table was stale in two rows, one of them the module round 4 added

**Where**: `docs/lld/oracle.md`, the `src/` and `tests/` rows
**What**: `src/` listed nine modules where there are ten, missing `pins.mjs`,
the module that resolves every installed version. `tests/` said five suites
where there are six. `git grep` found `pins.mjs` named in no tracked
documentation at all.
**Why it matters more than a typo**: this is the class pass 4 fixed as a
nitpick for `paths.mjs` and re-introduced for `pins.mjs` in the same round.
**Fixed**: both rows corrected. `ls tools/oracle/src` is ten `.mjs`,
`ls tools/oracle/tests` is six `*_test.mjs` plus the runner, and `npm test`
reports 76.

### D2, the sentence justifying the name predicate was false of every tree that reaches it

**Where**: `docs/lld/oracle.md` and `tools/oracle/src/pins.mjs`
**What**: both said `dist/esm/package.json` markers "sit in the middle of
several of these trees", scoped to the six packages the walk reaches.
**Why it is wrong**: measured, none of those six contains any nested
`package.json`. Each has exactly one manifest, at its own root, and
`require.resolve` lands in that root, so the walk terminates on its first step
with no marker anywhere on the path. The five trees that DO carry the marker
all expose `./package.json` and take the direct route. Pass 4 measured this
itself while raising the smell, fixed the testing gap, and carried the false
claim into the new module's doc comment.
**Evidence**: `find node_modules/<each of the six> -name package.json` returns
exactly one hit each. The five `@cornerstonejs/*` packages return two.
**Fixed**: both now say what is true. The predicate is a guard against a shape
that exists elsewhere in the same tree and would reach the walk the day an
exports map moves a package from one route to the other, and
`tests/pins_test.mjs` builds that shape explicitly so it is exercised before
the tree needs it.

### D3, "because the four real series do too" was false

**Where**: `docs/lld/oracle.md`
**What**: only two of the four real directories carry the `series` token.
`real/ct_cmb_mml` has 27 and `real/mr_eay131` has 15. `real/dx_varepop` and
`real/us_cmb_crc` are single instances and carry none. The count 62 is right
and is 20 plus 42, but a reader who checks the stated reason computes 20 plus
44 and cannot reconcile it.
**Evidence**: `awk` over the manifest grouping by directory, 27 and 15 with the
token, 1 and 1 without.
**Fixed**: the sentence now gives the arithmetic.

## Smell

### S1, `truncate`'s expectation was the boundary name, which the file says it must not be

**Where**: `tools/oracle/src/faults.mjs`
**What**: the module comment states the discipline, "`expect` below is a
fragment of the specific message and not just the boundary name". Five of the
six honoured it. `truncate` pinned `"decoded:"`, which is the literal prefix of
the driver's generic problem line, so ANY decode failure of that row satisfied
it: a corrupted corpus row, a codec regression, a loader change.
**Why it would bite**: the self test would report "red at decoded, as required"
for a `truncate` fault that had stopped truncating, which is the guard-nobody-
has-watched-fail shape the six injections exist to close.
**Fixed**: it now pins `Request more than currently allocated buffer`, the
reader's own message for a buffer that ends mid-element. Re-observed on the
full gate: still red at `decoded`, as required.

## Nitpicks

- `sidecar_test.mjs` still carried `voi.fallback`, removed from the committed
  spec in round 1. **Removed**, so nothing in the tree implies the key is
  supported.
- `FULL_RANGE` is exported for the tests while production compares the string
  literal. Left: the resolver's "every committed row resolves" test would catch
  a divergence.
- `.claude/commands/parity.md` and `docs/RELEASE.md` still say v5.8.9 while
  `CLAUDE.md` now says 5.8.2 with a D-11 pointer. **Left, and handed to the
  integrator**: both files are outside this story's region and the plan named
  only `CLAUDE.md` and `README.md`.
- The plan's claim that `README.md` states the parity target as v5.8.9 is false,
  the file carries no version string. **Reported as a plan defect**, no code
  change needed.
- `--sprint` and `--all` remain character-identical arms in `bin/ocelli.sh`.
  **Left, and handed to the integrator**: another story is editing that
  function and the arms are outside this story's region.

## Verified clean

- **Pass 4's D1 to D4 and S1 to S3 all hold**, checked against the tree.
  `--partial`'s behaviour was traced statement by statement: it suppresses
  exactly four completeness branches and nothing else, and a sidecar whose
  attributes disagree with pydicom is still caught in a partial directory,
  proved by mutating `rescaleIntercept`, `photometricInterpretation` and, on a
  real row, `bitsStored`, which went red with both values withheld.
- **Runbook rows 20, 21 and 22 were executed and their observed columns are
  accurate.** Row 20 also has a control: the same PNG staged at `docs/x.png`
  passes, which is what makes the path prefix the mechanism rather than the
  image. Row 23 was exercised for one of its six injections, which is also the
  evidence for S1.
- **The arithmetic was re-derived from the specification, not read from the
  code.** `fullRange`'s edges from PS3.3 C.11.2.1.2, the asymmetric `<=` and
  `>` unaltered. `minimumWidth` at 1 for LINEAR and `Number.MIN_VALUE`
  elsewhere, which is exactly `w > 0` for doubles, against C.11.2.1.3.1 and
  C.11.2.1.3.2. `canvasScale` re-derived from `parallelScale` and PS3.3
  C.7.6.2.1.1, and `reference_mono12`'s `parallelScale` of 16 was computed
  independently of the harness from 64 rows at 0.5 mm. Rounding is display only:
  `toFixed(6)` is applied after every comparison, never before.
- **Fixture provenance spot-checked against the generator**, not the harness:
  `ct_common`, `case_unsigned_16`, `case_monochrome1`, `case_multiframe`'s four
  literal arrays, `TRAP_ROWS`, `TRAP_COLS` and `NON_SQUARE_SPACING` all match.
- **Every number in the LLD and in `unsupported.json` was executed**: 91 rows,
  89 sidecars, 16 packages, 87 smaller than the canvas and exactly two larger,
  85 file windows and 4 colour, 16 low-information rows, 62 `series` tokens,
  and the 480 against 720 bytes of the YBR row under pydicom.
- **Guards and branches swept for anything nothing reaches.** All three pin
  cross-check branches driven red. `unsupported.json`'s `feature` is emitted.
  `render-params.json`'s `version` is asserted. `camera.mode` is enforced.
  `resolveServedPath`'s containment branch is genuinely reachable and covered
  three ways.
- **Boundary and tier**: no `wasm-bindgen`, no pixels across an Ocelli
  boundary, no wasm memory view, no render loop, no `queue.submit`, and all
  three tier rows answered `n/a` rather than omitted. No `as` cast, no
  `unsafe`, and the only Rust change is a doc comment.
- **Structure**: no new trait, generic, `Box<dyn>` or forwarding wrapper.
- Seventeen mutations: sixteen red as intended, and the seventeenth green by
  design, isolating `if field in expected` as the load-bearing half of the
  coverage check rather than decoration.
- Command results: `npm test` 76 of 76, the cross-read 89 and 9 and 2, the
  cross-read self test, `npx eslint .`, `prose_check.py` over 72 files,
  `staged_content_check.py --tracked`, `source_provenance_check.py` over 261
  files, `deviation_check.py` with 11 deviations, `sync_agent_skills.py
  --check` with 20 adapters, `backlog_check.py`, and `bash -n bin/ocelli.sh`,
  all exit 0.
- After remediation: `bin/ocelli.sh gate oracle` is ALL GREEN, 91 attempted, 89
  read back, 2 accounted for, identical digests across two passes, and all six
  faults red at their named boundary with the tightened `truncate` expectation.
