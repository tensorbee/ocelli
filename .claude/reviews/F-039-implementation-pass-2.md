# F-039 review, pass 2

**Reviewed**: working tree on `sprint/s13`, uncommitted on `9426845`. Pass 1's three findings were checked on the current files. Independent review. The reviewer did not write the code and did not write pass 1.
**Result**: 7 defects, 0 smells, 0 nitpicks

## Defects

### D1, the tier-resolution bullet now says the opposite of its heading

**Where**: `docs/lld/tier-resolution.md:456` and `docs/lld/tier-resolution.md:458`
**What**: The heading is "What this story deliberately does not do". The first bullet's bold sentence is "Tier resolution runs from `ocelli-wasm`."
**Why it is wrong**: Pass 1 D2 was fixed by rewriting that bullet in place. The sentences under the bold lead are right that F-004 left the edge off and F-039 added it. The heading still classifies the bullet as something not done. The other bullets under the same heading still lead with "It does not". A heading and the current-state sentence it introduces cannot both be true.
**Evidence**: `crates/ocelli-wasm/src/canvas.rs:200` calls `resolve_adapter` on the attach path, so the bold sentence matches the code. The defect is the heading the remediation left around it.

### D2, errors.md still says the module exports four functions

**Where**: `docs/lld/errors.md:303` to `docs/lld/errors.md:310`
**What**: The heading is "Four exports, and three of them are called exactly once". The table lists `ocelli_version`, `install_panic_hook`, `panic_record_ptr` and `panic_record_len`.
**Why it is wrong**: Pass 1 D1 corrected the same count in `docs/lld/build-targets.md` and now points at this file as where those three panic exports are specified. The specification of the three is here. The count is not. The module exports six functions. This file is living LLD. The plan's LLD impact list does not name it, so that list is incomplete for the export surface this story changed.
**Evidence**: `crates/ocelli-wasm/pkg/ocelli_wasm.js` exports `attachOffscreenCanvas`, `install_panic_hook`, `ocelli_version`, `panic_record_len`, `panic_record_ptr` and `resizeOffscreenCanvas`. `git diff` does not include `docs/lld/errors.md`.

### D3, the cold-start subject still describes a module with four exports and no Naga

**Where**: `tools/bench/subjects.json:32`, `tools/bench/src/runners/wasm_cold_start.mjs:11` and `tools/bench/src/runners/wasm_cold_start.mjs:339`
**What**: The subject note says the module holds four exported functions, no wgpu and no Naga. The runner comment says that sentence is the caveat `docs/lld/build-targets.md` puts on the size number. The runner writes the same sentence into `detail.caveat` on a new run.
**Why it is wrong**: Pass 1 D2 removed that sentence from `docs/lld/benchmarks.md`. `docs/lld/build-targets.md` no longer says it either. The release glue exports six functions, and the built wasm contains `naga-30.0.1`. A later cold-start record would attach the old sentence to this module. The stored entry in `ci/bench-baseline.json` is the F-006 measurement. Its artefact block says 16388 bytes, story F-006, recorded 2026-09-05. That stored caveat describes that artefact and is not this defect.
**Evidence**: A Python read of `crates/ocelli-wasm/pkg/ocelli_wasm_bg.wasm` printed size 1999061 and found the bytes `naga-30.0.1`. The subject note and the runner string were not in the pass 1 list.

### D4, the ocelli-core comment describes the budget move this story replaced

**Where**: `crates/ocelli-wasm/Cargo.toml:88`
**What**: The comment says the `ocelli-core` dependency is the larger half of the size-budget move recorded in `ci/wasm-size-budget.json`. The next comment says the F-039 render edge is the size cost recorded in that same file.
**Why it is wrong**: The budget file's recorded move is the edge to `ocelli-render`. The two comments cannot both describe that record. The story rewrote the file the first comment names and left the comment in the same manifest.
**Evidence**: `ci/wasm-size-budget.json` has `story` `F-039`, `from` 16388, `to` 1999061, `delta` 1982673 and a cause that names the render edge. `1999061 - 16388` is 1982673.

### D5, D-14 still says the measured wasm cost today is zero

**Where**: `docs/hld/DEVIATIONS.md:31`, the Why cell of row D-14
**What**: "Measured cost today is zero bytes, because `ocelli-wasm` does not depend on `ocelli-render` and so never reaches wgpu, whatever else it depends on." The same cell says the size budget moves only when the render path is wired in from S11.
**Why it is wrong**: The webgl feature itself is unchanged. The plan is right that this story does not change that feature set. The today sentence is a separate factual claim, and this working tree makes it false. F-005 already amended this cell when `ocelli-core` landed and the cost was still zero. This story is the one that makes the cost non-zero. `/complete-feature` does not edit the deviation register. It only requires that a relied-on deviation have a row. The prose exemption for `docs/hld/` does not make the sentence true. `DEVIATIONS.md` is this repository's register, not the author's HLD text. The row has to change in this story. The feature-set reason stays.
**Evidence**: `crates/ocelli-wasm/Cargo.toml` depends on `ocelli-render` at line 96 and on `wgpu` at line 101. The budget `bytes` field is 1999061. `.claude/plans/F-039-design.md` says deviations are none and that D-14's webgl feature is not changed. `.claude/commands/complete-feature.md` has no step that rewrites a Why cell.

### D6, the sprint page still says the budget has not moved

**Where**: `docs/sprints/CURRENT_SPRINT.md:52`
**What**: "The wasm size budget has not moved." It then says `ci/wasm-size-budget.json` records 16388 bytes, that `bearing_on_gate_A4` says the number that bears on gate A4 arrives with F-039, and that `ocelli-wasm` still reaches `ocelli-core` and nothing else.
**Why it is wrong**: Completion does not own this page. `/complete-feature` does not rewrite it. `/sync-sprint` rewrites it when a sprint opens, which is not this story. The warning that the status column goes stale names that column and points at the backlog. It does not cover this bullet. The bullet states the budget file's bytes and the text of `bearing_on_gate_A4`, and both are false on this tree. F-037 corrected a budget claim on the sprint page inside that story's commit, which is the precedent. Prior `AS_BUILT.md` entries are append-only history and are not this defect.
**Evidence**: The budget file's `bytes` is 1999061. Its `bearing_on_gate_A4` says this measurement is the first time the wasm module reaches wgpu and Naga, and that the number does not close A4. `crates/ocelli-wasm/Cargo.toml` also depends on `ocelli-render` and `wgpu`.

### D7, the changelog still attributes the F-005 delta to the current budget file

**Where**: `CHANGELOG.md:104`
**What**: The unreleased note says the first measurement was 14,104 bytes and 16,388 after F-005, with that delta and its cause attributed in `ci/wasm-size-budget.json`, and that the gap to gate A4 is 183x at the low end and 488x at the high one.
**Why it is wrong**: The budget file's recorded cause is the F-039 render edge. The F-005 cause is no longer in the file. `3,000,000 / 16,388` is about 183 and `8,000,000 / 16,388` is about 488. Those ratios describe the old module. `3,000,000 / 1,999,061` is about 1.5, so the current module is inside the low end of the 3 to 8 MB estimate rather than two orders of magnitude under it. The paragraph is current unreleased text and it does not mention the recorded size.
**Evidence**: `ci/wasm-size-budget.json` `cause` names the edge from `ocelli-wasm` to `ocelli-render`. `from` is 16388 and `to` is 1999061.

## Smells

None.

## Nitpicks

None. Pass 1's N1, N2 and N3 were not reopened. The remediation did not turn any of them into a defect.

## Verified clean

Pass 1 D1 is fixed. `docs/lld/build-targets.md:305` names `install_panic_hook`, `panic_record_ptr` and `panic_record_len` as F-005's, says `docs/lld/errors.md` specifies those three, and keeps "none of them is called after a trap" on those three. Line 309 says `attachOffscreenCanvas` and `resizeOffscreenCanvas` are F-039. The table at lines 298-303 ends with `panic_record_len`, then the two canvas exports. The panic rule is no longer attached to the canvas rows.

Pass 1 D2 is fixed in the two files it named. `docs/lld/tier-resolution.md:451` says F-039 makes `ocelli-wasm` depend on `ocelli-render` and on `wgpu`, and line 453 records 1,999,061 bytes. Lines 458-464 say the attach export calls `resolve_adapter`, the module contains Naga and exports six functions, and that cold start was not remeasured. The 2.3 ms figure is named as the F-006 measurement of the earlier module. `docs/lld/benchmarks.md:161` records the size half at 1,999,061 bytes for the module that reaches wgpu and Naga. Lines 163-165 say the 2.3 ms figure is the F-006 measurement, was not remeasured after F-039, and is not a measurement of the current module. Line 169 says the current release module exports six functions and contains Naga. A search of those two files plus `docs/lld/build-targets.md` found no remaining "does not depend on `ocelli-render`", "four exported" or "no Naga". The two remaining "16,388" hits in `build-targets.md` are the previous recorded baseline, and both say the parent commit was not rebuilt.

Those current-module claims match the tree, apart from the files in the defects above. The release glue exports those six names and no further function export. The wasm file on disk is 1,999,061 bytes. It contains `naga-30.0.1`. `1999061 / 1048576` is 1.90655, and `scripts/pin_and_size_check.py` formats that as `.2f` MiB, which is 1.91. `ci/bench-baseline.json` still records cold start `value` 2.3, `story` F-006, artefact `bytes` 16388. The updated LLD does not present 2.3 ms as a measurement of this module.

Pass 1 S1 is fixed. `crates/ocelli-render/src/extent.rs:41` returns `u32::MAX` when the rounded product is non-finite and positive, and returns 0 when it is non-finite and not positive. That return is before the empty-axis floor at line 47, so an overflowing positive product does not become 1. The test at line 156 is `device_extent(1e308, 1.0, 10.0) == Ok((u32::MAX, 10))`. Line 155 keeps the finite `1e308` at dpr 1 on the same saturation. `bin/ocelli.sh test ocelli-render extent::tests -- --test-threads=8` exited 0. 6 passed, including `an_overflowing_product_saturates_to_u32_max`, `extent_at_dpr_one_two_and_one_and_a_quarter` and `a_positive_css_size_whose_product_rounds_to_zero_becomes_one`. The kept rows are 100 by 50 at dpr 2 equals 200 by 100, dpr 1.25 equals 125 by 63, and 0.4 by 10 equals 1 by 10. `saturating_u32` still returns 0 for a non-finite value. The public function does not call it with one. That private arm is not a second overflow path.

The on-disk wasm is older than `extent.rs`. This pass did not rebuild it and did not re-run the ignored GPU suite. The recorded 1,999,061 is the file on disk. It is not a claim that the overflow fix was inside that build.

`docs/sprints/AS_BUILT.md` still says, in earlier entries, that the cost was zero and that the module had four functions and no Naga. Those entries are append-only history. They were not charged. The `ocelli_version` doc comment in `crates/ocelli-wasm/src/lib.rs:42` still says that function is the whole exported surface until F-101. That sentence was already false after F-005 added the panic exports. This pass did not charge it to F-039.
