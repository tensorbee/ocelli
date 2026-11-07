# F-209 review, pass 1

**Reviewed**: `d8d01d5d..3c206f67` on `work/f-209-xai` (four commits, HEAD `3c206f67124259b5991880050bc37f9a01eda25d`). Working tree clean apart from the untracked `.venv` and `tools/oracle/out`, which are not part of the story.
**Author / reviewer**: a Claude implementer wrote the four commits. This pass is a separate Claude reviewer instance that wrote none of the reviewed material. The independence comes from the session, and no generated trailer establishes it.
**Result**: 3 defects, 3 smells, 3 nitpicks

## Acceptance challenged

- AC1 to AC7 and AC23. I derived every value again from PS3.3 C.7.6.2.1.1, Equation C.7.6.2.1-1, with Pixel Spacing row first. They match the tests. I also wrote an independent Python reference (exact fractions, PS3.3 matrix, cornerstone3D 5.8.2 `ReferenceLinesTool` edge choice and `linePlaneIntersection`) and checked the real `POST /link` against it on four cases the suite does not have:
  - both planes oblique: target IOP `0.6\0.8\0\0\0\-1`, source IOP `1\0\0\0\0.8\0.6`
  - a source whose top-bottom edge is perpendicular to an oblique target normal, which takes the second edge pair
  - an oblique coplanar stack, dot 0.96, with in-plane IPP offsets
  - a tilted stack that both scrolls and draws

  All four agree to the last printed digit, apart from a final-ulp difference on one coordinate (`-19.266666666666666` against `-19.26666666666667`). The suite would accept these wrong implementations: swapped spacings, `<=` in the reduce, distance alone, a 0.75 threshold, two missing UIDs read as one frame, and a line read through the unit extension.
- AC8, AC9, AC14 and AC21. The token text, the Imager Pixel Spacing refusal, the round trip and the single `PAGE_PIXEL_CENTRE` all hold, and mutation turns them red.
- AC10 to AC13. Route answers, refusals and the declared-length cap. I recomputed the cap independently:
  - token 11 x 24 + 2 x 10 + 12 = 296
  - one listed image 1 + 64 + 1 + 296 = 362
  - one target line 6 + 1 + 1 + 1 + 5 + 1 + 1 + 65536 x 362 + 1 = 23,724,049
  - eight target lines and the source line give 189,792,761 bytes

  The body I generated myself had exactly that length and was admitted.
- AC15 to AC19 (B1 and B3 equivalents) on a synthetic in-memory folder. pydicom wrote five series in one frame of reference: axial CT, coronal CT, axial CT in reversed instance order, axial MR and one RGB image. They were injected into `#folder` through a `DataTransfer` and driven in Playwright Chromium against a release build on an ephemeral port. Scroll is by position, not by index: the reversed series went to 8, 7, 6 while the source went to 1, 2, 3. Window and zoom sync, the colour refusal, MR preset refusal, MR numeric W/L acceptance, the row click copying nothing, stale replies being dropped and the fetch failure all behaved as specified, apart from D1.
- AC17, AC20 and AC22 were not exercised (see Limitations).

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|--------------------------|---------------------------|------------------------------|----------------------|
| `cargo test -p ocelli-viewport --test link --test sync` | HEAD | now | 0, 10 and 7 passed |
| `cargo test -p ocelli-folder-viewer` | HEAD | now | 0, lib 73, preview 72, measure 14, value 24, others green |
| `cargo clippy -p ocelli-viewport -p ocelli-folder-viewer --all-targets -- -D warnings` | HEAD | now | 0 |
| `cargo build -p ocelli-viewport --no-default-features --target wasm32-unknown-unknown` | HEAD | now | 0 |
| `bin/ocelli.sh gate viewer-browser` | HEAD, Chromium 151.0.7922.34 | now | 0, ALL GREEN, F-208 aspect and readout checks unchanged |
| `python3 scripts/prose_check.py`, `scripts/deviation_check.py` | HEAD | now | both OK |
| Mutation catalogue, 18 mutants, scratch copy under `target/` with its own target dir, removed with `shutil` | HEAD copy | now | 12 red, 6 survived, listed below |
| Independent oblique reference against `POST /link` | release build, ephemeral port | now | 4 of 4 agree |
| Largest body, 189,792,761 bytes, under `/usr/bin/time -l` | release build | now | 200 in 0.34 s, peak RSS 352,534,528 bytes |
| Playwright B1/B3 equivalents, synthetic in-memory folder | release build, ephemeral port | now | see Acceptance and D1 |

Mutants turned red: `COPLANAR_DOT` 0.75, `<` to `<=`, first edge pair always, `(None, None)` true, zero-spacing refusal removed, the `sync.rs` source rule removed, `PAGE_PIXEL_CENTRE` 0 (measure and preview both red, so AC21 holds), `{:e}` to `{}`, the `/link` cap removed, and the one-frame check reduced to the first image.

Mutants that survived: the parallel test at `>= 1.0`, the parallel test at `> 0.5`, the perpendicular test at `== 0.0`, the perpendicular test at `< 0.5`, `drawn = shown`, the per-target `FRAME_LIST_CAP` guard removed, the `inspect_json` planes cap removed, and the reason precedence swapped.

## Defects

### D1, a stale reference line survives when the outlined viewport has nothing to send

**Where**: `examples/folder-viewer/src/page.html`, `linkRequest` (returns `null` when `currentImage(source)` is null) and `sendLink` (`if (!request) continue;`), reached from `setActive`.
**What**: the trigger is layout 3 by 3 with five series, so cells 5 to 8 are empty. Cell 2 (coronal) is outlined, with Reference lines on and Scroll on, and every other cell shows a line drawn from the coronal plane. Clicking empty cell 5 makes it active. Cells 0, 1, 3 and 4 keep their coronal lines (`display ""`, `y1 5.5`), while the outline now marks a cell that has no image. Cell 2, now a target, has no line. The lines describe a source that is no longer the source.
**Why it is wrong**: the plan's amendment "The send hook compares the image key" and the `setActive` comment in the page ("every other cell's line is now drawn from it"). Row 297's invariant is that a line is always drawn from the current state. The README tells the reader the line is "where the source image's plane cuts it", with the outlined viewport as the source.
**Evidence**: Playwright, state read back after the click: `afterEmptyActive: active 5, cells [[0,"uid:2.25.901",3,"","5.5"], [1,"uid:2.25.903",6,"","5.5"], [2,...,"none"], [3,"uid:2.25.904",0,"","5.5"], ...]`.
**Scope and regression**: present on HEAD. Any layout with an empty cell is affected, and so is a source whose series vanished. The smallest fix clears every line when there is no source image. A page check could pin that `linkRequest`'s null-source path clears lines, or a browser step could click an empty cell and assert no `.ref-lines` is displayed.

### D2, `use.html` says `closest_coplanar` refuses a non-finite distance

**Where**: `examples/folder-viewer/src/use.html`, chapter `#geometry`: "Both are `LinkError::Degenerate` for a point or distance that is not finite".
**What**: `closest_coplanar` skips a non-finite distance and refuses only when no distance is finite or the stack is empty. The LLD and the doc comment say this correctly.
**Evidence**: `POST /link` with a source at z = 0 and a stack at z = 1e200 and z = 5 answers `"index":1`, not `degenerate`. The first distance overflows to infinity and is skipped. The empty stack, which is the common Degenerate case, is not mentioned in use.html.
**Scope and regression**: prose only. Replace the sentence with the LLD's wording, "when no distance is finite, which includes an empty stack".

### D3, `more.html` says `reference_segment` reads a frame of reference

**Where**: `examples/folder-viewer/src/more.html`, `#reference` tiers paragraph: "The shell functions do not project a plane and do not read a frame of reference. `reference_segment` does both".
**What**: `reference_segment(source, target)` takes two `ImagePlane`s and reads no UID. The frame rule is `same_frame_of_reference`, and in this tree the caller (`link.rs` `answer`) applies it.
**Evidence**: the signature in `crates/ocelli-viewport/src/link.rs`, and the handler's separate `one_frame` check.
**Scope and regression**: prose only. Name `same_frame_of_reference` for the frame and `reference_segment` for the projection.

## Smells

### S1, the `PLANE_EPSILON` thresholds are pinned as values and never as behaviour

**Where**: `crates/ocelli-viewport/tests/link.rs`. `parallel_planes_have_no_line` uses dot exactly 1. AC4 and AC5 use top-bottom dots of exactly -1 and 0.
**Mechanism**: four mutants all survive the whole suite, including the folder viewer: `isParallel` changed to `>= 1.0` or `> 0.5`, and `isPerpendicular` changed to `== 0.0` or `< 0.5`. `the_thresholds_are_the_oracle_constants` checks the constants and not their use. A change to either comparison would silently draw a line between nearly parallel planes, or refuse a legitimate oblique one.
**Smallest regression**: two hand-computed pairs either side of `1 - 1e-3` for the parallel test, for example cosines giving dot 0.9995 and 0.998, and a source whose top-bottom dot is 5e-4 and then 2e-3.

### S2, the line-on-the-moved-image invariant is untested

**Where**: `examples/folder-viewer/src/link.rs`, `let drawn = index.and_then(|at| stack.get(at)).unwrap_or(shown);`.
**Mechanism**: `drawn = shown` survives. Every route test that moves a target uses parallel planes, so it has no line. The code is right today. My tilted-stack case (target IOP `1\0\0\0\0.96\0.28` at z = 0, 5, 10, source axial z = 5) answers `index 1, line [0.5,0.5,19.5,0.5]`, which my reference matches. The line on the shown image would be `[0.5,18.357..,19.5,18.357..]`. Plan row 297 states this invariant.
**Smallest regression**: that case as a route test.

### S3, two count guards nothing executes

**Where**: `link.rs` `target_line`, `if images.len() >= FRAME_LIST_CAP`, and `preview.rs` `plane_tokens`, `count.min(FRAME_LIST_CAP)`.
**Mechanism**: removing either one leaves the suite green. A body of 65,537 `- -` images fits well under `LINK_BODY_CAP`. That makes the first guard the only refusal the `link_json` doc promises for "more than `FRAME_LIST_CAP`", and no test runs it.
**Smallest regression**: a `- -` body one image over the cap, refused with `link`, and an object declaring 65,537 frames whose `planes` length is 65,536, if `inspect_json` admits such an object.

## Nitpicks

- N1. The reason precedence swap survives (`link.rs` `answer`). The page ignores `reason` and the combination can only arise as NotCoplanar with Degenerate, so this does not block.
- N2. README step 4 has one line far past the file's wrap width: "is above 0.9, which is under about 26 degrees. Window copies ...". The claim itself checks out, since acos 0.9 is 25.84 degrees.
- N3. The LLD's "The folder viewer's `POST /link` is the one caller today" leaves out `SyncScene`, which now calls `same_frame_of_reference`. The sentence is accurate for the geometry functions.

## Verified clean

- Geometry. `closest_coplanar` and `reference_segment` implement the transcribed cornerstone3D 5.8.2 rules over `ImagePlane::index_to_world` and `slice_normal`. The corners are corner-pixel centres from `columns - 1` and `rows - 1`. The edge pair is chosen by the perpendicular test. Line-plane intersection is `t = -(n.p0 - d) / n.dir`, read back through `index_to_world().inverse()`. The coplanar test refuses a NaN dot. Zero spacing on a singleton target axis is refused before the unit extension can be read. The four independent oblique cases agree.
- Casts. There is no `as` in the diff. `f64::from(u32)` is exact, `usize::try_from(frames)` saturates to the cap, and no rounding is introduced. The page draws the numbers as received.
- `SyncScene` now calls `same_frame_of_reference`. Its seven tests are unchanged and green, and removing the source check turns them red.
- Pixel units. One `PAGE_PIXEL_CENTRE` in `value.rs`, used by `measure_json` through `to_index` and by the handler through `from_index`. Mutating it turns F-211's measure fixtures and AC10 red together.
- Token and UID. `{:e}` round-trips bit for bit and the longest field is 24 characters. A UID that is not all digits and dots, or is longer than 64 characters, is reported `""`. `optional_text` strips the UI trailing NUL, so a padded odd-length UID is still linked.
- `/link` cap and exhaustion. The cap is refused from the declared length before any body is read. The largest admissible body is admitted and one byte more is refused. Both are tested, and I reproduced both outside the test. The cap sits below the existing 1 GiB `BODY_CAP`, which `/file` already admits, and the body goes through the same body permits. One maximum body peaks at 352 MB RSS, about 160 MB of parse on top of the body, and is answered in 0.34 s on release. That stays inside the envelope `/file` already allows, so it opens no new exhaustion route. The parse allocation is not counted in the inflight counter, which is accounting only. The count fields are sized for `u32` though DICOM Rows and Columns are US, so the cap is generous, not tight. It is still safe.
- Page. The token is treated as opaque, `linkList` is cached per series object, and series objects are rebuilt on every folder read. Only the active cell sends. A reply is applied only while its ticket is the latest and the target's `loadToken` and `viewIndex` match. In Playwright, a series loaded into a target during a delayed reply was not moved by that reply. A fetch failure cleared every line and moved no cell. The colour target kept its window and read `Window not applicable` with zoom skipped, as the amendment says for a hidden stage. With a colour source the presets were disabled with the new note. Lung reached CT and not MR, while a numeric W/L release reached MR. The window was not copied during the drag. Reset copied the file window. A row click copied nothing. Zoom copied 122% to the CT target with pan kept.
- F-208 and F-211 regressions. `gate viewer-browser` is green and its readout checks are unchanged. The measure tests are green.
- The implementer's listed deviations. Re-exporting `FrameOfReferenceUid` avoids a new dependency and is fine. The C.7.6.2.1.1 normal-as-implied wording and the C.7.4.1.1.1 series scoping are accurate readings, though I quoted the section from memory and not from a fetched current edition. A toolbar button for Reference lines matches the README amendment. `queueMicrotask` coalescing matches "every schedule in one task is one request". Writing the handler tests after the handler is acceptable because their values are hand-derived, and I re-derived them.

## Limitations and handoff

- Not exercised: AC17 and B2 (operator host and pseudo-PHI study only), AC20 (a Length mark plus a window repaint keeping the line) and AC22 (the target readout hiding on a synced `=`).
- No GPU or oracle job was run, and none is needed by this story.
- PS3.3 10.7.1.3, C.7.4.1.1.1 and PS3.5 Table 6.2-1 were checked from memory, not against a fetched current edition.
- Next owner: the implementer remediates D1 to D3 and S1 to S3. A fresh independent pass then reviews the changed material.
