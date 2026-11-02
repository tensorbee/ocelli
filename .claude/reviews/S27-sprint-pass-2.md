# S27 sprint review, pass 2

**Reviewed**: 84402dbc2d959c509961e3a8e719191b0cb6fe51...441bfea5752b9ee243f8b36355c6ccd3f27cb160, tree b6867b0d541ccfaa7f66e997ce71225c4d6dedd9, plus the uncommitted `docs/sprints/CURRENT_SPRINT.md` sentence
**Author / reviewer**: author grok, reviewer independent subagent. You did not write the remediation.
**Result**: 0 defects, 0 smells, 0 nitpicks

## Acceptance challenged

Pass 1 D1 is closed. The audit paragraph now says M7 stays open until this sprint closes. It does not say the story being done closes M7, and it does not say M7 is already complete. The status cell on line 10 is still `done`. `docs/sprints/BACKLOG.md` row F-071 is still `done`. The only uncommitted edit in that paragraph is the replacement of `sprint's story is done` with `sprint closes`. The same sentence still says this open does not publish.

The sprint is still open. HEAD is the F-071 feature commit. There is no sprint-profile commit on this branch. Tag `s26` is merge `84402db` and its message says M7 stays open. Closing this sprint is still what the same file says completes M7. That condition has not happened. The corrected sentence matches that state.

The drawing contract was replayed through `createAnnotationLayer` on a group that was already drawn, id `keep`, text `mm`. The SVG `innerHTML` and the event list were recorded first.

- id `" "` and point `x` `"1"` returned `bad-mark`
- points `Infinity` then `x` `"bad"` returned `bad-mark`
- id newline, empty points, text `"mm"` returned `bad-mark`
- id `" "` and `NaN` stayed `blank-id`

Each refusal left that `innerHTML` in place and added no event. The group stayed. The next sync of the same mark with text `null` returned ok and emitted `annotation.modified` then `annotation.rendered`, and the text element was gone. A refusal does not clear the stored list.

The number-pair guard is still locked. One comparison in `pointIsNumberPair` was changed so a non-number `x` is accepted. `annotation.test.ts` went red on id `" "` and `x` `"1"`, which returned `blank-id` instead of `bad-mark`. The file was restored and `cmp` was identical.

No new false sentence was found in the sprint diff or in the uncommitted line. The status cell, the backlog row, the five M7 estimates, the version, the budget ceiling, and the open-until-close wording were checked against the files and the tags, not against the sentence that states them.

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|--------------------------|---------------------------|------------------------------|----------------------|
| `git status --porcelain`, `git rev-parse HEAD`, `git rev-parse HEAD^{tree}`, `git log --oneline 84402db...HEAD` | branch `sprint/s27` | Executed now | Start status was `M docs/sprints/CURRENT_SPRINT.md` and untracked pass 1. HEAD `441bfea5752b9ee243f8b36355c6ccd3f27cb160`. Tree `b6867b0d541ccfaa7f66e997ce71225c4d6dedd9`. Commits `1eba7f6` open, `85dac39` design, `441bfea` F-071 |
| `git diff` of `CURRENT_SPRINT.md` and `git diff --name-status 84402db...HEAD` | Working tree against HEAD, then the committed range | Executed now | The uncommitted hunk replaces `sprint's story is done` with `sprint closes` and changes no other line. Committed names are the design, the two F-071 reviews, the changelog bullet, the public-api drawing section and its README row, the sprint ledgers, and `annotation.ts`, `annotation.test.ts`, `index.ts`, `index.test.ts`. No Rust, no `events.ts`, no `tools.ts`, no `pointer.ts`, no `cursor.ts`, no `commands.ts`, no version file, no `ci/wasm-size-budget.json`, no `DEVIATIONS.md` |
| `git rev-parse s26^{commit}`, `git log -1 s26`, `git rev-parse s24^{commit}`, `git log -1 s24` | Tags | Executed now | `s26` is `84402dbc2d959c509961e3a8e719191b0cb6fe51`, message `Merge sprint/s26, M7 stays open`. `s24` is `71ac28d516cc2c7a9ed9db337f1528e47cc397e6`, message `Merge sprint/s24, M6 is complete` |
| Backlog, plan, allocation, tracker, versions, budget | `BACKLOG.md` F-068 to F-072 and F-X003 and F-X011, `SPRINT_PLAN.md` M7, `allocation.json` F-071, tracker row F-071, `Cargo.toml`, root and package manifests, `packages/core/src/index.ts` `VERSION`, `ci/wasm-size-budget.json` | Executed now | F-071 `done`. F-068, F-069, F-070 and F-072 `done`. Estimates 3, 3, 4, 3 and 3. F-X003 `in-progress`. F-X011 `pending`. Allocation sprint `S27`. Plan table names F-071. Tracker days cell starts `not measured`. Versions `0.1.0`. Budget bytes `2151196`, tolerance `0.05`. `int(2151196 * 1.05)` is `2258755`, which is `int(baseline * (1 + TOLERANCE))` in `scripts/pin_and_size_check.py` |
| `git show 85dac39` and `git diff 85dac39 441bfea` for the design sentence | Design commit versus feature commit | Executed now | The non-array and null-owner sentence is absent from `85dac39` and present in `441bfea`. The design commit was not amended |
| `git log -1 --format=%B 441bfea` and the S26 sprint trailer | Feature trailer and `84402db` | Prior receipt, not re-run | Feature profile, `corpus=pass`, tree prefix `b6867b0d541c`, no comparison field. The trailer gate set is the declared gates minus `oracle`, `gpu`, `guards-deep` and `quirk-mutations`. S26 sprint trailer includes `wasm` and records `corpus=pass` |
| Deleted probe `packages/core/src/annotation.probe.test.ts` with `annotation.test.ts`, `index.test.ts`, `events.test.ts` | Copy saved at `/tmp/s27-pass2-annotation.ts` before the edit. sha256 `89f98b74452b63785e68a095b4cca3584cb804c1816a10a267b2ec86ab202be2` | Executed now. Probe removed before the mutation | Exit 0. 37 passed. 1 probe, 24 annotation, 4 index, 8 events |
| `npx vitest run packages/core/src/annotation.test.ts` after `typeof value["x"] === "number"` became `typeof value["x"] !== "boolean"` | One comparison in `pointIsNumberPair` | Executed now | Exit 1. 1 failed, 23 passed. `structure finishes before blank and before non-finite` received `blank-id` for id `" "` and `x` `"1"`. Expected `bad-mark`. Later expects in that test were not reached |
| `cp` back, `cmp`, `shasum -a 256`, probe absent | Bytes from `/tmp/s27-pass2-annotation.ts` | Executed now | `cmp` identical. sha256 matches the pre-mutation hash. The restored comparison is `typeof value["x"] === "number"` |
| `npx tsc --build` | Restored tree, no probe | Executed now | Exit 0 |
| `npx eslint` on `annotation.ts`, `annotation.test.ts`, `index.ts`, `index.test.ts` | Restored tree | Executed now | Exit 0 |
| Prose scan with the `prose_check.py` rules | Working `CURRENT_SPRINT.md`, the F-071 plan, both F-071 reviews, `public-api.md`, its README row file, `AS_BUILT.md`, `BACKLOG.md`, `SPRINT_TRACKER.md`, `CHANGELOG.md`, and the three commit messages | Executed now. Not `bin/ocelli.sh gate` | No em dash, en dash, or prose semicolon outside fences, inline code, and table rows |

Node v24.13.0. Vitest 4.1.11. TypeScript 5.9.3. ESLint 9.39.5. No `bin/ocelli.sh` gate, cargo, or wasm-pack. No `v*` tag exists. `vendor/openjph-core-0.1.0` has no `LICENSE` file. `max_review_passes` is assigned in `scripts/sprint_workflow.py` and no other file reads it.

## Defects

None. Pass 1 D1 is closed.

## Smells

None.

## Nitpicks

None.

## Verified clean

The working audit lines are `S26 closed at tag s26. The merge is 84402db. M7 stays open until this sprint closes, and this open does not publish.` The following sentences name the S27 allocation row, the S27 plan table, and F-071, then say F-069, F-072, F-068 and F-070 are done, F-X003 stays in progress, and F-X011 remains pending. Those statuses match the backlog cells. The paragraph does not say M7 is complete. Line 37 still says closing this sprint completes M7. The carried bullet says M7 closes with this sprint and this open does not publish. Both are the close, not a claim that the close has already happened.

`git diff --name-status` for the range is the design, the two F-071 reviews, the changelog bullet, the public-api drawing section and its README row, the sprint ledgers, and `packages/core/src/annotation.ts`, `annotation.test.ts`, `index.ts` and `index.test.ts`. No crate under `crates/` changed. `events.ts` is not in the diff. `annotation.ts` has no import. It does not contain `addEventListener`. `events.test.ts` still expects listeners `["pointer.ts"]` only and one `OCELLI_EVENTS` declaration. That file passed.

`createAnnotationLayer` closes over one element and one listener. `sync` refuses a non-array and a null `ownerDocument` with `bad-mark` before it writes. `refusalFor` checks the record, the points array, the id string, the text type, the four booleans, every point as an object whose `x` and `y` are `typeof "number"`, and non-null text with zero points, all as `bad-mark`. A blank id is `trim().length === 0` and is next. A non-finite coordinate is next. A duplicate of an earlier id in that call is last. The first failing mark in list order returns. Success copies the marks, diffs that copy, replaces the children, stores the copy, then calls the listener.

Removals use the previous order. A new id emits `annotation.added`, and `annotation.completed` only when that new mark is completed. Initial visible, locked and selected flags do not emit change events. An id that remains emits `annotation.modified` for points or text, then `annotation.completed` only on the rising edge, then visibility, lock and selection, and only for a flag that differs. `annotation.rendered` is last. Its ids are the accepted list in order. The eight names match `OCELLI_EVENTS` and the catalogue rows whose delivery is `shell` and whose producer is `shell annotation layer`. Those rows were not edited.

Each mark is one `g` in the SVG namespace. `data-ocelli-id` is the id. `data-selected` and `data-locked` are `true` or `false`. A hidden mark stays, with `display="none"`. A visible mark does not set `display`. A non-empty point list is one `polyline`. The points attribute uses `String` on each coordinate and one space between pairs. A non-null text is one `text` element at the last point. Null text adds no text element. The source does not use `innerHTML`. Comparison in `geometryChanged` is `!==` on the coordinates and on the text, which is `===`, with no epsilon. Stored ids are not trimmed.

A listener throw is not caught. The permanent test throws on the first event, leaves the group in the SVG, and the next sync emits `annotation.removed` for that id. The stored list was not rolled back.

`index.ts` re-exports `ANNOTATION_REFUSALS`, `createAnnotationLayer`, and the six types. `annotation.test.ts` imports those six types from `index.js`. `index.test.ts` lists the two values and names the six types in the type-only comment. `VERSION` is `0.1.0` and that test passed. `Cargo.toml`, `packages/core/package.json` and `packages/react/package.json` are `0.1.0` and are not in the diff.

The backlog row is `done` and depends on F-069, which is `done`. F-X003 is `in-progress`. F-X011 is `pending`. The tracker days cell starts with `not measured`. The current-sprint status cell is `done`. The S27 plan table and the `allocation.json` row both name F-071. The allocation note is `TS — DOM-native, stays out of wasm` and the layer cell is `Rust+TS`. No Rust file was added. The design's fenced HLD lines match section 3, section 10 and section 34, including the em dash and the semicolon inside those fences. The same em dash inside the inline allocation note is not scored. Changed prose outside fences, inline code and table rows has no em dash and no prose semicolon. The uncommitted sentence has neither.

`annotation.test.ts` has 24 `it` blocks. The AS_BUILT sha256 matches this file. Pass 1 of the feature review is 1 defect and 0 smells. Pass 2 is 0 and 0. The feature trailer is profile `feature`, corpus `pass`, and its tree prefix is this HEAD tree. It has no comparison field. That missing comparison is the later sprint verification, not a defect of this commit. The recorded wasm budget is 2,151,196 bytes with tolerance 0.05, and the integer ceiling is 2,258,755. The budget file is not in the diff. D-22 is still the publication block. This open does not publish. The workspace version stays `0.1.0`.

## Limitations and handoff

This pass did not run `bin/ocelli.sh gate`, cargo, wasm-pack, clippy, or `prose_check.py` as a gate. The feature trailer and the S26 sprint trailer were read and not re-executed. No browser layout pass was run. jsdom supplied the SVG nodes. World projection, hit testing, the tool classes and the F-094 content tree were not exercised. They are out of scope and absent from this module.

A reentrant `sync` from the listener is not specified and was not scored. The plan says a listener throw is not caught. It does not define reentry.

The mutated test failed on its first expect, so the Infinity-then-non-number expect and the newline expect in that same test were not the observed failure. Those two inputs were observed in the probe, before the mutation. A stored `-0` against a later `0` was not sent through `sync` in this pass.

Carried LINEAR, SIGMOID, ICC and JPEG-LS sentences in `CURRENT_SPRINT.md` were not re-derived. The design sentence that the operator declined a design-question round on 2026-10-31 has no separate record in the tree. It is not contradicted here and it is not scored.

The restored product matches the copy taken before the mutation. `cmp` against `/tmp/s27-pass2-annotation.ts` was identical. The probe file is gone. Nothing was staged, committed, pushed, or recorded with `record-sprint-review`. This file is not a sprint-profile record. D1 is closed. No further remediation is indicated by this pass. A later edit to the reviewed tree makes this pass stale.
