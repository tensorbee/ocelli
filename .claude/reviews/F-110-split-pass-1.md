# F-110 review, pass 1

**Reviewed**: working tree on sprint/s40 at HEAD e17e73e, plus untracked `scripts/decode_graph_check.py` and `ci/wasm-decode-size.json`
**Author / reviewer**: Author is the parent session. Reviewer is an independent subagent and did not write this diff.
**Reviewer id**: 01a1a052-d060-7eb0-885e-4b79e8a192c4
**Result**: 2 defects, 0 smells, 0 nitpicks

## Acceptance challenged

AC1. The decode feature must not link wgpu, naga, or ocelli-render. `decode-worker = ["dep:wgpu"]` makes `scripts/decode_graph_check.py` exit 1 before it runs `cargo tree`. Restoring `decode-worker = []` makes the same script exit 0. A separate `cargo tree -p ocelli-wasm --target wasm32-unknown-unknown --no-default-features --features decode-worker` prints no wgpu, naga, or ocelli-render row. The built `pkg-decode` module is 16455 bytes and does not contain those names.

AC2. The default wasm32 graph must still link wgpu. `cargo tree -p ocelli-wasm --target wasm32-unknown-unknown` resolves `wgpu v30.0.1` with `parking_lot`, `std`, `web`, `webgl`, `webgpu`, `wgpu-core`, and `wgsl`. The render module contains the byte strings `wgpu` and `naga`. That rejects a shipping module that dropped the renderer.

AC3 and AC4. The graph script's manifest half passed, so wasm-opt is a non-empty flag list and the wasm32 feature lists on `ocelli-render`, `ocelli-wasm`, and `ocelli-compute` are exactly `std`, `parking_lot`, `wgsl`, `webgpu`, and `webgl`. The package graph above does not resolve `dx12`, `metal`, `gles`, `vulkan`, `glsl`, or `spirv`. `web` and `wgpu-core` are features wgpu turns on from `webgl` and `webgpu`. They are not extra frontends.

AC5. Native `ocelli-render` names the default backends plus `webgl`. Native `ocelli-wasm` and `ocelli-compute` name the default backends and do not name `webgl`. The graph script checks that superset and passed.

AC6 and AC7. `--with-size` reports the render module at 2,171,133 bytes against baseline 2,151,196 and ceiling 2,258,755. It reports the decode module at 16,455 bytes against baseline 16,455 and ceiling 17,277. `ci/wasm-size-budget.json` is not in the diff. Its tolerance is still 0.05. The decode record uses the same 0.05.

AC8. `crates/ocelli-wasm/pkg-decode/ocelli_wasm.d.ts` exports `install_panic_hook`, `ocelli_version`, `panic_record_len`, and `panic_record_ptr`. It does not name `attachOffscreenCanvas`. The render bindings do. The unit tests accept a panic-surface fixture and refuse a canvas name and a missing `ocelli_version`. The suite passed.

The challenged claim that does not hold is the new step 4 sentence that the wasm32 and native wgpu feature sets differ. With the allow entry removed, `scripts/target_feature_check.py` still reports one identical resolve.

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|--------------------------|---------------------------|------------------------------|----------------------|
| `python3 scripts/decode_graph_check.py` | working tree, `decode-worker = []` | executed now | exit 0. Decode graph has no wgpu or naga. Render graph keeps wgpu. wasm-opt stays on. |
| same script after `decode-worker = ["dep:wgpu"]` | temporary edit of `crates/ocelli-wasm/Cargo.toml` only | executed now, then restored | exit 1. `FAIL: decode graph` and `decode-worker must not enable a dependency`. Manifest check returns before `cargo tree`. |
| same script after restore | `decode-worker = []` again | executed now | exit 0. Same OK line as the first run. |
| `python3 scripts/pin_and_size_check.py --with-size` | render `pkg/ocelli_wasm_bg.wasm` 2171133 bytes, decode `pkg-decode/ocelli_wasm_bg.wasm` 16455 bytes, both size JSON files | executed now | exit 0. Render ceiling 2258755. Decode ceiling 17277. |
| `python3 -B -m unittest discover -s scripts/tests -p test_pin_and_size_check.py` | `DecodeExports` plus the existing pin tests | executed now | exit 0. 17 tests. |
| `python3 scripts/target_feature_check.py` | `ci/target-feature-baseline.json` with the wgpu allow entry | executed now | exit 0. 20 dependencies, 1 declared exception. |
| same script with `allowed` set to `{}` | temporary edit of the baseline only | executed now, then restored | exit 0. Identical features on `aarch64-apple-darwin` and `wasm32-unknown-unknown`. No exception needed. |
| `python3 scripts/deviation_check.py` | `docs/hld/DEVIATIONS.md` and citations | executed now | exit 0. 27 deviations. D-01 matches `Cargo.toml`. |
| `cargo tree -p ocelli-wasm --target wasm32-unknown-unknown -e normal` | default features, package graph only | executed now | `wgpu v30.0.1` features `parking_lot,std,web,webgl,webgpu,wgpu-core,wgsl`. |
| decode `cargo tree -p ocelli-wasm` with `--no-default-features --features decode-worker` | decode package graph | executed now | no wgpu, naga, or ocelli-render row. |
| workspace feature dump through `scripts/target_feature_check.py` | both targets, normal edges | executed now | both resolves are `dx12`, `gles`, `metal`, `parking_lot`, `std`, `vulkan`, `web`, `webgl`, `webgpu`, `wgpu-core`, `wgsl`. |

`git diff` after the two restores still shows only the story edit for `crates/ocelli-wasm/Cargo.toml` and `ci/target-feature-baseline.json`. `decode-worker = []` is the working line. The allow object has the key `wgpu`.

## Defects

### D1, build-targets still says the workspace pin is untouched

**Where**: `docs/lld/build-targets.md:399`
**What**: The current-state section quotes `wgpu = { workspace = true, features = ["webgl"] }` and says the pin and the workspace entry are untouched.
**Why it is wrong**: The file says it describes the code today. The workspace entry is `wgpu = { version = "=30.0.1", default-features = false }` at `Cargo.toml:73`. `ocelli-render` no longer has one shared `features = ["webgl"]` line. Native and wasm32 are separate target tables at `crates/ocelli-render/Cargo.toml:41` and `:44`. D-27 exists because that workspace switch is required. A current-state claim that the entry is untouched is false.
**Evidence**: Read `Cargo.toml:69-73`, `crates/ocelli-render/Cargo.toml:41-45`, and `docs/lld/build-targets.md:397-401`. No command can make the quoted line true.
**Scope and regression**: Readers of the build LLD, including the next size or tier change. Present on this working tree. The shipping graph is still the trimmed one, so this does not by itself put native backends into `pkg/`. The smallest fix is to replace the quoted line and the untouched sentence with the target tables and the workspace `default-features = false` entry. Do not revive the old sentence in `docs/lld/tier-resolution.md`, which this diff already rewrote.

### D2, step 4 text says the feature sets differ and that the allow list is empty

**Where**: `docs/lld/build-targets.md:175` and `docs/lld/build-targets.md:441`
**What**: The new paragraph says the wasm32 feature set and the native feature set differ on purpose. The known-gaps bullet says `allowed` in `ci/target-feature-baseline.json` is empty.
**Why it is wrong**: This LLD describes today, and this section is about `scripts/target_feature_check.py`. That check compares resolved features. With the wgpu allow entry removed it still exits 0 and reports identical features. The baseline's `allowed` object is not empty. It has one key, `wgpu`. The two sentences cannot both be true, and each is false on the point named here. The manifests of the three crates do differ by target. The resolve this check actually computes does not.
**Evidence**: `python3 scripts/target_feature_check.py` exits 0 with `1 declared exception(s)`. The same command after `allowed` is set to `{}` exits 0 with no exception and the same identical-resolve line. The dumped wgpu sets are equal and both include `dx12`, `gles`, `metal`, and `vulkan`. `tools/oracle/Cargo.toml:30` names those backends on a shared `[dependencies]` line, so a workspace resolve turns them on for wasm32 as well as for the host. `cargo tree -p ocelli-wasm` does not include the oracle, and that package graph does not resolve those four backends.
**Scope and regression**: Present on this working tree. The false sentence is new in this diff. The empty-allow sentence was left in a file this story edited. The decode and render artefacts measured above are unaffected. The smallest fix is to say that the package graph is trimmed, that the workspace resolve stays identical because the oracle line is not target-gated, and that `allowed` currently names `wgpu` even though the check passes without that entry.

## Smells

None.

## Nitpicks

None.

## Verified clean

The workspace pin version is still `=30.0.1`. Defaults are off at `Cargo.toml:73`, which is the approved Cargo constraint. `ocelli-render`, `ocelli-wasm`, and `ocelli-compute` keep wgpu in target tables. The wasm32 lists are the D-27 set. Native lists are a superset of the pinned default backends. Only `ocelli-render` adds native `webgl`. `glsl` and `spirv` are absent from those lists and from both resolves that were printed.

`ocelli-wasm` does not put wgpu in the shared `[dependencies]` table. The wasm32 line is optional and carries the browser set. The native line is optional and carries the default backends without `webgl`. `render-worker` enables `ocelli-render`, `ocelli-viewport`, and wgpu. `default` is only `render-worker`. `decode-worker` enables nothing. `tier-b-probe` enables `render-worker`.

`crates/ocelli-wasm/src/lib.rs:34` compiles `canvas` only for wasm32 and `render-worker`. `lib.rs:39` compiles `session` only with `render-worker`. Uses of `ocelli_render` and `ocelli_viewport` are inside those two modules. The decode package graph does not contain `ocelli-render`.

`bin/ocelli.sh` runs the graph check, then `wasm`, then `wasm-decode`, then `--with-size`. `wasm-decode` passes `--no-default-features --features decode-worker`. `pkg-decode/` is gitignored. The `pins` gate calls `pin_and_size_check.py` without `--with-size`.

`check_size` writes only `ci/wasm-size-budget.json`, and only when that file is absent or `--accept-size` is set. On `--accept-size` it returns before `check_decode_size`. `check_decode_size` reads `ci/wasm-decode-size.json` and never writes it. This pass did not pass `--accept-size`. The render budget file is unchanged. Both tolerances are 0.05.

The decode export check looks for `export function` on the four panic and version names and refuses the substring `attachOffscreenCanvas`. The three `DecodeExports` tests are in `scripts/tests/test_pin_and_size_check.py`. The real decode `.d.ts` matches the accept case. The render `.d.ts` has those four plus the canvas exports. Counting `export function` and leaving out generated `initSync` gives 24 render exports, which matches the tier LLD.

The allow entry's only key is `wgpu`. The note cites D-27. It does not name another crate. Clearing it did not reveal a hidden crate or a hidden feature delta. It is not what makes the current check pass.

D-27 in `docs/hld/DEVIATIONS.md` matches the workspace line and the three target feature lists. `docs/lld/tier-resolution.md` states that workspace line and does not say the entry is untouched. Its header F-IDs match the last cell of its row in `docs/lld/README.md`. The `build-targets.md` header matches its row too.

The diff adds no `unsafe`. It adds no `wasm_bindgen` outside `ocelli-wasm`. It does not touch the render tolerance. The measured render module stays inside the recorded ceiling, so leaving `ci/wasm-size-budget.json` unedited matches AC7. wasm-opt remains the existing non-empty flag list. No patient data is in the diff.

The D-10 sentence "The pin is untouched" remains at `crates/ocelli-render/Cargo.toml:20`. The same comment then states the D-27 workspace change and says the version pin is still `=30.0.1`. That is historical D-10 wording with a later correction in the same file, not a second current-state claim that the entry was left as `wgpu = "=30.0.1"`.

## Limitations and handoff

This pass did not run `wasm-pack`, `bin/ocelli.sh gate`, or `cargo check`. The size and export results are for the modules already in `pkg/` and `pkg-decode/`. The graph results are `cargo tree`, not a fresh link.

`tools/oracle/Cargo.toml:30` and the `ocelli-native` dev-dependency still name native backends outside a target table. Only the oracle line is in the normal workspace resolve. That is why step 4 sees native backends on wasm32. It is not filed as its own defect because `cargo tree -p ocelli-wasm` does not take that line, and the built decode module does not contain wgpu or naga. A fix for D2 should decide whether to retarget that oracle line or to stop claiming that step 4 observes a split.

The allow filter drops every future problem whose first word is `wgpu`. Nothing in this tree is currently hidden by that filter. This pass did not add a one-sided feature and rerun the check.

`check_decode_size` fails only above the ceiling, which is the same direction as the render budget. A large shrink would pass. Linking wgpu would grow the 16455 byte module, so that is the direction AC1 and AC6 care about.

No GPU adapter was opened. No pixel fixture applies. Cold start was not measured. Gate A4 stays open. The parent fills `**Reviewer id**:` and remediates D1 and D2 before the next independent pass.
