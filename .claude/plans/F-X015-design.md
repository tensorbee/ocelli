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
   `.claude/skills/*/SKILL.md`, refuses malformed or unmatched markers, runs
   each marked block in a temporary directory with a deterministic locale,
   and byte-compares stdout with its declared output fence. No unmarked fence
   is executed.
3. Add unit tests for missing output, changed output, a nonzero example, a
   duplicate id, and an empty selection. Mutate one expected digit and observe
   the check red before claiming it.
4. Run the new check from the existing `skills` gate after adapter sync. Add
   its refusals and mutation probe to the guard catalogue. Regenerate the two
   changed Codex adapters from their canonical sources.

Anticipated implementation write set, exactly:

- `.claude/skills/dicom-expert/SKILL.md`
- `.claude/skills/dicom-tooling/SKILL.md`
- `.agents/skills/dicom-expert/SKILL.md`
- `.agents/skills/dicom-tooling/SKILL.md`
- `scripts/skill_examples_check.py`
- `scripts/tests/test_skill_examples_check.py`
- `bin/ocelli.sh`
- `scripts/guards/catalogue.py`
- `ci/guard-probe-budget.json`
- `docs/lld/guards.md`

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
