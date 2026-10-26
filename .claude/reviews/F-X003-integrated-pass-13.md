# F-X003 review, pass 13

**Reviewed**: staged tree `e7254654cd94f7016395e8fa600ef8668067d078` on `sprint/s16`, HEAD `22800f8`. The supplied diff `/tmp/review-F-X003-p13.diff` covers 21 paths and 9956 lines. `git write-tree` returns the reviewed tree, so the working tree and the index agree. The remediation between pass 12 tree `3825b0ca79ef7923b92969036e608e4df00b3b9e` and this tree touches four paths: `.claude/plans/F-X003-design.md`, `.claude/reviews/F-X003-integrated-pass-12.md`, `crates/ocelli-render/tests/cpu.rs` and `crates/ocelli-wasm/src/session.rs`. This pass reviewed the three product and plan changes, the clone-lock coverage claim, the AC7 instrument record and the fixture citation.

**Author / reviewer**: implementation by xAI and Codex. The reviewer is a separate participant and cannot supply their approval. This pass reviewed remediation written by the implementers, not by the reviewer.

**Result**: 0 defects, 0 smells, 2 nitpicks.

## Acceptance challenged

- **AC7, independently derived allocation case.** A settled mono draw with `Colormap::Stops` on a 2 by 1 device calls `rgba_of_preset` twice. The function holds a one-element `Rgb` array on the stack, and `map_preset_into` holds a one-element `Display` array on the stack, so the pixel loop allocates nothing on either preset. An idle settled draw returns from `gate` before `write_mono`, so `rgba_of_preset` is not called at all. A device pixel ratio change that keeps the same device extent calls `sync_cpu_buffer`, which compares lengths and does not allocate. These three derived cases agree with the plan's claim that the probe measured zero allocations and with the statement that the permanent counter covers only reservation growth.
- **Clone lock, independently derived coverage.** Every `Session::apply_commands` call site in the test module is at lines 1254, 1271 and 1276 in `apply_commands_uses_the_slot_once_and_a_refusal_returns_zero`, at 1338 in `a_voi_sequence_batch_returns_zero_and_the_fit_does_not_stick`, at 1911 inside `invert_committed`, at 1959 in `configured_stack_rejects_neighbours_and_old_epochs_without_publishing`, and at 2111, 2136 and 2146 in `a_successful_command_or_frame_invalidates_the_settled_cpu_frame`. `invert_committed` is called only at 1749 in `committed_rgb_replicates_the_reduced_sample_and_settles_latest` and at 1821 in `a_rejected_colour_request_keeps_the_monochrome_route`. All six tests take `clone_guard()` at their first statement. The identity presentation has `None` for both LUT selections, so `LutDescriptor::clone` does not run there. The only two tests that construct a `LutDescriptor` are at 1284 and 1410, and both are guarded. The counter cannot move outside the lock.
- **N3, independently derived citation.** `docs/hld/DEVIATIONS.md:203-209` says the pinned version is the definition of correct for this project. The fixture comment at `crates/ocelli-render/tests/cpu.rs:208-211` now cites deviation D-11 for that role, so the citation is accurate.

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|---|---|---|---|
| `git diff --stat 3825b0ca e7254654` | pass 12 tree against pass 13 tree | Executed now | Four paths changed, 94 insertions and 10 deletions |
| `git diff 3825b0ca e7254654` over the three remediation paths | Plan AC7 row and paragraph, fixture comment, two test guards | Executed now | The AC7 row names the storage counter and the probe. The R5 sentence is deleted. The guards are at 1613 and 1782. The fixture cites D-11 |
| `grep GlobalAlloc` over the tree | All tracked and untracked text | Executed now | Only prose mentions in `.claude/plans/F-X003-design.md` and `.claude/reviews/F-031-implementation-pass-1.md` and pass 2. No allocator implementation |
| `git ls-tree -r e7254654 crates/ocelli-render/tests` | Staged render test targets | Executed now | No probe file. `tests/cpu.rs` is the only CPU target |
| Read `crates/ocelli-wasm/src/session.rs:1094-1120` and every `#[test]` | Eight guards and all `apply_commands` call sites | Executed now | Guards at 1246, 1285, 1411, 1613, 1782, 1849, 1916 and 2091. No unguarded call site |
| Read `crates/ocelli-render/src/cpu.rs:238-246,523-560,650-680` | `reserve_source`, `write_mono`, `shade`, `rgba_of_preset` | Executed now | The counter moves only in `reserve_source`. The mono loop uses stack arrays and no `Vec` |
| Read `crates/ocelli-render/src/colour.rs:190-232` | `map_preset_into` | Executed now | Stack `[Display(0.0)]`, no allocation |
| Read `docs/hld/24-agent-code-standards.md:50` | HLD 27.2 R5 | Executed now | Unsafe permitted in `ocelli-wasm/src/ring.rs` and `ocelli-core/src/cast.rs` |
| Read `scripts/unsafe_allowlist_check.py:80-110` | R5 scan inventory | Executed now | `git ls-files`, so an untracked probe is invisible |
| `bin/ocelli.sh test ocelli-render --test cpu` | 14 CPU raster tests on the reviewed tree | Executed now | 14 passed, 0 failed, exit 0 |
| `git status --porcelain` and `git write-tree` | Working tree against reviewed tree | Executed now | Only staged changes, tree equals `e7254654` |

No oracle run, no `#[ignore]` GPU test, no `gate --sprint`, no mutation of product source and no formatter were executed. The GPU is exclusive and this pass did not use it.

## Defects

None. Both prior defects are closed, as recorded under Verified clean.

## Smells

None.

## Nitpicks

### N1, `RasterError::Canvas` carries two meanings
Carried from pass 11 and pass 12, still not blocking. `crates/ocelli-render/src/cpu.rs:50` says the variant is a refused CSS size or device pixel ratio, while `crates/ocelli-wasm/src/session.rs` returns it at 458, 499, 549, 657, 693, 715, 735, 748, 762, 775, 790, 805 and 835 when no raster is bound or no frame has been committed.

### N2, The publication generation check cannot fire
Carried from pass 11 and pass 12, still not blocking. `crates/ocelli-wasm/src/canvas.rs:920` and `:935` compare the viewport generation with `staged.generation`, but the stage closure already wrote the drawn generation before returning and the accept runs in the same wasm call with only the JS blit between them.

## Verified clean

- **D1 closed.** `committed_rgb_replicates_the_reduced_sample_and_settles_latest` takes `clone_guard()` at `crates/ocelli-wasm/src/session.rs:1613`, and `a_rejected_colour_request_keeps_the_monochrome_route` takes it at `:1782`. The other four tests that reach `Session::apply_commands` were already guarded at 1246, 1285, 1916 and 2091. `StackViewport::apply_commands` at `crates/ocelli-viewport/src/commands.rs:279` is the only presentation clone site in `ocelli-wasm` and `ocelli-viewport`. The identity presentation from `StackViewport::try_identity` at `commands.rs:139-146` has `None` for both LUT selections, so no descriptor clone occurs there. The lock comment at `session.rs:1109-1112` is true for this test module. The descriptor counter is deterministic because every test that can increment it holds the lock.
- **D2 closed.** The R5 sentence is deleted from `.claude/plans/F-X003-design.md`. The AC7 row at line 203 names the storage counter and a counting-allocator probe over one settled draw, and the evidence column says the probe is temporary, recorded and deleted, as F-031 did. The paragraph at lines 526-532 says the permanent counter covers backing-storage reservation and that the pixel loop's allocation freedom was measured by the temporary probe, with zero allocations observed and a `Vec` inside `rgba_of_preset` making it red with "settled draw allocated 1". That mutation route is real, because `write_mono` calls `rgba_of_preset` for every target pixel. The probe file is absent from the staged tree. The permanent `draw` helper assertion at `crates/ocelli-render/tests/cpu.rs:123-144` is now presented as the storage check only, not as the AC7 control.
- **N3 closed.** The fixture comment at `crates/ocelli-render/tests/cpu.rs:208-211` cites deviation D-11, and `docs/hld/DEVIATIONS.md` gives that deviation the role the comment names.
- **The storage assertion was executed.** `bin/ocelli.sh test ocelli-render --test cpu` ran all 14 CPU raster tests on the reviewed tree. They passed, including `section_18_3_display_quantizes_to_the_nearest_grey_byte` and the tests that use the `draw` helper's `raster.allocations()` assertion.
- **The R5 claim is true.** `scripts/unsafe_allowlist_check.py` enumerates `git ls-files`, so a probe that was never tracked and is now deleted cannot reach the scan. No `GlobalAlloc` implementation remains in the tree.
- **F-031 precedent.** `.claude/reviews/F-031-implementation-pass-1.md:26` and pass 2 line 42 record a counting `GlobalAlloc` in a temporary integration test, created, run and deleted. The AC7 record follows that method.
- **Carried clean items.** Quantisation, tier A/C, residual separation, texel selection, VOI arithmetic, AC3, AC4, AC6 and the boundary checks are unchanged since pass 11 and were verified clean there. The pass 12 receipts for `tier-c-vs-reference`, `codex-tier-a-c-final`, `tier-a-c-rounding` and `tier-c-rounding-gate` were not re-executed and remain prior receipts.

## Limitations and handoff

- Not exercised: no oracle run, no GPU test, no browser harness, no `gate --sprint`, no formatter and no mutation of product source or plan. The review constraints forbid editing those files, so the D1 and D2 original triggers could not be replayed. The probe claim is a historical receipt that cannot be re-run from the tree, and this pass verified it by inspection and by the executed storage assertion, not by re-running the allocator probe.
- Only one focused test was run, `ocelli-render --test cpu`. The `ocelli-wasm` session tests were not executed, so the guard placement is verified by source inspection only.
- AC6 remains unmet, AC5's coverage gate remains red by design because the census was not edited, AC8's runners record no sample, the SIGMOID width-half row and the fit-boundary group remain unresolved, and the F-X012 contradiction in `docs/lld/oracle.md` remains.
- F-X003 stays in progress. This review does not approve the change and cannot substitute for the implementers' approval.
