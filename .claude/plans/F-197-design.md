# F-197, volume from a gantry-tilted stack

**Status**: approved
**Epic ref**: E38.7
**Sprint**: S43
**Estimate**: 3w

## Normative source, transcribed

`Volume::assemble` in `ocelli-volume` refuses `ShearedVolume` when the lateral part of an inter-slice step is longer than 1e-6 mm. The duplicate bound is 1e-9 mm. A projected gap that departs from the median by more than 0.5 percent is `NonUniformSpacing`. The code comment states that shear is measured from geometry and never from a gantry tilt tag. Those three numbers stay.

`docs/hld/` describes volume construction and does not give a second constructor for a tilted stack. F-020 and F-049 already record that tilt is geometry, not the tag.

## What the specification does not cover

The HLD does not say whether a tilted stack is resampled onto an orthogonal grid. This plan does not resample. Pixels stay in the slice they arrived in. A new `TiltedVolume` records one lateral step, in millimetres, shared by every pair of neighbouring slices.

`Volume::assemble` is unchanged. The same tilted input still returns `ShearedVolume` from that function.

## Problem and reproducer

Two slices whose step is 3 mm along the normal and 3 mm across the row axis are a real gantry tilt. `Volume::assemble` refuses them. There is no constructor that keeps both components.

## Scope, API and compatibility

`TiltedVolume::assemble` accepts a `VolumeCandidate` when:

- the orthogonal checks other than shear succeed, including the 1e-9 mm duplicate bound and the 0.5 percent spacing rule
- every inter-slice lateral vector agrees with the first within 1e-6 mm
- the shared lateral length is greater than 1e-6 mm

A stack that is orthogonal still belongs to `Volume::assemble`. `TiltedVolume::assemble` returns unavailable for a lateral step at or under 1e-6 mm, so the two constructors do not both accept one stack.

The value stores the voxel buffer, the projected spacing, and the lateral step vector. It does not drop the lateral component.

Out of scope: reading the gantry tilt tag, changing `Volume::assemble`, and drawing the tilted volume.

## Consumer and provider map

| Consumer / entry point | Concrete provider | Caller context / policy | Observable output |
|------------------------|-------------------|-------------------------|-------------------|
| Example tilt section | `Volume::assemble` and `TiltedVolume::assemble` | The 3 mm by 3 mm synthetic pair | Orthogonal refusal, tilted spacing 3 mm, lateral length 3 mm |
| Existing volume callers | `Volume::assemble` | Unchanged | `ShearedVolume` for the tilted pair |

## State transitions and invariants

Stateless assembly. A failed tilted assembly returns no volume. A later orthogonal `assemble` of the same candidate still refuses.

## Ownership, commit and recovery

The returned volume owns its voxel store. No GPU texture is created.

## Numbered acceptance inventory

| ID | Trigger and expected behavior | Test / observation | Wrong implementation rejected | Evidence stage / owner |
|----|-------------------------------|--------------------|-------------------------------|------------------------|
| AC1 | The 3 mm normal step and 3 mm lateral step is `ShearedVolume` from `Volume::assemble` | existing-style unit test | Accepting it as an orthogonal volume | implementation |
| AC2 | The same candidate is a `TiltedVolume` with projected spacing 3 and lateral length 3 | hand fixture | Resampling onto a square grid | implementation |
| AC3 | A second step whose lateral vector differs by more than 1e-6 mm is refused | unit test | Averaging the two shears | implementation |
| AC4 | A gap that breaks the 0.5 percent rule is still refused | unit test | Relaxing the tolerance | implementation |
| AC5 | An orthogonal stack is refused by `TiltedVolume::assemble` | unit test | Returning two successful volumes | implementation |
| AC6 | The example page shows the orthogonal refusal and the tilted spacing | route test | No control on the page | integrated |

## Approach

Implement `TiltedVolume` beside `Volume` in `ocelli-volume`. Share the sort and the spacing checks. Branch only on the shear test. Do not copy pixel samples into a new grid.

## Dependencies, write ownership and resources

Depends on F-051 and F-049, both done. Writes `ocelli-volume`, the example server, the example page, the volume LLD, the changelog and the sprint ledgers. No GPU.

## Verification and evidence reuse

`bin/ocelli.sh test ocelli-volume` for the new cases. The 1e-9 mm and 0.5 percent tests of `Volume::assemble` stay green. Feature profile at story close.

## Boundary and tier

- wasm-bindgen: not touched
- Pixels across the boundary: no
- Render-loop allocation: none
- unsafe: none
- Tier A (WebGPU): n/a. This story does not draw
- Tier B (WebGL2): n/a
- Tier C (CPU): n/a

## Tests

| Category | What it proves | Where |
|----------|----------------|-------|
| fixture | Lateral 3 mm and normal 3 mm, citing image-plane geometry rather than a tilt tag | `ocelli-volume` tests |

## Parity surface covered

none

## Deviations

none

## LLD impact

One paragraph in `docs/lld/volumes.md`. The sentence that shear is refused stays true for `Volume::assemble`.

## Open questions

none
