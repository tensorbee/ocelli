# F-214 review, pass 3

**Reviewed**: range f75b0cc1..5f9ec1ce on work/f-214-xai, tip 5f9ec1ce, one commit. The worktree was clean apart from the untracked `.venv` and `tools/oracle/out`, which are not part of the story.
**Author / reviewer**: written by a separate Claude implementer agent. Reviewed by an independent Claude reviewer that wrote none of it. Mutants ran on a `git archive` copy of 5f9ec1ce in the reviewer's scratch directory with its own `CARGO_TARGET_DIR`, restored by copy from a second pristine archive and compared with `cmp`. The implementer's MM is not described in the tree, so its overlap with the mutants below could not be checked.
**Result**: 0 defects, 0 smells, 0 nitpicks

## Pass 2 findings replayed through the real caller

| Finding | Replay | State |
|---------|--------|-------|
| D1 | README step 5 now ends its list with "or a file whose frames this page could not read". A two-frame XA with one Per-frame Functional Groups item shows "This page could not read the frames of this file. The rate is the slider." in the page, with the cell fault "an image attribute is missing or not usable" | closed |
| D2 | Two-frame XA files in headless Chromium, one per folder. Frame Time `-40`, Frame Time `abc`, Frame Time Vector `0\-40`, Recommended Display Frame Rate `-5` and Cine Rate `0` each show "The cine timing in this file is not usable. The rate is the slider.", with the slider enabled and Use the slider hidden | closed |
| S1 | Both listener lines are pinned. Pass 2's MH turns the pin red and the new `window.mjs` step red, "W after a key, a press and a choice: focus on the control true, profile window-left, stored window-left". MJ turns the pin and the keyboard-choice step red | closed |

## Acceptance challenged

- **The unusable line against every Cine refusal.** `cine_timing` reaches `SeriesOpenReason::Cine` only through `frame_time_vector`, `one_positive_ds` for Frame Time and `one_positive_is` for Recommended Display Frame Rate and Cine Rate (`crates/ocelli-dicom/src/series_open.rs:916` to `:1019`). Each of the four was refused in the page as listed under D2, and each gave the new line. The adjacent case, Cine Rate `0` beside Frame Time `40`, plays at Frame Time 40 with the slider disabled, because the first usable source wins and the Cine Rate is never read. README's "the local process refuses" holds for that case.
- **README step 5's three fallbacks.** A step to another file: two files of three frames at Frame Time 40 show the Frame Time line on frames 1, 2, 4 and 5, and "Frame Time applies between frames of one file. The rate is the slider." on frames 3 and 6 with the slider enabled. Any file without a usable Frame Time: no timing, Cine Rate 30 and Frame Time Vector `0\40` show the no Frame Time, frame rate and Frame Time Vector lines, and the four refusals show the unusable line. A file whose frames this page could not read: the per-frame group case above.
- **The plan's cine table row.** `.claude/plans/F-214-design.md:80` now carries the ruled sentence, and it matches `page.html:5212` and the AC10 pin character for character. No other tracked file still carries the old "The Frame Time in this file is not usable" sentence (`git grep`). The table has no row for the `unreadable` line or the file boundary line, but pass 1's rulings D3 and S2 define those and the amendments section says rulings win over the text above, so the table is not contradicted.

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|--------------------------|---------------------------|------------------------------|----------------------|
| `cargo test -p ocelli-folder-viewer --test preview` | archive of 5f9ec1ce | now | 0, 115 passed |
| `cargo clippy -p ocelli-folder-viewer --all-targets -- -D warnings`, `cargo fmt --all --check` | archive of 5f9ec1ce | now | 0, clean |
| `python3 scripts/prose_check.py`, added lines searched for em-dash, en-dash and comment semicolons | worktree at 5f9ec1ce | now | OK, 975 files, none found |
| `bin/ocelli.sh gate viewer-browser` with a scratch `CARGO_TARGET_DIR` | worktree at 5f9ec1ce | now | 0, ALL GREEN, the profile line printed both new observations |
| `observeProfileKeys` and `profileProblems` from the tip's `window.mjs` against each mutant's release build | archive of 5f9ec1ce, `OCELLI_PORT=0`, stopped by PID | now | listed below |
| Scratch Playwright probes of the cine line and the file boundary | release build of the archive, synthetic XA files written by the probe with no patient attributes | now | listed above |
| `pgrep -fl 'chromium\|playwright\|oracle'` before the browser runs | host | now | only idle Playwright MCP servers and implementer agents whose prompts contain the words, no Chromium process |

Mutants, one at a time.

- MH, the `pointerdown` reset removed. Red: `the_page_offers_mouse_profiles` and the new `window.mjs` step.
- MJ, the `keydown` listener setting `false`. Red: the pin and "a choice after a key left the profile ocelli and focus on the control false".
- MK, the old Frame Time sentence restored in `cineText`. Red: `play_uses_the_file_frame_time`, the AC10 pin.
- ML, the Cine arm of `push_cine` mapped to `unreadable`. Red: `inspect_reports_frame_time`.
- MN, the reviewer's own and not on the implementer's list. `one_positive_is` widened from `parsed <= 0` to `parsed < 0`, so a Cine Rate of 0 is accepted. Red: `inspect_reports_frame_time` on the new case, "Cine Rate 0 and no Frame Time: inspect did not report \"cine\":\"unusable\"". `cargo test -p ocelli-dicom` stayed green under MN, so this viewer fixture is the only guard on a zero Cine Rate. That library gap predates this range.
- MO, the reviewer's own. The `profileKeyed = false` reset inside the `change` handler removed. Survived the pins and the gate step. It is treated as equivalent: the flag is read only in that handler, and every route to a `change` that the reviewer could construct passes a `keydown` or a `pointerdown` on the control first, which sets the flag either way. The one route that passes neither, a list opened from the keyboard and picked with the mouse, already arrives with the flag true in both versions. It is recorded here and not raised.

## Defects

None.

## Smells

None.

## Nitpicks

None.

## Verified clean

- The README sentence's added claims: the four attributes named are exactly those `read_timing` refuses with the Cine reason, and each was observed to give the unusable line.
- The `window.mjs` header comment's two claims were each observed on the tip: the keyboard choice kept the focus on Ocelli, and the key, press and choice ended blurred on Scroll on left with the stored key `scroll-left` and W/L active.
- The Rust comment above the two pinned listener lines, and the one above the AC10 pin, describe the lines they pin.
- The new fixture's comment. Cine Rate (0018,0040) is in the Cine Module, PS3.3 C.7.6.5, as frames per second with VR IS, so 0 is not a rate.
- `profileProblems` refuses a run that did not observe either new state.
- No `as` cast, `#[allow]`, `unsafe` or `wasm-bindgen` in the range. Nothing under `crates/ocelli-wasm` or `packages` changed, so the wasm size is unaffected. `use.html` and `more.html` carry no sentence the range made stale.

## Limitations and handoff

- Headless Chromium does not render a native `<select>` popup, so the pointer choice is a dispatched `pointerdown` and a programmatic choice. The gate step rests on the event order, not on a real popup.
- Not exercised: Play while another viewport is outlined, switching series while playing, Firefox and Safari.
- Owed: AC12, an operator click-through in real Chrome with a monochrome multi-frame XA or NM with a real Frame Time, including a mouse choice after a key on the control, and a list opened from the keyboard then picked with the mouse, each followed by W. The `docs/lld/series-open.md` entry for `cine_timing` is still owed at `/complete-feature`.
- This is a clean bounded review of the range. It is not completed acceptance and not sprint readiness.
