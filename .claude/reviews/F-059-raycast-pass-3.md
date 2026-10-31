# F-059 review, pass 3

**Reviewed**: working tree on `sprint/s19` at `42336565115749daa4f20e8e04a18f1cb72729f2`. Unstaged product paths are `crates/ocelli-render/src/graph.rs`, `crates/ocelli-render/src/lib.rs`, `crates/ocelli-render/tests/graph.rs`, `crates/ocelli-render/tests/view.rs`, `crates/ocelli-volume/src/lib.rs`, plus untracked `crates/ocelli-render/shaders/raycast.wgsl`, `crates/ocelli-render/tests/raycast.rs` and `crates/ocelli-volume/tests/resident.rs`. This review file is not part of that product diff. Pass 1 and pass 2 were not edited. Nothing was staged.
**Author / reviewer**: the sprint implementation agent wrote the ray cast, the multi-brick remediation, and the comment edit this pass reviews. The reviewer is an independent microscope subagent in a separate context and did not write that code or those comments. This is not a human review.
**Result**: 0 defects, 0 smells, 0 nitpicks

Pass 2's comment defect is closed. The rewritten sentences match admission and bind. No new defect was found.

## Acceptance challenged

The claim under test is the comment text, not a new pixel. Pass 2 already measured the integral, `LINEAR_EXACT`, `from_far`, tier C, and the two-brick colour. This pass did not reopen those, because the shader bytes are unchanged and the comment edit does not touch that arithmetic.

The discriminating case is admission room, derived from the function rather than from the comment. `volume_ray_admission` sets room to `sampled_limit` minus one, saturating at zero. A limit of 16 leaves room for 15 bricks. A ready count of 2 is not above that room, so the result is `Ok`. A ready count of 16 is above that room, so the result is `SampledTextureLimit`. `volume_raycast_has_no_cpu_image` asserts both calls and passed. That rejects a comment that still said the ray binds one brick.

`bind_volume_ray` takes `VolumeRaySource` and sets the ready count from `bricks.len()`. The body never calls `Volume::ready_bricks` and never receives a `Volume`. A shorter caller slice is therefore the whole set this call considers. When that length fits the sampled-texture room and `self.brick_slots`, the later success path pushes one view per element and stores that length as `brick_count`. A length above `brick_slots` returns `SampledTextureLimit` at the check before `create_texture` and before the view loop, so no prefix is installed. That rejects both "a shorter slice is refused" and "the ray draws a subset".

A count that fits the numeric room on tier C is still not bound. Admission returns `TierCUnavailable` before the room check. The new sentences do not deny that. The admission docs and the error list on `bind_volume_ray` still name tier C.

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|--------------------------|---------------------------|------------------------------|----------------------|
| `git rev-parse HEAD` and `git status --short` | `sprint/s19` working tree | Executed now, before the test | HEAD `42336565115749daa4f20e8e04a18f1cb72729f2`. Product paths are the five modified files and three untracked files listed above. |
| `shasum -a 256 crates/ocelli-render/shaders/raycast.wgsl` | untracked shader, not edited this pass | Executed now | `50b3ec4adc1acd0c5d52120d843afbb2c03df413510e77a5341ab46e7d1b1694`. Same digest pass 2 recorded after restoring the flag and slot mutations. |
| Flag test text | `raycast.wgsl` line 89 | Executed now, read only | One occurrence of `if (flag == 0u)`. It still `continue`s before `load_brick`. |
| `bin/ocelli.sh test ocelli-render --test raycast` | host rows only | Executed now | Exit 0. `volume_raycast_has_no_cpu_image` and `a_volume_mip_is_not_the_composite_ray` passed. 4 GPU tests ignored. No GPU command was run. |
| Read of `volume_ray_admission` and `bind_volume_ray` | `graph.rs` admission at 715-727, slot refusal at 2194-2198, residency texture at 2284, view loop at 2323-2333 | Executed now | Count 2 at limit 16 is `Ok`. Count 16 at limit 16 is `SampledTextureLimit`. Over-slot returns before any texture this call creates. No comparison with `Volume::ready_bricks`. |
| Comment search on the ray path | `graph.rs`, `raycast.wgsl`, `tests/raycast.rs` | Executed now | The only "shorter slice" sentence says the slice is bound as given. "One brick" remains on `clipped_brick` and `upload_brick`, which describe one brick's extent or one upload. |

Pass 2's GPU receipt stays a prior receipt. It is not evidence executed on this tree. The shader digest matches that receipt's restored file, and this pass did not edit the shader.

## Defects

None.

## Smells

None.

## Nitpicks

None.

## Verified clean

`GraphError::SampledTextureLimit` at `graph.rs:294-301` now says one sampled binding is the residency texture, room is that limit minus one, and a limit of zero, a ready count of zero, or a ready count above that room is refused. The same paragraph says a count above the compiled brick slots is that error, creates no texture, and does not draw a subset of the bricks the caller passed. Those sentences match the two return sites. Admission refuses the room cases and creates nothing. `bind_volume_ray` repeats `SampledTextureLimit` when `ready_bricks > self.brick_slots`, and that return sits before the residency `create_texture` and before the loop that pushes brick views. `slot.ray` is assigned only after the full slice has been viewed. A refused call leaves any previous binding in place. It does not replace it with a prefix. Encode draws that previous binding or, when `slot.ray` is absent, skips the volume viewport. It does not invent a subset of the refused slice.

`VolumeRaySource` at `graph.rs:803-818` says the function does not compare `bricks` with `Volume::ready_bricks`, and that a shorter slice is bound as given when it fits the sampled-texture room and the compiled slot count. The field doc says the bricks may arrive in any order and that the length is the ready count. `bind_volume_ray` uses `bricks.len()` as that count. `slot_for` in the shader searches `i < brick_count` by coordinate, so slice order is not an axis order. The summary line on `bind_volume_ray` says it binds the bricks in `source` and a residency row. The residency texture is `R32Uint`, written with `write_texture`, and admission runs before that texture is created.

`volume_ray_admission` still says a ready count that fits is not reduced. `brick_slot_count` is still `min(sampled - 1, bind_group - 1)`, and its comment says a smaller ready set is not substituted. On wgpu 30.0.1 the WebGPU defaults and the WebGL2 downlevel defaults both use 16 sampled textures and 1000 bindings per group, so the compiled slot count equals the sampled room after residency. The shader header's slot-count sentence names that room. It does not say one brick is the only binding. It says each ready brick is its own `texture_3d`.

`clipped_brick` and `upload_brick` still talk about one brick's extent or one brick copy. That is not a claim that the ray binds a single texture. `docs/lld/` was not treated as a defect. The render-graph LLD still describes brick upload, not this ray bind.

No mutation was performed. The shader hash was not changed by this pass. No product source was edited.

## Limitations and handoff

The host test locks admission of 2 bricks at limit 16 and refusal of 16 bricks at limit 16. It does not call `bind_volume_ray` with a slice shorter than `Volume::ready_bricks`, and it does not execute `ready_bricks > brick_slots` on a live device. Those two sentences are established by reading the function. A later edit that refused a short slice or bound a prefix would not be caught by the host row that passed here.

The four ignored GPU tests were not run. Pass 2's pixel results are not repeated as measurements of this comment edit. Arithmetic and shader bytes are unchanged from that pass. Native GL tier B was not opened. Mixed `R8Uint` and `R16Uint` rejection was not executed. This pass did not run `gate --sprint`, wasm, the oracle, or the corpus.

The story is not accepted and is not sprint-ready. This pass does not update the LLD, mark F-059 done, or commit. The comment defect from pass 2 can stand closed. Completion still belongs to a later verify and complete step, not to this review.
