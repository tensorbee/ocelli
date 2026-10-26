# F-101 review, pass 7

**Reviewed**: staged tree `a2e833f51b247cd4e0f7b656320b69777a1946a9`, confirmed by `git write-tree` at pin. The delta from `daf4dacda7e270e0df3165070b1bc34cdcfe9a4a` touches the four export-count remediation files, `docs/lld/benchmarks.md`, `docs/lld/tier-resolution.md`, `tools/bench/subjects.json` and `tools/bench/src/runners/wasm_cold_start.mjs`, plus the new `.claude/reviews/F-101-boundary-pass-6.md`, which is a review artefact and not F-101 product. The export-count remediation is byte-identical between `d5bc10584e8c588c5f89eba2bc5a9f950e1977ac` and `a2e833f5`. The index had no unstaged or untracked entries at pin.

**Author / reviewer**: Implementer recorded as Codex in `.claude/reviews/S16-codex-completion-evidence.md`. Reviewer is DeepSeek V4.1 Flash. I did not author the implementation, the remediation or the prior reviews. The remediation was authored by another agent. Independence is per changed material.

**Result**: 0 defects, 0 smells, 3 nitpicks

**Date**: 2026-10-26

## Previous findings

Pass 6 D1, the five present-tense export-count sentences that said six functions, is closed. All four product files now distinguish the F-039 module, which exported six functions and was recorded at 1,999,061 bytes, from the current module, which exports twenty-one functions. Pass 6 N1, the boundary test's buffer identity assertion, remains closed. Pass 4 S1, S2, N2, N3 and N4 remain closed. N5, `StackViewport.error` has no setter, remains a handoff to F-103 and is not an F-101 defect.

## Acceptance challenged

**Export inventory, current state.** I built the shipped release module with `wasm-pack build crates/ocelli-wasm --release --target web --out-dir target/review-count`. The glue exports 21 functions. The dev build to `target/review-dev` exports the same 21. The names are `alloc`, `apply_commands`, `attachOffscreenCanvas`, `bindCpuCanvas`, `bulk_ticket`, `commit_frame`, `commit_selected_frame`, `configure_stack`, `copy_state`, `drain_events`, `install_panic_hook`, `ocelli_version`, `panic_record_len`, `panic_record_ptr`, `publishCpuCanvas`, `publishCpuColor`, `publishCpuInteraction`, `release_bulk`, `resizeCpuCanvas`, `resizeOffscreenCanvas` and `ring_capacity`. The F-101 paragraph in `docs/lld/build-targets.md` lists 15 of these, and the six-row table lists the other six. The union has no missing, extra or duplicate name, so the current-state claim of twenty-one is true and the `docs/lld/build-targets.md:298` claim of twenty-one is true.

**Export inventory, F-039 historical state.** I did not accept the prior review's six-function claim on its own. I read the F-039 commit `b93eef0`. Its `crates/ocelli-wasm/src/lib.rs` exports `ocelli_version`, `install_panic_hook`, `panic_record_ptr` and `panic_record_len` on wasm32. Its `crates/ocelli-wasm/src/canvas.rs` exports `attachOffscreenCanvas` and `resizeOffscreenCanvas`. Its `Cargo.toml` declares `panic-probe = []` and does not make it a default feature, so `panic_probe_trigger` is not in the shipped module. The shipped F-039 module therefore exported six functions. The remediation's historical claim is true.

**Naga.** The F-039 commit adds `ocelli-wasm` to `ocelli-render` and to `wgpu`, and `ci/wasm-size-budget.json` records the F-039 edge as the first edge that pulls in wgpu and Naga. The current release wasm contains `naga-30.0.1` strings. The claim that the F-039 module contains Naga is true. The current module also contains Naga, so the mixed tense does not produce a false claim, but it is inconsistent prose and is recorded as nitpick N1.

**Size.** `ci/wasm-size-budget.json` records `bytes` 1999061 and a rebaseline story of F-039. `python3 scripts/pin_and_size_check.py --with-size` measured the current release module at 2,052,951 bytes against the baseline of 1,999,061 and the ceiling of 2,099,014, and exited 0. The budget was not moved by the remediation. The F-039 sentences describe the recorded 1,999,061-byte module correctly. The current module's measured size is not 1,999,061, which makes the placement of the unqualified "The recorded size is 1,999,061 bytes" sentence ambiguous in the subject note and the runner, and is recorded as nitpick N3.

**Cold start.** `ci/bench-baseline.json` records `wasm.cold_start` with value 2.3, story F-006, recorded 2026-09-05 and tolerance 0.25. `docs/lld/benchmarks.md` says the figure is the F-006 measurement and not a measurement of the current module. The F-039 sentences say cold start was not remeasured and that 2.3 ms is the F-006 measurement of the earlier module. Both statements match the baseline.

**Consistency.** The `wasm.cold_start` subject note, the runner header comment and the emitted `detail.caveat` carry the same four claims in the same order. They agree with each other and with `docs/lld/benchmarks.md`, apart from the clarity nitpicks below. The `docs/lld/tier-resolution.md` paragraph agrees with both.

**Guards and baselines.** The diff over `ci/`, `vitest.config.ts`, `vitest.boundary.config.ts`, `package.json`, `tools/bench/tests/`, `scripts/`, `crates/` and `packages/` is empty. No gate was disabled or weakened, no tolerance was changed, no baseline was widened and no size number was edited.

## Evidence

| Command or selected cases | Source and input identity | Executed now or prior receipt | Exit and observation |
|---|---|---|---|
| `git write-tree` at pin | index | executed now | `a2e833f51b247cd4e0f7b656320b69777a1946a9` matched the supplied tree |
| `git status --porcelain` at pin | index | executed now | 189 staged entries, no unstaged or untracked entries |
| `git diff --stat d5bc1058 a2e833f5` | index | executed now | only `.claude/reviews/F-101-boundary-pass-6.md`, the prior review report |
| `git diff --stat daf4dacd a2e833f5` | index | executed now | the four remediation files plus the pass-6 review report |
| `git diff daf4dacd a2e833f5 -- ci/ vitest.config.ts vitest.boundary.config.ts package.json tools/bench/tests/ scripts/ crates/ packages/` | config, baselines and tests | executed now | empty |
| `wasm-pack build crates/ocelli-wasm --release --target web --out-dir target/review-count` | release module | executed now | exit 0 |
| `grep -c "^export function" crates/ocelli-wasm/target/review-count/ocelli_wasm.js` | release glue | executed now | 21 |
| `wasm-pack build crates/ocelli-wasm --dev --target web --out-dir target/review-dev` | dev module | executed now | exit 0 |
| `grep -c "^export function" crates/ocelli-wasm/target/review-dev/ocelli_wasm.js` | dev glue | executed now | 21 |
| `git show b93eef0:crates/ocelli-wasm/src/lib.rs` and `.../src/canvas.rs` | F-039 source | executed now | six shipped exports, `panic_probe_trigger` behind the non-default `panic-probe` feature |
| `git show b93eef0:crates/ocelli-wasm/Cargo.toml` | F-039 features | executed now | `panic-probe = []` is not a default feature |
| `strings crates/ocelli-wasm/target/review-count/ocelli_wasm_bg.wasm` | release wasm | executed now | `naga-30.0.1` paths present |
| `python3 scripts/pin_and_size_check.py --with-size` | built release wasm and budget | executed now | exit 0, wasm size 2,052,951 bytes, baseline 1,999,061, ceiling 2,099,014 |
| `cat ci/wasm-size-budget.json` | size budget | executed now | `bytes` 1999061, `rebaselined.story` F-039, `to` 1999061 |
| `cat ci/bench-baseline.json` | bench baseline | executed now | `wasm.cold_start` value 2.3, story F-006, recorded 2026-09-05, tolerance 0.25 |
| `rg -n "six functions" docs/ tools/ packages/ .claude/reviews/` | docs, bench and review records | executed now | five product locations, all qualified as the F-039 module, plus historical review files |
| `rg -n "twenty-one|21|export" docs/lld/benchmarks.md docs/lld/tier-resolution.md docs/lld/build-targets.md tools/bench/subjects.json tools/bench/src/runners/wasm_cold_start.mjs` | docs and bench | executed now | current-state claims say twenty-one, `build-targets.md:298` says twenty-one, `build-targets.md:88` says the table names six exports |
| `bin/ocelli.sh gate bench` | staged tree | executed now | exit 0, 127 tests, 125 passed, 2 skipped |
| `node --test tools/bench/tests/run_test.mjs` | bench node suite | executed now | exit 0, 14 passed, 1 skipped |
| `bin/ocelli.sh gate prose` | staged tree | executed now | exit 0, 471 files |
| `bin/ocelli.sh gate content` | staged tree | executed now | exit 0 |
| `npm run lint` | package | executed now | exit 0 |
| `npm run typecheck` | package | executed now | exit 0 |
| `python3 scripts/sprint_workflow.py close-preflight` | sprint state | executed now | exit 1, FAIL: S16 is not ready to close, reasons in Limitations |
| Prior receipt, pass 6 | tree `daf4dacd` | prior receipt | 1 defect, 0 smells, 0 nitpicks |

## Defects

None.

## Smells

None.

## Nitpicks

### N1, the F-039 sentences pair a past-tense verb with a present-tense verb

**Where**: `docs/lld/benchmarks.md:169`, `docs/lld/tier-resolution.md:453`, `tools/bench/subjects.json:32`, `tools/bench/src/runners/wasm_cold_start.mjs:12` and `:338`.

**What**: The sentences say "The F-039 release module exported six functions and contains Naga". The tense is inconsistent. The claims are true, because the F-039 module contained Naga and the current module also contains Naga, so no reader is misled about a fact. The prose reads awkwardly.

**Why it is a nitpick**: It is a style inconsistency, not a false claim. The pass 6 review already noted it. It does not block completion.

**Scope and regression**: Four product files and the runner caveat. The smallest fix is to use one tense for the F-039 module, for example "exported six functions and contained Naga". The current module's Naga status can be stated separately if it matters.

### N2, "this measurement names" has an ambiguous referent in the benchmark LLD

**Where**: `docs/lld/benchmarks.md:169`.

**What**: The sentence says "The F-039 release module that this measurement names exported six functions". The paragraph discusses two measurements. The size half is the F-039 measurement at 1,999,061 bytes. The cold-start half is the F-006 measurement at 2.3 ms. The F-039 module is named by the size measurement, not by the cold-start measurement. The last-mentioned measurement before the sentence is the F-006 cold-start measurement. A reader who resolves "this measurement" to the cold-start measurement reads the relative clause as attributing the 2.3 ms measurement to the F-039 module, which the paragraph itself says is the F-006 measurement of the earlier module.

**Why it is a nitpick**: The intended reading is true, and the paragraph's next sentence names the 2.3 ms figure as the F-006 baseline. The ambiguity is clarity only.

**Scope and regression**: Readers of the benchmark LLD. The smallest fix is to name the measurement directly, for example "The F-039 release module that the size measurement names". A regression would be a reader citing the 2.3 ms figure as an F-039 measurement.

### N3, "The recorded size is 1,999,061 bytes" is not tied to the F-039 module in the subject note and the runner

**Where**: `tools/bench/subjects.json:32`, `tools/bench/src/runners/wasm_cold_start.mjs:11-13` and `:338-342`.

**What**: The sentence "The recorded size is 1,999,061 bytes" follows "The current module exports twenty-one functions". The current release module measured now is 2,052,951 bytes, which is within the 5 per cent ceiling of 2,099,014. The recorded size in `ci/wasm-size-budget.json` is 1,999,061, and that is the F-039 measurement. The sentence is true of the budget, but its placement after the current-module sentence leaves the referent unclear. A reader could attach the recorded size to the current module. The emitted `detail.caveat` sits beside `detail.artefact.bytes`, which is the current artefact's size, so the same ambiguity appears in a recorded result.

**Why it is a nitpick**: The claim is true of the size budget and the surrounding sentences name the F-039 module. No false claim is made. The ambiguity is clarity only.

**Scope and regression**: Readers of `bin/ocelli.sh bench --list`, the subject note and a recorded cold-start result. The smallest fix is to tie the recorded size to the F-039 module, for example "The F-039 module's recorded size is 1,999,061 bytes", or to state the current module's measured size separately. A regression would be a future cold-start record whose caveat is read as claiming the current artefact is 1,999,061 bytes.

## Verified clean

- Pass 6 D1 is closed. `docs/lld/benchmarks.md:169-170` now says the F-039 module exported six functions and the current module exports twenty-one. `docs/lld/tier-resolution.md:453-455` names the F-039 module and says the current module exports twenty-one and has not been remeasured. `tools/bench/subjects.json:32` distinguishes the F-039 module from the current module. `tools/bench/src/runners/wasm_cold_start.mjs:11-13` and `:338-342` do the same. No product sentence still says the current module exports six functions.
- The release glue exports 21 functions and the dev glue exports 21. The names match the F-101 paragraph's 15 plus the six-row table's six, with no missing, extra or duplicate name.
- The F-039 commit's shipped module exported six functions. The `panic-probe` feature is not a default feature. The F-039 module contains Naga.
- `ci/wasm-size-budget.json` records 1,999,061 bytes for the F-039 module. The current release module measured now is 2,052,951 bytes, inside the ceiling of 2,099,014. The budget file was not moved.
- `ci/bench-baseline.json` records 2.3 ms for `wasm.cold_start` with story F-006, recorded 2026-09-05 and tolerance 0.25. The 2.3 ms figure is correctly attributed to the F-006 measurement of the earlier module.
- The subject note, the runner header and the emitted caveat agree with each other and with `docs/lld/benchmarks.md` on the F-039 module, the current module, the recorded size, the unremeasured cold start and gate A4 staying open.
- No gate was disabled or weakened, no tolerance was changed, no baseline was widened and no size number was edited. The diff over `ci/`, `vitest.config.ts`, `vitest.boundary.config.ts`, `package.json`, `tools/bench/tests/`, `scripts/`, `crates/` and `packages/` is empty.
- `gate bench` exits 0 with 127 tests, 125 passed and 2 skipped. `node --test tools/bench/tests/run_test.mjs` exits 0 with 14 passed and 1 skipped. `gate prose` exits 0 over 471 files. `gate content` exits 0. `npm run lint` exits 0. `npm run typecheck` exits 0. `pin_and_size_check.py --with-size` exits 0 with the size inside the budget.
- `docs/lld/build-targets.md:88` says the table names six exports from F-005 and F-039. The six rows are correct, and `ocelli_version` is F-002's measurement root per the table row at line 307. The phrase "from F-005 and F-039" omits F-002 for `ocelli_version`. This is a pre-existing historical attribution imprecision, not a module export-count claim and not introduced by the remediation. The count of six table rows is correct.
- The `.claude/reviews/F-101-boundary-pass-6.md` change is a review artefact, not F-101 product. Its statement that the index moved to `d5bc1058` was true when it was written. The index is now `a2e833f5` because the file was staged after it was written. The four remediation paths are unchanged between the two trees.

## Limitations and handoff

The pass reviewed the remediation delta and the export-count claims. It did not re-audit the entire F-101 prose in `benchmarks.md`, the F-101 product logic, the CPU raster arithmetic, the oracle, GPU tests, `gate --sprint`, `#[ignore]` tests or the release size gate as a recorded baseline. The release build was used to count exports and to measure size, not as a recorded measurement. The F-039 historical export count was verified from the F-039 commit's source and feature table, not from a rebuilt F-039 artefact.

`python3 scripts/sprint_workflow.py close-preflight` exits 1 and reports:

```text
FAIL: S16 is not ready to close
  F-X023 is completed in run state but backlog status is in-progress
  neither completed nor carried: F-101
  F-101 last review pass 6 reports 1 defects and 0 smells
  no sprint-scope review recorded
  latest sprint-profile verification tree f4d963f1adad is stale for current tree a2e833f51b24
  working tree is not clean
  staged tree a2e833f51b24 is not HEAD tree 3b97147d10b2
```

These are preflight observations. The pass does not act on them. The pass 6 line is stale because this pass is not yet recorded in the sprint state. F-101 remains in progress until the workflow records the pass and the remaining preflight conditions are handled by the owning workflow.

The result is 0 defects and 0 smells, so the remediation passes this pass. N5 remains a handoff to F-103. A fresh independent pass is required after any further change to the tree.

No commit, push, tag or verification receipt was created. No product file was edited. The review build outputs are under the gitignored `crates/ocelli-wasm/target/` and `crates/ocelli-wasm/pkg/`.

This report is a new staged addition. At report time the tree that contains this report is not the reviewed tree `a2e833f51b247cd4e0f7b656320b69777a1946a9`. The reviewed tree's receipt must not be attached to the union that contains this report.
