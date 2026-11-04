# F-192 review, pass 3

**Reviewed**: uncommitted working tree on sprint/s42, parent 343255d, after the pass 2 remediation
**Author / reviewer**: implementation by the sprint host. Review by Claude Code, read only, with no shell.
**Result**: 1 defect, 0 smells, 2 nitpicks

## Acceptance challenged

- **Pass 2 D1, the rustdoc.** `SeriesOpenReason::Playback` (`series_open.rs:209`) now reads "present with another value representation, or with a value other than 0 or 1". `DisplaySeries::playback` (`:233`) reads "Loop or Sweep from the first frame in display order. US 0 or US 1." Both match `read_playback` (`:951-966`). That function returns `Loop` for an absent attribute, an `Empty` value, `[]` and `[0]`. It returns `Sweep` for `[1]` and refuses every other value and every non-US VR. An empty US has no value, so "a value other than 0 or 1" does not wrongly cover it. A grep for `LOOP|SWEEP` across `crates/ocelli-dicom`, `examples` and the LLD finds only the LLD's record of the design text contradiction (`series-open.md:46`) and the CS refusal fixture (`tests/series_open.rs:276`). Both are correct.
- **Pass 2 D2, whether the tests reach the guarded line.** `FrameGeometry::derive` (`frame_geometry.rs:247-259`) resolves Image Position Patient before it reads orientation or spacing, and it returns `MissingImagePosition` as soon as the position is absent. So an unpositioned slice with a bad spacing does reach `reject_malformed_unpositioned` (`series_open.rs:412-413`). Neither new test hits an earlier error that could hide the guard:
  - With `-1\1`, `plane_decimals` accepts the value, because it is DS, has two values and parses. `PixelSpacing::new` then refuses the negative at `:488-489`.
  - With `abc\1`, `parse_decimal_string` refuses the text at `:520-521`.

  The orientation is `AXIAL` in both fixtures, so the first `if let` block passes. If the second block is removed, nothing else refuses the frame and the result is `Ok`. That agrees with the implementer's receipt, and each test guards a different arm.
- **Pass 2 D3, whether the test reaches `series.rs:82` and discriminates.** The good part is synthetic object 0: axial, z = 3, Recommended Display Frame Rate 30. The bad part is `frame_time_zero_part`: axial, z = 0, Frame Time 0, same Series and Frame of Reference UIDs. Both parse and both declare one frame, so `frames_of` succeeds and `assemble_display` runs. They form one group, and z = 0 sorts first, so `first_frame` is the bad part at input index 1. `read_cine` refuses Frame Time 0 as `Cine`. The response is `{"error":"cine","frame":1}`. If line 82's field is changed to `"part"`, the `"frame":1` assertion and the `!contains("\"part\"")` assertion both fail. The multipart framing in the test matches `split_multipart`: a trailing CRLF before each marker is stripped, and the `--ocelli--` close is detected.

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|---|---|---|---|
| Read the pass 2 review | `.claude/reviews/F-192-series-open-pass-2.md` | executed now | Three defects, three nitpicks |
| Read `series_open.rs:100-299`, `:370-530`, `:640-850`, plus grep for playback | working tree | executed now | Docs at `:209` and `:233` match the reader |
| Read `frame_geometry.rs:245-284` | working tree | executed now | Position is resolved first, so `MissingImagePosition` comes before any spacing read |
| Read `tests/series_open.rs:1-120`, `:470-729` | working tree | executed now | The two spacing tests are at `:523-536`. The fixture writes `part.spacing` at `:676` |
| Read `series.rs` in full and `bytes.rs:115-214` | working tree | executed now | New test at `series.rs:361-391`. The doc comment is attached to the wrong function (D1 below) |
| Grep `LOOP|SWEEP` across the changed crates, examples and LLD | working tree | executed now | Only the LLD's contradiction note and the CS refusal fixture |
| The three remediation tests passed after restore. Clippy on ocelli-dicom and ocelli-example-server exited 0 | implementer | prior receipt, not mine | not reproduced |
| Spacing block removed: both D2 tests `Ok`. Field renamed to `part`: D3 test red | implementer | prior receipt, not mine | not reproduced. My reading agrees each one discriminates |

## Defects

### D1, the "Three synthetic objects" rustdoc is now attached to `frame_time_zero_part`

**Where**: `examples/example-server/src/bytes.rs:122-141`
**What**: The new `#[cfg(test)] pub(crate) fn frame_time_zero_part` was inserted between the existing doc comment and `synthetic_series_parts`. The doc block at `:122-127` therefore attaches to `frame_time_zero_part`. That block says "Three synthetic objects", explains why the button sends three, and describes the axial pair at z = 3 then z = 0 and the sagittal third. The function returns one object, a single axial slice at z = 0 with Frame Time 0. `synthetic_series_parts` at `:141`, the function the paragraph actually describes, now has no doc.
**Why it is wrong**: A false claim in prose is a defect under the severity table. The remediation caused it, and the text was correct in pass 2, where pass 2 cited the sagittal part at `bytes.rs:155`. The new helper's only reason to exist is that Frame Time 0 is refused, and its doc says nothing of the kind. The paragraph that records the departure from the design text ("The design text says the button sends two objects") is now detached from the function that makes the departure.
**Evidence**: `bytes.rs:122-127` is the doc block, `:128` is `#[cfg(test)]`, `:129` is `pub(crate) fn frame_time_zero_part`, and `:141` is `pub(crate) fn synthetic_series_parts` with nothing above it but a blank line. Clippy does not catch this because neither item is `pub` outside the crate.
**Scope and regression**: Example server, prose only, no behaviour change. Move `frame_time_zero_part` below `synthetic_series_parts`, or above the doc block, so the paragraph sits on `synthetic_series_parts` again. Give the helper its own one-line doc, for example "One axial slice at z = 0 whose Frame Time is 0, which assembly refuses as `cine`." No test is needed beyond re-reading the doc.

## Smells

None.

## Nitpicks

- `series.rs:361-391`: the test works only because the bad part sorts first in display order. `read_cine` reads timing from `first_frame` alone (`series_open.rs:677-679`), so a Frame Time 0 slice at z = 3 would be accepted and the request would return 200. Nothing in the test says this. One comment such as "z = 0 sorts before the good part's z = 3, so this frame is the cine source" would stop a later edit to the position from silently turning the test into a 200.
- `series.rs:362`: the name `an_assembly_error_names_the_frame_list_index` claims more than the test shows. Both parts are single-frame, so frame-list index 1 and part index 1 are the same number. The assertions prove the field name, which is what pass 2 asked for, not that the index counts frames rather than parts. This is not a reopening of D3. Either rename the test toward the field, or make the first part two-frame so that `"frame":2` separates the two counts.

## Verified clean

- **Pass 2 D1 closed.** Both public doc comments state the US 0 or 1 contract. No CS wording remains in the code or rustdoc.
- **Pass 2 D2 closed.** Both spacing tests reach `reject_malformed_unpositioned`, because the position is checked first in `derive`. One test reaches each refusal arm, `plane_decimals` grammar and `PixelSpacing::new`. Each expects `Geometry`, and each would be `Ok` without the block.
- **Pass 2 D3 closed in coverage.** `series.rs:82` now runs under test. The field name is asserted both positively and negatively. The test body is real Part 10 objects through the real multipart path, not a shortcut around `posted_reply`.
- **No regression elsewhere from the remediation.** The library change was doc-only, at `:209` and `:233`. The test additions use the existing `slice` builder and the ascending-tag `encode`. `frame_time_zero_part` is `#[cfg(test)]`, so it adds nothing to the shipped binary. Its element order is the same as `series_slice`, which is ascending. The pass 2 nitpicks (zero spacing on the unpositioned path, `-5\60`, empty US) are unchanged and stay nitpicks.

## Limitations and handoff

- **I ran no tests, clippy or mutations.** Every pass and red-run result in the Evidence table is the implementer's receipt. My agreement comes from reading the guarded paths.
- I had no shell, so I did not read `git diff`. I checked the remediation by reading the named files at the cited lines and around them.
- `App.tsx` and `docs/lld/series-open.md` were checked only by grep for LOOP and SWEEP. Neither was re-read in full, because the pass 2 remediation did not touch them.
- Next: the implementer moves the doc block back onto `synthetic_series_parts` and gives `frame_time_zero_part` its own doc. Then a fresh pass 4. The two nitpicks are optional.

F-192, pass 3: 1 defect, 0 smells, 2 nitpicks.
