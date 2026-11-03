# F-110 review, pass 5

**Reviewed**: staged tree `b2693c45f8c505a976343f91272481616bf8177f` on `sprint/s40`, HEAD `e17e73ea25b40bca46bce9cce7be81ed8c25d94e`. `git diff --cached` only. No unstaged paths. 35 staged paths. The worktree matches the index.
**Author / reviewer**: staged implementation / 01a1a09c-329c-7352-b07d-8f719c84c0b9
**Result**: 1 defect, 0 smells, 0 nitpicks

## Acceptance challenged

The claim under test was the rewritten tests sentence in `docs/sprints/AS_BUILT.md`. Seven named pin probes watch seven worded new sites. The other two new sites are `extend` calls with no words of their own.

The discriminating case was a site scan of `scripts/pin_and_size_check.py` at HEAD and at the index, then the two new probes with their output kept. HEAD has 48 pin sites. The index has 57. The nine new sites are seven worded refusals and two wordless `extend` calls. `pins.decode-module-absent` prints `no .wasm found under crates/ocelli-wasm/pkg-decode` after the render module has already been measured. `pins.decode-bindings-absent` prints `did not emit bindings` after both size lines. That rejects a probe that is red only because the render wasm is missing, and it rejects a count that treats the two `extend` calls as worded sites.

The second claim was that no staged or unstaged tracked doc still says decode workers load the render module. Both rewritten LLD sentences now say the decode artefact does not export the canvas and that a decode worker does not load the render module. The decode bindings match that export claim. The same old sentence is still the module comment in `crates/ocelli-wasm/src/canvas.rs`, which this diff does not edit.

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|--------------------------|---------------------------|------------------------------|----------------------|
| `git write-tree`, `git diff --cached --name-only`, `git status --porcelain=v1` | index of this worktree | executed now | tree `b2693c45f8c505a976343f91272481616bf8177f`. 35 staged paths. No unstaged paths. Remediation from pass 4 tree `99d92070a5631d2582414c8ec07f3d538534f50e` is the design-plan row, the README row, the two LLD sentences, the generated probe table, the AS_BUILT paragraph, and the two probes. |
| `discover._sites_in` on HEAD and on `:scripts/pin_and_size_check.py` | the scanner the census uses | executed now | HEAD 48. Index 57. Nine new sites. Seven have words. Two are `check_decode_size() #1` at line 419 and `check_size() #1` at line 491. Index text equals the worktree. |
| `python3 scripts/guard_probe.py --only pins.decode-module-absent --only pins.decode-bindings-absent` | staged catalogue and `scripts/pin_and_size_check.py` | executed now | exit 0. Both probes red. One control green. 2.4s. |
| Same two mutators run again, output kept | sandbox copy of tracked files | executed now | Control exit 0 with both size lines and no refusal. Module probe exit 1 after the render size line, then `no .wasm found under crates/ocelli-wasm/pkg-decode. Run bin/ocelli.sh wasm-decode first.` Bindings probe exit 1 after both size lines, then `did not emit bindings`. |
| `python3 scripts/guard_census.py` | staged catalogue, budget, and runbook | executed now | exit 0. `OK: 928 refusal(s) in 75 file(s), all claimed`. Second line carries 470 probes. Uncovered count 0. A direct probe count is 470 and includes the seven `pins.decode-*` ids. Recorded `pins` is 57 and `decode-graph` is 26. |
| `python3 scripts/prose_check.py --staged` | staged prose | executed now | exit 0. 16 files. |
| Decode and render `.d.ts` export lines | on-disk generated bindings, not tracked | executed now | Decode exports `install_panic_hook`, `ocelli_version`, `panic_record_len`, `panic_record_ptr`, plus `initSync`. No `attachOffscreenCanvas` and no `resizeOffscreenCanvas`. Render exports both canvas names among twenty-four project exports before `initSync`. |
| `git grep` for the old load sentence | tracked markdown | executed now | Living LLD hits are the two rewritten negatives. AS_BUILT and the pass 4 review name the old sentence as the closed finding. The F-039 plan still records that story's one-module decision. |
| Feature split, D-27 tables, size files, eslint ignore, and the five earlier pin probes | pass 4 tree `99d92070a5631d2582414c8ec07f3d538534f50e`, reviewer `01a1a08c-8002-74c2-a187-e35b79977b56` | prior receipt | That pass reported those items clean and reported these two defects. This pass did not rebuild them and did not start cargo, rustc, or wasm-pack. |

## Defects

### D1. The canvas module still says decode workers load it

**Where**: `crates/ocelli-wasm/src/canvas.rs:4`. The staged gate is `crates/ocelli-wasm/src/lib.rs:34` and `crates/ocelli-wasm/Cargo.toml:83`.

**What**: The module comment says decode workers load this module and do not have to call either export. `canvas` is compiled only for `wasm32` with `render-worker`. `decode-worker` enables nothing. The decode artefact does not contain this file.

**Why it is wrong**: F-110's contract is that the decode build does not link `ocelli-render` and does not export the canvas. HLD section 24 says decode workers never touch the GPU. The comment states the pre-split contract. The same staged diff rewrote that contract in `lib.rs`, `docs/lld/build-targets.md`, and `docs/lld/gpu-ownership.md`, and left this copy.

**Evidence**: Read of the module comment beside the `cfg` and `decode-worker = []`. `git diff --cached --name-only` does not list `canvas.rs`. `git grep` of tracked markdown no longer states the old contract as current, except as a record of the pass 4 finding and as the F-039 plan's own historical decision.

**Scope and regression**: Present on HEAD and on this staged tree. The build split can be correct while this comment stays false. The smallest change is to say that the decode-worker build does not compile this module. No pixel path depends on the sentence.

## Smells

None.

## Nitpicks

None.

## Verified clean

Pass 4's missing-module defect is closed. The index has nine new pin sites over HEAD's 48. The worded seven are the missing bindings return, `decode bindings do not export`, `decode bindings still name`, `no .wasm found under` with `wasm-decode`, the missing decode budget, the decode tolerance, and the decode ceiling. `pins.decode-module-absent` expects `no .wasm found` and deletes `pkg-decode/ocelli_wasm_bg.wasm`. `pins.decode-bindings-absent` expects `did not emit bindings` and deletes `pkg-decode/ocelli_wasm.d.ts`. Both went red on those decode messages while the render module stayed present. The control for `pin_and_size_check.py --with-size` exited 0. The other two new sites are `problems.extend(check_decode_exports(...))` and `problems.extend(check_decode_size())`. Their identities are the wordless fallbacks. The five earlier decode pin probes were red for their expects on the pass 4 tree. This pass did not rerun them. The runbook rows for the two new probes carry the same expects. Census exit 0 means that generated table matches the catalogue.

Pass 4's LLD defect is closed in the two files it named. `docs/lld/build-targets.md` says the render module still exports `attachOffscreenCanvas` and `resizeOffscreenCanvas`, the decode-worker artefact does not, and a decode worker does not load that module. `docs/lld/gpu-ownership.md` says the same of both functions. The on-disk decode bindings have the four panic exports and do not name `attachOffscreenCanvas`. The render bindings still export both canvas functions. No other staged doc states the old load contract as the current one. The AS_BUILT note names that old sentence as the pass 4 finding and then says both LLD files now deny the canvas export.

`decode-worker` remains an empty feature. The recorded pin count is 57 and the recorded `decode-graph` count is 26, both still equal under this census. The probe total on the OK block is 470, and that total includes the two new probes. Prose check over the staged set exited 0. Pass 4's receipt still covers the feature split, the D-27 tables, the size files, and the eslint ignore. Those files were not part of the remediation diff.

## Limitations and handoff

This pass did not run `cargo`, `rustc`, `wasm-pack`, `decode_graph_check.main`, or `bin/ocelli.sh gate --floor`. It did not remeasure the wasm bytes. The pass 4 receipt is the evidence for that split and for the five earlier pin probes.

`crates/ocelli-wasm/src/canvas.rs` is the remaining copy of the old sentence and is the defect above. `.claude/plans/F-039-design.md` still records that story's decision that decode workers load the one module. That plan is the historical design, not the living LLD, and this pass does not treat it as a second current claim.

`.claude/verify-ledger.json` was not recorded for `b2693c45f8c505a976343f91272481616bf8177f`. The AS_BUILT verification paragraph is not a receipt for this tree. Remediation of the canvas comment belongs to a different session. Re-review that comment. Do not treat this pass as floor evidence.
