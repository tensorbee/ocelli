# S16 Codex preparation review, pass 1

**Reviewed**: staged tree `6b562be784f4365e7781869b5c511e9e0a1162a8`
**Author / reviewer**: Codex authored the preparation. xAI model reviewed it.
This reviewer implemented F-X022 and F-X023 and does not approve that code here.
**Result**: 0 defects, 1 smell, 0 nitpicks in the inspected preparation
**Date**: 2026-10-18

## Acceptance challenged

This pass covers Codex's shared preparation beyond the later instruction
refinements already reviewed. The scope is the original workflow and command
text, the five design plans, and `verify_ledger.py invalidate` with its
focused test. It is not a whole-sprint review and it is not approval of the
two implementation repairs.

## Evidence

Commands below were read against the staged tree and the hook sources. The
sprint profile was not rerun. Codex had already copied the existing passing
receipt into ignored S16 run state.

| Claim | Source checked | Observation |
|-------|----------------|-------------|
| commit-msg writes the trailers | `.githooks/commit-msg` | Appends `verify_ledger.py trailer` output. Refuses a hand-written `Ocelli-Verify`. |
| pre-commit warns only | `.githooks/pre-commit` | Content, provenance and prose checks refuse. A missing receipt prints `NOTE` and exits 0. |
| pre-push checks the head | `.githooks/pre-push` | `check-commit` refuses a head with no matching trailer. |
| invalidate drops one tree | `scripts/tests/test_sprint_workflow.py` `VerificationAttempt` | Assert passes, invalidate makes that tree fail, the other tree remains, a second invalidate still returns 0. |
| F-048 spacing example | PS3.3 C.7.6.2.1.1 and `PixelSpacing` order | IPP `(10,20,30)`, row 2 mm, column 0.5 mm, index `(2,3,0)` is `(11,26,30)`. |
| First texel centre | `crates/ocelli-viewport/tests/view.rs` | Image `(0.5,0.5)` is texel `(0,0)`. Fragment position uses that same space at DPR 1. |
| Outside image | `crates/ocelli-render/src/graph.rs` | A mapped fragment outside the texture returns opaque black `(0,0,0,255)`. |
| State layout | HLD section 17.4 | Camera at 0, centre 64, width 68, slice 72, count 76, flags 80, pad 84, total 96. |
| Cache event kinds | `ocelli-cache` | Kinds 1 through 5 are already pinned. |
| DPR size | `device_extent` and F-039 | The bitmap is `round(css * dpr)` per axis. `Pt<Canvas>` is still named as CSS pixels by HLD section 16. |
| Render crate edges | `ocelli-render/Cargo.toml` | The crate already depends on `ocelli-pixel` and `ocelli-viewport`. |

## Defects

None in this scope.

## Smells

### S1, the sprint note says no story begins blocked

**Where**: `docs/sprints/CURRENT_SPRINT.md`, dependency section
**What**: That section says no S16 story depends on another S16 story, so no
story begins blocked. The F-048, F-101 and F-X003 drafts require serial
execution because they share the mapping, the boundary and the raster.
**Why it is a smell**: The later audit paragraph corrects the reading, but the
earlier sentence can still be executed as permission to implement the three
stories together.
**Evidence**: Backlog `Depends on` cells have no intra-sprint edge. The three
drafts state the source-level order.
**Scope**: Preparation prose. The drafts themselves keep the serial order.

## Verified clean

`invalidate` removes only the selected receipt and does not invent a pass.
The hook bodies match the canonical command text. F-X022's design keeps the
selected-chain refusal and does not claim DICOM forbids another view.
F-X023's design uses allocation membership, completed backlog status, and the
existing carry path. Its closing "not already present" paragraph is the
pre-implementation instruction and is not a claim about the current tree.
F-X003 leaves the missing CPU-baseline host unmet and does not invent a
threshold. No patient data, tolerance change or new deviation is proposed.

## Limitations and handoff

F-X022 and F-X023 code was not re-reviewed. Codex's scoped passes stand for
that code. Shared-preparation AC4 to AC7 were read here as Codex's text.
This pass does not mark any story done and does not record a sprint review.

Routine choices taken from the code, still inside draft plans until the
operator answers the material questions:

- DICOM index `(0,0,0)` stays IPP. No half-voxel is added in patient space.
- The public index adapter converts in index space, `image = index + 0.5`.
- A singular measurement map returns an error. It does not use the camera
  matrix rank-one fallback or centre matrix.
- Command buffers are read during the call and are not taken from the caller.
  A bulk commit releases its one outstanding allocation on success and on
  refusal. Cache kinds 1 to 5 stay unchanged.
- CPU samples outside the image are opaque black before the LUT. The carried
  LINEAR range finding is not clamped. ICC stays unavailable.

The verified index remains `6b562be784f4365e7781869b5c511e9e0a1162a8` until a
later approved change is staged. This file is not part of that receipt.
