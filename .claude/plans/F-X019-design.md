# F-X019, Decide whether a CI step that is not guaranteed to run counts as CI running the gate

**Status**: approved
**Epic ref**: Y1.14
**Sprint**: S04
**Estimate**: 1w

## Normative source, transcribed

_Transcriptions preserve the words in the HLD, with em-dashes and prose
semicolons normalised because plans are prose-checked. The tracked HLD remains
authoritative._

### `docs/hld/08-validation-architecture.md`, section 11

> Every pull request renders the corpus in CI. Every field bug becomes a
> permanent fixture.

### `docs/hld/12-workspace-and-build.md`, section 15.3

> Decision D2 is worthless unless it is enforced. This runs on every pull
> request.

### `docs/hld/24-agent-code-standards.md`, section 27.2

> | R6 | Provenance trailer on every commit | Cheap now, a retrofit across
> sixty thousand lines is not, and a device pathway may require it |

## What the specification does not cover

The HLD does not define whether a command on the right-hand side of `&&`
counts as guaranteed under GitHub Actions' `bash -e` execution. The repository
currently documents `cd x && gate` as legitimate while its measured residue
also accepts `false && gate` followed by a successful command.

## Approach

A gate counts only when either it is not conditional on a preceding `&&`
command, or failure of that preceding command necessarily makes the whole step
fail. Model the complete AND-list and the statements that follow it under the
measured `bash -e` shell. Accept a terminal `cd x && gate` because failure of
`cd` fails the step and success reaches the gate. Refuse or discredit
`false && gate` when a later successful statement can swallow the list status.

Replace the acceptance test that asserts the hole remains open with tests for
both sides of the decision. Extend the flat-shape cross-check against real bash
so future separator changes are compared with execution rather than argued
from syntax alone.

Anticipated write set: `scripts/ci_floor_check.py`,
`scripts/tests/test_guard_readers.py`, `scripts/guards/catalogue.py`,
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
| unit | A terminal successful-prefix AND-list counts and a swallowed failed-prefix AND-list does not | `scripts/tests/test_guard_readers.py` |
| differential | The scanner's answer matches `bash -e` for every accepted AND-list shape | `scripts/tests/test_guard_readers.py` |
| guard mutation | Moving a required gate behind a swallowed `&&` condition makes the CI guard red | `scripts/guards/catalogue.py` through `scripts/guard_probe.py` |

## Parity surface covered

None.

## Deviations

D-04 is relevant because CI gate coverage is one of its compensating controls.
No new deviation.

## LLD impact

Update `docs/lld/guards.md` to state the AND-list rule and remove the recorded
residue.

## Open questions

None. A gate is counted only when failure to reach it cannot leave the step
green.
