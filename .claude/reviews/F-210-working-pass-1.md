# F-210 review, pass 1

**Reviewed**: `git diff 198e197 e5dcf15` on `work/f-210-xai`, tree `e4c103ccd4a4fb1dca296b7d55d4847454c36dc0`. Nothing unstaged before or after the pass. The only file this pass adds is this report.
**Author / reviewer**: The implementation is by a Claude implementer agent. This independent review is by a separate Claude reviewer agent that wrote none of the F-210 code or plan. One limit on independence: both agents are the same model family.
**Result**: 3 defects, 4 smells, 4 nitpicks

## Acceptance challenged

- **AC20, standard transcription.** Checked against DICOM PS3.3 **2026d** and PS3.5 chapter 8, current edition at dicom.nema.org, fetched during this pass. The results per section are under Verified clean and in D1 and D2. The plan was not corrected before the code, which AC20 requires.
- **AC1, AC2, AC4, AC7, AC8, AC10, AC11, AC13, AC14, AC18, AC19.** Each one was mutated in a scratch copy and went red, as listed below.
- **AC12 on the corpus.** This is a discriminating case derived independently of the tests. Every manifest row was run through `decode_held`, and stage 1 in `HeldFrame::value` was compared with stage 1 in `recipe`. 86 rows had both, with 0 mismatches. 9 rows were refused at decode (8 `not-monochrome`, 1 `attribute`). No corpus row has the preview accept a frame that the value refuses.
- **Cross-reader refusal coupling.** Synthetic objects that the preview draws but whose inputs are irrelevant to stage 1 are covered in D3. This rejects the claim that the value is a function of the file's modality inputs and the cell only.
- **Per-frame Rescale Type.** An Enhanced CT object with `HU_MOD` in the Pixel Value Transformation Sequence reports `HU_MOD` and `rescale-type`, so the code is right. No test pins it (S2).

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|---|---|---|---|
| `cargo fmt --all --check` | worktree, tree e4c103cc | now | 0 |
| `cargo clippy --workspace --all-targets -- -D warnings` | same | now | 0 |
| `cargo test -p ocelli-folder-viewer -p ocelli-pixel` | same, no `corpus/data` in worktree | now | 0, 17 result lines all ok, `tests/value.rs` 15 of 15 |
| corpus tests `borrowed_bytes_match_the_parsed_pixels_for_every_accepted_corpus_file`, `recycled_grey_matches_a_fresh_decode_for_every_accepted_corpus_file` | scratch `git archive HEAD` under `target/review210`, `corpus/data` symlinked to the main checkout | now | 0, both ok |
| reviewer corpus probe, stage 1 agreement plus `value_json` at the centre and (0, 0) | same scratch, every manifest row | now | 86 agree, 0 mismatch. Units seen: 67 `HU rescale-type` (27 with padding -2000), 18 `unspecified none`, 1 `unspecified rescale-type`. No `ct-default` row in the corpus |
| mutations M1 to M19, see below | same scratch | now | 16 red, 3 survived (M7 is an equivalent mutant, M8 and M9 are S2 and S3) |
| refusal-coupling probe | same scratch, synthetic one-pixel CT | now | D3 |
| `npm run lint`, `npm run typecheck` | worktree | now | 0, 0 |
| `bin/ocelli.sh gate guards`, `gate bench` | worktree, `CARGO_TARGET_DIR=<wt>/target` | now | 0 ALL GREEN, 0 ALL GREEN |
| `guard_census.py`, `--check-runbook` | worktree | now | 0, 0. The LIMIT lines are the existing bench and panic-probe limits |
| `prose_check.py`, `source_provenance_check.py` | worktree | now | 0, 0 |
| implementer M1 to M15 | `.claude/scratch/F-210-progress.md` | prior receipt, not reused | not relied on |

Mutations (each applied alone, then reverted). "Red" means `cargo test -p ocelli-folder-viewer` failed:

- M1, `>` to `>=` in `cell_index`: red. M2, drop the last-cell branch: red. M3, ignore the row in the sample offset: red (AC11, AC14).
- M4, padding never matches: red (AC8). M5, CT default `unspecified`: red (7 tests). M6, drop the `US` arm: red (AC6).
- M7, read Units for every modality: survived. This is an equivalent mutant, because the `match` in `modality_unit` already gates on `PT`.
- **M8**, Rescale Type read at the top level only: **survived** (S2).
- **M9**, 24-bit padding guard raised to 64: **survived** (S3).
- M10, `ocelli_pixel::modality` `+ intercept` to `- intercept`: red in 8 `tests/value.rs` fixtures, including AC1, AC3 and AC11 as the plan predicts.
- M11, sign bit from Bits Allocated instead of Bits Stored: red (AC1). M19, no mask above Bits Stored: red (AC2).
- M12 to M18: the source header, a body with an id, `MAX_POINTS` at 65, value facts unset, clamping instead of refusing a point, a repeated points header accepted, and `stored_integer` dropping the sign. All red.

The scratch copy `target/review210` was deleted afterwards. `git status --short` is empty and `git write-tree` is still `e4c103cc`.

## Defects

### D1, the C.8.2.1 condition is transcribed incompletely, and the HU default ignores Multi-energy CT

**Where**: `examples/folder-viewer/src/value.rs:98-101` and `:110`, `examples/folder-viewer/tests/value.rs:341-342`, `.claude/plans/F-210-design.md:49` and `:188`.
**What**: The code comment and the plan say that C.8.2.1 requires Rescale Type on CT "only when the unit is not HU". The rule `(None, Some("CT"), _) => HU, ct-default` never reads Multi-energy CT Acquisition (0018,9361).
**Why it is wrong**: PS3.3 2026d C.8.2.1, Rescale Type (0028,1054), Type 1C: "Required if the Rescale Type is not HU (Hounsfield Units), or Multi-energy CT Acquisition (0018,9361) is YES. May be present otherwise." Rescale Intercept in the same table says the output is HU only when Image Type is ORIGINAL, not LOCALIZER, and "Multi-energy CT Acquisition (0018,9361) is either absent or NO". A multi-energy CT with no Rescale Type is therefore non-conformant, and HU cannot be inferred for it. Under C.11.1.1.2.1 its pixels may be Z_EFF, EDW, MGML or HU_MOD. The readout would print `HU` from `ct-default` on such an image, which is the quietly wrong label this project exists to avoid. The implementer's report of the extra clause is **confirmed**. The unit rule is right for every conformant object. It is wrong only for a non-conformant multi-energy object, where it should say `unspecified` (or refuse) instead of `HU`.
**Evidence**: The section text was fetched from `sect_C.8.2.html`. AC20 says "Any correction lands in this plan before the code", and the plan still carries the old sentence.
**Scope and regression**: Affects `/value` on CT and the F-211 consumer. Fix: read (0018,9361), and when it is `YES` with no Rescale Type, return `unspecified` from `none`. Correct the comment, the test comment and the plan line. Add a `CT multi-energy absent` row to `value_unit_follows_rescale_type`. No corpus row exercises it, because the corpus has no `ct-default` row at all.

### D2, the fixtures and the stored-sample comment cite PS3.3 C.7.6.3.1 for a rule that section does not contain

**Where**: `examples/folder-viewer/src/value.rs:231`, `examples/folder-viewer/tests/value.rs:2`, `:236`, `:251`, `.claude/plans/F-210-design.md:50`, `:169`, `:170`, `:230`.
**What**: Masking above Bits Stored and sign extension from bit `BitsStored - 1` are cited as "PS3.3 C.7.6.3.1 (Pixel Representation and Bits Stored)".
**Why it is wrong**: In 2026d, C.7.6.3.1 is "Image Pixel Module Attribute Descriptions". Its subsections are .1 Samples per Pixel through .8 Extended Offset Table, and none of them covers Bits Stored, High Bit or Pixel Representation. Those attributes sit in **C.7.6.3.3 Image Pixel Description Macro**, which says "High Bit (0028,0102) shall be one less than Bits Stored" and defers to PS3.5. The rule itself is **PS3.5 8.1.1**: "Receiving applications may not assume anything about the contents of unused bits, and in particular may not assume that they are zero, or that they contain sign extension bits", and "The sign bit shall be the High Bit in a Pixel Sample Value that is a 2's complement integer". The plan's line 50 also says that `unpack` "cites the section". It cites PS3.3 C.7.6.3 (`stored_pixel.rs:306`), not C.7.6.3.1. CLAUDE.md requires each pixel fixture to cite its DICOM section, and a citation to a section without the rule fails that requirement.
**Evidence**: `sect_C.7.6.3.html`, `sect_C.7.6.3.3.html` and `part05/chapter_8.html`, read now.
**Scope and regression**: Prose only, and the arithmetic is right (M11 and M19 go red). Change the citations to PS3.5 8.1.1 and PS3.3 C.7.6.3.3, in the plan and at the four code sites.

### D3, `/value` refuses frames the preview draws, for attributes the value never reads

**Where**: `examples/folder-viewer/src/value.rs:60` (`open_frame(parsed, frame).map_err(open_error)?`) and `:88-94`.
**What**: `value_facts` takes stage 1 and padding from `open_frame`. `open_frame` also parses the file window, the Presentation LUT Shape, Lossy Image Compression, Burned In Annotation and the private creators, and refuses the whole frame on any of them. A one-pixel CT with slope 1 and intercept -1024 was built and changed one attribute at a time:

| Change | `recipe` (preview) | `/value` |
|---|---|---|
| Window Width `0` | Ok | refused `window` |
| Window Width `abc` | Ok | refused `attribute` |
| Lossy Image Compression `1` | Ok | refused `attribute` |
| Burned In Annotation `MAYBE` | Ok | refused `attribute` |
| Presentation LUT Shape `BOGUS` | Ok | refused `attribute` |

**Why it is wrong**: The module doc (`value.rs:7-8`) says "It does not apply a window, so the value is the same for every window". The plan's invariant is "the value for a cell is a function of the file, the frame and the cell only", with C.11.1 as the only arithmetic input. In the first row the route refuses with `window`, whose sentence is "the window or presentation could not be applied", for a value that applies no window. The page then silently shows `C` and `R` only, while the user can still window the image with an override. Each changed attribute is non-conformant, but none of them is an input to PS3.3 C.11.1 or C.7.5.1.1.2. Refusing over them is a refusal the route's own contract does not allow. The rule "reports unavailable" covers inputs the feature cannot use, not inputs it does not read.
**Evidence**: The scratch probe `review_probe` printed the table above. The corpus has no such row, which is why AC12 and the 86-row agreement did not show it.
**Scope and regression**: Affects `/value` and F-211's probe. The fix is the operator's choice: (a) take stage 1 from `recipe.modality`, which AC12 already proves equal, and padding through a narrow reader. (b) Add a stage-1-and-padding accessor to `ocelli-dicom`, which is Q4's backlog row brought forward. (c) Keep `open_frame` and record in the plan and in the README sentence that a malformed window or flag attribute withholds the value. Regression: the five rows above as a table test asserting a value (or the declared refusal) for each.

## Smells

### S1, the boxed `ValueFacts` is outside F-216's cache accounting by design

**Where**: `examples/folder-viewer/src/preview.rs:460-464`, `examples/folder-viewer/src/held.rs:269` and `:1209`.
**What**: `Held::bytes()` charges `samples.capacity() + size_of::<HeldFrame>()`. `ValueFacts` is 120 bytes on the heap plus the `unit` string, up to 64 bytes for an LO value plus allocator rounding. The comment says the box exists so that "the held frame's accounted size grows by one pointer and not by the facts". Measured, the field is `Result<Box<ValueFacts>, PreviewError>` at 16 bytes, which is two words and not one pointer. `HeldFrame` is now 376 bytes.
**Why it is a smell**: F-216's plan (`.claude/plans/F-216-design.md:151`) defines `bytes()` as "capacity of the owned buffers plus size_of::<HeldFrame>()", and the RSS bound at `:279` relies on it. The box moves owned bytes out of `used` on purpose. For a cache full of small frames (1x1 up to tens of pixels, accounted at about 378 bytes each), about 150 unaccounted bytes per frame overruns the budget by about 40%. `FrameFacts`' six strings are an older gap of the same kind, carried from F-216. This story widens it.
**Scope and regression**: Charge `size_of::<ValueFacts>() + unit.capacity()`, through one `HeldFrame` method used by both `bytes()` and `reserve_for_insert`, and correct the comment. Regression: `held_bytes_are_the_buffer_capacity` extended for the new heap.

### S2, nothing tests per-frame Rescale Type resolution

**Where**: `examples/folder-viewer/src/value.rs:68`.
**What**: Mutation M8, which replaces `resolved_text(.. PIXEL_VALUE_TRANSFORMATION, RESCALE_TYPE)` with a top-level `optional_text`, passes every test.
**Why it is a smell**: In an Enhanced CT, Rescale Type is Type 1 inside the Pixel Value Transformation Sequence (PS3.3 C.7.6.16.2.9, Table C.7.6.16-10) and is usually absent at the top level. Under the mutant, an Enhanced multi-energy CT with `Z_EFF` or `HU_MOD` in its groups prints `HU`. The probe shows the current code reports `HU_MOD`, but no test would catch a regression. AC7 covers slope and intercept only.
**Regression**: Add to AC7 a frame whose group carries Rescale Type `HU_MOD` while the top level has none or `HU`.

### S3, nothing tests the 24-bit padding guard

**Where**: `examples/folder-viewer/src/value.rs:64`.
**What**: M9 raises the limit past 32 and every test still passes.
**Why it is a smell**: With Bits Stored above 24, `Stored(f32)` has already rounded the sample, so `stored_integer` can give a neighbour's integer, and a non-padding pixel can read as `padding` or the reverse. Bits Allocated 32 is accepted by `StoredBits::new`, so the guard is reachable, and a later refactor can delete it unnoticed.
**Regression**: A Bits Stored 32 object with Pixel Padding Value refused `attribute`, alongside the same object without padding answered.

### S4, PET `Units` loses to Rescale Type `US`

**Where**: `examples/folder-viewer/src/value.rs:107-111`.
**What**: The arm `(Some("US"), _, _)` runs before the PT `Units` arm, so a PET image carrying Rescale Type `US` and Units `BQML` reports `unspecified` from `rescale-type`.
**Why it is a smell**: PS3.3 2026d C.8.9.4 (PET Image Module), Rescale Intercept: "stored values (SV) and pixel value units (U) defined in Units (0054,1001): U = m*SV+b". Units is Type 1 in C.8.9.1. For a PET image, the normative unit of the rescaled value is Units, and `US` is the "Unspecified if Modality equals MR or PT" enumerated value of the multi-frame macro (C.7.6.16.2.9), not a statement that the unit is unknown. This is the outcome Q3's answer was meant to prevent. It is unconfirmed in practice, because the corpus has no PT row and the prevalence of legacy PET carrying a top-level `US` is unknown.
**Regression**: A `PT` case with Rescale Type `US` and Units `BQML` in `value_unit_follows_rescale_type`, after the plan's rule table is amended.

## Nitpicks

- N1. When the value is not finite, the reply is `PreviewError::Window` (`value.rs:192-194`), whose sentence is about the window. `attribute` is closer.
- N2. `decode_for_value` (`serve.rs:1353-1439`) is the third copy of F-216's decode, charge, store and release sequence, after `pooled_preview` and `preview_held`. All three copies are correct today. A later fix to the accounting has to reach all three.
- N3. The page re-requests the in-flight cell when the pointer leaves and comes back before the reply. That costs one duplicate request and changes nothing visible.
- N4. The test name `value_fractional_rescale_keeps_two_decimals` reads like a two-decimal rounding rule. There is none. The plan's rule is Rust's shortest round-trip `Display` for `f32`, and for example slope 1.23456, stored 123 and intercept -1024 print `-872.1491`. The name is true for its one case.

## Verified clean

- **AC20 transcriptions, against PS3.3 2026d**:
  - C.11.1. Output units = m*SV + b is confirmed. New in this edition: Rescale Type in the Modality LUT Module table is now 1C, "Required if Rescale Intercept is present". The plan does not mention it, and it does not change the rule, because the code already treats an absent Rescale Type on a non-CT, non-PT object as `unspecified`.
  - C.11.1.1.2 is titled "Modality LUT and Rescale Type". Its text is "Specifies the units of the output of the Modality LUT or rescale operation". The sentence the plan quotes ("specifies the output units of Rescale Slope and Rescale Intercept") is the Rescale Type row description in the C.11.1 table, not C.11.1.1.2. The implementer's report is **confirmed**. The defined terms OD, HU, US, MGML, Z_EFF, ED, EDW, HU_MOD and PCT match exactly, plus "Other values are permitted", which supports passing unknown codes through verbatim.
  - C.8.2.1: see D1, the implementer's report is **confirmed**.
  - C.8.9.1 is the **PET Series Module**, and Units (0054,1001) is Type 1 there. The PET Image Module is C.8.9.4. The implementer's report is **confirmed**. The code comment at `value.rs:101` is correct. The plan at `:188` and `:280` is wrong.
  - C.7.6.16.2.9 is the Pixel Value Transformation Macro, with Rescale Intercept, Slope and Type all Type 1 and `US` enumerated for MR and PT. The citation is correct.
  - C.7.5.1.1.2 is confirmed: stored space before the Modality LUT, inclusive range, Range Limit only with a Value, and the order depending on MONOCHROME1 or MONOCHROME2. `padding_of` and `PaddingRange::contains` implement it. `contains` takes min and max, so the order does not matter.
  - C.7.6.3.1: see D2.
- **Arithmetic.** There are no `as` casts in the diff. `cell_index` narrows to `f32` only after the coordinate is bounded by the extent, which is at most 65535, and goes through `exact_f32_integer` and `u32::try_from`. The `stored_at` offset uses checked multiplication and addition, followed by `get(start..end)`. `stored_integer` is checked at every step. Sign extension and masking come from the library `stored_sample` (M11 and M19 red). The value is `ModalityTransform::apply`, computed once per point from the `ModalityTransform` held in `ValueFacts`, and that transform is built once per held frame. The page does no arithmetic. Its contract test refuses the strings `slope`, `intercept` and `Rescale` in `page.html`.
- **Pixel cell.** The right edge is closed (`x == width` gives the last cell) and outside points are refused, not clamped (M1, M2, M16 red). Colour frames are refused at `frame_parts` before any value is read, so the one-sample offset in `stored_at` is never applied to RGB.
- **Route.** The order is: no id gives 400 `header`, a body with an id gives 400 `header`, a bad, empty, repeated or 65-pair points list gives 400 `points`, then 410 `file-gone`, then decode refusals, then `point`, then the held value refusal. The window headers are parsed by the shared header parser but never read by `value_route`, and AC14 runs them through both viewers. A hit is served without a decode or a workspace loan. A miss decodes once and stores, so the next `/value` and the next `/preview` report `held` (AC19, M12 red). The scheduled path charges and releases inflight exactly as `pooled_preview` does, including the error and job-not-run cases.
- **Page.** `postValue` sends `X-Ocelli-File`, `X-Ocelli-Frame` and `X-Ocelli-Points`, with no `body:`, no slope, no intercept and no pixels. A 410 re-posts once through `ensureFile` and retries once. A reply is shown only when its generation and both indices match the current cell. `resetValue` runs on a change of image or frame key and in `clearView`. Colour images send nothing. The 2 by 2 view sends nothing because `pointReadout` requires `gridMode === 1`.
- **Docs.** README step 7 matches the page's behaviour. `use.html` and `more.html` have no route list or cross sentence that this change makes stale. `git grep` found no surviving "column and a row" protocol sentence outside approved history.

## Limitations and handoff

- `.claude/scratch/F-210-progress.md` does **not** contain the "eight open decisions and five plan-versus-tree differences" in the assignment. They were presumably in the implementer's chat report, which this reviewer did not have. The differences this pass found, and the verdict on each:
  1. `value_json` takes `frame`. Fine.
  2. The `Box`. See S1.
  3. `prepare_held` fills `value` instead of `decode_held`. Fine.
  4. The route uses `preview_source` and its own decode instead of the plan's `Viewer::frame`, which does not exist in `held.rs`. Fine, see N2.
  5. A non-finite value is refused as `window`. See N1.
- Not exercised: the browser (AC16, sprint close), pydicom (AC17, pydicom is not installed), and multi-energy, PET or 32-bit objects from real devices, because the corpus has none.
- Next owner: the implementer remediates D1 to D3 and S1 to S4, D3 after the operator chooses its option. The plan's AC20 corrections land in `.claude/plans/F-210-design.md`. Then a fresh independent pass 2 runs.
