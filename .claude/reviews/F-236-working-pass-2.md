# F-236 review, pass 2

**Reviewed**: `1a811e6d..081b88b6` on `work/f-236-xai` in `/Users/atulsharma/Documents/projects/tensorbee/ocelli-wt-f-236`, two commits (`7a462c1e` remediations, `081b88b6` self-review comment corrections), two files (`packages/core/src/bulk.test.ts`, `packages/core/src/boundary.wasm.test.ts`). Tip `081b88b6`. The working tree was clean apart from the untracked `.venv` and `tools/oracle/out`, which are outside the change. No Rust file and no `bulk.ts` change in the range.
**Author / reviewer**: remediation by GLM in opencode with three self-review cycles (`.claude/scratch/F-236-self-review-pass2-1.md` to `-3.md`). Reviewed by Claude Opus 5.5 as an independent `/microscope --working` pass. This reviewer wrote neither the remediation nor its self-reviews. The pass 1 review was also written by a Claude Opus 5.5 session, so this pass is independent of the author and not of the pass 1 reviewer.
**Result**: 2 defects, 0 smells, 0 nitpicks

## Acceptance challenged

- **Pass 1 S1 replayed, AC5 kept frame.** Independently derived expectation: if the refused `commit_selected_frame` replaced the retained frame with the refused bytes, the second interaction publish must blit different pixels. Probed on the tip dev build through `acceptTransferredFrame`: committing 40, 41, 42, 1000 and -1000 at the current selection, each followed by `publishCpuInteraction`, blits the same image every time, 12800 bytes with 9600 zeros and 3200 of 255, which is opaque black in every one of the 80 by 40 pixels. Then a Rust mutant whose refusal branch commits the refused frame anyway (`let _replaced = self.commit_current_frame(ticket, len, meta)` in place of the release) leaves the tip boundary file 19 of 19 green. See D1.
- **Pass 1 D3 replayed, AC9 one copy.** The duplicated `set` line goes red. Three other second-copy forms stay green. See D2.
- **Pass 1 D1 replayed, the TS-side release after a thrown `ExportUnavailable`.** Closed. Both new cases go red when the release is deleted, when it presents ticket 0, and when the missing export returns 0 instead of throwing.
- **Pass 1 D2 replayed, the AC1 comment.** Closed. The base source under the tip tests fails AC1 with `OcelliError: commit refused`, as the corrected sentence says, and AC2 is the case with "saw 0".
- **GLM's m2c claim.** Holds for the form it describes. A `commitOwned` call duplicated after the assigned one stays green on both the unit and boundary files, because `commitAllocated` returns 0 at `presented !== ticket` once the first call cleared the ticket, before any route call. The same duplicate placed before the assigned call, or reassigning `code`, is red (3 unit and 3 boundary cases, "commit refused"), so only the ignored-result form survives, and the deleted clause did claim more than was observed.
- **Deleted claims.** "a second commit is red on the route length" and "draws idle" appear nowhere in either test file at tip, by grep, and the tree has no unstaged edit.

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|--------------------------|---------------------------|------------------------------|----------------------|
| `wasm-pack build crates/ocelli-wasm --dev --target web --out-dir target/boundary-wasm` | tip, `crates` identical to `081b88b6` | now | 0 |
| `npx vitest run --config vitest.boundary.config.ts` | tip, dev build above | now | 0, 19 passed |
| `npm test` | tip | now | 0, 61 files, 514 tests |
| `npm run lint`, `npm run typecheck` | tip | now | 0, 0 |
| `bin/ocelli.sh gate packages` | tip | now | ALL GREEN |
| `python3 scripts/prose_check.py` | tip | now | OK over 964 files |
| Prose scan of added lines and both commit messages | `git diff 1a811e6d..081b88b6`, `git log` | now | no em-dash, en-dash or comment semicolon |
| TS mutation catalogue, 12 mutants | scratch copy of `packages/core/src` under the session scratchpad, restored and byte-compared after each | now | table under "Verified clean" and D2 |
| Old-source RED | `282b9e82:packages/core/src/bulk.ts` with tip tests | now | unit 6 red (AC1 "commit refused", AC2 "saw 0", both D1 cases "CommitRefused to be ExportUnavailable", AC9, AC7), boundary 2 red (AC5, AC7) |
| Pixel response probe | scratch boundary file with an appended case, tip dev build | now | identical blits for every committed value without a window, distinct values 113, 142, 170 for 40, 41, 42 with `Fit` and `SetWindow` centre 41 width 10 |
| Rust mutant, replace on refusal | `git archive 081b88b6` into scratch, `commit_selected_frame` refusal commits the frame, dev build to a scratch pkg, `OCELLI_BOUNDARY_MODULE` pointed at it | now | tip boundary file 19 of 19 green. The AC5 case with `Fit` and `SetWindow` added is red on the pixel comparison and green on the pristine module |
| Rust mutant, clear on refusal | same scratch tree, refusal sets `last_meta = None` | now | AC5 red, "expected 1 to be 2", 18 others green, which confirms GLM's m3a |
| GLM self-review notes | `.claude/scratch/F-236-self-review-pass2-1.md` to `-3.md` | prior receipt, read | cross-checked below |

## Defects

### D1, the AC5 kept-frame pixel comparison cannot fail, and the comment says it catches a replaced frame

**Where**: `packages/core/src/boundary.wasm.test.ts:314` to `:317` (the claim), `:339` to `:341` and `:370` to `:372` (the observation).
**What**: the comment says "a refusal that cleared or replaced the retained frame fails the comparison or the count". The test commits a 1 by 1 frame with no `Fit` and no window, and in that state the CPU publish blits a uniform opaque black image regardless of the committed value. So `expect(blittedPixels(canvas)).toEqual(kept)` compares two constants. A cleared frame is caught by the blit count. A replaced frame is caught by nothing.
**Why it is wrong**: a factual sentence that is false, and an assertion that passes both ways (microscope sections 2 and 3). The plan's state table makes "the previous committed frame stays" the preserved state for a stale refusal, and the microscope composition rule asks that pixels respond to a changed input on a nonconstant frame or the producer refuse it. This is pass 1 S1 surviving in a new form: the frame is still not observed, only the publish.
**Evidence**: the probe above (identical histograms for 40, 41, 42, 1000, -1000). The replace-on-refusal Rust mutant leaves the tip boundary file green. Self-review cycle 2 measured the same image and recorded it as "not a uniform fill", which it is in every pixel.
**Scope and regression**: present on HEAD, test only. Smallest fix, measured: submit `packCommands(1, [{ kind: "Fit", width: 80, height: 40, columns: 1, rows: 1 }, { kind: "SetWindow", centre: 41, width: 10, function: "linear" }])` after `bindCpuCanvas`. With that, the case is green on the pristine module and red on the replace mutant at the pixel comparison. A control commit of a different value at the current selection after the final assertion, asserting the pixels then differ from `kept`, would keep the comparison honest if the window ever changes. Alternatively reduce the comment to what the count observes.

### D2, AC9 still passes three second-copy implementations, and the comment says a duplicated `set` answers 2

**Where**: `packages/core/src/bulk.test.ts:1043` (the title "one copy"), `:1046` to `:1051` (the comment), `:633` to `:636` (the `countingMemory` doc).
**What**: the count is of `wasm.memory` reads, which is the view count. It is red when the whole `set` line is duplicated. It stays green for a second `.set` through the same view, for two sets through one hoisted `wasm.memory.buffer`, and for a staging copy `new Uint8Array(buffer).slice()`, each of which copies `len` bytes twice. The comment "a duplicated `set` answers 2 and is red" is false for the first two.
**Why it is wrong**: the plan's "Size and speed" row requires "one `set` of `len` bytes per call" and names "A second copy" as the rejected wrong implementation. A test whose named wrong implementation passes is a defect under microscope section 2. The staging copy is the plausible regression, a defensive copy of a transferred buffer.
**Evidence**: mutants `m2a2_same_view`, `m2f_hoisted` and `m2e_staging`, each green on the whole `bulk.test.ts`.
**Scope and regression**: present on HEAD, test only. Smallest fix, measured: inside the AC9 loop spy on `Uint8Array.prototype.set` counting only calls whose `this.buffer` is `mock.wasm.memory.buffer`, and on `Uint8Array.prototype.slice`, restore, and assert the sum is 1. That is green on tip and red on all four second-copy mutants (`m2a`, `m2a2`, `m2e`, `m2f`). Or the lead amends AC9 to "one view and one reservation" and the title and comment say that.

## Smells

None.

## Nitpicks

None.

## Verified clean

- **Pass 1 D1 closed.** `bulk.test.ts` "releases the captured ticket through release_bulk when the selected-frame export is missing" and its cached twin go through `sessionBulkSink` over a mock lacking the export and `acceptTransferredFrame`, the real caller. Red under: release deleted (both, "expected [] to deeply equal [ 21 ]"), release presenting 0 (both), selected export missing returns 0 (selected case, "CommitRefused to be ExportUnavailable"), cached export missing returns 0 (cached case). Their comments were executed: `bulk.ts:307` to `:309` and `:294` throw before the ticket clear at `:320`, the catch at `:460` releases, `releaseAllocated` calls `release_bulk` and zeroes the sink ticket. `MOCK_TICKET` is 21.
- **Pass 1 D2 closed**, by the old-source RED above.
- **AC9's surviving half.** `m2a` (line duplicated) red with "expected 2 to be 1". `countingMemory`'s "the sink keeps the raw mock, so nothing the sink does is counted" holds, the sink is built from `mock.wasm`.
- **AC5's count half.** The clear-on-refusal mutant goes red at the blit count, independently rebuilt.
- **Stub doc at `boundary.wasm.test.ts:128` to `:136`.** `installCanvasStubs` is called at exactly three sites (`:318`, `:704`, `:738`). The two older CPU tests pass an anonymous `new StubCanvas()` and assert only the drained ring, so the recording cannot change what they observe.
- **m2c**, as under "Acceptance challenged".
- **Rules.** No Rust, no `bulk.ts`, no wasm-bindgen, no new view over wasm memory, no tier path. Tiers A, B and C n/a as the plan says. Prose rules clean.
- **User documents.** The range touches only tests, and pass 1 measured that `README.md`, `use.html` and `more.html` do not mention `acceptTransferredFrame`, so nothing there can be stale.

## AC table against evidence

| AC | Evidence now | Owed |
|----|--------------|------|
| AC1 | unit case, red on base | none |
| AC2 | unit case and boundary refusals, red on base | none |
| AC3 | unit case, green on base by design (refusal predates the story) | none |
| AC4 | "finishes the copy before commit_frame" green | none |
| AC5 | boundary case, red on base. Commit and refusal halves hold. Kept-frame half is D1 | D1 |
| AC6 | old-source RED reproduced, 6 unit and 2 boundary | record in handoff |
| AC7 | unit and boundary cases, red on base | none |
| AC8 | no Rust in either commit, so the module cannot change. Pass 1 ran `gate wasm` on tip | base and tip byte sizes in the handoff, which does not exist yet |
| AC9 | reservation half and line-duplicate copy half | D2, plus the `dist/bulk.js` base and tip sizes in the handoff |

Also owed at completion, as the plan assigns: `docs/lld/typescript-packaging.md:116` to `:122`, which still ends the order in `commit_frame`, and the `CHANGELOG.md` entry.

## Self-review catch figure

Self-review should have caught 2 of 2. D1: cycle 2 recorded "Candidate hole ... the AC5 comparison could in principle pass on unchanged garbage" and closed it by measuring one image, without changing the input, and that image is uniform in every pixel. D2: cycle 1 wrote the "duplicated `set`" sentence and ran only the whole-line duplicate, while the plan names "A second copy" without a form. GLM's m2c analysis and its deletion of the two unproved claims were correct.

## Limitations and handoff

- Not exercised: a browser worker, a GPU adapter, a release build. Built-module evidence is the dev build under Node. The Rust mutants were scratch builds under `CARGO_TARGET_DIR=$PWD/target`, never in the worktree.
- The suggested fixes for D1 and D2 were run in scratch only, and are evidence that a discriminating test exists, not the remediation.
- Next owner: the implementer remediates D1 and D2, then a fresh independent pass reviews the changed material. Pass 1 S1 has now survived two passes in changing form, worth the operator's attention. Scratch under the session scratchpad (`f236p2`, `f236rs`, `f236rs-pkg`) is deleted with Python `shutil` at the end of this pass.
