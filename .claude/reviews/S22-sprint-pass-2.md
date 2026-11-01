# S22 review, pass 2

**Reviewed**: `sprint/s22` at `36da0e78f296fc6ba707767643e1603979d55729`. `git rev-parse HEAD^{tree}` is `49bead9f49b6152298b12b380bbbcdc72f8d4a9f`. The product commit is unchanged. The prose fix is unstaged in `docs/sprints/CURRENT_SPRINT.md` and is not in that tree. Start `git status --porcelain=v1` was ` M docs/sprints/CURRENT_SPRINT.md` and `?? .claude/reviews/S22-sprint-pass-1.md`. End `git rev-parse HEAD` is the same commit. End `git status --porcelain=v1` lists ` M docs/sprints/CURRENT_SPRINT.md`, `?? .claude/reviews/S22-sprint-pass-1.md` and `?? .claude/reviews/S22-sprint-pass-2.md`.
**Author / reviewer**: independent repair pass. This reviewer did not write the prose fix and is not pass 1 reviewer `01a1982c-afb0-7670-b3ae-633df43bcc6c`. This is not a human review.
**Result**: 0 defects, 0 smells, 0 nitpicks

Pass 1 D1 is closed.

## Acceptance challenged

The two rewritten sentences in the working copy of `docs/sprints/CURRENT_SPRINT.md`. The first is lines 31 to 36. The second is line 146. The expected result is that `ocelli-seg` presents the labelmap, segment number 0 is transparent, fill writes the segment colour, the outline is the one-pixel 4-connected boundary, contour and surface stay later stories, F-063 is done, and F-X003 stays in progress. The plausible wrong result is a leftover scaffold claim, a leftover pending claim for F-063, or F-X003 marked done.

`present_labelmap` is exported from `crates/ocelli-seg/src/lib.rs:14`. Segment number 0 is written as RGBA 0 at `crates/ocelli-seg/src/labelmap.rs:126`. Fill returns the LUT colour at line 160. The neighbour steps at line 177 are only `(1, 0)`, `(-1, 0)`, `(0, 1)` and `(0, -1)`. The backlog row for F-063 is `done` at `docs/sprints/BACKLOG.md:222`. F-064 and F-065 stay `pending` on S23. F-X003 stays `in-progress` at `docs/sprints/BACKLOG.md:190`. The working-tree diff does not change that row.

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|--------------------------|---------------------------|------------------------------|----------------------|
| `git rev-parse HEAD`, `HEAD^{tree}`, `git status --porcelain=v1`, `git diff -- docs/sprints/CURRENT_SPRINT.md` | `sprint/s22` before this file | Executed now | Commit `36da0e78f296fc6ba707767643e1603979d55729`. Tree `49bead9f49b6152298b12b380bbbcdc72f8d4a9f`. The only tracked change is the two prose hunks. |
| Search of `docs/sprints/CURRENT_SPRINT.md` for `scaffold` and for `F-063 is pending` | working copy, not `HEAD` | Executed now | No matches. Line 146 says `F-063 is done`. |
| Read of `crates/ocelli-seg/src/lib.rs` and `crates/ocelli-seg/src/labelmap.rs` | committed presenter on this HEAD | Executed now. Tests were not re-run | The crate exports `present_labelmap`. Segment 0 is RGBA 0. Fill writes the LUT colour. Outline uses four orthogonal steps and alpha 255. |
| Read of `docs/sprints/BACKLOG.md` rows F-063, F-064, F-065 and F-X003 | committed backlog on this HEAD | Executed now | F-063 is `done`. Contour and surface are S23 and `pending`. F-X003 is `in-progress`. |
| Read of HLD section 4 and section 22 | `docs/hld/03-architecture-and-crates.md` and `docs/hld/19-render-graph.md` | Executed now | Section 4 assigns `ocelli-seg` segmentation state and the three representations. Section 22 names `SegOverlay`. |

## Defects

None. Pass 1 D1 is closed.

## Smells

None.

## Nitpicks

None.

## Verified clean

The working-tree diff replaces the scaffold sentence and the pending sentence. It adds no other hunk. `ocelli-seg` presents the labelmap through `present_labelmap`. A pixel whose segment number is 0 is written as `(0, 0, 0, 0)` and does not call `LabelColourLut::colour`. Fill of a labelled pixel that is not an outline returns that LUT colour, including its alpha. The outline test is the four orthogonal neighbours, one pixel, and an outside neighbour counts as different. Those are the same rules the committed tests name. This pass read them and did not execute them. Pass 1 already ran `bin/ocelli.sh test ocelli-seg --test labelmap` on this product commit with exit 0.

HLD section 4, in `docs/hld/03-architecture-and-crates.md`, gives `ocelli-seg` the segmentation state and the three representations. HLD section 22, in `docs/hld/19-render-graph.md`, names the overlay pass `SegOverlay`. Contour is F-064 and surface is F-065. Both remain S23 and `pending`. The retained sentence that this sprint does not reopen the image LUT chain, volume construction, fusion, or the candidate gate was not edited. The product commit is the same commit pass 1 reviewed.

The status table at line 10 still says F-063 is done. The S22 plan table at `docs/sprints/SPRINT_PLAN.md:382` names F-063. The allocation object at `docs/sprints/allocation.json:1329` is F-063 on sprint S22. Line 146 now says F-063 is done. F-X003 is still `in-progress` in the backlog. The diff leaves the words `F-X003 stays in progress` in place. It does not mark F-X003 done. No new sentence in the two hunks is false. The file has no remaining `scaffold` and no remaining `F-063 is pending`.

## Limitations and handoff

The sprint gate was not run. wasm-pack was not run. The labelmap tests were not re-run. Product code was not mutated. The prose fix is unstaged, so `HEAD^{tree}` does not contain it. This pass did not stage the file and did not record a sprint-review receipt. The feature commit is still not on `origin/sprint/s22`. This is a bounded review of the D1 prose fix. It is not a new whole-sprint gate record.
