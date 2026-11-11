# F-214 review, pass 1

**Reviewed**: range 1a52c504..4b2f5d40 on work/f-214-xai, tip 4b2f5d40 (commits 4c14691b and 4b2f5d40). The worktree was clean apart from the untracked `.venv` and `tools/oracle/out`, which are not part of the story.
**Author / reviewer**: written by a separate Claude implementer agent. Reviewed by an independent Claude reviewer that wrote none of it. Every mutation and probe ran on a `git archive` copy of the tip in the reviewer's scratch directory, with its own `CARGO_TARGET_DIR`.
**Result**: 4 defects, 2 smells, 3 nitpicks

## Acceptance challenged

- **AC1, AC2, AC12 (bindings half).** Each profile was driven through the real page in headless Chromium with real mouse events: left, middle, right and Ctrl with left, reading `drag.mode` on the press and the effect after release. All twelve cells match the plan's table. A wrong `pointerMode` that read the old fixed binding would have shown middle zoom and right pan in `scroll-left`, which the probe saw were W/L and zoom.
- **AC3, AC4.** Stored `radiant`, `constructor`, `__proto__` and the empty string all gave `ocelli`, and fps values `45`, `7.5`, `0` and `abc` all gave 10. Nothing was written back. A throwing `localStorage` getter, and throwing `getItem` and `setItem`, both left the page at `ocelli` and 10, showed the unsaved sentence after a choice, still toggled a side column and opened a folder.
- **AC5 to AC8.** Derived independently from PS3.3 C.7.6.5. Frame Time (0018,1063) is DS, the nominal time per frame in milliseconds, so DS 40 is 40 ms and 1000 / 40 is 25 per second. C.7.6.5.1.1 makes it an interval between frames of one multi-frame image, and its note allows 0 when there is one frame. C.7.6.5.1.2 makes the Frame Time Vector one value per frame with a first value of 0. Recommended Display Frame Rate (0008,2144) and Cine Rate (0018,0040) are IS in frames per second. The tests' expectations agree with this, as does the source.
- **AC10 and amendment 1, through the real caller.** A two-file series, file a at Frame Time 40 and file b at Frame Time 100, three frames each, with the slider at 5. Play measured five loops of `a1->a2 40 to 41 ms`, `a2->b0 200 to 202`, `b0->b1 100 to 101`, `b2->a0 200 to 202`. After Use the slider every step measured 200 to 202 ms, play kept running, and the slider was enabled again. A mutated `fileInterval` that ignored the file would have given 40 or 100 ms across the file boundary.
- **The other lines.** A file with no Frame Time, a one-frame file with Frame Time 40, and a three-frame file with Frame Time 0 showed the plan's lines and did not offer Use the slider. A single 30-frame file at Frame Time 1 showed "Playing slower than the file asks." while playing.

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|--------------------------|---------------------------|------------------------------|----------------------|
| `cargo test -p ocelli-dicom --test series_open` | archive of 4b2f5d40 | now | 0, 38 passed |
| `cargo test -p ocelli-folder-viewer --test preview` | archive of 4b2f5d40 | now | 0, 114 passed |
| `cargo clippy -p ocelli-dicom -p ocelli-folder-viewer --all-targets -- -D warnings`, `cargo fmt --check` | archive of 4b2f5d40 | now | 0, clean |
| `python3 scripts/prose_check.py` | worktree at 4b2f5d40 | now | OK, 973 files |
| Twelve mutants, `scratchpad/mutate.py` | archive, one mutant at a time, restored and compared with `cmp` afterwards | now | 10 red, 2 survived, listed below |
| Scratch edge probe inside `tests/preview.rs` | archive, removed afterwards | now | listed under D3 and S1 |
| Five Playwright probes against the release binary, `OCELLI_PORT=0` | archive build, synthetic XA and RGB files written to a temp folder and removed | now | listed above and under each finding |
| `cargo tree -p ocelli-wasm` | archive | now | no `ocelli-dicom` in the graph |
| fmt, workspace clippy, npm lint and test, gate viewer-browser, gate wasm, guard_census | implementer's progress note | prior receipt, not rerun | reported green |

Mutants. Red: M1 drop the one-frame zero rule, M2 read Preferred Playback Sequencing in `cine_timing`, M3 report seconds, M4 report `Unavailable` as `unusable`, M5 report a vector as `frame-time`, M6 apply the one-frame rule to two frames, M7 try Cine Rate before Frame Time, M8 pass a frame count of 1 to `read_timing`, M9 a bare `getItem` outside `try`, M10 drop the same-file check in `fileInterval`. M6, M7, M8 and M10 are the reviewer's own and were not in the implementer's list. Survived: M11, a projection failure reported as `none`, and M12, the one-frame rule's `== 0.0` widened to `<= 0.0`. Both are under S1.

## Defects

### D1, typing a tool key while the profile control has focus changes and keeps the mouse profile

**Where**: `examples/folder-viewer/src/page.html:1278` (the `change` listener) and `page.html:5096` (`typingTarget`, which does not count a `SELECT`)
**What**: a click on the Profile control leaves it focused. The page's tool keys still run, and the browser's type-ahead on the focused `<select>` also runs. Pressing W picks the W/L tool and also moves the control to "Window on left", which fires `change`, so `setProfile` rebinds middle and right and writes `ocelli.mouse`.
**Why it is wrong**: the plan's state table has one route to a profile change, the reader choosing one. A rebinding nobody asked for, kept across reloads, means the middle and right buttons do something the reader did not choose.
**Evidence**: probe 5. Profile `ocelli`, focus on `#mouse-profile`, press `w`. Result: `activeProfile` "window-left", `tool` "wl", `localStorage["ocelli.mouse"]` "window-left".
**Scope and regression**: present at the tip. Fix: have `typingTarget` return true for `SELECT`, so page keys do not run while the control has focus, and blur the control after a pointer choice so the next W reaches the page and not the type-ahead. The regression check is a browser step: choose a profile, press W, and confirm the profile and the stored key did not change.

### D2, `more.html` says the page keeps two choices, and it keeps four

**Where**: `examples/folder-viewer/src/more.html:383`, pinned by `examples/folder-viewer/tests/preview.rs:818`
**What**: "The image page keeps two choices in this browser, the mouse profile and the play rate." The page also keeps whether each side column is hidden, under `ocelli.no-series` and `ocelli.no-inspector`, through the same `readPreference`.
**Why it is wrong**: a false sentence in a user document is a defect under microscope section 3. The plan wrote this sentence, so the plan has the same error.
**Evidence**: probe 4. Hide the series column, reload. `body.no-series` is still set and `localStorage` holds `ocelli.no-series`.
**Scope and regression**: say what the page keeps, for example "The image page keeps the mouse profile, the play rate and whether each side column is hidden, in this browser." Then update the AC11 pin.

### D3, a frame-declaration refusal is reported as "The Frame Time in this file is not usable"

**Where**: `examples/folder-viewer/src/preview.rs:3029`, read by `cineText` in `page.html`
**What**: `Some(Err(_)) | None => ("unusable", None)` maps every `cine_timing` error, including `SeriesOpenReason::Metadata` for a malformed Number of Frames or functional-group sequence, plus a failed projection, to `unusable`. The page then says the file's Frame Time is not usable, even for a file that has no Frame Time.
**Why it is wrong**: the plan's table assigns `unusable` to "unusable, reason `Cine`" only. The sentence makes a claim about (0018,1063) that the data set does not support.
**Evidence**: scratch probe. Two frames, no timing attribute, and a Per-frame Functional Groups Sequence holding one item. Reply: `"cine":"unusable","frameTimeMs":null`. Mutant M11 changes the `None` arm and survives, so nothing pins this path.
**Scope and regression**: reachable only for a malformed data set, which is often refused elsewhere too. Fix: match `Err(e) if e.reason() == SeriesOpenReason::Cine` to `unusable`, and give any other refusal a value and line that make no claim about Frame Time. Add a fixture with no timing attribute and a bad frame declaration.

### D4, after a reload on a kept non-default profile the toolbar shows Scroll while the tool is W/L

**Where**: `examples/folder-viewer/src/page.html:726` (`class="icon active"` on `tool-scroll` in the markup), with `let tool = PROFILES[activeProfile].start` and no tool-button sync before a folder opens
**What**: choose Window on left, then reload. `tool` is "wl" and the bottom sentence is the W/L sentence, but the active chip is Scroll. The chip corrects itself only when a folder opens, because `syncControls` calls `syncToolButtons`.
**Why it is wrong**: the visible state disagrees with the tool that left drags will use, and the README says the mouse profile picks the tool that starts active.
**Evidence**: probe 2. Before opening a folder: `{ tool: "wl", chip: ["tool-scroll"] }`. After opening: `["tool-wl"]`.
**Scope and regression**: low impact, because nothing can be dragged before an image is shown. Fix: call `syncToolButtons()` once after `setLayout(1, 1)` at `page.html:1468`, or drop the hard-coded `active` from the markup and sync at load. Check by reloading on `window-left` and reading the active chip before opening anything.

## Smells

### S1, two refusal boundaries are not pinned

`frame_time_is_zero` at `crates/ocelli-dicom/src/series_open.rs:314` uses `== 0.0`, which is correct: a one-frame Frame Time of -40 must stay `Cine`, and the scratch probe showed it does, as `unusable`. Mutant M12, `<= 0.0`, turns a negative one-frame Frame Time into `Unavailable` and survives both suites. `cine_timing_single_frame_zero_is_unavailable` (`crates/ocelli-dicom/tests/series_open.rs:693`) should add a one-frame `-40` expecting `SeriesOpenReason::Cine`. M11, under D3, is the second gap.

### S2, the file rate is offered on `reported > 1`, not on whether another frame of the file is listed

`page.html:5186` and `page.html:5199` decide on `image.reported > 1`. A non-monochrome multi-frame file is listed as one image (`listed` is 1 unless `info.monochrome`), so Play never steps between its frames. Even so, the line says "Frame Time 40 ms, 25 per second, from the file" and the slider is disabled, though the slider sets every step. Probe 6: two RGB files of three frames at Frame Time 40, slider at 5. The line claims 25 per second, the slider is disabled, and the measured steps are 200 to 202 ms. Today the cell also shows "this preview shows monochrome frames only", which is why this is a smell and not a defect. But the condition will mislead as soon as colour cine is listed one way and played another. Key the condition on the upcoming image being another frame of the same file, which is what `fileInterval` already tests.

## Nitpicks

- N1. `examples/folder-viewer/tests/preview.rs:2325` says the helper keeps the data set in tag order for every cine attribute. AC5's call at line 2421 passes (0018,1063) before (0018,0040), which is not ascending (PS3.5 7.1). The parser accepts it, as the probe showed: an out-of-order Cine Rate was read as `rate`. Swap the two elements.
- N2. README step 6, line 269, says the profile picks the tool that starts active. It does not say that choosing a profile also switches the current tool, for example from Length to Scroll, as `setProfile` does.
- N3. Unconfirmed. A Frame Time above 2,147,483,647 ms is passed straight to `setTimeout`, which browsers clamp. A 16-character DS can spell such a value. Not reproduced, and the input is absurd.

## Judgement of the six adaptations

1. `cine_timing(&MetadataSet)`. Accepted. It keeps one projection per post and reuses `read_timing`, so there is no second parser. Amendment 2 asked for the timing alone, and M2 proves Preferred Playback Sequencing is not read. `use.html` names the real signature.
2. Fields in the frame-independent part. Accepted. Timing is a property of the object, C.7.6.5.1.1, so it belongs in the shared middle. `each_file_reports_its_own_frame_time` shows frame 3 carries it.
3. AC3's red from a mutation. Accepted. M9 goes red. The base already wrapped both storage lines in `try`.
4. README step numbers 5 and 7. Accepted. Checked against the tree.
5. The one-frame line. Accepted. It follows amendment 1 and reads true. Probe line D showed it.
6. `docs/lld/series-open.md` left to `/complete-feature`. Accepted as a deferral, and listed as owed below.

## Cornerstone 5.8.2 floor

No floor claim or named gap from the implementer is in the tree (plan, progress note, commit messages or docs), so this checks the floor directly against `tools/oracle/node_modules/@cornerstonejs/*` 5.8.2 in the main checkout.
- Cine. `tools/dist/esm/utilities/cine/playClip.js` plays at a caller-given `framesPerSecond`, default 30, with `setInterval(1000 / fps)`. It plays a Frame Time Vector only when the caller passes one, and it offers `reverse`, `bounce` and `loop`. The wadouri `cineModule` exposes only `frameTime` (`metaDataProvider.js:150`), and the stack path does not read it itself. `VideoUtilities.js:49-56` prefers Cine Rate, then Recommended Display Frame Rate, then 1000 / Frame Time. The page meets or exceeds the floor on Frame Time. The gaps against cornerstone are vector playback, reverse and bounce, which the plan declares out of scope. Cornerstone's precedence is the reverse of Ocelli's F-192 order, which PS3.3 does not settle.
- Bindings. `tools/dist/esm/enums/ToolBindings.js` and `ToolGroup` bind any tool to any button mask with modifier keys and have no default profile. The page's three fixed profiles are a subset of what the library allows an application. Arbitrary rebinding is a plan non-goal.

## Size and speed

- wasm. `cargo tree -p ocelli-wasm` contains no `ocelli-dicom`, the diff touches nothing under `crates/ocelli-wasm` or `packages`, and `Cargo.lock` is unchanged. A 0 byte delta follows from the dependency graph. The module was not rebuilt in this pass.
- Speed. The implementer's timing figures are not in the tree, so none could be checked. Measured here: playback intervals as above, within about 2 ms of the file's interval on 16 by 16 frames. `cine_timing` builds a second `MultiframeMetadata` per posted object beside the one `inspect` already builds. Its cost was not measured.

## Verified clean

- No `as` cast, `#[allow]`, `unsafe` or `wasm-bindgen` in the diff. `f64` `Display` writes no exponent, and the value is finite because `parse_decimal_string` rejects non-finite input.
- The per-second label for 40, 100, 33.3, 33.333333, 1 and 0.001 ms reads 25, 10, 30, 30, 1000 and 1000000, which is 1000 / ms to one place with a trailing `.0` removed.
- Middle-button W/L in `scroll-left` commits through the same `mode === "wl"` path in `onPointerUp` that F-213 wrote, so amendment 5 holds.
- Profile change from a drawing tool cancels the draft through `setTool`.
- README steps 5 and 7 and the `use.html` sentence were checked against the probes and the source, apart from N2.
- Prose check clean. No em-dash or en-dash in added lines.

## Limitations and handoff

- AC12 is still owed: an operator click-through in real Chrome, including a monochrome multi-frame XA or NM with a real Frame Time. Headless Chromium with synthetic files is not that.
- The `docs/lld/series-open.md` entry for `cine_timing` is still owed at `/complete-feature`.
- Not exercised: Play while another viewport is outlined, switching series while playing, the FPS write failing on its own, Frame Time Vector and rate lines in the browser (covered by unit tests only), the FRAME_LIST_CAP truncation, and gate viewer-browser itself. A scratch probe replaced the gate.
- The wasm module was not rebuilt, and the size claim rests on the dependency graph.
- Next: the implementer remediates D1 to D4 and S1 to S2, and then a fresh independent pass 2 reviews the changed material.
