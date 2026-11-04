# F-192 review, pass 1

**Reviewed**: uncommitted working tree on sprint/s42, parent 343255d. Paths: `crates/ocelli-dicom/src/{series_open.rs,frame_geometry.rs,frame_open.rs,lib.rs}`, `crates/ocelli-dicom/tests/series_open.rs`, `examples/example-server/src/{series.rs,bytes.rs,http.rs}`, `examples/viewer-react/src/App.tsx`, `examples/viewer-react/vite.config.ts`, `docs/lld/series-open.md`
**Author / reviewer**: implementation by the sprint host. Review by Claude Code, read only, with no shell.
**Result**: 4 defects, 2 smells, 3 nitpicks

## Acceptance challenged

- **AC1, projection order, from the fixture bytes.** `series_open_axial_order_is_the_projection` encodes input 0 as SOP `2.25.21`, IS `1`, IPP `0\0\3`, and input 1 as SOP `2.25.20`, IS `2`, IPP `0\0\0`, both with IOP `1\0\0\0\1\0`. Row × column = (0,0,1), so the projection is z. My own expectation is input 1 (z = 0) then input 0 (z = 3), which is `[(1,0),(0,0)]`. Instance order and input order both give `[(0,0),(1,0)]`, and so does a descending projection. The test rejects all three wrong sorts. The implementer's mutation receipt agrees.
- **AC9, synthetic JSON, from `bytes.rs:128-158`.** Part 0 is IS 1 at z = 3 with RDFR 30. Part 1 is IS 2 at z = 0 with Frame Time 100. Part 2 is sagittal with `SWEEP`. Expected: axial slots `[[{input 1}],[{input 0}]]`, timing `frame_time` 100 taken from the first displayed frame (part 1, not part 0's rate 30), playback loop, a second group with sweep. `series.rs:318-337` asserts every one of those. The 30 fps on part 0 is a good trap: reading cine from input order would surface `frames_per_second`, and the test refuses that.
- **AC7, cine fallthrough.** Frame Time `0` with RDFR `10` must error. A fallthrough would return `RecommendedDisplayFrameRate` and turn the test red. This rejects the wrong behaviour.
- **Merged missing UID.** Two absent Frame of Reference UIDs: `series_open_missing_frame_of_reference_uids_do_not_join` rejects a merge. **Absent against present: no test discriminates it** (S2).
- **A cross-entry-point case I derived myself:** the server feeds `assemble_display` one `ParsedFrame` per declared frame (`series.rs:84-95`). For a multiframe object that has no `(0020,0052)`, the absent-UID identity is keyed per frame, so the object's own frames never join each other (D3). No test drives a multiframe object through that path without both UIDs.
- **Cine attributes against PS3.3 C.7.6.5 and PS3.6** rather than against the plan table. This found D1 and D2.

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|---|---|---|---|
| Read every file in the brief completely, or at the named functions | working tree, parent 343255d | executed now (read only) | Findings below |
| Grep `PREFERRED_PLAYBACK_SEQUENCING` in `dicom-dictionary-std-0.10.0/src/tags.rs` | crate registry source | executed now | `PreferredPlaybackSequencing (0018,1244) US 1`, entry `vr: Exact(US)` |
| Grep `1244` across the repository | working tree | executed now | Encoded as `CS` in `tests/series_open.rs:526` and `bytes.rs:183`, and read as CS at `series_open.rs:856` |
| series_open integration tests, 23 passed | implementer | prior receipt, not mine | not reproduced |
| example-server tests, 9 passed | implementer | prior receipt | not reproduced |
| frame_geometry tests, 22 passed (AC8) | implementer | prior receipt | not reproduced |
| clippy, lint, typecheck, unsafe gate, exit 0 | implementer | prior receipt | not reproduced |
| Mutation: projection `total_cmp` → instance order | implementer | prior receipt | test red, then restored green. My reading of the fixture agrees it must go red |

## Defects

### D1, Preferred Playback Sequencing is read as CS text, but DICOM defines it as US 0 or 1

**Where**: `crates/ocelli-dicom/src/series_open.rs:852-866` (the `VR::CS` check is at line 856). The same encoding is in `crates/ocelli-dicom/tests/series_open.rs:526` and `examples/example-server/src/bytes.rs:183`.
**What**: `read_playback` returns `SeriesOpenReason::Playback` unless `(0018,1244)` has VR CS and the value is the string `LOOP` or `SWEEP`. A conformant file stores US `0` (looping) or `1` (sweeping). Explicit VR files carry `US`, and implicit VR files resolve to `US` from the dictionary. Either way `element.vr() != VR::CS` is true, so assembly fails and the whole request returns `{"error":"playback"}`. A file that does say Sweep can never produce `Playback::Sweep`.
**Why it is wrong**: PS3.6 lists `(0018,1244)` as US with VM 1. PS3.3 C.7.6.5 (Cine Module) gives its enumerated values as 0 = looping and 1 = sweeping. The plan's tag table says "LOOP or SWEEP". That is a factual error in the plan, and the code, the test fixture and the synthetic fixture all copied it. The plan's actual decisions (enum `Loop`/`Sweep`, absent means Loop, page label Bounce) are unaffected. Only the encoding is wrong. Under HLD 27.2 R2 the fixtures derive from the plan text and not from the standard, so AC6 does not prove what it claims.
**Evidence**: `dicom-dictionary-std-0.10.0/src/tags.rs:3080` reads `/// PreferredPlaybackSequencing (0018,1244) US 1 DICOM`, and line 17562 reads `vr: Exact(US)`. `series_open.rs:856` is `if element.vr() != VR::CS { return Err(... Playback ...) }`.
**Scope and regression**: Present in the working tree. It affects every caller of `assemble_display` and the `/series-open` route for any real cine object carrying the attribute. Smallest regression: fixtures with `(0018,1244)` US `1` expecting `Playback::Sweep`, US `0` expecting `Loop`, and US `2` expecting `SeriesOpenReason::Playback`. Re-encode the synthetic sagittal part as US `1` so AC9 still shows sweep.

### D2, a conformant Frame Time Vector is refused because its first value is 0

**Where**: `crates/ocelli-dicom/src/series_open.rs:792-799`. The test is `crates/ocelli-dicom/tests/series_open.rs:313-321`.
**What**: Every vector element must pass `is_positive`, so `0\33.3\33.3` returns `SeriesOpenReason::Cine` and the request fails. `series_open_vector_zero_does_not_fall_through` asserts that exactly this conformant shape (`0\60`) is an error. The positive fixture `40\60` is the non-conformant shape.
**Why it is wrong**: PS3.3 C.7.6.5.1.2 defines Frame Time Vector as the time increments between each frame and the previous one, and says the first frame has an increment of 0. The first value is not an interval. The plan's rule that "a non-finite, zero, or negative interval is an error" therefore does not require refusing it, and the `CineTiming::FrameTimeVector` doc comment "one positive interval per frame" (`series_open.rs:126`) misstates the attribute. As written, the one source that wins first fails on the files that carry it, which are typically ultrasound and XA cine.
**Evidence**: Reading the code: the loop at lines 792-799 applies `is_positive` to element 0. The fixture at test line 316 is `"0\\60"`, and the test expects `Cine`.
**Scope and regression**: Present in the working tree. It affects `assemble_display` and `/series-open`. Smallest regression: `0\60` with matching length expects `FrameTimeVector`, `40\0` expects `Cine`, and `0\-5` expects `Cine`. The current zero test should move its zero off index 0. I am not quoting the C.7.6.5.1.2 sentence verbatim in this pass. Confirm the wording against the standard before remediating.

### D3, frames of one multiframe object with no Frame of Reference UID split into one group per frame

**Where**: `crates/ocelli-dicom/src/series_open.rs:305-316` and `869-881` (`UidIdentity::Absent(index)` keyed by input index), together with `examples/example-server/src/series.rs:84-95`, which gives each frame its own input index.
**What**: `uid_identity` returns `Absent(input_index)`, and `same_uid` treats two `Absent` values as equal only when the indices match. The server expands an N-frame object into N `ParsedFrame`s with indices 0 to N-1. If `(0020,0052)` is absent (or `(0020,000E)`, which is rarer), no two frames of that object share an identity:
- positioned frames each start a new group in `place_positioned`
- unpositioned frames each start a new companion in `place_unpositioned`

A 60-frame XA or US cine with no Frame of Reference module comes back as 60 groups of one frame each. That cascades into cine: `frame_time_vector` compares the vector length (60) with the group length (1) and skips it.
**Why it is wrong**: The contract says two *missing UIDs* do not join each other. Frames of one object read one attribute from one data set, so they share one missing UID, not two. The plan's invariant is that one object is not split by a rule meant to separate distinct objects. Cine is the case this story exists for, and the Frame of Reference module is user-optional in the XA and US IODs, so this is an expected input and not a corner case.
**Evidence**: Reading the code. `prepare` passes `input_index` to `uid_identity` (lines 389-390). `frames_of` pushes `ParsedFrame::new(object, frame)` per frame, so `assemble_display` enumerates those frames as distinct inputs. The only multiframe test, `series_open_per_frame_position_orders_inside_one_object`, sets both UIDs (test lines 499-500), so the path is unexercised.
**Scope and regression**: Present in the working tree. It affects the library and `/series-open`. Smallest regression: the enhanced pair fixture without `(0020,0052)`, assembled as frames 0 and 1, expects one group with `frame_count() == 2`. Keep the existing two-object test green. The same split for an unpositioned two-frame object with no Frame of Reference UID should also be covered.

### D4, an unpositioned frame skips the orientation and spacing checks the plan and LLD say still apply

**Where**: `crates/ocelli-dicom/src/series_open.rs:396-400`, with `crates/ocelli-dicom/src/frame_geometry.rs:253-266`. The prose is `docs/lld/series-open.md:23`.
**What**: `FrameGeometry::derive` returns `MissingImagePosition` before it resolves orientation or spacing. A frame with no IPP and a malformed IOP (`abc`, or a non-unit cosine) or malformed Pixel Spacing is therefore classed as unpositioned and accepted.
**Why it is wrong**: The plan's Approach says "A malformed cosine is still an error for that frame." The LLD says "Any other geometry failure is an error for that input." For this input, both claims are false. Display output does not change, because an unpositioned frame's orientation is never used, but by the severity table this is a false prose claim and an unmet plan requirement.
**Evidence**: The order of evaluation in `derive` (position resolved at 253-259, orientation at 260-266). `prepare` maps only `MissingImagePosition` to `None` and has no further validation.
**Scope and regression**: Present in the working tree, in the library only. Smallest regression: no IPP plus IOP `1\0\0\0\0\0` (a zero column) expects `SeriesOpenReason::Geometry`. The fix is either to validate orientation and spacing for unpositioned frames or to delete the false LLD sentence. The plan sentence is the plan owner's decision. Note that this pass did not confirm whether a malformed IOP fails earlier, during the metadata projection. I believe it does not, because DS text stays text.

## Smells

### S1, the JSON `index` and `input` fields mean two different things

**Where**: `examples/example-server/src/series.rs:76,80,89`, `crates/ocelli-dicom/src/series_open.rs:147-148,164-168`, and `examples/viewer-react/src/App.tsx:218,271-276`.
**What**: A `declared_frame_count` or parse error reports the part index (`with_index(part_index)`). An `assemble_display` error, and every `FramePlace::input_index`, reports the position in the expanded frame list. After any multiframe part, the two diverge. If part 0 has 10 frames, a geometry failure in part 1 shows "geometry at 10", and `placeLabel` renders object 0 frame 3 as `3.3`. A reader takes that as file 3, frame 3.
**Mechanism**: `SeriesOpenError::index` is documented as "the caller's input index". The page presents both meanings with identical text. The first multiframe upload makes the page point at the wrong file, and the bug grows once D3 is fixed and multiframe groups become ordinary.
**Smallest regression**: A posted body made of a two-frame object then a single-frame object with a geometry error. The error should name part 1, or the field should be renamed to say it is a frame-list index.

### S2, no test covers "a missing UID does not join a present one"

**Where**: `crates/ocelli-dicom/src/series_open.rs:310-316`. The only absent-UID test is `crates/ocelli-dicom/tests/series_open.rs:366-377`.
**What**: Change the `_ => false` arm of `same_uid` to `true` for `(Present, Absent)`, and every test in the file still passes. `series_open_missing_frame_of_reference_uids_do_not_join` has both sides absent, and every other fixture has both present. The rule is in the plan scope, the `assemble_display` doc comment, the LLD and this review's contract, and the suite counts it as covered.
**Smallest regression**: One axial frame with Frame of Reference `2.25.11` and one without, same orientation and series, expecting two groups. Repeat with Series Instance UID absent on one side.

## Nitpicks

- `series_open.rs:789`: a wrong-length Frame Time Vector is skipped before its values are checked, so `0\-5\x` with the wrong length falls through silently. The plan sanctions the length skip, but the LLD's "a non-finite, zero, or negative present source is an error" reads as unconditional. Checking values before length would make the two rules agree.
- No test covers Cine Rate `0` or RDFR `0` on their own. `one_positive_is` handles both, by inspection.
- `series_open.rs:269-271`: "and [`SeriesOpenReason::FrameOutOfRange`] is not used here" sits under `# Errors` as a negative claim and reads as a dangling clause.

## Verified clean

- **Tolerances.** `cosines_agree` uses `DIRECTION_COSINE_TOLERANCE` with `<=` per component. `spacing_agrees` uses `SPACING_EQUALITY_MM` (1e-9). `positions_coincide` (`<=`) and `exceeds_geometry_tolerance` (`>`) use `GEOMETRY_TOLERANCE_MM` (1e-6), which is still private and unchanged at line 56. No new exported number. The helper comparison directions match `StackGeometry::derive`'s inline `> GEOMETRY_TOLERANCE_MM` tests. `derive` now calls the same `cosines_agree` and `spacing_agrees`, and its verdict order is unchanged. Plan contradiction, not a finding: the plan allowed only a spacing helper in `frame_geometry.rs`, and the tree adds `positions_coincide` and `exceeds_geometry_tolerance` as well. Neither changes a number or a verdict.
- **Projection.** `position.dot(normal)` uses the first positioned frame's `slice_normal()`, and later group members use the anchor normal (`series_open.rs:483-485`). This matches PS3.3 C.7.6.2.1.1 and the plan.
- **Slot order.** Stack ID, In-Stack Position Number, Temporal Position Index, instance number, SOP bytes, frame index, input index (`cmp_within_slot`). Present sorts before absent at every key. A slot boundary is decided against the slot's first frame by projection, before the within-slot sort, so the reorder cannot move a boundary. Both frames are kept. The temporal, stack-ID and in-stack fixtures each set instance numbers opposite to the expected order, so each discriminates.
- **Unpositioned order.** Instance number, SOP bytes, frame index, input index, placed after the positioned slots. A frame joins a positioned group only when exactly one group shares its UIDs, and otherwise goes to a companion (test lines 405-443).
- **The 90 degree in-plane rotation is a separate group.** Row and column are compared separately, and the `ROTATED` fixture shares the normal.
- **Cine source order and no fallthrough** for Frame Time, RDFR and Cine Rate. VRs are DS, DS, IS, IS, which matches PS3.6 for those four tags. No 10 fps default exists anywhere. `Unavailable` when every source is absent. Cine comes from the first frame in display order, and `series_open_later_frame_does_not_supply_cine` discriminates.
- **No LUT call.** `series_open.rs` has no `LutChain`, `map_into` or pixel arithmetic, and `project_metadata` skips Pixel Data, Float Pixel Data and Double Float Pixel Data.
- **No identifying data.** `ParsedFrame`'s Debug prints only the frame index. `SeriesOpenError` carries a reason and an index. `DisplaySeries` holds no UID. The JSON writer emits no UID, path or filename. Multipart headers are discarded. The fixtures contain no patient name.
- **React parser against the JSON writer, field by field:** `order`, `frames`, `slots[][].{input,frame}`, `timing`, optional `milliseconds`, optional `frames_per_second`, `playback`, `sheared`, `non_uniform_step`. Names and types match. Timing tokens and `loop`/`sweep` match the label functions and the `<option>` values, and Sweep is labelled Bounce. The error body `{error,index}` matches `seriesProblem`. `f64::to_string` cannot emit `inf` or `NaN`, because `parse_decimal_string` refuses non-finite values and `is_positive` refuses zero and negatives.
- **Routing.** `/series-open` is accepted for POST and OPTIONS, shares the 1 GiB `BODY_CAP`, and is proxied in `vite.config.ts`. One server, no second one.
- **AC9.** Three objects, two groups, axial order `input 1 then input 0`. The page shows group count, order, timing source, interval and a Loop/Bounce control seeded from the file value. The two-versus-three object contradiction is recorded in the LLD and in the `bytes.rs` doc, and the page meets AC9.
- **LLD against the code.** Grouping, order keys, the report semantics and the example-viewer paragraph match, except for line 23 (D4) and the cine rows that inherit D1 and D2.

## Limitations and handoff

- **I could not execute tests, clippy, lint or mutations in this pass.** Every pass count and mutation in the Evidence table is the implementer's receipt, not my own execution. D1 to D4 and S2 come from reading source and the dictionary crate. None is reproduced by a red test.
- D2 depends on the PS3.3 C.7.6.5.1.2 wording about the first frame's zero increment, which I did not quote verbatim here. Confirm it before changing the test.
- Not exercised: a real corpus cine object, implicit VR input, the browser page itself, or a multipart upload with more than one part.
- Next: the implementer fixes D1 to D4, S1 and S2, adding each smallest regression above and confirming it goes red before the fix. Then a fresh independent pass 2 reviews the changed material. This pass is a bounded review, not completed acceptance or sprint readiness.

F-192, pass 1: 4 defects, 2 smells, 3 nitpicks.
