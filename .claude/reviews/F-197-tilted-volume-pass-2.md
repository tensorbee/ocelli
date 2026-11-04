# F-197 review, pass 2

**Reviewed**: uncommitted F-197 on `sprint/s43` after the pass 1 repairs. HEAD was `9c67203`.
**Author / reviewer**: implementation by the sprint host. Review by Claude Code, read only, with no shell.
**Result**: 0 defects, 0 smells, 0 nitpicks

## Acceptance challenged

AC1 still returns `ShearedVolume` for the 3 mm normal step and the 3 mm lateral step. AC2 keeps projected spacing 3, lateral `(3, 0, 0)` and a cleared store. AC3 refuses a 2e-6 mm disagreement. AC4 refuses 3 mm then 3.031 mm and keeps 3 mm then 3.02 mm. AC5 refuses a zero lateral step, an exact 1e-6 mm step, and accepts 2e-6 mm only on `TiltedVolume::assemble`. AC6 is the `/tilt` route. The page test renders a reply of 4 and 5 and still finds the stack viewport. The live page was not opened.

## Evidence

Claude read the plan, `prepare`, the tilted tests, the route, the page and `docs/lld/volumes.md`. No shell was run. The restored comparisons are `LATERAL_BOUND_MM` at 1e-6, the spacing factor `0.005` and the duplicate bound `1e-9`.

## Closed from pass 1

D1. The near spacing pair and the 5e-10 mm and 1e-8 mm duplicate pair sit on either side of the shared bounds. A spacing factor of 0.006 would accept 3.031 mm. A duplicate bound of 1e-6 mm would refuse the 1e-8 mm stack.

D2. A lateral step of 2e-6 mm is `ShearedVolume` from `Volume::assemble` and a tilted volume whose stored step is `(2e-6, 0, 0)`.

D3. The LLD says a lateral step at or under 1e-6 mm is `OrthogonalStack`.

S1. `LATERAL_BOUND_MM` is the only comparison value for the refuse check, the agreement check and the orthogonal split.

S2. `TiltedVolume` keeps `origin`, `direction` and `order`. Descending arrival stores order `[2, 1, 0]`, origin `(0, 0, 0)` and lateral `(3, 0, 0)`.

S3. The agreement bound stays at the approved 1e-6 mm.

N1. The route requires `OrthogonalStack`.

N2. The page test still finds the stack viewport heading.

N3. The types table names `TiltedVolume`, and the shear paragraph names `Volume::assemble`.

N4. `VolumeError` stays exhaustive. Nothing is published.

## Verified clean

The lateral step is the inter-slice vector minus its projection on the unit normal. The 3 mm fixture leaves `(3, 0, 0)`. The store is cleared at the source dimensions. Samples are not copied onto another grid. The gantry tilt tag is not read. No `unsafe` was added. The page does not replace `OcelliViewport`.

## Limitations and handoff

Claude did not run the tests. The implementer had already seen three restored mutations go red: the lateral bound moved to 1e-3, the spacing factor moved to 0.006, and the duplicate bound moved to 1e-6. The live page was not clicked. Drawing the tilted volume remains out of scope.
