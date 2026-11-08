# F-214, Folder viewer mouse profiles and preferences

**Status**: approved
**Epic ref**: E38.24
**Sprint**: S46
**Estimate**: 2w

## Normative source, transcribed

The HLD does not specify this page. The rules it does give, and this plan keeps, are these.

`docs/hld/07-concurrency-and-typescript.md` section 10, "What stays TypeScript", quoted in a fence because the author's text carries semicolons:

```text
- DOM, pointer, touch and wheel events; canvas lifecycle; ResizeObserver
- Tool interaction state machines; the geometry and hit-testing behind them is Rust, in ocelli-geom
```

A mouse binding is a pointer-event decision, so it stays in the page. No pointer event crosses into Rust.

The cine source is DICOM, and Ocelli already reads it once. `crates/ocelli-dicom/src/series_open.rs`, F-192, declares the timing type and its order of precedence:

```rust
pub enum CineTiming {
    /// No timing attribute was present.
    Unavailable,
    /// Frame Time Vector `(0018,1065)`.
    ///
    /// PS3.3 C.7.6.5.1.2 says the first component is zero. Each later component
    /// is a positive interval in milliseconds.
    FrameTimeVector,
    /// Frame Time `(0018,1063)`, one interval in milliseconds.
    FrameTime {
        /// The interval in milliseconds.
        milliseconds: f64,
    },
    /// Recommended Display Frame Rate `(0008,2144)`.
    RecommendedDisplayFrameRate {
        /// Frames per second.
        frames_per_second: f64,
    },
    /// Cine Rate `(0018,0040)`.
    CineRate {
        /// Frames per second.
        frames_per_second: f64,
    },
}
```

`read_timing` tries Frame Time Vector first, when its length equals the frame count, then Frame Time, then Recommended Display Frame Rate, then Cine Rate. A present but unusable value is `SeriesOpenReason::Cine`, and the test `series_open_frame_time_zero_does_not_use_the_rate` pins that Frame Time `0` does not fall through to a rate. `DisplaySeries::timing` is "Cine timing from the first frame in display order". The tag is DS, PS3.3 C.7.6.5, in milliseconds.

The operator's note on the story, from `docs/sprints/allocation.json`: "Button bindings differ between viewers that radiologists use, so the binding is a preference rather than one fixed choice." The operator's brief cites the research file section 2. RadiAnt binds left to scroll, middle to window and level, right to zoom. Weasis binds left to window and level and middle to pan. That file is not in this repository, so this plan quotes the brief and not the file.

## What the specification does not cover

Everything about the profiles. The HLD names no binding, no profile and no preference store. The choices below are this plan's.

The three profiles. Left is always the chosen tool, because a drawing tool must stay reachable from the toolbar in every profile. A profile sets which navigation tool left starts on, and what middle and right do. Ctrl with left stays Pan in every profile, so a trackpad with one button can still pan.

| Profile id | Name on the page | Left starts on | Middle drag | Right drag | Ctrl and left |
|---|---|---|---|---|---|
| `ocelli` | Ocelli | Scroll | Zoom at the press | Pan | Pan |
| `scroll-left` | Scroll on left | Scroll | Window | Zoom at the press | Pan |
| `window-left` | Window on left | Window | Pan | Zoom at the press | Pan |

`ocelli` is the binding `pointerMode(event)` has today and is the default. `scroll-left` follows the RadiAnt row of the brief. `window-left` follows the Weasis row. The brief gives no right button for Weasis, so right is Zoom here, which is an open question.

The wheel is not part of a profile. It keeps F-207's rule, slice without Ctrl and zoom with Ctrl or a pinch.

Storage. One key, `ocelli.mouse`, holds a profile id. The page already keeps `ocelli.no-series` and `ocelli.no-inspector` under the same prefix. Every read and write of `localStorage` is inside `try` and `catch`, including the two that exist today. A missing key, an unknown value, or a thrown accessor gives `ocelli`. A failed write keeps the chosen profile for this page load and shows the sentence "This browser did not keep the choice. It applies until you reload." The slider rate is also kept, as `ocelli.fps`, an integer from 1 to 30. Anything else read from that key gives 10.

Cine. Play uses the file's Frame Time when the open series has one, and the slider otherwise. The page shows which. The rules:

| Timing of the series | Interval used by Play | Line beside the slider |
|---|---|---|
| `FrameTime { milliseconds }` | `milliseconds` | `Frame Time 40 ms, 25 per second, from the file` |
| `FrameTimeVector` | the slider | `This file gives a Frame Time Vector, which this page does not play. The rate is the slider.` |
| `RecommendedDisplayFrameRate` or `CineRate` | the slider | `This file gives a frame rate and no Frame Time. The rate is the slider.` |
| `Unavailable` | the slider | `This file gives no Frame Time. The rate is the slider.` |
| unusable, reason `Cine` | the slider | `The cine timing in this file is not usable. The rate is the slider.` |

The per-second figure is `1000 / milliseconds`, shown to one decimal place and trimmed of a trailing `.0`. That is display only. The loop waits `milliseconds` minus the time the step took. While the file's timing is in use the slider is disabled, and a "Use the slider" toggle hands the rate back. A step that takes longer than the interval is not skipped, so playback can run slower than the file asks, and the line says so while playing. No frame is dropped to keep the rate.

The series timing is the timing of `series.images[0]`, the first image in display order, which is `DisplaySeries::timing`'s own rule.

Playback stays loop. Preferred Playback Sequencing, sweep, is out of scope.

## Problem and reproducer

`cargo run -p ocelli-folder-viewer`, open a folder. Left drag runs the chosen tool, middle zooms, right pans, and nothing changes that. A reader used to RadiAnt or Weasis has to relearn the mouse. Open a multi-frame ultrasound or XA object with Frame Time 33.3. Play runs at the slider's 10 per second. `inspect_json` does not report any cine field, so the page cannot know.

## Scope, API and compatibility

In scope:

- A `MOUSE` select in the inspector with the three profiles, the `PROFILES` table in the page, and `pointerMode(event)` reading it.
- The `MOUSE` key list re-rendered from the active profile.
- The two keys and the guarded storage above, and wrapping the two existing panel reads and writes.
- `inspect_json` gains `"cine"` and `"frameTimeMs"`.
- Play uses the file interval as above.
- One sentence in `more.html` `#preferences`: "The image page keeps four choices in this browser, the mouse profile, the play rate and whether each of the two side columns is hidden. There is still no preferences module in the crates or packages." The `not yet` mark stays, because the module it names still does not exist.

Out of scope: key rebinding, per-tool bindings, sweep playback, Frame Time Vector playback, preferences for presets or layouts, and the React example's preferences, which F-205 owns.

The `inspect_json` fields. `cine` is one of `"frame-time"`, `"vector"`, `"rate"`, `"none"` and `"unusable"`. `frameTimeMs` is the DS value as a JSON number when `cine` is `"frame-time"`, and `null` otherwise. Existing fields are unchanged, so `inspect_json_reports_the_series_fields` keeps passing.

Provider choice. `read_timing` is private to `ocelli-dicom`. The two ways to reach it:

- A. Call `assemble_display(&[ParsedFrame::new(&parsed, 0)])` from `inspect_json` and read `timing()` from the one group. No crate API change. A geometry refusal on that object also loses the timing, and the page would have to report "not readable" for a file whose Frame Time is fine.
- B. Add `pub fn cine_timing(parsed: &ParsedDicom) -> Result<(CineTiming, Playback), SeriesOpenError>` to `series_open.rs`. It projects the metadata and calls the existing `read_cine` with the declared frame count. No second Frame Time parser. `assemble_display` is unchanged.

This plan recommends B, because A ties cine to geometry. The choice is in Open questions.

## Consumer and provider map

| Consumer / entry point | Concrete provider | Caller context / policy | Observable output |
|------------------------|-------------------|-------------------------|-------------------|
| `pointerMode(event)` | `PROFILES[activeProfile]` in the page | Left is `tool`, Ctrl with left is Pan in all profiles | The drag mode |
| Profile select | `setProfile(id)` | Sets `tool` to the profile's start tool, writes `ocelli.mouse` | The MOUSE list and the active tool chip |
| Page load | `readPreference(key, allowed, fallback)` | try and catch, closed set of values | The profile and the slider rate |
| `/inspect` | `inspect_json` then `cine_timing` (B) | Frame 0 of the posted object | `cine` and `frameTimeMs` |
| `startPlay` | `series.images[0].info.cine` and `frameTimeMs` | The file interval unless "Use the slider" is on | The step interval and the timing line |
| Wheel | unchanged F-207 `onWheel` | Not part of a profile | unchanged |
| W/L on middle or left | unchanged `dragWindow` path in `onPointerMove` | The existing "no window yet" guard in `onPointerDown` | `overrideWindow` on release |

The page owns the unit change from milliseconds to the per-second label. Rust owns parsing the DS.

## State transitions and invariants

| Entry point / starting state | Input or event | Resulting state and output | Refusal and preserved state |
|------------------------------|----------------|----------------------------|-----------------------------|
| Profile `ocelli`, tool Length | Choose `window-left` | Profile `window-left`, tool W/L, MOUSE list shows Middle Pan, Right Zoom | A thrown `setItem` keeps the profile for this load and shows the sentence |
| Profile `scroll-left`, image shown | Middle drag | Window drag. Release commits `overrideWindow` | No window in the file and none chosen: the press is ignored, as left W/L is today |
| Drawing press in progress | Change profile | The draft is cancelled first, then the profile changes | No half drawn mark is kept |
| Page load, `ocelli.mouse` is `"radiant"` | Read | Profile `ocelli` | Nothing is written back |
| Page load, `localStorage` throws | Read | Profile `ocelli`, rate 10 | The page still opens a folder |
| Series with Frame Time 40 | Play | Steps every 40 ms less the step time. Slider disabled | Pressing "Use the slider" switches to the slider without stopping play |
| Playing at the file rate | Open another series with no Frame Time | Play stops, as `selectSeries` does today. The line changes | The next Play uses the slider |
| Frame Time `0` | `/inspect` | `cine` is `"unusable"`, `frameTimeMs` null | Not replaced by a rate, as in `series_open_frame_time_zero_does_not_use_the_rate` |

## Ownership, commit and recovery

| Resource or visible state | Acquired or staged by | Lifetime identity | Observable commit point | Failure and duplicate-operation behavior |
|---------------------------|-----------------------|-------------------|-------------------------|------------------------------------------|
| `activeProfile` | `setProfile` | The page load | The MOUSE list repaints | Choosing the active profile again writes again and changes nothing visible |
| `ocelli.mouse`, `ocelli.fps` | `writePreference` | This browser origin, which is the loopback port | `setItem` returns | A throw is caught. The in-page value stays |
| Play loop | `startPlay` with `playToken` | The token | Each `showIndex` | A stale loop exits on token change, as today |

The origin is `http://127.0.0.1:<port>/`. The port is ephemeral unless `OCELLI_PORT` is set, so a stored profile only survives a restart when the port is the same. The MOUSE section says so in one line.

## Numbered acceptance inventory

| ID | Trigger and expected behavior | Test / observation | Wrong implementation rejected | Evidence stage / owner |
|----|-------------------------------|--------------------|-------------------------------|------------------------|
| AC1 | The page offers the three profile ids and names | `the_page_offers_mouse_profiles` reads `id="mouse-profile"` and the three `value=` strings | One fixed binding | implementation |
| AC2 | `pointerMode` reads the profile table and keeps Ctrl with left as Pan | Same test pins the `PROFILES` literal and the Ctrl branch | A profile that drops pan for a one-button trackpad | implementation |
| AC3 | Every `localStorage` access is inside `try` | `every_storage_access_is_guarded` scans each line holding `localStorage.` and requires `try {` on that line or the line above | The two existing reads left as they are, or a new bare `getItem` | implementation |
| AC4 | An unknown stored value gives `ocelli` | Same page test pins the allowed-set check in `readPreference` | Using the stored string as a key and reading `undefined` | implementation |
| AC5 | Frame Time 40 is reported as 40 ms | `inspect_reports_frame_time` posts a frame with `(0018,1063)` DS `40` and reads `"cine":"frame-time"` and `"frameTimeMs":40`. PS3.3 C.7.6.5, 1000 / 40 is 25 per second | Reading Cine Rate, or reporting seconds | implementation |
| AC6 | No timing gives `none` and null | Same test without the tag | `0` in place of null | implementation |
| AC7 | Frame Time `0` with Recommended Display Frame Rate `10` gives `unusable` | Same test | Falling through to the rate | implementation |
| AC8 | A Frame Time Vector of matching length gives `vector` | Same test, frames 2, vector `0\40` | Playing the vector's second value as one interval | implementation |
| AC9 | `cine_timing` agrees with `assemble_display` on every timing fixture | `series_open.rs` test `cine_timing_matches_display_timing` over the existing cine fixtures | A second parser that disagrees on `0` | implementation (B only) |
| AC10 | `startPlay` uses `frameTimeMs` when present and not overridden | Page test pins the branch that reads `frameTimeMs` before `fpsInput` | Converting to fps and rounding to the slider's integer | implementation |
| AC11 | `more.html` says the page keeps four choices and that no preferences module exists | `more_and_use_are_static_pages` extended | Claiming a preferences module | implementation |
| AC12 | A live browser drag in each profile does what the table says, and a file with Frame Time plays at that rate | Click-through on Chrome against a running viewer | Anything the string pins cannot see | external, sprint close, operator |

The Rust tests read page strings. They do not execute the page. AC12 is not claimed by any Rust test and stays open until someone performs it.

## Approach

Replace the body of `pointerMode` with a lookup in a frozen `PROFILES` object keyed by profile id and button. Add `readPreference` and `writePreference`, each one `try` and `catch`, and route the two existing panel reads and writes through them. Add the select to the MOUSE section and render its key rows from the table. In `preview.rs`, call the provider and push the two fields after `edgeBottom`. In `startPlay`, compute the interval once per step from the series timing or the slider.

Rejected: storing the whole binding table in `localStorage`, which lets a stale or edited value bind a button to a mode the page does not have. Rejected: parsing Frame Time in the page from the headers JSON, which is a second parser.

## Dependencies, write ownership and resources

Declared: F-207, done. Source-level: `ocelli-dicom::series_open` from F-192, done.

Writes `examples/folder-viewer/src/page.html`, `src/more.html`, `src/preview.rs`, `tests/preview.rs`, and under B `crates/ocelli-dicom/src/series_open.rs`, `crates/ocelli-dicom/src/lib.rs` and `crates/ocelli-dicom/tests/series_open.rs`.

`page.html` is also written by F-208, F-209, F-211, F-213, F-215 and F-216 in this sprint, and `preview.rs` by F-210, F-212, F-213, F-215 and F-216. These run serially. **Sprint build order, decided in the S46 design round on 2026-11-05.** F-217, F-216, F-210, F-211, F-208, F-209, F-212, F-213, then this story, then F-215. F-119 is independent. F-208 therefore lands first and moves view state per viewport, so the profile stays page-wide and Play reads the active viewport's series. The `cine` and `frameTimeMs` fields reach the page through F-216's `POST /file`, whose body is byte-identical to `/inspect`. The profile table binds mouse buttons only, so it does not collide with F-208's `G` and `Shift G`, F-211's tool letters or F-213's digits. No GPU.

## Verification and evidence reuse

`bin/ocelli.sh check ocelli-folder-viewer`, and `bin/ocelli.sh check ocelli-dicom` under B. Then `cargo test -p ocelli-folder-viewer --test preview`. Mutation: change `40` in the fixture's expected JSON to `25` and see AC5 go red. Remove the `try` around one `getItem` and see AC3 go red. The sprint gate is the later whole-sprint run. A page edit after that record invalidates it.

## Boundary and tier

- wasm-bindgen: not touched
- Pixels across the boundary: no wasm crossing. The page still receives grey bytes from the local process.
- Render-loop allocation: none. The play loop allocates nothing new per step.
- unsafe: none
- Tier A (WebGPU): n/a. This page does not open a device.
- Tier B (WebGL2): n/a
- Tier C (CPU): full and unchanged. The grey comes from F-206's CPU preview. Profiles and cine change which frame is asked for and when, not its pixels.

## Tests

| Category | What it proves | Where |
|----------|----------------|-------|
| contract | The three profiles, the table, guarded storage, the Play branch | `examples/folder-viewer/tests/preview.rs` |
| fixture | Frame Time 40 ms, PS3.3 C.7.6.5, and the four other timing outcomes | `examples/folder-viewer/tests/preview.rs` |
| unit | `cine_timing` equals `DisplaySeries::timing` on the existing fixtures | `crates/ocelli-dicom/tests/series_open.rs` (B) |

No pixel arithmetic. The per-second label is display text and is not a measurement.

## Parity surface covered

none. Appendix B has no row for mouse bindings or cine timing.

## Deviations

none

## LLD impact

Under B, `docs/lld/series-open.md` gains `cine_timing`. The folder viewer is not published API.

## User-facing documentation impact

- `README.md`, "Start the local series viewer". Step 4: Play uses the file's Frame Time when it has one and the FPS control otherwise, with "Use the slider" to override, and the FPS value is remembered in this browser. Step 6: the mouse buttons follow the MOUSE profile chosen in the right column, and the step names the three profiles and that Ctrl with the left button pans in all of them. One sentence that a stored profile survives a restart only on the same `OCELLI_PORT`.
- `examples/folder-viewer/src/more.html`, `#preferences` (already in the write set): the one sentence in Scope, with the `not yet` mark kept.
- `examples/folder-viewer/src/use.html`, chapter 7 "Rust, parse": one sentence naming `cine_timing(&parsed)` beside `open_frame`, because option B adds it as a public `ocelli-dicom` function.

`README.md` and `use.html` join the write set for these edits.

## Open questions

1. **Cine provider, A or B.** Affects AC5 to AC9. A keeps the write set inside the example and loses Frame Time on any geometry refusal. B adds one public function to `ocelli-dicom` that reuses `read_cine`. Recommendation B. Owner: operator. Deferral blocks the `preview.rs` change.
2. **Right button in `window-left`.** Affects AC1 and AC2. The brief gives Weasis left and middle only. Options: Zoom at the press, which this plan takes, or Pan, which would make middle and right the same. Owner: operator. Deferral keeps Zoom.
3. **Rates other than Frame Time.** Affects AC10. The provider also resolves Recommended Display Frame Rate and Cine Rate. Options: use only Frame Time as the story says, which this plan takes, or use the resolved rate too. Owner: operator. Deferral keeps the story's scope.
4. **The research file.** It is cited by the brief and is not tracked. If it should be the source for the profile table, its section 2 belongs in `docs/` so the next plan can quote it rather than the brief. Owner: operator. Deferral leaves the brief as the quoted source.

### Answers

Given by the operator on 2026-11-05 in the S46 consolidated design round and relayed by the sprint lead. The operator accepted the recommended answer, or the plan's stated choice where no separate recommendation was written, for every question.

1. Answered: recommended B, a public `cine_timing` in `ocelli-dicom/src/series_open.rs` that reuses `read_cine`.
2. Answered: the plan's choice, the right button zooms at the press in `window-left`.
3. Answered: the plan's choice, only Frame Time drives the play rate, as the story says.
4. Answered: the plan's choice, the research file stays untracked and the brief stays the quoted source. Moving its section 2 into `docs/` is not part of this story.

No question is left open.

## Lead amendments before implementation, against F-209's tree

Made by the sprint lead on 2026-11-07 from read-only groundwork against `work/f-209-xai` at 3c206f67. Where these differ from the text above, these win. F-214 starts after F-213 lands and re-checks every path and line named here.

1. **Cine timing is per object.** PS3.3 C.7.6.5.1.1 defines Frame Time as the interval between frames of one multi-frame image. The file's interval applies only while stepping through frames of that same file. Stepping between files of a series uses the page's default interval, and a test with two files of different Frame Time shows each plays at its own.
2. **`cine_timing` reads the timing alone,** through `read_timing`, so a bad Preferred Playback Sequencing value never hides a good Frame Time. A fixture with Frame Time 40 and an invalid playback value plays at 40.
3. **AC9 uses single-object fixtures.** New Frame Time Vector fixtures on one multi-frame object, because the existing vector fixtures group two single-frame objects and `assemble_display`'s length check is per group.
4. **AC7's unusable value needs two or more frames.** Frame Time 0 on a single-frame object is legal per the C.7.6.5.1.1 note and reports `none`.
5. **Middle-button window drags go through F-213's window state,** clearing the preset and the file window index the same way the left-button window drag does, keyed on the drag mode rather than the button.
6. **Names.** The series loader is `loadSeries`. README steps, and the AC12 file, are measured from the tree. AC12 uses a monochrome multi-frame XA or NM file, because ultrasound cine is usually colour.

### Lead rulings after review pass 1

- **D1.** A key pressed while the profile control has focus does not change the profile. `typingTarget` treats a SELECT as a typing target, and the control is blurred after a pointer choice. A browser step chooses a profile, presses W, and requires the profile and the stored key unchanged.
- **D2.** `more.html`, this plan's sentence and the AC11 pin name all four kept choices: the mouse profile, the play rate, and the two hidden side columns.
- **D3.** `unusable` is reported only for a `SeriesOpenReason::Cine` refusal. Any other refusal gets a line that makes no claim about Frame Time. A fixture covers a non-cine refusal on a file with no timing, red under pass 1's M11.
- **D4.** The tool buttons are synced once at start, so a reload on `window-left` shows W/L active.
- **S1.** A one-frame Frame Time of -40 is a test case expecting the Cine refusal, red under M12.
- **S2.** The file rate is offered only when the next image is another frame of the same file, the test `fileInterval` already makes, and the line and the slider follow it.
- **Nitpicks taken.** N1, the fixture's tag order follows its comment. N2, the README says choosing a profile also switches the current tool. N3 is not taken.

### Lead rulings after review pass 2

- **D2.** A `SeriesOpenReason::Cine` refusal reads "The cine timing in this file is not usable. The rate is the slider.", true for a refused Frame Time, Frame Time Vector, Recommended Display Frame Rate or Cine Rate. The AC10 pin and README step 5 follow, and a fixture with Cine Rate 0 and no Frame Time replies `unusable`. `cine_timing` is not widened to name the failed attribute.
- **D1.** README step 5 names the third fallback, a file whose frames this page could not read.
- **S1.** Both focus-flag listener lines are pinned in `the_page_offers_mouse_profiles`, and `window.mjs` gains a step that dispatches a keydown, then a pointerdown, then a choice, and requires the control blurred and W to pick W/L, red under pass 2's MH.
- **N1 not taken.** It joins the accepted keyboard case, and AC12's click-through covers it on real Chrome.

### Operator showcase status rule, 2026-11-08

The operator requires every user-facing feature in the viewer and asks for
a versioned comparison matrix on More. Status now describes the usable
viewer workflow, separately from library implementation and tier support.
The four persisted choices make Preferences partial, replacing the earlier
not-yet index wording. The absent general preferences module remains named.
No interaction or persistence behaviour changes in this documentation update.
