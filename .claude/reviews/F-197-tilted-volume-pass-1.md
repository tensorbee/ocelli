# F-197 review, pass 1

**Reviewed**: uncommitted F-197 on `sprint/s43`. HEAD was `9c67203`.
**Author / reviewer**: implementation by the sprint host. Review by Claude Code, read only, with no shell.
**Result**: 3 defects, 3 smells, 4 nitpicks

## Acceptance challenged

AC1 holds for the 3 mm normal step and the 3 mm lateral step. AC2 holds for projected spacing 3, lateral `(3, 0, 0)`, and a cleared 72-byte store. AC3 holds for a 2e-6 mm disagreement and would not hold for a disagreement between 1e-6 mm and 2e-6 mm. AC4 holds for a 7 mm gap and does not reject a tolerance moved from 0.5 percent to 50 percent. AC5 holds at a zero lateral step and at exactly 1e-6 mm. It does not hold for a step of 2e-6 mm. AC6 holds on the `/tilt` route. The page test renders a reply of 4 and 5. The live page was not opened.

## Evidence

Claude read the plan, `prepare`, the tilted tests, the route, the page and `docs/lld/volumes.md`. No shell was available, so the unchanged `Volume::assemble` behaviour was judged by reading the `Shear::Refuse` arm. The implementer had already seen the 1e-6 agreement mutation go red and the volume suite stay green.

## Findings

D1. The half-percent fixture uses gaps 3, 3, 7 and 3. Moving `0.005` to `0.5` would still refuse that stack. The exact-duplicate fixture would also stay red if `1e-9` moved to `1e-3`. AC4 names a relaxed tolerance as the wrong implementation, and these fixtures do not reject it.

D2. The 1e-6 mm case checks only the equal bound. Moving the orthogonal refusal from `> 1e-6` to `> 1e-3`, or the tilted refusal from `<= 1e-6` to `<= 1e-3`, leaves every current test green. A stack can then be accepted by both constructors.

D3. `docs/lld/volumes.md` says a shorter lateral step is `OrthogonalStack`. The code refuses a step at or under 1e-6 mm. Shorter also reads as shorter than the neighbouring step, which is the agreement check above it.

S1. The same 1e-6 mm bound is three separate literals. The two constructors partition the stacks only while the refuse test and the keep test stay equal. Nothing fails if they drift.

S2. `TiltedVolume` drops `order`, `origin` and `direction` after `prepare` computes them. The lateral sign is the sorted step. A caller that fills the cleared store by arrival index on a descending stack points that step the wrong way. `Volume::order` exists so that projection is not derived again outside this crate.

S3. The approved agreement bound is 1e-6 mm. Image Position Patient is a decimal string, and a real tilt of a few degrees can disagree by more than a micrometre after ordinary rounding. The drawing story can then see `InconsistentShear` for a stack this constructor was meant to keep. The number is the approved plan and this pass does not move it.

N1. The route treats any tilted error on the orthogonal stack as success. It should require `OrthogonalStack`.

N2. The page test does not assert that the stack viewport is still mounted.

N3. The types table has no `TiltedVolume` row, and the shear paragraph says this story refuses the stack beside a paragraph that stores it.

N4. `VolumeError` is not `non_exhaustive`. Nothing is published.

## Remediation

The following pass reviews the repaired tree. A gap just over 0.5 percent is refused and a projected separation just over 1e-9 mm is not a duplicate. A 2e-6 mm lateral step is `ShearedVolume` from `Volume::assemble` and a tilted volume from `TiltedVolume::assemble`. The LLD names the at-or-under bound. One shared lateral constant is used by both constructors. The tilted value keeps `order`, `origin` and `direction`. The route matches `OrthogonalStack`. The page test still sees the stack viewport. The 1e-6 mm agreement bound stays.
