# F-071 review, pass 2

**Reviewed**: working tree on sprint/s27 at 85dac3985d20 plus uncommitted F-071 paths. HEAD is `85dac3985d2031adedbd65806e52d2711cc5873e` (`S27, approve the F-071 design`). Tracked diff: `.claude/plans/F-071-design.md`, `CHANGELOG.md`, `docs/lld/README.md`, `docs/lld/public-api.md`, `packages/core/src/index.ts`, `packages/core/src/index.test.ts`. Untracked product: `packages/core/src/annotation.ts`, `packages/core/src/annotation.test.ts`.
**Author / reviewer**: author grok. Reviewer is an independent subagent. This reviewer did not write the remediation and did not resume the pass 1 reviewer.
**Result**: 0 defects, 0 smells, 0 nitpicks

## Acceptance challenged

Pass 1 D1 is closed. The contract sentence is unchanged in `docs/lld/public-api.md` and in `.claude/plans/F-071-design.md`. One mark is structure, then a blank id, then a non-finite coordinate, then a duplicate of an earlier id. Structure includes every point being an object whose `x` and `y` are `typeof "number"`, and non-null text with zero points.

Those three pass 1 inputs were replayed through `createAnnotationLayer(...).sync`, with a previously drawn group `keep` still in the SVG:

- id `" "` and point `{ x: "1", y: 2 }` returned `bad-mark`
- points `[{ x: Infinity, y: 1 }, { x: "bad", y: 1 }]` returned `bad-mark`
- id `"\n"`, points `[]`, text `"mm"` returned `bad-mark`
- id `" "` and `{ x: NaN, y: 1 }` stayed `blank-id`

Each refusal left that group's `innerHTML` in place and added no event. A later sync that cleared the text emitted `annotation.modified` then `annotation.rendered` and removed the text element.

Adjacent calls through the same `sync` rejected the other wrong winners. A non-number before `NaN` on the same mark was `bad-mark`. A blank id with only finite numbers stayed `blank-id`. A blank id with `Infinity` stayed `blank-id`. Two finite marks with the same id stayed `duplicate-id`. A second mark that repeated the id and carried `NaN` stayed `non-finite`. A second mark that repeated the id and carried `x: "bad"` was `bad-mark`. `{ x: NaN, y: "bad" }` was `bad-mark`. An earlier non-finite mark followed by a non-string id stayed `non-finite`, so the call does not pick the earliest name in `ANNOTATION_REFUSALS` from a later mark. An earlier blank id followed by a non-string id stayed `blank-id`.

AC1 was challenged with a new mark that is completed, hidden, locked and selected. The listener received `annotation.added`, `annotation.completed`, then `annotation.rendered`. It did not receive a flag event, and `annotation.rendered` was not first. The group had `display="none"`, `data-locked="true"` and `data-selected="true"`.

AC8 was challenged with previous ids `b`, `a`, `c` and a next list of only `a`. The events were `annotation.removed` for `b`, `annotation.removed` for `c`, then `annotation.rendered` with ids `["a"]`. That is previous order, not a sort.

AC9 was challenged with a stored `-0` against a later `0`. The listener received only `annotation.rendered`. The previous `g` was disconnected and the points attribute was `0`, which is `String(-0)`.

AC10 was challenged by the refusals above. They wrote nothing and emitted nothing. A valid sync after them still drew.

AC12 was challenged by pushing a point, writing `visible`, and writing `x` on the caller's objects after `sync`. The next sync of the original values emitted only `annotation.rendered`.

AC13 remains locked by the permanent source read and by `events.test.ts`, which still expects listeners `["pointer.ts"]` only. The restored source has no import and no `addEventListener`.

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|---|---|---|---|
| `npx vitest run packages/core/src/annotation.test.ts packages/core/src/index.test.ts packages/core/src/events.test.ts` | Restored product. `annotation.ts` sha256 `89f98b74452b63785e68a095b4cca3584cb804c1816a10a267b2ec86ab202be2`. `annotation.test.ts` sha256 `aa0929c44e56b16ac91d65bfee2711fd8c2229eeaff2b7caea84a5da1e027554` | Executed now, before the mutation. Not a receipt. | Exit 0. 36 passed. 24 annotation, 4 index, 8 events. |
| `npx tsc --build` | Same tree, no probe file | Executed now | Exit 0. No diagnostics. |
| `npx eslint packages/core/src/annotation.ts packages/core/src/annotation.test.ts packages/core/src/index.ts packages/core/src/index.test.ts` | Same tree | Executed now | Exit 0. |
| Deleted probe `packages/core/src/annotation.probe.test.ts` | Published `createAnnotationLayer` from `annotation.ts`. Cases are the four refusals above plus the adjacent order cases, the insert, the reorder, the copy, and the throw. | Executed now. The file was removed before the mutation. | Exit 0. 3 passed. |
| `npx vitest run packages/core/src/annotation.test.ts` after `pointIsNumberPair` treated a non-boolean `x` as a number | One comparison in `annotation.ts`. `typeof value["x"] === "number"` became `typeof value["x"] !== "boolean"`. | Executed now | Exit 1. 1 failed, 23 passed. `structure finishes before blank and before non-finite` received `blank-id` for id `" "` and `x: "1"`. Expected `bad-mark`. The later expects in that test were not reached. |
| `cp /tmp/f071-pass2-annotation.ts` back, then `cmp` and `git diff --stat` | Bytes saved before the edit | Executed now | `cmp` identical. sha256 matches the pre-mutation hash above. `git diff --stat` is the same six-file stat. No probe file remains. |

Node v24.13.0. Vitest 4.1.11. TypeScript 5.9.3. ESLint 9.39.5. No cargo, wasm-pack, or `bin/ocelli.sh` gate.

## Defects

None. Pass 1 D1 is closed. The shape walk in `refusalFor` finishes every point, and the text-with-no-points rule, before the blank-id return. The non-finite walk is after that return. No new defect was found.

## Smells

None.

## Nitpicks

None.

## Verified clean

`createAnnotationLayer(svg, listener)` closes over one element and one listener. There is no module singleton. A second layer on a second element kept id `only` while the first layer redrew `c`.

`refusalFor` in `packages/core/src/annotation.ts` checks the mark in the plan order. The record and the points array, the id type, the text type, the four booleans, every point's number pair, and non-null text with an empty point list are `bad-mark`. A blank id is next. A non-finite coordinate is next. A duplicate of an earlier id in that call is last. `sync` returns on the first refusal and does not write. A non-array list and a null `ownerDocument` are `bad-mark` before `removeChild` or `appendChild`. The null-owner host in the permanent test does not throw from those methods.

Success copies the marks, diffs that copy, replaces the SVG children, stores the copy, then calls the listener. Removals use the previous order. A new id emits `annotation.added`, and `annotation.completed` only when that new mark is completed. Initial visible, locked and selected flags do not emit change events. An id that remains emits `annotation.modified` for points or text, then `annotation.completed` only on the rising edge, then visibility, lock and selection, and only for a flag that differs. `annotation.rendered` is last and its ids are the accepted list in order. A pure reorder emitted only `annotation.rendered`, and the groups followed the new order. An identical list rewrites the SVG.

Comparison is `!==` on the coordinates and on the text, which is `===`. There is no epsilon. Stored ids are not trimmed. `"a"` and `" a"` were both drawn. An empty string id was `blank-id`.

The SVG namespace is `http://www.w3.org/2000/svg` on the `g`, the `polyline` and the `text`. `data-ocelli-id` is the id. `data-selected` and `data-locked` are the strings `true` or `false`. A hidden mark stays, with `display="none"`. A visible mark does not set `display`. The polyline is the first child and only exists when the point list is non-empty. Three points `(-1.5, 0)`, `(2, -0)`, `(8, 9)` produced `points="-1.5,0 2,0 8,9"`. Text `"<b>end</b>"` was one text element at `8,9` with that `textContent` and no `b` child. Null text adds no text element. Empty points and null text is an empty group. The product source does not use `innerHTML`.

A throw from the listener is not caught. Throwing on `annotation.completed` left the polyline `1,2 3,4` in the SVG, did not deliver `annotation.rendered` for that call, and the next sync of the same mark emitted only `annotation.rendered`. The stored list had not been rolled back.

`annotation.ts` does not import `pointer.ts`, `pointer.js`, `cursor.ts` or `tools.ts`. It does not call `addEventListener`. It does not touch wasm, pixels or `unsafe`. Tiers A, B and C are not branched. `events.ts` is not in the diff. `packages/core/src/index.ts` re-exports `ANNOTATION_REFUSALS`, `createAnnotationLayer`, and the six types. `index.test.ts` lists the two values and names the six types in the type-only comment. `VERSION` stays `0.1.0`.

The drawing section, the changelog bullet, the README row and the added plan sentence match this tree. They have no em dash and no prose semicolon. The display list is not an SR content item. F-094 is not implemented here.

## Limitations and handoff

No browser layout pass was run. jsdom supplied the SVG nodes. World projection, hit testing, the tool classes and the F-094 content tree were not exercised. They are out of scope and absent from this module.

A reentrant `sync` from the listener is not specified and was not scored. The plan says a listener throw is not caught. It does not define reentry.

The approved plan's fenced HLD quotation still contains an em dash and a semicolon, and line 30 still has an em dash inside an inline code span. Those lines are on HEAD. The uncommitted sentence does not add either mark. This pass does not score them.

This pass does not make F-071 done. The product is still uncommitted. No further remediation is indicated for D1 or for a new finding. A later edit to this tree makes this pass stale.
