Result: 1 defects, 0 smells, 0 nitpicks

# F-071 review, pass 1

**Reviewed**: working tree on `sprint/s27` against HEAD `85dac3985d2031adedbd65806e52d2711cc5873e` (`S27, approve the F-071 design`). Tracked diff: `.claude/plans/F-071-design.md`, `CHANGELOG.md`, `docs/lld/README.md`, `docs/lld/public-api.md`, `packages/core/src/index.ts`, `packages/core/src/index.test.ts`. Untracked product: `packages/core/src/annotation.ts`, `packages/core/src/annotation.test.ts`. The plan diff is one sentence: a non-array list and a null owner document are `bad-mark`.
**Author / reviewer**: the implementation is uncommitted, so git does not name its author. This reviewer did not write F-071 and did not resume an earlier reviewer.
**Result**: 1 defects, 0 smells, 0 nitpicks

## Acceptance challenged

The contract is `.claude/plans/F-071-design.md`, including the added refusal sentence. HLD sections 3 and 10 place the drawing layer in the TypeScript shell. HLD section 34 says the drawing layer renders from the SR content-item tree and does not own it. F-094 owns that type. This module's display list is not presented as that type.

AC1 is locked. Moving `annotation.rendered` ahead of every other name failed `a new mark is added and drawn`. The received list was `annotation.rendered` then `annotation.added`.

AC2, AC3, AC4, AC5, AC7 and AC8 are locked by the permanent expectations for completion after `added`, no initial flag events, `modified` for a point change and a text change, the rising edge, separate lock and selection names, and removal in previous order. A deleted probe also saw a new completed, hidden, locked and selected mark emit only `annotation.added`, `annotation.completed` and `annotation.rendered`, with `display="none"` and both data flags set.

AC6 is implemented. The hidden test requires the same `g`, `display="none"`, and a polyline. This pass did not delete the group as its mutation. The mutation that ran was rendered-first, and that also failed the hidden test because the event order changed. The restored product keeps the group.

AC9 is locked for a tampered `points` attribute. The probe also saw an identical sync disconnect the previous `g` and append a new one, and saw `-0` compare equal to `0`.

AC10 is locked for a duplicate id. The probe extended it. Blank id, non-finite, bad header, non-array (`null`, an array-like object, and a string), and duplicate all left the previous `innerHTML` unchanged and emitted nothing. A valid sync after those refusals still applied.

AC11 is only partly locked. The permanent test rejects a later mark winning, blank losing to `NaN` on the same mark, and duplicate winning over `NaN` on the same mark. It does not reject a blank id winning over a coordinate that is not a number, or an earlier non-finite point winning over a later non-number point. That is D1. Header structure does precede blank. A blank id with `visible: "yes"` or `text: 4` returned `bad-mark`.

AC12 is locked for a mutated point coordinate. The probe also pushed onto the caller's point array and flipped `visible` on the caller's mark object. The next sync of the original values emitted only `annotation.rendered`.

AC13 is locked. `annotation.test.ts` reads `annotation.ts` and rejects `addEventListener`, `pointer.ts` and `pointer.js`. `events.test.ts` still expects listeners `["pointer.ts"]` only. The restored source has no import and no listener.

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|---|---|---|---|
| `npx vitest run packages/core/src/annotation.test.ts packages/core/src/index.test.ts packages/core/src/events.test.ts` | Restored tree. `annotation.ts` sha256 `dcc33dbb1dbf551015643c1cf14b91853e9c73a2c671946a6b9b8ec509bb623b` | Executed now, before the mutation and again after restore. Not a receipt. | Exit 0 both times. 35 passed. 23 annotation, 4 index, 8 events. |
| Same command after moving the `annotation.rendered` push to the start of `diff` | `annotation.ts` only. Restored from `/tmp/f071-annotation.ts` with `cp`. Not `git checkout`. | Executed now | Exit 1. 13 failed, 22 passed. `a new mark is added and drawn` received `annotation.rendered` then `annotation.added`. |
| Deleted `packages/core/src/annotation.probe.test.ts`, first run | Published `createAnnotationLayer`. Probe removed before the mutation. | Executed now | Exit 1 by assertion dump. Blank id plus `x: "1"` was `blank-id`. Blank id plus text and no points was `blank-id`. Infinity then `x: "bad"` was `non-finite`. A non-number before `NaN` was `bad-mark`. |
| Deleted probe, second run | Same module. Expectations were the refusals the code returns, plus preservation and copy cases. | Executed now | Exit 0. 4 passed. Those passes confirm the returns. They do not make the returns match the plan. |
| `git diff HEAD --stat` and `git status --short` | Restored tree, review file not yet written | Executed now | Same six-file stat as the start of the pass. Untracked product files were only the two annotation sources. |
| `cmp` of `annotation.ts` against `/tmp/f071-annotation.ts` | Restored bytes | Executed now | Identical. sha256 matches the pre-mutation hash above. `annotation.test.ts` stayed `c6b9d2bce6f0098833895637a8252527c74f5feb60f669c4557c2e8dde25960d`. |

Node v24.13.0. Vitest 4.1.11. No `bin/ocelli.sh`, cargo, wasm-pack, tsc or eslint in this pass.

## Defects

### D1, point structure loses to blank id and to an earlier non-finite point

**Where**: `packages/core/src/annotation.ts:123` through `:134`, with the point decision in `pointRefusal` at `:89`. The order claim is `docs/lld/public-api.md:181` and `.claude/plans/F-071-design.md:43`.
**What**: On one mark, a coordinate that is not a number is `bad-mark`, but that check runs only after the blank-id return, and it returns on the first point. A non-null `text` with no points is also `bad-mark`, and that return is in the same late phase. A blank id therefore hides both. An earlier non-finite point hides a later point that is not a number.
**Why it is wrong**: The plan says the checks for one mark are structure, then a blank id, then a non-finite coordinate, then a duplicate of an earlier id. The public-api section repeats that sentence. A value that is not a number is not a non-finite coordinate. The code already classifies it as `bad-mark`, which is the structure refusal. Header structure does run first. A blank id whose `visible` is not a boolean returns `bad-mark`. Point shape was left in the later walk. The LLD sentence is false of this tree for that reason. Do not edit the sentence to match the short walk. The plan is the contract.
**Evidence**: Deleted probe, second run, exit 0 because it asserted the code's returns. `sync` of id `" "` and point `{ x: "1", y: 2 }` returned `{ ok: false, refusal: "blank-id" }`. `sync` of `{ x: Infinity, y: 1 }` then `{ x: "bad", y: 1 }` returned `non-finite`. `sync` of id `"\n"`, points `[]` and text `"mm"` returned `blank-id`. Each left a previously drawn group in place and emitted nothing. The same run returned `bad-mark` for a blank id with a non-boolean `visible`, which shows the header phase is already in the right place. The permanent suite still passed, 23 annotation tests. `the first bad mark wins` uses `NaN` with a blank id, which correctly stays `blank-id`, and `a structural mistake is bad-mark` uses a normal id for text with no points.
**Scope and regression**: Callers that repair the named field first will edit the id, or the infinite coordinate, and only then learn the mark was structural. The SVG refusal rule itself holds. Present in the uncommitted implementation, not in HEAD. Smallest fix is to finish every `bad-mark` check before the blank-id return, and to finish that shape walk before any `non-finite` return. Keep finiteness after blank so the existing blank-plus-`NaN` case stays `blank-id`. Smallest regression is the three inputs above, each expecting `bad-mark`, plus the existing blank-plus-`NaN` case still expecting `blank-id`.

## Smells

None.

## Nitpicks

None.

## Verified clean

Entry point is `createAnnotationLayer(svg, listener)` and `sync`. There is no module singleton. Two layers on two elements did not share a previous list. The second element's id stayed `only` while the first reordered.

Refusal writes nothing. After a successful mark `keep` with text `mm`, these inputs preserved `innerHTML` and added no events: blank id, `NaN`, a non-string id, the three D1 inputs, a duplicate id, `null`, `{ length: 1 }`, and a string. The following sync of `keep` with `text: null` emitted `annotation.modified` then `annotation.rendered` and removed the text element.

Success order matches the plan except for D1. Removals use the previous order. A list `[b, a, c]` followed by `[c, a]` emitted one `annotation.removed` and then `annotation.rendered` with ids `["c", "a"]`. A pure reorder `[a, b]` to `[b, a]` emitted only `annotation.rendered` with ids `["b", "a"]`, and the groups were in that order. `modified` is points or text only, compared with `!==`, which is `===`. `-0` against `0` did not emit `modified`. `String(-0)` in the points attribute was `0`, so the attribute and the comparison agree. Text `""` is a text element. Null text is not. Three points `(-1.5, 0)`, `(2, -0)`, `(8, 9)` with text `end` produced `points="-1.5,0 2,0 8,9"` and a text element at `8,9`. The elements use the SVG namespace. A visible mark does not set `display`. A hidden mark stays, with `display="none"`, including on insert.

The stored list is a copy. After `sync`, pushing a point and setting `visible` on the caller's objects did not change the next diff. An identical sync replaced the group. The previous node was disconnected.

A listener throw is not caught. Throwing on `annotation.completed` left the polyline `1,2 3,4` in the SVG, did not deliver `annotation.rendered` for that call, and the next `sync([])` emitted `annotation.removed` then `annotation.rendered`. The stored list had not been rolled back.

`annotation.ts` does not import `pointer.ts`, `cursor.ts` or `tools.ts`. It does not call `addEventListener`. It does not touch wasm, pixels, or `unsafe`. Tiers A, B and C are not branched. There is no Rust path in `git diff HEAD --name-only` or in the untracked product files. `events.ts` is not in the diff. `packages/core/src/index.test.ts` still expects `VERSION` `0.1.0`, and that test passed. No `package.json` version is in the diff. F-X003 is not in the diff. `docs/sprints/BACKLOG.md` still lists it `in-progress`.

The plan addition is implemented. A non-array is `bad-mark`. The suite's null owner document is `bad-mark` and does not call `removeChild`. The new drawing section, the changelog bullet, the README row and the plan sentence have no em dash and no prose semicolon. Aside from D1, the drawing-section sentences matched the code that was run. The display list is not an SR content item. F-103 names are produced here and were not added to the catalogue file.

`git diff HEAD --stat` after restore, and before this review file existed, was the same six-file stat as the start of the pass. `git diff --stat` does not list this untracked review file.

## Limitations and handoff

Typecheck and lint were not run. No browser layout pass was run. jsdom supplied the SVG nodes. World projection, hit testing, the 26 tool classes, and the F-094 content tree were not exercised. They are out of scope and absent from this module.

A reentrant `sync` from the listener is not specified. One deleted probe called `sync` once from the first event. The inner list replaced the element, and the outer call still delivered its own `annotation.rendered` afterwards. That is not scored. The plan says a listener throw is not caught, and it does not define reentry. The tool-state emitter is synchronous in the same way.

This pass does not make F-071 done. The next change is `refusalFor` plus the D1 regressions. A fresh reviewer should see that tree. The product bytes were restored before this file was written.
