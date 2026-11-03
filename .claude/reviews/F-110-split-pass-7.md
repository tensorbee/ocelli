# F-110 review, pass 7

**Reviewed**: staged tree `c836d306b1c1eabeea782f368b747f714f748eb7` on `sprint/s40`, HEAD `e17e73ea25b40bca46bce9cce7be81ed8c25d94e`. `git diff --cached` only. `git write-tree` was read again immediately before this file was written and still equals that hash. The worktree had no unstaged tracked diff. 38 staged paths. The delta from pass 6 tree `60b1e7a8a8d014a1ddffb6c7ff13288869045021` is `.claude/reviews/F-110-split-pass-6.md` and one sentence in the F-110 notes of `docs/sprints/AS_BUILT.md`.
**Author / reviewer**: staged implementation / 01a1a0ac-0c28-70e1-b186-9c394f5970ea. Independent of the staged author. The parent inserted the reviewer id.
**Result**: 0 defects, 0 smells, 0 nitpicks

## Acceptance challenged

The new claim is the F-110 sentence in `docs/sprints/AS_BUILT.md`. It says pass 6 reported 0 defects, 0 smells and 0 nitpicks, and names reviewer `01a1a0a6-2e09-76b3-bc6a-2f20c9cdca4d`.

The discriminating case is the staged pass 6 header. A different count, or a different reviewer id, would make that sentence false. The same pass requires that `crates/ocelli-wasm/src/canvas.rs` still says the decode-worker build does not compile this module, and that `mod canvas` in `crates/ocelli-wasm/src/lib.rs` stays gated on `render-worker`. A comment that says decode workers load the module, or a `mod canvas` with no `render-worker` gate, would be a new false claim beside the sentence.

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|--------------------------|---------------------------|------------------------------|----------------------|
| `git write-tree`, `git rev-parse HEAD`, `git diff --cached --name-only`, `git status --porcelain=v1` | index of this worktree | executed now | tree `c836d306b1c1eabeea782f368b747f714f748eb7`. HEAD `e17e73ea25b40bca46bce9cce7be81ed8c25d94e`. 38 staged paths. No unstaged tracked paths. |
| `git diff` of `60b1e7a8a8d014a1ddffb6c7ff13288869045021` against this index | pass 6 tree against the staged tree | executed now | Two paths. The pass 6 review is new. AS_BUILT changes one sentence. |
| Staged pass 6 header | `.claude/reviews/F-110-split-pass-6.md` lines 3 to 5 | executed now | Result line is 0 defects, 0 smells, 0 nitpicks. Author line names reviewer `01a1a0a6-2e09-76b3-bc6a-2f20c9cdca4d` and says the parent inserted that id. |
| F-110 notes | `docs/sprints/AS_BUILT.md` line 4637 | executed now | The new sentence says pass 6 reported 0 defects, 0 smells and 0 nitpicks, with that reviewer id. The counts and the id match the header. |
| `git diff --name-only` from HEAD to `60b1e7a8a8d014a1ddffb6c7ff13288869045021`, and `git cat-file -e` | pass 6 tree | executed now | 37 paths. The pass 6 review file is not in that tree. That matches the review's own reviewed line. |
| `canvas.rs` lines 1 to 6 and `lib.rs` lines 25 to 42 | staged sources, worktree matches the index | executed now | The module comment says the decode-worker build does not compile this module. `mod canvas` is `cfg(all(target_arch = "wasm32", feature = "render-worker"))`. `session` is `cfg(feature = "render-worker")`. |
| `ocelli-wasm` features and the `wasm-decode` arm | staged manifest and `bin/ocelli.sh`, read only | executed now | `decode-worker = []`. `default = ["render-worker"]`. `wasm-decode` passes `--no-default-features --features decode-worker`. Not a cargo resolve. |
| Search for the old load sentence | staged Rust, TypeScript, and living LLD | executed now | No current statement that decode workers load the canvas module. `docs/lld/build-targets.md` and `docs/lld/gpu-ownership.md` say a decode worker does not load this module. AS_BUILT names the old wording as the pass 5 finding. |

## Defects

None.

## Smells

None.

## Nitpicks

None.

## Verified clean

The new AS_BUILT sentence is true of the staged pass 6 file. The result line is 0 defects, 0 smells and 0 nitpicks. The author line carries reviewer `01a1a0a6-2e09-76b3-bc6a-2f20c9cdca4d`. The file says the parent inserted that id. The sentence does not add a build measurement or a different mechanism.

`crates/ocelli-wasm/src/canvas.rs` still says the decode-worker build does not compile this module. `crates/ocelli-wasm/src/lib.rs` compiles `canvas` only for wasm32 with `render-worker`. `session` is compiled only with `render-worker`. `decode-worker` enables nothing. The `wasm-decode` arm turns default features off and selects only `decode-worker`. That is a reading of the manifest and the script.

The two living LLD sentences remain negative. They say the decode artefact does not export the canvas functions and that a decode worker does not load this module. No new sentence in the delta states the old load contract.

The pass 6 review's claim that its own file was absent from tree `60b1e7a8a8d014a1ddffb6c7ff13288869045021`, and that the tree had 37 staged paths, matches `git diff --name-only` against HEAD and `git cat-file`. Other `Pass 6 reported` lines in AS_BUILT belong to earlier stories and are not in this delta.

## Limitations and handoff

This pass did not run `cargo`, `rustc`, `wasm-pack`, or `bin/ocelli.sh`. It did not rebuild the decode module and did not rerun the pin probes. The pass 6 receipt remains the evidence for the feature split, the size files, and the probe runs on tree `60b1e7a8a8d014a1ddffb6c7ff13288869045021`. This pass is not floor evidence and not a sprint-profile record.

`.claude/plans/F-039-design.md` and the earlier F-110 review files were not used as the current contract.
