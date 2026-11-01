# S24 sprint review, pass 2

**Reviewed**: HEAD `486be24e6ece9b09315e8d0da953ebdf74364b8c` on `sprint/s24`, plus the unstaged repair in `docs/sprints/CURRENT_SPRINT.md`. `git rev-parse HEAD` returned that commit. `git rev-parse HEAD^{tree}` returned `246c4be008b8c669962a4c5d91bfa367e30665ef`. The feature message is `F-067, segmentation intersection and labelmap edge blend`. `git diff HEAD` names only `docs/sprints/CURRENT_SPRINT.md`. Untracked `.claude/reviews/S24-sprint-pass-1.md` is also present. That file is the prior review, not product. This pass did not treat the feature commit as the whole tree. No other tracked path is dirty. `rustc 1.97.1 (8bab26f4f 2026-07-14)`.
**Author / reviewer**: independent reviewer. This pass did not write F-067, the design, pass 1, or the unstaged sprint prose. The feature trailer says `Ocelli-Generated-By: grok`. A trailer does not identify the author.
**Result**: 1 defect, 0 smells, 0 nitpicks

## Acceptance challenged

Pass 1 D1 required both unfinished sentences to say F-067 is done. It also required M6 to stay open, the workspace version to stay `0.1.0`, and no release. The working copy says `F-067 is done` in the sprint-subject paragraph, the carried-in bullet, and the audit intake. The subject paragraph and the carried-in bullet also say `M6 stays open until this sprint closes`. The carried-in heading still says M6 is not complete and this open does not publish. `Cargo.toml` keeps `version = "0.1.0"` and is not in the diff. The backlog row for F-067 is `done`. F-063 through F-066 are `done`. None of those sentences mark M6 closed.

The overlay compile claim was read, not mutated. `pipeline_compiled` is true for `SegOverlay` plus `LabelmapEdgeProjection`. Creation pushes that key onto the existing labelmap copy shader. Slab, volume and stack do not receive it. The module banner and the `BlendUnavailable` variant still say the edge stays uncompiled. That present-tense claim is what this pass rejects. `F-X011` remaining pending is a different story and is not this defect.

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|--------------------------|---------------------------|------------------------------|----------------------|
| `git rev-parse HEAD`, `git rev-parse HEAD^{tree}`, `git status --porcelain`, `git diff --name-only HEAD` | branch `sprint/s24` | Executed now | HEAD `486be24e6ece9b09315e8d0da953ebdf74364b8c`. Tree `246c4be008b8c669962a4c5d91bfa367e30665ef`. Dirty tracked path is only `docs/sprints/CURRENT_SPRINT.md`. Untracked pass 1 review is present |
| `git diff HEAD -- docs/sprints/CURRENT_SPRINT.md` | unstaged repair against `486be24` | Executed now | Replaces `F-067 is the remaining M6 story` and `F-067 is pending` with `F-067 is done`. Adds `M6 stays open until this sprint closes`. Drops `before any code`. No em dash and no prose semicolon in the added lines |
| `grep '^| F-' docs/sprints/BACKLOG.md` with the S24 awk, and the M6 rows F-063 to F-067 | backlog status column | Executed now | S24 prints `F-067` and `done`. All five M6 rows are `done` |
| `Cargo.toml` workspace `version` | line 7 | Executed now, read | `0.1.0`. `git diff HEAD -- Cargo.toml` is empty |
| Read `docs/sprints/CURRENT_SPRINT.md` lines 25 to 30, 68 to 73 and 141 | working copy | Executed now | F-067 is done in all three places. M6 is not described as closed |
| Read F-067 `AS_BUILT.md` lines 4016 to 4029, Unreleased `CHANGELOG.md` lines 13 and 17, and the LLD hunks in `git diff 8f2d179..HEAD` | committed prose | Executed now | Overlay edge is described as compiled. Slab, volume and stack stay uncompiled. No present-tense claim there that F-067 is unfinished |
| Read `pipeline_compiled` at `graph.rs:1262` and the compile push at `graph.rs:1677` | HEAD source | Executed now, read, not re-run | Overlay edge key is compiled on `LABELMAP_WGSL`. Slab, volume and stack are outside that push |
| Read `graph.rs:29` and `labelmap.rs:21` | HEAD source | Executed now | Both still say `Labelmap edge stays uncompiled` |
| Pass 1 contour and GPU mutations | restored tree at this same HEAD | Prior receipt, not re-run | Pass 1 recorded the midpoint cut and the caller-fill readback going red, then restored. This pass did not change code and did not repeat them |

No sprint gate and no wasm-pack run were started. Cargo tests were not started.

## Defects

### D1, live docs still say the labelmap edge stays uncompiled

**Where**: `crates/ocelli-render/src/graph.rs:29` and `crates/ocelli-render/src/labelmap.rs:21`

**What**: The module banner says `Labelmap edge stays uncompiled.` The `LabelmapDrawError::BlendUnavailable` variant says `The overlay blend is not composite. Labelmap edge stays uncompiled.` `show_labelmap` accepts `LabelmapEdgeProjection` on a segmentation overlay and forces outline without fill. Creation compiles that overlay key on the existing copy shader.

**Why it is wrong**: A false present-tense claim blocks completion. The same file documents the current contract at `BlendMode` and at `pipeline_compiled`. Those docs say the overlay key compiles and that slab, volume and stack have no edge body. `docs/lld/feature-availability.md` says the same for the F-067 row. The variant doc is the reason a caller is given for `BlendUnavailable`. An overlay edge show is not that refusal.

**Evidence**: The two sentences were read at the lines above. `pipeline_compiled` returns true for `PassKind::SegOverlay` when the blend is `LabelmapEdgeProjection` (`graph.rs:1265`). The compile list pushes that key at `graph.rs:1680` with the comment that slab, volume and stack do not receive it. Pass 1's mutations were not repeated.

**Scope and regression**: Present on HEAD. The intersection arithmetic and the edge readback were not re-executed. Pass 1's sprint-record defect is closed by the unstaged sentences and is not this finding. Smallest change is to stop saying the edge stays uncompiled in these two docs, and to describe `BlendUnavailable` as a blend this show does not accept. Do not mark M6 closed. Do not change `0.1.0`.

## Smells

None.

## Nitpicks

None.

## Verified clean

Pass 1 D1 is closed in the working copy. The subject paragraph says `F-067 is done. M6 stays open until this sprint closes.` The carried-in bullet says `F-067 is done. M6 stays open until this sprint closes.` The audit intake says `F-067 is done.` The old words `pending` and `remaining M6 story` are gone from this file. `F-X011 remains pending` is still present and is a different story. The purpose sentence still says this sprint intersects a contour and compiles the edge blend. That sentence follows the done sentence and does not call F-067 pending or remaining. The carried-in heading `M6 is not complete, and this open does not publish` stays true while the sprint is open. A finished M6 would name `0.6.0`, and the text does not say that version is current.

The F-067 `AS_BUILT` entry says the overlay compiles the blend and that slab, volume and stack stay uncompiled. Its deviation note says the first approved wording left the overlay key false, then says the plan now compiles that key. That is not a claim that the key is uncompiled now. The new Unreleased changelog bullet says the edge blend draws the outline and ignores fill. The older composite bullet now says `Labelmap edge was left for a later story.` That is past tense for the earlier story. The bullet above it records the edge blend. Touched LLD sentences in `docs/lld/render-graph.md`, `docs/lld/feature-availability.md` and `docs/lld/README.md` say the overlay compiles labelmap edge projection. They keep slab, volume and stack uncompiled, including `show_slab` at `graph.rs:3161`. That slab sentence is the slab pass, not the overlay key.

`show_labelmap` returns `TierCUnavailable` before the grid is read. On a GPU overlay it treats only composite and labelmap edge as accepted blends, then replaces the caller style for the edge blend. `show_contour` still requires composite, so this blend is `BlendUnavailable` there. The workspace version is `0.1.0`. The backlog authority matches the status table. No release was requested by the repair.

F-063 `AS_BUILT` line 3954 and the F-066 note at line 4014 still contain older `stays` wording. They are prior story records, not the F-067 entry, and they were not counted as this defect. Rewriting them would make those earlier stories claim the edge was already compiled.

## Limitations and handoff

This pass did not run contour tests, GPU tests, clippy, `prose_check.py`, `bin/ocelli.sh gate`, or wasm-pack. Pass 1's mutation receipts were read and not replayed. No code was edited. `git diff HEAD` still names only `docs/sprints/CURRENT_SPRINT.md`. The pass 1 review stays untracked. Nothing was staged, committed or pushed. This file is not a sprint-profile record.

Pass 1 D1 is closed. D1 in this pass is unfixed. The next owner corrects the two live sentences without closing M6 and without changing the version. A new independent pass then reviews that tree. This pass is not sprint acceptance.
