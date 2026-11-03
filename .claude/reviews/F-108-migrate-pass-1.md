# F-108 review, pass 1

**Reviewed**: Uncommitted F-108 implementation on `sprint/s37`. HEAD is `b6c7bdb`. The review file is not part of the product diff.

**Reviewer**: Independent subagent `01a19f69-1425-7bd3-be51-0ba24282feeb`. This reviewer did not write the F-108 code or the design.

**Result**: 1 defect, 0 smells, 0 nitpicks

## Acceptance challenged

AC1 to AC4 each use one listener call, and the quote sits against the parenthesis. The suite does not put two calls in one file, does not use a single-quoted `MOUSE_DOWN`, and does not leave `ELEMENT_ENABLED` sitting in a comment.

The probe is `/tmp/f108-migrate-probe.test.ts`. It is not in the repo. It imports `rewriteListenerNames`, `CORNERSTONE_EVENT_MAP`, and the package index from source.

Two calls in one source both rewrite. A comment that only contains `ELEMENT_ENABLED` stays. `addEventListener('MOUSE_DOWN', down)` becomes `addEventListener('pointer.down', down)`. `MOUSE_DOWN_ACTIVATE` and `TOUCH_START_ACTIVATE` stay. Each of the 103 rows was tried for `addEventListener` and `removeEventListener`, and for both quote characters, with the quote against the parenthesis. Mapped names change to the row's Ocelli name. Null rows stay. A template, a mismatched quote, an escaped literal, a second argument, another call, a nested call, and `cornerstone.enable` stay. `null`, `undefined`, a number, `false`, and a plain object return null. Text with no listener literal keeps the same characters. A second pass keeps the same text. `core.rewriteListenerNames` is the same function.

None of that contradicts the plan.

The same probe does contradict the plan when the call uses the optional whitespace the approach describes. A mapped literal is replaced, and the whitespace between the call name and the quote is removed. A null row with the same spacing keeps every character, because the miss path returns the whole match. Details are under Defects.

## Evidence

| Command | Exit | Observation |
|---|---|---|
| `npm test -- packages/core/src/migrate.test.ts` after `const next = "viewport.mounted"` | 1 | `leaves an unavailable listener name` failed. Expected `element.addEventListener("CAMERA_MODIFIED", listener);` and received `element.addEventListener("viewport.mounted", listener);`. The enable test also failed because `ELEMENT_DISABLED` became `viewport.mounted`. |
| `cmp /tmp/f108-migrate.ts.blob packages/core/src/migrate.ts` | 0 | Byte match. sha256 before the edit and after the restore: `ff114addc76840f1f81283beaf5229d384652e4bd7a7d74490597acc50dcd2c2`. sha256 while mutated: `84d08ada81a4a801e9be393969c90b80309061cae8e637c1afee750d18ad6b03`. |
| `npm test -- packages/core/src/migrate.test.ts packages/core/src/index.test.ts` | 0 | 8 tests passed. |
| `npx tsc -p packages/core --noEmit --pretty false` | 0 | No diagnostics. `--incremental false` was not passed. |
| `npx eslint` on `migrate.ts`, `migrate.test.ts`, `index.ts`, and `index.test.ts` | 0 | No diagnostics. |
| `npx vitest run --config /tmp/f108-vitest.config.ts` | 1, then 0 | The first run failed the whitespace expectation. A follow-up on the restored function printed the three strings below and its other checks passed. |

The printed strings were:

- kept: `addEventListener ( "CAMERA_MODIFIED" , listener);`
- changed: `addEventListener("viewport.mounted" , listener);`
- newline form: `removeEventListener('viewport.unmounted',\n  listener);` from a source whose quote was on the next line

The public-api header and the last cell of the public-api row in `docs/lld/README.md` are the same 279-character string and both end in F-108. The migration header and the last cell of the migration row are both F-108.

## Defects

1. A successful rename deletes the optional whitespace the plan's scan allows.

The approach scans for the call name, optional whitespace, `(`, optional whitespace, and a quoted literal with no escape. It then says to replace that literal. The implementation includes both whitespace spans in the match and writes the call back as `name(` plus the quote.

```ts
const LISTENER =
  /(^|[^A-Za-z0-9_])(addEventListener|removeEventListener)\s*\(\s*(['"])([A-Z0-9_]+)\3/g;
```

```ts
return `${prefix}${call}(${quote}${next}${quote}`;
```

`addEventListener ( "ELEMENT_ENABLED" , listener);` becomes `addEventListener("viewport.mounted" , listener);`. The spaces around `(` are gone. The space before the comma stays, because it sits outside the match.

`removeEventListener(\n  'ELEMENT_DISABLED',\n  listener);` becomes `removeEventListener('viewport.unmounted',\n  listener);`. The newline and the indent between `(` and the quote are gone.

`addEventListener ( "CAMERA_MODIFIED" , listener);` is unchanged. A null row returns the whole match, so this loss happens only when the name is rewritten.

AC1 and AC3 never put whitespace in that position, so `migrate.test.ts` stays green. The characters that are not the literal do not all stay.

## Smells

None.

## Nitpicks

None.

## Verified clean

- `rewriteListenerNames` has the planned signature and is a value export on the package object. `index.test.ts` passed the runtime set. The probe saw the same function.
- With the quote against `(`, every catalogue row behaves as the map says. Both listener calls and both quote characters were checked. There are 103 rows and the cornerstone names are unique.
- `cornerstone.enable` stays. `migrate.ts` imports `CORNERSTONE_EVENT_MAP` from `events.ts` and does not import the seam. It does not call `enableElement` and it does not write a file.
- There is no second copy of the 103-row table. Null `ocelli` rows are omitted from the rename map.
- A comment that only mentions `ELEMENT_ENABLED` stays. So do a template literal, a bare string, another call, a nested call, a mismatched quote, and a literal that contains a backslash.
- A non-string returns null and does not throw. The same source returns the same text, including a second pass over an already rewritten string.
- `docs/lld/migration.md` names `enableElement`, the once-per-frame drain, stack then MPR then 3D, the `ELEMENT_ENABLED` to `viewport.mounted` example, and that `CAMERA_MODIFIED` stays because that row has no Ocelli name.
- The F-108 LLD headers match their README cells, as the evidence table says. The new LLD sentences have no em-dash and no prose semicolon.
- `events.ts` and `seam.ts` are outside the diff. No wasm-bindgen, no `unsafe`, and no pixel path was added.
- Forcing every lookup to `viewport.mounted` made the `CAMERA_MODIFIED` assertion fail. The saved blob was compared back with `cmp` and the sha256 matches the pre-edit file.

## Limitations and handoff

This pass did not run `bin/ocelli.sh gate`, cargo, rustc, wasm-pack, or the oracle. The plan's feature profile is still the floor gate plus the corpus gate, on a later completion commit.

The whitespace loss is a defect, so this pass is not clean. The next owner should keep both scanned whitespace spans when a literal is replaced, and should add a regression that goes red if `addEventListener (` or a newline after `(` is collapsed. Ledger files still say pending. That belongs to the completion commit after a clean pass. The implementation is uncommitted.

dwv, Horos, and the medical viewer Grok were not opened.
