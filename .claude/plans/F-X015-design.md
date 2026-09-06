# F-X015, Execute the skills' worked examples, because the skills gate asserts nothing about their numbers

**Status**: approved
**Epic ref**: Y1.10
**Sprint**: S04
**Estimate**: 1w

## Normative source, transcribed

From `docs/hld/15-lut-chain.md`, section 18.2:

```text
// PS3.3 C.11.2.1.2 -- LINEAR. Requires w >= 1.
// c' = c - 0.5 ; w' = w - 1
// x <= c' - w'/2 -> ymin
// x > c' + w'/2 -> ymax
// else y = ((x - c') / w' + 0.5) * (ymax - ymin) + ymin
// PS3.3 C.11.2.1.3.2 -- LINEAR_EXACT. Requires w > 0.
// x <= c - w/2 -> ymin
// x > c + w/2 -> ymax
// else y = ((x - c) / w + 0.5) * (ymax - ymin) + ymin
// PS3.3 C.11.2.1.3.1 -- SIGMOID. Requires w > 0.
// y = (ymax - ymin) / (1 + exp(-4 * (x - c) / w)) + ymin
```

Section 18.3 gives this worked table:

| **Input (HU)** | **LINEAR** | **LINEAR_EXACT** | **Why this row** |
|----|----|----|----|
| −160 | 0.000 | 1.594 | LINEAR boundary is c'−w'/2 = −160 exactly; the comparison is \<=, so this clamps |
| 40 | 127.819 | 127.500 | The window centre. A 0.32 divergence no reviewer would see by eye |
| 240 | 255.000 | 255.000 | LINEAR upper bound is c'+w'/2 = 239, so 240 clamps |
| −60 | 63.910 | 63.750 | Mid-lower quarter; catches sign and slope errors |

Deviation D-13 corrects the first `LINEAR_EXACT` value to `0.000`, because
section 18.2 clamps at `x <= c - w/2`. The formulas remain authoritative.

From `docs/hld/24-agent-code-standards.md`, section 27.2:

| **#** | **Rule** | **Why** |
|----|----|----|
| R2 | Tests derive from the spec or the oracle, never from reading the implementation | An agent asked to test a function will assert what it does, not what it should do |
| R3 | Every function doing pixel arithmetic needs a fixture test with hand-computed values, citing the DICOM section | This is the defect class that reaches patients |

From section 27.3, with the source punctuation normalised:

> That a new test would actually fail if the code were wrong. Mutate one
> constant, re-run, confirm it goes red.

## What the specification does not cover

The HLD does not define executable-document markup, which fenced blocks are
safe and self-contained, how expected stdout is attached to a block, or which
interpreter executes it. It also does not make every Python fence executable.
Several tooling examples require a corpus path, pydicom, numpy, or filesystem
writes and must not be guessed into the floor gate.

## Approach

1. Add narrow HTML markers in the canonical skill sources around numeric,
   self-contained examples. Each marker names `python3`, whether stdout is
   compared with the following output fence, and whether success is asserted
   by the block itself. Mark the VOI block and stored-value assertion block in
   `dicom-tooling`. Add a self-contained check beside the corrected VOI table
   in `dicom-expert`, so the table that agents read is checked independently
   too.
2. Add `scripts/skill_examples_check.py`. It scans every canonical
   `.claude/skills/*/SKILL.md` and parses all files before executing any block.
   Refuse a marker that is not exact and at column zero. Permit only `python3`
   through an argument vector with no shell. Run each block with a timeout in
   its own temporary directory and minimal environment. Bound every failure
   diagnostic. Byte-compare stdout examples with their declared output fence,
   and require assertion examples to exit cleanly without output. Execute no
   unmarked fence.
3. Add adversarial parser and execution tests. Cover missing output, changed
   output, a nonzero example, timeout, bounded diagnostics, duplicate ids,
   empty code, empty selection, near-markers, unknown modes and interpreters,
   parse-before-run ordering, process isolation, and arbitrary CLI selectors.
   Mutate one expected digit and observe the check red before claiming it.
4. Run adapter sync, the new check and its unit suite in exact order from the
   existing `skills` gate. Invoke that named gate from CI. Assert the exact arm
   from the already gate-reached catalogue suite, so removing the checker or
   its unit suite cannot leave the named gate green. Add the checker's refusals
   and expected-digit mutation probe to the guard catalogue.
5. Regenerate the two changed Codex adapters, the guard budget and the runbook
   table. Validate both canonical skill folders and update the LLD index.

Implementation write set, exactly 15 paths:

- `.claude/skills/dicom-expert/SKILL.md`
- `.claude/skills/dicom-tooling/SKILL.md`
- `.agents/skills/dicom-expert/SKILL.md`
- `.agents/skills/dicom-tooling/SKILL.md`
- `scripts/skill_examples_check.py`
- `scripts/tests/test_skill_examples_check.py`
- `bin/ocelli.sh`
- `.github/workflows/ci.yml`
- `scripts/guards/catalogue.py`
- `ci/guard-probe-budget.json`
- `scripts/tests/test_guard_catalogue.py`
- `docs/runbooks/guard-verification.md`
- `docs/lld/guards.md`
- `docs/lld/README.md`
- `.claude/plans/F-X015-design.md`

The initial ten-path estimate omitted the plan correction, the named CI
registration, the existing exact-registration holder, the generated runbook,
and the LLD index. Those paths are required by the approved mechanism, so the
measured write set is fifteen.

Pass-1 review measured four corrections within that set:

- Execute SIGMOID at a value whose exponent reduces independently to `+1`,
  assert its static output, and exercise both a positive width and the
  zero-width refusal. Keep exponent-sign and width-predicate mutations in the
  catalogue so either semantic error remains observable.
- Anchor the canonical skills root lexically to this repository before path
  resolution. Refuse a root symlink and a child symlink escape as separate
  cases.
- Recognise valid blockquoted and list-nested unmarked fences as inert while
  treating an exact column-zero declaration after either container as live.
- Keep the independent expert table check beside the table, but express its
  four rows as compact exact rational arithmetic instead of duplicating the
  two verbose VOI function bodies.

Pass-2 review measured three further corrections within the same mechanism:

- Give an active list container precedence over the top-level fence grammar,
  including valid two-space and three-space continuations. Dedenting from an
  unclosed list-contained fence must make a column-zero declaration live.
- Apply CommonMark's backtick-fence info-string restriction. A backtick in
  that info string makes the line ordinary text and cannot hide a live marked
  example. Tilde fences retain their separate rule.
- Keep the expert check compact, but evaluate `-160` and `240` against both
  LINEAR and LINEAR_EXACT boundary predicates before deriving their row
  values.

The implementation write set remains fifteen paths. The two staged review
records bring the frozen pass-2 candidate to seventeen tracked paths.

## Boundary and tier

- wasm-bindgen: not touched
- Pixels across the boundary: no
- Render-loop allocation: none
- unsafe: none
- Tier A (WebGPU): n/a
- Tier B (WebGL2): n/a
- Tier C (CPU): n/a

## Tests

| Category | What it proves | Where |
|----------|----------------|-------|
| unit | Marker parsing fails closed and only declared examples execute | `scripts/tests/test_skill_examples_check.py` |
| fixture | The VOI outputs are computed from PS3.3 C.11.2.1.2, C.11.2.1.3.2 and C.11.2.1.3.1, including corrected D-13 row one | marked skill examples run by `scripts/skill_examples_check.py` |
| fixture | Stored-value sign extension keeps the three hand-computed cases already cited by the skill | marked `stored_value` example |
| mutation | Changing one expected digit fails at the named example | guard catalogue probe |

## Parity surface covered

None. Appendix B has no row whose `Covered by` value is Y1.10.

## Deviations

D-13 is consumed by the corrected expected value. No new deviation is needed.

## LLD impact

- `docs/lld/guards.md`

## Open questions

None.
