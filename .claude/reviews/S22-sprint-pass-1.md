# S22 review, pass 1

**Reviewed**: `sprint/s22` at `36da0e78f296fc6ba707767643e1603979d55729`. `git rev-parse HEAD^{tree}` is `49bead9f49b6152298b12b380bbbcdc72f8d4a9f`. The range is `971376e5cabda25ae03018e0f01b855fba09fb06...HEAD`. The commits are `d5d5cbb44b68c602f213f366d71b3578006c967c`, `ae6a4f6226b332f5aa76b3507ad548a259a1edd0`, `74814ba84870ce251dcc33d4eb40a9b286d577a8` and `36da0e78f296fc6ba707767643e1603979d55729`. Start `git status --porcelain=v1` was empty. End `git rev-parse HEAD` is the same commit. End `git status --porcelain=v1` lists only `?? .claude/reviews/S22-sprint-pass-1.md`.
**Author / reviewer**: independent whole-sprint pass. This reviewer did not write F-063 and is not pass 1 or pass 2 of that story. The commit author field is Atul Sharma. The trailer says `Ocelli-Generated-By: grok`. A trailer does not identify a human author. This is not a human review.
**Result**: 1 defect, 0 smells, 0 nitpicks

## Acceptance challenged

F-063. DICOM PS3.3 C.8.20.2 says a segmentation pixel is a segment number, pixel value 0 is background and is not a segment number, and segment numbers start at 1. The approved plan adds the colour and the outline. Index 0 is not a LUT entry. Outline is one pixel and 4-connected. Alpha is 255. A diagonal neighbour does not make the centre an outline.

`a_diagonal_neighbour_does_not_make_an_outline` is the plus in `crates/ocelli-seg/tests/labelmap.rs`. Indices are `[0, 1, 0, 1, 1, 1, 0, 1, 0]`. The centre, index 4, has orthogonal neighbours that are all segment 1, so the expected texel is fill `(10, 20, 30, 128)`. The four corners are segment 0 and stay transparent. Each arm is outline `(10, 20, 30, 255)`. That test passed on this commit. The committed walk is still `(1, 0)`, `(-1, 0)`, `(0, 1)` and `(0, -1)` at `crates/ocelli-seg/src/labelmap.rs:177`. `OUTLINE_ALPHA` is still 255 at line 85 and the outline branch still writes it at line 156. Pass 2 already replaced those four steps with eight neighbours and the same test went red. The committed text matches that restored file, so this pass did not repeat the mutation.

`seg_overlay_compiles_composite_only` passed. `pipeline_compiled` is true for `SegOverlay` plus `Composite`. It is false for `SegOverlay` plus `LabelmapEdgeProjection`, for slab labelmap edge, and for `SegOverlay` plus `Mip`. Stack and volume composite stay compiled. The predicate is not "true for that one key and no other pass".

`show_labelmap` returns `LabelmapDrawError::TierCUnavailable` at `graph.rs:3261` before `grid.width` is read at line 3283. `set_voi` matches only stack, volume ray and slab at `graph.rs:2155` and returns `PassHasNoUniform` for every other registered pass, including `SegOverlay`, before a buffer write. `encode_frame` calls `ctx.queue().submit` once, at `graph.rs:3594`. `show_labelmap` does not submit. The copy shader is `textureLoad` of the uploaded texel. It has no neighbour test and no VOI uniform.

A refusal in `present_labelmap` returns before `out.copy_from_slice` at line 146. The missing-colour row and the sentinel row passed. Contour, surface, the representation registry and labelmap edge projection are not in the product diff. `BACKLOG.md` still marks F-X003 `in-progress`. The feature commit is not on `origin/sprint/s22`. Its trailer has `corpus=pass` and no comparison fields.

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|--------------------------|---------------------------|------------------------------|----------------------|
| `git rev-parse HEAD`, `HEAD^{tree}`, `git status --porcelain=v1` | `sprint/s22` before this file | Executed now | Commit `36da0e78f296fc6ba707767643e1603979d55729`. Tree `49bead9f49b6152298b12b380bbbcdc72f8d4a9f`. Status empty. |
| `git status -sb` and `git rev-parse @{u}` | feature commit against origin | Executed now | `sprint/s22...origin/sprint/s22 [ahead 2]`. Upstream `ae6a4f6226b332f5aa76b3507ad548a259a1edd0`. The feature commit and the design approval are not on that upstream. |
| `bin/ocelli.sh test ocelli-seg --test labelmap` then the command status | committed presenter, including the plus fixture | Executed now | Exit 0. 7 passed, 0 failed. |
| `bin/ocelli.sh test ocelli-render --test labelmap` then the command status | committed `pipeline_compiled` host row. Ignored rows not selected | Executed now | Exit 0. 1 passed, 0 failed, 2 ignored. |
| Added-line scan for `unsafe`, `wasm_bindgen` and ` as ` | `git diff -U0 971376e...HEAD -- crates` | Executed now | No `unsafe` and no `wasm_bindgen`. The only ` as ` hits are the English word in comments. |
| Feature trailer against `git rev-parse HEAD^{tree}` and `bin/ocelli.sh` gate names | commit `36da0e7` | Executed now. The gates were not re-run | `tree=49bead9f49b6` matches the tree prefix. The gate list is the floor set plus `corpus`. It has no `gpu`, no `oracle` and no comparison fields. `ci/wasm-size-budget.json` is `bytes` 2151196 and `tolerance` 0.05, which is ceiling 2258755. |
| `crates/ocelli-seg/Cargo.toml` and the `Cargo.lock` hunk | dependency direction | Executed now | `ocelli-seg` has no dependency entries. The lock hunk adds `ocelli-seg` under `ocelli-render` only. |

## Defects

### D1, CURRENT_SPRINT still says the crate is the empty scaffold and that F-063 is pending

**Where**: `docs/sprints/CURRENT_SPRINT.md:31` and `docs/sprints/CURRENT_SPRINT.md:145`
**What**: Line 31 says `ocelli-seg` is still the F-001 scaffold. Line 145 says F-063 is pending. The status table on line 10 says done. `docs/sprints/BACKLOG.md` says done. `crates/ocelli-seg/src/lib.rs` exports `present_labelmap`. The feature commit changed only the table cell from pending to done.
**Why it is wrong**: A false sentence in the sprint record is a defect. The file itself says the backlog is the authority for that table. The closed S21 text rewrote the matching status sentence once those stories were done. These two sentences were left in the open tense.
**Evidence**: Read on this HEAD. No command was required. The backlog diff changes only the F-063 status cell to done. F-X003 stays `in-progress`.
**Scope and regression**: Present on HEAD. The pixels are not wrong. A reader who trusts the narrative over the table treats the story as unstarted. The smallest change is those two sentences. Uncertainty: a reader who treats "What this sprint is" and "Audit intake" as a frozen open log would not call them current. This pass does, because the S21 close rewrote that status sentence and this file's own table was already updated.

## Smells

None.

## Nitpicks

None.

## Verified clean

Index 0 stays transparent. `present_labelmap` writes RGBA 0 for segment 0 and does not call `LabelColourLut::colour`. `colour` subtracts 1 before the table lookup, so the table has no entry 0. `background_is_not_a_segment` passed. That cites PS3.3 C.8.20.2. The HLD citations used by the story are real headings. Section 22 is the render graph in `docs/hld/19-render-graph.md`. Section 4 is the crate table in `docs/hld/03-architecture-and-crates.md`, where `ocelli-seg` is the segmentation crate. Section 31 of `docs/hld/26-differentiating-capabilities.md` says a segmentation mask should not round-trip through JavaScript and that a kernel with no fallback reports unavailable.

Outline is one pixel. The four orthogonal steps are the only neighbourhood. An outside neighbour counts as background. Alpha 255 replaces the fill on that pixel only. The plus centre stayed at alpha 128 on this run. The arms were 255. The corners stayed transparent. A refusal allocates `pending` and copies into the caller buffer only after the loop. Both the short-buffer row and the missing-colour row left the sentinels. The render host row did not run the ignored device tests.

`labelmap.wgsl` loads the presenter texel and returns it. The vertex triangle matches `slab.wgsl`. The pipeline blend is `None`. The shader does not recompute the outline and does not read a VOI uniform. `set_voi` on `SegOverlay` still returns `PassHasNoUniform`. `pipeline_compiled` is true for composite segmentation and false for labelmap edge. `show_labelmap` refuses tier CPU before it reads the grid. A non-composite blend returns `BlendUnavailable` before `present_labelmap`. `encode_frame` still has one `queue.submit`. `stage` uses `write_texture`.

`ocelli-seg` does not depend on wgpu. `ocelli-render` depends on `ocelli-seg`. The new Rust has no `unsafe` and no `wasm-bindgen`. `voi.wgsl` is not in the diff. Contour, surface, the representation registry and labelmap edge projection are not implemented. F-064, F-065, F-066 and F-067 stay pending. F-X003 stays `in-progress`. This pass did not mark it done.

The feature commit message describes the presenter, the tests and the HLD sections. It does not say the commit was pushed. It does not carry comparison evidence. `AS_BUILT` says not to push it and that the sprint push needs a later comparison record. The changelog and the LLD rows match the presenter, the copy shader, the uncompiled edge and the single submit. The feature trailer tree prefix matches `HEAD^{tree}`. The budget baseline and ceiling named in `AS_BUILT` match `ci/wasm-size-budget.json`. The stated wasm measurement of 2170992 bytes was not re-executed here.

## Limitations and handoff

The sprint gate was not run. wasm-pack was not run. Clippy was not rerun. The two ignored GPU tests were not run. A missing adapter returns success before their readback asserts, so a green ignored row is not by itself a readback. Pass 1 recorded that those asserts passed on an Auto adapter in 0.21s. The feature trailer does not list the `gpu` gate. This pass did not reopen an adapter and did not repeat the eight-neighbour mutation or the outline-alpha mutation.

Tier B was not opened. `show_labelmap` after `install_recovered` and `rebind` was not run. `rebind` builds a slot whose `labelmap` is `None`, so the next show is not an equal-bytes hit. That was read and not executed. A target larger than the label was not drawn. The GPU fixture is the symmetric 3 by 3, so it does not discriminate a row flip. The CPU plus does not cross the shader. A second non-zero segment beside segment 1 was not executed. The orthogonal mismatch branch is the same comparison the plus already hits with 0.

The S21 close sentence in the audit intake, the gate tree `adb6adafd1f7`, and the "31 of 31" claim were not re-checked. No patient data was added. This is a bounded review of the committed sprint. It is not a sprint-profile record and it does not push.
