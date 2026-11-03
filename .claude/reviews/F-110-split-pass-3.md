# F-110 review, pass 3

**Reviewed**: working tree on sprint/s40 at HEAD e17e73e, including untracked `scripts/decode_graph_check.py` and `ci/wasm-decode-size.json`. Nothing was staged. `ci/wasm-size-budget.json` is not in the diff.
**Author / reviewer**: Author is the parent session. Reviewer is a fresh independent subagent and did not write the diff or the earlier passes.
**Reviewer id**: 01a1a070-422a-7772-ac36-b9d91e1f322b
**Result**: 0 defects, 0 smells, 0 nitpicks

## Acceptance challenged

AC1. The decode graph must not link wgpu, naga, or ocelli-render. `python3 scripts/decode_graph_check.py` exited 0. A separate decode `cargo tree` printed no wgpu, naga, ocelli-render, ocelli-viewport, or ocelli-compute row. `pkg-decode/ocelli_wasm_bg.wasm` is 16455 bytes and does not contain those names or `attachOffscreenCanvas`. `decode-worker = []`.

AC2. The default wasm32 graph must still link wgpu. The same graph check exits 0 only when the default tree contains `wgpu v`. The render package graph resolves `wgpu v30.0.1`. The render module contains the byte strings `wgpu` and `naga`.

AC4. The oracle wasm32 feature list is `std`, `parking_lot`, `wgsl`, `webgpu`, and `webgl`. It does not contain `dx12`. `dx12` remains on the oracle native line. The graph check's workspace half passed, so the wasm32 resolve does not enable `dx12`, `metal`, `gles`, `vulkan`, `glsl`, or `spirv`.

AC5. `ocelli-render` names `webgl` on both target tables. The wasm32 lists on `ocelli-render`, `ocelli-wasm`, and `ocelli-compute` are the D-27 set with `default-features = false`. Native lines name the pinned default backends. Only `ocelli-render` adds native `webgl`. There is no shared line whose only feature is `webgl`.

AC6 and AC7. The render module is 2,171,133 bytes against baseline 2,151,196 and ceiling 2,258,755. The decode module is 16,455 bytes against baseline 16,455 and ceiling 17,277. Both recorded tolerances are 0.05. The render budget file was not edited. This pass did not pass `--accept-size`.

AC8. The decode `.d.ts` exports `install_panic_hook`, `ocelli_version`, `panic_record_len`, and `panic_record_ptr`. It does not name `attachOffscreenCanvas`. The render `.d.ts` has those four plus the canvas exports. Leaving out generated `initSync` leaves 24 render exports.

Pass 2 D1 is closed. The D-14 We do cell no longer quotes `wgpu = { workspace = true, features = ["webgl"] }`. It says `ocelli-render` names `webgl` on both target tables, points at D-27 for `{ version = "=30.0.1", default-features = false }`, and says there is no shared line whose only feature is `webgl`. The Why cell says the version pin is still `=30.0.1`.

Pass 2 D2 is closed. The known-gaps bullet says the names come from `[workspace.dependencies]` and that `checked_dependencies` is not read by the check. The length one-liner is gone. The read path loads `allowed` as a filter and builds the name set from the workspace table.

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|--------------------------|---------------------------|------------------------------|----------------------|
| `pgrep -lf 'cargo\|rustc\|wasm-pack'` | host processes before the graph check | executed now | no match |
| `python3 scripts/guard_probe.py --only pins.range --only pins.partial-version --only pins.absent --only pins.table-form` | pin probes over the table-form workspace entry | executed now | exit 0. Three red, one green. `pins.table-form` stayed green. |
| `python3 scripts/deviation_check.py` | `docs/hld/DEVIATIONS.md` | executed now | exit 0. 27 deviations. D-01 matches `Cargo.toml`. |
| `python3 scripts/decode_graph_check.py` | working tree after the oracle target split | executed now | exit 0. Decode graph has no wgpu or naga. Render graph keeps wgpu. wasm-opt stays on. |
| `python3 scripts/target_feature_check.py` | `ci/target-feature-baseline.json`, `allowed` key `wgpu` | executed now | exit 0. 20 dependencies, 1 declared exception. |
| default `cargo tree -p ocelli-wasm --target wasm32-unknown-unknown -e normal` | package graph, format `{p}\|{f}` | executed now | `wgpu v30.0.1` features `parking_lot,std,web,webgl,webgpu,wgpu-core,wgsl`. |
| decode `cargo tree -p ocelli-wasm` with `--no-default-features --features decode-worker` | decode package graph | executed now | no wgpu, naga, ocelli-render, ocelli-viewport, or ocelli-compute row. |
| file sizes and `.d.ts` export lines | `pkg/ocelli_wasm_bg.wasm`, `pkg-decode/ocelli_wasm_bg.wasm`, both `.d.ts` files | executed now | 2171133 and 16455 bytes. Decode bindings have the four names plus `initSync` and no canvas name. |
| `rg` of `features = ["webgl"]` in `docs/hld/DEVIATIONS.md` | current deviations file | executed now | no match. |

`git status` after these commands matches the story diff. The render budget file is still unmodified. `decode-worker = []`. The oracle wasm32 feature list does not contain `dx12`.

## Defects

None.

Pass 2 D1 is closed. Pass 2 D2 is closed. No new defect was found.

## Smells

None.

## Nitpicks

None.

## Verified clean

The workspace entry is `{ version = "=30.0.1", default-features = false }` at `Cargo.toml:73`. No current-state sentence in the changed docs says that entry is untouched. D-14's Why cell says the version pin is still `=30.0.1` and that the `default-features` switch belongs to D-27. D-27's We do cell matches that workspace line, the three wasm32 lists, and the native lists, including native `webgl` only on `ocelli-render`.

`docs/lld/build-targets.md` and `docs/lld/tier-resolution.md` quote the same workspace entry and the same target split. Their header F-IDs match the last cell of their rows in `docs/lld/README.md`. The old shared-feature quote is absent from the deviations file.

`scripts/target_feature_check.py` builds its name set from `[workspace.dependencies]`. It reads `allowed` and does not read `checked_dependencies`. With only `wgpu` in `allowed`, the checker exited 0, so the other direct dependencies still resolve the same features on both targets. A wgpu difference is recorded and then dropped only when the first word of the problem is in `allowed`. The graph check shows the wasm32 and host wgpu sets differ, so removing that key would make the feature check fail. That deletion was not repeated in this pass. The code path is the filter at `scripts/target_feature_check.py:171`.

The printed package-graph feature field also contains `web` and `wgpu-core`. Those names are activated by `webgl` and `webgpu` inside wgpu 30.0.1. The step 4 sentence lists the five requested names and the four-backend host delta. The graph check requires those five names on the workspace wasm32 resolve and rejects the four native backends there.

`bin/ocelli.sh` runs the graph check, then `wasm`, then `wasm-decode`, then `--with-size`. `wasm-decode` passes `--no-default-features --features decode-worker` and writes `pkg-decode/`. That directory is gitignored. `canvas` compiles only for wasm32 and `render-worker`. `session` compiles only with `render-worker`. `default` is `render-worker` only. `tier-b-probe` enables `render-worker`.

`check_size` writes `ci/wasm-size-budget.json` only when that file is absent or `--accept-size` is set. On `--accept-size` it returns before `check_decode_size`. `check_decode_size` reads `ci/wasm-decode-size.json` and refuses a tolerance other than 0.05. Both artefacts were built on 2026-11-03. The measured render module is inside the ceiling, so leaving the render budget file unedited matches AC7.

The pin probes edit `version = "=30.0.1"` inside the table. `pins.table-form` reorders the keys and stays an accept probe. `docs/runbooks/guard-verification.md` row 7 matches the range probe. The catalogue note that the pin is unchanged is about that reorder. The same note says D-27 already writes the table.

The D-10 sentence "The pin is untouched" remains at `crates/ocelli-render/Cargo.toml:20`. The same comment then states the D-27 workspace change and says the version pin is still `=30.0.1`. That is the version pin, not a claim that the workspace entry was left as `wgpu = "=30.0.1"`.

The diff adds no `unsafe`. It adds no `wasm_bindgen` outside `ocelli-wasm`. No patient data is in the diff.

## Limitations and handoff

This pass did not run `wasm-pack`, `bin/ocelli.sh gate`, or `cargo check`. The size and export results are for the modules already in `pkg/` and `pkg-decode/`. The graph results are `cargo tree`, not a fresh link.

This pass did not delete the `allowed` key and did not remove `webgl` from a native line. The host half of `scripts/decode_graph_check.py` requires the native backends and `webgl`. That half passed. The wasm32 half passed by rejecting the native backends.

`check_decode_size` fails only above the ceiling, which is the same direction as the render budget. A large shrink would pass. Linking wgpu would grow the 16455 byte module, so that is the direction AC1 and AC6 care about.

No GPU adapter was opened. No pixel fixture applies. Cold start was not measured. Gate A4 stays open. The parent fills `**Reviewer id**:`.
