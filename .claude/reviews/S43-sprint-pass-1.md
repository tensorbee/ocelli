# S43 review, pass 1

**Reviewed**: sprint/s43 at `fa655ed`, tree `5dee84c7171e53c15e0d40856cc648458e29b5a4`, diff against main `054bbff`.
**Author / reviewer**: implementation by the sprint host. Review by Claude Code, read only, with no shell.
**Result**: 1 defect, 4 smells, 4 nitpicks

## Acceptance challenged

- **F-195.** AC1 to AC10 were checked against `calibration.rs`, `stats.rs`, the route and the hand fixtures. Imager 0.2 and 0.4 divided by 2 gives 0.1 and 0.2 bit for bit. Multiplying would give 0.4 and 0.8. `5 * 1.8` rounds to exactly 9.0, so SUVbsa 90 compares exactly. SUVlbm is 350. `suv_bw` keeps its order of checks. `elapsed_seconds < 0.0` still accepts negative zero. The only new check is that `scale` must be finite and positive, and that is a constant.
- **F-196.** AC1 to AC5 were checked against `tid2010.ts`, `secondary-capture.ts` and their tests. The test counts in AS_BUILT are right. Six and five.
- **F-197.** `prepare` is the old `assemble_from` body. The duplicate test, the sort and the 0.005 rule are unchanged. Only the shear arm now branches. The tilted fixture gives lateral `(3, 0, 0)` and projected spacing 3. The test counts are right. Ten unit tests and two route tests. AS_BUILT says 3.031 is refused and 3.02 is kept. That holds when worked out. Median 3.0155 against departure 0.0155, then median 3.01 against departure 0.01.
- **F-198.** `loadCore` sets the flag only after the initialiser returns. The worker posts a status and an accepted count, and never pixels. `holdSessionBytes` never calls `commit_frame`. AC5 is met for a synthetic one-sample frame. It is not met for what the sprint's "What done means" promises. See D1.
- **Cross-story.** Both new routes are registered in `route`, `open_path`, `body_messages` and the vite proxy. `App` now takes `ready` from the worker's status and no longer calls `coreAvailable()` on the main thread. That fits the README sentence. No story touches another story's arithmetic.

## Evidence

| Check | Source | Executed now / prior receipt | Observation |
|---|---|---|---|
| `gate --sprint` | tree 5dee84c7 | prior receipt | ALL GREEN, 31 gates |
| compare gate | candidate 2bf5c039, reference e117f11b | prior receipt | 71 pass, 0 fail, 31 unmeasured |
| Tolerance literals | grep of added lines in the sprint diff | read by the reviewer | `LATERAL_BOUND_MM = 1e-6`, `<= 1e-9`, `* 0.005`, all unchanged from main |
| `unsafe` added | same grep | read by the reviewer | none in code, only in plan and review prose |
| `wasm_bindgen` added | same grep | read by the reviewer | one attribute, `canvas.rs` `resolvedTier`, inside `ocelli-wasm` |
| Tier C path in `canvas.rs` | `open_session` and the `Ok(None)` arm | read by the reviewer | sets `unavailable` and no tier label |
| `resolve` contract | `ocelli-render/src/probe.rs` | read by the reviewer | `resolve_adapter` returns `None` for tier C, and the tier resolves once at startup |

## Defects

### D1, the done statement and the F-198 row claim decode, cache and a DICOMweb render that were not built

**Where**: `docs/sprints/CURRENT_SPRINT.md` "What done means" and the F-198 row, and `docs/sprints/BACKLOG.md` F-198.

**What**: "What done means" says the example viewer renders a frame through `@ocelli/core` and `ocelli-wasm`, including worker decode, the cache, the resolved tier, and a DICOMweb source. The F-198 row says the same work is done. What was built is different. The worker writes one little-endian `f32` value of 40 through `writeFrame` and publishes it. The DICOMweb envelope is parked unparsed in the bulk slot by `holdSessionBytes`. Nothing decodes it, and no `ocelli-cache` type is reached from the worker.

**Why it is wrong**: these lines are new in this diff and they are false. The approved plan narrowed the scope, and AS_BUILT's "What was built" is accurate. The plan records no deviation, and no backlog row carries the dropped decode, cache and DICOMweb render. F-199, F-200 and F-201 each depend on F-198 as done, so S44 opens on a dependency that does not provide what its row says.

**Evidence**: `core-worker.ts` start and accept, `bulk.ts` `holdSessionBytes`, `dicomweb.md` saying that wasm entry is still absent, and both ledger lines above.

**Scope and regression**: prose and ledgers only. Rewrite the done statement and the row title to say what was built, and file a row for worker decode, cache and the DICOMweb render. Or record the narrowing as a deviation from the backlog row.

## Smells

### S1, `resolvedTier` re-runs detection and ignores a session that already resolved tier C

**Where**: `crates/ocelli-wasm/src/canvas.rs` `resolved_tier` and the `Ok(None)` arm of `ensure_device`.

**What**: `stored_tier` reads only `table.adapter` and `table.tier`. When `open_session` resolves tier C, it sets `table.unavailable = true` and leaves `table.tier` as `None`. A later `resolvedTier` then calls `resolve(TierRequest::Auto)` again. A label cached first by `resolvedTier` also stays cached when a later `open_session` resolves tier C, because there is still no adapter.

**Why it is wrong**: the probe says the tier resolves once at startup, and a session that changes tier mid-flight is the quietly different answer D-07 refuses. The fill-rate probe is timing based. A second detection can disagree with the first, and the page would then print A or B for a session that is drawing on tier C. The doc comment says the tier `resolve` kept for this worker, and on this path nothing was kept.

**Evidence**: source trace. The re-detection is confirmed by reading. A flipped label is plausible until it is run on an adapter that D-07 demotes. The example page does not reach this, because it never calls `open_session`.

**Scope and regression**: set `table.tier = Some("C")` in the `Ok(None)` arm, or have `stored_tier` return `C` when `unavailable` is true. Test it with a host-testable form of `stored_tier_label` that takes the unavailable flag.

### S2, a published `loadCore` latches false forever after an initialiser that throws synchronously

**Where**: `packages/core/src/load.ts`.

**What**: `run` is async. If `initialise()` throws before it returns a promise, `catch` and `finally` both run synchronously, and `finally` sets `pending = null`. Only then does the caller store the already resolved false promise. `pending` is never cleared again. A later `loadCore` returns that stale false. Its initialiser is never called, and `coreAvailable()` stays false for that realm.

**Why it is wrong**: `loadCore` is part of the published `@ocelli/core` surface. The type `() => Promise<unknown>` accepts a non-async arrow that throws. The tests only use `Promise.reject`, which goes through the await, so they stay green.

**Evidence**: ordinary async function semantics. The example worker passes an async arrow, so the example does not hit this.

**Scope and regression**: assign `pending` before the call can settle, or clear it only when `pending` is still the promise being settled. Add a test with a synchronous throw followed by a succeeding load.

### S3, the key-object and secondary-capture files omit attributes their IODs require

**Where**: `packages/core/src/tid2010.ts`, `packages/core/src/secondary-capture.ts`, and `docs/lld/key-object.md`.

**What**: the key-object writer emits no Current Requested Procedure Evidence Sequence, no Content Date or Time, no Instance Number and no Series Number. Both writers omit the Patient module entirely. Patient Name and Patient ID are Type 2, so they must be present even when empty.

**Why it is wrong**: from memory of PS3.3 C.17.6.2, the evidence sequence, Content Date and Time, and Instance Number are Type 1 in the Key Object Document module. The LLD says the code writes one Key Object Selection document. F-201's key-image export depends on F-196, and a conformant receiver may reject these files. The plan narrowed the content tree. It did not say the file leaves out mandatory module attributes.

**Evidence**: reading the two part10 datasets. Plausible, because PS3.3 was not opened in this pass.

**Scope and regression**: either emit the Type 1 and empty Type 2 attributes, or say in `key-object.md` that the files are round-trip fixtures and not conformant instances. If the writer changes, the reader's refusal of a present `PatientName` must change with it.

### S4, the tilted constructor classifies a stack as orthogonal from the first step alone

**Where**: `crates/ocelli-volume/src/lib.rs` `prepare`, the `Shear::Keep` bound check.

**What**: take sorted positions `(0, 0, 0)`, `(6e-7, 0, 3)` and `(2.1e-6, 0, 6)`. The lateral steps are 6e-7 and 1.5e-6, which differ by 9e-7, so the agreement check passes. `TiltedVolume::assemble` returns `OrthogonalStack`, because only the first step is compared with the bound. `Volume::assemble` returns `ShearedVolume` for the second step.

**Why it is wrong**: the stack has no constructor, and each refusal points at the other. `OrthogonalStack` is documented as an orthogonal stack belonging to `Volume::assemble`, which is false for this stack. Steps near a micrometre are where decimal-string rounding of Image Position Patient sits. The approved plan only promises that the two constructors never both accept a stack, and that still holds.

**Evidence**: source trace through both `Shear` arms. Not executed.

**Scope and regression**: compare the largest lateral length with the bound, not the first, or document that both can refuse. A three-slice test with the positions above would pin the result.

## Nitpicks

1. `contour_volume` refuses a negative-zero area as `Frame` because `total_cmp` orders `-0.0` below zero. `length_ratio` in the same file treats both zeros alike.
2. `postWadoInstance` caps a fetched instance at 1024 pages, about 64 MiB. A larger instance fails with a `RangeError` from `memory.grow` inside the sink. It also copies the body out with `exportDecodedFrame`, a name that describes a decoded frame.
3. The tilt panel says the pixels stay in the slices they arrived in, and AS_BUILT says the kept order means a later fill does not project the positions again. `TiltedVolume::assemble` consumes the candidate and returns a zeroed store, and no fill function exists.
4. `tilted_pair()` in `tests/tilted.rs` builds three slices.

## Verified clean

- **Tolerances.** `LATERAL_BOUND_MM` is `1e-6` and is the only comparison value for the refuse check, the agreement check and the orthogonal split. The 1e-9 mm duplicate bound and the 0.005 spacing factor have the same literals as main. No other tolerance moved.
- **`unsafe`.** No `unsafe` block was added anywhere. The two allowed files are untouched.
- **wasm-bindgen.** It appears only in `ocelli-wasm`, as `resolvedTier`. The example server gains `ocelli-cache`, `ocelli-geom` and `ocelli-volume` and no binding crate. The `tier_label` cfg is `all(feature = "render-worker", any(test, target_arch = "wasm32"))`, which covers every caller in `canvas.rs`.
- **Prose that checks out.** The CHANGELOG bullets match the route outputs. The README header sentences match `App` and `CorePanel`. The LLD files match the code, apart from S3's conformance wording. The AS_BUILT test counts and review-pass counts are right. The prose that was read has no em dash and no prose semicolon.
- **Boundary.** No pixels cross it. The worker's messages carry booleans, a tier string and a byte count.

## Limitations and handoff

The reviewer ran nothing. All gate, compare and mutation results come from the host's receipts. PS3.3 was not opened, so S3 rests on memory of C.17.6.2 and the Patient module types. S1's flipped label needs a run on hardware where D-07 demotes an adapter. The live page was not clicked. The production `/@fs` URL and the empty tier line were not counted as blocking. D1 needs either honest prose plus a follow-up row, or a recorded deviation, before close. A second pass should review that change and whichever smells are fixed.
