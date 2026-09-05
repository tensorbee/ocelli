# F-X007, Oracle volume and MPR reference renders, so the spacing rows are asked something

**Status**: approved
**Epic ref**: Y1.2
**Sprint**: S03
**Estimate**: 3w

## Normative source, transcribed

_Transcriptions below are verbatim except for one normalisation, the same one
F-010's plan declared: a prose semicolon in the source is written as a comma
and an em-dash as a hyphen, because `scripts/prose_check.py` covers
`.claude/plans/` and `docs/hld/` is exempt. No word is changed. Fenced code
blocks are exempt from that checker and are transcribed character for
character. Where the exact bytes matter, the tracked Markdown under
`docs/hld/` wins._

### `docs/hld/16-volume-representation.md`, section 19, in full

This is the whole of section 19. It is one code listing and three bullets, and
the fact that it is this short is load-bearing for the section below on what
the specification does not cover.

```rust
pub struct Volume {
    pub dims: [u32; 3],
    pub spacing: [f64; 3], // mm
    pub origin: Pt<World>, // IPP of the first slice
    pub direction: glam::DMat3, // derived from ImageOrientationPatient
    pub voxel: VoxelKind, // U8 | I16 | U16 | F32
    pub data: VoxelStore, // one contiguous allocation, x fastest
    pub levels: Vec<Level>, // multiscale; level 0 is full resolution
    pub present: bitvec::BitVec, // per-slice, for progressive assembly
}
```

> - **Layout.** One contiguous allocation, x fastest then y then z. Slice n
>   starts at n \* dims\[0\] \* dims\[1\] \* bytes_per_voxel. Do not introduce a
>   per-slice Vec, the whole point is a single upload region.
>
> - **Progressive assembly.** Clear the 3D texture at creation and upload
>   slices as they land. present drives the loading indicator and tells the
>   oracle which frames are comparable yet.
>
> - **Bricks and a level axis, from the start.** The volume carries a
>   multiscale level axis and 128³ brick decomposition even when a series fits
>   comfortably. Phase 1 uploads every brick, §30 makes residency selective.
>   Retrofitting a level axis into a volume model that never had one is the
>   rewrite cornerstone3D cannot afford.

Two clauses in it bear directly on this story. `origin` is specified as **the
IPP of the first slice**, not a derived or averaged point. And **present tells
the oracle which frames are comparable yet**, which is section 19 naming the
oracle as a consumer of volume assembly state. That is the only sentence in
the HLD that connects volumes to the oracle, and it is the hook this story
hangs on.

### `docs/hld/25-first-ten-files.md`, section 28, the framing sentence and entry 4, verbatim

> In this order. The goal of the first two weeks is to diff one windowed 2D
> image against cornerstone3D - everything below serves that.

> | 4 | tools/oracle/ | The differential harness. Nothing else should start
> before this works |

This is what justified F-010 rendering stack viewports only. The clause that
does the work is "the goal of the **first two weeks**". It is a statement of
ordering, not a statement of scope, and S02 is over.

### `docs/hld/08-validation-architecture.md`, section 11, in full

> Cornerstone3D is a correct reference implementation that can render any
> series you own. The harness pushes the same study through both stacks and
> compares frames within a written per-modality tolerance, with metadata
> diffed alongside pixels because a wrong rescale slope can still produce a
> plausible image.
>
> Every pull request renders the corpus in CI. Every field bug becomes a
> permanent fixture. In production, shadow mode renders both libraries and
> alerts on divergence - the oracle running against real clinical traffic, and
> the same corpus a regulatory submission would want to see.

**"Cornerstone3D is a correct reference implementation" is the sentence this
story tests and, for one specific thing, falsifies.** See the measurement
below. Section 11 also says the harness pushes "the same **study**" through
both stacks, which is a series-level noun, not an instance-level one, so the
volume pass is closer to what section 11 describes than the stack pass is.

### `docs/hld/13-core-types.md`, section 16, verbatim

```rust
// ocelli-core/src/space.rs
use core::marker::PhantomData;
/// CSS pixels inside a viewport element. Origin top-left, y increases down.
pub enum Canvas {}
/// DICOM patient coordinate system (LPS), millimetres.
pub enum World {}
/// Voxel indices within a volume. Origin at voxel (0,0,0).
pub enum Index {}
#[derive(Debug, PartialEq)]
pub struct Pt<S> { pub x: f64, pub y: f64, pub z: f64, _s: PhantomData<S> }
// NOTE: derive(Clone, Copy) would add an S: Clone bound that the marker
// types do not satisfy. Implement by hand.
impl<S> Clone for Pt<S> { fn clone(&self) -> Self { *self } }
impl<S> Copy for Pt<S> {}
impl<S> Pt<S> {
    pub const fn new(x: f64, y: f64, z: f64) -> Self {
        Self { x, y, z, _s: PhantomData }
    }
}
pub struct Transform<A, B> { m: glam::DMat4, _p: PhantomData<(A, B)> }
impl<A, B> Transform<A, B> {
    pub fn apply(&self, p: Pt<A>) -> Pt<B> { /* ... */ }
    pub fn inverse(&self) -> Transform<B, A> { /* ... */ }
    pub fn then<C>(&self, next: &Transform<B, C>) -> Transform<A, C> { /* ... */ }
}
```

and the framing sentence, verbatim:

> Cornerstone represents canvas points, world points and voxel indices all as
> number\[\]. Mixing them is a silent, common and expensive bug.

A reformat is a geometry claim, and this is the section that says why the
claim needs a type. This story writes no Rust, so it instantiates none of
these, but the world coordinates it records in the sidecar are `World` in
section 16's sense, LPS millimetres, and the plan says so wherever it names a
number.

### `docs/hld/22-testing-and-tolerance.md`, section 25.1, verbatim

> Write it down once and hold it. Tuning tolerance per failure is how a suite
> stops meaning anything.
>
> - **Monochrome 16-bit (CT, MR, CR, DR):** maximum absolute difference <= 1
>   LSB on at least 99.9% of pixels, zero pixels differing by more than 2.
>
> - **Colour and ultrasound:** perceptual difference below a stated threshold,
>   because chroma subsampling and YBR conversion legitimately differ.
>
> - **Geometry:** world coordinates within 1e-6 mm, canvas coordinates within
>   a quarter pixel.
>
> - A tolerance change is a pull request with a rationale, reviewed like code.

**The geometry line is the one this story applies.** Every world-coordinate
comparison in the plan below is against 1e-6 mm and none other. This story
sets no new tolerance and changes none.

### `docs/hld/DEVIATIONS.md`, D-11, verbatim

> | D-11 | Appendix B, "Measured from cornerstone3D v5.8.9 source", and the
> parity target stated as v5.8.9 | The oracle pins `@cornerstonejs/core`,
> `@cornerstonejs/tools` and `@cornerstonejs/dicom-image-loader` at exactly
> `5.8.2` | **v5.8.9 does not exist.** [...] 5.8.2 is the highest published
> 5.8.x and therefore the nearest installable reference. | F-010 |

and the paragraph that governs how this story treats a reference measurement,
verbatim:

> **What is not claimed.** That 5.8.2 and v5.8.9 are the same software. Nobody
> can check that, because one of them cannot be obtained. What is claimed is
> narrower and checkable: 5.8.2 is the highest published version in the 5.8
> series, so it is the closest thing to the stated reference that this project
> can actually pin, install and hold still.

Every claim this story makes about cornerstone3D is a claim about 5.8.2 as
installed under `tools/oracle/node_modules`, with a file and a line number, and
about nothing else.

### `docs/lld/oracle.md`, "Stack viewports only, and what that leaves undone", verbatim

> HLD section 28 says the goal of the first two weeks is to diff one windowed
> 2D image, and F-011 is a pixel-diff comparator, so every reference frame here
> is a **stack** render of one frame of one instance. Volume and MPR reference
> renders are not produced.
>
> The consequence is worth naming rather than inferring from an output
> directory. The twenty synthetic spacing rows, ten under
> `synthetic/ct_series_uniform/` and ten under
> `synthetic/ct_series_nonuniform/`, are rendered here as twenty independent
> stacks. (Sixty-two manifest rows carry the `series` category token: these
> twenty, plus the twenty-seven of `real/ct_cmb_mml` and the fifteen of
> `real/mr_eay131`. The other two real directories are single instances and
> carry no such token. The twenty are the ones written to exercise spacing.)
> **The volume builder's refusal path that the non-uniform ten exist to
> exercise is asked nothing by this story.** Those rows are covered as pixels
> and untouched as geometry. Volume reference renders belong to the later E2
> stories.

### `docs/lld/oracle.md`, "The render parameters are declared, not defaulted", the governing sentence, verbatim

> **A change to that file changes every reference frame.** Treat it the way HLD
> 25.1 treats a tolerance: a pull request with a rationale, reviewed like code.

### `docs/sprints/allocation.json`, the F-X007 note, in full

> F-010 renders stack viewports only, which HLD section 28 justifies for the
> first two weeks. The consequence is recorded in docs/lld/oracle.md and is a
> real gap: the corpus carries ten uniform-spacing and ten non-uniform-spacing
> series rows, the non-uniform ten exist specifically to exercise the volume
> builder's REFUSAL path, and a stack-only oracle renders all twenty as
> independent stacks and asks them nothing. Until this lands, no reference
> output covers the geometry HLD section 16 warns about most, where using
> SpacingBetweenSlices instead of projected IPP compresses the volume along z
> and leaves every axial view looking perfect.

### `docs/sprints/CURRENT_SPRINT.md`, what done means and the ordering constraint, verbatim

> - **F-X007** reference renders volumes and MPR, so the ten non-uniform
>   spacing rows exercise the volume builder's refusal path.

> F-011 is the one with a real ordering constraint inside the sprint: F-X007
> adds reference renders that F-011 will then compare, so a comparator written
> before F-X007 lands must not assume stack-only input.

### `scripts/corpus_synth.py`, the two series by construction, verbatim

```python
# The oblique orientation the two CT series use. Both direction cosines are
# exactly unit length and their cross product is exactly unit length, so every
# slice position below is an exact decimal and a reviewer can check the
# arithmetic without a calculator.
#   row = (0.8, 0.6, 0), col = (0, 0, -1), normal = row x col = (-0.6, 0.8, 0)
SERIES_ORIENTATION = ["0.8", "0.6", "0.0", "0.0", "0.0", "-1.0"]
SERIES_NORMAL = (-0.6, 0.8, 0.0)
SERIES_SPACING = 2.5
SERIES_SLICES = 10

# Slice 7 is displaced by half a gap. The median gap stays 2.5, and the two
# gaps either side of it become 3.75 and 1.25. A volume builder that averages
# instead of refusing produces a plausible reformat that is wrong by a
# constant factor, which no measurement tool flags.
NONUNIFORM_SLICE = 7
NONUNIFORM_OFFSET = 1.25
```

and, from `case_series`:

```python
        ds.SliceThickness = f"{SERIES_SPACING}"
        # SpacingBetweenSlices is deliberately absent. It is frequently wrong
        # when present, and the ground truth is the projected IPP difference.
```

and, from `ct_common`, which both series call:

```python
    ds.PixelSpacing = list(NON_SQUARE_SPACING)
```

where `NON_SQUARE_SPACING = ["0.5", "0.25"]`, declared with:

```python
# Deliberately non-square everywhere except where a case says otherwise. A
# square-pixel fixture cannot catch a transposed PixelSpacing index, and
# PS3.3 C.7.6.2.1.1 makes PixelSpacing[0] the spacing BETWEEN ROWS.
```

## What cornerstone3D 5.8.2 actually does with a non-uniform-spacing series

This is the hard question the story turns on, and it is answered by reading
5.8.2's own source under `tools/oracle/node_modules/@cornerstonejs/core/`,
which is MIT and in bounds. **The answer is that it averages, silently, with
no tolerance, no warning and no error.** Every quotation below is verbatim
from the installed tree with its absolute path and line numbers.

### It takes the mean gap

`/Users/atulsharma/Documents/projects/tensorbee/ocelli/tools/oracle/node_modules/@cornerstonejs/core/dist/esm/utilities/calculateSpacingBetweenImageIds.js`,
lines 53 to 67:

```js
    if (!usingWadoUri) {
        const distanceImagePairs = imageIds.map((imageId) => {
            const distance = getDistance(imageId);
            return {
                distance,
                imageId,
            };
        });
        distanceImagePairs.sort((a, b) => b.distance - a.distance);
        const numImages = distanceImagePairs.length;
        spacing =
            Math.abs(distanceImagePairs[numImages - 1].distance -
                distanceImagePairs[0].distance) /
                (numImages - 1);
    }
```

That is `|d_last - d_first| / (N - 1)`. The intermediate distances telescope
out, so it is the arithmetic mean of the N-1 gaps. It is not the first gap,
not the median, not the minimum, and **no individual gap is ever compared with
any other**. There is nothing in the function to apply a tolerance to.

### The distance is the projected IPP, which is the one thing it gets right

Same file, lines 39 to 52:

```js
    const rowCosineVec = vec3.fromValues(imageOrientationPatient[0], imageOrientationPatient[1], imageOrientationPatient[2]);
    const colCosineVec = vec3.fromValues(imageOrientationPatient[3], imageOrientationPatient[4], imageOrientationPatient[5]);
    const scanAxisNormal = vec3.create();
    vec3.cross(scanAxisNormal, rowCosineVec, colCosineVec);
    const refIppVec = vec3.fromValues(referenceImagePositionPatient[0], referenceImagePositionPatient[1], referenceImagePositionPatient[2]);
    const usingWadoUri = imageIds[0].split(':')[0] === 'wadouri';
    let spacing;
    function getDistance(imageId) {
        const { imagePositionPatient } = metaData.get(MetadataModules.IMAGE_PLANE, imageId);
        const positionVector = vec3.create();
        const ippVec = vec3.fromValues(imagePositionPatient[0], imagePositionPatient[1], imagePositionPatient[2]);
        vec3.sub(positionVector, refIppVec, ippVec);
        return vec3.dot(positionVector, scanAxisNormal);
    }
```

So the ordering and the extent do come from projected `ImagePositionPatient`
onto the cross product of the two direction cosines, exactly as PS3.3
C.7.6.2.1.1 implies and as the allocation note wants. `SliceThickness` and
`SpacingBetweenSlices` are read only on two dead-end branches, the
single-image case at lines 23 to 38 and the degenerate fallback at lines 85 to
110. **So the allocation note's stated failure mode, "using SpacingBetweenSlices
instead of projected IPP", is not the reference's failure mode.** The
reference's failure mode is different and worse for our purposes: it uses the
projected IPP and then throws away everything except the endpoints.

### There is no tolerance, no warning and no error

Grepping the whole of `core/dist/esm` for a spacing-related throw finds two,
and neither is about uniformity:

```
utilities/calculateSpacingBetweenImageIds.js:75:  throw new Error('Incomplete metadata required for volume construction.');
loaders/imageLoader.js:186:  throw new Error('createAndCacheLocalImage: dimensions and spacing are required');
```

The only spacing-related console output is three `console.debug` calls at
lines 89, 93 and 106 of the same file, and lines 86 and 87 gate all three:

```js
    const { strictZSpacingForVolumeViewport } = getConfiguration().rendering;
    if ((spacing === 0 || isNaN(spacing)) && !strictZSpacingForVolumeViewport) {
```

`strictZSpacingForVolumeViewport` defaults to `true`
(`core/dist/esm/init.js:16`), so on a default init even that branch is dead,
and a zero or NaN spacing is returned unremarked.

`isValidVolume`, the only "is this series a volume" gate in 5.8.2, checks
`seriesInstanceUID`, `modality`, `columns`, `rows`, `imageOrientationPatient`
and `pixelSpacing` per image
(`core/dist/esm/utilities/isValidVolume.js:32-66`) and reads
`imagePositionPatient` nowhere in the file. A series with arbitrary spacing
returns `validVolume === true`.

### The averaged number becomes a uniform grid, by relabelling

`core/dist/esm/utilities/generateVolumePropsFromImageIds.js`, lines 21 to 24:

```js
    const { zSpacing, origin, sortedImageIds } = sortImageIdsAndGetSpacing(imageIds, scanAxisNormal);
    const numFrames = imageIds.length;
    const spacing = [PixelSpacing[1], PixelSpacing[0], zSpacing];
    const dimensions = [Columns, Rows, numFrames].map((it) => Math.floor(it));
```

Line 23 is worth noticing on its own: `PixelSpacing[1]` on the x axis and
`PixelSpacing[0]` on the y axis is PS3.3 C.7.6.2.1.1 applied correctly, since
`PixelSpacing[0]` is the between-rows spacing and rows advance along the
column direction cosine. **cornerstone3D 5.8.2 gets the in-plane index right
and the through-plane one wrong.** Slice k is then written into frame slot k by
its ordinal position in the sorted array
(`core/dist/esm/cache/classes/ImageVolume.js:107` and
`BaseStreamingImageVolume.js:75`), never by its physical position, so a
displaced slice is not interpolated into place, it is relabelled.

### What that means for our two corpus series, in numbers

The harness's image ids come from
`dicomImageLoader.wadouri.fileManager.add(file)`, and
`dicom-image-loader/dist/esm/imageLoader/wadouri/fileManager.js:2-5` returns
`` `dicomfile:${fileIndex - 1}` ``. So `imageIds[0].split(':')[0]` is
`"dicomfile"`, not `"wadouri"`, and the harness takes the `!usingWadoUri`
branch quoted above rather than the coarser first-to-middle one at lines 68 to
84. That is a fact about our own driver and it is recorded here because a
future change of loading scheme would change which branch the reference takes.

Hand-computed from PS3.3 C.7.6.2.1.1 and the generator's constants. The
transcription above gives `row = (0.8, 0.6, 0)` and `col = (0, 0, -1)`, so

```
n = row x col
  = (0.6*(-1) - 0*0,  0*0 - 0.8*(-1),  0.8*0 - 0.6*0)
  = (-0.6, 0.8, 0.0)          |n| = sqrt(0.36 + 0.64) = 1 exactly
```

and each slice sits at `IPP_k = d_k * n`, so `IPP_k . n = d_k * |n|^2 = d_k`.

| k | d_k uniform (mm) | d_k non-uniform (mm) | gap into k, non-uniform |
|---|---|---|---|
| 0 | 0.00 | 0.00 | - |
| 1 | 2.50 | 2.50 | 2.50 |
| 2 | 5.00 | 5.00 | 2.50 |
| 3 | 7.50 | 7.50 | 2.50 |
| 4 | 10.00 | 10.00 | 2.50 |
| 5 | 12.50 | 12.50 | 2.50 |
| 6 | 15.00 | 15.00 | 2.50 |
| 7 | 17.50 | **18.75** | **3.75** |
| 8 | 20.00 | 20.00 | **1.25** |
| 9 | 22.50 | 22.50 | 2.50 |

`|d_9 - d_0| / 9` is `22.5 / 9 = 2.5` for **both** series. The offset is added
to one interior slice and cancels out of the endpoint difference, so:

- the reference's `zSpacing` is 2.5 for both,
- the mean gap is 2.5 for both, so an averaging builder is undetectable from
  the spacing value,
- the median gap is 2.5 for both, so a median builder is undetectable too,
- `SliceThickness` is 2.5 on every slice of both, so a tag reader gets 2.5 as
  well,
- **only an examination of the individual gaps separates the two series.**

The generator's own comment says a builder that averages is "wrong by a
constant factor". On this construction it is not a constant factor, it is one
slice placed 1.25 mm out of position, which is exactly half a slice gap. That
is worth correcting in the corpus LLD and is listed under LLD impact.

### The consequence, stated as the plan's central claim

`origin` is the IPP of the sorted first slice
(`sortImageIdsAndGetSpacing.js:45`), which is `[0, 0, 0]` for both series.
`direction` is the two cosines and their cross product, identical for both.
`dimensions` is `[20, 12, 10]` for both. `spacing` is `[0.25, 0.5, 2.5]` for
both. The two series carry byte-identical pixel content per slice index, since
`case_series` writes `trap_frame(probe=False) + index * 16` in both. The sort
produces the same order in both.

**Therefore cornerstone3D 5.8.2 is predicted to render the uniform and the
non-uniform series to bit-identical reformat frames.** The two series differ
only in slice 7's `ImagePositionPatient`, and a renderer that honoured that
difference could not produce the same pixels.

That is a falsifiable prediction, it is cheap to check, and this story checks
it rather than assuming it. If the run comes back with different digests, the
reasoning above is wrong somewhere and the plan is re-derived before the story
is claimed done.

**So the reference's behaviour is not a refusal, and the oracle cannot use it
to prove our refusal is right.** This is the same class as the recorded SIGMOID
divergence and it is handled the same way: recorded as a committed fact about
the reference, with the ground truth coming from PS3.3 and the generator's
construction rather than from the reference.

## What the specification does not cover

This section is long, because on this story the specification covers less than
the story's own brief implies.

1. **The HLD nowhere says slice spacing comes from projected
   `ImagePositionPatient`.** The allocation note attributes that warning to
   "HLD section 16", and section 16 is the coordinate and value spaces and says
   nothing about spacing. Section 19, the volume representation, is transcribed
   above in full: it declares `spacing: [f64; 3]` and says `origin` is "IPP of
   the first slice", and it says nothing about how spacing is derived, nothing
   about `SpacingBetweenSlices`, nothing about `SliceThickness`, and nothing
   about non-uniform spacing. A repository-wide grep of `docs/hld/` for
   `SpacingBetweenSlices`, `ImagePositionPatient`, `non-uniform` and `slice
   spacing` returns no hit outside `DEVIATIONS.md`. **The rule is real, it
   comes from PS3.3 C.7.6.2.1.1, and the HLD does not state it.** See open
   question 1.
2. **The HLD nowhere says a volume builder refuses a non-uniform series.**
   CURRENT_SPRINT and the allocation note both call it "the volume builder's
   refusal path", and the corpus was built to exercise it, but no HLD section
   states the policy, the threshold, or what "refuse" means to a caller. This
   plan therefore does not decide it. It records the measurement that lets it
   be decided. See open question 2.
3. **Nothing says which reformats a reference render should produce.** Three
   orthogonal orientations is this plan's choice, and section 19 does not make
   it.
4. **Nothing says what a volume reference frame is, as bytes or as an output
   name.** F-010 answered that for stacks and `docs/lld/oracle.md` records it.
   The volume equivalent is this plan's, and it is written down as a contract
   because F-011 is being built at the same time.
5. **Nothing states a uniformity threshold for real clinical data.** HLD 25.1's
   1e-6 mm is a comparison tolerance for geometry, which is the right tool for
   the synthetic series where the arithmetic is exact by construction. It is
   not a statement about how far a real scanner may drift before a series stops
   being a volume. This plan records the measured deviation for the real series
   and classifies nothing. See open question 3.
6. **Nothing says whether the reference's own answer may be recorded as
   correct.** D14 and the SIGMOID precedent say a divergence is attributed to a
   side. This plan follows that precedent, and the precedent is repository
   practice from S02 rather than HLD text.

## Approach

### 1. Nothing that F-010 produced moves

The strongest property this story can have is that the eighty-nine existing
reference frames are byte-identical afterwards, because F-011 is reading them
concurrently. Three things arrange it:

- **`tools/oracle/render-params.json` is not changed.** See the section on it
  below.
- **`tools/oracle/page/app.mjs` is not changed at all.** The stack render page
  is left alone. The `kind` discriminator F-011 needs is added in
  `src/sidecar.mjs`, which shapes the sidecar and never the frame.
- **The volume pass runs in its own page**, `page/volume.html` with its own
  bundle `page/dist/volume.js`, loaded after the stack passes have finished and
  the stack page has been closed. cornerstone3D 5.8.2 defaults to
  `renderingEngineMode: ContextPool` with `webGlContextCount: 7`
  (`core/dist/esm/init.js:17-18`), so a second viewport in the same engine
  might well have been harmless, and "might well have been" is not a property
  worth spending F-011's schedule on. A separate page makes the stack pass
  provably the same program it was.

**Acceptance check, run by the implementer and recorded in the completion
note.** Run `bin/ocelli.sh oracle` on the tree before the change and copy
`out/run.json` aside. Run it after. Assert every one of the eighty-nine
`rows[].sha256` values is unchanged. The output directory is not committed, so
this is a measurement the implementer takes and records, not a standing test.

### 2. A volume subject is declared, and its members are manifest rows

A new committed file, `tools/oracle/volume-params.json`, declares the volume
subjects and the parameters of the volume pass. Resolution follows
`render-params.json`'s shape so a reader holds one rule: `base`, then rules in
file order, later winning, a key replaced wholesale.

It declares only what is genuinely new. `canvas`, `background`,
`interpolation` and the VOI resolution are **read from
`render-params.json`'s `base`** so that there is one declaration of each and
not two that could drift. `src/volume.mjs` refuses a `volume-params.json` that
names any key `render-params.json` already owns, for the same reason
`src/params.mjs` refuses a rule that sets `canvas`.

Declared subjects, each naming a corpus directory and the ordered member paths:

| Subject id | Directory | Members | Truth |
|---|---|---|---|
| `volume__synthetic__ct_series_uniform` | `synthetic/ct_series_uniform/` | 10 | hand-computed |
| `volume__synthetic__ct_series_nonuniform` | `synthetic/ct_series_nonuniform/` | 10 | hand-computed |
| `volume__real__ct_cmb_mml` | `real/ct_cmb_mml/` | 27 | measured, not asserted |
| `volume__real__mr_eay131` | `real/mr_eay131/` | 15 | measured, not asserted |

Strict in both directions, in the manner `unsupported.json` already sets:

- every declared member must be a manifest row, and the run refuses a member
  that is not,
- every manifest row carrying the `series` category token must belong to
  exactly one declared subject, and the run refuses a `series` row that belongs
  to none, so the file cannot silently stop covering a directory somebody adds,
- a subject is attempted only when **every** member is in the selected rows. A
  partially selected subject is refused by name rather than skipped, so
  `--rows synthetic/ct_series_uniform/slice_000.dcm` says what it did rather
  than producing no volumes and reporting success.

The four real single-instance directories carry no `series` token and are
therefore not subjects, which the LLD already records.

### 3. What the volume pass does, per subject

1. Load all members into the page as `dicomfile:` image ids, the same way the
   stack page does, so the same decoders produce the same pixels.
2. `volumeLoader.createAndCacheVolume(volumeId, { imageIds })`. In 5.8.2 the
   streaming loader is the default `unknownVolumeLoader`
   (`core/dist/esm/loaders/volumeLoader.js:14`), so no extra package and no
   registration is needed. **No new pin is required and
   `tools/oracle/package.json`'s dependency set does not change.**
3. `volume.load()`, and wait on `Enums.Events.IMAGE_VOLUME_LOADING_COMPLETED`
   with `framesProcessed === totalNumFrames`
   (`core/dist/esm/cache/classes/BaseStreamingImageVolume.js:66-72`). Never on
   a sleep, and a timeout is a failure.
4. Record the reference's own geometry from the built volume: `dimensions`,
   `spacing`, `origin`, `direction`, `dataType`, and the sorted image id order
   it chose.
5. For each declared orientation, set the volume on an `ORTHOGRAPHIC` viewport
   (`core/dist/esm/enums/ViewportType.js:4`), `setOrientation` with a value
   from `Enums.OrientationAxis`
   (`core/dist/esm/enums/OrientationAxis.js:3-5`), `resetCamera`, paint the
   sentinel, `render()`, wait on `IMAGE_RENDERED`, read back, hash.
6. Purge the volume cache and the image cache between subjects and between
   determinism passes, so eviction order cannot differ.

Orientations declared: `AXIAL`, `SAGITTAL`, `CORONAL`. All three are worth
having on these subjects and the reason is specific rather than
completionist. `MPR_CAMERA_VALUES`
(`core/dist/esm/constants/mprCameraValues.js:2-18`) fixes those three planes in
patient LPS axes, and the two synthetic series are **oblique**, with normal
`(-0.6, 0.8, 0)` lying in the patient XY plane. So none of the three is the
acquisition plane and all three cut across the slice normal. The allocation
note's "leaves every axial view looking perfect" is a property of an
axis-aligned series, and the corpus's series are not axis-aligned, so all three
orientations are informative here. The page refuses any orientation it does not
implement rather than passing it through, exactly as it already refuses a
camera mode it does not implement.

### 4. The measured geometry is computed from the files, not from the reference

A new node module, `tools/oracle/src/geometry.mjs`, computes the subject's
geometry from the `attributes` blocks that `dicom-parser` already read
independently in the stack pass. It never reads a cornerstone3D module. This
is the harness's own second opinion, and it exists for the same reason the
sidecar already carries two readings of the same metadata: a harness that
transcribed the reference's own answer could not show the reference getting it
wrong, which is exactly the case here.

It computes, all in `World` millimetres in section 16's sense:

- `normal`, the cross product of the two direction cosines, PS3.3 C.7.6.2.1.1,
- `projections[]`, each member's `ImagePositionPatient` dotted with `normal`,
- `gaps[]`, the consecutive differences after sorting by projection,
- `meanGapMm`, `medianGapMm`, `minGapMm`, `maxGapMm`, `maxDeviationFromMeanMm`,
- `voxelAxes`, the three world-space steps of one voxel:
  `columnStep = PixelSpacing[1] * rowCosine`,
  `rowStep = PixelSpacing[0] * columnCosine`,
  `sliceStep = meanGapMm * normal`.

**`PixelSpacing[0]` multiplies the COLUMN direction cosine and
`PixelSpacing[1]` multiplies the ROW direction cosine.** PS3.3 C.7.6.2.1.1
gives `PixelSpacing` as `[between rows, between columns]`, and advancing one
row moves along the column direction cosine, which is the second IOP triplet.
Getting that index the other way round is the transposition the corpus's
non-square `[0.5, 0.25]` exists to catch, and it is a hand-computed fixture
below rather than a comment.

`uniform` is decided only where a truth entry declares it, against HLD 25.1's
1e-6 mm, and is `null` for a subject with no truth entry. The harness records
`maxDeviationFromMeanMm` unconditionally, for every subject, so the number a
later policy decision needs is in the output whether or not this story
classifies it.

### 5. `volume-truth.json`, the committed ground truth and the recorded divergence

A second new committed file, strict in both directions in the manner
`unsupported.json` sets. It carries, per subject that has one, a hand-computed
expectation citing PS3.3, and it carries the recorded fact of what 5.8.2 does.

```
{
  "cornerstone3DVersion": "5.8.2",
  "subjects": {
    "volume__synthetic__ct_series_nonuniform": {
      "citation": "PS3.3 C.7.6.2.1.1, and scripts/corpus_synth.py case_series",
      "normal": [-0.6, 0.8, 0.0],
      "projectionsMm": [0, 2.5, 5, 7.5, 10, 12.5, 15, 18.75, 20, 22.5],
      "gapsMm": [2.5, 2.5, 2.5, 2.5, 2.5, 2.5, 3.75, 1.25, 2.5],
      "uniform": false,
      "toleranceMm": 1e-6,
      "why": "...",
      "referenceDivergence": {
        "field": "spacing[2]",
        "reference": 2.5,
        "truth": null,
        "attributedTo": "reference",
        "why": "..."
      }
    }
  },
  "framePairs": [
    {
      "left": "volume__synthetic__ct_series_uniform",
      "right": "volume__synthetic__ct_series_nonuniform",
      "expect": "identical",
      "why": "..."
    }
  ]
}
```

The two strict directions:

- the harness's own `measuredGeometry` must match the declared truth within
  1e-6 mm, and a mismatch fails the run. That is what makes `geometry.mjs` a
  tested instrument rather than a source of numbers nobody checked,
- a declared `referenceDivergence` that does not occur fails the run, and a
  reference geometry that diverges from the truth with no declared entry fails
  the run. So the file cannot grow into a list of excuses and a stale claim
  cannot hide a reference that improved.

`framePairs` is the sharpest of the three. It asserts the prediction from the
measurement section: the three orientation digests of the uniform subject
equal the three of the non-uniform subject. If cornerstone3D ever stops
averaging, that assertion goes red and names the reason. Recording a
divergence as three equal digests is a stronger and cheaper claim than
recording it as a sentence.

### 6. A frame that shows nothing is refused, and a volume that shows nothing is too

The existing read-back guards carry over unchanged in intent and are
re-implemented on the volume page: the canvas is the declared size, the frame
is not one value, and the frame is not still the sentinel magenta. A reformat
plane that misses the volume comes back uniform background and is caught by
the second of those.

**A volume has a degeneracy the stack path has no analogue for**, and it needs
its own guard: a volume assembled from the wrong slices, or from one slice ten
times, renders a perfectly plausible frame that hashes stably. Two checks:

- `dimensions[2]` equals the member count, and the reference's sorted image id
  list is a permutation of exactly the members, with no repeats. This is the
  volume's version of the stack page's "the viewport is showing THIS row's
  image".
- **The z profile.** `case_series` writes `trap_frame(probe=False) + index * 16`,
  so the stored value at a fixed in-plane voxel is a ramp of exactly 16 per
  slice along the volume's z axis. The harness reads the built volume's scalar
  data at one declared in-plane voxel down the z axis and asserts the ten
  values differ by exactly 16 per step, in order. Hand-computed from the
  generator's constant and from PS3.3 C.7.6.3.1.4 for the 12-in-16 unsigned
  container. A volume that dropped, duplicated or reordered a slice fails it.
  This applies to the two synthetic subjects, and the truth file declares it,
  so the real subjects are not asked a question their content cannot answer.

### 7. Every new refusal has a fault, observed red

The catalogue in `tools/oracle/src/faults.mjs` gains six entries. Each breaks
one thing, and each `expect` is a fragment of that guard's own message rather
than the boundary name, following the existing entries.

| Fault | Boundary | What it breaks |
|---|---|---|
| `volume-short-load` | `volume-loaded` | one member is dropped, so `dimensions[2]` and the member count disagree |
| `volume-slice-shuffle` | `volume-loaded` | two members are swapped, so the z ramp is not monotonic by 16 |
| `bad-orientation` | `reformat-presented` | `volume-params.json` asks for an orientation the page does not implement |
| `no-reformat-render-event` | `reformat-presented` | the volume viewport never calls `render()` |
| `reformat-uniform-canvas` | `reformat-read-back` | the reformat frame is overwritten with one value after a real render |
| `volume-geometry-drift` | `volume-geometry` | a projected position is perturbed by 2e-6 mm, which is over 25.1's 1e-6, so the truth assertion goes red |

The last one is the important one and it is the same argument the LLD makes
for `stale-frame`. Without it the geometry assertion would be present,
authoritative-looking and never watched, and the tolerance it is written
against would be a number nobody had seen decide anything. The perturbation is
2e-6 specifically, so it also proves 1e-6 is the tolerance that acted and not
some looser default.

The existing twelve faults use `--rows <fault.row>` with a substring, and a
volume fault names the subject directory, so the runner needs no change to
select a whole series. **The cost is six more browser launches on every oracle
gate, taking the self test from twelve to eighteen.** The LLD already accepts
that trade and states it, and it is named here rather than discovered.

The sprint's standing expectation applies without exception: the mutation that
proves each of these red must not be run in the same command that adds it.

### 8. This story serialises the GPU, plainly

`bin/ocelli.sh` declares `oracle` a GPU gate and keeps it out of `--floor`,
which is D-04. F-X007 develops against that gate and cannot share it. F-011
reads its output and needs it. F-X006 answers A1 and A2 against our own
decoders and will want a browser too. **Only one of these can hold the oracle
gate at a time, and the sprint has to run them serially over it or accept
results nobody can attribute.**

Two precise notes, because "needs the GPU" is doing two different jobs here.
The oracle forces software rasterisation and asserts it, so what it actually
needs is a browser and CPU rather than a GPU. What is scarce is the gate slot
and the determinism claim, which is made over two passes in one browser on one
machine. Two harnesses running at once on one host contend for CPU, which does
not change a digest but does change a timeout, and `RENDER_TIMEOUT_MS` is 30
seconds. A volume pass over a 27-slice real CT series under SwiftShader is the
slowest thing this harness has ever done. So the serialisation is a real
scheduling constraint and not a formality.

### 9. `render-params.json` is not changed, and why

**This story does not change `tools/oracle/render-params.json`.** The LLD's
rule about it is one sentence, "a change to that file changes every reference
frame", and the rule is worth keeping literally true rather than qualified.
Concretely:

- `run.json` records `renderParamsSha256`, and leaving it unchanged is
  mechanical evidence that the eighty-nine stack frames are the same artefact
  F-011 started against,
- the new parameters are properties of a pass that did not exist when that file
  was written, and folding them in would make every future stack-frame
  provenance question require reading a volume section to rule it out,
- the shared parameters are not duplicated. `volume-params.json` reads
  `canvas`, `background`, `interpolation` and the VOI resolution from
  `render-params.json`'s `base` and is refused if it names any of them.

The new file carries the same review discipline in its own `$notes`: a change
to it changes every volume reference frame, and it is a pull request with a
rationale, reviewed like code.

### 10. The VOI for a volume

Resolved by the existing `src/voi.mjs`, the one function the tests exercise and
the browser executes, applied to the subject's first member. **The harness
refuses a subject whose members do not all resolve the same window**, because a
single volume viewport carries one `voiRange` and a per-member window would be
recorded in the sidecar as a parameter that produced the frame having produced
nothing. That is the same rule `src/params.mjs` applies to `canvas`. All ten
members of both synthetic series declare centre 40 and width 400 through
`ct_common`, so the refusal is not reached by the corpus as it stands and is
covered by a unit test, which is the same position the CT default and the
full-range rule are already in.

## The output contract F-011 consumes

Stated precisely, because F-011 is being built at the same time and this is
what lets that happen.

### Naming

`volume__` is a **reserved output-name prefix**. `rowId` in
`src/manifest.mjs` gains a refusal for a corpus path that reduces to an id
starting with `volume__`, so a future corpus row cannot collide with a volume
frame. No current row does.

- Subject id: `volume__` plus the series directory with `/` replaced by `__`.
  - `volume__synthetic__ct_series_uniform`
  - `volume__synthetic__ct_series_nonuniform`
  - `volume__real__ct_cmb_mml`
  - `volume__real__mr_eay131`
- Frame id: the subject id, `__`, and the orientation in the exact spelling of
  `Enums.OrientationAxis`, upper-cased as the enum's key rather than its value:
  `AXIAL`, `SAGITTAL`, `CORONAL`.
  - `volume__synthetic__ct_series_nonuniform__SAGITTAL`

Each frame id writes the same three files beside the stack frames, in the same
flat `out/` directory: `<id>.raw`, `<id>.png`, `<id>.json`. Four subjects times
three orientations is twelve new frames.

### The `kind` discriminator

**Every sidecar gains a top-level `"kind"`, including the existing eighty-nine.**
`"stack"` on those, `"volume-reformat"` on the new twelve. F-011 switches on it
and must not infer a shape from a filename. This is the single field that makes
"a comparator written before F-X007 lands must not assume stack-only input"
actionable.

### The volume sidecar

Additive to the stack sidecar's shape. `renderParams`, `voi`, `camera`,
`frame`, `reference` and `files` keep their existing shapes and meanings, so
F-011's readers for those are reusable unchanged. New blocks:

```
{
  "kind": "volume-reformat",
  "volume": {
    "id": "volume__synthetic__ct_series_nonuniform",
    "seriesDirectory": "synthetic/ct_series_nonuniform",
    "members": [
      { "path": "synthetic/ct_series_nonuniform/slice_000.dcm",
        "sha256": "...", "imageId": "dicomfile:0", "stackSidecar": "<id>.json",
        "instanceNumber": 1,
        "imagePositionPatient": [0, 0, 0],
        "imageOrientationPatient": [0.8, 0.6, 0, 0, 0, -1],
        "pixelSpacing": [0.5, 0.25],
        "sliceThickness": 2.5,
        "spacingBetweenSlices": null }
    ],
    "referenceSortedImageIds": ["dicomfile:0", "..."],
    "build": { "outcome": "built", "error": null, "consoleDuringBuild": [] },
    "referenceGeometry": {
      "dimensions": [20, 12, 10],
      "spacing": [0.25, 0.5, 2.5],
      "origin": [0, 0, 0],
      "direction": [0.8,0.6,0, 0,0,-1, -0.6,0.8,0],
      "dataType": "Uint16Array"
    },
    "measuredGeometry": {
      "normal": [-0.6, 0.8, 0],
      "projectionsMm": [0, 2.5, 5, 7.5, 10, 12.5, 15, 18.75, 20, 22.5],
      "gapsMm": [2.5,2.5,2.5,2.5,2.5,2.5,3.75,1.25,2.5],
      "meanGapMm": 2.5, "medianGapMm": 2.5,
      "minGapMm": 1.25, "maxGapMm": 3.75,
      "maxDeviationFromMeanMm": 1.25,
      "uniform": false,
      "toleranceMm": 1e-6,
      "voxelAxes": {
        "columnStepMm": [0.2, 0.15, 0],
        "rowStepMm": [0, 0, -0.5],
        "sliceStepMm": [-1.5, 2.0, 0]
      }
    },
    "truth": { "source": "volume-truth.json", "uniform": false,
               "citation": "PS3.3 C.7.6.2.1.1" },
    "referenceAgreesWithTruth": false,
    "referenceDivergence": {
      "field": "spacing[2]", "reference": 2.5, "truth": null,
      "attributedTo": "reference", "why": "..." },
    "zProfile": { "voxel": [10, 6], "storedValues": [...],
                  "expectedStepPerSlice": 16 }
  },
  "reformat": {
    "orientation": "SAGITTAL",
    "viewportType": "ORTHOGRAPHIC",
    "blendMode": "COMPOSITE",
    "slabThicknessMm": null,
    "millimetresPerCanvasPixel": 0.0871,
    "cameraMode": "reset"
  },
  "volumeParamRulesApplied": [ ... ],
  "renderParams": { "canvas": {...}, "background": [...],
                    "interpolation": "NEAREST", "voi": {...} },
  "voi": { "source": "file", "windowCenter": 40, "windowWidth": 400,
           "voiLutFunction": "LINEAR", "origin": "file-top-level" },
  "camera": { "parallelScale": ..., "position": [...], "focalPoint": [...],
              "viewUp": [...], "viewPlaneNormal": [...],
              "flipHorizontal": ..., "flipVertical": ... },
  "frame": { "width": 512, "height": 512,
             "format": "RGBA8, top row first, as ImageData from the viewport canvas",
             "sha256": "...", "statistics": { ... } },
  "reference": { "cornerstone3D": "5.8.2", "dicomImageLoader": "5.8.2",
                 "browser": "...", "adapter": "..." },
  "files": { "raw": "<id>.raw", "png": "<id>.png" }
}
```

Three points F-011 should hold:

- the `volume` block is **repeated identically in all three** orientation
  sidecars for a subject. That is deliberate. The existing design's rule is
  that a frame never travels alone, and a comparator that had to join two files
  to explain one frame would break that,
- `camera` gains `viewPlaneNormal`, which the stack sidecar does not carry and
  which a reformat is meaningless without,
- **`downsampled` stays a stack-only concept.** `canvasScale` in
  `src/params.mjs` derives a per-axis magnification from the source's row and
  column spacing, and a reformat plane has no source pixel grid to be a
  magnification of. Rather than publish a number that means something else
  under the same name, the volume frame records
  `reformat.millimetresPerCanvasPixel`, computed from the camera's own
  `parallelScale` and the canvas height, and F-011 decides what to do with it.

### `run.json`

Additive. Nothing existing changes shape or meaning, with one exception named
below.

- `story` becomes `"F-010, F-X007"`.
- `rows[]` **stays stack-only**, and every entry gains `"kind": "stack"`. The
  accounting identity `readBack + unsupported === applicable` is asserted over
  `rows[]`, and mixing volume frames into it would break an assertion that
  currently means something exact.
- new `volumeParamsSha256` and `volumeTruthSha256`, beside
  `renderParamsSha256` and `unsupportedSha256`.
- new `volumes[]`, one entry per subject:
  `{ id, seriesDirectory, memberCount, outcome, referenceAgreesWithTruth,
  referenceDivergence, measuredGeometry, referenceGeometry,
  frames: [{ id, orientation, sha256 }] }`.
- new `framePairs[]`, one entry per declared pair:
  `{ left, right, expect, orientations: [{ orientation, leftSha256,
  rightSha256, identical }] }`.
- `boundaries` gains `volumesApplicable`, `volumesBuilt`, `volumesRefused`,
  `reformatsPresented`, `reformatsReadBack`.
- `determinism.mismatches[]` entries gain `kind` and use `id` for a volume
  frame where a stack entry uses `path`.
- `lowInformation.rows[]` entries gain `kind`, and a saturated reformat is
  listed there under the same declared `informationFloor` threshold.
- `checks` gains `volumePass: boolean`, so a run that skipped the volume pass
  is not indistinguishable from one that did it. It is `false` only when a
  `--rows` selection contains no complete subject.

**The one exception.** `check_sidecars.py` today refuses any sidecar whose
`row.path` is not a manifest row, and it globs `out/*.json`. It gains the
`kind` partition: stack sidecars keep every check they have today unchanged,
and volume sidecars are cross-read against pydicom on their per-member geometry
attributes, which is where the new load-bearing metadata is. Completeness stays
strict in both directions on both partitions.

## Boundary and tier

- wasm-bindgen: not touched. This story is Node, a browser and a Python
  cross-read. It writes no Rust and does not touch `tools/oracle/Cargo.toml` or
  `tools/oracle/src/lib.rs`, which are F-011's.
- Pixels across the boundary: no. There is no Ocelli boundary in this story.
  The pixels are cornerstone3D's and they go to a file.
- Render-loop allocation: none. There is no Ocelli render loop here.
- unsafe: none.
- Tier A (WebGPU): n/a. This runs somebody else's renderer, as F-010 did.
- Tier B (WebGL2): n/a. The reference runs on SwiftShader under a WebGL2
  context, which the run asserts. **That is a fact about the reference and not
  a tier declaration by Ocelli.** cornerstone3D v5 renders through vtk.js on
  WebGL2 and `@kitware/vtk.js` is a hard dependency of core 5.8.2, so a volume
  render has no other path there.
- Tier C (CPU): n/a for this story, and worth one extra sentence because the
  temptation to read it otherwise is real here. D-07's A7.1b consequence is
  that "CPU MPR is required, not optional", split into F-X004. This story
  produces the reference reformats that F-X004's claim will eventually be
  measured against, and it declares no CPU path of its own.

All three rows are n/a rather than omitted, for the reason F-010's plan gave
and which applies more strongly to a story with "MPR" in its title: it would be
easy to read this as declaring a tier because it renders volumes. It does not.

## Tests

| Category | What it proves | Where |
|----------|----------------|-------|
| `fixture` | The slice normal, the projected positions, the gaps, and the two voxel in-plane steps are the hand-computed values from PS3.3 C.7.6.2.1.1 and `scripts/corpus_synth.py`'s constants, within HLD 25.1's 1e-6 mm | `tools/oracle/tests/geometry_test.mjs` |
| `fixture` | `PixelSpacing[0]` multiplies the COLUMN direction cosine and `PixelSpacing[1]` the ROW one, asserted on the deliberately non-square `[0.5, 0.25]` so a transposition fails | `tools/oracle/tests/geometry_test.mjs` |
| `fixture` | The z profile of the built volume is a ramp of exactly 16 stored units per slice, in order, per `case_series` and PS3.3 C.7.6.3.1.4 | `tools/oracle/tests/volume_test.mjs` and the volume page |
| `fixture` | Every sidecar's per-member `ImagePositionPatient`, `ImageOrientationPatient`, `PixelSpacing` and `SliceThickness` match pydicom's independent reading of the same file | `tools/oracle/check_sidecars.py` |
| `golden` | Twelve reformat frames decode, build, present and read back through the pinned cornerstone3D, deterministically over two passes | `tools/oracle/`, Playwright |
| `golden` | The uniform and non-uniform subjects render to identical digests in all three orientations, which is the recorded reference divergence | `tools/oracle/volume-truth.json`, asserted by the driver |
| `browser` | Each of the six new refusals is observed red at its own boundary for its own reason | `tools/oracle/src/faults.mjs`, `tests/faults.mjs` |
| `unit` | Subject resolution, member completeness in both directions, the partial-selection refusal, and the refusal of a `volume-params.json` key that `render-params.json` owns | `tools/oracle/tests/volume_test.mjs` |
| `unit` | `rowId` refuses a corpus path reducing to the reserved `volume__` prefix | `tools/oracle/tests/manifest_test.mjs` |

The hand-computed table the `fixture` rows assert, derived above from PS3.3
C.7.6.2.1.1 and `scripts/corpus_synth.py`, and repeated here so it sits beside
the tests that use it:

```
IOP        = [0.8, 0.6, 0.0,  0.0, 0.0, -1.0]
row        = (0.8, 0.6,  0.0)
col        = (0.0, 0.0, -1.0)
normal     = row x col = (-0.6, 0.8, 0.0),  |normal| = 1 exactly

PixelSpacing = [0.5, 0.25]   ->  between rows 0.5, between columns 0.25
rowStep    = PixelSpacing[0] * col = 0.5  * (0, 0, -1)     = ( 0.0,  0.0, -0.5)
columnStep = PixelSpacing[1] * row = 0.25 * (0.8, 0.6, 0)  = ( 0.2,  0.15, 0.0)

uniform     projections = 0, 2.5, 5, 7.5, 10, 12.5, 15, 17.5,  20, 22.5
non-uniform projections = 0, 2.5, 5, 7.5, 10, 12.5, 15, 18.75, 20, 22.5
non-uniform gaps        = 2.5 x6, 3.75, 1.25, 2.5
mean gap   = 22.5 / 9 = 2.5    for BOTH series
median gap = 2.5               for BOTH series
SliceThickness = 2.5           on every slice of BOTH series
SpacingBetweenSlices absent    from every slice of BOTH series
```

The mutation check HLD 27.3 asks for is stated rather than left to the
implementer's judgement. Change `NONUNIFORM_OFFSET`'s effect in the fixture's
expected `gapsMm` from `3.75` to `2.5` and the geometry suite must go red.
Swap `PixelSpacing[0]` and `PixelSpacing[1]` in `voxelAxes` and the
transposition fixture must go red. Neither may be run in the command that adds
the test.

## Parity surface covered

`docs/hld/B-parity-surface.md`'s `Covered by` column is described in the HLD
but is not present in the tracked import, which F-010's plan already recorded.
Against the surface table as it stands, this story touches one row and covers
none of it as an Ocelli feature:

| Surface | Count | This story |
|---|---|---|
| Viewport types | 12 | Produces reference output through `ORTHOGRAPHIC`, one of the twelve. It implements no Ocelli viewport. |

So the honest answer is **none**, the same as F-010's, and for the same reason:
this builds the instrument that will measure a parity feature, not the feature.
It is written out rather than left blank because a blank section and a section
nobody wrote read identically later.

## Deviations

**None applied by this plan.** Two are described in the open questions below
for the operator to apply if the answers go that way, since the design command
forbids filling an HLD gap with a plausible design presented as specified.

D-11 is relied on and is unchanged. Its row names three cornerstone3D packages
by name and this story adds no package, so its text stays accurate.

D-04 is unchanged. The oracle stays out of `--floor`.

D-07 is cited only in the tier C row, and this story declares no tier.

## LLD impact

`docs/lld/oracle.md`, and this is the largest single edit:

- **"Stack viewports only, and what that leaves undone" is replaced**, not
  amended. Its whole content is now false. It becomes "Stack and volume, and
  what each covers", carrying forward the sixty-two `series` rows accounting
  because that arithmetic is still correct and still useful.
- new sections for the volume pass, the subject declaration, the two new
  committed files, the volume boundaries, the `<id>` scheme, the `kind`
  discriminator, and the `run.json` additions.
- **a new section recording what cornerstone3D 5.8.2 does with non-uniform
  spacing**, with the file and line quotations from this plan, sitting beside
  the existing SIGMOID divergence record, because those two are the same kind
  of fact.
- the layout table gains the two new committed files and the volume page.
- the suite-count word moves from "ten" to "twelve".
  `tests/registration_test.mjs` asserts that word against the directory, so it
  goes red until it is updated, which is the intended behaviour.

`docs/lld/corpus.md`:

- the twenty spacing rows are now exercised as geometry as well as pixels, with
  a pointer to the oracle LLD's new section.
- **a correction.** `scripts/corpus_synth.py`'s comment says an averaging
  builder is "wrong by a constant factor". On this construction the mean and
  the median are both exactly 2.5, identical to the uniform series, and the
  error is one slice placed 1.25 mm out of position rather than a scale error.
  The corpus LLD says so. The generator's comment is not edited by this story,
  because changing a comment in `scripts/corpus_synth.py` does not change a
  digest but does invite a regeneration nobody asked for, and the manifest is
  what ties every reference frame to a corpus.

## Write set

Created:

- `tools/oracle/volume-params.json`
- `tools/oracle/volume-truth.json`
- `tools/oracle/src/geometry.mjs`
- `tools/oracle/src/volume.mjs`
- `tools/oracle/page/volume.html`
- `tools/oracle/page/volume.mjs`
- `tools/oracle/tests/geometry_test.mjs`
- `tools/oracle/tests/volume_test.mjs`

Modified:

- `tools/oracle/run.mjs`
- `tools/oracle/build-page.mjs`
- `tools/oracle/package.json` (the `test` script's suite list only)
- `tools/oracle/check_sidecars.py`
- `tools/oracle/src/sidecar.mjs`
- `tools/oracle/src/output.mjs`
- `tools/oracle/src/manifest.mjs`
- `tools/oracle/src/faults.mjs`
- `tools/oracle/tests/manifest_test.mjs`
- `tools/oracle/tests/sidecar_test.mjs`
- `docs/lld/oracle.md`
- `docs/lld/corpus.md`

Deliberately not touched, and each for a stated reason above:
`tools/oracle/render-params.json`, `tools/oracle/unsupported.json`,
`tools/oracle/page/app.mjs`, `tools/oracle/page/index.html`,
`tools/oracle/src/params.mjs`, `tools/oracle/src/voi.mjs`,
`tools/oracle/src/paths.mjs`, `tools/oracle/src/pins.mjs`,
`tools/oracle/src/server.mjs`, `tools/oracle/src/unsupported.mjs`,
`tools/oracle/tests/registration_test.mjs`, `tools/oracle/Cargo.toml`,
`tools/oracle/src/lib.rs`, `bin/ocelli.sh`, `corpus/manifest.tsv`,
`scripts/corpus_synth.py`, `docs/hld/`, `docs/sprints/`, `CHANGELOG.md`.

## Open questions

1. **The HLD does not say slice spacing comes from projected
   `ImagePositionPatient`, and the allocation note says it does.** Section 19
   is transcribed above in full and contains no such sentence. Section 16 is
   the coordinate spaces and contains no such sentence. A grep of `docs/hld/`
   finds nothing. The rule is correct, it comes from PS3.3 C.7.6.2.1.1, and it
   is not in the specification.
   **Blocks:** whether this plan may state the rule as normative, or must carry
   it as a repository decision. **If the latter**, a deviation row is wanted at
   the next free number, and precisely this text. The number is written `D-NN`
   here rather than guessed, because `scripts/deviation_check.py` refuses a
   plan that cites a row nobody has written yet, and because F-X009's plan is
   proposing a row in the same design round. The row wanted is this, and it is
   given as a fenced block rather than inline because the type it quotes
   carries a semicolon:

```text
| D-NN | §19 specifies `spacing: [f64; 3]` and `origin` as "IPP of the first
slice" and says nothing about how through-plane spacing is derived |
Through-plane spacing is derived from consecutive ImagePositionPatient values
projected onto the cross product of the two ImageOrientationPatient direction
cosines, never from SpacingBetweenSlices (0018,0088) or SliceThickness
(0018,0050) | PS3.3 C.7.6.2.1.1 makes the projected IPP the only spacing the
standard defines between two instances. The other two tags are Type 3 and
Type 2 respectively and are frequently wrong, which scripts/corpus_synth.py
records at case_series. §19 does not state a derivation and the corpus was
built to exercise one, so the rule is written down here rather than inferred |
F-X007 |
```

   **The preferred answer is that this becomes an HLD amendment to section 19
   rather than a deviation**, because a deviation records a departure and this
   is a gap, but that is the operator's call and this plan does not take it.

2. **Nothing states the volume builder's refusal policy, and the story's
   "done" sentence names it.** CURRENT_SPRINT says F-X007 is done when the ten
   non-uniform rows "exercise the volume builder's refusal path". There is no
   volume builder, D7 holds and no port code exists, and no HLD section states
   that a non-uniform series is refused, at what threshold, or what a caller
   sees.
   **Blocks:** what this story's `volume-truth.json` may write in
   `ocelliVerdict` for the non-uniform subject. This plan currently writes only
   `uniform: false` and the measured deviation, and asserts nothing about what
   Ocelli should do, so it can land either way. **The narrower reading, which
   this plan takes**, is that F-X007 is done when the reference output makes the
   refusal decision checkable, and the refusal itself belongs to the volume
   story that has no F-ID in this sprint. Confirm, or say what else is wanted.

3. **What uniformity tolerance applies to a real series.** HLD 25.1's 1e-6 mm
   is a comparison tolerance and is exactly right for the two synthetic
   subjects, whose arithmetic is exact by construction. It will classify almost
   any real series as non-uniform, which is not a useful claim. This plan
   therefore records `maxDeviationFromMeanMm` for `real/ct_cmb_mml` and
   `real/mr_eay131` and classifies neither, leaving `uniform: null`.
   **Blocks:** nothing in this story, and it blocks the eventual builder.
   Flagged so the number is in the output when the question is asked.

4. **Whether the two real series should be subjects at all in this sprint.**
   They give F-011 realistic volume references, and they are the slowest thing
   this harness has ever done: 27 and 15 slices at real CT and MR resolutions,
   three reformats each, twice for determinism, under SwiftShader.
   **Blocks:** the oracle gate's wall-clock budget. The mitigation is already
   in the design, since subjects are declared in a committed file and dropping
   the two real ones is a one-line change with a rationale. Ask for a stated
   budget, or accept measuring it first and reporting the number.

5. **Whether the `wadouri:` branch should ever be exercised.** The harness's
   `dicomfile:` ids take the `!usingWadoUri` branch. The `wadouri:` branch at
   `calculateSpacingBetweenImageIds.js:68-84` is coarser still: it computes
   `|d_first - d_middle| / floor(N/2)` from two slices only, and
   `sortImageIdsAndGetSpacing.js:38` does not sort at all, it keeps the
   caller's order and may reverse it in place. On our non-uniform series that
   path never even looks at slice 7, since the displacement lies outside the
   first-to-middle span, so it also answers 2.5.
   **Blocks:** nothing here. It is a second reference behaviour that a DICOMweb
   deployment would meet and this harness does not. Recorded in the LLD, and
   worth an S04 story if the answer is that it should be covered.

6. **Whether `volume-truth.json` and `volume-params.json` should be one file.**
   Two files split "what we asked for" from "what is true", mirroring
   `render-params.json` against `unsupported.json`. One file would be less to
   read. This plan takes two.
   **Blocks:** only the layout table in the LLD. Say if one is preferred.

---

## Decisions taken in the design round

Answers to `## Open questions`, taken in the S03 consolidated round.

**1. Cite PS3.3 C.7.6.2.1.1 directly. No deviation row and no HLD change.**
This story does not need the projected-IPP rule to be normative in the HLD,
because it computes `measuredGeometry` from the standard itself and cites the
standard at the site. The rule's normative status matters to the future volume
builder, not to a reference-render harness. **The gap is real and is recorded as
a finding** in `docs/lld/oracle.md`, that HLD section 19 gives a three-element
spacing array in millimetres and says nothing about how the through-plane
element is derived, so the story that builds the volume can raise it with
evidence rather than with an assertion. The `D-NN` block in `## Open questions` is **not applied** and is
kept as the record of what was considered and declined, so the next person does
not propose it again from scratch.

**2. The narrow reading of "done" is confirmed.** F-X007 is done when the
reference output makes the refusal decision checkable. It asserts nothing about
what Ocelli should do with a non-uniform series, because no volume builder
exists and no HLD section states a policy. `volume-truth.json` writes
`uniform: false` and the measured deviation, and writes no Ocelli verdict.

**3. Uniformity for the real series is recorded, not judged.** Publish
`maxDeviationFromMeanMm` and leave `uniform: null` for the two real series.
25.1's 1e-6 mm is exact for the synthetic subjects by construction and would
classify almost any real series as non-uniform, so applying it to a real series
would produce a verdict the corpus cannot support.

**4. All four subjects run, and the volume timeout is a declared parameter.**
The two real series are the slowest thing this harness has attempted, so
`volume-params.json` carries its own timeout with a written reason rather than
inheriting `RENDER_TIMEOUT_MS`. **If a subject cannot complete, it is declared
and named with its reason, never silently dropped**, which is the same rule
`unsupported.json` already applies to the stack pass. Declaring the subjects in
a committed file is what makes that possible.

**5. The `wadouri:` branch stays recorded and unexercised.** The harness takes
the `dicomfile:` branch, which is the more careful one, and exercising the
coarser branch is a separate question worth its own story.

**6. `volume-params.json` and `volume-truth.json` stay two files.** One declares
what to render and one declares what is true, and they are reviewed by different
people for different reasons. The LLD layout table names both.

**7. NEW, from F-011's design round. Record `canvasPixelsPerSourcePixel` for
every stack row, not only the two decimated ones.** This is one line in
`src/params.mjs` and `src/sidecar.mjs`, both of which this story already
modifies, and it removes a second copy of the canvas-scale derivation that
F-011 would otherwise have to write in Rust. HLD section 18's rule about a
second copy of the LUT chain is about arithmetic living once, and the reasoning
generalises. **This adds `src/params.mjs` to this story's write set.**

**8. The output contract published in this plan is binding on F-011.** The
`kind` discriminator, the `volume__` id scheme, `rows[]` staying stack-only and
every `run.json` addition being additive are what let the two stories be built
in the same wave. F-011's comparator dispatches on `kind` and refuses an unknown
value. Any change to that contract during implementation is reported to the
integrator before it lands, not discovered at the merge.

**9. F-X009 lands last** and absorbs this story's six new faults and any new
refusal in `run.mjs` or the volume page. This story lists them in its handoff.
