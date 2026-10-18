# F-100 review, pass 3

**Reviewed**: working tree on `sprint/s15`, uncommitted F-100 files
**Result**: 0 defects, 0 smells, 0 nitpicks

## Defects

None.

## Smells

None.

## Nitpicks

None.

## Verified clean

Pass 2's nitpick is the only intended delta, and it is corrected. The section 17.4 fence in `docs/lld/public-api.md` line 29 is:

```text
_pad: [u32; 3]
```

The HLD struct in `docs/hld/14-the-boundary-in-code.md` line 95 is:

```text
    pub _pad: [u32; 3],
```

The name is `_pad` on both lines. The type is a three-element `u32` array on both lines. The fence still omits `pub` and the trailing comma, matching the six fields above it. A search for the bare word `pad` in `docs/lld/public-api.md` finds nothing.

No other product change is visible against what pass 2 accepted. `git diff --stat` is still 4 files, 49 insertions and 6 deletions. The files are `docs/lld/README.md`, `docs/lld/typescript-packaging.md`, `packages/core/src/index.test.ts` and `packages/core/src/index.ts`. The diff still only adds the F-100 index exports, the hand-written `PUBLISHED` names, the type-only comment, the packaging row and the LLD index row. `git status --short` shows those four modifications plus the untracked `docs/lld/public-api.md`, `packages/core/src/viewport.ts`, `packages/core/src/viewport.test.ts`, `packages/core/src/node-test-modules.d.ts` and the pass 1 and pass 2 reviews. No Rust crate and no `packages/react` path is in that set.

`ViewportId` is still this brand at `packages/core/src/viewport.ts` lines 80 to 82:

```text
number & { readonly [viewportIdBrand]: "ViewportId" }
```

`WINDOW_PRESETS` still begins with `soft_tissue` at `packages/core/src/viewport.ts` line 111. `coreAvailable` still returns false at `packages/core/src/index.ts` line 119. `viewport.ts` has no import line. Its only `wasm` text is the comment that the module does not call into wasm. The test still rejects `wasm.memory` and `wasm-bindgen` in that source.

`npx vitest run packages/core/src/viewport.test.ts packages/core/src/index.test.ts` exited 0. Both files passed, 10 tests. `npm run typecheck` exited 0 (`tsc --build`).

`python3 scripts/prose_check.py` exited 0 and printed `OK: voice rules clean over 400 file(s)`. That command lists tracked files only. `docs/lld/public-api.md` is untracked, so that run does not cover it. The checker's own `scan` function, applied to the file, reported 0 problems. The only lines containing a semicolon are 23 and 29, both inside the section 17.4 fence. There is no em-dash and no en-dash. The command table rows have neither.

`packages/core/src/node-test-modules.d.ts` is still the three-call shim for `readFileSync`, `dirname`, `join` and `fileURLToPath`. It did not grow.

No source was mutated.
