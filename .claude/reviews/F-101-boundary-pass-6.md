# F-101 review, pass 6

**Reviewed**: staged tree `daf4dacda7e270e0df3165070b1bc34cdcfe9a4a`, confirmed by `git write-tree` at pin. The delta from `1d7a0fce852ec1a61ccb01f1c9acd8eafe57be4d` touches `docs/lld/build-targets.md`, `packages/core/src/boundary.wasm.test.ts`, the new `.claude/reviews/F-101-boundary-pass-5.md` and `.claude/reviews/F-X023-instruction-pass-4.md` (outside F-101). The index had no unstaged or untracked entries at pin. The tree moved after the pin when another agent staged four export-count paths, disclosed in the limitations.

**Author / reviewer**: Implementer recorded as Codex in `.claude/reviews/S16-codex-completion-evidence.md`. Reviewer is DeepSeek V4.1 Flash. I did not author the implementation, the remediation or the prior reviews. The remediation was authored by another agent. Independence is per changed material.

**Result**: 1 defect, 0 smells, 0 nitpicks

**Date**: 2026-10-26

## Previous findings

D1 (the `build-targets.md` export count) is closed. N1 (the boundary test's buffer identity assertion) is closed. S1, S2, N2, N3 and N4 from pass 4 remain closed. N5 (`StackViewport.error` has no setter) remains a handoff to F-103 and is not an F-101 defect.

## Acceptance challenged

**Export inventory.** The shipped release module was built with `wasm-pack build crates/ocelli-wasm --release --target web --out-dir target/release-count`. Its glue exports 21 functions. The dev build exports the same 21. The six rows in the panic probe table are `ocelli_version`, `install_panic_hook`, `panic_record_ptr`, `panic_record_len`, `attachOffscreenCanvas` and `resizeOffscreenCanvas`. The F-101 paragraph lists 15 exports. The union of the six and the fifteen has no missing name, no extra name and no duplicate, so the count of twenty-one and the six-table-row claim are both true. `build-targets.md` does not contradict itself elsewhere about the export count. The same count is still false in `benchmarks.md:169`, which says "The current release module exports six functions". That is the defect below.

**Copy-out across memory growth (AC6).** The test seeds a frame, copies the 96-byte state and the ring out, captures `memoryBefore` as the pre-growth `wasm.memory.buffer`, grows wasm memory by writing a 1 MiB frame, and asserts the memory grew. The new pre-growth assertion `expect(state.buffer).not.toBe(memoryBefore)` is the discriminator. A view returned before growth carries `memoryBefore` and detaches, so the assertion goes red. I confirmed this by mutating the built glue to drop the `.slice()` in `copy_state` and `drain_events`. The boundary suite failed on the growth test with a detached buffer, and the baseline passed 7 after restoring the glue byte-identical. The post-growth assertion `expect(state.buffer).not.toBe(wasm.memory.buffer)` can pass against a pre-growth view, because that view's buffer is the old `memoryBefore` and not the new buffer. It is redundant and not a discriminator, but it is harmless because the pre-growth assertion and the content equality both catch the view. The comment is true.

## Evidence

| Command or selected cases | Source and input identity | Executed now or prior receipt | Exit and observation |
|---|---|---|---|
| `git write-tree` at pin | index | executed now | `daf4dacda7e270e0df3165070b1bc34cdcfe9a4a` matched the supplied tree |
| `git status --porcelain` at pin | index | executed now | 185 staged entries, no unstaged or untracked entries |
| `wasm-pack build crates/ocelli-wasm --dev --target web --out-dir target/boundary-wasm` | dev module | executed now | exit 0 |
| `grep -c "^export function" crates/ocelli-wasm/target/boundary-wasm/ocelli_wasm.js` | dev glue | executed now | 21 |
| `wasm-pack build crates/ocelli-wasm --release --target web --out-dir target/release-count` | release module | executed now | exit 0 |
| `grep -c "^export function" crates/ocelli-wasm/target/release-count/ocelli_wasm.js` | release glue | executed now | 21 |
| `npx vitest run --config vitest.boundary.config.ts` | built dev module | executed now | exit 0, 7 passed |
| Glue mutation, remove `.slice()` from `copy_state` and `drain_events` | built dev glue | executed now | exit 1, the growth test failed with a detached buffer, glue restored with `cmp`, baseline 7 passed |
| `bin/ocelli.sh test ocelli-wasm --lib` | crate | executed now | exit 0, 31 passed |
| `bin/ocelli.sh test ocelli-viewport` | crate | executed now | exit 0, 47 passed across 8 suites |
| `npx vitest run packages/core/src/bulk.test.ts packages/core/src/ring.test.ts packages/core/src/commands.test.ts` | TS core | executed now | exit 0, 37 passed |
| `npm run lint` | package | executed now | exit 0 |
| `npm run typecheck` | package | executed now | exit 0 |
| `bin/ocelli.sh clippy ocelli-wasm` | crate | executed now | exit 0 |
| `bin/ocelli.sh gate unsafe` | staged tree | executed now | exit 0, 155 files checked, 2 permitted |
| `bin/ocelli.sh gate bindgen` | staged tree | executed now | exit 0 |
| `bin/ocelli.sh gate bench` | staged tree | executed now | exit 0, 127 tests, 125 passed, 2 skipped |
| `node --test tools/bench/tests/run_test.mjs` | bench node suite | executed now | exit 0, 14 passed, 1 skipped |
| `bin/ocelli.sh gate prose` | staged tree | executed now | exit 0, 470 files |
| `bin/ocelli.sh gate content` | staged tree | executed now | exit 0 |
| `git diff 1d7a0fce daf4dacd -- ci/ vitest.config.ts vitest.boundary.config.ts package.json` | config and baselines | executed now | empty |
| `rg -n "six functions" docs/ tools/` | docs and bench | executed now | five locations, `benchmarks.md:169`, `tier-resolution.md:453`, `subjects.json:32`, `wasm_cold_start.mjs:12` and `:337` |
| `git write-tree` after the concurrent change | index | executed now | `d5bc10584e8c588c5f89eba2bc5a9f950e1977ac`, four paths changed after the pin |
| Prior receipt, pass 5 | tree `1d7a0fce` | prior receipt | 1 defect, 1 nitpick |

## Defects

### D1, the release-module export count is still six outside `build-targets.md`

**Where**: `docs/lld/benchmarks.md:169` ("The current release module exports six functions and contains Naga"), `docs/lld/tier-resolution.md:453`, `tools/bench/subjects.json:32`, and `tools/bench/src/runners/wasm_cold_start.mjs:12` and `:337`.

**What**: F-101 raised the shipped release module's export count from six to twenty-one. The remediation corrected `docs/lld/build-targets.md:298`, but the same present-tense sentence survives in the benchmark LLD, the tier-resolution LLD, the `wasm.cold_start` subject note and the cold-start runner caveat. `benchmarks.md:169` says "The current release module exports six functions", which is false of the current module. The other four say "The release module exports six functions" and are attached to the recorded F-039 measurement, but the present tense reads as a claim about the current module.

**Why it is wrong**: The microscope's severity table says a claim in prose that is false is a defect, and rule 3 says every factual sentence is checkable. F-101's design plan's LLD impact names `benchmarks` as a document to update. The export inventory is part of F-101's deliverable. The release build exports 21.

**Evidence**: `wasm-pack build crates/ocelli-wasm --release --target web --out-dir target/release-count` exited 0. `grep -c "^export function" crates/ocelli-wasm/target/release-count/ocelli_wasm.js` reported 21. The names are `alloc`, `apply_commands`, `attachOffscreenCanvas`, `bindCpuCanvas`, `bulk_ticket`, `commit_frame`, `commit_selected_frame`, `configure_stack`, `copy_state`, `drain_events`, `install_panic_hook`, `ocelli_version`, `panic_record_len`, `panic_record_ptr`, `publishCpuCanvas`, `publishCpuColor`, `publishCpuInteraction`, `release_bulk`, `resizeCpuCanvas`, `resizeOffscreenCanvas` and `ring_capacity`. `rg -n "six functions" docs/ tools/` found the five sentences. `build-targets.md:298` now says twenty-one and matches the glue.

**Scope and regression**: Present in the reviewed tree `daf4dacd`. The affected consumers are readers of the benchmark LLD and the tier-resolution LLD, the `wasm.cold_start` subject note and the cold-start result caveat. The smallest fix is to change the current-state claims to twenty-one and to qualify the F-039-scoped sentences as the F-039 module, which exported six functions and was recorded at 1,999,061 bytes, noting that the current module exports 21 and has not been remeasured. The smallest meaningful guard is a doc consistency check that fails when a current-state export-count claim disagrees with the release glue's `export function` count. Uncertainty: the tier-resolution and bench sentences may be intended as history, but they use the present tense and a reader sees them as current.

## Smells

None.

## Nitpicks

None.

## Verified clean

- D1 from pass 5 is closed. `build-targets.md:298` says twenty-one. The dev and release glues export 21. The six table rows are exactly the six named exports. The F-101 paragraph lists 15. The union has no missing, extra or duplicate name. No other export-count claim in `build-targets.md` contradicts the count.
- N1 from pass 5 is closed. The pre-growth identity assertion discriminates a copy from a view. A view returned before growth carries `memoryBefore` and detaches, so `expect(state.buffer).not.toBe(memoryBefore)` goes red for a view. The glue mutation that drops the `.slice()` in `copy_state` and `drain_events` failed the growth test, and the restored baseline passed 7. The post-growth assertion can pass against a pre-growth view and is not a discriminator, but it is redundant and harmless because the pre-growth assertion and the content equality both catch the view. The comment is true.
- The boundary test still passes on the real built dev module, 7 passed. The rest of the file is unchanged in behaviour. The delta adds two identity assertions and changes the comment.
- No gate was disabled or weakened, no tolerance changed and no baseline widened. The diff over `ci/`, `vitest.config.ts`, `vitest.boundary.config.ts` and `package.json` is empty.
- `gate unsafe` and `gate bindgen` exit 0. There is no `unsafe` outside `crates/ocelli-wasm/src/ring.rs` and `crates/ocelli-core/src/cast.rs`, and `wasm-bindgen` is confined to `ocelli-wasm`.
- `gate content` and `gate prose` exit 0. No patient data, DICOM corpus file or build artefact is staged. The new prose has no em-dash and no prose semicolon.
- The new `build-targets.md` prose is true. The six named exports are the measurement root (`ocelli_version`), the three panic exports and the two render-target exports. The 15 F-101 exports are listed in the section above. Decode workers are not required to call the two render-target exports.
- The `.claude/reviews/F-X023-instruction-pass-4.md` change is outside F-101 and is an attribution clarification consistent with the pass 3 receipt.
- The new `.claude/reviews/F-101-boundary-pass-5.md` is the prior review, not F-101 product.
- N5 (`StackViewport.error` has no setter) remains a handoff to F-103. The `error` flag bit is still never set, and F-101 does not claim it.

## Limitations and handoff

The pass reviewed the remediation delta and the export-count claims. It did not re-audit the entire F-101 prose in `benchmarks.md`, the F-101 product logic, the CPU raster arithmetic, the oracle, GPU tests, `gate --sprint`, `#[ignore]` tests or the release size gate. The release build was used only to count exports, not as a size measurement.

The `.claude/reviews/F-X023-instruction-pass-4.md` attribution change is outside F-101 and is disclosed separately. I did not review the rest of the F-X023 material.

After the pinned tree was confirmed and this report was staged, another agent staged changes to `docs/lld/benchmarks.md`, `docs/lld/tier-resolution.md`, `tools/bench/subjects.json` and `tools/bench/src/runners/wasm_cold_start.mjs`. The index moved to `d5bc10584e8c588c5f89eba2bc5a9f950e1977ac`. The four paths change the five export-count sentences to distinguish the F-039 module, which exported six functions and was recorded at 1,999,061 bytes, from the current module, which exports twenty-one functions. This is a candidate remediation of D1. It has not been independently reviewed by this pass, so the new tree needs a fresh pass. One observation for that pass: the new sentences pair the past-tense `exported six functions` with the present-tense `contains Naga`. The tense is inconsistent but the claims are true of the two modules. I do not count it as a finding here because it is outside the pinned tree.

The result is 1 defect on the reviewed tree `daf4dacd`, so F-101 stays in progress. The remediation must update or qualify the five export-count sentences, then a fresh independent pass must review the new tree. N5 does not block F-101.

No commit, push, tag or verification receipt was created. No product file was edited. The glue mutation was in the gitignored built artefact, restored byte-identical with `cmp`, and the baseline was re-run green.

This report is a new staged addition. At report time `git write-tree` is the tree that contains this report, which is not the reviewed tree `daf4dacda7e270e0df3165070b1bc34cdcfe9a4a`. The reviewed tree's receipt must not be attached to the union that contains this report.
