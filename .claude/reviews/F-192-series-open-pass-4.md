# F-192 review, pass 4

**Reviewed**: uncommitted working tree on sprint/s42 after the pass 3 doc move
**Author / reviewer**: implementation by the sprint host. Review by Claude Code, read only, with no shell.
**Result**: 0 defects, 0 smells, 1 nitpick

## Acceptance challenged

- **Pass 3 D1, which item the doc is attached to.** `bytes.rs:122-127` is the "Three synthetic objects" block, and `:128` is `pub(crate) fn synthetic_series_parts` with nothing in between. `frame_time_zero_part` now sits at `:197-211`, after `series_slice`, with its own doc at `:197-199` directly above `#[cfg(test)]` at `:200`. Each doc now attaches to the function it describes.
- **Whether the restored doc is still true of `synthetic_series_parts`.** I checked it against the body at `:129-157`:
  - Object 0 is instance 1 at `0\0\3`. Object 1 is instance 2 at `0\0\0`. Both use orientation `1\0\0\0\1\0`. So z = 3 is first in the body and z = 0 is second, and display order by position runs against instance number. That matches the doc.
  - Object 2 has orientation `0\1\0\0\0\-1`. Its row direction is +y and its column direction is -z, which is sagittal. That matches "The third object is sagittal."
  - Two orientations give two groups, and the axial group has two slices, so the body has three objects. That matches the design departure the doc records.
  - The text is the same as the version pass 2 verified.
- **Whether the new doc is true of `frame_time_zero_part`.** The body at `:202-210` is SOP `2.25.23`, instance 3, position `0\0\0`, axial orientation, Frame Time `"0"`, and no rate or playback. That matches "One axial slice at z = 0 whose Frame Time is 0." For "does not fall through", `read_timing` (`series_open.rs:844-864`) calls `one_positive_ds` for Frame Time before it tries Recommended Display Frame Rate or Cine Rate. `one_positive_ds` (`:902-926`) returns `Err(Cine)` at `:922-923` when the parsed value is not positive. So a Frame Time of 0 is an error and does not fall through to a later timing source. The claim holds.
- **The `series.rs` comment.** At `series.rs:363` it reads "z = 0 sorts before the good part at z = 3, so the bad frame is the cine source." `read_cine` is called on `first_frame` alone (`series_open.rs:677-679`), and pass 3 established that the z = 0 part sorts first. The comment is true, and it closes pass 3's first nitpick.

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|---|---|---|---|
| Read the pass 3 review | `.claude/reviews/F-192-series-open-pass-3.md` | executed now | One defect, two nitpicks |
| Grep for both function definitions | `examples/example-server/src/bytes.rs` | executed now | `synthetic_series_parts` at `:128`, `frame_time_zero_part` at `:201` |
| Read `bytes.rs:100-227` | working tree | executed now | Each doc block sits directly above its own item |
| Read `read_cine`, `read_timing`, `one_positive_ds` and the `read_cine` call site | `crates/ocelli-dicom/src/series_open.rs` | executed now | Frame Time 0 errors at `:922-923` before any rate is read |
| Grep `series.rs` around the test | working tree | executed now | New comment at `:363` |

## Defects

None.

## Smells

None.

## Nitpicks

- `bytes.rs:199`: "Assembly refuses that value as cine" is true only when this part becomes the cine source. `read_cine` reads timing from `first_frame` alone, so the same object accepts if it is not first in display order. The one caller at `series.rs:363` documents that condition, so the doc is not false where it is used. Adding "when it is the first frame in display order" would make the helper's doc stand alone. This is the same sensitivity as pass 3's first nitpick, now on the helper instead of the test.

## Verified clean

- **Pass 3 D1 closed.** The three-object paragraph is back on `synthetic_series_parts`, and it is still true of that body. `frame_time_zero_part` has its own doc, which is accurate.
- **No new false claim from the move.** Moving the helper changed only its position and its doc. `#[cfg(test)]` is still on it. Its arguments are the same as the ones pass 3 described, z = 0 and Frame Time 0 with matching Series and Frame of Reference UIDs through `series_slice`, so pass 3's discrimination analysis of `series.rs:361-391` still holds.
- The new prose has no em dash and no prose semicolon.

## Limitations and handoff

- I ran no tests, clippy or mutations. I had no shell, so I did not read `git diff`. I checked the remediation by reading the named lines and the code around them.
- I did not re-check pass 3's second nitpick, the test name. It is optional and the move did not touch it.
- Next: the nitpick is optional. Under the loop rule this pass ends the review, because it has zero defects and zero smells.

F-192, pass 4: 0 defects, 0 smells, 1 nitpick.
