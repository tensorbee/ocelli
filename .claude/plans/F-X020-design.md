# F-X020, Stop gen_sprint_plan.py's write mode silently overwriting a hand-curated SPRINT_PLAN.md

**Status**: approved
**Epic ref**: Y1.15
**Sprint**: S04
**Estimate**: 1w

## Normative source, transcribed

_Transcriptions preserve the words in the HLD, with em-dashes and prose
semicolons normalised because plans are prose-checked. The tracked HLD remains
authoritative._

### `docs/hld/24-agent-code-standards.md`, section 27

> Given agent-assisted delivery, review bandwidth - not generation speed - is
> the binding constraint, and reviewing generated code is slower per line than
> reviewing a colleague's because intent cannot be inferred. These rules exist
> to move as much of that burden as possible onto machines.

### `docs/hld/24-agent-code-standards.md`, section 27.2

> | R1 | Never re-type file content from tool output, edit in place with a
> script | Tool output truncates, and the truncation is silent |

> | R2 | Tests derive from the spec or the oracle, never from reading the
> implementation | An agent asked to test a function will assert what it does,
> not what it should do |

## What the specification does not cover

The HLD does not define the lifecycle of the generated sprint plan. The
repository does: generation is bootstrap-only, then the document is
hand-curated and `--check` verifies its structured planning data.

## Approach

Change the bare invocation of `scripts/gen_sprint_plan.py` to refuse when
`docs/sprints/SPRINT_PLAN.md` already exists. Add an explicit `--force` flag
for the exceptional bootstrap or intentional full regeneration path. Keep
`--check` read-only and unchanged in meaning. The refusal names the protected
file and the two available explicit modes.

Add a temporary-directory unit test proving the bare invocation cannot alter
an existing file, `--force` can create or replace one when deliberately
requested, and `--check` remains read-only. Register a standing guard probe
for the new refusal and update the census budget in the same change so the
guard cannot become an unowned refusal.

Refactor `main` to accept explicit allocation and plan paths, defaulting to the
current repository constants from the CLI entry point. Tests call that seam
with temporary paths. This is dependency injection into one concrete function,
not a new trait or wrapper.

Anticipated write set: `scripts/gen_sprint_plan.py`,
`scripts/tests/test_gen_sprint_plan.py`, `scripts/guards/catalogue.py`,
`ci/guard-probe-budget.json`, and `docs/lld/guards.md`.

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
| unit | Bare write refuses an existing plan and preserves its bytes | `scripts/tests/test_gen_sprint_plan.py` |
| unit | `--force` is the only replacement path and `--check` remains read-only | `scripts/tests/test_gen_sprint_plan.py` |
| guard mutation | Removing the existing-file refusal makes its standing probe red | `scripts/guards/catalogue.py` through `scripts/guard_probe.py` |

## Parity surface covered

None.

## Deviations

None.

## LLD impact

Update `docs/lld/guards.md` with the protected write mode and its standing
probe.

## Open questions

None.
