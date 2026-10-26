# F-X003 review, pass 11

**Reviewed**: staged tree `01c9201aa8e69e4337e71e7ecea726bdcfa9c3af` on `sprint/s16`, HEAD `22800f8c1b916818784c63b44dde1760c3512f04`. The supplied diff `/tmp/review-F-X003.diff` covers 17 paths. The staged tree carries 157 changed paths. This pass also reviewed `.claude/plans/F-X003-design.md` with its `Repair, 2026-10-25, display-code quantisation` section, plus `docs/lld/oracle.md`, `docs/lld/comparator.md` and `docs/lld/pixel-pipeline.md`, because the acceptance names them.
**Author / reviewer**: implementation by xAI and Codex. The reviewer is a separate participant and cannot supply their approval.
**Result**: 6 defects, 0 smells, 2 nitpicks.

## Acceptance challenged

- **AC1, independently derived tie.** PS3.3 C.11.2.1.3.2 at centre 40 and width 400 gives `((40 - 40) / 400 + 0.5) * 255 = 127.5`. The repaired `display_index` returns 128 because `rounded` is 128.0 and the search takes the greatest integer not above it. Truncation returns 127. `section_18_3_display_quantizes_to_the_nearest_grey_byte` asserts byte 128 for both LINEAR and LINEAR_EXACT, so a revert is rejected. The second discriminator is PS3.3 C.11.2.1.3.1 at stored -60, centre 40, width 400, which gives `255 / (1 + e) = 68.58`, so rounding gives 69 and truncation gives 68. `slope_and_monochrome1_and_sigmoid_use_the_chain_once` asserts 69.
- **AC2, independently derived DPR 1.25 case.** CSS 4 by 1 at dpr 1.25 gives a 5 by 1 device extent. Device centre 4.5 divided once by 1.25 is CSS 3.6, which is texel 3 for an unfitted identity view. Treating the device coordinate as CSS puts 4.5 outside a 4-column image. A second division gives 2.88 and texel 2. `dpr_one_and_a_quarter_uses_one_division` asserts stored 40 at pixel 4 and stored 10 at pixel 0, so either wrong behaviour is rejected. I also derived the fit, anisotropic spacing, shift, flip, quarter turn and zoom fixtures from `View::fit` and `index_maps`, and they agree with the asserted texels.
- **AC5, deliverable versus standing result.** The A/C bound is the deliverable. `tools/oracle/compare-out/tier-a-c-rounding/compare.json` has 67 pass, 0 fail and 17 unmeasured, and every per-view `renderHashes.reference` equals `renderHashes.candidate`. The aggregate hash is `95ac9185be70be670e29a62bdafeec2180f287785bf99658fa3aa50218d45aa4` on both sides, and maximum absolute difference is 0. External cornerstone parity is a separate receipt, `tier-c-rounding-gate`, at 39 pass, 28 fail and 17 unmeasured, still red. The plan and the LLD keep the two apart.
- **AC7, idle path.** A settled `publishCpuCanvas` whose raster returns `wrote == false` returns from `publish_after` before `blit_retained`, so no `putImageData` and no kind 6. The browser `idle` steps assert a put delta of 0 and no events.
- **Residual groups, separately attributed and not absorbed.** The register `tools/oracle/reference-divergence.json` has one entry, `sigmoid-width-below-one`, reachable, and it did not fire on `synthetic__ct_sigmoid_width_half` because `referenceDivergenceEntry` is null for that record. The three MR rows carry the `bias` qualifier with `biasPasses` false, which separates them from the 25 rows that fail the max-absolute predicate at 64 codes. The 15 weak or decimated views appear in `absorbedDivergences`, so the gate names them instead of excusing them.

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|--------------------------|---------------------------|------------------------------|----------------------|
| Read `/tmp/review-F-X003.diff` | 17 paths against staged tree `01c9201a` | Executed now | 9127 lines read, no command exit |
| Read `tier-c-rounding-gate/compare.json` | Tier C candidate against matched reference, Oct 25 | Prior receipt, read now | `comparison-failure`, 39 pass, 28 fail, 17 unmeasured |
| Read `tier-a-c-rounding/compare.json` | Tier C candidate as reference, tier A candidate, Oct 25 | Prior receipt, read now | `coverage-loss`, 67 pass, 0 fail, 17 unmeasured, identical hashes |
| Read `tier-c-vs-reference/compare.json` | Pre-repair tier C against reference | Prior receipt, read now | 0 pass, 67 fail, 17 unmeasured. The named CT row has informative mean -0.49102919698248715, 36,646 pixels at -1 and none at +1 |
| Read `codex-tier-a-c-final/compare.json` | Pre-repair tier A against tier C | Prior receipt, read now | 67 pass, 0 fail, 28 of 84 per-view hashes differ, maxAbs 1, max informative mean 0.0037249936353526015 |
| Computed column histogram of `synthetic__ct_series_uniform__slice_000.diff.raw` | Synthetic 512 by 512 row, image rectangle starting at column 43 | Executed now, read-only python3 | Absolute difference 64 only at canvas columns 42 and 469, with 341 and 384 pixels |
| Read vtk.js quantisation sites | `node_modules/@kitware/vtk.js/Rendering/Core/ColorTransferFunction.js` lines 494 to 497 and 701 to 703 | Executed now | `Math.floor(... * 255 + .5)` for the table build and for the RGB outputs |
| Read `crates/ocelli-viewport/src/commands.rs:279` | `StackViewport::apply_commands` | Executed now | Clones `self.presentation`, so it increments the descriptor clone counter |
| Read `crates/ocelli-render/src/cpu.rs` and `tests/cpu.rs` | Raster and fixtures | Executed now | Allocation counter only moves in `reserve_source` |

No test, gate, oracle run, GPU test, browser harness or formatter was executed. The GPU is exclusive and this pass did not use it.

## Defects

### D1, Clone-counter assertion can fail while the draw is correct
**Where**: `crates/ocelli-wasm/src/session.rs:1442-1450`
**What**: `draw_and_idle_do_not_clone_lut_buffers` reads the process-global `LUT_DESCRIPTOR_CLONES` counter, draws twice, and asserts the counter is unchanged. `LutDescriptor::clone` increments that atomic from any thread. `StackViewport::apply_commands` clones the presentation at `crates/ocelli-viewport/src/commands.rs:279`, and `a_voi_sequence_batch_returns_zero_and_the_fit_does_not_stick` in the same test binary builds a presentation with two sequence descriptors and calls `apply_commands`.
**Why it is wrong**: the `test` gate is `cargo test --workspace` with default threading, so the two tests run on separate threads in one process. The sibling's clone can land between the `before` read and the assertion, and the assertion then fails although the draw cloned nothing. That is a false red, which is the mirror of the microscope's rule that a test must fail when the code is wrong.
**Evidence**: read of `crates/ocelli-pixel/src/lut.rs:41` (the increment), `commands.rs:279` (the clone), and both tests. Not reproduced, because a race needs repeated runs and this pass does not run suites.
**Scope and regression**: `ocelli-wasm` library tests only. Smallest regression: make the counter thread-local and add a control that a clone on a second thread does not change the drawing thread's count.

### D2, Fixtures still claim the preset tables truncate
**Where**: `crates/ocelli-render/tests/cpu.rs:4-6`
**What**: the module doc says "Grey and Stops index the existing preset tables, which truncate the address toward zero", and `assert_grey_near` at lines 170 to 173 repeats that the table "truncates toward zero, so the channel may be one below `stored`". After the repair, `display_index` rounds to the nearest code, so both sentences are false. A LINEAR_EXACT identity display within `f32` error of `stored` rounds to `stored`, never to `stored - 1`.
**Why it is wrong**: `crates/ocelli-render/src/colour.rs:363-380` and the plan both state the opposite. This is the file whose fixtures are the repair's evidence, and the comment was carried from the pre-repair code.
**Evidence**: read of the rounded search and the two doc blocks.
**Scope and regression**: documentation only, so no test can catch it. Delete the stale sentences rather than assert them.

### D3, The AC7 allocation assertion cannot reject the named wrong implementation
**Where**: `crates/ocelli-render/tests/cpu.rs:163-164`
**What**: the `draw` helper asserts `raster.allocations()` is unchanged across a settled mono draw. `allocations` is incremented only by `CpuRaster::reserve_source` at `crates/ocelli-render/src/cpu.rs:239`, and `raster_into` never calls it, so the assertion cannot fail. A `Vec` allocated inside `shade` or `rgba_of_preset` would leave the counter unchanged and the test green.
**Why it is wrong**: the design's AC7 row names "A `Vec` grown inside the pixel loop" as the wrong implementation the counter must reject. `lut_descriptor_clones` covers descriptor clones and the browser counters cover JS constructors, but no instrument covers a Rust heap allocation in the pixel loop.
**Evidence**: read of `cpu.rs:154`, `cpu.rs:239` and the `raster_into` call graph.
**Scope and regression**: AC7's mono allocation claim rests on inspection, not on the named instrument. Smallest regression: count allocations with a test-only global allocator across one settled draw.

### D4, Repair record says the 67 failures came only from the signed-mean bound
**Where**: `.claude/plans/F-X003-design.md:479-480`
**What**: "67 views fail the PS3.3 C.11 chain only through HLD 25.1's signed-mean bound of `0.1`". The pre-repair receipt `tools/oracle/compare-out/tier-c-vs-reference/compare.json` has 67 fails, of which 25 also fail the max-absolute predicate with maximum absolute difference 64. One of them, `synthetic__mr_nonsquare_spacing`, has `biasPasses` true and fails only the predicate.
**Why it is wrong**: HLD 25.1's 99.9 percent within one LSB rule and its signed-mean bound are separate tests. The sentence understates the residual groups and implies the rounding repair should close all 67. The measured outcome is 39 closed by rounding, 25 left by the fit-boundary and nonsquare groups, and 3 left by a residual 0.17 bias.
**Evidence**: read of the pre-repair receipt, counting fails with `predicatePasses == false`.
**Scope and regression**: design record only. Expected: 39 views failed only the signed-mean bound, 25 failed the max-absolute predicate, and 3 retain a residual bias.

### D5, Repair record cites byte identity that the named pre-repair receipt contradicts
**Where**: `.claude/plans/F-X003-design.md:486-487`
**What**: "Tier A and tier C agree byte for byte, so this is one shared defect in the quantisation and not a tier divergence". The bullet sits under "Measured against `codex-tier-c-final/candidate` and its matched reference". The pre-repair A/C receipt `tools/oracle/compare-out/codex-tier-a-c-final/compare.json` has aggregate render hashes `3a6499af5ec45f10c63db5307fbff858eb59447be439c02838cb484b74469ba1` and `03dd21dc717b9db3cb9f0b8a3bfe3fff478f537793a6b430c1ff330f64c6eec7`, 28 of 84 per-view hashes differing, maximum absolute difference 1, and maximum informative signed mean 0.0037249936353526015. Byte identity holds only for the post-repair `tier-a-c-rounding` run.
**Why it is wrong**: the sentence is offered as evidence that the defect was shared. The pre-repair one-code difference was itself a boundary artefact that the rounding absorbed, so the receipt a reader would check does not support the claim.
**Evidence**: read of both receipts.
**Scope and regression**: design record only. Expected: pre-repair tier A and tier C differed by at most one code on 28 of 84 frames, and are byte-identical after the repair.

### D6, LLD states a current cornerstone failure count that the repair changed
**Where**: `docs/lld/oracle.md:1224-1225`
**What**: the section says "The cornerstone comparison remains red with 67 pixel failures and 17 unmeasured views". The post-repair receipt `tools/oracle/compare-out/tier-c-rounding-gate/compare.json` is 39 pass, 28 fail and 17 unmeasured.
**Why it is wrong**: `docs/lld/comparator.md` requires prose numbers to be updated in the same change as the measurement. The repair changed the count from 67 to 28 and did not update this passage, so a reader taking the LLD as the current status is told the wrong number.
**Evidence**: read of the LLD and the post-repair receipt.
**Scope and regression**: normative LLD. Expected: 28 pixel failures and 39 passes, with the receipt named.

## Smells

None found.

## Nitpicks

### N1, `RasterError::Canvas` carries two meanings
**Where**: `crates/ocelli-render/src/cpu.rs:52`
**What**: the variant doc says "CSS size or the device pixel ratio was refused", and the Display string is "cpu raster canvas size was refused". `Session::draw_cpu_mono`, `draw_interaction`, `cpu_device`, `defer_cpu_presentation` and `accept_cpu_presentation` also return it when no raster is bound or no frame has been committed.
**Why it is worth noting**: a caller reading the message is told the size was refused when the real cause is a missing bind. A separate variant or a neutral message would keep the diagnostic honest.

### N2, The publication generation check cannot fire
**Where**: `crates/ocelli-wasm/src/canvas.rs:920` and `:935`
**What**: `accept_staged` and `publish_staged` compare `session.viewport().generation()` with `staged.generation`. The stage closure already wrote the drawn generation to the viewport before returning, and the accept runs in the same wasm call with only the JS blit between them, so the mismatch branch cannot fire.
**Why it is worth noting**: a reader may believe publication rejects stale frames, when the rejection actually happens in `CpuRaster::gate` as `RasterError::Stale`. The branch is harmless, and a debug assertion would say the same thing without implying a live guard.

## Verified clean

- **Quantisation.** `display_index` and the WGSL `table_index` implement the same rule for every input. Both clamp a non-finite value to 0, both clamp values at or below 0 to 0, both clamp values at or above `last` to `last`, and both add 0.5 before taking the greatest integer not above the sum. The binary search in `display_index` converges to that integer for `last` 0, 1, 2, 3 and 255. The palette path adds 0.5 to an integral stored index, which is a no-op, so palette addressing is unchanged.
- **Reference rounding.** The pinned vtk.js builds its colour table and its RGB outputs with `Math.floor(... * 255 + .5)`. The reference rounds, so the repair's premise holds.
- **Pre-repair signature.** The receipt matches the stated signature on the named CT row, including the absence of a +1 bucket, and tier C moved from 0 pass, 67 fail, 17 unmeasured to 39 pass, 28 fail, 17 unmeasured.
- **Tier A against tier C.** All 84 produced frames are byte-identical, the aggregate hash matches on both sides, maximum absolute difference is 0, and the maximum informative signed mean is 0.0. The `coverage-loss` verdict comes only from the shared census, which was not edited.
- **Residual separation.** The three MR rows carry the `bias` qualifier, the 25 max-absolute failures are distinguishable by `predicatePasses` and by a 64-code maximum, the 15 weak or decimated views are named in `absorbedDivergences`, and the single register entry did not fire. The fit-boundary claim is true for the synthetic row I histogrammed: the 64-code pixels are exactly the two columns immediately outside the image rectangle.
- **Texel selection.** `canvas_to_index` returns a centre-based index, `IMAGE_FROM_INDEX` converts it to the corner convention the shader floors, and `texel_of` rejects a negative or at-or-past-extent image coordinate. The CPU and GPU paths therefore agree at identity, DPR 1.25, DPR 2, fit, anisotropic spacing, measured shift, flip, quarter turn and zoom.
- **VOI arithmetic.** LINEAR uses `c - 0.5` and `w - 1` with `<=` at the low bound and `>` at the high bound. LINEAR_EXACT uses neither. SIGMOID divides by `w`. Width validation is per function.
- **AC3.** Grey and Stops index the existing preset tables, palette and RGB go through `ColorTransform` once, ICC and every other unsupported space return `Unavailable` before any write, and no colour threshold was added.
- **AC4.** Interaction sets refining, a stale completion returns `Stale` and leaves refining set, a settle draws only the current generation, and an idle settle writes nothing. The native tests and the browser harness cover the mono and RGB directions from settled and invalidated states.
- **AC6.** The plan and the LLD keep it unmet. No host, budget or threshold is invented, and the runners record no sample.
- **Boundary.** No new `wasm_bindgen` outside `ocelli-wasm`. No pixel getter and no `ring_ptr`. No new unsafe. One `queue.submit` per frame in the native GPU candidate. The producer refuses paths that escape the corpus root, identifiers outside the allowed set, and output directories that overlap the reference run.

## Limitations and handoff

- Not exercised: no test, gate, oracle run, GPU test, browser harness or formatter. No adapter was opened. The GPU-side rounding is evidenced only by the post-repair A/C byte identity and by ignored GPU tests that this pass did not run.
- The supplied diff covers 17 of the 157 staged paths. This pass read the plan and the LLDs outside it. The remaining staged paths still owe review.
- The SIGMOID width-half row and the fit-boundary group remain unresolved. The register entry is reachable but did not fire.
- AC6 remains unmet. AC5's coverage gate remains red by design because the census was not edited. AC8's runners record no sample.
- `docs/lld/oracle.md` carries an older F-X012 record that says the presented reference tracked the width-0.5 curve and "does not claim nearest-integer rounding", while the later S16 diagnosis says the reference followed a width-1.5 curve. That contradiction predates this patch and should be reconciled when the SIGMOID row is closed.
- F-X003 stays in progress. This review does not approve the change and cannot substitute for the implementers' approval.
