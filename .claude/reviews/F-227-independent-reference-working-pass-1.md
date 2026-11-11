# F-227 independent AC16 reference, working pass 1

# F227 independent window-stage review

Result: 0 defects, 0 smells in the bounded normv4 and private reference window stage. This is readiness of this arithmetic stage for composition and independent operator evidence, not F227 completion, a study observation or verification of the upstream sampling/rank calculator. No canonical source was changed.

Root authored the norm, reference and tests. The reviewer wrote none of those files, supplied the four-point hand counterexample and requested source-contract clarifications. The review used the approved F227 plan and historical review text, not current product arithmetic or captured product window output. The lead's separate product-source exposure after initial draft45d is explicitly retained in normv4 and is not presented as pre-exposure independence.

Pinned identities:

- Normv4 b503b3d44f837645cd78e4ed679f8cf90092faff56a39ff2374305db4390b4a0.
- Reference 746e3edab10778c2f2a509a2251a81f90fdc908613a9d8dace5e091036828b4e.
- Eleven hand controls fec4c9b3051471d8e2d24d6f3e23116fe26bea9188a2220ca6447469971836d3.

## Contract and source assessment

Plan items 5 to 8 specify nearest ranks and f32 arithmetic, and item 9 retains the constant value as centre. Pass 3's geometric search is made explicit as an initial candidate plus 48 changes. Accepted pass 4 review describes changes relative to the first width and power-of-two units. Pass 7 mutation M4 independently pins availability of the 48th change. My earlier upward-binade interpretation was overbroad. Directional neighboring spacing is consistent with the retained pass 2 narrowing direction and is now explicitly recorded, with post-freeze source exposure disclosed.

Pass 4 line 459 requires all four original and moved endpoints. Its remediation ruling changes the pair on which the window is built, without expressly withdrawing that four-point check. Normv4 retains it. For q=2^-24, original low 1-17q and high 1+34q move to 1-q and 1+2q. Width 3q and centre 1 satisfy the original extremes but put moved low near 42.5. That independently distinguishes a margin check from an original-only check.

Pass 5 requires the strict half-grey outward checks, and pass 6 applies them to constants as well. Exact centre 127.5 remains. The reference enforces both centre and outward constraints. Fewer than 3 numeric neighboring steps uses the constant branch. Numeric steps merge zero while rank sorting distinguishes negative zero before positive zero. The latter is an upstream composition requirement. Equal pairs retain low's sign, while close-pair midpoint halves round before addition, preserving the subnormal 3q/4q control.

The reference explicitly rounds every operation to binary32, uses no fused multiply-add and imports no Ocelli code. Its LINEAR_EXACT branch ordering matches the normative piecewise formula, including <= on the lower boundary and > on the upper boundary. Infinite comparison bounds are allowed under IEEE comparisons, while finite inputs/centre/positive width and finite accepted outputs are required. The finite MAX centre/width/input control correctly gives 127.5 despite an infinite upper comparison bound.

The width search checks the base first, then 48 changes k=0 to 47. It uses the initial width and directional neighbor gap, not accumulated previous candidates. The first passing window is returned. The constant reach control distinguishes the final candidate from an off-by-one budget. The midpoint-cap loop is equivalent to min(16,floor((distance-1)/2)) for numeric neighboring values, including the zero crossing. No tolerance or budget was changed.

The code treats an overflowing preliminary raw high-low span as a closed nonfinite-window refusal. This review reads the norm's initial-span clause in that sense. Detailed Rust refusal-variant equivalence is not claimed. Inputs outside the independent tool's covered policy must remain unavailable/refused, never approximate or passing. A constraint-only check cannot stand in for exact candidate-window equality.

## Executed evidence

The private pinned control copy ran all 11 tests, direct exit 0 in 0.059639167 seconds. A separate copy dropped only moved endpoints from the percentile extreme check. It exited 1 in 0.049997959 seconds at the four-point test: it returned width 3q instead of independently expected 2q. Every other test passed. This is a semantic rejection, not a launch/import failure.

Receipts are f227-independent-window-review-green.json and f227-independent-window-review-original-only-mutant.json, with adjacent complete logs. Identity.json pins the mutant as 5f930b6838066e62b42a9d3ec4a27134a79c96f0421cb4e5a64ebbaac528e5c2. Root's norm/reference/tests were rehashed after the checks and remained unchanged. No product, browser, GPU, actual study or full-workspace run occurred.

# Independent F227 verifier reader/sampling review, pending v5 repair

Codex child glm_codec_merge read the declared seven-path v3 patch and actual352-line main reader/controller,105-line scalar and all four permanent test modules. Review is source only. Canonical DICOM expert guidance and approved F227 sampling/finite-binary32 clarification were read. No DICOM input, study, browser, native job, actual test or hardware was run. Root's confirmed v3 buffered-native CLI privacy failure prevents whole-v3 approval. The separate f244 reviewer owns v4 lifetime-FD privacy delta, not this reviewer.

Unchanged sampling/closed comparison logic is consistent with approved norms: caller-supplied exact page-order records, min(M,16) fixed spread including first/last, cap split by declared K before skips, row-major stride ceiling, stored-zero and per-frame padding exclusion before separately rounded binary32 rescale, finite rejection, total order signed-zero handling, pooled nearest ranks and no replacement for skipped frames. Unsupported/decode failure marks coverage false and blocks any automatic equality pass. Supplied actual decimal strings are compared as finite binary32 bit patterns, no display rounding/tolerance or expected Ocelli output reuse. The file-VOI mode is explicitly only selected-sample ineligibility, not real browser/scroll/label acceptance.

Covered reader descriptor shape is explicit before pixel_array: monochrome only,1sample,8/16allocated,1throughallocated stored,highbit=stored-1,signedness0/1,integer array and declared selected-frame shape. Floating Pixel Data and Modality LUT are closed unsupported. Per-frame then shared then top-level rescale/VOI macro lookup, zero-based selected frame, and local padding metadata are sourced independently through pydicom. Malformed partial rescale, unknown decode and nonfinite pool cannot pass. Extra reader coverage is not invented.

The scalar file is byte-identical to root's separately independently reviewed746e3eda reference. Its explicit operation rounding, inward endpoint checks, constant/background widening, directional ULP search, finite refusal and negative-zero handling match the current normative amendment. All12 hand controls, descriptor bypass, sampling/exclusion tests, wrong-bit observation and Python/direct-FD canary tests were read as source evidence. Their author's direct green/mutant receipts remain separate from this read-only review.

One reader eligibility defect was identified before approval: _present treats nonempty all-empty WindowCenter/Width component lists as supplied file VOI. Current product and lead ruling treat all-empty numeric slots as absence. Such a false ineligible stage can make file-voi-control pass. Root accepted matching absence semantics with a window-value-specific predicate while retaining independent SQ presence semantics. Await frozen v5 actual-pydicom RED/GREEN and delta before resolving this finding or approving the repaired reader. No false operator-input failure or runtime claim is made from this source analysis.

Closed result projection excludes filenames, identifiers, actual/expected values, exception strings and metadata. Paths are caller-owned relative descendants checked after resolution, no file discovery. Output is a fresh private 0600 file with existing targets refused. CLI privacy remains excluded until the separate current-entrypoint review and actual controls pass. Manual/operator authorization, fixture grouping/page-order selection, actual studies and browser first/middle/last/scroll observations remain open, not completed by this tool.

## V5 eligibility finding closure

Result for the repaired reader/sampling/bit-comparison scope:0 remaining supported defects,0smells,0nits. Approve exact seven-path v5 source identity from durable-handoff-v5.json, patch00fb27677db88ca43bad91e53a90d217ad3947acf07594d789e4f41b9982adbd and maina97225c26d24e547bd7cbd37295a783f44638276998f84a426a6d2edf53cec9c, subject to the separate independent privacy review. All seven actual source hashes independently match the complete patch extraction and author's handoff. Generated pyc artifacts are excluded.

The only executable reader change from v4 is a WC/WW-specific nonrecursive slot-presence check. None, empty collections and all-null/empty component lists remain absent, while numeric zero or any nonempty numeric component remains supplied. VOI LUT Sequence keeps its independent sequence presence rule, so a nonempty sequence containing an empty item is not treated as absent window values. Scalar, sampling, modality and CLI privacy code remain unchanged. The documentation accurately distinguishes lifetime CLI suppression from temporary callable-main restoration, separately reviewed by f244.

Read all five new permanent controls. Actual pydicom-generated files exercise both public consumers: all-empty WC/WW must allow the independently known constant7 auto window, and must not certify file-voi-control. Typed None tests use actual Dataset/DataElement/MultiValue rather than inventing a JSON shape. Real zero, mixed slots and nonempty SQ remain ineligible. No new arithmetic, decimal-equality tolerance, cutoff or product behavior is selected.

The retained old-source both-routes control directly exits1 and logs two intended assertion failures: automatic mode incorrectlyFalse and file-voi-control incorrectlyTrue. This is semantic RED, unlike the earlier retained setup-error attempt. Current frozen synthetic suite exits0 with42tests. This reviewer read the receipts/source and did not run them. Root's actual native HTTP proof then independently establishes hand ranks before product calls, actual-route CLI0 and one-f32-bit wrong observation CLI1/different-window, with scalar/main/binary hashes stable and owned server stopped. These are generated synthetic controls, not operator or browser evidence.

The reader eligibility finding is resolved. Existing v3 privacy failure remains preserved and is not retroactively clean. Whole-tool adoption still requires the separate v4 lifetime-FD privacy approval and root's full-source approval. Actual operator-root execution, browser observations, manual labels/scrolling and fullAC16 acceptance remain open.

# F227 CLI privacy delta, independent participant review

Reviewer: Codex f244_review. Author: Codex f225_review for the CLI and these added tests. The sampling proposal originated with GLM. Root authored the separate scalar reference, which this reviewer previously reviewed. This pass addresses only the v4 entrypoint/privacy delta and associated documentation and controls. It does not independently approve unchanged sampling or scalar code again.

## Source identity

Frozen seven-path patch: `/private/tmp/ocelli-s46-codex/f227-ac16-independent-verifier-glm/durable-seven-path-v4.patch`, SHA256 `9dbc80331ba436c3ece965cf9d91f1563712ed743a8623a627d5eaad862143e7`.

Main source SHA256 `4e6872f9645fd7ebd7c25d1ad90106c01a3aa0556b08b93876b1ee9f6dbbd156`. CLI tests SHA256 `a43e57e6861a290987a6793e2344a74051609079849b32ac80649a08346c9e16`. Complete seven source hashes are in adjacent `identity.json`. They were rechecked unchanged after controls.

## Result

0 defects, 1 documentation smell. The actual script entrypoint duplicates original stdout locally for the deliberately closed summary, directs FD1 and FD2 and global Python streams to devnull, and leaves logging and warning suppression installed through interpreter shutdown. The summary duplicate is not installed globally. Native stdio buffered before the temporary mapping boundary ends consequently still flushes into devnull. Production uses no ctypes.

Callable main retains a temporary serial boundary and restores descriptors, Python streams and prior logging disable state. That callable boundary does not claim process-lifetime buffered-native isolation. The actual process entrypoint supplies that stronger boundary. Error results retain only closed stages/counts. Successful real script invocation and refused invocation produce only the closed summary. Existing result files remain preserved.

S1: `auto_window_reference.md` currently says the CLI boundary temporarily discards diagnostics and restores both streams. That description applies to callable main. The actual script now intentionally keeps suppression through shutdown. Describe the two lifetimes separately. No code or policy change is requested.

## Executed discrimination

Copied the exact seven files into a private control directory. Only the seven CLI tests were replayed, using existing NumPy and pydicom with synthetic fixtures. Direct receipt `f227-cli-v4-independent-control.json`: exit0, elapsed0.5675047499826178 seconds. All7 tests pass, including actual runpy entrypoint buffered/direct/native/Python/logging atexit canaries, actual successful script invocation, private result permissions and callable restoration on success/refusal.

In a separate copy, changed only actual `__main__` from `_entrypoint()` to `main()`. The permanent helper remains present. Mutant source SHA256 `9e486a668db02a9a7f867b866cd9666889fa250762629834bd1530d88fe5b01d`. Direct receipt `f227-cli-v4-independent-entrypoint-mutant.json`: exit1, elapsed0.47031566698569804 seconds. The actual-entrypoint test fails on synthetic buffered and atexit canaries. The other6 tests pass. This discriminates actual call-site/lifetime use rather than merely the helper's existence. Logs and receipts are outside these private source copies under the existing evidence root.

## Limits

No operator study, browser, native viewer, benchmark, source build or canonical/index edit. No tolerance or numerical-policy changes. The earlier two controller privacy failures remain separately preserved and are not attributed to GLM. The controls do not assert arbitrary malicious plugin containment, security isolation or full AC16 acceptance. Root owns adoption and any operator authorization. Documentation follow-up is required for a clean complete packet.

## Bounded v5 documentation follow-up

The v5 patch SHA256 is `00fb27677db88ca43bad91e53a90d217ad3947acf07594d789e4f41b9982adbd`. Its documentation SHA256 is `79bedee4a36644eb17e2c43e964bcb10906b8f616bb2979594bf2283ffdf171b`. The exact approved sentences now distinguish process-lifetime CLI suppression from temporary callable-main restoration. S1 is closed. This narrow follow-up is 0D0S.

Compared exact source segments of `_quiet_diagnostics`, `main`, `_entrypoint` and the actual `__main__` call with the pinned v4 control copy. All are byte-identical. Whole v5 main source SHA256 is `a97225c26d24e547bd7cbd37295a783f44638276998f84a426a6d2edf53cec9c`. The reader's new WC/WW predicate and five new tests are outside this privacy follow-up and receive separate root/codec review. No runtime tests were repeated. The prior7test control and semantic mutant remain applicable to unchanged privacy code. No operator-study or AC16 completion claim is supplied.

The original v4 report remains byte-preserved as `review-v4-original.md`. Exact source comparison and all seven v5 hash checks are recorded in `v5-doc-closure.json`.

## Root final composition and consumer review

Root independently reviewed the full reader, sampling, comparator, CLI and export-helper source. The scalar source and hand controls were authored by root and have the independent arithmetic review above. The other components were authored through GLM and corrected by participant f225. Root found two actual controller privacy defects with direct native-FD and buffered-C-stdio controls, both retained as semantic failures. The final entrypoint keeps diagnostics discarded through shutdown. The independent reader's all-empty-window finding is resolved without changing product behavior. The model's page-ordinal export and synchronous cleanup defects were corrected before adoption. Root is independent of those repairs.

The final Python source is `a97225c26d24e547bd7cbd37295a783f44638276998f84a426a6d2edf53cec9c`, scalar `746e3edab10778c2f2a509a2251a81f90fdc908613a9d8dace5e091036828b4e`. Root executed all 42 synthetic controls, exit 0 in 0.566163 seconds. The actual native viewer consumer control independently established ranks 2/198 and values 2/196 before product calls, then accepted the observed route strings through the CLI and rejected a one-binary32-step wrong width with exit 1 and different-window. Command exit 0 in 0.303700 seconds. Source and binary hashes matched before/after, and the owned server stopped. This is synthetic consumer evidence, not operator-study acceptance.

The corrected caller-input helper first passed 11 VM controls, with original ordinal-frame and cleanup failures preserved. Root read every line and used real headless Chromium 151.0.7922.34 under Node 24.13.0. The actual download preserved the viewer order z.dcm then a.dcm despite lexical filenames, both source-frame indices 0, and unrounded cached route strings. The independent CLI accepted both the automatic mapping and selected-sample file-VOI control. A manual viewport override produced only the fixed refusal. The final comment-annotated helper repeated this browser proof, exit 0 in 0.846145 seconds. Browser and owned server closed, and all source/binary hashes stayed unchanged. This proves the download/input composition on synthetic files, not per-frame clinical display, scrolling or labels.

The first integrated lint run failed on missing browser context, an externally invoked function and empty catch blocks. Root added an exact-file classic-script context with actual readonly inputs and no rule disable, plus exported-function and silent-catch comments. The executable helper AST is identical. Corrected lint passes. Permanent VM controls are retained as the explicit Node test beside the explicit Python suite. No floor-discovery claim is made for either acceptance-tool suite. Final profile verification is separate.

# Independent F227 helper lint-context review

Codex child glm_codec_merge reviewed root's exact current ESLint/helper change. Result:0 supported defects,0 smells,0 nits in this narrow integration-context adjustment. No lint, tests, browser, hardware, product edits or candidate/index writes were run by this reviewer. Root owns direct corrected lint, AST and actual synthetic browser receipts.

The configuration adds one exact path tools/bench/export_active_window_check.js, sourceType script and13readonly external bindings. The helper is a classic script injected/called in the existing folder page, not an ES module. Eight lexical page inputs have actual current declarations: cacheGeneration,activeViewport,seriesOf,seriesWindowJobs,currentImage,seriesWindowRefused,seriesWindows,seriesHeader. The five other declarations are actual used browser APIs:window,document,URL,Blob,setTimeout. No new global API or writable binding is invented. Removing this one block exactly reconstructs HEAD's existing configuration. No rule or ignore entry is disabled, widened or relaxed for other files. Recommended JavaScript checks remain active.

Frozen pre-lint helper09569c62d3130b599943da1365272233c9a63299c1a4e40dc0b0d3c3b8cc6dd6 matches its author handoff. The new helperad613741c5b35c55e8be07e90b412a65ed3a39b8d2d2e432164ea31912c7708b differs only by the explicit externally called entrypoint declaration comment and three purposeful empty-catch explanations. Removing those exact four comments reproduces every original byte. The catch bodies remain deliberately silent to preserve closed failure/privacy and best-effort cleanup semantics already reviewed. The exported comment names only the actual external function and does not disable no-unused-vars globally. No executable behavior changed.

This is legitimate missing integration context, not a gate-weakening workaround or a new functional model defect. The initial27lint failures remain failed evidence. This review certifies neither the earlier helper implementation nor actual browser export/operator acceptance. Root's AST identity and rerun on the new helper hash remain separate execution proof. Exact source/config identities and reconstruction checks are in checks.json.

## Final F227 permanent test wrapping and scope check

Independent narrow review by Codex child glm_codec_merge. Result:0 supported defects,0 smells,0 nits. Root authored this final wrapper and scope prose. No tests, browser, model, native job or hardware ran here. Earlier complete LLD and helper implementation reviews are not repeated.

The permanent ESM file210f254c6a0b5e954dd9d159efa6182273ce7bcdda516ab57924045a756c0420 preserves every byte from function setup through all11 testcase functions relative to original controller-vm-test.cjs. Only module imports, source-relative helper loading and standard node:test registration replace CJS imports, command-line helper path and manual results/exit handling. Each actual case registers under its existing name. The relative URL resolves to the adopted helper, and no benchmark gate/CI list changes. Explicit Node and Python test commands are documented in their respective tool procedures. No automatic floor discovery is asserted.

The final plan write-set/scope paragraph accurately adds the independent acceptance tool, tests and caller-input helper as acceptance instruments, with no product endpoint, grouping, arithmetic, public API, tolerance or allocation change. The final LLD paragraph's42Python/11VM controls match supplied receipts and the inspected test inventory. Actual native positive/wrong-bit and Chromium download/CLI composition proofs are separately retained. Latest Chromium proof binds helperad613741, readera97225c2 and binary85b4 before/after, both modes CLI0, exact page order/source frames and manual-override closed refusal, with owned server stopped. These synthetic instrument checks are not operator study, first/middle/last/scroll/label observations or idle-host timing. Both new paragraphs say so explicitly. No AC16 completion is inflated from readiness.

Source identities, immutable testcase-byte comparison and bounded paragraph hashes are in final-test-packaging-checks.json. Root's actual11GREEN and final feature-profile results remain separate execution receipts.

## Final-profile integration findings and repairs

The feature floor at tree87f2ab131f87214cbfeaf90091eaae3c77f65373 failed bench, lint and guards, with26other gates passing. It remains a failed attempt. Root and the earlier bounded packaging reviewer had missed the exact runner-list requirement. This is a coordinator/reviewer integration miss, not a defect in the later GLM registration patch.

Root independently reviewed that two-file registration patch8fb8d731390470d72cc8548b6a6017f12ee56d0868b8760effcf9e593c35b98b. It appends the existing permanent Node suite once in both mandatory benchmark lists, preserves every old command and suite, and disables no test or gate. Its actual old-state paths test fails at the missing-suite assertion, repaired paths test passes, and the21-suite pure command reports345tests,339pass,0fail,6explicit browser opt-ins skipped. Root separately replaced the flagged standalone conditional expression with equivalent if/else statements, observed lint0 and all11controls0. This changes no acceptance assertion. The Node suite is now executed in the benchmark gate, superseding the earlier explicit-only description.

Root also independently reviewed the three-path scalar census repair: one declared offline-tool entry with all four refusal strings, exact permanent error assertions and the generated runbook row. No repository gate executes this Python calculator. Its not-a-guard classification follows that actual consumer boundary, with ownerF227 and a real explicitly invoked permanent test path. It creates no exemption for an existing gate and changes no existing claim, limit, search budget or arithmetic. The test uses an explicit module import recognized by the unchanged census, and actually calls that imported module. The2^63exhaustion fixture is derived from16upward steps spanning2^44 versus maximum candidate width2^24, so no allowed window can meet the strict half-grey check. Four wrong-return mutants fail because the expected ValueError is missing, rather than from setup. The actual scalar suite and census/runbook checks pass. Root's full42control invocation after the repair exits0 in0.576434seconds.

These are bounded source, semantic and target checks. The repaired final tree still requires a fresh complete feature profile. No passing receipt was recorded for the failed tree. Actual operator and idle-host observations remain separate.
