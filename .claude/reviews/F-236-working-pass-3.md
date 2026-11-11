# F-236 review, pass 3

**Reviewed**: `0499febd..13eec9bf` on `work/f-236-xai` in `/Users/atulsharma/Documents/projects/tensorbee/ocelli-wt-f-236`, one commit (`13eec9bf`), two files (`packages/core/src/boundary.wasm.test.ts`, `packages/core/src/bulk.test.ts`). Tip `13eec9bf`. The working tree was clean apart from the untracked `.venv` and `tools/oracle/out`, which are outside the change. No Rust file and no `bulk.ts` change in the range, and `crates`, `Cargo.toml` and `Cargo.lock` are identical to the tip by `git diff --quiet`.
**Author / reviewer**: remediation by GLM in opencode with three self-review cycles (`.claude/scratch/F-236-self-review-pass3-1.md` to `-3.md`). Reviewed by Claude Opus 5.5 as an independent `/microscope --working` pass. This reviewer wrote neither the remediation nor its self-reviews. Passes 1 and 2 were also written by Claude Opus 5.5 sessions, so this pass is independent of the author and not of the earlier reviewers.
**Result**: 0 defects, 1 smell, 1 nitpick

## Acceptance challenged

- **Pass 2 D1 replayed, AC5 kept frame. Closed.** Independently derived expectation from PS3.3 C.11.2.1.2.1 `LINEAR` with centre 41 and width 10: the ramp runs from 36 to 45, and `((x - 40.5) / 9 + 0.5) * 255` gives 113.33, 141.67, 170.00 and 198.33 for 40, 41, 42 and 43, so 113, 142, 170 and 198 after rounding. A probe on the pristine dev module measured exactly those four grey levels over the 1600 image pixels with `Fit` and `SetWindow`. The refused values 41 and 42 therefore blit images that differ from the kept 40, and the control value 43 differs too.
  - The control: `boundary.wasm.test.ts:403` to `:407` publishes after committing 43 and asserts the image differs from `kept`. Green on the pristine module, so the comparison is shown able to fail.
  - The replace mutant: the selected-frame refusal branch at `session.rs:240` commits the refused frame (`let _replaced = self.commit_current_frame(ticket, len, meta)`). The tip boundary file goes red at `:391`, the pixel comparison, "expected [ +0, +0, +0, 255, ... ] to deeply equal [ ... ]", 18 others green.
  - The adjacent clear mutant: the same branch also sets `last_meta = None`. Red at `:390`, the blit count, "expected 1 to be 2".
- **The lead's concern about "byte-identical images with the Fit and window".** The tree does not say that. The comment at `boundary.wasm.test.ts:319` to `:321` says a 1 by 1 frame "under the Fit alone" and "without a window" blits byte-identical images for 40 to 43. Measured true: with `Fit` alone, 40, 41, 42, 43 and 1000 all blit 4800 zero bytes and 8000 bytes of 255, and only -1000 differs. With no command, every value blits the same image. With `SetWindow` alone and no `Fit`, every value also blits the same image, so the case needs both commands and submits both. With both, the four values are four distinct images, as above.
- **Pass 2 D2 replayed, AC9 one copy. Closed for the four ruled forms.** The tip counter at `bulk.test.ts:1091` to `:1114` is green on the tip and red on each of the four mutants the ruling lists: duplicated `set` line, second `set` through one view, two `set` calls through one saved buffer, defensive `Uint8Array` `slice`. The duplicated line fails first at `reads` (`:1113`). With the `reads` assertion removed in scratch it fails at the copy count, "expected 2 to be 1", so the comment's "each answer 2" holds on the copy counter itself.
- **GLM's two disclosed blind spots.** Six further second-copy forms were run against the tip test. All six stay green, see S1. The comment claims only the forms it names, but the AC9 title and the plan's AC9 row are wider than what the counter observes.

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|--------------------------|---------------------------|------------------------------|----------------------|
| Three dev builds, pristine, replace and clear | `git archive 13eec9bf` into the session scratchpad, `CARGO_TARGET_DIR` a scratch cache, session.rs touched before each build | now | each log has one `Compiling ocelli-wasm`. md5 pristine `3403130f...`, clear `924d9134...`, replace `e4de9892...` |
| Same three builds before the touch | same scratch cache shared by three trees | now | replace and clear reported Finished in 0.10s and 0.07s with no compile, md5 identical to pristine. See "Limitations" |
| Boundary file against each scratch module through `OCELLI_BOUNDARY_MODULE` | tip test file, unmodified | now | pristine 19 of 19. replace red at `:391` only. clear red at `:390` only |
| Pixel probe, four configurations by six values | scratch copy of the boundary file with an appended case, pristine module | now | histograms quoted under "Acceptance challenged" |
| TS copy-mutant catalogue, 10 mutants | scratch copy of `packages/core/src`, `bulk.ts:454` replaced, restored and compared after the run | now | 4 ruled forms red at `:1113` or `:1114`. 6 further forms green, see S1 |
| Widened counter, the S1 fix | scratch copy of `bulk.test.ts` | now | green on tip, red on the 4 ruled forms and on 3 of the 6 further forms. `tsc -b packages/core/tsconfig.test.json` exit 0 |
| `wasm-pack build crates/ocelli-wasm --dev --target web --out-dir target/boundary-wasm` | worktree, fresh scratch `CARGO_TARGET_DIR` | now | 0, one compile of ocelli-wasm, md5 `3403130f30cd947409532638b5118463`, matching the scratch pristine build |
| `npx vitest run --config vitest.boundary.config.ts` | tip, the dev build above | now | 0, 19 passed |
| `npm run lint`, `npm run typecheck` | tip | now | 0, 0 |
| `npm test` | tip | now | 0, 61 files, 514 tests |
| `bin/ocelli.sh gate packages` | tip | now | ALL GREEN |
| `python3 scripts/prose_check.py` | tip | now | OK over 965 files |
| Prose scan of the added lines and the commit message | `git diff 0499febd..13eec9bf`, `git log -1 13eec9bf` | now | no em-dash, en-dash or comment semicolon |
| GLM self-review notes | `.claude/scratch/F-236-self-review-pass3-1.md` to `-3.md` | prior receipt, read | cross-checked below |

## Defects

None.

## Smells

### S1, AC9's copy counter misses the most idiomatic defensive copy of a transferred buffer

**Where**: `packages/core/src/bulk.test.ts:1044` (the title "one copy"), `:1091` to `:1111` (the counter), and the plan's AC9 row, which names "A second copy" as the wrong implementation.
**What**: the counter counts `Uint8Array.prototype.set` calls whose target is the wasm memory and every `Uint8Array.prototype.slice`. Six second-copy forms of `bulk.ts:454` stay green on the whole `bulk.test.ts`:
- `buffer.slice(0)`, the `ArrayBuffer` slice that is the usual defensive copy of a transferred buffer
- a staging copy, `tmp.set(new Uint8Array(buffer))` then `view.set(tmp)`
- a second copy through an `Int8Array` view over one saved buffer
- a constructor copy, `new Uint8Array(new Uint8Array(buffer))`
- `Uint8Array.from`
- `structuredClone(buffer)`

The tip code has one copy, so the title is true of the tree today. The comment names only the forms that are red, so no sentence is false. What remains is that a regression adding the first or second form, which doubles the copy of up to 64 MB per frame, keeps AC9 green.
**Why it matters**: the plan's "Size and speed" row requires one `set` of `len` bytes and rejects "A second copy", and microscope section 2 counts a test that passes both ways on a named wrong implementation. The ruling's four forms are a floor that is met.
**Evidence**: the TS catalogue above. GLM's cycle 1 and 2 notes ran two of these (form5, form6), found them green, and recorded them as "the counter's blind spots, not claimed by the comment". So the blind spot was disclosed but not raised as a finding.
**Fix, measured in scratch**:
- Spy on `Object.getPrototypeOf(Uint8Array.prototype)` (the `%TypedArray%` prototype) for `set` and `slice`, instead of `Uint8Array.prototype`.
- Count a `set` when its target buffer is the wasm memory's buffer or its source's `buffer` is the transferred `buffer`.
- Spy on `ArrayBuffer.prototype.slice` and count only when `this === buffer`. The mock's own capture at `bulk.test.ts:574` slices the wasm memory, so a predicate without that filter would count the mock.
- Restore all three in the `finally`.

That variant is green on tip and red on the four ruled forms plus `buffer.slice(0)`, the staging `set` and the `Int8Array` copy, and it typechecks. The constructor copy, `from` and `structuredClone` stay unobserved by any prototype spy, and the comment should say so in one clause. The other option is that the lead narrows AC9's wrong-implementation column to the counted forms. This is the third pass to raise AC9's copy half, after pass 1 D3 and pass 2 D2, in a narrower form each time.

## Nitpicks

### N1, one event named as two

`boundary.wasm.test.ts:322` to `:324` say "the control commit after the final transfer". The control commit is the final transfer, the 43 at `:398`. "the final transfer is the control" would say it once.

## Verified clean

- **D1 closed**, by the probe, the control and both Rust mutants above, through `acceptTransferredFrame` and `publishCpuInteraction`, the real caller and the real sink. The submit assertion `toBe(2)` at `:347` is a live setup guard. It counts both commands.
- **Refusal then recovery.** After both refusals, `bulk_ticket()` is 0 and `sliceIndex` is 0. The next current transfer commits through the same caller, and the sink after recovery shows the new value. This covers the sink after refusal as well as after recovery.
- **D2 closed for the ruled forms**, as above. The `countingMemory` doc at `bulk.test.ts:635` to `:638` now says the read count observes the view and not the copy. The second-`set`-through-one-view mutant executes that sentence: reads stay 1 and copies are 2.
- **Spy hygiene.** Both prototypes are restored in a `finally`. The other 34 cases in the file and the other 60 files stay green in the same run, so nothing leaks. GLM's throw-path variant was not re-run. The `finally` makes it hold by construction.
- **GLM's E1.** Reproduced. The clear mutant's md5 `924d91346da192739acd7c0f43109d59` matches GLM's record of the binary it found in the worktree's `boundary-wasm`. That binary came from pass 2's reviewer building with the worktree's own target. The worktree module was rebuilt from source with a fresh scratch cache before the final green run.
- **Rules.** The range changes no Rust, no `bulk.ts`, no wasm-bindgen use, no view over wasm memory outside `bulk.ts` and no tier path. Tiers A, B and C are n/a as the plan says. The prose rules are clean.
- **User documents.** The range touches tests only. `README.md`, `use.html` and `more.html` do not mention `acceptTransferredFrame`, as pass 1 measured, so nothing there can be stale.

## AC table against evidence

| AC | Evidence now | Owed |
|----|--------------|------|
| AC1 to AC4 | unchanged since pass 2, unit file green | none |
| AC5 | boundary case. Commit, refusal, count and pixel halves all hold under both Rust mutants | none |
| AC6 | old-source RED, pass 2 receipt | record in the handoff |
| AC7 | unit and boundary cases green | none |
| AC8 | no Rust in the range | base and tip wasm sizes in the handoff |
| AC9 | reservation half and the four ruled copy forms red | S1, plus base and tip `dist/bulk.js` sizes in the handoff |

There is no F-236 handoff under `.claude/handoffs` yet. Also owed at completion, as the ruling assigns: `docs/lld/typescript-packaging.md:118` to `:119`, which still ends the order in `commit_frame`, and the `CHANGELOG.md` entry, which has no F-236 line.

## Self-review catch figure

Self-review should have caught 1 of 1 blocking finding. GLM ran two of S1's forms, saw them green, and wrote them down as unclaimed blind spots instead of a finding against the plan's "A second copy". N1 is not counted. GLM's D1 work was correct and well measured. Its 113, 142, 170 and 198 match this pass's independent PS3.3 derivation, and its correction of pass 2's "uniform opaque black" for the `Fit` state was right.

## Limitations and handoff

- **Cache hazard, measured.** One `CARGO_TARGET_DIR` shared by two source trees with the same relative layout reports a mutant build as fresh and ships the previous binary when the edited file's mtime is older than the last build. Here that gave a 0.10s Finished and an identical md5. A reviewer or implementer should touch the mutated file or use a cache per variant, and check for a `Compiling ocelli-wasm` line and a changed md5 before trusting a mutant run.
- **Not exercised**: a browser worker, a GPU adapter, a release build. The built-module evidence is the dev build under Node.
- **Status of the S1 fix**: it was run in scratch only. That shows a discriminating counter exists. It is not the remediation.
- **Next owner**: the implementer remediates S1, or the lead rules on AC9's wording. Then a fresh independent pass reviews the changed material.
- **Scratch**: everything is under the session scratchpad (`f236p3`) and is deleted with Python `shutil` at the end of this pass.
