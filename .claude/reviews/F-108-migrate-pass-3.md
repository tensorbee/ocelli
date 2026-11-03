# F-108 review, pass 3

**Reviewed**: Staged tree `4ba90113c128b2ec262d1f465302da7731f284f0` on `sprint/s37`. HEAD is `b6c7bdb`. `git status --short` at the start listed only staged paths, and the working tree matched the index. This review file is not staged.

**Reviewer**: Independent subagent `01a19f87-6117-79b0-821e-ecefdda48370`. This reviewer did not write the code, the design, or passes 1 and 2.

**Result**: 0 defects, 0 smells, 0 nitpicks

## Acceptance challenged

AC1 to AC4 were checked against `CORNERSTONE_EVENT_MAP`, not against the title of each unit test. `ELEMENT_ENABLED` maps to `viewport.mounted`. `ELEMENT_DISABLED` maps to `viewport.unmounted`. `CAMERA_MODIFIED` has a null Ocelli name. The three pass 1 whitespace strings were sent through `rewriteListenerNames` again.

- `addEventListener ( "ELEMENT_ENABLED" , listener);` became `addEventListener ( "viewport.mounted" , listener);`
- a `removeEventListener` call whose quote sits on the next indented line kept that newline and that indent, and the name became `viewport.unmounted`
- `addEventListener ( "CAMERA_MODIFIED" , listener);` stayed

The same three shapes are the regression in `migrate.test.ts`. The probe printed the same three results.

Each of the 103 rows was tried for `addEventListener` and `removeEventListener`, for both quote characters, and for six gaps. The gaps were `(`, a space before `(`, a newline and indent after `(`, a tab on each side of `(`, a carriage return on each side of `(`, and a blank line after `(`. The character before the call was `.`, so the name was not glued to an identifier. Mapped names changed to that row's Ocelli name and every other character stayed. Null rows stayed. A second pass kept that text. There are 29 mapped rows and 74 null rows. The cornerstone names are unique and each matches `[A-Z0-9_]+`. No mapped Ocelli name is itself an all-cap literal, and none contains a quote, a backtick, `$`, or a backslash.

One mixed source kept `cornerstone.enable`, a single-quoted `MOUSE_DOWN` rewritten to `pointer.down`, a spaced `MOUSE_DOWN_ACTIVATE`, a tabbed `TOUCH_START_ACTIVATE`, a bare `ELEMENT_ENABLED` string, a comment that only contains that name, `dispatchEvent`, `fooaddEventListener`, `addEventListenerFoo`, a nested call, a lowercase name, an escaped quote, a mismatched quote, and a template literal. The spaced `ELEMENT_ENABLED` call became `viewport.mounted`. A second pass kept that text.

`null`, `undefined`, a number, `false`, a plain object, a symbol, and a bigint return null and do not throw. An empty string and text with no listener literal keep the same characters. `core.rewriteListenerNames` is the same function as the direct export.

The floor failure was the listener scan in `does_not_listen_and_does_not_declare_a_second_catalogue`. The repaired predicate is `/addEventListener\s*\(/`. On `packages/core/src` and `packages/react/src`, skipping test files, the listener list is `["pointer.ts"]` and the catalogue list is `["events.ts"]`. `migrate.ts` contains the letters `addEventListener` in a comment and in the rewrite pattern, and that text does not match the call pattern. A string that is only those letters stays out. So does the same string followed by a space, a tab, or a newline. `pointer.ts` stays in. It calls `options.target.addEventListener(type, onDom)`.

Replacing the predicate with `text.includes("addEventListener")` made that one test fail. The received list was `["migrate.ts", "pointer.ts"]`. The saved index blob was copied back and `cmp` matched.

None of that contradicts the plan. The repair closes the listener false positive that failed the floor. D1 from pass 1 stays fixed. This pass did not re-run `bin/ocelli.sh gate --floor`.

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|---|---|---|---|
| `git write-tree` and `git rev-parse HEAD` | Index before any edit | Executed now | Exit 0. Tree `4ba90113c128b2ec262d1f465302da7731f284f0`. HEAD `b6c7bdbcd01aec1c77e3d50aaf58039ff2e959d4`. |
| `git status --short` | Staged F-108 paths | Executed now | Only staged paths. No unstaged product edit. |
| `shasum -a 256 packages/core/src/migrate.ts` | Current `migrate.ts` | Executed now, before the scan edit, during it, and after restore | Exit 0. `286fe2f00b8eb2343947f9654d64554efcb262aab5e803246bb406c48c8e254e` every time. |
| `npx vitest run packages/core/src/events.test.ts packages/core/src/migrate.test.ts packages/core/src/index.test.ts` | Restored product sources, before the scan edit | Executed now | Exit 0. 17 tests passed. 5 are in `migrate.test.ts`, 8 are in `events.test.ts`, and 4 are in `index.test.ts`. |
| `npx tsc -p packages/core --noEmit --pretty false` | `packages/core` tsconfig, include `src/**/*` | Executed now | Exit 0. No diagnostics. `--incremental false` was not passed. |
| `npx vitest run --config /tmp/f108-pass3-vitest.config.mjs` | Probe under `/tmp`, importing `migrate.ts`, `events.ts`, and `index.ts` | Executed now | Exit 0 on the corrected probe. 1 test passed. The three whitespace strings above were written to `/tmp/f108-pass3-probe-out.txt`. Counts were 103, 29, and 74. Listeners were `["pointer.ts"]`. Files containing the letters were `migrate.ts` and `pointer.ts`. Declarations were `["events.ts"]`. |
| Same probe with a prefix of `x` | `xaddEventListener("CACHE_SIZE_EXCEEDED", listener)` | Executed now, then discarded | Exit 1. The name stayed `CACHE_SIZE_EXCEEDED` because `x` is an identifier character. That is the boundary the pattern requires. The catalogue rerun used `.` and passed. |
| `npx vitest run packages/core/src/events.test.ts` after `text.includes("addEventListener")` | Index blob `c0310ac56f876f1c105c69053a7a9c07f95aaace` saved first | Executed now | Exit 1. `does_not_listen_and_does_not_declare_a_second_catalogue` failed. Expected `["pointer.ts"]` and received `["migrate.ts", "pointer.ts"]`. The other 7 tests in the file passed. The declaration expect was not reached. |
| `cmp` of the saved blob onto `packages/core/src/events.test.ts`, then `shasum -a 256` and `git write-tree` | Pre-edit `events.test.ts` | Executed now | `cmp` exit 0. sha256 returned to `4624d8b54c2db2bcb52899ac709d6d9875af45c48353c495be5ad0429fb6994c`. Tree returned to `4ba90113c128b2ec262d1f465302da7731f284f0`. |
| `pgrep -lf 'cargo\|rustc\|wasm-pack'` | Process list | Executed now | No matching compiler. Printed `NO_COMPILER`. |

The public-api header and the last cell of the public-api row in `docs/lld/README.md` are the same 279-character string and both end in F-108. The migration header and the last cell of the migration row are both F-108. `docs/lld/migration.md`, the F-108 paragraph in `docs/lld/public-api.md`, and the new `AS_BUILT` section have no em-dash and no prose semicolon.

## Defects

None.

The `events.test.ts` repair closes the false positive that failed the floor. `migrate.ts` is not a listener file under `/addEventListener\s*\(/`, and the `includes` form lists it. D1 from pass 1 stays fixed. The pattern captures the whitespace around `(` and writes that group back.

```ts
const LISTENER =
  /(^|[^A-Za-z0-9_])(addEventListener|removeEventListener)(\s*\(\s*)(['"])([A-Z0-9_]+)\4/g;
```

```ts
return `${prefix}${call}${gap}${quote}${next}${quote}`;
```

## Smells

None.

## Nitpicks

None.

## Verified clean

- D1 is fixed. The three pass 1 strings, the mixed source, and the six catalogue gaps keep every character except the mapped name.
- AC1 to AC4 match the plan. A non-string returns null. A template literal stays. `CAMERA_MODIFIED` stays. `cornerstone.enable` stays while `ELEMENT_DISABLED` becomes `viewport.unmounted`. `ELEMENT_ENABLED` becomes `viewport.mounted` and the old name is gone.
- `rewriteListenerNames` has the planned signature and is a value export. `index.test.ts` passed the hand-written runtime set, which names `rewriteListenerNames`. The probe saw the same function.
- There are 103 rows. 29 have an Ocelli name and 74 do not. Null rows are omitted from the rename map. No mapped Ocelli name can match the listener literal on a second pass. `migrate.ts` imports `CORNERSTONE_EVENT_MAP` from `events.ts` and does not import the seam. It does not call `enableElement` and it does not write a file.
- `docs/lld/migration.md` names `enableElement`, the once-per-frame drain, stack then MPR then 3D, the `ELEMENT_ENABLED` example, and that `CAMERA_MODIFIED` stays. `enableElement` registers one target and dispatches `viewport.mounted`. `packages/react/src/pump.ts` drains the ring once in `stepBody`. F-107 and F-109 are still pending. The guide does not implement a flag or shadow mode.
- The new `AS_BUILT` behaviour sentences match the function. The section records pass 1 as 1 defect, 0 smells and 0 nitpicks, reviewer `01a19f69-1425-7bd3-be51-0ba24282feeb`, and pass 2 as 0, 0 and 0, reviewer `01a19f76-48c2-76b3-b70c-56df59f3a196`. The recorded `migrate.ts` sha256 matches the file. `migrate.test.ts` has five tests.
- The listener repair leaves the declaration check on `export const OCELLI_EVENTS =`. The green run expected `["events.ts"]` and passed. React sources have no `addEventListener`. Non-test files that contain the letters are only `migrate.ts` and `pointer.ts`.
- `events.ts` and `seam.ts` are outside the diff. No `wasm-bindgen`, no `unsafe`, and no pixel path was added. The only `as` in `migrate.ts` is `as const` on the rename pair. It is not a numeric cast.
- `migrate.ts` was not edited. Its sha256 stayed `286fe2f00b8eb2343947f9654d64554efcb262aab5e803246bb406c48c8e254e`.

## Limitations and handoff

This pass did not run `bin/ocelli.sh gate`, cargo, rustc, wasm-pack, or the oracle. No compiler from that set was already running. The plan's feature profile is still the floor gate plus the corpus gate, on the completion commit.

`.claude/verify-ledger.json` has no row for tree `4ba90113c128b2ec262d1f465302da7731f284f0`. The new `AS_BUILT` verification sentence says the gates and the corpus result are the ledger entry for this commit. That sentence is the standing completion template. It is not a receipt that this pass confirmed, and a done status before the profile is not a finding here. The floor still has to be run on this tree. The tracker phrase about staged feature verification is in the same position.

The `includes` mutation failed on the listener list before the declaration expect. The green run before that edit is the observation that the declaration list is `["events.ts"]`. `migrate.ts` was not mutated again. Pass 2 already forced every lookup to `viewport.mounted` and restored the same sha256.

The first probe config could not resolve `vitest/config` from `/tmp`. That run is an environment miss. The probe files were removed and are not part of the diff. Pass 1 and pass 2 were not edited.

dwv, Horos, and the medical viewer Grok were not opened.
