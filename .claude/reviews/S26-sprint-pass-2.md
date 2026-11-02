Result: 0 defects, 0 smells, 0 nitpicks

# S26 sprint review, pass 2

**Reviewed**: `sprint/s26` at `6cb75d26b0507a0436e76616bbc0d4c37fddaa7d` (`F-072, cursor and tool state`). Parent `fc7ed4728c7eaf9e3a195a8fd2356b8ca4c8f8fa` (`F-069, pointer, touch and wheel dispatch`). Design approval `1d1e066868ca2b3591745759cd7fff9da3e9b08a`. Sprint open `6792bcebcfb04f4e5331399968dde225667686c5`. Base `main` is `f1cd800c6c5e7a9f74db66ce40d4ba42521e00f5`. The unstaged path in scope is `docs/lld/public-api.md`. Pass 1 remains at `.claude/reviews/S26-sprint-pass-1.md` and was not edited. This pass was not recorded with `scripts/sprint_workflow.py`.
**Author / reviewer**: Both story commits carry `Ocelli-Generated-By: grok`. A generated trailer does not establish the author. This reviewer is an independent subagent. It did not write F-069, F-072, or the public-api remediation, and it did not resume an earlier reviewer.
**Result**: 0 defects, 0 smells, 0 nitpicks

## Acceptance challenged

The payload contract is `.claude/plans/F-069-design.md`. The HLD batches one command buffer per frame and does not give the formula. Pan is `basis.panX + (x - originX)` and `basis.panY + (y - originY)`. It is not the client position. Zoom is `basis.zoom * exp(-(y - originY) / 256)`. Rotate is `basis.radians + (x - originX) / 256`. The last sample in the frame is the one sample. The code stores that sample as `lastX` and `lastY`.

Pass 1 D1 said the LLD packed Pan, Zoom and Rotate as the basis plus the raw delta from the origin. The working-tree paragraph now states the three plan formulas. The wrong behaviour rejected here is a sum in the zoom arm only. Basis zoom is 1 and the first move is y 256 from origin 0, so the plan factor is `Math.exp(-1)`. The sum packed 257.

The cursor contract is `.claude/plans/F-072-design.md`. The HLD does not name cursor strings or when `tool.activated` fires. Subject `tool.modeChanged` comes before `tool.activated`. Primary owns both phases. An override does not change the shared table.

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|---|---|---|---|
| `git rev-parse HEAD`, `git status --short`, `git diff --stat`, `git log --oneline f1cd800..HEAD`, `shasum -a 256` of `pointer.ts` and `cursor.ts` | `sprint/s26` before the mutation | executed now | HEAD `6cb75d26b0507a0436e76616bbc0d4c37fddaa7d`. Four commits above the base. Product hashes `0b359fd185998ef14c8c6aa963ad85ccdaf516b10852aab502226abff60c0a63` and `fb1c8ae76c1275c1cd1fccff90dfb92089b61a76108f93304de528e91f11fb1e`. Only `docs/lld/public-api.md` was modified. Pass 1 was the only untracked file. |
| `git diff --name-only f1cd800...HEAD` and a search of that diff for `unsafe` and `wasm-bindgen` | 27 files, the same set pass 1 counted | executed now | No `tools.ts`, `events.ts`, `commands.ts`, `package.json`, `Cargo.toml`, `crates/ocelli-render`, `cpu.rs`, oracle producer, version file, `ci/wasm-size-budget.json`, or SVG path. The words `unsafe` and `wasm-bindgen` appear only in design and review prose that says they are not touched. |
| `git diff` of `docs/lld/public-api.md`, plus `CHANGELOG.md`, `AS_BUILT.md` and `CURRENT_SPRINT.md` against the base | unstaged LLD plus committed sprint prose | executed now | The LLD replacement states the three formulas. Those three files do not say the packed Zoom or Rotate value is the basis plus the raw delta. |
| Zoom arm replaced by `basis.zoom + (lastY - originY)` | `packages/core/src/pointer.ts` only. Copied to `/tmp/pointer.ts.s26-pass2` first. | executed now | `npx vitest run packages/core/src/pointer.test.ts` exited 1. 2 failed, 29 passed. `zoom and rotate use the gesture origin` received `{ kind: "Zoom", factor: 257 }`. Expected factor `0.36787944117144233`, which is `Math.exp(-1)`. `a non finite zoom is not packed` received a finite factor `1.7976931348623157e+308`. |
| Restore from `/tmp/pointer.ts.s26-pass2`, then `cmp` and `shasum -a 256` | the pre-mutation copy | executed now | `cmp` matched. Both product hashes matched the values above. `git diff --stat` was only `docs/lld/public-api.md`. |
| `npx vitest run` on `cursor.test.ts`, `pointer.test.ts`, `events.test.ts`, `index.test.ts` | restored bytes | executed now | Exit 0. 77 passed. 34 cursor, 31 pointer, 8 events, 4 index. |
| Backlog rows, `SPRINT_PLAN.md` M7 line, package `"version"` fields, and the `Result:` lines of the F-069 and F-072 reviews | committed sprint docs | executed now | F-069 and F-072 are `done`. F-068 and F-070 stay `done`. F-071 is `pending`. F-X003 is `in-progress`. Root, core and react versions are `0.1.0` and those files are not in the diff. Tracker cells match the review files: F-069 is 5/1, 2/0 and 0/0. F-072 is 0/10, 0/2, 0/4, 0/2, 0/2, 0/2, 0/2 and 0/0. |

Node v24.13.0. Vitest 4.1.11. No browser and no GPU. The first mutation script stopped before its own restore because `status` is read-only in zsh. The copy in `/tmp` was written back before the green suite, and the hash check above is that restore.

## Defects

None. Pass 1 D1 is closed on this working tree. `docs/lld/public-api.md` now says Pan is `basis.panX + (x - originX)` and `basis.panY + (y - originY)`, Zoom is `basis.zoom * exp(-(y - originY) / 256)`, and Rotate is `basis.radians + (x - originX) / 256`. The same paragraph says the last sample in the frame is the one sample, and that the value is not the client position. It does not claim a raw sum for Zoom or an unscaled delta for Rotate.

`CHANGELOG.md`, `docs/sprints/AS_BUILT.md` and `docs/sprints/CURRENT_SPRINT.md` still say Pan, Zoom and Rotate come from a caller basis because those camera writes replace. They do not restate the false formula.

## Smells

None.

## Nitpicks

None.

## Verified clean

The corrected pointer paragraph was checked against `packages/core/src/pointer.ts` and against `set_pan`, `set_zoom` and `set_rotation` in `crates/ocelli-viewport/src/view.rs`. Those three methods assign the stored value. `scroll_by` adds. The dispatcher captures `basis()` at pointer down. `produce` uses `lastX` and `lastY` against the gesture origin, with scale 256. `takeFrame` calls `packCommands` once when the command list is non-empty and does not call `submitCommands`, `writeCommands` or `packPointerFrame`. `packCommands` builds a `Uint8Array` and does not call wasm.

The scroll sentences that were not D1 still match the source. One frame pushes at most one `Scroll`. `wheelStepOf` sums the samples and `addI32` clamps the combined step once. Pixel mode contributes the sign of the summed `deltaY`, line mode sums truncations, and page mode sums signs. A scroll drag contributes `sign(lastY - segmentY)`, which is that frame's y change, and `segmentY` moves to `lastY` at the end of the frame. Fit, FlipHorizontal, FlipVertical, SetWindow, ApplyPreset, SetInvert and SetColormap are the other seven, and a non-camera command is blocked as `payload-not-in-gesture`.

The rest of the pointer section matches the source. A click is a net distance strictly under 3. `engaged` stays set once a sample is at least 3 from the origin, so a later sample still packs. A second down after `pointerup` opens a new gesture before `takeFrame`. A second pointer id sets `sawExtra` and does not replace the open gesture. `pointerType` `touch` uses the touch names. `mouse` and `pen` use the pointer names. The pointer and cursor sections have no em dash and no prose semicolon.

`cursor.ts` imports `tools.js` and `generated.js`. It does not import `pointer.ts` or `pointer.js` and does not call `addEventListener`. `createToolState` reads the group, calls `registry.setMode`, and emits only when that call is ok and the subject was present. Subject `tool.modeChanged` is first when the stored mode changes. `tool.activated` follows it and only when the new mode is `active`. Demoted names are walked in `result.value.demoted` and reported as `passive`. `registerTool` and `removeTool` only forward. There is no `try` around the listener, and the registry write is already done. `createCursorController` copies `COMMAND_CURSORS` and replaces pairs on the copy. One chosen tool supplies both phases. Primary is any binding whose button is `primary`. Otherwise the first active tool in the returned list is used. `group` returns tools in registration order, and `registerTool` pushes. `sync` assigns `style.cursor` on every call. The cursor strings in the LLD match `COMMAND_CURSORS`. The cursor hash is the F-072 pass 8 hash, so this pass did not reopen that file as a product edit.

F-069 and F-072 are done. F-071 is pending and is not in the diff, so closing S26 does not complete M7. `docs/sprints/SPRINT_PLAN.md` says M7 is S25 to S27, five stories and 16 engineer-weeks. The backlog estimates for F-068, F-069, F-070, F-071 and F-072 are 3, 3, 4, 3 and 3. F-X003 stays in progress. No Rust product file is in the sprint diff.

## Limitations and handoff

No blocking finding. The product hashes above are the restored HEAD hashes. This review file is not part of that tree. Pass 1 stays untracked and was not treated as a clean pass.

No `bin/ocelli.sh` run, no cargo test, no wasm build, and no browser. Packed zoom was not applied through `View::set_zoom`. Packed scroll was not applied through `scroll_by`. The feature ledger rows were not re-read and their gates were not repeated.

The zoom mutation stopped at the first assertion, so this run did not print the second zoom frame or the rotate expects. The restored suite includes the whole test, which expects `Math.exp(-2)` and radians 1 then 2. The rotate arm was not replaced. The source line is `basis.radians + (lastX - originX) / 256`.

Deleting the `< 3` check in `doubleCandidate` was not tried. The code on HEAD still uses `< 3`, and `double click uses 400 milliseconds` passed in the restored suite. F-072 pass 8 left two active primary owners unspecified. The cursor bytes match that pass, so this pass did not file the tie. A listener that calls `setMode` again from inside the callback was not exercised. The plan does not define that re-entry.
