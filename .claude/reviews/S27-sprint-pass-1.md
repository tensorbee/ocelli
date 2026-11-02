# S27 sprint review, pass 1

**Reviewed**: 84402dbc2d959c509961e3a8e719191b0cb6fe51...441bfea5752b9ee243f8b36355c6ccd3f27cb160, tree b6867b0d541ccfaa7f66e997ce71225c4d6dedd9
**Author / reviewer**: author grok, reviewer independent subagent. You did not write the sprint.
**Result**: 1 defect, 0 smells, 0 nitpicks

## Acceptance challenged

The contract is `.claude/plans/F-071-design.md` and the drawing section in `docs/lld/public-api.md`. HLD `docs/hld/03-architecture-and-crates.md` section 3 places the SVG annotation layer in the TypeScript shell. HLD `docs/hld/07-concurrency-and-typescript.md` section 10 leaves that layer DOM-native. HLD `docs/hld/26-differentiating-capabilities.md` section 34 says the in-memory type is a DICOM SR content-item tree and the drawing layer renders from it and does not own it. Decision D13 says the annotation type is that tree. F-094 is `pending` and is not this story. The display list is not that type.

Pass 1 D1 was replayed through `createAnnotationLayer` on a group that was already drawn, id `keep`, text `mm`. The SVG `innerHTML` and the event list were recorded first.

- id `" "` and point `x` `"1"` returned `bad-mark`
- points `Infinity` then `x` `"bad"` returned `bad-mark`
- id newline, empty points, text `"mm"` returned `bad-mark`
- id `" "` and `NaN` stayed `blank-id`

Each refusal left that `innerHTML` in place and added no event. The next sync of the same mark with text `null` returned ok and emitted `annotation.modified` then `annotation.rendered`, and the text element was gone. A refusal does not clear the stored list.

A new mark that is completed, hidden, locked and selected emitted `annotation.added`, `annotation.completed`, then `annotation.rendered`. It did not emit a flag event. The group had `display="none"`, `data-locked="true"` and `data-selected="true"`. Points `(-1.5, 0)`, `(2, -0)`, `(8, 9)` produced `points="-1.5,0 2,0 8,9"`.

An earlier non-finite mark followed by a non-string id stayed `non-finite`. The call does not pick a later structural refusal. A second layer on a second element kept id `only` while the first element stayed empty.

The number-pair guard is locked. One comparison in `pointIsNumberPair` was changed so a non-number `x` is accepted. `annotation.test.ts` went red on id `" "` and `x` `"1"`, which returned `blank-id` instead of `bad-mark`. The file was restored and `cmp` was identical.

Closing M7 was challenged against the sprint record. The status cell and the backlog row are `done`. The same file says closing this sprint completes M7. There is no sprint-profile commit. That is D1.

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|--------------------------|---------------------------|------------------------------|----------------------|
| `git status --porcelain`, `git rev-parse HEAD`, `git rev-parse HEAD^{tree}`, `git log --oneline 84402db...HEAD` | branch `sprint/s27` | Executed now | Empty status at start. HEAD `441bfea5752b9ee243f8b36355c6ccd3f27cb160`. Tree `b6867b0d541ccfaa7f66e997ce71225c4d6dedd9`. Commits `1eba7f6` open, `85dac39` design, `441bfea` F-071 |
| `git diff --stat` and `--name-status` for the range | 14 files | Executed now | No Rust, no `events.ts`, no `tools.ts`, no `pointer.ts`, no `cursor.ts`, no `commands.ts`, no version file, no `ci/wasm-size-budget.json`, no `DEVIATIONS.md`, no `compare-expectations.json` |
| Deleted probe `packages/core/src/annotation.probe.test.ts` with `annotation.test.ts`, `index.test.ts`, `events.test.ts` | Restored `annotation.ts` sha256 `89f98b74452b63785e68a095b4cca3584cb804c1816a10a267b2ec86ab202be2` | Executed now. Probe removed before the mutation | Exit 0. 39 passed. 3 probe, 24 annotation, 4 index, 8 events |
| `npx vitest run packages/core/src/annotation.test.ts` after `typeof value["x"] === "number"` became `typeof value["x"] !== "boolean"` | One comparison in `pointIsNumberPair`. Copy saved at `/tmp/s27-pass1-annotation.ts` before the edit | Executed now | Exit 1. 1 failed, 23 passed. `structure finishes before blank and before non-finite` received `blank-id` for id `" "` and `x` `"1"`. Expected `bad-mark`. Later expects in that test were not reached |
| `cp` back, `cmp`, `shasum -a 256`, delete probe, `git status --porcelain` | Bytes from the copy | Executed now | `cmp` identical. sha256 matches the pre-mutation hash. Status empty before this review file |
| `npx tsc --build` | Restored tree, no probe | Executed now | Exit 0 |
| `npx eslint` on `annotation.ts`, `annotation.test.ts`, `index.ts`, `index.test.ts` | Restored tree | Executed now | Exit 0 |
| `git log -1 --format=%B 441bfea` | Feature trailer | Prior receipt, not re-run | `profile=feature`, `corpus=pass`, `tree=b6867b0d541c`, no comparison field. Gate names match `--floor` plus `corpus` |
| `git rev-parse s26^{commit}` and `git log -1 s26` | Tag `s26` | Executed now | Peels to `84402dbc2d959c509961e3a8e719191b0cb6fe51`, message `Merge sprint/s26, M7 stays open` |
| Backlog and version reads | `BACKLOG.md`, `Cargo.toml`, both package manifests, `ci/wasm-size-budget.json` | Executed now | F-071 `done`. F-X003 `in-progress`. F-X011 `pending`. F-094 `pending`. Versions `0.1.0`. Budget bytes `2151196`, tolerance `0.05`, `int(baseline * 1.05)` is `2258755` |

Node v24.13.0. Vitest 4.1.11. TypeScript 5.9.3. ESLint 9.39.5. No `bin/ocelli.sh` gate, cargo, or wasm-pack.

## Defects

### D1, M7 is still tied to story completion after the story is done

**Where**: `docs/sprints/CURRENT_SPRINT.md:130`

**What**: The audit intake says `M7 stays open until this sprint's story is done`. The status cell on line 10 says `done`. `docs/sprints/BACKLOG.md` row F-071 says `done`. `SPRINT_TRACKER.md` has the F-071 row. Line 37 says closing this sprint completes M7. The open commit `1eba7f6` wrote the audit sentence while the status cell was `pending`. The feature commit `441bfea` changed that cell to `done` and did not change the audit sentence.

**Why it is wrong**: A false sentence in the sprint record blocks completion. The story being done is not what completes M7. This tree has the story done and the sprint still open. There is no sprint-profile commit, and the feature trailer has no comparison receipt. M7 therefore stays open after the story is done. The sentence is not a historical open note that is still accurate. The same file's status cell was updated, and the S26 text this section replaced already said the finished S26 stories were done while M7 stayed open. `and this open does not publish` on the same line is still true and is not the defect.

**Evidence**: `grep '^| F-' docs/sprints/BACKLOG.md | awk -F'|' '$4 ~ / S27 / {print $2, $9}'` printed `F-071` and `done`. `git log -L 128,134:docs/sprints/CURRENT_SPRINT.md` for this range shows the audit sentence landing only in `1eba7f6`. `git show 441bfea:docs/sprints/CURRENT_SPRINT.md` still has the until-story clause beside status `done`.

**Scope and regression**: prose only. Present on HEAD. The drawing layer is not wrong. Smallest change is to say M7 stays open until this sprint closes. Do not mark M7 closed. Do not publish. The workspace version stays `0.1.0`. F-X003 stays `in-progress`. F-X011 stays `pending`. Days actual stays `not measured`.

## Smells

None.

## Nitpicks

None.

## Verified clean

`git diff --name-status` for the range is the design, the two F-071 reviews, the changelog bullet, the public-api drawing section and its README row, the sprint ledgers, and `packages/core/src/annotation.ts`, `annotation.test.ts`, `index.ts` and `index.test.ts`. No crate under `crates/` changed. `events.ts` is not in the diff. `annotation.ts` has no import. It does not contain `addEventListener`, `pointer.ts`, `pointer.js`, `cursor.ts` or `tools.ts`. `events.test.ts` still expects listeners `["pointer.ts"]` only and one `OCELLI_EVENTS` declaration. That file passed.

`createAnnotationLayer` closes over one element and one listener. `sync` refuses a non-array and a null `ownerDocument` with `bad-mark` before it writes. `refusalFor` checks the record, the points array, the id string, the text type, the four booleans, every point as an object whose `x` and `y` are `typeof "number"`, and non-null text with zero points, all as `bad-mark`. A blank id is `trim().length === 0` and is next. A non-finite coordinate is next. A duplicate of an earlier id in that call is last. The first failing mark in list order returns. Success copies the marks, diffs that copy, replaces the children, stores the copy, then calls the listener.

Removals use the previous order. A new id emits `annotation.added`, and `annotation.completed` only when that new mark is completed. Initial visible, locked and selected flags do not emit change events. An id that remains emits `annotation.modified` for points or text, then `annotation.completed` only on the rising edge, then visibility, lock and selection, and only for a flag that differs. `annotation.rendered` is last. Its ids are the accepted list in order. The eight names match `OCELLI_EVENTS` and the catalogue rows whose delivery is `shell` and whose producer is `shell annotation layer`. Those rows were not edited.

Each mark is one `g` in the SVG namespace. `data-ocelli-id` is the id. `data-selected` and `data-locked` are `true` or `false`. A hidden mark stays, with `display="none"`. A visible mark does not set `display`. A non-empty point list is one `polyline`. The points attribute uses `String` on each coordinate and one space between pairs. A non-null text is one `text` element at the last point. Null text adds no text element. The source does not use `innerHTML`. Comparison in `geometryChanged` is `!==` on the coordinates and on the text, which is `===`, with no epsilon. Stored ids are not trimmed.

A listener throw is not caught. The permanent test throws on the first event, leaves the group in the SVG, and the next sync emits `annotation.removed` for that id. The stored list was not rolled back.

`index.ts` re-exports `ANNOTATION_REFUSALS`, `createAnnotationLayer`, and the six types. `index.test.ts` lists the two values and names the six types in the type-only comment. `VERSION` is `0.1.0` and that test passed. `Cargo.toml`, `packages/core/package.json` and `packages/react/package.json` are `0.1.0` and are not in the diff.

The backlog row is `done` and depends on F-069, which is `done`. F-X003 is `in-progress`. F-X011 is `pending`. The tracker days cell starts with `not measured`. The current-sprint status cell is `done`. The S27 plan table and the `allocation.json` row both name F-071. The allocation note is `TS — DOM-native, stays out of wasm` and the layer cell is `Rust+TS`. No Rust file was added. The design's fenced HLD lines match section 3, section 10 and section 34, including the em dash and the semicolon inside those fences. The same em dash inside the inline allocation note is not scored. Changed prose outside fences, inline code and table rows has no em dash and no prose semicolon.

`annotation.test.ts` has 24 `it` blocks. The AS_BUILT sha256 matches this file. Pass 1 of the feature review is 1 defect and 0 smells. Pass 2 is 0 and 0. The feature commit adds the non-array and null-owner sentence to the plan. `85dac39` does not contain it. The feature trailer is profile `feature`, corpus `pass`, and its tree prefix is this HEAD tree. It has no comparison field. That missing comparison is the later sprint verification, not a defect of this commit. Tag `s26` points at merge `84402db`. Tag `s24` peels to `Merge sprint/s24, M6 is complete`. The recorded wasm budget is 2,151,196 bytes with tolerance 0.05, and the integer ceiling is 2,258,755. The budget file is not in the diff. `vendor/openjph-core-0.1.0` has no `LICENSE` file. D-22 is still the publication block. This open does not publish.

## Limitations and handoff

This pass did not run `bin/ocelli.sh gate`, cargo, wasm-pack, clippy, or `prose_check.py`. The feature trailer and the S26 sprint trailer were read and not re-executed. No browser layout pass was run. jsdom supplied the SVG nodes. World projection, hit testing, the tool classes and the F-094 content tree were not exercised. They are out of scope and absent from this module.

A reentrant `sync` from the listener is not specified and was not scored. The plan says a listener throw is not caught. It does not define reentry.

The mutated test failed on its first expect, so the Infinity-then-non-number expect and the newline expect in that same test were not the observed failure. Those two inputs were observed in the probe, before the mutation. A stored `-0` against a later `0` was not sent through `sync` in this pass. The insert path did render `String(-0)` as `0`.

The design sentence that the operator declined a design-question round on 2026-10-31 has no separate record in the tree. It is not contradicted here and it is not scored. Carried LINEAR and SIGMOID sentences in `CURRENT_SPRINT.md` were not re-derived.

The restored product matches HEAD. `cmp` against `/tmp/s27-pass1-annotation.ts` was identical. The probe file is gone. Nothing was staged, committed, pushed, or recorded with `record-sprint-review`. This file is not a sprint-profile record. D1 is unfixed. The next owner corrects the audit sentence without closing M7 or publishing, then a new independent pass reviews that tree. This pass is not sprint acceptance.
