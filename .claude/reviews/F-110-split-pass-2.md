# F-110 review, pass 2

**Reviewed**: working tree on sprint/s40 at HEAD e17e73e, including untracked `scripts/decode_graph_check.py` and `ci/wasm-decode-size.json`. No files were staged.
**Author / reviewer**: Author is the parent session. Reviewer is a fresh independent subagent and did not write the diff or pass 1.
**Reviewer id**: 01a1a062-ff03-73d1-a69f-276601766b6b
**Result**: 2 defects, 0 smells, 0 nitpicks

## Acceptance challenged

AC1. The decode graph must not link wgpu, naga, or ocelli-render. `python3 scripts/decode_graph_check.py` exited 0. A separate decode `cargo tree` had no wgpu, naga, or ocelli-render row. `pkg-decode/ocelli_wasm_bg.wasm` is 16455 bytes and does not contain those names or `attachOffscreenCanvas`.

AC2. The default wasm32 graph must still link wgpu. The same graph check exited 0 only after it saw `wgpu v` on the default tree. The render package graph resolves `wgpu v30.0.1`. The render module contains the byte strings `wgpu` and `naga`.

AC4 and the workspace half of the new check. Adding `dx12` only to the oracle wasm32 feature list made `scripts/decode_graph_check.py` exit 1 with `wasm32 workspace resolve enables dx12`. Restoring that list made the script exit 0. The wasm32 list printed by `scripts/target_feature_check.py` with `allowed` set to `{}` has no `dx12`.

AC5. The three manifests name the D-27 sets in target tables. Native `ocelli-render` also names `webgl`. The graph script's manifest half passed on that tree. This pass did not delete `webgl` from the native render line.

AC6 and AC7. `--with-size` printed the render module at 2,171,133 bytes against baseline 2,151,196 and ceiling 2,258,755, and the decode module at 16,455 against baseline 16,455 and ceiling 17,277. `ci/wasm-size-budget.json` is not in the diff. Both recorded tolerances are 0.05. This pass did not pass `--accept-size`.

AC8. The decode `.d.ts` exports the four panic and version names and does not name `attachOffscreenCanvas`. The render `.d.ts` has those four plus the canvas exports. `DecodeExports` is in the 17 unit tests that passed.

The challenged claim that does not hold is the deviations row for D-14, which still quotes a wgpu line the manifests no longer have. A second claim, that `checked_dependencies` is the list step 4 watches, stays green when that list is emptied.

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|--------------------------|---------------------------|------------------------------|----------------------|
| `pgrep -lf 'cargo\|rustc\|wasm-pack'` | host processes before the checks | executed now | no match |
| `python3 scripts/decode_graph_check.py` | working tree after the oracle target split | executed now | exit 0. Decode graph has no wgpu or naga. Render graph keeps wgpu. wasm-opt stays on. |
| same script with `dx12` added only to the oracle wasm32 feature list | temporary edit of `tools/oracle/Cargo.toml` | executed now, then restored byte for byte | exit 1. `wasm32 workspace resolve enables dx12` |
| same script after that restore | original oracle bytes, `dx12` once, on the native line | executed now | exit 0 |
| `python3 scripts/target_feature_check.py` with `allowed` set to `{}` | temporary edit of `ci/target-feature-baseline.json` | executed now, then restored byte for byte | exit 1. Host list includes `dx12`, `gles`, `metal`, and `vulkan`. wasm32 list is `parking_lot`, `std`, `web`, `webgl`, `webgpu`, `wgpu-core`, `wgsl`. |
| same script after restore | `allowed` has one key, `wgpu` | executed now | exit 0. 20 dependencies, 1 declared exception. |
| same script with `checked_dependencies` set to `[]` | temporary edit, `allowed` left as `wgpu` | executed now, then restored byte for byte | exit 0. Same 20 and 1 exception line. |
| `python3 scripts/pin_and_size_check.py --with-size` | render module 2171133 bytes, decode module 16455 bytes | executed now | exit 0. Render ceiling 2258755. Decode ceiling 17277. |
| `python3 scripts/deviation_check.py` | `docs/hld/DEVIATIONS.md` | executed now | exit 0. 27 deviations. D-01 matches `Cargo.toml`. |
| `python3 -B -m unittest discover -s scripts/tests -p test_pin_and_size_check.py` | pin tests plus `DecodeExports` | executed now | exit 0. 17 tests. |
| decode and default `cargo tree -p ocelli-wasm --target wasm32-unknown-unknown -e normal` | package graphs, not the workspace resolve | executed now | decode tree has no wgpu, naga, or ocelli-render. Default wgpu features are `parking_lot,std,web,webgl,webgpu,wgpu-core,wgsl`. |

`git status` after the restores matches the story diff. `ci/wasm-size-budget.json` is not modified. `tools/oracle/Cargo.toml` still has one `dx12`, on the native line.

## Defects

Pass 1 D1 is closed. The D-14 section in `docs/lld/build-targets.md` now quotes the workspace entry as `{ version = "=30.0.1", default-features = false }`, which is `Cargo.toml:73`. It no longer says that entry is untouched.

Pass 1 D2 is closed. Step 4 and the known-gaps bullet no longer say that `allowed` is empty or that the two wgpu resolves are the same. Clearing `allowed` exits 1, and the wasm32 list from that run has no `dx12`. The oracle wgpu dependency is in target tables. Putting `dx12` on the wasm32 table makes the graph check fail.

### D1, D-14 still quotes a wgpu line the crate does not have

**Where**: `docs/hld/DEVIATIONS.md:31`
**What**: The We do cell still says `wgpu = { workspace = true, features = ["webgl"] }` in `ocelli-render`.
**Why it is wrong**: The register's own header says the We do column is what this repository does. That line is not in the tree. Native and wasm32 are separate target tables at `crates/ocelli-render/Cargo.toml:41` and `:45`. The workspace entry has `default-features = false`. D-27's We do cell matches those lines. D-14's cell does not. A reader who restores the quoted line is not restoring the current manifest.
**Evidence**: `rg` of `features = ["webgl"]` hits only that deviations row. Read of `crates/ocelli-render/Cargo.toml:41-45` and `Cargo.toml:73`. `python3 scripts/deviation_check.py` exits 0 because it does not compare this cell to the manifest.
**Scope and regression**: Present on this working tree. The shipping graph is still the trimmed one. The smallest fix is to replace that quoted line with the target tables, or to say in the same cell that D-27 replaced it. Do not put `features = ["webgl"]` back as the only feature.

### D2, the known-gaps bullet says an unread array is the watch list

**Where**: `docs/lld/build-targets.md:446`
**What**: The bullet says `checked_dependencies` is the list being watched, and points at a one-liner that prints that array's length.
**Why it is wrong**: This LLD describes today, and this bullet describes `scripts/target_feature_check.py`. The read path loads `allowed` and does not read `checked_dependencies`. The names it compares come from `[workspace.dependencies]`. The committed array has 9 entries and includes `dicom`, which is not a workspace key. The checker reported 20 shared names. The one-liner prints 9.
**Evidence**: `scripts/target_feature_check.py:171` reads only `allowed`. With `checked_dependencies` set to `[]` and `allowed` left in place, the checker still exited 0 and still reported 20 dependencies and 1 exception. Restoring the file and running it again exited 0 with the same line.
**Scope and regression**: Present on this working tree. The sentence was kept while this story rewrote the bullet. The live check is stricter than the sentence, so emptying the array did not hide the wgpu split. The smallest fix is to stop calling that array the watch list. Do not teach the checker to trust it. That would narrow the gate to a stale 9 names.

## Smells

None.

## Nitpicks

None.

## Verified clean

The workspace pin version is still `=30.0.1`, with defaults off at `Cargo.toml:73`. `ocelli-render`, `ocelli-wasm`, and `ocelli-compute` keep wgpu in target tables. The wasm32 lists are the D-27 set, with `default-features = false`. Native lists name `std`, `parking_lot`, `dx12`, `metal`, `gles`, `vulkan`, `wgsl`, and `webgpu`. Only `ocelli-render` adds native `webgl`. `ocelli-oracle` uses the same split. The `ocelli-native` dev-dependency is native-only and does not name `webgl`.

The workspace wasm32 resolve and the default `ocelli-wasm` package graph name the same wgpu features: `parking_lot`, `std`, `web`, `webgl`, `webgpu`, `wgpu-core`, and `wgsl`. The host resolve adds `dx12`, `gles`, `metal`, and `vulkan`. `web` and `wgpu-core` are activated by the `webgl` and `webgpu` features inside wgpu 30.0.1. They are on both targets. The step 4 sentence names the requested set and the four-backend delta, which is the difference the checker printed.

`glsl` and `spirv` are absent from both wgpu resolves. On the render package graph, naga shows `glsl-out` and `wgsl-in`. In the pinned wgpu manifest, the `glsl` feature enables `naga/glsl-in`, and that feature is not in the resolved wgpu set. `glsl-out` is not that frontend.

`decode-worker` enables nothing. `default` is `render-worker` only. `render-worker` enables `ocelli-render`, `ocelli-viewport`, and wgpu. `tier-b-probe` enables `render-worker`. `canvas` compiles only for wasm32 and `render-worker`. `session` compiles only with `render-worker`. `ocelli-compute` is not in the default wasm package graph. The comment that says so is true.

`bin/ocelli.sh` runs the graph check, then `wasm`, then `wasm-decode`, then `--with-size`. `wasm-decode` passes `--no-default-features --features decode-worker` and writes `pkg-decode/`. That directory is gitignored. The `pins` gate still calls `pin_and_size_check.py` without `--with-size`.

`check_size` writes `ci/wasm-size-budget.json` only when that file is absent or `--accept-size` is set. This pass did not use `--accept-size`. `check_decode_size` reads `ci/wasm-decode-size.json` and refuses a tolerance other than 0.05. Both artefacts were built on 2026-11-03. The recorded render baseline stays 2,151,196. The measured render module is inside the ceiling, so leaving the render budget file unedited matches AC7.

The render `.d.ts` has 25 `export function` lines. Leaving out generated `initSync` leaves 24, which matches the tier LLD. The decode `.d.ts` has the four required names plus `initSync`.

D-27 matches the workspace line and the three target feature lists. `docs/lld/tier-resolution.md` states that workspace line and does not say the entry is untouched. Its header F-IDs match the last cell of its row in `docs/lld/README.md`. The `build-targets.md` header matches its row too.

The diff adds no `unsafe`. It adds no `wasm_bindgen` outside `ocelli-wasm`. No patient data is in the diff.

The D-10 sentence "The pin is untouched" remains at `crates/ocelli-render/Cargo.toml:20`. The same comment then states the D-27 workspace change and says the version pin is still `=30.0.1`. That is the version pin, not a claim that the workspace entry was left as `wgpu = "=30.0.1"`.

## Limitations and handoff

This pass did not run `wasm-pack`, `bin/ocelli.sh gate`, or `cargo check`. The size and export results are for the modules already in `pkg/` and `pkg-decode/`. The graph results are `cargo tree`, not a fresh link.

This pass did not remove `webgl` or a native backend from every native line. Removing `webgl` from `ocelli-render` would fail the manifest half of `scripts/decode_graph_check.py` before `workspace()` runs. The host half of `workspace()` was read. The live host resolve contains every feature that half requires. The wasm32 half was executed by the `dx12` insertion.

`check_decode_size` fails only above the ceiling, which is the same direction as the render budget. A large shrink would pass. Linking wgpu would grow the 16455 byte module, so that is the direction AC1 and AC6 care about.

No GPU adapter was opened. No pixel fixture applies. Cold start was not measured. Gate A4 stays open. The parent fills `**Reviewer id**:` and remediates D1 and D2 before the next independent pass.
