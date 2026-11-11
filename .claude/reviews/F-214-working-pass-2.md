# F-214 review, pass 2

**Reviewed**: range 1e16dd5b..fce44ad8 on work/f-214-xai, tip fce44ad8, one commit. The worktree was clean apart from the untracked `.venv` and `tools/oracle/out`, which are not part of the story.
**Author / reviewer**: written by a separate Claude implementer agent. Reviewed by an independent Claude reviewer that wrote none of it. Mutations and Rust probes ran on a `git archive` copy of fce44ad8 in the reviewer's scratch directory with its own `CARGO_TARGET_DIR`. The base page came from a `git archive` of 1e16dd5b built into a second scratch target. The implementer's M11 to M21 list is not in the tree, so the overlap with the mutants below could not be checked.
**Result**: 2 defects, 1 smell, 1 nitpick

## Pass 1 findings replayed through the real caller

| Finding | Replay | State |
|---------|--------|-------|
| D1 | The gate's new step, then probes with a real mouse press on the control. Choose, then W: profile and `ocelli.mouse` kept, tool W/L, control not focused. Pass 1's own trigger, focus then W: the page key no longer runs. Adjacent case, a key on the control, then a press and a choice, then W: control blurred, W/L chosen, profile kept | closed |
| D2 | Choose Window on left, FPS 5, `[` and `]`, reload. `localStorage` holds exactly `ocelli.fps`, `ocelli.mouse`, `ocelli.no-inspector` and `ocelli.no-series`, and `body` has both classes. The page writes nowhere else (`grep writePreference\|sessionStorage\|indexedDB`) | closed |
| D3 | A two-frame XA with one per-frame functional group item and no timing shows "This page could not read the frames of this file. The rate is the slider." in the page, and the cell shows the fault "an image attribute is missing or not usable". The same file with Frame Time 40 shows the same line | closed |
| D4 | Stored `window-left`, `scroll-left` and `ocelli`, reload with no folder: the active chip is `tool-wl`, `tool-scroll` and `tool-scroll` | closed |
| S1 | Mutant MC, `== 0.0` to `<= 0.0`, turns `cine_timing_single_frame_zero_is_unavailable` red | closed |
| S2 | Two RGB files of three frames at Frame Time 40, FPS 5. The line is "Frame Time applies between frames of one file. The rate is the slider.", the slider is enabled, Use the slider is hidden, and every step measured 200 to 202 ms | closed |
| N1 | `tests/preview.rs:2443` passes (0018,0040) before (0018,1063), and every other `cine_frames` call is in ascending order | closed |
| N2 | README step 6 now says a profile choice also switches the tool. Length, then Window on left: W/L. Pan, then Ocelli: Scroll | closed |

## Acceptance challenged

- **Check 1, `unreadable`.** Every non-Cine route was driven through `inspect_json`: Number of Frames `ab`, `0`, `-2`, `2\2` and empty all reply `"cine":"unreadable","frameTimeMs":null`. The per-frame group count fixture does the same, in the page as well. `push_cine(&mut out, None)`, the projection failure arm, writes `"cine":"unreadable"`. A real projection failure could not be reached through a parsed file, because `MetadataSet::insert` fails only on a duplicate tag. A Frame Time with VR FL stays `unusable`. The `unreadable` line makes no Frame Time claim. The `unusable` line does make one for refusals that are not about Frame Time, under D2 below.
- **Check 2, the keydown flag.** A key on the control followed by a real mouse press and a choice resets the flag on `pointerdown`, and the control is blurred, so W reaches the page. A key followed by a choice with no press on the control leaves it focused. That is the route of a list opened from the keyboard and then picked with the mouse, which headless Chromium cannot show natively. See N1. Nothing in the suite guards the flag in either direction. See S1.
- **Check 3, window.mjs.** `bin/ocelli.sh gate viewer-browser` runs `window.mjs` (`bin/ocelli.sh:181`), and on the tip it printed the profile line and `ALL GREEN`. The tip's `observeProfileKeys` and `profileProblems`, run against the base 1e16dd5b binary, report all four problems: W changed the profile to window-left, the control kept the focus, P picked Pan, and the reload showed `tool-scroll`.
- **The S2 line at a file boundary.** A two-file monochrome series, file a Frame Time 40, file b Frame Time 100, three frames each, FPS 5. Walking it shows the Frame Time line on frames 1, 2, 4 and 5, and the boundary line on frames 3 and 6 with the slider enabled. Play measured `1->2 40`, `2->3 41`, `3->4 201`, `4->5 101`, `5->6 101`, `6->1 201` over three loops. With Use the slider on, every step measured 200 to 201 ms. This matches the ruling.

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|--------------------------|---------------------------|------------------------------|----------------------|
| `cargo test -p ocelli-dicom --test series_open` | archive of fce44ad8 | now | 0, 38 passed |
| `cargo test -p ocelli-folder-viewer --test preview` | archive of fce44ad8 | now | 0, 115 passed |
| `cargo clippy -p ocelli-dicom -p ocelli-folder-viewer --all-targets -- -D warnings`, `cargo fmt --all --check` | archive of fce44ad8 | now | 0, clean |
| `python3 scripts/prose_check.py`, added lines searched for em-dash, en-dash and comment semicolons | worktree at fce44ad8 | now | OK, 974 files, none found |
| `bin/ocelli.sh gate viewer-browser` | worktree at fce44ad8, scratch target | now | 0, ALL GREEN, profile line printed |
| `observeProfileKeys` and `profileProblems` from the tip's `window.mjs` | release build of 1e16dd5b | now | four problems, listed above |
| Scratch Playwright probes against the release binary, `OCELLI_PORT=0`, stopped by PID | archive build of fce44ad8, synthetic XA files written by pydicom with no patient attributes | now | listed above and under each finding |
| Scratch Rust probes through `inspect_json` and `push_cine` | archive, appended tests removed and the file compared with `cmp` | now | listed under check 1 and D2 |
| Ten mutants, `f214-p2-mutate.py` | archive, one at a time, restored by copy and compared with the worktree | now | 8 red, 2 survived, below |

Mutants. Red: MA `unreadable` arm back to `unusable` (`a_frame_declaration_refusal_makes_no_frame_time_claim`), MB the Cine guard inverted (that test and `inspect_reports_frame_time`), MC the one-frame rule widened to `<= 0.0`, MD SELECT dropped from `typingTarget` (Rust pin and the gate step, "P on the focused control ran a page key"), ME the blur removed (Rust pin and three gate problems), MF the start-up `syncToolButtons()` removed (Rust pin and the gate step), MG `from !== to` dropped from `sameFileNext`, MI `offered` keyed on `reported > 1` again. Survived: MH the `pointerdown` reset removed, and MJ the `keydown` listener setting `false`, which blurs after every keyboard choice too. Both are under S1. MB, MG, MH, MI and MJ are the reviewer's own.

## Defects

### D1, README step 5's list of reasons leaves out the new `unreadable` line

**Where**: `README.md:253` to `README.md:255`
**What**: "A step to another file, and any file without a usable Frame Time, uses the FPS control, and the line says why: a Frame Time Vector, which this page does not play, a frame rate with no Frame Time, no Frame Time, or a Frame Time that is not usable." The range added a fifth line, "This page could not read the frames of this file. The rate is the slider.", and the list was not updated.
**Why it is wrong**: microscope section 3. A change in the diff left a user document stale. A reader who sees the new line finds it in no list.
**Evidence**: the `badfg` and `badfgtimed` probes above show the new line in the page.
**Scope and regression**: README only. Add "or a file whose frames this page could not read" to the list, together with whatever D2's fix changes in the same sentence.

### D2, a refused frame rate or Frame Time Vector is reported as "The Frame Time in this file is not usable"

**Where**: `examples/folder-viewer/src/preview.rs:3031` maps every `SeriesOpenReason::Cine` refusal to `unusable`, and `examples/folder-viewer/src/page.html:5212` gives that value the Frame Time sentence. `README.md:255` repeats it.
**What**: `SeriesOpenReason::Cine` also covers a refused Frame Time Vector (0018,1065), Recommended Display Frame Rate (0008,2144) and Cine Rate (0018,0040). This comes from `frame_time_vector` and `one_positive_is` in `crates/ocelli-dicom/src/series_open.rs:937` and `:999`. A file with no Frame Time and a Cine Rate of 0 is told its Frame Time is not usable.
**Why it is wrong**: this is pass 1's D3 class through the Cine reason. The line claims something about (0018,1063) that the data set does not support. The material predates this range and pass 1 cleared it. It is raised here because it is a false claim a user reads.
**Evidence**: scratch probe through `inspect_json`. Two frames and no Frame Time, with Cine Rate `0`, with Recommended Display Frame Rate `-5`, and with Frame Time Vector `0\-40`. Each replies `"cine":"unusable","frameTimeMs":null`.
**Scope and regression**: reachable only for a malformed timing attribute. Fix: say what is true for every Cine refusal, for example "The cine timing in this file is not usable. The rate is the slider.", and update the AC10 pin and README step 5 to match. A sentence that names Frame Time only for a refused (0018,1063) would need `cine_timing` to say which attribute it refused, and that is a lead ruling. Add a fixture with Cine Rate 0 and no Frame Time that pins the reply and the line.

## Smells

### S1, the focus flag behind check 2 has no guard in either direction

`page.html:1282` and `page.html:1283`. Removing the `pointerdown` reset (MH) leaves both suites and the gate green. Without it, a sequence that real Chrome delivers, Tab to the control, any key, then a mouse press and a choice, keeps the control focused, and the next W moves the profile to Window on left and stores it. That is pass 1's D1 again. Setting the flag `false` on keydown (MJ) also survives. The Rust pin holds only the blur line, and the gate's `selectOption` fires neither event. Fix: pin both listener lines in `the_page_offers_mouse_profiles`. Also add a gate step that dispatches `keydown`, then `pointerdown`, then a choice on `#mouse-profile`, and requires the control blurred and the next W to pick W/L. Confirm that step goes red under MH.

## Nitpicks

- N1. Unconfirmed. A list opened from the keyboard and picked with the mouse fires `keydown` on the control and, if Chrome sends no `pointerdown` to the control for a click in its popup, no reset. The control then stays focused after a pointer choice. Headless Chromium reproduces only the event sequence without the press, which kept the control focused. It is close to the accepted type-ahead case, because the reader used the keyboard on the control. Worth one step in the AC12 click-through.

## Verified clean

- `push_cine`'s doc comment matches the reply for every value it names, and `unreadable` is reached by each non-Cine refusal tried and by `None`.
- The D3 fixture puts (5200,9230) before (7FE0,0010), in tag order. PS3.3 C.7.6.16 does require one Per-frame Functional Groups item per frame, as the comment says.
- The S1 test's -40 case returns `SeriesOpenReason::Cine`, and its comment cites C.7.6.5 correctly.
- `sameFileNext` returns false for a listed image followed by itself, which is a one-image series of one RGB file, and for the step to another file. `fileInterval`, `cineText` and `syncCineLine` all use it, so the interval, the line and the slider agree on every step measured.
- The more.html sentence, the plan's two edited rows and the AC11 pin name the same four choices, and the probe found exactly four keys.
- The window.mjs header comment's claims were each observed: no option starts with P, and the reload shows W/L before a folder opens.
- No `as` cast, `#[allow]`, `unsafe` or `wasm-bindgen` in the range. Nothing under `crates/ocelli-wasm` or `packages` changed, so the wasm size is unaffected.

## Limitations and handoff

- Headless Chromium does not render a native `<select>` popup, so a mouse pick inside the list was approximated by a press on the control and a programmatic choice. S1's sequence and N1 rest on the event order, not on a real popup.
- A real projection failure could not be produced from a parsed file. The `None` arm was exercised by calling `push_cine` directly.
- Not exercised: Play while another viewport is outlined, switching series while playing, Firefox and Safari.
- Owed: AC12, an operator click-through in real Chrome with a monochrome multi-frame XA or NM with a real Frame Time. It should include choosing a profile with the mouse after a key on the control, and opening the list from the keyboard then picking with the mouse, each followed by W. The `docs/lld/series-open.md` entry for `cine_timing` is still owed at `/complete-feature`.
- Next: the implementer remediates D1, D2 and S1, and then a fresh independent pass 3 reviews the changed material.
