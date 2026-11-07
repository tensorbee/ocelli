# F-209 review, pass 2

**Reviewed**: `8ce953b1..6540bf22` on `work/f-209-xai`, two commits, `9222b768` (remediation of pass 1) and `6540bf22` (the gate description line), HEAD `6540bf224e122f7ec7aae76e052d9b40f85c7b88`. Working tree clean apart from the untracked `.venv` and `tools/oracle/out`, which are not part of the story. Pinned against `.claude/plans/F-209-design.md` and its lead amendments.
**Author / reviewer**: a Claude implementer wrote both commits. This pass is a separate Claude reviewer instance that wrote none of the reviewed material and did not write pass 1. The independence comes from the session, and no generated trailer establishes it.
**Result**: 0 defects, 0 smells, 4 nitpicks

## Acceptance challenged

Every pass 1 finding was replayed through its real caller.

- **D1, closed.** I drove the release page in Playwright Chromium with my own synthetic folder, independent of `link.mjs`'s generator. pydicom wrote five 40 by 40 series in one Frame of Reference, with no patient attributes: axial CT, coronal CT, sagittal CT, axial MR and axial CT in reversed order. The bytes were injected into `#folder` from memory through a `DataTransfer`, and the viewer ran on an ephemeral port. I derived each expected line by hand from PS3.3 C.7.6.2.1.1. The coronal plane at y = 0 cuts an axial image along row 0 and the sagittal image along column 0. The axial plane at z = 0 cuts a coronal or sagittal image, whose rows run from z = 39 down, along row 39.
  - Original trigger. At 3 by 3 with Scroll on, the coronal source is in cell 1. Cells 0, 3 and 4 show `(0.5,0.5)-(39.5,0.5)` and cell 2 shows `(0.5,0.5)-(0.5,39.5)`, as derived. A click on empty cell 5 leaves **no** line displayed. On a binary built from the pre-fix page, the same click leaves all four lines displayed, which is pass 1's observation.
  - Adjacent paths. A click back on cell 1 draws the same four lines again. With empty cell 5 outlined, turning Reference lines off and then on again shows nothing. The sagittal source in cell 2 gives vertical lines at column 0 in cells 0, 1, 3 and 4. From empty cell 6, going to 2 by 2 moves the outline to cell 0. Cells 1 and 2 then show row 39 and cell 3, the MR axial, shows no line because it is parallel. Opening series 3 in outlined cell 1 (a series swap) redraws cells 0 and 3 from the sagittal plane and leaves no line on the now-parallel cell 2. Going to 1 by 1 shows no line, and going back to 2 by 2 draws them again. There were no page errors.
- **D2, closed.** I ran the corrected `use.html` sentences as a scratch test against the library. An empty stack gives `Err(Degenerate)`. A stack whose only distance overflows (z = 1e200) gives `Err(Degenerate)`, and the same stack with a finite plane added answers `Ok(1)`. A target with one row at row spacing 0 gives `Err(Degenerate)`. A 13-row target at row spacing 0 is refused by the constructor (`InvalidPixelSpacing`), so "zero spacing on its single row or column" is the only zero-spacing case `reference_segment` can be handed. A source at 1e300 gives `Err(Degenerate)` through the non-finite point. Every clause holds.
- **D3, closed.** `more.html` now says only that `reference_segment` projects the plane, which its signature `(&ImagePlane, &ImagePlane)` bears out. The wrong half of the sentence was deleted, not explained.
- **S1, closed.** I computed every new expected value again in exact fractions, with my own PS3.3 matrix and the cornerstone3D 5.8.2 edge rule, without reading the implementation's output:
  - m = 50: dot 2499/2501 = 0.99920, above 0.999, so `Parallel`
  - m = 40: dot 0.99875, line `(0, 1601/16)` to `(127/2, 1601/16)`, which is 100.0625
  - m = 4000: q = 4.99999969e-4, perpendicular branch, rows `-64000/15999999` and `191936012/15999999`
  - m = 1000: q = 1.999998e-3, the other branch, rows `999999/250` = 3999.996 and `-93000173/10000`

  All four equal the test constants. In the mutants, `>= 1.0`, `> 0.5`, `== 0.0`, `< 0.5` and `< 2.1e-3` all go red. `> 0.5` and `< 0.5` also go red from `ocelli-viewport --test link` alone, so the epsilon module carries them without the folder-viewer test.
- **S2, closed.** The tilted stack has row `(1,0,0)` and column `(0,0.96,0.28)` at z = 0, 5 and 10, with the axial source at z = 5. I get dot 0.96, distances 5, 0 and 5, `index 1`, and `[0.5,0.5,64,0.5]` with the page's half pixel. On the shown image 0 the line would be `[0.5,18.357142857142858,64,18.357142857142858]`. `drawn = shown` goes red.
- **S3, closed.** `target 1 0 0` with 65,536 `- -` pairs is a 262,156-byte body, far under `LINK_BODY_CAP`. The `>=` check runs before each push, so 65,536 images are admitted and the 65,537th is refused. The test asserts both, and also `X-Ocelli-Error: link`. Removing the guard goes red, and so does weakening it to `>`. The 65,537-frame object gives `"frames":65537`, and removing `count.min(FRAME_LIST_CAP)` goes red.
- **N1, now held.** For the flat target, the normal is `(1,0,0) x (0,0,-1) = (0,1,0)`, dot 0 with the axial source. Scroll is therefore refused as not coplanar, and the line is degenerate from row spacing 0 on one row. Swapping the precedence goes red.

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|--------------------------|---------------------------|------------------------------|----------------------|
| Mutation catalogue, 11 mutants, `cargo test -p ocelli-viewport -p ocelli-folder-viewer` | scratch copy of HEAD under `target/`, own `CARGO_TARGET_DIR`, removed with `shutil` | now | 10 red, 1 survived (`> 0.9988`, N4) |
| `> 0.5` and `< 0.5` against `--test link --no-fail-fast` only | same copy | now | both 101, epsilon tests named |
| Exact-fraction reference for the four epsilon cases and the tilted stack | independent Python | now | all equal the test constants |
| Scratch test executing the D2 sentences | same copy, file removed afterwards | now | as listed above |
| Playwright replay, 12 states, synthetic in-memory folder | worktree release build at HEAD, ephemeral port | now | D1 closed, adjacent paths as derived, 0 page errors |
| Same replay on a pre-fix binary | scratch copy with the `sendLink` hunk reverted | now | four stale lines after the empty-cell click |
| `node examples/folder-viewer/tests/browser/link.mjs` on the pre-fix page | same pre-fix copy | now | exit 1, "2 reference lines stayed with no source image" |
| `bin/ocelli.sh gate viewer-browser` | worktree HEAD, Chromium 151.0.7922.34 | now | 0, ALL GREEN, aspect.mjs, readout.mjs and link.mjs all pass, F-208 aspect and readout lines unchanged |
| `cargo test -p ocelli-viewport -p ocelli-folder-viewer` | scratch copy identical to HEAD (`diff -rq`) | now | 0, link 14, preview 76, lib 73, value 24, sync 7, the rest green |
| `cargo clippy ... --all-targets -D warnings`, `cargo fmt --check` | same | now | both 0 |
| `python3 scripts/guard_probe.py --only viewer-browser.link-stale-line` | worktree HEAD | now | 0, the probe drove the guard red for its declared reason, control green |
| `python3 scripts/guard_census.py`, `--check-runbook`, `--render-runbook` | worktree HEAD | now | all 0, 0 watched by nothing, and the render left `git status` clean, so the runbook is the generated output byte for byte |
| `npx eslint` on link.mjs, `prose_check.py`, `deviation_check.py`, `gate --list` | worktree HEAD | now | all clean, and the gate line names the three scripts |

## Defects

None.

## Smells

None.

## Nitpicks

- **N1, pass 1 N2 carried.** README line 225 is still 111 characters ("window when a W/L drag ends, on a preset and on Reset. A preset is not sent to a viewport whose image is not"). The reflow moved the overlong text down one line and did not fix it.
- **N2.** In `docs/lld/viewport.md` the inserted clause "and `SyncScene` also calls `same_frame_of_reference`" comes just before "It sends each plane as a token". `It` can now be read as `SyncScene`, which sends no token. Starting that sentence with "The route" would remove the doubt.
- **N3.** `link.mjs` records `after.active` and `linkProblems` never checks that it is 5. If the click missed, the lines would stay and the check would still go red, so this cannot produce a false green. The message would then blame "no source image" for a click that never landed.
- **N4.** The thresholds are now bracketed, but only so far. A parallel threshold anywhere between dot 0.99875 and 0.99920 passes the suite, and `> 0.9988` survived. A perpendicular threshold anywhere between 5e-4 and 2e-3 passes too. That is enough to catch a wrong order of magnitude, a wrong operator or a zero. It does not pin the constant, which `the_thresholds_are_the_oracle_constants` pins by value.

## Verified clean

- The `sendLink` change. A null request now clears every line and then loops. `clearAllLines` raises the ticket, but `busy` serialises the loop, so no reply is in flight at that point and nothing valid is dropped. `linkWanted()` false still breaks before `linkRequest`, and each of its routes clears separately: `setReference(false)` clears every line and `setLayout` clears the hidden cells. With a source image and no target, the clear has no visible effect. A loading source flickers its targets' lines off until the load schedules again, which is correct for "drawn from the current state".
- `link.mjs`. It writes 12 synthetic 32 by 32 CT slices with no patient attributes into a temporary folder and removes it in `finally`. It starts the viewer on `OCELLI_PORT=0`, and `startViewer` refuses 8742 and 8765. It stops the viewer by PID. Its three-series layout gives two axial targets, and before the click it observed 2 lines, as the gate output shows. It is wired into `bin/ocelli.sh` after readout.mjs with `&&`, and CI's `viewer-browser` job runs that gate.
- Catalogue and runbook. `viewer-browser.link` follows its two siblings' shape. Its `silent` claim holds, since `discover.py`'s `SCAN_ROOTS` has no `examples/`. The probe's substitution removes the stale-line branch, and the probe goes red. The `limit` sentence, "the nothing-shown branch has no probe", is true. Rows 581 to 584 and the limit bullet are exactly what `--render-runbook` writes.
- Comments in the new tests. Every arithmetic comment in both new test blocks was checked against my own derivation, including "66.5 s", "TL - BL ... dot 0.28", "y = 0" and "18.357...". The `inspect_json` test counts `null` entries, and the mutation proves that count discriminates.
- Prose. `use.html` and `more.html` as above. The LLD's SyncScene clause is true (`sync.rs` lines 91 and 96). README step 4 says nothing about an empty source, so the fix leaves it true. The F-208 AS_BUILT entry's "two headless Chromium checks" describes F-208 and is historical.
- F-208 and F-211. The aspect and readout checks are green under the gate. The measure and value tests are green, and `PAGE_PIXEL_CENTRE` is untouched.

## Limitations and handoff

- Not exercised in this pass: AC17 and B2 (operator host and pseudo-PHI study only), AC20 and AC22, which pass 1 also left open. Pass 1's other replays (window and zoom sync, colour and MR refusals, stale replies, fetch failure) were not re-run, and the diff does not touch those paths.
- PS3.3 10.7.1.3's single-row zero-spacing allowance was checked from memory, not against a fetched current edition. The constructor's refusal of a multi-row zero spacing agrees with that reading.
- I created and removed one scratch test file in the scratch copy with `rm -f`, not with `shutil`. Nothing in the tracked worktree was touched apart from writing this review. The scratch copy itself was removed with `shutil`. The `--render-runbook` run rewrote the runbook in place and left it identical.
- A viewer from 5 November is still listening on 8742 (PID 98891, cwd the main checkout). It is not this pass's, and I left it running.
- The remediation adds `link.mjs`, a catalogue entry, runbook rows and a `bin/ocelli.sh` line, none of which are in the plan's amended write set. They follow from pass 1's suggested regression and from the guard census, but the lead may want to record them.
- When the story completes, F-209's AS_BUILT entry should say the gate runs three browser checks.
- This is a clean bounded review of the changed material. It is not completed acceptance and not sprint readiness.
