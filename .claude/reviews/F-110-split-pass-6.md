# F-110 review, pass 6

**Reviewed**: staged tree `60b1e7a8a8d014a1ddffb6c7ff13288869045021` on `sprint/s40`, HEAD `e17e73ea25b40bca46bce9cce7be81ed8c25d94e`. `git diff --cached` only. The worktree matched the index when `git write-tree` was read. 37 staged paths. This review file was not in that tree.
**Author / reviewer**: staged implementation / 01a1a0a6-2e09-76b3-bc6a-2f20c9cdca4d. Independent of the staged author. The parent inserted the reviewer id.
**Result**: 0 defects, 0 smells, 0 nitpicks

## Acceptance challenged

The claim under test was the new sentence in the F-110 notes of `docs/sprints/AS_BUILT.md`. Pass 5 left `crates/ocelli-wasm/src/canvas.rs` saying decode workers load that module. This tree's module comment says the decode-worker build does not compile this module.

The discriminating case was that comment beside the `render-worker` cfg in `crates/ocelli-wasm/src/lib.rs`, the empty `decode-worker` feature, and the `wasm-decode` flags. A current claim that decode workers load the render module or the canvas module would falsify the sentence. Tracked Rust, TypeScript, and living LLD were searched for that claim.

The search rejects the old contract as a current statement. `.claude/plans/F-039-design.md` still records that story's one-module decision. No living LLD and no current module comment states that decision as today's contract.

The pass 4 closures were checked again on this tree. Seven worded pin sites have the seven named probes. The two `extend` sites have no words. `docs/lld/build-targets.md` and `docs/lld/gpu-ownership.md` say the decode artefact does not export the canvas.

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|--------------------------|---------------------------|------------------------------|----------------------|
| `git write-tree`, `git diff --cached --name-only`, `git status --porcelain=v1` | index of this worktree | executed now | tree `60b1e7a8a8d014a1ddffb6c7ff13288869045021`. 37 staged paths. No unstaged paths. The pass 5 tree was `b2693c45f8c505a976343f91272481616bf8177f`. |
| Read of `canvas.rs` lines 1 to 6 and `lib.rs` lines 25 to 43 | staged sources | executed now | The module comment says the decode-worker build does not compile this module. `mod canvas` is `cfg(all(target_arch = "wasm32", feature = "render-worker"))`. `session` is `cfg(feature = "render-worker")`. |
| `crates/ocelli-wasm/Cargo.toml` features and `bin/ocelli.sh` `wasm-decode` | staged manifest and script | executed now | `decode-worker = []`. `default = ["render-worker"]`. `render-worker` enables `ocelli-render`, `ocelli-viewport`, and `wgpu`. `wasm-decode` passes `--no-default-features --features decode-worker`. |
| `git grep --cached` for the old load sentence | tracked Rust, TypeScript, and Markdown | executed now | No Rust or TypeScript hit. Living LLD hits are the two negative sentences. AS_BUILT names the old sentence as the pass 5 finding. The F-039 plan and earlier review files keep the historical wording. |
| `discover._sites_in` on HEAD and on `:scripts/pin_and_size_check.py` | the scanner the census uses | executed now | HEAD 48. Index 57. Nine new sites. Seven have words. Two are `check_decode_size() #1` at line 419 and `check_size() #1` at line 491. |
| `python3 scripts/guard_probe.py --only pins.decode-module-absent --only pins.decode-bindings-absent` | staged catalogue and `scripts/pin_and_size_check.py` | executed now | exit 0. Both probes red for their declared reason. One control green. 2.6s. |
| Decode and render `.d.ts` export lines | on-disk generated bindings, not tracked | executed now | Decode exports `install_panic_hook`, `ocelli_version`, `panic_record_len`, `panic_record_ptr`, and `initSync`. No `attachOffscreenCanvas` and no `resizeOffscreenCanvas`. Render exports both canvas names. |
| Feature split, D-27 tables, size files, and the five earlier pin probes | pass 4 tree `99d92070a5631d2582414c8ec07f3d538534f50e` | prior receipt | Not rebuilt here. This pass did not start cargo, rustc, wasm-pack, or a floor gate. |

## Defects

None.

## Smells

None.

## Nitpicks

None.

## Verified clean

Pass 5 D1 is closed. `crates/ocelli-wasm/src/canvas.rs` does not say decode workers load that module. The comment says the decode-worker build does not compile this module. `lib.rs` compiles `canvas` only for wasm32 with `render-worker`, and compiles `session` only with `render-worker`. `decode-worker` enables nothing. `ocelli-render` stays optional behind `render-worker`. The `wasm-decode` arm turns default features off and selects only `decode-worker`. That is a reading of the manifest and the script. It is not a cargo resolve.

The new AS_BUILT sentence matches that comment. It records pass 5 and reviewer `01a1a09c-329c-7352-b07d-8f719c84c0b9`, says the canvas module comment still said decode workers load that module, and says this tree says the decode-worker build does not compile it. "It" is the canvas module. The comment that was read says the decode-worker build does not compile this module. The sentence does not add another mechanism.

No tracked Rust file, TypeScript file, or living LLD states the old load contract as current. `docs/lld/build-targets.md` says the render module still exports `attachOffscreenCanvas` and `resizeOffscreenCanvas`, the decode-worker artefact does not, and a decode worker does not load this module. `docs/lld/gpu-ownership.md` says the decode-worker artefact does not export either function and a decode worker does not load this module. The on-disk decode bindings match the export half of that claim. The render bindings still export both canvas functions.

`.claude/plans/F-039-design.md` still records that story's decision that decode workers load the one module. The staged pass 4 and pass 5 reviews name the old sentence as a finding. Those files are history. They are not a second current claim.

The pass 4 pin closure is still true on this tree. The seven worded new sites are `did not emit bindings`, `decode bindings do not export`, `decode bindings still name`, `no .wasm found under` with `wasm-decode`, the missing `ci/wasm-decode-size.json` record, `decode size tolerance`, and `decode wasm module is`. The seven probes are `pins.decode-bindings-absent`, `pins.decode-export-missing`, `pins.decode-names-canvas`, `pins.decode-module-absent`, `pins.decode-budget-absent`, `pins.decode-tolerance`, and `pins.decode-size-ceiling`. The two sites with no words are `problems.extend(check_decode_exports(...))` and `problems.extend(check_decode_size())`. The two new probes went red for `no .wasm found` and `did not emit bindings`. Their control stayed green. The five earlier pin probes were not rerun.

## Limitations and handoff

This pass did not run `cargo`, `rustc`, `wasm-pack`, `decode_graph_check.main`, or `bin/ocelli.sh gate --floor`. It did not remeasure the wasm bytes. The pass 4 receipt is the evidence for the feature split, the size files, and the five earlier pin probes. The two probe runs are guard evidence, not a link graph.

The pass 5 review file is staged and was not rewritten. `.claude/verify-ledger.json` has no row for `60b1e7a8a8d014a1ddffb6c7ff13288869045021`. The AS_BUILT verification paragraph is not a receipt for this tree. This pass is not floor evidence.
