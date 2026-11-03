# F-108 review, pass 2

**Reviewed**: Uncommitted F-108 repair on `sprint/s37`. HEAD is `b6c7bdb`. Pass 1 is unchanged.

**Reviewer**: Independent subagent `01a19f76-48c2-76b3-b70c-56df59f3a196`. This reviewer did not write the code, the design, or pass 1.

**Result**: 0 defects, 0 smells, 0 nitpicks

## Acceptance challenged

AC1 to AC4 each use one listener call. The whitespace regression calls the function three times. Neither puts a mapped call, a null row, and `cornerstone.enable` in one source. Neither walks the catalogue with whitespace between `(` and the quote.

The probe was `packages/core/src/f108-pass2-probe.test.ts`. It is not in the repo. It was deleted before the mutation and before the restored-tree commands. It imports `rewriteListenerNames`, `CORNERSTONE_EVENT_MAP`, and the package index from the package source.

The three pass 1 strings were printed.

- `addEventListener ( "ELEMENT_ENABLED" , listener);` became `addEventListener ( "viewport.mounted" , listener);`
- `removeEventListener(\n  'ELEMENT_DISABLED',\n  listener);` became `removeEventListener(\n  'viewport.unmounted',\n  listener);`
- `addEventListener ( "CAMERA_MODIFIED" , listener);` stayed

Those three shapes were also placed in one source with a single-quoted `MOUSE_DOWN`, a spaced `MOUSE_DOWN_ACTIVATE`, a tabbed `TOUCH_START_ACTIVATE`, a bare `ELEMENT_ENABLED` string, a comment that only contains that name, `dispatchEvent`, `fooaddEventListener`, `addEventListenerFoo`, a nested call, a lowercase second argument, an escaped quote, a mismatched quote, a template literal, and `cornerstone.enable`. Mapped listener literals changed. Every other token stayed. A second pass kept that text.

Each of the 103 rows was tried for `addEventListener` and `removeEventListener`, for both quote characters, and for six gaps: `(`, ` (`, a newline and indent, a tab on each side of `(`, CRLF around `(`, and a blank line after `(`. Mapped names changed to that row's Ocelli name and every other character stayed. Null rows stayed. A second pass kept the same text. The cornerstone names are unique and each matches `[A-Z0-9_]+`. There are 29 mapped rows and 74 null rows. No mapped Ocelli name is itself an all-cap literal, and none contains a quote, a backtick, `$`, or a backslash. `null`, `undefined`, a number, `false`, a plain object, a symbol, and a bigint return null and do not throw. An empty string and text with no listener literal keep the same characters. `core.rewriteListenerNames` is the same function.

None of that contradicts the plan. D1 from pass 1 is fixed.

## Evidence

| Command | Exit | Observation |
|---|---|---|
| `npx vitest run packages/core/src/f108-pass2-probe.test.ts` | 0 | 1 test passed. The three strings above were printed. The probe file was removed after this run. |
| `npm test -- packages/core/src/migrate.test.ts` after `const next = "viewport.mounted"` | 1 | 3 failed and 2 passed. `leaves an unavailable listener name` expected `element.addEventListener("CAMERA_MODIFIED", listener);` and received `element.addEventListener("viewport.mounted", listener);`. The enable example failed because `ELEMENT_DISABLED` became `viewport.mounted` while `cornerstone.enable` stayed. The whitespace example failed on the newline form, which became `viewport.mounted` instead of `viewport.unmounted`, and vitest did not reach that example's `CAMERA_MODIFIED` expectation. |
| `cmp /tmp/f108-pass2-migrate.ts.blob packages/core/src/migrate.ts` | 0 | Byte match. sha256 before the edit and after the restore is `286fe2f00b8eb2343947f9654d64554efcb262aab5e803246bb406c48c8e254e`. That is the expected pre-edit hash. sha256 while mutated is `32f8b78042acc25c1d8aab4df621eede19251834ecb91eed34af734f8ec751e2`. |
| `npm test -- packages/core/src/migrate.test.ts packages/core/src/index.test.ts` | 0 | 9 tests passed. 5 are in `migrate.test.ts` and 4 are in `index.test.ts`. |
| `npx tsc -p packages/core --noEmit --pretty false` | 0 | No diagnostics. `--incremental false` was not passed. |
| `npx eslint` on `migrate.ts`, `migrate.test.ts`, `index.ts`, and `index.test.ts` | 0 | No diagnostics. |

The public-api header and the last cell of the public-api row in `docs/lld/README.md` are the same 279-character string and both end in F-108. The migration header and the last cell of the migration row are both F-108.

## Defects

None.

D1 from pass 1 is fixed. The pattern captures the whitespace around `(` as its own group and writes that group back. A name that is not in the rename map still returns the whole match.

```ts
const LISTENER =
  /(^|[^A-Za-z0-9_])(addEventListener|removeEventListener)(\s*\(\s*)(['"])([A-Z0-9_]+)\4/g;
```

```ts
return `${prefix}${call}${gap}${quote}${next}${quote}`;
```

`addEventListener ( "ELEMENT_ENABLED" , listener);` becomes `addEventListener ( "viewport.mounted" , listener);`. The spaces around `(` stay. The space before the comma stays.

`removeEventListener(\n  'ELEMENT_DISABLED',\n  listener);` becomes `removeEventListener(\n  'viewport.unmounted',\n  listener);`. The newline and the indent between `(` and the quote stay.

`addEventListener ( "CAMERA_MODIFIED" , listener);` is unchanged.

## Smells

None.

## Nitpicks

None.

## Verified clean

- D1 is fixed, as the three strings, the mixed source, and the six catalogue gaps show.
- `rewriteListenerNames` has the planned signature and is a value export on the package object. `index.test.ts` passed the runtime set. The probe saw the same function.
- With or without whitespace around `(`, every catalogue row behaves as the map says. Both listener calls and both quote characters were checked. There are 103 rows and the cornerstone names are unique. 29 rows have an Ocelli name and 74 do not.
- `cornerstone.enable` stays. `migrate.ts` imports `CORNERSTONE_EVENT_MAP` from `events.ts` and does not import the seam. It does not call `enableElement` and it does not write a file.
- There is no second copy of the 103-row table. Null `ocelli` rows are omitted from the rename map. No mapped Ocelli name can match the listener literal on a second pass.
- A comment that only mentions `ELEMENT_ENABLED` stays. So do a bare string, a template literal, another call, a nested call's surrounding tokens, a mismatched quote, a literal that contains a backslash, `fooaddEventListener`, and `addEventListenerFoo`.
- A non-string returns null and does not throw. The same source returns the same text, including a second pass over an already rewritten string.
- `docs/lld/migration.md` names `enableElement`, the once-per-frame drain, stack then MPR then 3D, the `ELEMENT_ENABLED` to `viewport.mounted` example, and that `CAMERA_MODIFIED` stays because that row has no Ocelli name.
- The F-108 LLD headers match their README cells, as the evidence table says. The new LLD sentences have no em-dash and no prose semicolon.
- `events.ts` and `seam.ts` are outside the diff. No wasm-bindgen, no `unsafe`, and no pixel path was added.
- Forcing every lookup to `viewport.mounted` made the `CAMERA_MODIFIED` assertion fail. The saved blob was compared back with `cmp` and the sha256 matches the pre-edit file.

## Limitations and handoff

This pass did not run `bin/ocelli.sh gate`, cargo, rustc, wasm-pack, or the oracle. The plan's feature profile is still the floor gate plus the corpus gate, on a later completion commit.

D1 is fixed and this pass found no new defect. Nothing in the product diff needs another repair from this review. Ledger files still say pending. That belongs to the completion commit after a clean pass. The implementation is uncommitted. Pass 1 was not edited. The probe file was removed and is not part of the diff.

Under the forced lookup, vitest failed the whitespace example on `ELEMENT_DISABLED` before it reached that example's `CAMERA_MODIFIED` line. The unavailable-name test is the assertion that printed the `CAMERA_MODIFIED` rewrite.

dwv, Horos, and the medical viewer Grok were not opened.
