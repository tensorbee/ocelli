# F-267, folder viewer corners on enhanced multiframe CT and MR

**Status**: approved
**Epic ref**: E38.62
**Sprint**: S47
**Estimate**: 2w

This plan replaces the S46-close draft of the same file. That draft settled
its three open questions and was then reviewed independently. Round 2 fixed
four defects and four smells. Round 4 fixed the round 3 review's two
defects, five smells and four nitpicks: the top-level zero-length and
whitespace-only fall-through now has its own plants with a group value
behind each one, and the padding trim is stated exactly as the scan path's
`to_str` trims, every trailing space and NUL in any mix. The five smells
gained plants for a present multi-value top-level element over a group
value, group multi-value, LO for kVp and TR, the padded group fixture as
bytes, and the two classic mAs links on the grey reply. The four nitpicks
re-measured the `bin/ocelli.sh` numbers at the main checkout's sprint/s47
85ea89c4 and named the parser sources at first use. The coordinator's
amendment after the round 4 review added AC16 for a non-text top-level
element over a group value, a two-NUL group file in AC1, an all-NUL plant
in AC14, and corrected what AC9 and AC12 claim. The one question round 2
carried, a stray top-level Echo Time on a multi-echo enhanced MR, was
settled by the coordinator on 2026-11-11 from the story row: the top-level
attribute first where both exist, kept uniformly, and AC3's Echo Time row
asserts it. The plan carries no open question.

One prior claim is corrected plainly. The earlier text said the quotes were
fetched from the 2026d edition. **No 2026d edition exists.** The current
edition at `dicom.nema.org` is **2026c**, and that is what this plan
quotes. Every tree fact below was checked with a command in a named
session, and the standard facts name which session fetched and
re-fetched them. The commands are named where they matter.

The plan claims no implementation, no review cleanliness and no completion.
The estimate is inherited, lead-suggested and not capacity-approved.

## Normative source, transcribed

Everything in this section is from PS3.3, PS3.5 and PS3.6 edition 2026c,
fetched from `dicom.nema.org/medical/dicom/2026c/`. The round-2 session
fetched every quote below. The round-3 session, 2026-11-11, re-fetched and
re-checked the sections the story turns on: C.7.6.16.1.1, C.7.6.16.2.1,
C.7.6.16.2.3, C.8.15.3.8, C.8.15.3.9, C.8.15.3.11, C.8.13.5.2, C.8.13.5.4,
C.7.6.2, C.8.2.1 and C.8.3.1, Tables A.38-1, A.38-2, A.36-1 and A.36-2,
and PS3.5 6.2's FD row. The round-4 session re-fetched C.7.6.2 and read
Slice Thickness (0018,0050) as Type 2 in Table C.7-10, the classic home
AC14 cites. The PS3.6 Table 6-1 rows are carried from the
round-2 transcription unchanged, because `chapter_6.html` is larger than
the round-2 session's fetch limit, and AC13's reviewer check re-verifies every
row against the edition. The remaining round-2 checked statements, that no
fetched CT or MR macro page carries (0018,0050) and that no Enhanced MR
macro carries Echo Time, rest on that round's fetch of every macro page.

### PS3.3 2026c C.7.6.16.1.1, Functional Group

> Functional Groups are defined by editorial convention in Macros. Those
> Functional Groups that apply to all Frames are included in the Shared
> Functional Groups Sequence (5200,9229). Functional Groups whose Attribute
> Values may vary from Frame to Frame are included in the Per-Frame
> Functional Groups Sequence (5200,9230).
>
> A single Functional Group shall not be included in both the Shared
> Functional Groups Sequence (5200,9229) and the Per-Frame Functional Groups
> Sequence (5200,9230).

And in Table C.7.6.16-1, the Shared Functional Groups Sequence row and the
indented row beneath it:

> Only a single Item shall be included in this Sequence. See Section
> C.7.6.16.1.1 for further explanation.
>
> Include zero or more Functional Group Macros that are shared by all
> Frames. The selected Functional Group Macros shall not be present in the
> Per-Frame Functional Groups Sequence (5200,9230). For each IOD that
> includes this Module, a table is defined in which the permitted
> Functional Group Macros and their usage is specified.

So the standard states a prohibition, not a precedence: a macro present at
both levels is malformed. A reader that must still answer uses per-frame
first and keeps the duplicate observable, which is what
`MultiframeMetadata::resolve` in `crates/ocelli-dicom/src/multiframe.rs`
already does, including `DuplicateSources`.

### PS3.3 2026c C.7.6.16.2.1, Pixel Measures Macro, Table C.7.6.16-2

> Pixel Measures Sequence (0028,9110), Type 1. Identifies the physical
> characteristics of the pixels of this Frame. Only a single Item shall be
> included in this Sequence.
>
> \> Slice Thickness (0018,0050), Type 1C. Nominal reconstructed slice
> thickness (for tomographic imaging) or depth of field (for optical
> non-tomographic imaging), in mm.

The macro's other attributes are Pixel Spacing (0028,0030) Type 1C and
Spacing Between Slices (0018,0088) Type 1C. On the two IODs this story
touches it is the only functional-group macro that carries Slice Thickness.
Checked in the round-2 session by searching every fetched macro page of
Tables A.38-2 and A.36-2 for the tag (0018,0050): it appears in no CT macro
(C.8.15.3.1 to C.8.15.3.15) and no MR macro (C.8.13.5.1 to C.8.13.5.15),
only here and in the prose of C.7.6.16.2.3.1.

### PS3.3 2026c C.8.15.3.9, CT X-Ray Details Macro, Table C.8-125

> CT X-Ray Details Sequence (0018,9325), Type 1. Contains the Attributes
> defining the X-Ray information. If Multi-energy CT Acquisition
> (0018,9361) is NO or is absent, only a single Item shall be included in
> this Sequence.
>
> \> KVP (0018,0060), Type 1C. Nominal peak kilo voltage output of the
> X-Ray generator used. ... Required if Frame Type (0008,9007) Value 1 of
> this Frame is ORIGINAL or Image Type (0008,0008) Value 1 is ORIGINAL.
> May be present otherwise.

Table C.8-125 also says the converse for the YES case:

> If Multi-energy CT Acquisition (0018,9361) is YES, one or more Items
> shall be included in this Sequence.

**kVp has a second home in Enhanced CT, and the earlier draft's claim that
CT X-Ray Details is the only one was wrong.** PS3.3 2026c C.8.15.3.11, CT
Additional X-Ray Source Macro, Table C.8-126b:

> CT Additional X-Ray Source Sequence (0018,9360), Type 1. Contains the
> Attributes defining the data acquisition in a multiple X-Ray source
> system beyond the primary source. The primary X-Ray source is specified
> in the CT X-Ray Details Sequence. One or more Items shall be included in
> this Sequence.
>
> \> KVP (0018,0060), Type 1. Peak kilo voltage output of the X-Ray
> generator used.

The same macro carries Exposure in mAs (0018,9332) Type 1. Table A.38-2
lists CT Additional X-Ray Source, C.8.15.3.11, with usage "C - Required if
the image is reconstructed from a system with multiple X-Ray sources and
Multi-energy CT Acquisition (0018,9361) is NO or is absent". This story
reads the primary source only, through CT X-Ray Details, and does not read
the additional-source sequence. Its sequence is multi-item by definition,
so the reader's one-item rule would refuse it in any case.

### PS3.3 2026c C.8.15.3.8, CT Exposure Macro, Table C.8-124

> CT Exposure Sequence (0018,9321), Type 1. Contains the Attributes
> defining exposure information. If Multi-energy CT Acquisition (0018,9361)
> is NO or is absent, only a single Item shall be included in this
> Sequence.
>
> \> Exposure in mAs (0018,9332), Type 1C. The exposure expressed in
> milliampere seconds, for example calculated from exposure time and X-Ray
> tube current. Required if Frame Type (0008,9007) Value 1 of this Frame is
> ORIGINAL or Image Type (0008,0008) Value 1 is ORIGINAL. May be present
> otherwise.

As with CT X-Ray Details, the YES case permits one or more Items. Both
shapes are legal, and both resolve to an empty corner under the one-item
rule below, which is a designed outcome and is tested.

### PS3.3 2026c C.8.13.5.2, MR Timing and Related Parameters Macro, Table C.8-89

> MR Timing and Related Parameters Sequence (0018,9112), Type 1. Identifies
> the timing and safety information of this Frame. Only a single Item shall
> be included in this Sequence.
>
> \> Repetition Time (0018,0080), Type 1C. The time in ms between two
> successive excitations of the same volume. Shall be 0 (zero) if there is
> a single excitation per volume. Required if Frame Type (0008,9007) Value
> 1 of this Frame is ORIGINAL. May be present otherwise.

### PS3.3 2026c C.8.13.5.4, MR Echo Macro, Table C.8-91

> MR Echo Sequence (0018,9114), Type 1. Identifies echo timing of this
> Frame. Only a single Item shall be included in this Sequence.
>
> \> Effective Echo Time (0018,9082), Type 1C. The time in ms between the
> middle of the excitation pulse and the peak of the echo produced for
> kx=0. Required if Frame Type (0008,9007) Value 1 of this Frame is
> ORIGINAL. May be present otherwise.

Echo Time (0018,0081) itself appears nowhere in the Enhanced MR Image IOD.
Checked in the round-2 session against 2026c: all fifteen Enhanced MR functional
group macros, C.8.13.5.1 through C.8.13.5.15, where only MR Echo carries an
echo-time attribute at all, the Enhanced MR Image page C.8.13, which
contains the Enhanced MR Image Module and the MR Image and Spectroscopy
Instance Macro and has no Echo Time text, and the MR Pulse Sequence Module
C.8.13.4. The Enhanced MR echo-time attribute is Effective Echo Time, whose
definition is the classic Echo Time definition. PS3.3 2026c C.8.3.1, MR
Image Module, Table C.8-4:

> Echo Time (0018,0081), Type 2. Time in ms between the middle of the
> excitation pulse and the peak of the echo produced (kx=0). In the case of
> segmented k-space, the TE(eff) is the time between the middle of the
> excitation pulse to the peak of the echo that is used to cover the center
> of k-space.

Both name the same quantity, the excitation-to-echo-peak time at kx=0. On an
enhanced MR file the corner's `echoTime` reads Effective Echo Time. That is
not a guess. It is the IOD's own echo-time attribute under the standard's
own name.

### PS3.6 2026c Table 6-1, VR and VM rows

Extracted in the round-2 session from the downloaded `chapter_6.html`, and
carried unchanged per the provenance note above:

| Tag | Name | Keyword | VR | VM |
|-----|------|---------|----|----|
| (0018,0050) | Slice Thickness | SliceThickness | DS | 1 |
| (0018,0060) | KVP | KVP | DS | 1 |
| (0018,0080) | Repetition Time | RepetitionTime | DS | 1 |
| (0018,0081) | Echo Time | EchoTime | DS | 1 |
| (0018,1150) | Exposure Time | ExposureTime | IS | 1 |
| (0018,1151) | X-Ray Tube Current | XRayTubeCurrent | IS | 1 |
| (0018,1152) | Exposure | Exposure | IS | 1 |
| (0018,1153) | Exposure in µAs | ExposureInuAs | IS | 1 |
| (0018,9082) | Effective Echo Time | EffectiveEchoTime | FD | 1 |
| (0018,9112) | MR Timing and Related Parameters Sequence | MRTimingAndRelatedParametersSequence | SQ | 1 |
| (0018,9114) | MR Echo Sequence | MREchoSequence | SQ | 1 |
| (0018,9321) | CT Exposure Sequence | CTExposureSequence | SQ | 1 |
| (0018,9325) | CT X-Ray Details Sequence | CTXRayDetailsSequence | SQ | 1 |
| (0018,9332) | Exposure in mAs | ExposureInmAs | FD | 1 |

PS3.6 gives one VR per tag, and the functional group tables do not restate
VRs. Three of the five story fields are decimal strings inside their groups,
Slice Thickness, kVp and TR, and exactly two attributes on this story's
path are FD: Exposure in mAs (0018,9332) and Effective Echo Time (0018,9082).
TE has a DS top-level form, Echo Time (0018,0081), and an FD group form.
The Keyword column is where the mAs provenance token comes from.

PS3.5 2026c 6.2, Table 6.2-1, the FD row, transcribed:

> Floating Point Double | Double precision binary floating point value
> represented in IEEE 754 binary64 format. All IEEE 754 values are
> permitted, including NaN (Not a Number) and infinity values. |
> not applicable | 8 bytes fixed

So an FD element may legally carry a non-finite value, and a rule has to
say what the corner does with one. This plan shows a non-finite value as an
absent field. A corner line prints a quantity with a unit, and the tree
already treats a present but non-finite value as no value in the same
position: `push_cine` at `preview.rs` line 3417 takes a Frame Time only
`if milliseconds.is_finite()`.

The S46-close draft's premise that the groups carry "FD VRs this preview
does not read" across the board is corrected here. The real gap is that the
corner path reads the top-level object only, and the groups carry four of
the five fields as text it never resolves.

### Which IOD includes which macro, PS3.3 2026c Tables A.38-2 and A.36-2

Enhanced CT Image, Table A.38-2: Pixel Measures M. CT Exposure and CT
X-Ray Details are C, each "Required if Image Type (0008,0008) Value 1 is
ORIGINAL or MIXED, may be present otherwise". Enhanced MR Image, Table
A.36-2: Pixel Measures M, MR Timing and Related Parameters C, MR Echo C,
the two C rows carrying that same wording.
Neither table restricts these macros to one functional group level, so a
producer may place them shared or per-frame as its data varies. The reader
must therefore resolve through both levels, which the existing `resolve`
does. Frame Content is the counter-example whose usage row does carry the
restriction, "M - May not be used as a Shared Functional Group", which
shows the table states such restrictions when they exist.

### The classic attributes the top-level rule covers

PS3.3 2026c C.8.2.1, CT Image Module, Table C.8-3, carries KVP (0018,0060)
Type 2, Exposure (0018,1152) Type 3 and Exposure in µAs (0018,1153) Type 3
at the top level, and C.8.3.1, MR Image Module, Table C.8-4, carries
Repetition Time (0018,0080) Type 2C and Echo Time (0018,0081) Type 2. Slice
Thickness (0018,0050) is in the Image Plane Module, C.7.6.2. None of these
modules is in the Enhanced CT or Enhanced MR module tables (PS3.3 2026c
Tables A.38-1 and A.36-1, checked in the round-2 and round-3 sessions), so a conformant enhanced
file carries these fields only inside functional groups. A file that
carries both a classic top-level attribute and the group form is
non-conformant, and the standard defines no precedence for it. The story
row's "the top-level attribute first where both exist" is viewer policy for
exactly that case, and this plan keeps it as stated.

### The tree's own contract for the corners

`.claude/plans/F-212-design.md`, transcribed:

> Every derived string is made in Rust, in a new
> `examples/folder-viewer/src/corners.rs`, and sent in the inspect JSON. The
> page places strings and does not reformat them.

> Non-goals: a configurable corner layout, a hide-patient-data key,
> functional-group acquisition fields, oblique two-letter edges from Image
> Orientation (Patient), Estimated Radiographic Magnification Factor, and a
> scale bar in the 2 by 2 view or in the F-208 grid cells.

> 4. **Enhanced multiframe acquisition fields.** Left empty. Reading Exposure
> in mAs (0018,9332) and the MR timing sequences needs FD support in this
> preview. Owner: operator. Recommendation: a follow-up story. Deferral
> leaves those corner lines empty on enhanced files, never wrong.

And the page's printing rule, `examples/folder-viewer/src/page.html` lines
3460 to 3463:

> Each attribute value below was formatted by the local process, and the
> page adds only labels and units. A field the file does not have leaves
> its line out. A DS arrives as a JSON number and prints as JavaScript
> writes it, so a stored -12.50 shows as -12.5.

HLD `24-agent-code-standards.md` 27.2 R3: "Every function doing pixel
arithmetic needs a fixture test with hand-computed values, citing the DICOM
section". The FD rule here is formatting, not pixel arithmetic, but the
story row requires the fixture regardless, and it cites the PS3.3 macro
that declares each FD attribute.

The one FD rule already has a tree precedent. `serve.rs` line 2473:
"`Display` for `f64` never uses an exponent, as `/measure` writes it."
`measure.rs` lines 451 to 452 say the same for `f32`, and `preview.rs`
line 3434 applies it to `frameTimeMs`. `Display` on a finite `f64` prints
the shortest form that reads back as the same value, and never an
exponent. Measured in the round-3 session with rustc and re-measured in
round 4 with the same probe: 5.5 prints `5.5`,
3.0 prints `3`, 12.25
prints `12.25`, 0.1 prints `0.1`, 0.123456789 prints `0.123456789`, 1.0e-7
prints `0.0000001`, 1.0e21 prints `1000000000000000000000`, negative zero
prints `-0`, and 1.0e300 prints 301 digits with no exponent.

### The precedence rule, stated once

This subsection is the single statement of the corner precedence. Every
other section refers to it and does not restate it.

**Order.** For each of the five fields, the corner for the shown frame reads
sources in this order and shows the first source that yields a token.

1. Top level. The field's classic top-level attribute or attributes, read
   from the projection `frame_facts` receives, a `MetadataSet`, and not
   from the `DefaultDicomObject` the scan path holds. `frame_facts` at
   `preview.rs` line 2324 takes only the `MetadataSet` and the
   `MultiframeMetadata`, and `InspectParts` at line 3145 holds no object
   either, so the scan path's `corner_decimal` cannot be called here. The
   top-level rule is stated on the projection instead, in terms of the
   projected element's own `value()` and `semantic_text()`
   (`metadata.rs` lines 239 and 249). An absent element, a present
   `MetadataValue::Empty` element, and a text element whose one component
   is empty after the trim below are absent and fall to the groups. A
   `MetadataValue::Text` element, which DS and IS project to
   (`metadata.rs` lines 341 to 342), yields a token only when it has
   exactly one component, with no empty-part filter, after the trim
   below, through `clean_decimal`. A multi-valued element yields no
   token, which is the refusal the scan path's own `corner_decimal`
   makes for the same bytes, because `to_str` joins the components back
   with `\` and `clean_decimal` does not parse that (`preview.rs` lines
   3662 to 3666, its own doc comment). A stored `5\` projects as two
   components, `5` and empty, because the reader splits the value bytes
   on the backslash (dicom-parser 0.10.0, the version `Cargo.lock` pins
   at line 313, `src/stateful/decode.rs` lines 486 and 495), and a
   stored `5\6` as two components, so both refuse, as they refuse in the
   scan path.
   Any other projected
   variant, `Float64` from an FD or an OD element, `Unsigned16` from a US
   element and the rest, is present and non-text, and ends the read with
   no token. mAs is the `exposure_mas` chain over the two top-level
   tags, Exposure (0018,1152) as IS, then Exposure in µAs (0018,1153) as
   IS over 1000, each under this same one-component rule. **The top
   level has no VR check.** The projected text rule above is the only
   gate there: any text VR yields a token under it, exactly as the scan
   path's `corner_text` accepts `Str` and `Strs` whatever the element's
   VR, and a non-text VR ends the read with no token. The VR check is
   the group read's, below, and AC7's FD-encoded KVP row plants the
   non-text direction.
2. The frame's own per-frame functional group item, through
   `MultiframeMetadata::resolve(frame, group, attribute,
   TopLevelFallback::Disallowed)`.
3. The Shared Functional Groups item, through the same `resolve` call,
   which already orders per-frame before shared and returns the first
   present source.

**The top level first is the story row's rule, uniformly.** The row says
"the top-level attribute first where both exist", and the coordinator
confirmed on 2026-11-11 that this is decided, not an open question. A
multi-echo enhanced MR that carries a stray classic Echo Time (0018,0081)
at the top level shows that value on every frame. AC3's Echo Time row
asserts it.

**Absent and zero-length fall through at the top level only.** At the top
level, an absent element and a present zero-length one fall to the groups.
That is the mapping `corner_text` already makes from `Empty` to absent
(`preview.rs` line 3650). A whitespace-only top-level element is absent
the same way: the scan path's `optional_text` at `preview.rs` line 2261
trims it to empty and returns `None`, so the projection rule trims first
and treats the empty remainder as absent. Inside the groups there is no
fall-through, because the read is one `resolve` call (`multiframe.rs`
line 234): a per-frame element that is present but zero-length is
returned by `resolve` itself (lines 267 to 280) and never replaced by
the shared one, so it answers the read with no token. That is the same
answer `spacing_pair` gives a resolved `Empty` element, `Unusable` at
`preview.rs` lines 2446 to 2448, and a shared zero-length element
answers the same way. AC2 carries the group row and AC14 the
top-level rows, each planted with a group value behind it.

**A present top-level value that yields no token stops.** For the four
single-attribute fields, a present top-level element that is neither
absent, nor zero-length, nor trim-empty, and that yields no token, ends
the read with no token. The two cases are a multi-valued text element
and a non-text element. Neither falls through to the groups. This is the
measurement rule the tree keeps in `spacing_pair` and
`calibration_inputs` at `preview.rs` lines 2434 and 2488: a present
attribute that is not one usable value refuses rather than reading the
next source. AC7's `5\6` row plants the multi-value stop with a shared group value
behind it, and AC16 plants the non-text stop the same way, so a read
that falls through to the group prints that value and goes red.

**The mAs chain falls through on unusable.** `exposure_mas` at
`corners.rs` line 129 already falls from an unusable Exposure to a usable
Exposure in µAs, so the chain keeps that rule and adds the group's Exposure
in mAs as its last source. Exposure `abc` at the top level with Exposure in
mAs 5.5 in the group shows 5.5 with source `ExposureInmAs`. A multi-valued
Exposure `200\300` is unusable under the one-component rule the same way,
and `integer_string` at `corners.rs` line 304 refuses the joined text the
scan path would pass, so both paths fall to the next source. A top-level
value that is not text is unusable the same way.

**Inside the groups, resolve's own precedence holds.** The per-frame value
wins over the shared value even when it is present but unusable, so an
unusable per-frame value empties the field rather than reading the shared
one. A group sequence that is present but not a one-item SQ makes
`resolve` return `MultiframeError::InvalidFunctionalGroup`
(`multiframe.rs` line 376, `attribute_in_group`), which the corner maps to
no token, so the field is empty and nothing refuses. That includes a
conformant Multi-energy CT Acquisition file, whose CT Exposure or CT
X-Ray Details Sequence legitimately has several items. The Pixel Measures
Sequence is the one exception and is today's refusal, not a corner case,
because `spacing_pair` reads it first and its `map_err` refuses the
frame's facts through `held.facts.clone()?` at `preview.rs` line 1384.
That exception is stated in the refusal policy below.

**The token read checks VR.** A resolved group element yields a token only
when its VR, `element.vr()` (`metadata.rs` line 233), equals the field's
expected VR from PS3.6 2026c Table 6-1: DS for (0018,0050), (0018,0060)
and (0018,0080), FD for (0018,9082) and (0018,9332). A DS element yields
its token from `semantic_text` with exactly one component, no empty-part
filter, after the same trim, through `clean_decimal`. An FD element
yields its token from `MetadataValue::Float64` with exactly one value
through `fd_text`. The value variant alone is not the VR check, because
dicom-parser 0.10.0, the version `Cargo.lock` names at line 313, decodes
OD through the same F64 path as FD: `src/stateful/decode.rs` line 1056
in `read_value_preserved` and line 1015 in `read_value` are the same
arm, `VR::FD | VR::OD => self.read_value_od(header)`, and
`read_value_od` returns `PrimitiveValue::F64` (line 750). The collector
builds each element from the token's `into_value`, whose default
strategy is `ValueReadStrategy::Preserved` (`dicom-parser 0.10.0
src/dataset/mod.rs` lines 331 to 333, called from dicom-object 0.10.0's
`src/collector.rs` line 1154), so an OD element projects to
`MetadataValue::Float64` exactly as an FD one does. A non-DS text VR
projects to `Text` exactly as DS does, the `read_value_strs` arm of
`read_value_preserved` at `decode.rs` lines 1039 to 1050 carrying LO at
1043 and DS at 1047. The AC12 rows plant an OD field and a non-DS text VR
field for exactly this reason.

**The text is trimmed the same way at both levels.** The scan path trims
exactly as `to_str` does: dicom-core 0.10.0, the version `Cargo.lock`
names at line 259, defines `to_str` in `src/value/primitive.rs` with
`trim_end_matches([' ', '\u{0}'])` at lines 580, 583 and 588, which
removes every trailing space and NUL in any mix, and `optional_text`
(`preview.rs` line 2261) trims spaces at both ends after it. A
classic Slice Thickness stored as the bytes 31 32 00 00 reads `12`
through that path. The projected element's `semantic_text` trims spaces
by VR but not a NUL (`metadata.rs` lines 249 to 265), so both corner
levels remove every trailing space and NUL in any mix,
`trim_end_matches([' ', '\u{0}'])`, after the VR trim and before
`clean_decimal`: the top-level read, so a NUL-padded classic value
prints identically from the grey path and the scan path, and the group
read, so a NUL-padded DS inside a group yields its token at both levels
alike and the two levels do not disagree about padding. A trim of one
trailing NUL only would leave `12\0` from those bytes, which
`clean_decimal` does not parse, and the classic file would lose its
Thickness line while its scan JSON shows 12. AC1 carries a group row
with those two-NUL bytes and AC7 the top-level row.

## What the specification does not cover

1. **How per-frame corner values reach the page.** The standard and the HLD
   say nothing about the viewer's corner payload. The F-212 contract sends
   derived strings in the inspect JSON, which is per file, and the story
   requires values per shown frame. This plan decides: the values ride the
   grey reply as response headers, the same per-frame channel that already
   carries spacing and edge facts for the shown frame. The comparison with
   the per-frame `/inspect` route, which does not depend on how the frame
   is drawn, and the reason for the choice, are under Consumer and
   provider map.
2. **The FD formatting rule and its rounding.** Settled as tree precedent:
   Rust `Display` on a finite value, empty for a non-finite one, VM 1
   only. No rounding and no trimming, because PS3.5 6.2 encodes FD as
   binary IEEE 754 with no padding to trim.
3. **Whether `echoTime` on enhanced MR shows Effective Echo Time.** Settled
   by the definition match transcribed above. The alternative, leaving TE
   empty on every enhanced MR file, would leave a field the file does
   carry unshown, which is the wrong reading of "left empty rather than
   guessed". The story's empty rule is for a field absent from both places.
4. **Precedence when a value is present but unusable.** Settled in the
   precedence rule above, once. The four single-attribute fields stop, the
   mAs chain falls through as `exposure_mas` already does, and zero-length
   and whitespace-only fall through at the top level while inside the
   groups a resolved zero-length element answers with no token, because
   one `resolve` call returns the per-frame element and never the shared
   one.
5. **The provenance token for the FD mAs.** Settled as `ExposureInmAs`, the
   PS3.6 2026c keyword of (0018,9332), in the pattern of the existing
   `Exposure` and `ExposureInuAs` tokens.
6. **A `Slice Location` counterpart on enhanced files.** There is none.
   PS3.3 2026c C.7.6.16.2.3, Plane Position (Patient) Macro, Table
   C.7.6.16-4, carries only Image Position (Patient). F-212 deliberately
   does not derive Slice Location from plane position, and this plan keeps
   that. `sliceLocation` stays empty on enhanced files.
7. **How the page keeps a stale frame's facts off the screen.** The HLD
   says nothing about the page's repaint paths. Decided here: the facts
   object carries the file serial and frame it belongs to, and the
   acquisition lines use it only when it matches the image and frame being
   painted, with a browser check. The existing spacing, edge and
   calibration reads are not changed by this story.

## Problem and reproducer

`push_corner_fields` at `examples/folder-viewer/src/preview.rs` line 3672,
called from the inspect reply builder at line 3368, reads every acquisition
field on the top-level `DefaultDicomObject` only. Its loop at lines 3723 to
3731 pushes `sliceLocation`, `sliceThickness`, `kvp`, `repetitionTime` and
`echoTime` through `corner_decimal` from the tag constants at lines 108 to
114, and exposure through `corner_text` and `exposure_mas` at lines 3732 to
3736. Checked with `grep -n "push_corner_fields\|corner_decimal"
examples/folder-viewer/src/preview.rs`. An enhanced CT or MR file keeps
these in functional groups, so on one of those files the corner lines are
absent today. That is F-212's stated non-goal, quoted above, and the story
filed to close it.

Reproducer. Build a two-frame enhanced CT in the pattern of
`enhanced_frames` at `examples/folder-viewer/tests/preview.rs` line 1256,
which already hand-builds a shared functional group and a per-frame
functional group. The new file carries SOP Class UID
1.2.840.10008.5.1.4.1.1.2.1, Enhanced CT Image Storage, where
`enhanced_frames` itself carries the classic CT UID, and Slice Thickness
only in the shared Pixel Measures Sequence. `inspect_json(&file, 0)`
reports `"sliceThickness":null` today. After this story, the grey reply
for the shown frame carries `X-Ocelli-Slice-Thickness` with the stored
token, and the page prints the Thickness line. The same file with the field
in a per-frame group shows each frame's own value. Expected values come
from PS3.3 by hand, never from the implementation.

## Scope, API and compatibility

In scope: Slice Thickness, kVp, mAs, TR and TE resolved for the shown frame
under the precedence rule above, formatted by the existing DS rule and the
one FD rule, and shown on the existing viewer front door. The scan JSON
keeps F-212's field names and shape and is unchanged. The grey reply gains
six response headers. The page parses five of them and changes only the
acquisition lines of `paintOverlays`.

Non-goals: an enhanced Slice Location counterpart, per-frame window or VOI
changes, palette or ICC changes, any change to classic files' corners or
their scan JSON, exposure computed from tube current and exposure time, a
new viewer page, corners in the 2 by 2 grid cells, a change to
`ocelli-dicom`, and a change to the existing spacing, edge or calibration
fact reads. The resolution uses the crate's existing public API.

The per-field sources, with the top-level tag or tags and the group tag
stated separately because they are not the same tag for every field:

| Field | Top level, classic home | Group sequence | Group attribute | Group VR |
|-------|-------------------------|----------------|-----------------|----------|
| Slice Thickness | (0018,0050), Image Plane C.7.6.2 | Pixel Measures (0028,9110) | (0018,0050) | DS |
| kVp | (0018,0060), CT Image Module C.8.2.1 | CT X-Ray Details (0018,9325) | (0018,0060) | DS |
| mAs | (0018,1152) then (0018,1153) over 1000, CT Image Module C.8.2.1 | CT Exposure (0018,9321) | (0018,9332) | FD |
| TR | (0018,0080), MR Image Module C.8.3.1 | MR Timing and Related Parameters (0018,9112) | (0018,0080) | DS |
| TE | (0018,0081), MR Image Module C.8.3.1 | MR Echo (0018,9114) | (0018,9082) Effective Echo Time | FD |

Concrete new types and signatures:

```rust
// examples/folder-viewer/src/corners.rs, public, re-exported by lib.rs
/// The one FD token: a finite value as `Display` writes it, `None` for a
/// non-finite one. PS3.5 6.2 encodes FD as IEEE 754 binary with no padding.
pub fn fd_text(value: f64) -> Option<String>
```

```rust
// examples/folder-viewer/src/preview.rs, six fields appended to FrameFacts
pub slice_thickness: Option<String>,   // top level (0018,0050) or (0028,9110) > (0018,0050), DS
pub kvp: Option<String>,               // top level (0018,0060) or (0018,9325) > (0018,0060), DS
pub exposure_mas: Option<String>,      // (0018,1152) then (0018,1153) over 1000, or (0018,9321) > (0018,9332), FD
pub exposure_source: &'static str,     // "" | "Exposure" | "ExposureInuAs" | "ExposureInmAs"
pub repetition_time: Option<String>,   // top level (0018,0080) or (0018,9112) > (0018,0080), DS
pub echo_time: Option<String>,         // top level (0018,0081) or (0018,9114) > (0018,9082)
```

A private `corner_token` beside `spacing_pair` resolves one field for one
frame and never refuses. It takes the field's top-level tag or tags and its
group sequence, group attribute and group VR separately, because for TE the
top-level tag is Echo Time (0018,0081) and the group attribute is Effective
Echo Time (0018,9082), and for mAs the top level is the two-tag
`exposure_mas` chain while the group is one FD attribute. It applies the
precedence rule above: the top level first under the projection rule
stated there, then `multi.resolve` per frame, every `MultiframeError`
mapped to `None`.

`Grey` in `examples/folder-viewer/src/serve.rs` gains the same six fields,
`from_preview` maps them from `preview.facts`, and the header writer adds:

- `X-Ocelli-Slice-Thickness`
- `X-Ocelli-Kvp`
- `X-Ocelli-Exposure-Mas`
- `X-Ocelli-Exposure-Source`
- `X-Ocelli-Repetition-Time`
- `X-Ocelli-Echo-Time`

Each of the five value headers is written only when its token is `Some`, in
the pattern of `X-Ocelli-Row-Spacing` at `serve.rs` line 2438. The tokens
are validated JSON number tokens, so no header can carry a newline.
`X-Ocelli-Exposure-Source` is written always, as `X-Ocelli-Spacing-Source`
is, with one of four fixed `&'static str` values and an empty value when
no source answered. It is the tests' observable for which link of the
chain answered. **The page does not parse it.** Nothing on the page reads
an exposure source today, the scan JSON's `exposureSource` included, so a
parse through `headerNumber` would give `Number("ExposureInmAs")`, which
is `NaN`, and a value nothing reads. The page parses the five value
headers only.

`page.html` parses the five headers into the `facts` object of
`requestGrey`'s reply through the existing `headerNumber`, which returns a
JS `Number` or `null`, and stamps the object with the identity it belongs
to, `serial: fileSerial(image.file)` and `frame: image.frame`, the same
identity `cacheKey` at page.html line 2609 already builds from.
`paintOverlays` then reads each of the five fields on the acquisition
lines at page.html lines 3476 to 3481 as: the facts value when
`vp.shownFacts` carries the serial and frame of the image being painted,
otherwise the scan field `info.X`. This is the stale-facts fix. Without
the identity, a repaint through the corner toggle, a flip, a tile-mode
change or a resize while the next frame's grey is in flight would paint
the previous image's thickness on the new image, because `vp.shownFacts`
still holds the previous `applyGrey`'s object from page.html line 2585.

The page reprints through `Number`, exactly as the scan path's JSON number
already does, so the stored token and the printed line differ where
JavaScript writes an exponent: a stored `0.0000001` prints as `1e-7`, the
stored 1e21 token `1000000000000000000000` prints as `1e+21`, and a stored
`-0` prints as `0`. This is the DS path's own behaviour, not a new rule,
and the plan states it so a reader does not mistake it for a formatter
bug. The Rust side still writes `0.0000001`, `1000000000000000000000` and
`-0`, and those are what the fixture asserts.

Refusal policy. No new read refuses a scan, a grey or a frame. A malformed
group in one of the corner's own groups is an empty field. `PreviewError`
and the reply shapes are unchanged. The existing refusal of a malformed
multiframe structure through `MultiframeMetadata::new`, which refuses the
frame's facts via `held.facts.clone()?` at `finish_preview` line 1384, is
today's behaviour and is not touched by this story. A malformed group
inside an otherwise valid structure is not a construction failure, it is
an `InvalidFunctionalGroup` from `resolve`, and the precedence rule maps
it to an empty field. **The one exception is the Pixel Measures Sequence,
and it is today's refusal, not a corner outcome:** `spacing_pair` reads
it for spacing and maps a `resolve` error to a refusal of the frame
(`preview.rs` lines 2440 to 2441), which `held.facts.clone()?` at line
1384 propagates as a refusal of the reply. A malformed Pixel Measures
Sequence at either level refuses the frame today and after this story,
and the corner adds nothing there. The corner-empty rule covers CT
Exposure, CT X-Ray Details, MR Timing and Related Parameters and MR
Echo, which nothing else in the read path resolves.

Compatibility. The scan JSON is byte-identical for every file the current
suite covers, because `push_corner_fields` is untouched. **Classic** in
this plan names where a field lives, the classic top-level homes of
PS3.3 C.7.6.2, C.8.2.1 and C.8.3.1, and never the SOP Class. A file whose
SOP Class UID is the classic CT UID can still carry functional groups,
and the suite's `enhanced_frames` builder does exactly that
(`tests/preview.rs` lines 1275 to 1278, UID 1.2.840.10008.5.1.4.1.1.2).
Such a file with a group value and no top-level value now shows the group
value on the grey reply, which is this story's point, while its scan JSON
is unchanged. A file whose five fields live at the top level and whose
groups carry no copy resolves every field under the projection rule,
which matches the scan path's `corner_decimal` and `exposure_mas` on
every input the projection carries: absent, empty, whitespace-only, one
value, multi-value, wrong VR and NUL padding. AC7's plants assert that
agreement on the boundary inputs. The new headers are additions to the
grey reply and no consumer outside this page reads them.

## Consumer and provider map

| Consumer / entry point | Concrete provider | Caller context / policy | Observable output |
|------------------------|-------------------|-------------------------|-------------------|
| `paintOverlays`, single view corners | `requestGrey` reply headers from `Grey`, set into `vp.shownFacts` at `applyGrey`, used when the serial and frame match the painted image | Once per drawn frame, labels and units on the page, reprints through `Number` | The Thickness, tube and timing lines of the shown frame |
| Scan list and the page's `info` | `inspect_json` unchanged, `push_corner_fields` top level only | Per file, frame-independent fields only | Unchanged JSON, byte-identical |
| Per-frame `/inspect` reply, `refreshFrameWindows` | `InspectParts::frame_fields` | Windows and VOI per frame as today, the new tokens ride `FrameFacts` and are not serialized there | Unchanged JSON shape |
| `KeptReplies` pool | `kept_replies`, byte-counted per frame | The kept per-frame strings do not grow, because the corner tokens are not serialized into them, and `KeptReplies` stores strings, not `FrameFacts` | Same memory accounting |
| Length tool and scale bar | `calibration_inputs` and `choose_spacing` | Unchanged, the corner read adds no calibration input | Unchanged |
| `ocelli-dicom` | `MultiframeMetadata::resolve` and `MetadataValue`, existing public API | Per-frame then shared when the top level is absent, duplicates retained | No new export |
| F-202's worker-surface consumer | The shared Rust provider, a future story's design task | Draws frames through `packages/core` and the wasm session, not through `/preview`. `corner_token` is private to this example crate, so no current export carries the rule into any other crate. Moving a shared provider into a crate a wasm consumer reaches is F-202's to own | Not this story's output |

**The channel choice, made against the alternative.** The per-frame
`/inspect` route (`InspectParts::frame_fields`, `preview.rs` line 3163,
whose `frame_facts` call is at line 3175) does not depend on how the frame
is drawn, which the grey reply does. It was considered as the channel and
rejected for this story, with the F-202 consequence kept visible rather
than hidden. The reasons: the corners annotate the drawn frame, and the
grey reply is the frame's own response, so the values and the pixels they
annotate arrive together and paint once. The page already consumes
per-frame facts from exactly that reply, so five more fields is the
smallest change. Moving them to `/inspect` would need a new ask on every
frame open, because `refreshFrameWindows` at page.html line 3864 runs only
for non-colour files with `windowsPerFrame` and a frame above 0, a new
info flag to know whether to ask, and a two-phase paint in which the
corners arrive after the frame is shown. A worker-drawn consumer gets
neither HTTP channel today, because its bytes live in the browser, so the
header choice does not take anything away from F-202 that the inspect
choice would have given it. F-202's plan is a draft whose module and
signature selection is deferred to its own design completion, and its
consumer row above names the shared contract it will consume, whose move
into a wasm-reachable crate is that story's to own.

Units, rounding, selection and the absent-field rule are owned by the Rust
provider, `corners.rs` for the FD rule and `clean_decimal` for DS. The
page owns only labels, units and the JavaScript printing of a JSON number.

## State transitions and invariants

| Entry point / starting state | Input or event | Resulting state and output | Refusal and preserved state |
|------------------------------|----------------|----------------------------|-----------------------------|
| Grey request for frame n | `POST /preview` by held file id with `X-Ocelli-Frame: n`, or with the file bytes when not yet held, `sendPreview` at page.html line 1770 | Headers carry frame n's own resolved tokens under the precedence rule | A malformed group in a corner field gives no header, the reply still draws |
| Frame switch, grey cached | `takeGrey` hits for the new frame's key | `applyGrey` sets `shownFacts` from the cached reply and repaints | The previous frame's lines are never shown after the new paint |
| Repaint while a grey is in flight | Corner toggle, flip, tile change or resize through the existing `paintOverlays` callers | The acquisition lines use `info`, because the tagged facts do not match the image and frame being painted | No value is ever shown from a frame or image other than the one on screen, and `applyGrey` replaces all of them at once |
| Image without a drawable frame | `showIndex` marks it undrawable | `vp.shownFacts = null` at page.html line 2487, corners fall back to `info` | No invented values, the scan-level fields still apply |
| Field absent at all three levels | Any frame | No header, no corner line | Never zero, never another frame's value |
| Top level present, no token | Any frame | Per the precedence rule: a non-text or multi-value element stops the four single-attribute fields with no header, the mAs chain falls to its next source, and a zero-length or whitespace-only element is absent and falls to the groups | Never a guess from tube current and exposure time, never a first component printed from a multi-value element |
| Top level zero length | Any frame | Treated as absent, the groups decide (AC14) | Consistent with `corner_text`'s `Empty` to absent mapping |
| Per-frame element zero length, shared value planted | Any frame | The field is empty and the shared value is not read, because one `resolve` call answers and never falls to shared (AC2) | Never the shared value behind a present but empty per-frame element |

Invariant: the corner values shown belong to the frame on screen. The five
fields are resolved per frame request, never at scan time and never for
frame 0 on behalf of other frames.

## Ownership, commit and recovery

Not applicable in the render-loop sense. The read runs once per frame
request in `frame_facts`, called from `prepare_held` at `preview.rs` line
1079 for the grey reply and from `InspectParts::frame_fields` at line 3175
for the per-frame inspect reply, the same place the spacing and edge facts
are computed today, **and once per frame for every frame of a kept file
at post time**, because `kept_replies` at `preview.rs` line 3113 loops
`parts.frame_fields(frame)` for every frame at lines 3119 to 3121. It
allocates at most six short tokens per frame request and is not in the
render loop. There is no cached state to invalidate: the grey reply is
computed from the posted or held bytes, and the page's grey cache is
keyed by file serial, frame, window and invert at `cacheKey`, page.html
line 2609, as today. A repeated identical request computes the same
tokens. Header output is deterministic for a given file and frame.

## Numbered acceptance inventory

| ID | Trigger and expected behavior | Test / observation | Wrong implementation rejected | Evidence stage / owner |
|----|-------------------------------|---------------------|-------------------------------|------------------------|
| AC1 | A two-frame enhanced CT carries Slice Thickness only in the shared Pixel Measures Sequence. Each frame's grey reply carries `X-Ocelli-Slice-Thickness` with the stored DS token. A second file stores the shared Slice Thickness as the bytes 35 2E 30 00, `5.0` with one trailing NUL, and shows the trimmed `5.0`, and a third file stores the shared Slice Thickness as the bytes 31 32 00 00, `12` with two trailing NULs, and shows `12`, so a group trim of one NUL only fails | New test in `examples/folder-viewer/tests/preview.rs` on the `enhanced_frames` pattern, asserting the header through the existing `one_exchange` and `header_line` helpers | A reader that stays top-level only, so the header is missing, or a group text read that keeps the NUL and refuses a padded value the file carries | implementation |
| AC2 | The same file with the attribute in the frame's own group and a different value in the shared group shows the per-frame value. A per-frame Slice Thickness element present but zero-length, with a shared value planted, shows no header and never the shared value, because one `resolve` call returns the per-frame element and never falls to shared | Same suite, per-frame Pixel Measures with two different values, plus the zero-length plant | The shared value winning over the per-frame one, or a zero-length per-frame element treated as absent and the shared value read behind it | implementation |
| AC3 | Top-level first, per field, with the top-level tag and the group tag stated separately. Three rows. A classic Slice Thickness at the top level with the group form present shows the top-level value. A stray top-level Echo Time (0018,0081) 15 with per-frame Effective Echo Time (0018,9082) 30 shows TE 15, the story row's rule as the coordinator settled it on 2026-11-11. A top-level Exposure 400 with group Exposure in mAs 5.5 shows 400 with source `Exposure` | Same suite, three planted files | Reading the group's attribute tag at the top level, which misses the stray Echo Time and shows 30, or any group value overriding a present top-level attribute | implementation |
| AC4 | FD values format by one rule, hand-computed from PS3.5 6.2 and citing PS3.3 2026c C.8.15.3.8 Table C.8-124 and C.8.13.5.4 Table C.8-91. Rows, bytes and expected tokens: 5.5 `4016000000000000` to `5.5`, 3.0 `4008000000000000` to `3`, 12.25 `4028800000000000` to `12.25`, 0.1 `3fb999999999999a` to `0.1`, 0.123456789 `3fbf9add3739635f` to `0.123456789`, 1e-7 `3e7ad7f29abcaf48` to `0.0000001`, 1e21 `444b1ae4d6e2ef50` to `1000000000000000000000`, negative zero `8000000000000000` to `-0`, +inf `7ff0000000000000`, -inf `fff0000000000000` and NaN `7ff8000000000000` to no token. The page reprints through `Number`, so `0.0000001` shows `1e-7`, the 1e21 token shows `1e+21` and `-0` shows `0`, the DS path's own reprint rule | The `corners.rs` unit fixture, a `#[cfg(test)]` module that calls `fd_text` directly, plus one planted grey reply per FD attribute | A formatter with a fixed six-significant-digit precision, which 0.123456789 rejects, an exponent writer, which 1e-7 and 1e21 reject, a value trimmed or rounded, a non-finite printed into a line with a unit, or a Rust-side normalization of `-0` to `0` | implementation, reviewer |
| AC5 | A field absent from the top level and both group levels leaves its corner line out. In a second file the attribute is present in frame 0's per-frame group and absent from frame 1's, so frame 1's reply carries no header | Same suite, both files | A guessed zero, or a value carried from frame 0 | implementation |
| AC6 | The mAs chain, per the precedence rule. Five rows. A classic Exposure 200 shows 200 with source `Exposure`, and Exposure in µAs 200000 shows 200 with source `ExposureInuAs`, both as today and each asserted on the grey reply too: Exposure 200 carries `X-Ocelli-Exposure-Mas` 200 with `X-Ocelli-Exposure-Source` `Exposure`, and Exposure in µAs 200000 carries 200 with source `ExposureInuAs`, so a projection that drops or swaps the uAs link fails. An enhanced CT with group Exposure in mAs 5.5 and no usable top-level exposure shows 5.5 with source `ExposureInmAs`, with Exposure Time (0018,1150) and X-Ray Tube Current (0018,1151) planted and not multiplied. A top-level Exposure `abc`, present and unusable, with group Exposure in mAs 5.5, falls through and shows 5.5 with source `ExposureInmAs`. A top-level Exposure `200\300` with no other source shows nothing, the one-component rule, as `integer_string` at `corners.rs` line 304 refuses the joined text the scan path passes | Same suite, the two classic rows through the existing scan tests plus one planted grey reply per link, the three enhanced rows planted, and the scan JSON's `exposureSource` unchanged for classic files and empty for the enhanced one | Recomputing mAs from tube current and exposure time, stopping the chain at an unusable top-level value, dropping or swapping the uAs link, printing a first component from a multi-value Exposure, or inventing a tag | implementation |
| AC7 | Classic files are unchanged. `inspect_json_reports_the_corner_fields` and `a_corner_attribute_that_is_not_text_does_not_refuse_the_scan` pass byte-identically, and the corner text for a classic file prints the same values from facts as from info. Five plants. A stored `-12.50` Slice Thickness prints `-12.5`. A classic top-level KVP written as FD 120.0 stays blank, no header and no kVp line, because a non-text top-level value yields no token as the scan path does today. A classic Slice Thickness stored as the bytes 31 32 00 00, `12` with two trailing NULs, prints 12 from facts and from info, every trailing NUL trimmed and not one. A top-level Slice Thickness stored `5\` shows nothing, its two projected components refusing the one-component rule as the scan path's joined `to_str` refuses `clean_decimal`. A top-level Slice Thickness stored `5\6` with shared Pixel Measures 3.0 planted shows nothing, the stop rule with a group value behind it | The existing tests at `tests/preview.rs` lines 5657 and 5758, unmodified, plus the AC11 browser check and the four new planted suite rows | A shared path perturbed, a header parsed into text that prints `-12.50` where the scan prints `-12.5`, a wrongly encoded top-level KVP printed as `120 kVp`, a one-NUL-only trim that leaves facts empty where info shows 12, a top-level read that filters empty parts and prints `5` from `5\` where the scan path refuses it, or a read that lets a present multi-value element fall to the shared group and prints 3.0 | implementation |
| AC8 | Two frames of one enhanced file whose per-frame Slice Thickness differs show each frame's own value as it is shown, and the read resolves the requested frame, never frame 0 for all | Same suite, two frames, two different planted values, both frames requested | Resolving frame 0 for every request, the stale-read class | implementation |
| AC9 | Negative controls. Swapping the per-frame and shared reads turns AC2 red. Dropping the top-level-first branch turns AC3 red. Breaking `fd_text` turns AC4 red. Pinning frame 0 turns AC8 red. Removing the VR check turns AC12's six OD and LO rows red, because they then print a token, while its four FD-for-DS and DS-for-FD rows stay empty under a dispatch on the value variant and are guarded by AC4 and AC1 instead. Treating a non-text top-level value as absent turns AC16's first two rows red. Reading a top-level non-text value through the value variant instead of the text rule turns AC7's FD-encoded KVP row red, printing 120 kVp where the row expects blank. Filtering empty parts of a multi-value top-level element turns AC7's `5\` row red, printing `5` where the row expects nothing. Letting a present multi-value top-level element fall to the groups turns AC7's `5\6` row red, printing the shared 3.0. Mapping a present top-level zero-length or whitespace-only element to a stop instead of absent turns AC14 red, printing no header where the row expects the group value. Taking the first component of a multi-value group element turns AC15 red, printing 5 or 5.5 where the rows expect nothing | Recorded in the review note with the mutation and the red run | Tests that pass on wrong code | implementation, reviewer |
| AC10 | A malformed or multi-item corner group leaves that field empty and the scan and grey succeed. Four plants. A `CT Exposure Sequence` with two items and Multi-energy CT Acquisition absent, which is malformed under PS3.3 2026c C.8.15.3.8. A `CT X-Ray Details Sequence` with two items under the same condition. A conformant Multi-energy CT Acquisition file whose two sequences legitimately carry several items, empty by design under the one-item rule. A malformed `MR Echo Sequence` with two items in the shared item, which empties every frame's TE because `resolve` errors before precedence and nothing but the corner reads that group. A malformed Pixel Measures Sequence is not a plant here: `spacing_pair` already refuses the frame for it, the exception stated in the refusal policy | Same suite, planted, asserting the reply status and the missing header for each | Propagating `MultiframeError` as a refusal of the scan or the frame | implementation |
| AC11 | The visible surface is the existing viewer front door. With the viewer the browser checks start, the corner lines show the shown frame's own values and change when the frame changes. With the next image's preview reply held through the page's route, a repaint through the corner toggle or a resize paints the new image's `info` values, never the previous image's facts values, and releasing the reply paints the new image's own facts | A headless-Chromium check in `examples/folder-viewer/tests/browser/` in the pattern of `window.mjs` and the held-reply mechanism of `changed.mjs`, which starts the viewer through `tools/bench`'s `startViewer`, holds `**/preview` with `page.route`, and reads the corner element text, added to the `viewer-browser` gate's case arm | A Rust-only helper counted as end-user acceptance, or a stale facts object painting across an image switch | integrated browser |
| AC12 | Wrong VR gives no token, one plant per wrong VR, and the token read checks `element.vr()` against the field's expected VR (PS3.6 2026c Table 6-1) before any value read. Ten plants. Group Slice Thickness, DS expected, written OD. Group Slice Thickness written LO. Group KVP, DS expected, written FD. Group KVP written LO. Group Exposure in mAs, FD expected, written DS. Group Exposure in mAs written OD. Group Repetition Time, DS expected, written FD. Group Repetition Time written LO. Group Effective Echo Time, FD expected, written DS. Group Effective Echo Time written OD. Each leaves the field empty, because an OD element projects to `MetadataValue::Float64` exactly as FD does and a text VR other than DS projects to `Text` exactly as DS does, so the value variant alone cannot tell the VR | Same suite, ten planted files, asserting the missing header with the other sources absent | A per-field variant dispatch with no `element.vr()` guard, which passes a five-row inventory of the original kind, because each planted wrong VR then yields a variant the field's branch ignores, and which still prints Exposure in mAs from an OD element, a Slice Thickness from an LO element, and kVp or TR from an LO element | implementation |
| AC13 | The reviewer checks every sequence, attribute, VR, VM and type in this plan against the 2026c edition and names the edition | Review note listing each row and the section read | Fixtures citing unchecked tags, or a plan citing an edition that does not exist | reviewer |
| AC14 | A top-level zero-length, whitespace-only or all-NUL element falls through to the groups. Five plants, each with a group value behind it. A top-level Slice Thickness, Type 2 at its classic home PS3.3 2026c C.7.6.2 Table C.7-10, present but zero-length, with shared Pixel Measures 5.0, shows 5.0. A top-level Slice Thickness of spaces only with the shared value 3.0 shows 3.0. A top-level KVP present but zero-length with group KVP 120 shows 120. A top-level Echo Time present but zero-length with per-frame Effective Echo Time 30 shows 30. A top-level Slice Thickness stored as the bytes 00 00 with shared Pixel Measures 5.0 is empty after the trailing-NUL trim, so it is absent and shows 5.0, and a read that checks for empty before the trim stops instead and goes red | Same suite, five planted files, asserting the grey header | An implementation that maps a present top-level element with no token to a stop prints no header on all five and passes every other row, because AC2's zero-length plant is inside a group and no other row pairs a present but empty top-level element with a group value | implementation |
| AC15 | A multi-value inside a group yields no token. Two plants. Group Slice Thickness stored as the DS `5\6` in the shared Pixel Measures Sequence leaves Thickness empty. Group Exposure in mAs stored as 16 bytes, two little-endian f64 values 5.5 and 6.5, in the per-frame CT Exposure Sequence leaves mAs empty | Same suite, two planted files, asserting the missing header with no other source present | A group read that keeps the first component of a multi-value element prints 5 from the DS and 5.5 from the FD | implementation |
| AC16 | A present non-text top-level element stops, and the mAs chain falls through it. Three plants, each with a group value behind it. A top-level KVP written FD 120.0 with group CT X-Ray Details KVP 100 shows no kVp header and no kVp line. A top-level Slice Thickness written FD 4.0 with shared Pixel Measures 5.0 shows no Thickness header. A top-level Exposure written as a non-text value with group Exposure in mAs 5.5 shows 5.5 with source `ExposureInmAs`, the mAs chain's fall-through on unusable | New tests in `examples/folder-viewer/tests/preview.rs` on the `enhanced_frames` pattern, asserting the headers through `one_exchange` and `header_line` | A read written as `let Some(parts) = element.semantic_text() else { fall to the groups }`, which treats any non-text top-level value as absent and prints 100 kVp and 5.0, or an mAs chain that stops on the non-text Exposure and shows no mAs | implementation |

Unrun ownership. AC11 needs the built viewer binary and the headless
browser host the suite already uses. If that host is unavailable the row
stays open with its owner named and is never passed. No test has run for
this filing.

## Approach

1. Add `fd_text` to `corners.rs` and re-export it. Fixture first, from the
   hand-computed rows in AC4, watched red before the function exists.
2. Append the six fields to `FrameFacts`, compute them in `frame_facts`
   through a private `corner_token` beside `spacing_pair`, applying the
   precedence rule once: the top level from the `MetadataSet` projection
   under the one-component rule stated there, then
   `MultiframeMetadata::resolve` with `TopLevelFallback::Disallowed`,
   reusing `clean_decimal` and `fd_text`. `FrameFacts::empty` gains the
   six empty values.
3. Thread the six tokens through `from_preview`, `Grey` and the header
   writer in `serve.rs`.
4. Parse the five value headers in `requestGrey`'s facts object, stamp the
   serial and frame, and read them in `paintOverlays` under the identity
   match before the `info` fallback, on the thickness, tube and timing
   lines only. The exposure-source header is not parsed.
5. Tests: the `enhanced_frames`-style builders extended with the five
   groups, the `corner_object`-style planted builder for top-level cases,
   and the browser check for AC11.

Rejected alternatives. Carrying the five fields as per-frame arrays in the
scan JSON, like `planes`, grows the scan payload by five tokens per frame
up to `FRAME_LIST_CAP` and changes the JSON shape F-212 pinned. Extending
`refreshFrameWindows` to fetch corners needs a new ask and a new info flag
for files without per-frame windows, and the reply arrives after paint,
the comparison made under Consumer and provider map. Putting the read in
`ocelli-dicom` adds a one-caller helper to a library crate whose public
API this story does not otherwise need, against the structural rule
against wrappers with one caller. Reading every group eagerly at parse
time duplicates `resolve`. Inferring Slice Location from plane position is
F-212's declared non-derivation.

## Dependencies, write ownership and resources

Dependencies: F-212 and F-019, both done in the backlog at lines 358 and
139, checked with `grep -n "F-212\|F-019" docs/sprints/BACKLOG.md`. S47
holds no ordering constraint against either.

Write set: `examples/folder-viewer/src/preview.rs`, `src/corners.rs`,
`src/lib.rs`, `src/serve.rs`, `src/page.html`, `tests/preview.rs`, a new
`tests/browser/` check, the `viewer-browser` case arm and its description
string in `bin/ocelli.sh`, at the line numbers stated under Verification,
and `README.md`. No crate manifest
changes, no new dependency, no shader, no `ocelli-dicom` change.

Files that cannot have concurrent writers. F-245's approved plan at
`.claude/plans/F-245-design.md` line 278 writes `page.html`, `serve.rs`,
`lib.rs`, `csp.rs`, `bin/ocelli.sh`, `README.md`, `use.html`, `more.html`
and a new browser check, so this story overlaps it on `page.html`,
`serve.rs`, `lib.rs`, the `viewer-browser` case arm and its description
line in `bin/ocelli.sh` and `README.md`. F-245 does not list
`preview.rs` or `corners.rs`. F-202's
plan is a draft with no settled write list, and the story's allocation
note makes F-245 and F-202 consumers of this corner contract, never a
second viewer page. The overlap is region-disjoint in `serve.rs`, where
F-245 adds a configuration route and policy and this story extends the
grey header block, and in `page.html`, where F-245 reworks the chooser and
fetch paths and this story adds the facts parse and the acquisition lines
in `paintOverlays`. The two stories claim no concurrent writer on the same
file and land through the usual review order, whichever is first, and the
later one rebases. The browser check starts the viewer binary the gate
uses, whose page is compiled in at `serve.rs` line 40, and leaves no
server running, per the no long-lived servers rule.

## Verification and evidence reuse

Focused checks while iterating: `bin/ocelli.sh check ocelli-folder-viewer`,
`bin/ocelli.sh test ocelli-folder-viewer` and
`bin/ocelli.sh clippy ocelli-folder-viewer`. The crate name is the
manifest's at `examples/folder-viewer/Cargo.toml` line 2, checked this
session. The browser row runs under the existing `viewer-browser` gate,
whose executable script list is the case arm at `bin/ocelli.sh` lines 206
to 221, where the new check's node line is added, and whose description
string at line 70 lists the modules the arm runs, so it gains the new
module's name and F-ID in the same change, and whose checks start the
viewer through `tools/bench/src/viewer_work.mjs` `startViewer` at line
370. Those `bin/ocelli.sh` numbers are as of the main checkout's
sprint/s47 85ea89c4, measured there, where F-268's docs gate has already
landed, and are re-measured at the implementation's own base. The final
required profile is the feature profile at the integration
tree.

Evidence invalidates on any later change to the corner payload shape, the
six header names, `FrameFacts`, the precedence rule, the top-level
projection rule, `fd_text`, the facts identity tag, or the
`paintOverlays` fallback order. The FD fixture's
expected values are typed from this plan, not read from the
implementation, per HLD 27.2 R2. No performance claim is made, so no
threshold is set. The scan JSON's byte-identity for classic files is
asserted by the unmodified existing tests, which is the baseline policy.

## Boundary and tier

- wasm-bindgen: not touched
- Pixels across the boundary: no
- Render-loop allocation: none. The read is per frame-open, beside the existing spacing and edge facts
- unsafe: none
- Tier A (WebGPU): n/a. Metadata formatting, no rendering or compute change
- Tier B (WebGL2): n/a
- Tier C (CPU): n/a

All three tier rows are not applicable. The story touches no rendering or
compute path. The corner values appear identically on every tier because
they are made by the local process before any GPU work, the same place the
spacing and edge letters are made today.

## Tests

| Category | What it proves | Where |
|----------|----------------|-------|
| fixture | The one FD rule against hand-computed IEEE 754 rows citing PS3.3 2026c C.8.15.3.8 Table C.8-124, C.8.13.5.4 Table C.8-91 and PS3.5 6.2 FD | `corners.rs` under `#[cfg(test)]`, calling `fd_text` directly, the lib.rs re-export being for the crate's public surface as for the other corners helpers, not for this fixture |
| unit | The precedence chain, the VM 1 rules, the VR checks, the unusable, zero-length, fall-through and group multi-value cases, and the malformed-group cases | `examples/folder-viewer/tests/preview.rs` |
| integration | The grey reply headers per frame on synthetic enhanced CT and MR files, the wrong-VR rows, and the classic files' unchanged payloads | Same suite, plus the existing tests unchanged |
| browser | The corner lines on the page the built viewer serves, showing the shown frame's own values, changing on a frame step, and never painting a held previous image's facts | `examples/folder-viewer/tests/browser/`, new file, registered in the `viewer-browser` case arm |

The fixture is a story requirement in the row itself, and it follows HLD
27.2 R3 in form even though formatting is not pixel arithmetic. The
per-frame DS tokens are proved by the same integration rows through the
stored-text tokens. In the existing test at `tests/preview.rs` line 5676
the stored `-12.50` is Slice Location, not one of the five fields. The
stored Slice Thickness there is `5.0`. The `-12.50` Slice Thickness plant
of AC7 is a new plant in the browser check.

## Parity surface covered

None directly. `docs/hld/B-parity-surface.md` is a coarse table of viewport
types, tool classes, blend modes, VOI functions, transfer syntaxes,
segmentation representations, core events and adapters, and it names no
corner overlay row and no enhanced-metadata row. cornerstone3D 5.8.2 has no
four-corner text, as F-212's plan already recorded. This story extends
F-212's viewer convention to enhanced files and establishes no parity row.

## Deviations

None. The story needs no new `D-NN` row. The tier answer is n/a under
D-07's own rule, and the reader keeps its existing refusal behaviour.

## LLD impact

None. The local example is not the published API, the same answer F-212's
plan recorded. The FD read uses `ocelli-dicom`'s existing public API and
adds nothing to `docs/lld/`. If the review lands a change inside
`ocelli-dicom` after all, `docs/lld/dicom-ingest.md` is the file to
update.

## User-facing documentation impact

- `README.md`, "Start the local series viewer", step 8, the corner
  sentence at lines 419 to 424: add that on an enhanced multiframe CT or
  MR the thickness, kVp, mAs, TR and TE lines show the shown frame's own
  values from its functional groups, with mAs from Exposure in mAs, and TE
  on enhanced MR from Effective Echo Time, and that a field the file does
  not carry leaves its line out.
- `examples/folder-viewer/src/use.html`: no change. The corner provider is
  in the example and adds no library export.
- `examples/folder-viewer/src/more.html`: the S47 sprint card at line 214
  already names enhanced image corners, so only the step 8 sentence in the
  README needs the behaviour, and no card needs a new row. Verified with
  `grep -n "corner" examples/folder-viewer/src/more.html`.

## Open questions

None. The S46-close draft's three questions were settled in round 2 and
are transcribed above: the exact sequences and attributes per field, from
PS3.3 2026c, feeding AC1 to AC4, AC6, AC10, AC12 to AC16. The
reconciliation of the top-level rule with the per-frame rule inside the
groups is the precedence rule, because the standard prohibits a macro at
both group levels and the classic top-level versus group precedence is
the story row's own policy. The FD read path lives in the folder
viewer's preview provider beside `spacing_pair`, reusing the public
`MultiframeMetadata::resolve` and `MetadataValue::Float64`, with the FD
formatting rule as a public `corners.rs` function and no library crate
change. The one question round 2 carried, a stray top-level Echo Time on
a multi-echo enhanced MR, was settled by the coordinator on 2026-11-11
from the story row: the top-level attribute first where both exist, kept
uniformly, recorded in the precedence rule and asserted by AC3's Echo
Time row. No open question remains.

## Lead design approval, 2026-11-11

Approved after four GLM design rounds and the coordinator amendment, on a final independent check with no defect and no smell. The operator's S47 design round settled nothing in this plan, and the multi-echo Echo Time rule is decided by the story row.
