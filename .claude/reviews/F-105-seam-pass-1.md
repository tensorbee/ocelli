# F-105 review, pass 1

**Reviewed**: Uncommitted F-105 implementation on `sprint/s37`. HEAD is `ab92b6d0fdafc388a70d7c72822f9ae27386c330` (`S37, approve the migration designs`). The review file is not part of the product diff.

**Reviewer**: Independent subagent `01a19f54-1096-7b82-a512-23836a073e43`. This reviewer did not write the F-105 code or the design.

**Result**: 0 defects, 0 smells, 0 nitpicks

## Acceptance challenged

The suite does not lock several plan rows that the implementation does implement.

AC2 shows that a second `enableElement` on the same fake returns null and that mounted was delivered once. It does not call the first seam's `dispatch` after that refusal. The state table says the first seam still dispatches.

AC3 shows that `dispatchOnElement` returns false after `disableElement`. It does not call the seam object returned by `enableElement`. A module-level slot that cleared on disable would also let one later object enable. The suite never enables two objects at once, and it never passes a non-element.

AC1 checks `toMatchObject` for generation 4. A cloned detail would pass. The plan says the detail is the same object the caller passed. The fake records the call inside `dispatchEvent`. It is not a listener.

AC5 spies on `HTMLDivElement.prototype.dispatchEvent` for mounted and unmounted. It does not drain a named event, so it does not show that the div and `onEvents` both see that event.

The probe is `/tmp/f105-seam-probe.test.tsx`, outside the repo. It imports `@ocelli/core` from `packages/core/src/index.ts`.

After disable, the returned seam's `dispatch` is false. A fresh object enables and delivers `viewport.resized`. `enableElement` returns null for null, for undefined, for a number, for a plain object, and for an object whose `dispatchEvent` is not a function. Two live fakes both register and each receives only its own later event. A second enable returns null and the first seam still dispatches `pointer.click`. The presented `detail` is the caller object. A throw from a later `dispatch` returns false and the next dispatch on that same seam succeeds. During `viewport.unmounted`, `dispatchOnElement` is already false. Mounted and unmounted both have `bubbles` false and `cancelable` false. A real `EventTarget` listener hears `viewport.mounted`, then `frame.presented` with generation 4, then `viewport.unmounted`.

`OcelliViewport` was rendered with one presented ring record (kind 6, generation 4, width 8, height 9, dropped 7). A listener added after commit and the `onEvents` callback received the same detail object. `bubbles` and `cancelable` on that DOM event were false. Dispatch during unmounted was false, and dispatch after unmount was false. Two host divs both received mounted and both received unmounted.

None of this contradicts the plan.

## Evidence

| Command | Exit | Observation |
|---|---|---|
| `npm test -- packages/core/src/seam.test.ts` after changing only `deliver`'s `bubbles` to true | 1 | `delivers a presented frame on the enabled element` failed. `expected true to be false` at `delivered.bubbles`. The other 5 tests passed. |
| restore `packages/core/src/seam.ts` from `/tmp/f105-seam.ts.blob` and `cmp` | 0 | Byte match. sha256 before the edit and after the restore: `585c06cdc220e17f3073984acaa0dec5877cb7be372c29d963d973004582c779`. |
| `npm test -- packages/core/src/seam.test.ts packages/core/src/index.test.ts packages/react/src/OcelliViewport.test.tsx` | 0 | 15 tests passed. |
| `npx tsc -p packages/core --noEmit --pretty false` | 0 | No diagnostics. |
| `npx eslint` on the six implementation files | 0 | No diagnostics. |
| `npx vitest run --config /tmp/f105-vitest.config.ts` | 0 | 6 probe tests passed. |

The unmounted `bubbles: false` was not edited. Both sites are false again on the restored file.

## Defects

None.

## Smells

None.

## Nitpicks

None.

## Verified clean

- `enableElement`, `disableElement`, and `dispatchOnElement` match the plan's signatures and live on the package object. `index.test.ts` passed the exact runtime export set.
- Registration is a `WeakMap`. A second enable does not replace the first. Disable deletes the entry before `viewport.unmounted`. A throw from that notification does not put the entry back. A throw from mounted delivery removes the entry and returns null. A throw from a later delivery returns false and leaves the entry.
- Unknown names return false without calling `dispatchEvent`. Catalogue delivery uses the Ocelli name, `bubbles: false`, `cancelable: false`, and the caller object as `detail`.
- The host div enables in an effect and disables in cleanup. Drained named events go to that div and the existing `onEvents` callback still runs, including `dropped`.
- `events.ts` is not in the diff. No wasm-bindgen, no `unsafe`, and no pixel path was added.
- Ledger files are absent. That is expected before the completion commit.

## Limitations and handoff

This pass did not run `bin/ocelli.sh gate`, cargo, wasm-pack, or the oracle. The plan's feature profile is still the floor gate plus the corpus gate, on a later completion commit.

`OcelliViewport.tsx` imports `@ocelli/core`, and the package entry is gitignored `packages/core/dist`. The component test therefore runs the local emit. `dist/seam.js` follows the same control flow as `src/seam.ts`. The probe loaded the source module directly and agreed.

F-108 was not started. The implementation is uncommitted. The next owner can write the ledgers and record the feature profile.

dwv, Horos, and the medical viewer Grok were not opened.
