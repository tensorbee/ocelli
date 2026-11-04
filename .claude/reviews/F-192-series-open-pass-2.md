# F-192 review, pass 2

**Reviewed**: uncommitted working tree on sprint/s42, parent 343255d, after the pass 1 remediation
**Author / reviewer**: implementation by the sprint host. Review by Claude Code, read only, with no shell.
**Result**: 3 defects, 0 smells, 3 nitpicks

## Acceptance challenged

- **D1 remediation, from PS3.6 and the reader.** `read_playback` (`series_open.rs:951-968`) returns `Loop` when the attribute is absent. It then requires `VR::US`, maps `Empty` and `[]` to `Loop`, `[0]` to `Loop`, `[1]` to `Sweep`, and anything else to `Playback`. Before the fix, a conformant US 1 file failed the request. The new tests reject that behaviour: `series_open_sweep_is_playback` (US 1), `series_open_playback_zero_is_loop` (US 0), `series_open_playback_two_is_an_error` (US 2) and `series_open_playback_code_string_is_an_error` (CS `SWEEP`, which the old reader accepted). The synthetic sagittal part is now US 1 (`bytes.rs:155,182-185`), so AC9's `"playback":"sweep"` assertion at `series.rs:338` goes through the US path.
- **D2 remediation, Frame Time Vector.** At offset 0 the code accepts `Equal | Greater` (`series_open.rs:887-891`). At every later offset it requires `is_positive`. NaN gives `partial_cmp` `None` and is refused. `inf` cannot reach this point because `parse_decimal_string` refuses non-finite values (`frame_geometry.rs:733`). The length check still runs before any value is parsed (`:880-882`), which the plan requires. `0\60` with Frame Time 100 must produce `FrameTimeVector`, so a reader that refuses the leading zero, or one that falls through, fails `series_open_vector_leading_zero_is_the_vector`. `40\0` must produce `Cine`, which catches a reader that dropped the later-zero rule.
- **D3 remediation, my own cross-entry-point case.** `uid_identity` now keys an absence by `object_key(parsed)`, the address of the borrowed `ParsedDicom` (`series_open.rs:970-987`). Every `ParsedFrame` borrows its object for the whole `assemble_display` call, and `ParsedDicom` holds data, so it is not zero-sized. Two distinct live objects therefore cannot share an address, and the frames of one object always do. `series_open_one_object_without_frame_of_reference_stays_one_group` (positioned) and `series_open_unpositioned_object_without_frame_of_reference_stays_one_group` would each give two groups under the old input-index key. `series_open_missing_frame_of_reference_uids_do_not_join` builds two separate objects in a `Vec` and still expects two groups. Hand check of the positioned case: frame 0 is z = 3 and frame 1 is z = 0, so the expected order is `[(1,1),(0,0)]`, which is what the test asserts.
- **D4 remediation.** After `MissingImagePosition`, `reject_malformed_unpositioned` resolves orientation and spacing in the same functional-group order and with the same DS, multiplicity and grammar rules as `decimal_values`. It then calls `ImageOrientationPatient::new` and `PixelSpacing::new`. An absent orientation returns `None` and stays unpositioned, as AC4 requires. The orientation half has a discriminating test (`1\0\0\0\0\0`, a zero column, which `image_plane.rs:42-48` refuses). The spacing half has none (D2 below).
- **S2 remediation.** In both `does_not_join_a_present_one` tests the absent frame comes first and opens the group, so the second frame meets `same_uid(Absent, Present)`. That is the mixed arm. If it returned true the two frames would merge and `two_groups` would fail with left 1 right 2. My reading agrees with the implementer's mutation receipt.
- **S1 remediation.** `series.rs:77` writes `"part"` for `declared_frame_count` errors and `:82` writes `"frame"` for `assemble_display` errors. `seriesProblem` reads either. The page text and the LLD say which is which. No test reaches the `"frame"` branch (D3 below).

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|---|---|---|---|
| Read `series_open.rs` in full | working tree, parent 343255d | executed now (read only) | Findings below |
| Read `frame_geometry.rs` in full, plus `ImageOrientationPatient::new`, `PixelSpacing::new` and `ImagePlane::new` in `ocelli-pixel/src/image_plane.rs` | working tree | executed now | `PixelSpacing::new` refuses only non-finite or negative values. `ImagePlane::new` also refuses zero spacing when the matching dimension is not 1 |
| Read `tests/series_open.rs` in full | working tree | executed now | 32 `#[test]` functions by count. None encodes a malformed Pixel Spacing on an unpositioned frame |
| Read `series.rs` in full, `bytes.rs:122-195`, `App.tsx:205-344`, `docs/lld/series-open.md` | working tree | executed now | Neither example-server test reaches `series.rs:82` |
| Grep `.claude/commands/microscope.md` for the severity table and sections 2 to 4 | working tree | executed now | A false prose claim is a defect. A guard nothing executes is a defect class |
| series_open tests, 32 passed. example-server tests, 9 passed. clippy, lint, typecheck exit 0 | implementer | prior receipt, not mine | not reproduced |
| Seven named tests red before the fix | implementer | prior receipt | not reproduced. My reading agrees each one discriminates |
| `same_uid` mixed arm set to true, both `does_not_join_a_present_one` tests red, arm restored | implementer | prior receipt | not reproduced. My reading agrees |

## Defects

### D1, two public doc comments still describe the CS encoding that D1 removed

**Where**: `crates/ocelli-dicom/src/series_open.rs:209` and `:233`
**What**: The doc on `SeriesOpenReason::Playback` reads "Preferred Playback Sequencing is present and not LOOP or SWEEP." The doc on `DisplaySeries::playback` reads "LOOP or SWEEP from the first frame in display order." The reader no longer accepts either string. A CS `LOOP` now returns `Playback`, which is the opposite of what line 209 says, and the actual contract is US 0 or US 1.
**Why it is wrong**: Under the severity table, a false claim in prose is a defect. The `Playback` enum doc at `:114-118` and the LLD at line 46 were both corrected, so the remediation is internally inconsistent. A caller reading the error variant's rustdoc would build CS fixtures, which is exactly the mistake pass 1 found in the tests and the synthetic body.
**Evidence**: Line 209 as quoted above. Lines 955-966 refuse any VR other than US and accept only `[0]`, `[1]` and empty. `series_open_playback_code_string_is_an_error` asserts that CS is refused.
**Scope and regression**: rustdoc only, on public API. No behaviour changes. Rewrite both sentences to say US 0 or 1 (for example "present and not US 0 or 1" and "Loop or Sweep from the first frame in display order"), or delete them. No test is needed beyond re-reading the doc.

### D2, the unpositioned Pixel Spacing check has no test that reaches its error arm

**Where**: `crates/ocelli-dicom/src/series_open.rs:475-490`. The claims are at `series_open.rs:295-296` ("A malformed cosine or spacing on that frame is still an error") and `docs/lld/series-open.md:23` ("A present orientation or pixel spacing that cannot be read is still an error").
**What**: Every unpositioned fixture encodes Pixel Spacing `1\1` (`tests/series_open.rs:46`, `:660`, `:588`). No test gives an unpositioned frame a spacing that `plane_decimals` or `PixelSpacing::new` refuses. Deleting the whole second `if let` block in `reject_malformed_unpositioned`, or swapping its `?` for `.ok()`, would leave all 32 tests green.
**Why it is wrong**: Microscope section 4: a guard that exists, looks authoritative and is never reached survives a green suite. The rustdoc and the LLD both state this check as behaviour, and the remediation brief lists it as half of the D4 fix. Only the orientation half is proven. `series_open_unpositioned_malformed_orientation_is_an_error` errors out at line 473, before the spacing block runs.
**Evidence**: My read of all 32 tests. `PixelSpacing::new` (`image_plane.rs:82-87`) refuses only non-finite or negative values, and `plane_decimals` refuses non-DS VRs, the wrong multiplicity and unparseable text. No fixture triggers any of those on Pixel Spacing.
**Scope and regression**: Library, test coverage. Smallest regression: one frame with no IPP, orientation `AXIAL` and `spacing = "-1\\1"`, expecting `SeriesOpenReason::Geometry`. Confirm it goes red with the spacing block removed. A second case, `spacing = "abc\\1"`, would also cover the `plane_decimals` grammar arm.

### D3, the S1 assembly error field `"frame"` is written by a line no test executes

**Where**: `examples/example-server/src/series.rs:82`, which is read by `examples/viewer-react/src/App.tsx:220-222`
**What**: S1 was remediated by splitting the error field into `"part"` for parse and part-count errors and `"frame"` for assembly errors. The two example-server tests cover a successful synthetic body and a multipart parse failure (`"part":0`). Neither causes an `assemble_display` error, so `error_json(..., "frame", ...)` never runs. Changing line 82 back to `"part"` brings back pass 1's exact misdirection, where a frame-list index is shown as a part number, and the suite stays green.
**Why it is wrong**: Microscope section 4, and pass 1's S1 named this regression: "A posted body made of a two-frame object then a single-frame object with a geometry error." It was not added. The LLD (line 50) and the page text (`App.tsx:335-336`) both promise the distinction, and nothing checks it.
**Evidence**: `series.rs:321-359` contains the only two tests, and I read both. The multipart test's body is `not-dicom`, which fails at `parse_part10` on line 67, before `assemble_parsed` runs.
**Scope and regression**: Example server and page. Smallest regression: a multipart body with one valid single-frame part and a second part that parses but fails assembly. For example, take the synthetic slice with Frame Time `0`, which yields `cine`, or an unpositioned slice with IOP `1\0\0\0\0\0`. Assert `"error":"cine"` (or `geometry`), assert `"frame":1`, and assert that `"part"` is absent. A two-frame first part would also show the index running across frames rather than parts, but the field-name assertion alone catches the reversion.

## Smells

None.

## Nitpicks

- `series_open.rs:488`: `PixelSpacing::new` accepts `0\0`, so an unpositioned frame with zero spacing on Rows 512 is accepted, while the positioned path refuses it in `ImagePlane::new` (`image_plane.rs:157-161`). No display output depends on it. The LLD's "cannot be read" wording does not strictly cover a readable zero, so this is not a false claim. It would read more cleanly if the check matched the positioned path or the prose named the narrower rule.
- No test covers a negative first Frame Time Vector component (`-5\60`). By inspection it is refused at `:887-896`. Pass 1's suggested `0\-5` is also missing. It shares `is_positive` with the `40\0` case.
- No test covers an empty US `(0018,1244)`, which `:959,961` map to `Loop`. The positive vector fixture `40\60` (`tests/series_open.rs:314`) is still the non-conformant shape, now that `0\60` exists beside it.

## Verified clean

- **D1 closed in behaviour.** The reader is US only, accepts 0, 1 and empty, and refuses every other value and VR. The four tests discriminate. The synthetic sagittal part is US 1 and its comment cites PS3.6. The `Playback` enum doc and the LLD are correct. The LLD records the plan's LOOP and SWEEP contradiction, and the plan sentence is left to the plan owner. The only residue is D1 above.
- **D2 closed.** A leading 0 is accepted. A later 0, a negative or a non-finite component is `Cine` with no fallthrough. The length skip runs before values are parsed. The module doc (`:9-13`), the `FrameTimeVector` doc (`:136-137`) and the LLD line 44 agree with the code.
- **D3 closed.** The absence key is the object address. `same_uid` treats Absent as equal only to the same address, and Present only to the same text. Both multiframe regressions and the two-object regression are present and discriminate. The address is never serialised. `UidIdentity` is private and `DisplaySeries` holds none of it.
- **D4 closed for orientation.** It uses the functional-group resolution order and DS rules of `decimal_values` and the one `ImageOrientationPatient::new` validator, so there is no second copy of the cosine rule. Absent orientation stays unpositioned. Spacing coverage is D2 above.
- **S1 closed in behaviour.** `"part"` is used at `:29,35,39,47,50,54,57,63,67,77` and `"frame"` at `:82`. `seriesProblem` and `placeLabel` match the writer. The page sentence matches the LLD. Coverage is D3 above.
- **S2 closed.** Both mixed-arm tests exist and discriminate, and the implementer's mutation receipt is consistent with my reading.
- **Pass 1 nitpick on `declared_frame_count`'s `# Errors`** is resolved, because the dangling `FrameOutOfRange` clause is gone (`:278-281`).
- **Re-read of the whole diff for regressions from the fix.** The tolerance helpers in `frame_geometry.rs` are unchanged and `GEOMETRY_TOLERANCE_MM` is still private. Projection, slot order, unpositioned placement, the cine source order, the absence of a default frame rate, no LUT call, no identifying data in errors or JSON, and `lib.rs` exports are all as pass 1 recorded. The test fixtures emit elements in ascending tag order, including the new `(0018,1244)` US and CS rows.

## Limitations and handoff

- **I could not run tests, clippy, lint, typecheck or mutations.** Every pass count and mutation result in the Evidence table is the implementer's receipt. D2 and D3 come from reading every test in both suites, not from a red run.
- I had no shell, so the whole-diff re-read was a read of the named files and not of `git diff` output. `frame_open.rs` was checked only at `project_metadata` and its call site.
- Not exercised: a real corpus cine object, implicit VR input for `(0018,1244)`, the browser page itself, and a multipart upload with more than one part.
- Next: the implementer fixes D1 to D3, adds each smallest regression, and confirms it goes red before the fix where a fix changes behaviour. D2 and D3 are coverage, so the check is red with the guarded line removed. Then a fresh pass 3.

F-192, pass 2: 3 defects, 0 smells, 3 nitpicks.
