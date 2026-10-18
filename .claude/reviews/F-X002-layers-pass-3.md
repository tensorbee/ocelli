# F-X002 review, pass 3

**Reviewed**: working tree on sprint/s14 against 879f80f
**Result**: 0 defects, 0 smells, 1 nitpick

## Defects

## Smells

## Nitpicks

### N1, the SwiftShader fixture is still shorter than the recorded renderer string

**Where**: `crates/ocelli-render/src/caps.rs:425` and `crates/ocelli-render/src/caps.rs:2528-2530`

**What**: Unchanged since pass 2. The negative fixture is `ANGLE (Google, SwiftShader Device)`. The recorded renderer string also contains `LLVM 10.0.0`. The shipped needle is still `llvmpipe`. This does not block.

## Verified clean

The new eslint block is `eslint.config.js` for `tools/swiftshader/**/*.mjs`, placed immediately after the F-042 `tools/tier-b/**/*.mjs` block. It sets `languageOptions.globals` and does not set `rules`. The four globals are `console`, `process`, `URL`, and `document`, each `readonly`, the same four as the tier-b block.

`tools/swiftshader/run.mjs` uses each of them in code eslint parses. `console.log` and `console.error` use `console`. `process.exit` uses `process`. `new URL` in `serve` uses `URL`. The `page.waitForFunction` callback uses `document`. No other unbound name appears in the script's own statements. `OffscreenCanvas` occurs only inside the page source string. `npm run lint` exited 0, so no missing global remains and none of the four is an unused name that would hide a typo.

`assertTheBanIsIntact` throws when a block sets `no-restricted-syntax` or the wasm-view rule to off outside the two recorded allowances, or when an enforcement block is not exactly the one error-level ban. The SwiftShader block has no `rules` property, so those checks skip it. `npm run lint` loads `eslint.config.js`, which calls `assertTheBanIsIntact` before export, and the command exited 0. The ban was not weakened.

The earlier `bin/ocelli.sh gate --floor` run failed on `lint` and is not a pass. Its wasm measurement was 2,011,491 bytes, baseline 1,999,061, ceiling 2,099,014. `docs/sprints/AS_BUILT.md` states that same wasm figure and says the budget file was not moved. This pass did not re-measure wasm and did not start a floor gate. The notes that say two review passes describe the passes before this one.

`scripts/lavapipe_layer.py` still returns 10 for the probe branch at line 59. `python3 -m unittest scripts/tests/test_lavapipe_layer.py` exited 0 with 6 tests. The file was not mutated. `ci/lavapipe-layer.json` is absent. Chrome was not started.
