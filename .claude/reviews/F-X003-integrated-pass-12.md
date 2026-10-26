# F-X003 review, pass 12

**Reviewed**: staged tree `3825b0ca79ef7923b92969036e608e4df00b3b9e` on `sprint/s16`, HEAD `22800f8`. The supplied diff `/tmp/review-F-X003-p12.diff` covers 20 paths and 9866 lines. This pass reviewed the plan repair section, `docs/lld/oracle.md`, the `ocelli-wasm` session test module, the viewport command clone site, the render CPU loop and its fixtures, and the four oracle receipts the acceptance names.
**Author / reviewer**: implementation by xAI and Codex. The reviewer is a separate participant and cannot supply their approval.
**Result**: 2 defects, 0 smells, 3 nitpicks.

## Acceptance challenged

- **AC7, independently derived instrument.** The plan says HLD 27.2 R5's two-file unsafe allow-list rules out a global allocator, so the loop's allocation freedom is by inspection. `.claude/reviews/F-031-implementation-pass-1.md:26` and pass 2 line 42 record a counting `GlobalAlloc` in a temporary integration test, created, run and deleted, as accepted evidence. The `draw` helper at `crates/ocelli-render/tests/cpu.rs:144` and `:164` reads and re-reads `raster.allocations()`, but `allocations` is incremented only by `reserve_source` at `crates/ocelli-render/src/cpu.rs:243`, and `raster_into` never calls it. A `Vec` pushed inside `shade` would leave the counter at zero and the helper green. The discriminating case is a temporary counting `GlobalAlloc` over one settled 1 by 1 mono draw, which would go red on that mutation. The plan's claim is false and the AC7 instrument is unresolved.
- **Clone lock, independently derived coverage.** `crates/ocelli-wasm/src/session.rs:1109-1112` says every test that can clone a presentation takes `CLONE_LOCK`. `committed_rgb_replicates_the_reduced_sample_and_settles_latest` at `:1612` and `a_rejected_colour_request_keeps_the_monochrome_route` at `:1780` call `invert_committed` at `:1904-1910`, which calls `Session::apply_commands` and then `StackViewport::apply_commands` at `crates/ocelli-viewport/src/commands.rs:279`, which clones `self.presentation`. Neither test takes `clone_guard()`. The identity presentation from `StackViewport::try_identity` has no `LutDescriptor`, so `LutDescriptor::clone` does not run and the counter stays put today. The claim is still false, and a descriptor added to either test would reintroduce the false red in `draw_and_idle_do_not_clone_lut_buffers`.
- **The four closed findings were challenged against the receipts.** The 42 only-bias, 24 both and one only-predicate split matches `tier-c-vs-reference`. The 28 of 84 one-code pre-repair frames and the zero post-repair differences match `codex-tier-a-c-final` and `tier-a-c-rounding`. The 39 pass, 28 fail and 17 unmeasured split matches `tier-c-rounding-gate`, and no stale 67 failure sentence remains in the LLD.

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|--------------------------|---------------------------|------------------------------|----------------------|
| Read `/tmp/review-F-X003-p12.diff` | 20 paths against staged tree `3825b0ca` | Executed now | 9866 lines read, no command exit |
| Read `.claude/plans/F-X003-design.md` repair section | Display-code quantisation, AC7 row and paragraph | Executed now | The R5 justification at lines 526-533 is false, the AC7 row at line 203 still names the counter as the observation |
| Read `crates/ocelli-wasm/src/session.rs` test module | Six guards, two unlocked `invert_committed` callers, `CLONE_LOCK` comment | Executed now | Guards at 1246, 1285, 1411, 1847, 1914 and 2089, no guard at 1612 or 1780 |
| Read `crates/ocelli-viewport/src/commands.rs:279` and `try_identity` | `StackViewport::apply_commands`, identity presentation | Executed now | `apply_commands` clones `self.presentation`, identity has `VoiTransform::new(None, ...)` so no `LutDescriptor` |
| Read `crates/ocelli-pixel/src/lut.rs:40-43` | `LutDescriptor::clone` | Executed now | Increments `LUT_DESCRIPTOR_CLONES` only when a descriptor is cloned |
| Read `crates/ocelli-render/tests/cpu.rs` | Module doc, `draw` helper, `assert_grey_near`, fixture comments | Executed now | Nearest-code prose at lines 4-6 and 168-173, historical truncation only at line 210, allocation assertion at lines 144 and 164 |
| Read `crates/ocelli-render/src/cpu.rs` | `reserve_source`, `raster_into`, `shade`, `rgba_of_preset`, `RasterError::Canvas` | Executed now | `allocations` moves only at line 243, `raster_into` never calls `reserve_source`, the loop creates no `Vec`, `Canvas` still has two meanings |
| Read `docs/hld/24-agent-code-standards.md:50` | HLD 27.2 R5 | Executed now | Unsafe permitted in `ocelli-wasm/src/ring.rs` and `ocelli-core/src/cast.rs`, not a general prohibition |
| Read `.claude/reviews/F-031-implementation-pass-1.md:26` and pass 2 line 42 | Counting `GlobalAlloc` temporary test | Prior receipt, read now | A counting `GlobalAlloc` in a temporary integration test was created, run and deleted as accepted evidence |
| Read `tools/oracle/compare-out/tier-c-vs-reference/compare.json` | Pre-repair tier C against reference | Prior receipt, read now | 67 fail, 42 only bias, 24 both, 1 only predicate, CT row informative mean -0.49102919698248715 with 36,646 at -1 and 37,985 at 0 |
| Read `tools/oracle/compare-out/codex-tier-a-c-final/compare.json` | Pre-repair tier A against tier C | Prior receipt, read now | 67 pass, 0 fail, 17 unmeasured, 28 of 84 per-view hashes differ, maxAbs 1, maximum informative mean 0.0037249936353526015, aggregate hashes 03dd21dc and 3a6499af |
| Read `tools/oracle/compare-out/tier-a-c-rounding/compare.json` | Post-repair tier A against tier C | Prior receipt, read now | 67 pass, 0 fail, 17 unmeasured, 0 differing hashes, maxAbs 0, aggregate hash 95ac9185 on both sides |
| Read `tools/oracle/compare-out/tier-c-rounding-gate/compare.json` | Post-repair tier C against reference | Prior receipt, read now | 39 pass, 28 fail, 17 unmeasured, 24 only predicate, 3 only bias at about 0.17, 1 both |
| Read `docs/lld/oracle.md:1221-1231` | Current cornerstone and A/C status | Executed now | 39 passes, 28 failures and 17 unmeasured, `tier-c-rounding-gate` named, 67 is the A/C pixel-pass count, no stale 67 failure |
| Read `crates/ocelli-wasm/src/canvas.rs:916-940` | Staged publication generation check | Executed now | The branch remains unreachable because the stage closure already wrote the generation |

No test, gate, oracle run, GPU test, browser harness or formatter was executed. The GPU is exclusive and this pass did not use it.

## Defects

### D1, Clone lock does not cover every `apply_commands` test
**Where**: `crates/ocelli-wasm/src/session.rs:1109-1112`, `:1612`, `:1780`, `:1904-1910`
**What**: The comment says every test that can clone a presentation takes the lock. `committed_rgb_replicates_the_reduced_sample_and_settles_latest` and `a_rejected_colour_request_keeps_the_monochrome_route` call `invert_committed`, which calls `apply_commands`, which clones the presentation at `crates/ocelli-viewport/src/commands.rs:279`, without `clone_guard()`. The identity presentation has no `LutDescriptor`, so the descriptor counter does not move today. A descriptor added to either test would make `draw_and_idle_do_not_clone_lut_buffers` see a sibling clone and fail while the draw is correct.
**Why it is wrong**: the stated invariant is false. The lock is not taken by every test that calls `apply_commands`, which is the coverage this pass was asked to check. The counter is deterministic only because those two presentations happen to have no descriptor.
**Evidence**: read of the six guards, the two tests, `invert_committed`, `try_identity`, `LutDescriptor::clone` and `StackViewport::apply_commands`.
**Scope and regression**: `ocelli-wasm` library tests. Smallest regression: a source-level test that every `#[test]` fn reaching `apply_commands`, directly or through `invert_committed`, takes `clone_guard()`. The fix is `let _clone_guard = clone_guard();` at the top of both tests.

### D2, AC7 instrument is unresolved and its R5 justification is false
**Where**: `.claude/plans/F-X003-design.md:526-533`, `crates/ocelli-render/tests/cpu.rs:144` and `:164`
**What**: The plan says HLD 27.2 R5's two-file unsafe allow-list rules out a global allocator, so no runtime instrument can see a heap allocation the loop does not route through `reserve_source`. That is false. F-031 pass 1 and pass 2 used a counting `GlobalAlloc` in a temporary integration test, created, run and deleted, and the gate reads `git ls-files` so the temporary test was never scanned. R5 also permits unsafe in `ocelli-core/src/cast.rs`, so a permanent instrument in an allowed file is not ruled out. The `draw` helper still asserts `allocations()` is unchanged across a mono `raster_into`, but `raster_into` never calls `reserve_source`, so the assertion cannot fail and cannot reject a `Vec` in `shade` or `rgba_of_preset`. The AC7 row at line 203 still names the counter as the observation and the `Vec` as the wrong implementation rejected, while the repair paragraph says it is not mechanically rejected.
**Why it is wrong**: the AC7 row names a `Vec` grown inside the pixel loop. The plan's limitation is justified by a false claim, and the test still asserts an invariant that passes both ways. The microscope treats a test that passes both ways as a defect, not a nitpick.
**Evidence**: read of the plan paragraph and row, the `draw` helper, `reserve_source`, `raster_into`, `shade` and `rgba_of_preset`, HLD 24 section 27.2 R5, and the F-031 reviews.
**Scope and regression**: AC7's mono allocation claim. Smallest regression: a temporary counting `GlobalAlloc` integration test over one settled mono draw, as F-031 did, which goes red when a `Vec` is pushed inside `shade`. Record the count and delete the test, or add a permanent counter in an allowed file. The expected is that the plan records that route or implements it. The actual is a false R5 justification and a vacuous assertion.

## Smells

None found.

## Nitpicks

### N1, `RasterError::Canvas` carries two meanings
Carried from pass 11, still not blocking. `crates/ocelli-render/src/cpu.rs:52` says the variant is a refused CSS size or device pixel ratio, while the session returns it at `:455`, `:589`, `:651`, `:688`, `:700`, `:711`, `:726`, `:745` and `:759` when no raster is bound or no frame has been committed.

### N2, The publication generation check cannot fire
Carried from pass 11, still not blocking. `crates/ocelli-wasm/src/canvas.rs:920` and `:935` compare the viewport generation with `staged.generation`, but the stage closure already wrote the drawn generation before returning and the accept runs in the same wasm call with only the JS blit between them.

### N3, The plan says the corrected fixtures cite D-11
The fixture at `crates/ocelli-render/tests/cpu.rs:205-210` cites PS3.3 C.11.2.1.2 and C.11.2.1.3.2, HLD 25.1 and the pinned reference, but not D-11 by name. Add the deviation ID to the fixture comment or reword the plan.

## Verified clean

- **D2 closed.** `crates/ocelli-render/tests/cpu.rs` module doc at lines 4-6 and `assert_grey_near` at lines 168-173 say the preset table rounds to the nearest code, ties up. The only remaining truncation sentence is the historical line 210, "the one-code truncation that used to sit here", which is true.
- **D4 closed.** `tier-c-vs-reference` has 67 fails, 42 with `biasPasses` false and `predicatePasses` true, 24 with both false at maxAbs 64, and one, `synthetic__mr_nonsquare_spacing`, with `biasPasses` true and `predicatePasses` false. The plan's split is exact.
- **D5 closed.** `codex-tier-a-c-final` has 28 of 84 per-view hashes differing, maxAbs 1 and maximum informative signed mean 0.0037249936353526015. `tier-a-c-rounding` has 0 differing hashes, maxAbs 0 and the aggregate hash `95ac9185be70be670e29a62bdafeec2180f287785bf99658fa3aa50218d45aa4` on both sides. The plan's claim is exact.
- **D6 closed.** `docs/lld/oracle.md:1227-1229` says 39 passes, 28 pixel failures and 17 unmeasured views, and names `tools/oracle/compare-out/tier-c-rounding-gate/compare.json`. The 67 at line 1223 is the A/C run's pixel-pass count, not a failure count. No stale 67 failure sentence remains.
- **D1 partial.** The descriptor counter is deterministic today. The only descriptor-cloning `apply_commands` test, `a_voi_sequence_batch_returns_zero_and_the_fit_does_not_stick`, takes the lock at line 1285. The other `apply_commands` tests use the identity presentation, which has no `LutDescriptor`, so `LutDescriptor::clone` does not run. The lock coverage claim is still false, as D1 records.
- **D3 unresolved.** The loop is allocation-free by inspection. `shade` writes into `out` with `write_pixel`, `rgba_of_preset` uses a stack `[Rgb; 1]`, and `map_preset_into` writes into it. No `Vec` is created in the loop. The instrument and its justification are still wrong, as D2 records.
- **Quantisation, tier A/C, residual separation, texel selection, VOI arithmetic, AC3, AC4, AC6 and the boundary checks** are unchanged from pass 11 and were verified clean there. This pass re-read the plan and the receipts and found no new contradiction in those rows.
- **AC7 idle path.** A settled `publishCpuCanvas` whose raster returns `wrote == false` returns from `publish_after` before `blit_retained`, so no `putImageData` and no kind 6.

## Limitations and handoff

- Not exercised: no test, gate, oracle run, GPU test, browser harness or formatter. The GPU is exclusive and this pass did not use it.
- The diff covers 20 paths. The staged tree is `3825b0ca79ef7923b92969036e608e4df00b3b9e`. The prior pass's clean items are carried, not re-executed.
- The two defects remain. AC6 remains unmet. AC5's coverage gate remains red by design because the census was not edited. AC8's runners record no sample. The SIGMOID width-half row and the fit-boundary group remain unresolved. The F-X012 contradiction in `docs/lld/oracle.md` remains.
- F-X003 stays in progress. This review does not approve the change and cannot substitute for the implementers' approval.
