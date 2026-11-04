# F-198 review, pass 4
**Reviewed**: staged tree 5d0148de3cc07e15bd906227382a7d1a65529303
**Author / reviewer**: implementation by the sprint host. Review by Claude Code, read only, with no shell.
**Result**: 0 defects, 1 smell, 2 nitpicks

## Acceptance challenged

AC1: `index.test.ts` imports `coreAvailable` without calling `loadCore` and expects false. `load.ts` starts `ready` at false.

AC2: in `load.ts`, `run` sets `ready = true` only after `await initialise()` returns. The catch returns false and leaves the flag alone. While a load is pending, a second call gets the same promise, and after a success the initialiser is never called again. `load.test.ts` asserts each of those transitions and checks `calls` equals 2.

AC3: `boundary.wasm.test.ts` requires `resolvedTier` and still bans the five pixel getter names. The worker's status message carries `available`, `tier` and `published`. The `accepted` message carries a byte count and the boundary. Neither message carries pixel data.

AC4: `postWadoInstance` gets the bytes through `DicomwebClient.retrieveInstance` and hands the caller the multipart envelope plus `kind.boundary`. `instance-bytes.ts` reads no DICOM tag.

AC5: the worker calls `loadCore`, then `bindCpuCanvas`, `writeFrame` of one f32 sample, `publishCpuCanvas`, and last `resolvedTier`. This is the only tier path the example takes. No adapter is stored on it, so the label comes from `resolve(TierRequest::Auto)`.

`holdSessionBytes` in `bulk.ts` calls `release_bulk`, `alloc` and `bulk_ticket` and copies the bytes into the slot. It calls neither `commit_frame` nor `commit_dicomweb_response`. The test double counts `commit_frame` calls and expects zero.

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|--------------------------|---------------------------|------------------------------|----------------------|
| `cargo clippy -p ocelli-wasm --all-targets -- -D warnings` | staged tree, default features | prior receipt | exit 0 |
| `cargo check -p ocelli-wasm --target wasm32-unknown-unknown` | staged tree, default features | prior receipt | exit 0 |
| `cargo test -p ocelli-wasm --lib tier_label` | staged tree, default features | prior receipt | 2 passed |
| `bin/ocelli.sh gate --floor` | staged tree | prior receipt | ALL GREEN, 26 gates |
| `bin/ocelli.sh gate corpus` | staged tree | prior receipt | ALL GREEN, 95 rows |
| Playwright Chromium `resolvedTier` | `crates/ocelli-wasm/target/boundary-wasm` | prior receipt | printed `A`, page realm, not the example worker |
| Feature table read | `crates/ocelli-wasm/Cargo.toml` lines 77 to 117 | read by the reviewer | `ocelli-render` is `optional = true` and enabled only by `render-worker` |
| Gate coverage of `decode-worker` | `bin/ocelli.sh` line 523, `scripts/decode_graph_check.py` line 199 | read by the reviewer | `wasm-decode` builds the wasm32 module. The graph check runs `cargo tree` only. Neither compiles host tests with `--no-default-features` |
| `tier_label` call sites | `crates/ocelli-wasm/src` grep | read by the reviewer | all six callers are in `canvas.rs`, plus the module's own tests |

Every receipt above is the sprint host's. This reviewer ran nothing.

## Defects

None.

## Smells

### S1, the test arm of the tier_label cfg drops the render-worker requirement

**Where**: `crates/ocelli-wasm/src/lib.rs:39`

**What**: the predicate is `any(test, all(target_arch = "wasm32", feature = "render-worker"))`. Its `test` arm does not require `render-worker`. `tier_label.rs` line 5 has `use ocelli_render::Tier`, and `ocelli-render` is an optional dependency that only `render-worker` enables. So a host test build of the declared `decode-worker` configuration compiles `tier_label` without the crate it imports. That covers `cargo test -p ocelli-wasm --no-default-features --features decode-worker`, and the same configuration under `cargo clippy --all-targets`. It should fail with an unresolved crate `ocelli_render`.

**Why it is wrong**: before this change, the `decode-worker` host test build compiled `panic` and the crate root tests. `session` and `canvas` are both gated on `render-worker`, so they stay out of that build. The new arm is the only path that pulls `ocelli-render` into it. The comment on lines 37 and 38 says "Tests still compile this module", and in that configuration the compile fails. That is the "host test that no longer compiles" class the brief asks about. The floor stays green only because no gate compiles that configuration.

**Evidence**: source trace through `Cargo.toml` lines 82, 114 and 117 and `tier_label.rs` lines 5 and 31. Not executed. Verdict PLAUSIBLE until someone runs the command.

**Scope and regression**: `all(feature = "render-worker", any(test, target_arch = "wasm32"))` keeps the wasm worker arm and the default host test arm. It also stays absent on host non-test builds, so it still satisfies the dead_code fix. Regression check: run the `decode-worker` host test command above and confirm it compiles and passes.

## Nitpicks

### N1, the production bundle carries a dead absolute path

`vite.config.ts` still defines `__OCELLI_WASM_URL__` as `/@fs` plus the absolute package path. A production bundle cannot serve it, so the page stays on `core not built`, which is the declared failure state. The current tree does not change this.

### N2, an empty tier line on a failed load

`start` posts `postStatus("")` when the load fails, and `core-panel.tsx` renders `Tier` with nothing after it. The current tree does not change this.

## Verified clean

The cfg remediation holds for the wasm worker. In `crates/ocelli-wasm/src/lib.rs`, the predicate `cfg(any(test, all(target_arch = "wasm32", feature = "render-worker")))` is a superset of the `canvas` predicate `cfg(all(target_arch = "wasm32", feature = "render-worker"))`. Every non-test caller of `tier_name` and `stored_tier_label` is inside `canvas.rs`. `tier-b-probe` implies `render-worker` at `Cargo.toml` line 99, so `tier_b_section_183` never compiles without the module. A host non-test build leaves the module out, which closes the dead_code denial. In a default-feature host test build, both functions are reached from the two tests, so neither is dead. The only gap is S1's `decode-worker` test arm.

Other checks:
- `tier_label.rs` has no `unsafe`, and neither does any other line in the diff. The only `unsafe` mentions under `crates/ocelli-wasm/src` are doc comments.
- wasm-bindgen appears only in `ocelli-wasm`. The new attribute is `#[wasm_bindgen(js_name = resolvedTier)]` in `canvas.rs`, and no TypeScript file imports wasm-bindgen.
- The two tier_label tests derive from the adapter-wins rule rather than from the body. Removing the `adapter` branch would make `stored_tier_label(Some(Tier::A), Some("C"))` return `Some("C")` and go red.
- `resolved_tier` writes the cached label only when no adapter is stored and no label is cached. It then re-reads through `stored_tier`, so an adapter opened during the await still wins.
- One worker loads one wasm instance through `loadCore`. The StrictMode test creates a fresh canvas for each effect setup and asserts two transfers.
- The flag prose in `lib.rs`, `index.ts`, `build-targets.md`, `typescript-packaging.md`, `dicomweb.md`, `CURRENT_SPRINT.md` and `README.md` matches `load.ts`.
- The `lib.rs` doc for `ocelli_version` says it returns `CARGO_PKG_VERSION`, the same text as `VERSION`. The literal `0.1.0` test and `index.ts` agree.

## Limitations and handoff

The reviewer ran nothing, opened no browser, and did not click the live example page. The worker realm was never the realm that printed `A`.

S1 is a source trace. It needs one run of the `decode-worker` host test command to confirm or clear it. If it is fixed, the next pass reviews the new predicate.

The completion ledgers will need updating after this pass:
- `docs/sprints/AS_BUILT.md` names pass 3 as the last review.
- `docs/sprints/SPRINT_TRACKER.md` says three review passes.
- Neither records the cfg remediation or this pass.
