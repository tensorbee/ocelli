# F-236 review, pass 4

**Reviewed**: `edc494a0..f559ed8d` on `work/f-236-xai` in `/Users/atulsharma/Documents/projects/tensorbee/ocelli-wt-f-236`, one commit (`f559ed8d`), two files (`packages/core/src/bulk.test.ts`, `packages/core/src/boundary.wasm.test.ts`). Tip `f559ed8d`. The working tree was clean apart from the untracked `.venv` and `tools/oracle/out`, which are outside the change. `crates`, `Cargo.toml`, `Cargo.lock` and `packages/core/src/bulk.ts` are unchanged in the range and identical to the tip by `git diff --quiet`.
**Author / reviewer**: remediation by GLM in opencode with three self-review cycles (`.claude/scratch/F-236-self-review-pass4-1.md` to `-3.md`). Reviewed by Claude Opus 5.5 as an independent `/microscope --working` pass. This reviewer wrote neither the remediation nor its self-reviews. Passes 1 to 3 were also written by Claude Opus 5.5 sessions, so this pass is independent of the author and not of the earlier reviewers.
**Result**: 0 defects, 0 smells, 0 nitpicks

## Acceptance challenged

- **Pass 3 S1 replayed, AC9's copy counter. Closed.** The counter at `bulk.test.ts:1093` to `:1136` spies on `Object.getPrototypeOf(Uint8Array.prototype)` for `set` and `slice` and on `ArrayBuffer.prototype.slice` filtered to `this === buffer`, as the lead's ruling requires. Each of the seven ruled forms of `bulk.ts:454` was applied in a scratch tree and run against the unmodified tip test and against an instrumented copy that prints the copy and read counts before asserting:

  | Form | Tip test | First red line | Copies, reads |
  |------|----------|----------------|---------------|
  | duplicated `set` line | red | `:1138` (reads) | 2, 2 |
  | second `set` through one view | red | `:1139` (copies) | 2, 1 |
  | two `set` calls through one saved buffer | red | `:1139` | 2, 1 |
  | `Uint8Array` `slice` | red | `:1139` | 2, 1 |
  | `buffer.slice(0)` | red | `:1139` | 2, 1 |
  | staging copy, `tmp.set(new Uint8Array(buffer))` then `view.set(tmp)` | red | `:1139` | 2, 1 |
  | `Int8Array` copy through one saved buffer | red | `:1139` | 2, 1 |
  | tip | green, 35 of 35 | none | 1, 1 by the green assertions |

  So the comment's "each answer 2 on the copy count and are red" holds for all seven, including the duplicated line, whose copy count is 2 even though `reads` fails first.
- **The comment's blind-spot clause.** Executed, not argued from: a constructor copy (`new Uint8Array(new Uint8Array(buffer))`), `Uint8Array.from` and `structuredClone(buffer)` each leave the whole file green, 35 of 35. The clause is true and appears once, at `bulk.test.ts:1059` to `:1061`. Per the ruling, these forms are not findings.
- **AC9's row wording.** `.claude/plans/F-236-design.md:170` reads "A second copy by `set` or `slice` through any typed-array view or of the transferred buffer, or a second allocation", which matches the ruling at `:216`. The counter observes exactly that set of forms.
- **Pass 3 N1 replayed. Closed.** `boundary.wasm.test.ts:322` to `:324` now say "the final transfer is the control: its publish asserts a different value changes the blit". That matches the case: the control commit is the final transfer of 43 and its publish is the image comparison. Boundary file 19 of 19 on the pristine module.

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|--------------------------|---------------------------|------------------------------|----------------------|
| Seven-form copy catalogue plus tip, tip test and instrumented test | `git archive f559ed8d` into the session scratchpad, `bulk.ts:454` replaced per form, restored and compared after the run | now | table above. Restore check True for both files |
| Blind-spot forms, constructor copy, `from`, `structuredClone` | same scratch tree | now | each exit 0, 35 passed |
| Throw-path hygiene probe | scratch `bulk.ts` throws after the copy for 16-byte frames, scratch test appends a case asserting `%TypedArray%.prototype.set`, `.slice` and `ArrayBuffer.prototype.slice` are native and `Uint8Array.prototype` has no own `set` or `slice` | now | AC9 and five other 16-byte cases red on the throw, the probe green, 6 failed and 30 passed |
| Probe control | same probe, AC9's `finally` with the `set` restore removed | now | probe red, so it can see a leaked spy |
| Probe on tip | tip `bulk.ts`, tip test plus the probe | now | 36 passed |
| `git diff --name-only edc494a0..f559ed8d` | range | now | the two test files only |
| `wasm-pack build crates/ocelli-wasm --dev --target web --out-dir target/boundary-wasm` | worktree, fresh scratch `CARGO_TARGET_DIR` | now | 0, one `Compiling ocelli-wasm`, md5 `3403130f30cd947409532638b5118463`, matching pass 3's pristine build |
| `npx vitest run --config vitest.boundary.config.ts` | tip, the dev build above | now | 0, 19 passed |
| `npm run lint`, `npm run typecheck` | tip | now | 0, 0 |
| `npm test` | tip | now | 0, 61 files, 514 tests |
| `bin/ocelli.sh gate packages` | tip | now | 0, ALL GREEN |
| `python3 scripts/prose_check.py` | tip | now | 0, OK over 966 files |
| Prose scan of the added lines and the commit message | `git diff edc494a0..f559ed8d`, `git log -1 f559ed8d` | now | no em-dash, en-dash or comment semicolon |
| GLM self-review notes | `.claude/scratch/F-236-self-review-pass4-1.md` to `-3.md` | prior receipt, read | agree with this pass on every colour and line number |

## Defects

None.

## Smells

None.

## Nitpicks

None.

## Verified clean

- **S1 closed** for all seven ruled forms through the real `acceptTransferredFrame` on the generation, selected and cached routes, with the copy value observed and not inferred from the red.
- **Spy hygiene.** All three originals are captured before installation and restored in the `finally` at `bulk.test.ts:1132` to `:1136`. After a throw inside the window the three prototypes are native and `Uint8Array.prototype` carries no own `set` or `slice`. The probe goes red when one restore is removed, so it is a live check. Moving the spy to `%TypedArray%.prototype` also removes pass 3's shadowing own property on `Uint8Array.prototype`.
- **The `ArrayBuffer` filter.** The mock's capture at `bulk.test.ts:574` slices `memory.buffer` during the commit. Tip green with exactly 1 shows the `this === buffer` filter keeps it out of the count. GLM's notes record the unfiltered variant red on tip, which is consistent.
- **Comment and code agree.** The `countingMemory` doc at `:636` to `:638` and the AC9 comment at `:1045` to `:1065` claim only what the three spies count, and every sentence in them was executed above.
- **Write set.** Only `packages/core/src` test files changed. No Rust, no `bulk.ts`, no wasm-bindgen use, no view over wasm memory outside `bulk.ts` and no tier path. Tiers A, B and C are n/a as the plan says.
- **User documents.** The range touches tests only, and `README.md`, `use.html` and `more.html` do not mention `acceptTransferredFrame`, so nothing there can be stale.

## AC table against evidence

| AC | Evidence now | Owed |
|----|--------------|------|
| AC1 to AC4 | unchanged since pass 2, unit file green | none |
| AC5 | boundary case green on the pristine module, unchanged since pass 3's mutants | none |
| AC6 | old-source RED, pass 2 receipt | record in the handoff |
| AC7 | unit and boundary cases green | none |
| AC8 | no Rust in the range | base and tip wasm sizes in the handoff |
| AC9 | reservation half and the seven ruled copy forms red, tip green, row wording matches the ruling | base and tip `dist/bulk.js` sizes in the handoff |

There is no F-236 handoff under `.claude/handoffs` yet. Also owed at completion, as the ruling assigns: `docs/lld/typescript-packaging.md:118` to `:119` and the `CHANGELOG.md` F-236 entry, both the lead's completion edits.

## Self-review catch figure

Self-review should have caught 0 of 0 blocking findings. GLM's three cycles reported zero findings and this pass found none. Its catalogue colours and line numbers (`:1138` for the duplicated line, `:1139` for the other six) match this pass's independent runs.

## Limitations and handoff

- **Not exercised**: a browser worker, a GPU adapter, a release build. The built-module evidence is the dev build under Node.
- **Clean bounded review, not completion.** This pass closes the review loop for the reviewed range. AC6, AC8 and AC9's handoff receipts and the lead's completion edits are still owed, and `gate --sprint` belongs at sprint end.
- **Scratch**: everything is under the session scratchpad (`f236p4`) and is deleted with Python `shutil` at the end of this pass.
