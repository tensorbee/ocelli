# F-110 review, pass 4

**Reviewed**: staged tree `99d92070a5631d2582414c8ec07f3d538534f50e` on `sprint/s40`, HEAD `e17e73ea25b40bca46bce9cce7be81ed8c25d94e`. `git diff --cached` only. No unstaged or untracked paths.
**Author / reviewer**: staged implementation / 01a1a08c-8002-74c2-a187-e35b79977b56
**Result**: 2 defects, 0 smells, 0 nitpicks

## Acceptance challenged

The claim under test was that the five new pin probes watch every added pin refusal, including a missing decode module. They do not. `pins.decode-budget-absent` watches a missing `ci/wasm-decode-size.json`. No probe deletes the decode wasm or the decode bindings, and no `expect` is the refusal those paths print.

The split itself holds for the manifests this pass could execute without the compiler. `decode-worker` enables nothing. The wasm32 wgpu tables that `check_trimmed` reads are exactly `std`, `parking_lot`, `wgsl`, `webgpu`, and `webgl`. None of those tables names `dx12`, `metal`, `gles`, `vulkan`, `glsl`, or `spirv`. The render budget file is not in the diff. The on-disk render module is inside the old ceiling. The decode record matches the on-disk decode module at the same 5 percent tolerance. The decode bindings export the four panic-surface names and do not contain `attachOffscreenCanvas`.

What that does not establish is the later LLD sentence that decode workers load the render module. The same edited file says the decode build is a different artefact.

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|--------------------------|---------------------------|------------------------------|----------------------|
| `git write-tree` and `git diff --cached --name-only` | index of this worktree | executed now | tree `99d92070a5631d2582414c8ec07f3d538534f50e`. 33 staged paths. Worktree matches the index. |
| `decode_graph_check.manifests()` | staged `scripts/decode_graph_check.py` and the wgpu manifests | executed now | `[]`. No cargo. This does not run `graphs()` or `workspace()`. |
| `discover._sites_in` on HEAD and the index copy of `scripts/pin_and_size_check.py` | same scanner the census uses | executed now | HEAD 48 sites. Index 57 sites. Nine new identities. |
| `discover._sites_in` on `scripts/decode_graph_check.py` | staged script | executed now | 26 sites, matching `ci/guard-probe-budget.json`. |
| `python3 scripts/guard_census.py` | staged catalogue and budget | executed now | exit 0. Uncovered count 0. |
| `python3 scripts/guard_probe.py` with ten `--only` ids | the five `pins.decode-*` probes and the five `decode-graph.*` probes | executed now | exit 0. 10 red for the declared `expect`. 6 controls green. 3.8s. |
| `check_size(False)` and `check_decode_size()` | `crates/ocelli-wasm/pkg/ocelli_wasm_bg.wasm` and `pkg-decode/ocelli_wasm_bg.wasm` | executed now | render 2,171,133 against baseline 2,151,196, ceiling 2,258,755. Decode 16,455 against baseline 16,455, ceiling 17,277. Both returned `[]`. |
| Missing decode directory under the repo root, then a wasm with no `.d.ts` | in-process assignment of `DECODE_PKG`, directory removed afterwards | executed now | `no .wasm found under crates/ocelli-wasm/pkg-decode-not-built`. Then `did not emit bindings`. Neither string is a probe `expect`. |
| `python3 -B -m unittest discover -s scripts/tests -p test_pin_and_size_check.py` | staged unit file | executed now | 17 tests, exit 0. The printed size OK line is the mocked `main(["--with-size"])` case, not a second size measurement. |
| `python3 scripts/prose_check.py --staged` | staged prose | executed now | exit 0. 14 files. |
| `npm run lint` | `eslint.config.js` with `pkg-decode/` present on disk | executed now | exit 0. |
| `python3 scripts/decode_graph_check.py` and `bin/ocelli.sh gate --floor` | full graph and floor | not run | Author receipt from before this pass said the graph check exited 0. This pass did not start cargo, rustc, or wasm-pack. |

## Defects

### D1. The five pin probes do not watch the missing decode module

**Where**: `docs/sprints/AS_BUILT.md:4630`. Refusals at `scripts/pin_and_size_check.py:400` and `scripts/pin_and_size_check.py:380`. Probe expects at `scripts/guards/catalogue.py:6986`.

**What**: The entry says the pin catch-all moved from 48 sites to 57, names the missing decode module among the added sites, and says `pins.decode-size-ceiling`, `pins.decode-export-missing`, `pins.decode-names-canvas`, `pins.decode-budget-absent`, and `pins.decode-tolerance` watch them. The 48 and 57 counts are true. The five probes do not watch the missing module. They also do not watch a decode wasm that has no bindings file.

**Why it is wrong**: A catch-all count records that a refusal exists. It does not drive that refusal red. The sentence says the probes watch the missing module. None of them builds that state, and none of their `expect` strings is the message the branch prints. The same gap covers the bindings-absent return, which is another of the nine new sites and is not one of the five probed messages. `problems.extend(check_decode_exports(...))` and `problems.extend(check_decode_size())` are the other two new identities. They have no words of their own. The five named groups are not the nine sites the count moved by.

**Evidence**: Site diff against HEAD produced these nine new messages. The worded ones are the ceiling, the missing budget, the tolerance, `no .wasm found under`, `did not emit bindings`, `do not export`, and `still name`. Pointing `DECODE_PKG` at `crates/ocelli-wasm/pkg-decode-not-built` returned `no .wasm found under crates/ocelli-wasm/pkg-decode-not-built. Run bin/ocelli.sh wasm-decode first.` A directory that held only a 1000 byte wasm returned `did not emit bindings`. The ten-probe run was red only for the five declared pin expects: `decode wasm module is`, `do not export ocelli_version`, `still name attachOffscreenCanvas`, `wasm-decode-size.json is absent`, and `decode size tolerance`.

**Scope and regression**: Present on this staged tree. The real `pkg-decode` tree is healthy, so the gate path does not hit these branches today. The smallest regression is one probe that omits the decode wasm and expects `no .wasm found`, and one that writes the wasm without `ocelli_wasm.d.ts` and expects `did not emit bindings`. Narrowing the sentence without those probes leaves the branches unwatched. Deleting either `return` drops the recorded count. Changing the condition to `if False` keeps the count and keeps every current probe green.

### D2. The LLD still says decode workers load the render module

**Where**: `docs/lld/build-targets.md:332`. Same sentence at `docs/lld/gpu-ownership.md:367`, which this diff does not edit.

**What**: The export section says decode workers load the same module and are not required to call the two render-target exports. That module, in the sentences above it, is the one with twenty-four exports, including `attachOffscreenCanvas`. The pipeline section of the same file says `bin/ocelli.sh wasm-decode` writes a different module, and that `canvas` and `session` compile only with `render-worker`.

**Why it is wrong**: F-110's acceptance is that the decode build does not link `wgpu`, Naga, or `ocelli-render`, and that its bindings do not name `attachOffscreenCanvas`. The generated decode `.d.ts` on disk has the four panic-surface exports and `initSync`. It does not have `attachOffscreenCanvas`. A reader of the export section is told the opposite arrangement from the pipeline section. `docs/lld/gpu-ownership.md` repeats the old sentence, so correcting only the edited file leaves the claim in the LLD.

**Evidence**: Read of both sections. `rg "export function" crates/ocelli-wasm/pkg-decode/ocelli_wasm.d.ts` lists `install_panic_hook`, `ocelli_version`, `panic_record_len`, `panic_record_ptr`, and `initSync`. The render `.d.ts` lists `attachOffscreenCanvas` among twenty-four project exports plus `initSync`. No shell loader in the diff points a decode worker at `pkg-decode`. The sentence is not saved by that absence. It states the pre-split contract as the current one.

**Scope and regression**: Present on this staged tree. The build split can be correct while this sentence stays false. The smallest change is to say that the decode-worker artefact does not export the canvas, and that the render module still does. Update both files. No pixel path depends on the sentence.

## Smells

None.

## Nitpicks

None.

## Verified clean

`decode-worker` is an empty feature list. `default` is `render-worker` only. `render-worker` enables `ocelli-render`, `ocelli-viewport`, and `wgpu`. `tier-b-probe` enables `render-worker`. `canvas` is `wasm32` and `render-worker`. `session` is `render-worker`. `manifests()` accepts that shape, the wasm-opt flag list, and the wasm32 tables of `ocelli-wasm`, `ocelli-render`, and `ocelli-compute`.

Every staged wgpu line was read. wasm32 tables request `default-features = false` and `std`, `parking_lot`, `wgsl`, `webgpu`, `webgl`. Native tables name `std`, `parking_lot`, `dx12`, `metal`, `gles`, `vulkan`, `wgsl`, and `webgpu`. `ocelli-render`'s native table also names `webgl`. `tools/oracle` and the `ocelli-native` dev-dependency are target-gated. The workspace pin is `{ version = "=30.0.1", default-features = false }`. `ci/wasm-size-budget.json` is not in the diff and still records 2,151,196 bytes at tolerance 0.05. `ci/wasm-decode-size.json` records 16,455 bytes at tolerance 0.05. `check_size(False)` agreed with both files and with the modules on disk.

The decode `.d.ts` exports `install_panic_hook`, `ocelli_version`, `panic_record_len`, and `panic_record_ptr`, and does not contain `attachOffscreenCanvas`. The five pin probes and five decode-graph probes go red for those exact `expect` strings, and their six controls stay green. Census exit 0. The pin catch-all is 57 and `decode-graph` is 26, both equal to the budget and to a fresh site scan. The four new constants are `DECODE_EXPORTS`, `DECODE_ABSENT`, `WASM_FEATURES`, and `NATIVE_FEATURES`. `constants_count` is 39.

`docs/lld/guards.md` matches the catalogue. `decode-graph` claims `scripts/decode_graph_check.py` with `*`, gate `wasm`. The pin prose matches the ceiling refusal and the canvas-name refusal. `docs/lld/build-targets.md` says eslint ignores `**/pkg/**` and `**/pkg-decode/**`. `eslint.config.js` has that ignore beside `**/pkg/**`, in the global ignores block. `npm run lint` exited 0 with generated `pkg-decode` glue present. No tracked path is under `pkg-decode`. The pattern does not match `packages/`.

`scripts/target_feature_check.py` reads `allowed` and does not read `checked_dependencies`. The `--write` path overwrites that array from `[workspace.dependencies]`. `docs/lld/build-targets.md` says it is not the watch list. D-27 is the deviation row. D-14 still says why `webgl` is named. The version text stays `=30.0.1`. AS_BUILT marks tiers A, B, and C n/a for the decode module and says the render module keeps `webgl`. The tracker row says the duration was not measured. It does not say a feature gate passed. `CHANGELOG.md` does not say this story publishes. The diff adds no bench subject and no pixel arithmetic. The Rust edit is the feature `cfg` on `canvas` and `session`.

The hand-written probe rows above the generated markers in `docs/runbooks/guard-verification.md` were left as they were. The generated table includes the ten new probes and their `expect` strings. Those strings match the refusals the probe run printed.

## Limitations and handoff

This pass did not run `cargo`, `rustc`, `wasm-pack`, `decode_graph_check.main`, or `bin/ocelli.sh gate --floor`. `manifests()` does not prove the cargo-tree classifiers against a live resolve. The author reported that resolve green on this tree before the pass. `classify_workspace` refuses a wasm32 feature only when it is `dx12`, `metal`, `gles`, `vulkan`, `glsl`, or `spirv`, or when one of the five D-27 names is missing. `check_trimmed` locks the exact set for three crates and does not load `tools/oracle/Cargo.toml`. That oracle wasm32 table matches the set today.

`.claude/verify-ledger.json` has no row for `99d92070a5631d2582414c8ec07f3d538534f50e`. The AS_BUILT verification paragraph is not a receipt for this tree. The tracker does not claim the gate passed. The notes already say a later sprint-profile record is required and that closing S40 does not publish.

Remediation belongs to a different session. Re-review the changed prose and any new probes. Do not treat this pass as floor evidence.
