# S39 review, pass 1

**Reviewed**: Range `3553c4db98a42d41525056b957a0f6f628c53ce9..HEAD` on branch `sprint/s39`. HEAD is `238329cadab4800a312bbdc68a376c79e18074a1`. Tree is `8b601bd5c34472617bca879dd0c70b1e893eb32a`. The open is `0d73b9a`. The design is `c446a84`. F-107 is `bf8981a`, tree `2f090d53326425ec9a8045e482ef2ad5b1e8f6e4`. F-109 is `238329c`. `git status --porcelain` was empty before this file. This review file is not part of the reviewed tree.

**Author / reviewer**: Author is the parent session. Reviewer is independent subagent `01a1a01e-f66b-7963-847e-5a2ba5cec064`. This reviewer did not write the S39 code, the designs, or the story reviews.

**Result**: 0 defects, 0 smells, 0 nitpicks

Closing this sprint completes M12 and does not publish. The version stays 0.1.0. There is no measured duration and no ratio. F-X003 stays in progress. This pass does not mark it done.

The feature gates already ran. This pass did not run the sprint profile.

## Acceptance challenged

HLD section 12 says both libraries coexist, with a feature flag per viewport. Stack viewports move first, then MPR, then 3D. Shadow mode renders both, diffs, alerts, and only then flips the flag. The author's strangler sentence contains an em-dash. This review does not copy it.

The approved plans name the gaps section 12 does not name. The kinds are `stack`, `mpr`, and `volume`. All three flags start off. A later kind cannot turn on while the earlier kind is off. An earlier kind cannot turn off while a later kind is on. `enableFlagged` enables only while that kind is on. Turning the kind off disables the elements this module enabled. `shadowDivergence` counts samples whose absolute difference is greater than an integer limit from 0 to 255. Equal differences at the limit are not counted. Unequal lengths and any other limit return null. The buffers are not written. The function does not render and does not import the other library. Those names, the off default, the order checks, the limit, and the refusal to import or render the other library are local decisions. They are not HLD formulas.

F-107 AC1 rejects MPR while stack is off. `setKindEnabled("mpr", true)` returned false on the package export. Stack, MPR, and volume stayed off.

AC2 rejects volume before MPR. It also rejects turning stack off while MPR is on, and turning MPR off while volume is on. On three real `EventTarget` values, volume off stopped only the volume seam. MPR off then stopped only the MPR seam. The stack seam still dispatched until stack was turned off.

AC3 rejects a second enable and a repeated true that disables. A second `setKindEnabled("stack", true)` left the listener list at `viewport.mounted` only. `frame.presented` still dispatched.

AC4 rejects an enable while the flag is off. It also rejects a mount that is still dispatching after `enableFlagged` returns. The probe used a real `EventTarget`. A `viewport.mounted` listener called `setKindEnabled("stack", false)` and that call returned true. After `enableFlagged` returned, the seam was null, the kind was off, and `dispatchOnElement` of `frame.presented` was false. The element recorded `viewport.mounted` and then `viewport.unmounted`. It did not record `frame.presented`. A second real element whose mount turned stack off left the first seam unable to dispatch. Both elements recorded `viewport.unmounted`.

AC5 rejects an unknown kind and a repeated set that disables. Unknown true and unknown false returned false. The stack element stayed mounted until a later off. A second off, when the flag was already off, returned true and did not send another `viewport.unmounted`.

F-109 AC1 rejects a count of 0 for `[0]` against `[2]` at limit 1. It also rejects counting `[0]` against `[1]` at limit 1. Changing `>` to `>=` made that second call return 1.

AC2 rejects counting equal bytes at limit 0. The same mutation returned 2 for `[0, 5]` against `[0, 5]`.

AC3 rejects unequal lengths and the limits 1.5, 256, and -1. Those stayed null under the mutation. The restored probe also got null for `NaN` and for `Infinity`. The buffers stayed unchanged.

AC4 rejects writing either buffer. The equal call at limit 0 is 0. The changed call is 1. The mutation failed the equal call and returned 2. On the restored function the probe copies stayed equal to the inputs.

An independent count rejects a non-strict comparison that the single-byte fixture could still hide. `[0, 10, 20]` against `[0, 12, 25]` at limit 2 returned 1. The absolute differences are 0, 2, and 5. Only 5 is above the limit. Difference 255 at limit 255 returned 0. The same bytes at limit 254 returned 1. Difference 4 at limit 4 returned 0 and at limit 3 returned 1. Empty buffers returned 0. One buffer passed as both arguments returned 0 and was not written.

The count does not flip a flag. With stack on, `shadowDivergence` of `[0]` and `[2]` at limit 1 returned 1 and left stack on. MPR stayed off. `shadow.ts` has no import. The F-109 commit does not edit `flag.ts`.

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|---|---|---|---|
| `git status --porcelain`, `git rev-parse HEAD`, `git rev-parse HEAD^{tree}` | `sprint/s39` before this review file | Executed now | Status empty. HEAD `238329cadab4800a312bbdc68a376c79e18074a1`. Tree `8b601bd5c34472617bca879dd0c70b1e893eb32a` |
| `git diff --name-only 3553c4db98a42d41525056b957a0f6f628c53ce9..HEAD` | Open `0d73b9a`, design `c446a84`, F-107 `bf8981a`, F-109 `238329c` | Executed now | 19 paths. `seam.ts`, `alongside.ts`, `events.ts`, and `migrate.ts` are absent. F-109 does not include `flag.ts` |
| `shasum -a 256` of `flag.ts`, `shadow.ts`, `seam.ts`, and `alongside.ts` | Clean tree before the mutation and after restore | Executed now | `c4daca94e72ace24876443d7c3923dc816e80834b00b7627cb115d34b709753f`, `49eb84526e5fd59f3de4a7d29f4266046e35c9593e6b23ecf1c4c9fda558a001`, `585c06cdc220e17f3073984acaa0dec5877cb7be372c29d963d973004582c779`, and `fcd56226cadfa29f03206dbdf2a7b169466bb899d0161963baef3da89b1ed836` |
| Header compare in Python | `docs/lld/public-api.md` line 3 and `docs/lld/migration.md` line 3 against the last cell of each README row | Executed now | Public-api strings are equal, 300 characters, and both end in F-109. Migration header and migration cell are both `F-108, F-106, F-107, F-109` |
| `npx vitest run` on `flag.test.ts`, `shadow.test.ts`, `index.test.ts`, `events.test.ts`, `seam.test.ts`, and `alongside.test.ts` | Clean tree. Node v24.13.0. Vitest 4.1.11 | Executed now | Exit 0. 6 files, 37 tests passed. 10 flag, 4 shadow, 4 index, 8 events, 6 seam, 5 alongside |
| `npx tsc -p packages/core --noEmit --pretty false` | `packages/core` tsconfig. TypeScript 5.9.3 | Executed now | Exit 0. No diagnostics. `--incremental false` was not passed |
| `npx vitest run packages/core/src/shadow.test.ts` after `>` became `>=` | Mutated sha256 `66f51c108d8702e502bdd5b55d969269f643f89c50770ef5aafa8ca10a2cfb81` | Executed now | Exit 1. 3 failed and 1 passed. AC1 expected 0 and received 1 for `[0]` against `[1]` at limit 1. AC2 expected 0 and received 2. AC4 expected 0 and received 2 for equal bytes at limit 0. AC3 stayed green |
| `cmp` of `/tmp/s39-shadow.ts.blob` after copy-back, then `shasum -a 256` | Restored `shadow.ts` | Executed now | `cmp` exit 0. sha256 returned to `49eb84526e5fd59f3de4a7d29f4266046e35c9593e6b23ecf1c4c9fda558a001`. `flag.ts` stayed `c4daca94e72ace24876443d7c3923dc816e80834b00b7627cb115d34b709753f`. `git diff --quiet` on `shadow.ts` was exit 0 |
| `npx vitest run --config /tmp/s39-sprint-vitest.config.ts` | Config outside the repo | Executed now, discarded | Exit 1. The config could not resolve `vitest/config` from `/tmp`. No product assertion ran. The config and the unused probe were deleted |
| `npx vitest run packages/core/src/s39-sprint-probe.test.ts` | Restored source. 8 cases. Real `EventTarget`. Package exports | Executed now | Exit 0. 8 passed. `PROBE_ROLLBACK {"seamNull":true,"kind":false,"via":false,"types":["mounted","unmounted"]}`. The probe file was deleted afterward |
| `npx eslint` on `flag.ts`, `flag.test.ts`, `shadow.ts`, `shadow.test.ts`, `index.ts`, and `index.test.ts` | Restored tree. ESLint from the repo | Executed now | Exit 0. No diagnostics |
| `python3 scripts/backlog_check.py` | `docs/sprints/BACKLOG.md` | Executed now | Exit 0. `OK: 213 F-IDs, 130 done, 5 hooks scheduled in Phase 1, 2 import defects declared` |
| Python compare of the F-106 section | Base `AS_BUILT.md` against HEAD | Executed now | The F-106 section is byte-identical. HEAD also has the F-107 and F-109 sections |
| Feature-commit trailers, `git rev-parse` of both trees, `wc -c` of the existing wasm | Prior verify records and the on-disk module | Trailer read, not re-run. Byte count executed now | F-107 trailer `profile=feature`, `corpus=pass`, `tree=2f090d533264`. F-109 trailer `profile=feature`, `corpus=pass`, `tree=8b601bd5c344`. Both prefixes match `git rev-parse`. The existing `ocelli_wasm_bg.wasm` is 2,171,131 bytes. This pass did not rebuild it |

No cargo, rustc, or wasm-pack command was started. `bin/ocelli.sh gate` was not run.

## Defects

None.

## Smells

None.

## Nitpicks

None.

## Verified clean

`git diff --name-only` lists the two designs, the three story reviews, `CHANGELOG.md`, the migration and public-api LLD files plus their index rows, the four sprint ledger files, and `flag.ts`, `flag.test.ts`, `shadow.ts`, `shadow.test.ts`, `index.ts`, and `index.test.ts`.

The three flags start off. `ORDER` is `stack`, then `mpr`, then `volume` at `packages/core/src/flag.ts` line 14. A same-value set returns true before any disable, at lines 66 to 68. An unknown kind returns false at lines 62 to 64. The previous-kind check and the next-kind check are lines 72 to 84. Turning a kind off deletes the stored rows, clears that kind's set, sets the flag false, and then calls `disable`. A repeated off does not enter that path.

`enableFlagged` reserves the element before `enableElement`. After that call, `kept` is true only when the reservation is still that kind and the flag is still on, at line 130. Otherwise the seam is disabled and the function returns null, at lines 131 to 143. The real-target probe observed that end state. A mount while the flag is off returns null and does not dispatch `viewport.mounted`.

`shadowDivergence` returns null before the walk when the lengths differ or `Number.isInteger(limit)` fails or the limit is outside 0 to 255, at `packages/core/src/shadow.ts` lines 14 to 19. The counted comparison is `>` at line 27. The function does not write either buffer and does not import another module. It does not edit `flag.ts` or the corpus comparator. Those paths are absent from the F-109 commit.

The source index exports `enableFlagged`, `kindEnabled`, `setKindEnabled`, and `shadowDivergence`. The probe saw one function identity for each. `index.test.ts` lists the four value names. `ViewportKind` is named there as type-only. `SideBySide` and `ElementSeam` stay type-only.

`flag.ts` imports `enableElement` and the `ElementSeam` type from `./seam.js` only. Neither new module imports another viewer. No wasm-bindgen and no `unsafe` was added. Tiers A, B, and C stay not applicable, as both plans state. There is no pixel arithmetic and no DICOM fixture. The counts are hand-computed from the F-109 plan. This function is not the section 25.1 comparator.

Backlog marks F-107 and F-109 `done`. `CURRENT_SPRINT.md` marks both `done`. The two sprint count commands both printed 2. The backlog awk for S39 printed both rows as `done`. M12 in the backlog is F-105, F-106, F-107, F-108, and F-109. Each of those five status cells is `done`. `python3 scripts/backlog_check.py` reported 130 done. F-X003 remains `in-progress`. F-X011 remains `pending`.

`SPRINT_TRACKER.md` has an F-107 row and an F-109 row. Both cells start with `not measured` and say no isolated duration was captured. Neither row records a ratio. `CURRENT_SPRINT.md` says there is no new measured duration and no ratio. The F-107 row says two review passes, the last at 0 defects and 0 smells, and a mount-rollback mutation. The committed pass 1 result is 1 defect, 0 smells, and 0 nitpicks. Pass 2 is 0 defects, 0 smells, and 0 nitpicks. The F-109 row says one review pass at 0 defects and 0 smells and a limit-comparison mutation. That matches the committed pass 1 review.

`AS_BUILT.md` has an F-107 section and an F-109 section. The F-106 section is unchanged from the base. The F-107 section says ten tests. `flag.test.ts` has ten `it` calls. The F-109 section says four tests. `shadow.test.ts` has four.

Unreleased Added starts with the count sentence. The next bullet is the flag sentence. The count sentence says the difference is above an integer limit, unequal lengths return nothing, the buffers are not written, and the function does not render. The flag sentence says the three flags start off, stack turns on before MPR, MPR turns on before volume, an element is enabled only while its kind is on, and turning that kind off disables the elements this story enabled. `docs/lld/migration.md` states that order, that off path, the greater-than count, both null refusals, and the refusal to render or import the other library. `docs/lld/public-api.md` states that order, that off path, the greater-than count, the unequal-length refusal, and the refusal to render.

`package.json` was not in the range. `packages/core/package.json`, `packages/react/package.json`, and the workspace `Cargo.toml` are version `0.1.0`. `index.test.ts` still expects `0.1.0`. D-22 remains in `docs/hld/DEVIATIONS.md`. The row says `openjph-core` 0.1.0 carries no licence text and that the pins assertion fails until the notice material is obtained. That still blocks publication.

`ci/wasm-size-budget.json` was not in the range. It still records 2,151,196 bytes and tolerance 0.05. The sprint names the ceiling 2,258,755. Multiplying that byte count by one plus the tolerance gives 2258755.8, so the named ceiling is the integer part. The on-disk wasm module is the 2,171,131 bytes the sprint attributes to the S38 gate. This pass did not rebuild it and did not remeasure a release build.

Added Markdown in the range has no em-dash. Every semicolon in that Markdown diff is TypeScript inside a fence, in the two designs or in the F-107 pass 1 review.

## Limitations and handoff

The committed flag suite uses a fake `dispatchEvent`. The real `addEventListener` rollback, the three-kind order, the unknown false, and the shadow boundaries above come from the probe. The probe is not part of the suite. It was deleted. `git status --porcelain` was empty again before this review file was written.

Calling `disable` on the seam returned by `enableFlagged` leaves the flag on and leaves the module entry in place. A later `enableFlagged` on that element returns null even though `frame.presented` no longer dispatches. Turning the kind off and on again clears the entry, and a new enable then dispatches. The plan's removal path is `setKindEnabled` false. This pass does not treat the other disable door as a defect or a smell.

An element enabled through `enableElement` is not stored. `enableFlagged` returns null. Turning the kind off does not disable that outside seam. Calling `disable` on that outside seam returns true, and `frame.presented` then stops.

The probe checked the state after `enableFlagged` returned. It did not sample `frame.presented` in the mounted listener after `setKindEnabled` had returned and before `enableFlagged` itself returned. A throwing listener on a real `EventTarget` was not re-run. The unit test that expects a null seam replaces `dispatchEvent` so the method throws. That is a different input from a listener exception.

No DOM parent chain was built. No browser document was opened. No adapter was opened. The feature floor, the corpus gate, and the comparison report were not re-run. A clean result here is not a sprint-profile record.

This review file is not part of the reviewed tree.

Closing this sprint completes M12 and does not publish. The version stays 0.1.0. There is no measured duration and no ratio. F-X003 stays in progress.

dwv, Horos, and the medical viewer Grok were not opened.
