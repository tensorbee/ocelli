# S37 review, pass 1

**Reviewed**: Range `ed605243..HEAD` on `sprint/s37`. HEAD is `90682553c2fc038e883746a657ab9bb3e00fc4d5`. Tree is `e95fb2a9c9899ad6befbac49c7b435ce8f461bbd`. This review file is not part of that tree.

**Reviewer**: Independent subagent `01a19f98-e2b0-7370-91fb-5c3bc0a7f8bc`. This reviewer did not write the S37 code, the designs, or the story reviews.

**Result**: 0 defects, 0 smells, 0 nitpicks

## Acceptance challenged

The approved plans are the contract. A missing HLD formula was not treated as specified. Cornerstone behaviour was not treated as specified. Closing this sprint does not complete M12. It does not publish.

F-105 was checked against the seam plan, not against the title of each unit test. A second `enableElement` on the same object returns null, and the first seam still dispatches. The presented `detail` is the caller object. An unknown name returns false and does not call `dispatchEvent`. During `viewport.unmounted`, `dispatchOnElement` is already false, so the registration is gone before that event. A second live element still receives only its own later event. `null` and a plain object return null. Mounted, presented, and unmounted events have `bubbles` false and `cancelable` false.

The React host uses the package functions. After render, `enableElement` from `@ocelli/core` on that div returns null. One later `requestAnimationFrame` drains the ring once. The div and `onEvents` receive the same `frame.presented` object, generation 4, with `dropped` 7. Dispatch during unmount is false. Dispatch after unmount is false. A later enable on that same div succeeds.

F-108 was checked against `CORNERSTONE_EVENT_MAP`. All 103 cornerstone names are unique and match `[A-Z0-9_]+`. There are 29 mapped rows and 74 null rows. Every mapped Ocelli name is in `OCELLI_EVENTS`. Each row was sent through `addEventListener` and `removeEventListener`, with both quote characters, with the quote against `(`, with a space before `(`, and with a newline and indent after `(`. Mapped names change to that row's Ocelli name and every other character stays. Null rows stay. A second pass keeps the text. `ELEMENT_ENABLED` becomes `viewport.mounted`, which is the event type `enableElement` dispatches. `ELEMENT_DISABLED` becomes `viewport.unmounted`. `CAMERA_MODIFIED` stays. `cornerstone.enable` stays. A template literal stays. A glued `fooaddEventListener` stays. A number returns null. The module text has no `node:fs` and no `writeFile`.

The catalogue scan uses `/addEventListener\s*\(/`. On the non-test files in `packages/core/src` and `packages/react/src`, that pattern matches `pointer.ts` and does not match `migrate.ts`. `migrate.ts` still contains the letters `addEventListener`. Replacing the predicate with `text.includes("addEventListener")` lists `migrate.ts`.

`git diff --name-only ed605243..HEAD` has 23 paths. None is `packages/core/src/events.ts`. None is under `crates/`. F-105's commit does not include `events.ts`. F-108 imports `CORNERSTONE_EVENT_MAP` and does not edit the map. The F-105 section in `docs/sprints/AS_BUILT.md` is byte-identical before and after the F-108 commit.

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|---|---|---|---|
| `git status --short`, `git rev-parse HEAD`, `git rev-parse HEAD^{tree}` | `sprint/s37` before edits | Executed now | Status empty. HEAD `90682553c2fc038e883746a657ab9bb3e00fc4d5`. Tree `e95fb2a9c9899ad6befbac49c7b435ce8f461bbd` |
| `git diff --name-only ed605243..HEAD` and per-commit names | F-105 `b6c7bdb`, F-108 `9068255` | Executed now | 23 paths. `events.ts` absent. No crate, shader, or measurement product file |
| `shasum -a 256` of `seam.ts`, `migrate.ts`, and `events.test.ts` | Clean tree before mutation and after each restore | Executed now | `585c06cdc220e17f3073984acaa0dec5877cb7be372c29d963d973004582c779`, `286fe2f00b8eb2343947f9654d64554efcb262aab5e803246bb406c48c8e254e`, `4624d8b54c2db2bcb52899ac709d6d9875af45c48353c495be5ad0429fb6994c` |
| Header compare | `docs/lld/public-api.md` and the public-api row in `docs/lld/README.md` | Executed now | Same 279-character string. Both end in F-108. Migration header and migration row cell are both F-108 |
| Map walk in `/tmp/s37-sprint-probe.test.tsx` | Source `events.ts`, `migrate.ts`, and `seam.ts` | Executed now | Exit 0. 103 rows, 29 mapped, 74 null. Mapped names are in `OCELLI_EVENTS` |
| Same probe, seam cases | Source `seam.ts` | Executed now | Exit 0. Second enable is null. First seam still dispatches the same detail. Nested dispatch during unmounted is false |
| Same probe, `OcelliViewport` | Package emit via `@ocelli/core`, one presented ring record, kind 6, generation 4, dropped 7 | Executed now | Exit 0. One drain. Div and `onEvents` share the detail. Package `enableElement` on the live div returns null |
| `npx vitest run packages/core/src/seam.test.ts` after `deliver` `bubbles: true` | Blob `01d1d4bc2043c73b69d01543fae11a0b215d4477` saved first | Executed now | Exit 1. Vitest 4.1.11. `delivers a presented frame on the enabled element` expected false and received true at `delivered.bubbles`. 5 passed |
| `git cat-file` restore, `cmp`, `shasum -a 256` | Saved seam blob | Executed now | `cmp` matched. Hash returned to `585c06cdc220e17f3073984acaa0dec5877cb7be372c29d963d973004582c779` |
| `npx vitest run packages/core/src/events.test.ts` after `text.includes("addEventListener")` | Blob `c0310ac56f876f1c105c69053a7a9c07f95aaace` saved first | Executed now | Exit 1. Expected `["pointer.ts"]` and received `["migrate.ts", "pointer.ts"]`. 7 passed |
| `git cat-file` restore, `cmp`, `shasum -a 256`, `git rev-parse HEAD^{tree}` | Saved events test blob | Executed now | `cmp` matched. Hash returned to `4624d8b54c2db2bcb52899ac709d6d9875af45c48353c495be5ad0429fb6994c`. Tree unchanged. Status empty |
| `npx vitest run` on `seam.test.ts`, `migrate.test.ts`, `events.test.ts`, `index.test.ts`, and `OcelliViewport.test.tsx` | Restored tree | Executed now | Exit 0. 28 passed. 6 seam, 5 migrate, 8 events, 4 index, 5 host |
| `npx tsc -p packages/core --noEmit --pretty false` | Restored tree | Executed now | Exit 0. `--incremental false` was not passed |
| `pgrep -lf 'cargo\|rustc\|wasm-pack'` | Process list before the suite | Executed now | No matching compiler |

## Defects

None.

## Smells

None.

## Nitpicks

None.

## Verified clean

`enableElement` returns null for a second registration and for a value with no `dispatchEvent` function. The registration is a `WeakMap`. Disable deletes the entry and then dispatches `viewport.unmounted` with `bubbles` false and `cancelable` false. A throw from mounted delivery removes the entry and returns null. The delivered event type is the Ocelli name. The detail of a caller event is that object. Unknown names return false without calling `dispatchEvent`.

`OcelliViewport` imports `enableElement` and `dispatchOnElement`. The effect enables the host div and the cleanup calls the returned seam's `disable`. The pump callback still drains once per frame. Drained named events are dispatched on that div and the existing `onEvents` callback still runs, including `dropped`. The package emit in `packages/core/dist/seam.js` follows the same control flow as `packages/core/src/seam.ts`, including both `bubbles: false` sites.

`rewriteListenerNames` returns null for a non-string and otherwise returns a string. It does not write a file. Whitespace captured between the call name and the literal is written back. `docs/lld/migration.md` names `enableElement`, the once-per-frame drain, stack then MPR then 3D, the `ELEMENT_ENABLED` example, and that `CAMERA_MODIFIED` stays because that row has no Ocelli name. The codemod does not import the seam and does not rewrite `cornerstone.enable`.

The public-api header and the last cell of the public-api row are the same string and both end in F-108. The migration header and the migration row's last cell are both F-108. Backlog rows for F-105 and F-108 are `done`. `CURRENT_SPRINT.md` marks both `done`. F-X003 stays `in-progress`. F-X011 stays `pending`. Unreleased names the element seam and the listener rewrite, and it says nothing is published. `AS_BUILT.md` has both sections. The F-105 section was not rewritten into the F-108 section.

The new tracker cells start with `not measured`. They do not record a duration or a ratio. `packages/core/package.json` is outside the diff and its version is `0.1.0`. D-22 remains in `docs/hld/DEVIATIONS.md`. The F-105 verify trailer names tree `747b52e15d89`, which is `b6c7bdb^{tree}`. The F-108 verify trailer names tree `e95fb2a9c989`, which is this HEAD tree. Added sprint markdown, with fenced blocks and inline code removed, has no em-dash and no prose semicolon.

M12 is S37 to S39. F-106 remains pending in S38. F-107 and F-109 remain pending in S39. Closing S37 does not complete M12 and does not publish.

## Limitations and handoff

This pass did not run cargo, rustc, wasm-pack, the oracle, or `bin/ocelli.sh gate`. No such compiler was running. Feature-profile trailers were read and were not re-executed. The sentence that the S36 sprint gate measured 2,171,131 bytes was a label change from the S35 sentence in the open commit. This pass did not rebuild the wasm module, so that byte count was not remeasured. The budget file still records 2,151,196 bytes with tolerance 0.05. The size script's ceiling is `int(baseline * (1 + tolerance))`, which is 2,258,755. The pins gate was not run.

The host test and the probe's component path load gitignored `packages/core/dist`. The unit probe loaded `packages/core/src`. Those are separate module instances. The emit was read and matches the source control flow. The probe file was deleted. It was not added under `packages/` or `docs/`.

Story reviews were prior receipts. This pass re-executed the acceptance claims above. A clean result here is not a sprint-profile record.

dwv, Horos, and the medical viewer Grok were not opened.
