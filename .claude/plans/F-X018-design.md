# F-X018, Make close-preflight see the sprint review, and key its verification on the tree it is about

**Status**: approved
**Epic ref**: Y1.13
**Sprint**: S04
**Estimate**: 1w

## Normative source, transcribed

_The source uses em-dashes. They are written as hyphens here because the prose
gate covers plans. The tracked HLD remains authoritative._

From `docs/hld/24-agent-code-standards.md`, section 27:

> Given agent-assisted delivery, review bandwidth - not generation speed - is
> the binding constraint, and reviewing generated code is slower per line than
> reviewing a colleague's because intent cannot be inferred. These rules exist
> to move as much of that burden as possible onto machines.

From section 27.2:

| **#** | **Rule** | **Why** |
|----|----|----|
| R6 | Provenance trailer on every commit | Cheap now; a retrofit across sixty thousand lines is not, and a device pathway may require it |

From `docs/hld/08-validation-architecture.md`, section 11, with source
punctuation normalised:

> Every pull request renders the corpus in CI. Every field bug becomes a
> permanent fixture. In production, shadow mode renders both libraries and
> alerts on divergence - the oracle running against real clinical traffic,
> and the same corpus a regulatory submission would want to see.

Deviation D-04 moves corpus rendering out of CI. Its compensating control is
verification evidence bound to the exact tree, later carried by the
`Ocelli-Verify` trailer.

## What the specification does not cover

The HLD does not define `.claude/scratch/SNN-run.json`, sprint-scope review
records, `close-preflight`, or the choice of a Git tree hash. The repository
workflow already defines a clean review as a claim about a tree and
`verify_ledger.py` already uses `git write-tree`. This story reuses that exact
identity rather than introducing a second hash convention.

Old run-state files have no sprint-review collection and verification entries
have no tree field. Reads must remain compatible so the active S04 state can
be upgraded by recording new evidence. Missing fields never count as clean or
current evidence.

## Approach

1. Add top-level `sprint_reviews` to newly initialised run state. Add a
   `record-sprint-review` command that records pass, defect, smell and nitpick
   counts plus `git write-tree`. Use `setdefault` on load or write so the
   already-created S04 state upgrades without manual editing.
2. Add the current staged-tree hash to every new `record-verification` entry.
   Do not infer a tree for legacy entries.
3. Change `close-preflight` to require the latest sprint-scope review to have
   zero defects and zero smells and to name the current staged tree. Require
   the latest passing sprint-profile verification to name that same tree.
   Per-feature reviews remain required independently.
4. Update `/run-sprint` so each whole-sprint microscope pass is recorded at
   sprint scope, and so remediation is followed by another consolidated
   verification before readiness. Update `/microscope` and `/close-sprint` to
   state the recorded-tree requirement explicitly.
5. Add guard probes for an absent sprint review, a dirty review, a stale review
   tree, a stale verification tree, and current clean evidence. The probes use
   disposable run state and a staged tree, never the real scratch file.

Anticipated implementation write set, exactly:

- `scripts/sprint_workflow.py`
- `.claude/commands/run-sprint.md`
- `.claude/commands/microscope.md`
- `.claude/commands/close-sprint.md`
- `.agents/skills/run-sprint/SKILL.md`
- `.agents/skills/microscope/SKILL.md`
- `.agents/skills/close-sprint/SKILL.md`
- `scripts/guards/catalogue.py`
- `ci/guard-probe-budget.json`
- `docs/lld/guards.md`

The live `.claude/scratch/S04-run.json` is gitignored state and is not an
implementation write. The first new review or verification record upgrades it.

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
| unit | Legacy run state loads but cannot satisfy a current-evidence preflight | `scripts/guards/catalogue.py` sprint-lifecycle probes |
| unit | Missing, dirty and stale sprint reviews refuse close-preflight | sprint-lifecycle probes |
| unit | A passing verification for another tree refuses close-preflight | sprint-lifecycle probes |
| unit | Current clean per-feature reviews, sprint review and verification pass together | accept control in the guard catalogue |
| mutation | Changing the index after recording review or verification makes preflight red | guard probes using a disposable repository |

No pixel or geometry arithmetic is added, so no DICOM fixture applies.

## Parity surface covered

None. Appendix B has no row whose `Covered by` value is Y1.13.

## Deviations

D-04 is strengthened by exact-tree evidence. No new deviation is needed.

## LLD impact

- `docs/lld/guards.md`

## Open questions

None.
