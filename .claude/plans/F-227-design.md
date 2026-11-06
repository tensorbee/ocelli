# F-227, An automatic window per series when the file has none, labelled Auto window

**Status**: approved
**Epic ref**: E38.33, proposed, see "Proposed backlog row"
**Sprint**: S46
**Estimate**: 3w

Story row, to be filed by the sprint lead in `docs/sprints/BACKLOG.md` and `docs/sprints/allocation.json` before implementation starts: "Automatic window per series when the file has none: one window from the 1st to 99th nearest-rank percentile of modality values pooled over up to 16 frames spread through the series, each frame through its own rescale, leaving out stored zero and the Pixel Padding Value range, computed once in ocelli-pixel, labelled Auto window in the folder viewer, and every window in the file still used as it is". Layer All, depends on F-210 and F-213, milestone M20.

## Proposed backlog row

```text
| F-227 | E38.33 | S46 | Automatic window per series when the file has none: one window from the 1st to 99th nearest-rank percentile of modality values pooled over up to 16 frames spread through the series, each frame through its own rescale, leaving out stored zero and the Pixel Padding Value range, computed once in ocelli-pixel, labelled Auto window in the folder viewer, and every window in the file still used as it is | All | 3w | F-210, F-213 | pending |
```

`grep -c 'E38\.3[3-9]' docs/sprints/BACKLOG.md docs/sprints/allocation.json` prints 0 for both on `8729656`, so E38.33 is the next free E38 number. The sibling plans written in the same round take other epics (F-224 E5.7, F-225 E3.8, F-230 E16.6), so this row takes E38.33 and F-228 takes E38.34. If another story is filed into E38 first, the lead allocates in F-ID order and corrects both numbers. E4.9 was considered and rejected: `docs/sprints/BACKLOG.md` "Recorded defects" names E4.9 as F-145's unresolved dependency, and filing a story there would silently resolve a defect that is recorded on purpose. The estimate rose from 2w to 3w with the per-series answer, which adds a route, a page cache and a benchmark workload. Allocation notes: "Operator decisions of 2026-11-05 on the legacy Cornerstone issue list, auto window, then per series. Library half is parallel safe. Viewer half runs after F-213 in the viewer order."

## Normative source, transcribed

### The operator's answers, given on 2026-11-05 and relayed by the sprint lead

First answer: "Auto window, labelled. Ocelli computes the 1st to 99th percentile window, ignoring background zeros, and the corner says 'Auto window'. Any window in the file is always used as it is."

Second answer, the option the operator chose: "One window for the whole series, computed from a fixed sample of frames spread through it (for example up to 16 decoded frames). Scrolling keeps relative brightness between slices. Opening the series costs those extra decodes once."

### `docs/hld/15-lut-chain.md` section 18, the stage table and the instruction

|  |
|----|
| This is the highest-risk arithmetic in the project. It is specified in DICOM PS3.3 C.11 and the stages apply strictly in order. Implement it once, in ocelli-pixel, and let the shader read the parameters — do not let a second copy of this logic appear anywhere. |

| **Stage** | **From → To** | **Source** |
|----|----|----|
| 1\. Modality LUT | Stored → Modality | Rescale slope/intercept, or a Modality LUT Sequence which takes precedence |
| 2\. VOI LUT | Modality → Display | Window centre/width with a function, or a VOI LUT Sequence |
| 3\. Presentation LUT | Display → Display | Identity or INVERSE; presentation state may override |
| 4\. Palette / ICC | Display → RGB | Palette colour LUT, or the display colour pipeline |

### Section 18.1

```rust
pub fn modality(sv: Stored, slope: f32, intercept: f32) -> Modality {
    Modality(sv.0 * slope + intercept)
}
```

|  |
|----|
| If a Modality LUT Sequence is present it wins over slope and intercept. PET SUV is a separate path and needs the radiopharmaceutical sequence — do not fold it in here. |

### Section 18.2, the three functions, verbatim

```text
// PS3.3 C.11.2.1.2 -- LINEAR. Requires w >= 1.
// c' = c - 0.5 ; w' = w - 1
// x <= c' - w'/2 -> ymin
// x > c' + w'/2 -> ymax
// else y = ((x - c') / w' + 0.5) * (ymax - ymin) + ymin
// PS3.3 C.11.2.1.3.2 -- LINEAR_EXACT. Requires w > 0.
// x <= c - w/2 -> ymin
// x > c + w/2 -> ymax
// else y = ((x - c) / w + 0.5) * (ymax - ymin) + ymin
// PS3.3 C.11.2.1.3.1 -- SIGMOID. Requires w > 0.
// y = (ymax - ymin) / (1 + exp(-4 * (x - c) / w)) + ymin
```

### Section 18.3, the fixture rows, with deviation D-13 applied

Centre 40, width 400, output 0 to 255. LINEAR then LINEAR_EXACT. `-160` gives 0.000 and 0.000 (D-13 corrects the HLD's 1.594 to 0.000). `40` gives 127.819 and 127.500. `240` gives 255.000 and 255.000. `-60` gives 63.910 and 63.750. This story adds no VOI formula and these rows stay green through `crates/ocelli-pixel/tests/voi.rs`.

### Section 7, `docs/hld/05-rendering.md`, the compute sentence

|  |
|----|
| Anything wanting compute — GPU segmentation, histogram passes, compute-based resampling — is tier A only and must degrade, not fail. |

### What the HLD says about a missing window

Nothing. `grep -rn -i "default window\|no window\|percentile\|auto.window" docs/hld/*.md` prints nothing. Section 31's kernel list names "per-frame histogram and ROI statistics" as a future tier A compute kernel and says nothing about a window derived from one. This plan uses the viewer's own series grouping, which is not a volume.

### The library today, read from the tree

`crates/ocelli-pixel/src/lut.rs:339`, `VoiTransform::new`, refuses an empty window:

```rust
if centers.is_empty() || centers.len() != widths.len() {
    return Err(PixelError::MismatchedWindowMultiplicity);
}
```

`crates/ocelli-dicom/src/frame_open.rs:98`:

```rust
pub enum WindowInput {
    /// A sequence or a validated window pair.
    Transform(VoiTransform),
    /// No sequence and no centre or width. Nothing is invented.
    Absent,
}
```

`examples/folder-viewer/src/use.html` line 602, inside the `open_frame` example: "No VOI LUT Sequence and no window. The caller chooses, Ocelli does not guess."

`crates/ocelli-dicom/src/frame_open.rs:141`, "Inclusive stored-pixel padding from PS3.3 C.7.5.1.1.2", with `PaddingRange::value` documented as "The Pixel Padding Value, in stored space, before the modality stage" and `contains(stored: i32)` inclusive between the minimum and maximum of value and limit.

### The folder viewer today, read from the tree

`examples/folder-viewer/src/preview.rs:735` on `8729656`, `window`, falls back when both Window Center and Window Width are empty:

```rust
if centers.is_empty() && widths.is_empty() {
    let (center, width) = full_range(modality, stored)?;
    return Ok((vec![center], vec![width], VoiFunction::LinearExact, true));
}
```

`full_range` (line 752) takes the minimum and maximum modality value of every sample, returns `(low, 1.0)` when they are equal, and otherwise `((low + high) / 2.0, high - low)`. The page appends `  full range` to the bottom right window line (`page.html:1883`), and the reply header is `X-Ocelli-Full-Range` (`serve.rs:271`). In the F-216 worktree (`ocelli-wt-f-216`, read only, uncommitted on `8b00223`) the same rule moves into `file_window_of` at decode time, as `FileWindow::Full` and `HeldFrame::full_range`, computed once per held frame.

**The viewer's series.** `page.html:1302` `groupSeries(found)` runs once over every opened file. The series key is `"uid:" + seriesUid`, or `"file:" + order` for a file with no Series Instance UID. A monochrome multi-frame object contributes one image per frame up to `FRAME_LIST_CAP`. `series.images` is sorted by Instance Number, then file name, then frame (`page.html:1337`). Each image is `{ file, frame, info, reported }`, and after F-216 each file has an `X-Ocelli-File` id.

`examples/folder-viewer/tests/preview.rs` pins `a_zero_length_window_is_full_range` and `a_missing_window_is_labelled_full_range`, both one stored sample 50 giving grey 128, and `the_page_posts_a_frame_and_returns_its_grey_byte`, which asserts `X-Ocelli-Full-Range: 0`.

### The oracle, verified not to depend on the viewer's fallback

`tools/oracle/render-params.json` `modalityVoiDefaults` declares CT as centre 40, width 400, `LINEAR`, and `"*"` as `"rule": "full-range"`. `tools/oracle/src/voi.mjs` `fullRange(min, max)` returns `windowCenter: (max + min + 1) / 2` and `windowWidth: max - min + 1` under LINEAR, and `resolveVoi` writes the chosen window into the reference sidecar's `voi` object. The Ocelli candidate reads its window from that sidecar: `tools/oracle/src/produce_current.rs` `linear_window` reads `/voi/windowCenter`, `/voi/windowWidth` and `/voi/voiLutFunction`, and `gpu_candidate.rs` takes `VoiParams::from_chain` of a presentation built from the same injected values. No oracle source names `ocelli-folder-viewer`. **Changing the viewer default does not touch LUT chain parity.** This story leaves `render-params.json` and `voi.mjs` unchanged.

### The reference, cornerstone3D 5.8.2, for the record and not as a target

`tools/oracle/node_modules/@cornerstonejs/core` is 5.8.2. `RenderingEngine/StackViewport.js` `_getInitialVOIRange(image)` takes the image's own `windowCenter` and `windowWidth`, then `voiRange = this._getPTPreScaledRange() || voiRange`, where `_getDefaultPTPrescaledVOIRange` returns `{ lower: 0, upper: 5 }` only for a PT image pre-scaled with an `suvbw` parameter. Otherwise an absent window leaves the range to the image's own minimum and maximum, per image. That is the behaviour the operator's team reported. This story deliberately differs from it and claims no parity row for the default.

### External definitions the fixtures rely on, cited and not quoted

The standard text is not in the repository, so these are citations a reviewer checks at the source.

- PS3.3 C.11.1, the Modality LUT Module, stage 1 above, applied per frame. PS3.3 C.7.6.16.2.9, Pixel Value Transformation, which lets rescale differ per frame in an enhanced object, and the pseudo-PHI PT series show it differing per image.
- PS3.3 C.11.2.1.3.2, LINEAR_EXACT, transcribed above through section 18.2.
- PS3.3 C.11.2, a VOI LUT Sequence and a window pair are alternatives, and the sequence is used when present.
- PS3.3 C.7.5.1.1.2, Pixel Padding Value (0028,0120) and Pixel Padding Range Limit (0028,0121), values in stored pixel space that are not part of the image, declared per object.
- PS3.3 C.7.6.3.1, Pixel Representation and Bits Stored, for the signed fixture.
- The nearest-rank percentile: for sorted values `x[1..n]` and `0 < P <= 100`, the P-th percentile is `x[ceil(P * n / 100)]`. Several sample percentile definitions exist and they disagree on small samples, so this plan names the one it uses rather than citing a source for "the" percentile. The fixtures below are computed from this formula, not from a library.

## What the specification does not cover

The HLD and DICOM say nothing about a window for a frame that has none. The operator's two answers fix the rule. Item 11 is Answered: operator, 2026-11-05, per series. Every other item is Answered: lead decision under the operator's delegation of 2026-11-05.

1. **Where it lives.** `crates/ocelli-pixel/src/auto_window.rs`, a new module beside `lut.rs`. F-075's histogram is in `crates/ocelli-geom/src/stats.rs`, but it is a binned histogram over caller `f64` samples, `ocelli-geom` does not depend on `ocelli-pixel`, and a VOI window is a stage 2 input that needs `ModalityTransform`. A percentile read from bins would be approximate, which is the quietly wrong class. The frame selection, the cap split, the pooling and the window are computed here once and every caller reuses them.
2. **What is measured.** Modality values, PS3.3 C.11.1, **each frame through its own `ModalityTransform`**. Within one PT series of the pseudo-PHI study the Rescale Slope differs per image, so a pool of stored values, or of values through one frame's slope, is a different and wrong distribution. AC3 is built to tell them apart.
3. **What "background zeros" means: stored value 0, plus that frame's own Pixel Padding Value range when its object declares one.** Both are stored-space predicates, tested per frame before its rescale. (a) PS3.3 C.7.5.1.1.2 defines padding in stored space per object, and `PaddingRange` already models it that way. (b) For the modalities the operator named, stored 0 and modality 0 coincide. The pseudo-PHI PT series have intercept 0 on every image, MR and US carry no rescale, and the MG Secondary Capture has intercept 0. (c) Where they differ, stored 0 is the right one. An unsigned CT with intercept -1024 and no window writes air outside the field of view as stored 0, which is -1024 HU, while modality 0 there is water. AC4 tells the two apart.
4. **What is not left out.** Negative values and every other stored value.
5. **Percentile definition: nearest rank, no interpolation, over the pooled values.** For `n` counted values pooled from every sampled frame, sorted ascending, `low = x[ceil(n / 100)]` and `high = x[ceil(99 n / 100)]`, one-based. In integers `rank_low = n.div_ceil(100)` and `rank_high = (99 * n).div_ceil(100)`, both at least 1 for `n >= 1`, with no `as` cast. The order in which frames are added cannot change the answer, because the answer is two order statistics of a multiset.
6. **Which frames are sampled.** `pub const AUTO_WINDOW_SERIES_FRAMES: usize = 16`, the operator's example number. For a series of `M >= 1` images in the viewer's order, `K = min(M, 16)` images are sampled at indices `i_k = k * (M - 1) / (K - 1)` in integer division for `k = 0..K`, and `[0]` when `K = 1`. The first image (index 0) and the last (index `M - 1`) are always included, the indices strictly increase because the step `(M - 1) / (K - 1)` is at least 1 when `M >= K`, and a series of at most 16 images is sampled in full. `pub fn series_sample(images: usize) -> Vec<usize>` in the library returns them, so the page holds no rule. `M = 0` is refused. Worked: `M = 17` gives 0 to 14 and 16. `M = 267` gives 0, 17, 35, 53, 70, 88, 106, 124, 141, 159, 177, 195, 212, 230, 248 and 266.
7. **The sample cap and how it is split.** `pub const AUTO_WINDOW_SAMPLE_CAP: usize = 1 << 20` is the total over the pool. Each of the `K` declared frames gets `budget = AUTO_WINDOW_SAMPLE_CAP / K` in integer division, so the total never exceeds the cap. A frame of `N` samples is read at every `stride`-th sample in row-major order from index 0, with `stride = N.div_ceil(budget)`. The stride depends on `N` and `K` only, never on content. With 16 frames the budget is 65,536, so every 128 by 128 PET and every 256 by 256 MR frame is read exactly, and a 512 by 512 CT frame has stride 4. A single frame keeps the whole cap, which is the per-image rule of the first answer. The exclusions apply after the stride. The scratch is at most 4 MiB of `f32`.
8. **The window produced: LINEAR_EXACT, `width = high - low`, `center = low + width / 2`, in `f32`.** Under C.11.2.1.3.2, `x <= c - w/2` is `x <= low`, giving `ymin`, and `x > c + w/2` is `x > high`, giving `ymax`, so `[low, high]` maps exactly onto `[ymin, ymax]` with no half and no one. LINEAR would need `c = (low + high + 1) / 2` and `w = high - low + 1`, exact only for integer modality values, which a PET slope does not give. A `width` or `center` that is not finite is refused.
9. **Edge cases.**
   - Constant counted values, `high - low` not greater than zero: `center = low`, `width = 1`, grey 128. This keeps the pinned one-sample fixtures. Basis `Constant`.
   - Every sampled value left out: `center = m_max + 0.5`, `width = 1`, where `m_max` is the largest modality value among the left-out samples of the pool. Every sample is `ymin`, black on `MONOCHROME2`. Basis `Background`. Exact while `|m_max| < 2^22`.
   - Two counted values: ranks 1 and 2, the window is exactly the two values.
   - Signed data, PS3.3 C.7.6.3.1: no special case.
   - A non-finite modality value in any sampled frame: refused, `AutoWindowError::NonFiniteModality`. A finite stored integer gives a non-finite modality value only when the rescale overflows `f32`, which is a malformed rescale that a window would hide. Float Pixel Data does not reach this function today.
   - An empty frame, an empty pool, more frames added than declared, or zero frames declared: refused, item 13.
10. **Which images get it.** Every greyscale image, any Modality (0008,0060), with no Window Center and no Window Width (absent or zero length) and no VOI LUT Sequence. A half or mismatched window and a VOI LUT Sequence keep today's `window` refusal. Colour images have no VOI stage. CT is not exempt.
11. **Per series. Answered: operator, 2026-11-05, per series.** One window for every eligible image of the series, from the pooled sample of item 6, so scrolling keeps the same window and therefore the same relative brightness between slices.
12. **Mixed series.** A sampled image that is not eligible under item 10 (it has a file window, a VOI LUT Sequence, is colour, or refuses to decode) contributes nothing and is counted as skipped. It is not replaced by a neighbour, because a replacement rule would make the sample depend on content. If no sampled image contributes, the series has no series window, and its eligible images use the per-image window of item 16. An image with its own window always shows its own window, whatever the series window is.
13. **Library API: an accumulator.** `AutoWindowPool::new(frames)` declares `K`, `add_frame` folds one frame with its own modality stage and padding, `finish` returns the window. `auto_window(stored, modality, padding)` is a pool of one. One code path serves both.
14. **Series means the viewer's series**, `groupSeries` in `page.html`: one Series Instance UID, or one file with none. Its images are in the page's sort order, which is the order `series_sample` indexes. The local process does not group, because it holds files, not series, under F-216.
15. **Decode the sample before the first paint, no provisional window.** When the page shows a series whose first shown image is eligible, it asks for the series window first and draws that image only with it. Reasons: the operator accepted the extra decodes on open, a provisional per-image window would show the first slice at one brightness and then switch, which is the flicker the operator chose to remove, and two windows for one image in one second is a quietly different picture. The honest limit: the first image of a no-window series waits for up to 16 decodes and one selection over at most 2^20 values. AC17 measures it. The sampled frames stay held in F-216's cache, so scrolling to them later does not decode again.
16. **The per-image window remains for requests that carry no series window**: thumbnails, the four-image tile mode before its series window arrives, a series whose sample contributed nothing, and any direct `/preview` caller. It is the same library function with one frame. The page labels it differently (AC15) so the corner never claims a series window it does not have. Thumbnails use it so that opening a folder of 26 series does not decode 16 frames of each.
17. **How the series window reaches a preview.** The local process computes it from a list of held files and returns its centre and width as text. The page stores that text verbatim, per series, and sends it back on each `/preview` of that series as `X-Ocelli-Series-Window: <center> <width>`. The process parses it, checks both are finite and the width positive, and applies it only when the image is eligible. Text round trip, no page arithmetic: Rust's `f32` `Display` is the shortest string that parses back to the same `f32`, and the page never converts it to a JavaScript number. The process holds no series state, so nothing on the server can go stale.
18. **What invalidates the cached series window.** A new folder or synthetic open clears every series window (`dropCache`). A series change selects that series' own entry, computed on first view. Reset does not invalidate it, Reset returns to it. F-216 eviction does not invalidate it, because the window is a function of the sampled files' bytes, which eviction does not change. Eviction during the computation is F-216's 410 rule: the page re-posts the gone file once and retries the request once. A process restart is the same 410 path. Nothing else invalidates it, and a W/L drag, preset, file index or region window replace the shown window without clearing the entry.
19. **The library does not change `open_frame`.** `WindowInput::Absent` still means "Nothing is invented". No wasm export and no `@ocelli/core` surface is added.
20. **Padding in the viewer.** Each sampled frame passes the padding range F-210 stores in its `HeldFrame::value`, from `open_frame(..).padding()`. `PaddingRange` is `i32` and `Stored` is `f32`, exact for integers up to `2^24`. A declared padding value or limit beyond that makes that frame skipped for the series window and the `window` refusal for its per-image window, the rule F-210 uses for the value at the pointer.
21. **F-209 sync.** Reset is copied as Reset, so each linked target shows its own series' window. Two cells showing the same series share its one cached entry and show the same numbers.

## Problem and reproducer

The operator's study, outside git, attributes and aggregate counts only, read with `.venv/bin/python3` and pydicom with `stop_before_pixels=True` for the survey. Five PT series of 128 by 128, `MONOCHROME2`, Bits Stored 16, Pixel Representation 1, 267, 267, 175, 175 and 174 images, carry no Window Center, no Window Width and no VOI LUT Sequence. Their Rescale Intercept is 0 and their Rescale Slope differs per image. Four declare Units `BQML` and one `CPS`. One MG Secondary Capture series of 57 images, 1421 by 1024, unsigned, has no window either. On the middle image of one PT series, 8,200 of 16,384 stored samples are 0 and 92.7 percent of samples lie below one eighth of the frame maximum. Under today's full-range rule that slice is drawn almost entirely in the bottom eighth of the grey range, and each slice gets a different range as the slope changes.

Synthetic reproducer, which AC1 makes permanent: 200 samples, 100 of stored 0, stored 1 to 99, and one stored 1000, slope 0.5. Today's full range is centre 250 and width 500, and stored 50 (25 Bq/ml) draws grey 13. Under this story it draws grey 128.

```bash
bin/ocelli.sh test ocelli-pixel --test auto_window
bin/ocelli.sh test ocelli-folder-viewer
```

## Scope, API and compatibility

### Library, `crates/ocelli-pixel/src/auto_window.rs`

```rust
use alloc::vec::Vec;
use ocelli_core::Stored;
use crate::{ModalityTransform, PixelError, VoiFunction, VoiTransform};

/// Samples read over one pool. Split equally over the declared frames.
pub const AUTO_WINDOW_SAMPLE_CAP: usize = 1 << 20;
/// Frames sampled from one series.
pub const AUTO_WINDOW_SERIES_FRAMES: usize = 16;

/// Indices of the sampled images of a series of `images`, first and last included.
pub fn series_sample(images: usize) -> Result<Vec<usize>, AutoWindowError>;

/// Inclusive stored-value range left out of the window, PS3.3 C.7.5.1.1.2.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct StoredRange { pub low: Stored, pub high: Stored }

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AutoWindowBasis { Percentile, Constant, Background }

/// A LINEAR_EXACT window from the 1st and 99th nearest-rank percentiles.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AutoWindow {
    pub center: f32,
    pub width: f32,
    pub low: f32,
    pub high: f32,
    /// Counted values over the pool, after stride and exclusion.
    pub counted: usize,
    /// Frames added.
    pub frames: usize,
    pub basis: AutoWindowBasis,
}

impl AutoWindow {
    /// `VoiTransform::new(None, &[center], &[width], 0, VoiFunction::LinearExact, ymin, ymax)`.
    pub fn transform(&self, ymin: f32, ymax: f32) -> Result<VoiTransform, PixelError>;
}

/// One pool over up to `frames` frames, each with its own modality stage.
pub struct AutoWindowPool { /* values: Vec<f32>, left-out maximum, declared, added */ }

impl AutoWindowPool {
    /// `frames` from 1 to `AUTO_WINDOW_SERIES_FRAMES`.
    pub fn new(frames: usize) -> Result<Self, AutoWindowError>;
    /// Fold one frame. Its stride is `len.div_ceil(AUTO_WINDOW_SAMPLE_CAP / frames)`.
    pub fn add_frame(
        &mut self,
        stored: &[Stored],
        modality: &ModalityTransform,
        padding: Option<StoredRange>,
    ) -> Result<(), AutoWindowError>;
    pub fn finish(self) -> Result<AutoWindow, AutoWindowError>;
}

/// A pool of one frame. The per-image window.
pub fn auto_window(
    stored: &[Stored],
    modality: &ModalityTransform,
    padding: Option<StoredRange>,
) -> Result<AutoWindow, AutoWindowError>;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AutoWindowError {
    EmptySeries, FrameCount, TooManyFrames, EmptyFrame, EmptyPool,
    NonFiniteModality, InvalidPadding, NonFiniteWindow,
}
```

`EmptySeries` is `series_sample(0)`. `FrameCount` is `new(0)` or `new(17)`. `TooManyFrames` is an `add_frame` beyond the declared count. `EmptyFrame` is an empty `stored`. `EmptyPool` is `finish` with no frame added. A failed `add_frame` leaves the pool as it was, so the caller can skip that frame and continue, which is item 12's rule. `finish` with fewer frames added than declared is allowed, because skipped frames are the normal mixed-series case, and the budgets stay those of the declared count so the cap still holds.

`lib.rs` adds `pub mod auto_window` and re-exports its public items. `PixelError` is not extended, because `crates/ocelli-render/src/cpu.rs:970` matches it exhaustively. The crate stays `no_std` with `alloc`. `slice::select_nth_unstable_by` and `f32::total_cmp` are in `core`.

### Folder viewer

- `preview.rs` (after F-216, F-210 and F-213): `FileWindow::Full` becomes `FileWindow::Auto`. `HeldFrame::full_range` becomes `HeldFrame::auto: Result<AutoWindow, PreviewError>`, the per-image window, filled once in `file_window_of`. `full_range` the function is deleted. `FrameRequest` gains `series_window: Option<(f32, f32)>`. In the file-window state of an eligible image, the series window when present, else `HeldFrame::auto`. `Preview::full_range` becomes `Preview::auto: AutoSource`, with `AutoSource::{None, Series, Image}`. A series window together with a preset, a numeric override or a file window index is the `window` refusal, the same rule as preset against numeric today. On an image that is not eligible the series window is not applied and the reply says `none`.
- `fn series_window(viewer: &Viewer, images: &[(FileId, u32)]) -> Result<SeriesWindow, PreviewError>` in a new `examples/folder-viewer/src/series_window.rs`: `series_sample(images.len())`, then for each sampled image `Viewer::frame(id, frame)` (F-216, decoding under a permit when not held), eligibility from the held recipe, `add_frame` with that frame's unpacked samples, its own `ModalityTransform` and its own padding from `HeldFrame::value`, then `finish`. `SeriesWindow { window: AutoWindow, sampled: usize, skipped: usize }`. A gone id is F-216's 410 and nothing is returned.
- `serve.rs`: `POST /series-window`, body one line per series image in page order, `<file id> <frame>`, no Part 10 bytes, at most 65,536 lines. Reply 200 JSON `{"center":"25","width":"49","function":"LINEAR_EXACT","basis":"percentile","sampled":16,"skipped":0,"counted":262144}`, centre and width as Rust `f32` `Display` strings. Refusals: 410 `file-gone` with the gone id, 400 `header` for a malformed line, 400 `window` when nothing contributed. `X-Ocelli-Series-Window` is parsed with the existing `once` header rule. The reply header `X-Ocelli-Full-Range` becomes `X-Ocelli-Auto-Window: series`, `image` or `none`.
- `page.html`: a per-open `Map` from series key to `{ center, width, sampled, skipped }` holding the reply's strings. On showing a series whose first shown image is eligible by its inspect `windowCenter` and `windowWidth` being empty, the page awaits `/series-window` before the first `/preview` of the main view, through one shared in-flight promise per series. Every main-view `/preview` in the file-window state sends `X-Ocelli-Series-Window`. The bottom right window line appends `  Auto window` for `series` and `  Auto window, this image` for `image`. Reset returns to the file-window state, which is the series window. F-213's window list shows one `Auto window` row on an eligible image, which selects the same state. `full range` disappears from the page.
- `tools/bench/src/viewer_work.mjs`: one more synthetic workload, `pet_no_window`, 267 files of 128 by 128, signed 16-bit, no window, Rescale Slope varying per file, stored 0 outside a disc, written by the existing writer into the ignored `tools/bench/out/`. `viewer.first_image` and `viewer.open` report it as a separate informational row.

Non-goals: a series window for a VOI LUT Sequence series, a series window in thumbnails, any change to `open_frame`, the oracle, `render-params.json` or `voi.mjs`, a wasm or `@ocelli/core` export, a histogram view, and any GPU path.

Compatibility: the header rename is a breaking change to the local protocol, whose only consumer is `page.html`. The two pinned fixture names change to say auto, with the same grey 128. Every image with a file window keeps its bytes (AC12).

## Consumer and provider map

| Consumer / entry point | Concrete provider | Caller context / policy | Observable output |
|------------------------|-------------------|-------------------------|-------------------|
| Page, first view of an eligible series | `POST /series-window`, then `series_window`, `series_sample`, `AutoWindowPool` | Before the first main-view paint, once per series per open | Cached strings, `sampled`, `skipped` |
| Main view, file-window state | `/preview` with `X-Ocelli-Series-Window`, then the one `LutChain` | Every slice of the series | Grey, `X-Ocelli-Auto-Window: series` |
| Thumbnails, tiles before the series window, direct callers | `HeldFrame::auto`, `auto_window` | Per image | Grey, `X-Ocelli-Auto-Window: image` |
| Preset, index, override, region | Unchanged F-207 and F-213 paths | Series header refused alongside | `none` |
| Page corner | Reply header | F-212 corner, F-213 word slot | `Auto window` or `Auto window, this image` |
| F-209 sync | Reset copied as Reset | Each target its own series | Its own series window |
| F-210 value at the pointer | Unchanged | Not affected | Unchanged |
| Oracle reference and candidate | `render-params.json`, sidecar `voi` | Unchanged | Unchanged verdicts |
| `ocelli_dicom::open_frame` | Unchanged | The caller chooses | Unchanged |

Owner of frame selection, the cap split, per-frame modality, exclusion, ranks and the window: `ocelli-pixel` alone. Owner of the series list and its order: the page's `groupSeries`. Owner of the cached strings: the page, without arithmetic. Owner of the grey: the one `LutChain`.

## State transitions and invariants

| Entry point / starting state | Input or event | Resulting state and output | Refusal and preserved state |
|------------------------------|----------------|----------------------------|-----------------------------|
| Series never shown this open, first image eligible | Select the series | `/series-window`, entry cached, first image drawn with it, corner `Auto window` | `window` refusal: the page uses the per-image window and the corner says `Auto window, this image`. 410: F-216 re-post once, then retry once, then the fault sentence |
| Series window cached | Slice change | Same centre and width, same corner text | That slice's own refusal, previous image stays |
| Series window shown | Preset, index, region or drag | That window, `none` | A refused one leaves the series window shown, F-213's rule |
| Any window | Reset or the `Auto window` row | The cached series window again, no request to `/series-window` | None |
| Series A cached | Show series B, then A again | B computed once, A from its entry | None |
| Any | New open | Every entry cleared | None |
| Series window cached | F-216 eviction or process restart | Entry kept, next `/preview` re-posts the file by F-216's rule | None |
| Image with its own window in a series with a series window | Show it | Its own window, `none` | None |

Invariants: an image with a window pair never reports `series` or `image`. A series window is a pure function of the sampled images' bytes and the series length. The page sends back exactly the strings it received.

## Ownership, commit and recovery

| Resource or visible state | Acquired or staged by | Lifetime identity | Observable commit point | Failure and duplicate-operation behavior |
|---------------------------|-----------------------|-------------------|-------------------------|------------------------------------------|
| Sampled held frames | `series_window` through `Viewer::frame` | F-216 `HeldKey::Frame` | `Lru::insert` | Decode refusal skips that frame. Duplicate decode displaces with equal content |
| Pool scratch | `AutoWindowPool` | One call | `finish` | Dropped on return or refusal |
| Page series entry | The page, from a 200 reply | Series key within one open | Map insert, before the first paint | A refusal inserts nothing. Two concurrent requests for one series share one promise |

## Numbered acceptance inventory

Every value is hand-computed from the formulas above and was cross-checked by the plan author with an independent Python script using `numpy.float32` arithmetic, not the implementation. Grey is `floor(y + 0.5)` clamped to 255, which is `display_index` in `crates/ocelli-render/src/colour.rs:482`. Slope and intercept follow PS3.3 C.11.1 per frame, the window C.11.2.1.3.2.

| ID | Trigger and expected behavior | Test / observation | Wrong implementation rejected | Evidence stage / owner |
|----|-------------------------------|--------------------|-------------------------------|------------------------|
| AC1 | One PET-like frame through `auto_window`, 200 samples: 100 of stored 0, stored 1 to 99, stored 1000, slope 0.5. Counted 100, `low = 0.5`, `high = 49.5`, centre 25, width 49. Stored 20 is 49.44, grey 49. Stored 50 is grey 128. Stored 0, 1 are 0. Stored 99, 1000 are 255 | `fixture_pet_like_frame_leaves_out_background_zeros` | Counting zeros gives centre 24.5 and stored 50 grey 130. Today's min and max give grey 13 | implementation |
| AC2 | **Pooled differs from per image.** Frame A stored 1 to 100, frame B stored 101 to 200, both slope 1, `new(2)`. Pooled: counted 200, ranks 2 and 198, `low = 2`, `high = 198`, centre 100, width 196. Frame A's stored 100 is grey 128 and frame B's stored 150 is 192.55, grey 193. Per image, A alone is centre 50, width 98 and its stored 100 is grey 255, B alone is centre 150, width 98 | `fixture_series_pool_is_one_window` | A per-image implementation gives A's stored 100 grey 255. Averaging the two per-image windows gives centre 100 and width 98 | implementation |
| AC3 | **Each frame through its own slope.** Frame A stored 1 to 100 slope 1, frame B stored 1 to 100 slope 2. Pool of modality values: ranks 2 and 198 of 200 are `low = 2` and `high = 196`, centre 99, width 194. A's stored 50 (50) is 63.09, grey 63. B's stored 50 (100) is 128.81, grey 129 | `fixture_series_pool_uses_each_frame_rescale` | Pooling stored values, or applying frame A's slope to both, gives centre 50, width 98. Applying frame B's slope to both gives centre 100, width 196 | implementation |
| AC4 | **Exclusion per frame.** Frame A, 50 of stored 0 and stored 1 to 50, no padding. Frame B, 20 of stored -2000 and stored 51 to 100, signed, padding `-2000` to `-2000` for B only. Counted 100, `low = 1`, `high = 99`, centre 50, width 98. Without B's padding: `low = -2000`, centre -950.5, width 2099. Stored 0 against modality 0, single frame: 50 of stored 0 and stored 1000 to 1099, slope 1, intercept -1024, gives `low = -24`, `high = 74`, centre 25, width 98, and stored 1049 grey 128 | `fixture_series_pool_excludes_per_frame`, `fixture_stored_zero_is_left_out_not_modality_zero` | Ignoring per-frame padding. Counting zeros. Excluding modality 0, which gives `low = -1024` | implementation |
| AC5 | **The cap split.** `new(2)`. Frame A has `2^19 + 1` samples, stored 60000 at odd indices and `(i / 2) mod 100 + 1` at even index `i`. Budget `2^20 / 2 = 2^19`, stride 2, 262,145 values read. Frame B stored 1 to 100. Pool 262,245, `low = 1`, `high = 99`, centre 50, width 98. A frame of `2^20` samples alone in `new(1)` is read exactly | `fixture_cap_is_split_across_frames` | Giving each frame the whole cap reads A exactly and gives `high = 60000`. Starting at index 1 | implementation |
| AC6 | **Which frames.** `series_sample(1) = [0]`, `(2) = [0, 1]`, `(16) = 0 to 15`, `(17) = 0 to 14 and 16`, `(267) = [0, 17, 35, 53, 70, 88, 106, 124, 141, 159, 177, 195, 212, 230, 248, 266]`, `(175) = [0, 11, 23, 34, 46, 58, 69, 81, 92, 104, 116, 127, 139, 150, 162, 174]`. `(0)` is `EmptySeries` | `fixture_series_sample_spacing` | Rounding instead of flooring. Omitting the last image. A random or content sample | implementation |
| AC7 | Nearest rank, single frame. AC1 under linear interpolation would give grey 115, nearest rank 128. Stored 1 to 150: ranks 2 and 149, centre 75.5, width 147, stored 75 grey 127 | `fixture_nearest_rank_is_exact`, `fixture_ranks_round_up` | Interpolation. Floor ranks give 1 and 148. Zero-based indexing gives 3 and 150 | implementation |
| AC8 | Edge cases. 16 of stored 7: `Constant`, centre 7, width 1, grey 128. 16 of stored 0: `Background`, centre 0.5, width 1, grey 0. A pool of one all-zero frame and one all-padding frame (-2000): `Background`, every grey 0. Stored -50 to 49 signed: centre -0.5, width 99, stored -1 grey 126, stored 0 grey 129, stored 1 grey 131. Stored 3 and 9: centre 6, width 6, greys 0, 128, 255 | `fixture_constant_background_signed_and_two_values` | Width 0. Mid grey for a blank PET end slice. Unsigned reading of signed data | implementation |
| AC9 | Refusals and pool rules. `new(0)` and `new(17)` are `FrameCount`. A third `add_frame` on `new(2)` is `TooManyFrames`. `finish` with none added is `EmptyPool`. Slope `3.0e38` on stored 2 is `NonFiniteModality` and leaves the pool unchanged, so a following valid frame and `finish` give that frame's window. A NaN padding bound is `InvalidPadding`. `AutoWindow::transform(0.0, 255.0)` of AC1 equals `VoiTransform::new(None, &[25.0], &[49.0], 0, VoiFunction::LinearExact, 0.0, 255.0)` | `auto_window_pool_refusals` | A failed frame that poisons the pool. A transform built as LINEAR | implementation |
| AC10 | Viewer, pinned inputs renamed. One stored 50, no window, no series header: grey 128, `X-Ocelli-Auto-Window: image`. A zero-length Window Center and Width: the same. A windowed frame: `none` | `a_missing_window_is_labelled_auto`, `a_zero_length_window_is_auto`, `the_page_posts_a_frame_and_returns_its_grey_byte` with its header line changed | A label set on a file that has a window | implementation |
| AC11 | `/series-window` over held synthetic Part 10 files built from AC3's two frames, each with its own Rescale Slope, Modality `PT`, no window: 200, centre `99`, width `194`, sampled 2, skipped 0. Then `/preview` of frame B's file with `X-Ocelli-Series-Window: 99 194` gives grey 129 at stored 50 and `series` | `the_series_window_pools_each_file_rescale` | Pooling through one file's slope. A route that reads the page's numbers into arithmetic of its own | implementation |
| AC12 | Any window in the file is used as it is. Every existing windowed fixture keeps its bytes and `none`. A windowed file in a `/series-window` list is skipped and counted. A windowed file previewed with a series header keeps its own window and replies `none`. A half window and a VOI LUT Sequence keep the `window` refusal | Existing fixtures, `a_windowed_image_ignores_the_series_window`, `a_half_window_is_not_auto` | Applying the series window over a file window | implementation |
| AC13 | Mixed and padded series. A list of three files where the middle one has a window: sampled 3, skipped 1, the window of the other two. A list where every file has a window: 400 `window`. A signed CT-like file with Pixel Padding Value -2000, the AC4 frame B samples, alone: 50 counted, ranks 1 and 50, `low = 51`, `high = 100`, centre 75.5, width 49 | `a_mixed_series_skips_windowed_images`, `a_padded_file_leaves_out_its_padding` | Replacing a skipped frame with a neighbour. Ignoring F-210's padding | implementation |
| AC14 | Request rules. A series header with a preset, a numeric override or a file index is 400 `window`. A non-finite or non-positive width in the header is 400 `window`. A gone id in the list is 410 `file-gone`. A malformed line is 400 `header`. A series window computed once is reused, a `cfg(test)` counter shows one pool per `/series-window` request and none per `/preview` | `the_series_window_refuses_conflicts`, `the_series_window_is_not_recomputed_per_preview` | Silently preferring one window. Pooling on every preview | implementation |
| AC15 | The page. A no-window series shows `Auto window` with the reply's `WW` and `WL`. Every slice of that series shows the same `WW` and `WL`. A preset or drag removes the word, Reset and F-213's `Auto window` row restore it with no new `/series-window` request. A refused `/series-window` gives `Auto window, this image`. The page source has no `full range` and no arithmetic on the series strings | String test extending `the_page_offers_a_folder_picker`, and a browser click-through on the synthetic `pet_no_window` workload at sprint close counting `/series-window` requests per series | A per-slice window. A page that parses the strings into numbers | implementation, then integrated |
| AC16 | Browser evidence on the operator's study, release build, Chromium. For each of the five PT series and the MG Secondary Capture series, the corner reads `Auto window`, the `WW` and `WL` are identical on the first, middle and last image and while scrolling with ArrowDown, and equal the centre and width an independent script computes from that series with pydicom: the `series_sample` indices over the page's order, each file's own slope, stored 0 and padding left out, the cap split, nearest rank, `float32`. A windowed CT and MR show no `Auto window` | Operator or lead on the operator's Mac. Screenshots and numbers kept outside git, only pass or fail and series counts recorded | A browser path that differs from the fixtures | external, owner sprint lead, prerequisite F-213 and this story integrated |
| AC17 | Cost. F-216's `viewer.open` and `viewer.first_image` on F-216's workloads still meet F-216's answered thresholds (open of 1,693 by 512 at most 1,100 ms, first image at most 20 ms on the CT series and 250 ms on the 17 MB DX). Those workloads carry Window Center 40 (`tools/bench/src/viewer_work.mjs`), so they never request a series window. The new informational row `pet_no_window` reports first image including `/series-window`, median of 9, release build, F-216's host, with no threshold, because the operator accepted the cost. Expected, stated before measurement: 16 decodes of a 128 by 128 frame from held bytes and one selection over 262,144 values, under 30 ms on that host | `bin/ocelli.sh bench viewer.open viewer.first_image` with the run record | A regression on the windowed workloads. A cost claimed without a run | external, owner sprint lead, F-216's host |
| AC18 | The oracle is untouched. The story's diff has no file under `tools/oracle`, and `bin/ocelli.sh gate oracle` verdicts are unchanged | Diff review, oracle gate in sprint verification | Changing the reference default to match the viewer | implementation, sprint verification |

Semantic RED, recorded in the progress note. AC1 to AC9 are also run against three deliberately wrong bodies, min and max, per image instead of pooled, and one slope for the pool, and each named case fails. AC10 and AC11 run against today's `full_range` and fail on the label and the AC11 grey. Mutations after GREEN, each run and seen red: count stored 0 (AC1), per-image windows (AC2, AC11, AC15), one slope for the pool (AC3, AC11), drop per-frame padding (AC4, AC13), whole cap per frame (AC5), round the sample indices (AC6), floor the ranks (AC7), width 0 for a constant (AC8), a failed frame that poisons the pool (AC9), apply the series window over a file window (AC12), and pool per preview (AC14).

## Approach

1. `crates/ocelli-pixel/src/auto_window.rs`: `series_sample`, then `AutoWindowPool`. `add_frame` validates padding and the frame, computes the stride from the declared count, walks `stored.iter().step_by(stride)`, applies that frame's modality stage, refuses a non-finite value without changing the pool by staging into a local length mark and truncating on refusal, pushes counted values and tracks the left-out maximum. `finish` selects the two order statistics. Stored 0 is tested on the bits with the sign masked, `sample.0.to_bits() & 0x7fff_ffff == 0`, because the workspace denies `clippy::float_cmp`. `auto_window` is `new(1)`, `add_frame`, `finish`.
2. `crates/ocelli-pixel/tests/auto_window.rs`: AC1 to AC9, each with a doc comment citing PS3.3 C.11.1, C.11.2.1.3.2, C.7.5.1.1.2, C.7.6.3.1 or C.7.6.16.2.9 and the operator's answers of 2026-11-05.
3. Folder viewer, after F-213 integrates: `series_window.rs`, the route, the request header and its refusals, `FileWindow::Auto`, the reply header, the page cache and labels, the F-213 row, and AC10 to AC15.
4. The `pet_no_window` workload and the AC17 run.
5. Documentation and LLD.

Rejected alternatives: a per-image window (the operator chose per series), a provisional per-image window then a switch (item 15), the server holding series state or a series token (stale state the page cache avoids), the server grouping series (it holds files, not series), a percentile from F-075's binned histogram, linear interpolation, excluding modality 0, and replacing a skipped sample with a neighbour.

## Dependencies, write ownership and resources

Declared dependencies F-210 and F-213, both pending. Source-level dependencies: F-216's held frame, `Viewer::frame`, 410 rule and bench runners (in progress), F-210's `HeldFrame::value` padding range, F-213's window state and list, F-212's corner line, F-209's sync of Reset, F-208's tile mode.

**Two halves, two positions in the order.** The library half writes only `crates/ocelli-pixel/src/auto_window.rs`, `crates/ocelli-pixel/src/lib.rs` and `crates/ocelli-pixel/tests/auto_window.rs`, which no other S46 story writes, so it may start now and run in parallel. The viewer half writes files every viewer story writes, plus `tools/bench/src/viewer_work.mjs`, which F-216 and F-223 also write. It is serialised **after F-213 and before F-214**: F-217, F-216, F-210, F-211, F-208, F-209, F-212, F-213, F-227, F-214, F-215, then F-228's viewer half. F-223 may run before or after it, but not concurrently, because of `viewer_work.mjs`.

Write set: `crates/ocelli-pixel/src/auto_window.rs` (new), `crates/ocelli-pixel/src/lib.rs`, `crates/ocelli-pixel/tests/auto_window.rs` (new), `examples/folder-viewer/src/series_window.rs` (new), `examples/folder-viewer/src/lib.rs`, `examples/folder-viewer/src/preview.rs`, `examples/folder-viewer/src/held.rs`, `examples/folder-viewer/src/serve.rs`, `examples/folder-viewer/src/page.html`, `examples/folder-viewer/tests/preview.rs`, `tools/bench/src/viewer_work.mjs`, `tools/bench/tests/viewer_work_test.mjs`, `README.md`, `examples/folder-viewer/src/use.html`, `examples/folder-viewer/src/more.html`, `docs/lld/pixel-pipeline.md`, `docs/lld/benchmarks.md`.

Resources: AC16 needs the operator's study and Mac. AC17 needs F-216's host otherwise idle. Never ports 8742 or 8765. The viewer runs with `OCELLI_PORT` set to a port just bound and released.

## Verification and evidence reuse

```bash
bin/ocelli.sh check ocelli-pixel
bin/ocelli.sh test ocelli-pixel --test auto_window
bin/ocelli.sh test ocelli-pixel
bin/ocelli.sh clippy ocelli-pixel
bin/ocelli.sh test ocelli-folder-viewer
bin/ocelli.sh clippy ocelli-folder-viewer
bin/ocelli.sh gate nostd
bin/ocelli.sh gate bench
bin/ocelli.sh native
bin/ocelli.sh bench viewer.open viewer.first_image
```

Final profile `bin/ocelli.sh gate --sprint`, which includes `oracle` for AC18. Evidence is invalidated by any change to `auto_window.rs`, `ModalityTransform::apply`, `VoiTransform::new`, `display_index`, `series_window.rs`, `file_window_of`, the page's series cache or window line, `groupSeries`, or the bench workload writer. AC17's row is a measurement with no threshold for `pet_no_window` and F-216's thresholds for its own workloads. A missing run is a missing observation.

## Boundary and tier

- wasm-bindgen: not touched
- Pixels across the boundary: no. The page receives grey bytes and two strings from its local process, as F-206 and F-216 accepted
- Render-loop allocation: none. One pool scratch of at most `AUTO_WINDOW_SAMPLE_CAP` values per `/series-window` request and per held frame's per-image window
- unsafe: none
- Tier A (WebGPU): n/a in this story. No GPU consumer calls the pool. A later consumer passes `AutoWindow::transform` into the chain and `VoiParams::from_chain` reads the same centre and width, so the window is the same on every tier. The HLD's tier A histogram pass is not needed because the window is computed once on the host
- Tier B (WebGL2): n/a, the same reason
- Tier C (CPU): full. The local process computes the window and runs the one CPU `LutChain`

## Tests

| Category | What it proves | Where |
|----------|----------------|-------|
| fixture | AC1 to AC8, hand-computed pooled and single-frame windows and greys, per-frame rescale and exclusion, the cap split and the frame indices, citing PS3.3 C.11.1, C.11.2.1.3.2, C.7.5.1.1.2, C.7.6.3.1 and C.7.6.16.2.9 | `crates/ocelli-pixel/tests/auto_window.rs` |
| unit | AC9 refusals, pool recovery and the transform equality | `crates/ocelli-pixel/tests/auto_window.rs` |
| fixture | AC10, AC11 and AC13 through the route and `preview_frame`, same citations | `examples/folder-viewer/tests/preview.rs` |
| contract | AC12 file windows untouched, AC14 request rules and reuse, AC15 page strings | `examples/folder-viewer/tests/preview.rs` |
| browser | AC15 click-through, AC16 on the operator's study | sprint close, outside git |
| benchmark | AC17 | `tools/bench`, F-216's runners |

## Parity surface covered

None. `docs/hld/B-parity-surface.md` row "VOI LUT functions, 3" counts functions, and this story uses LINEAR_EXACT without changing it. The default window for a file with none is a viewer policy, and cornerstone3D 5.8.2's differs as transcribed above. No parity is claimed.

## Deviations

None. The HLD does not specify a default window, and the operator chose this one. The oracle keeps its declared defaults.

## LLD impact

- `docs/lld/pixel-pipeline.md`, "VOI stage": a new subsection "The automatic window" stating the series sample, the cap split, per-frame rescale and exclusion, nearest rank, LINEAR_EXACT, the three bases, the refusals and pool recovery, that `open_frame` still invents nothing, and that the oracle injects its own declared window.
- `docs/lld/benchmarks.md`: the `pet_no_window` workload and its informational row.

## User-facing documentation impact

- `README.md`, "Start the local series viewer": one sentence in the refusal paragraph. A series whose images carry no window is drawn with one automatic window for the whole series, from the 1st to 99th percentile of modality values over up to 16 images spread through it, leaving out stored zero and the padding value, and the corner says Auto window. Opening such a series decodes those images first. A window in the file is always used as it is. Step 5 names the `Auto window` row.
- `examples/folder-viewer/src/use.html`, chapter 9 "The LUT chain": the `open_frame` example keeps its comment that Ocelli does not guess, and a new example builds an `AutoWindowPool` over `series_sample(n)` frames, each with its own `opened.modality()` and padding, then the chain from `AutoWindow::transform(0.0, 255.0)`, with one sentence that this is the caller's labelled choice.
- `examples/folder-viewer/src/more.html`: a new feature section `#auto-window`, status built, stating the folder viewer's series rule, the per-image fallback in thumbnails, where the function lives, and that it is not a cornerstone3D parity row.

## Open questions

None open. The rule is the operator's two answers quoted at the top.

- Q1, stored 0 or modality 0 or padding. Answered: lead decision under the operator's delegation of 2026-11-05. Stored 0 and each frame's declared padding range, item 3, AC4.
- Q2, nearest rank or interpolated. Answered: lead decision under the operator's delegation of 2026-11-05. Nearest rank, item 5, AC7.
- Q3, cap and sampling. Answered: lead decision under the operator's delegation of 2026-11-05. `2^20` in total, split equally over the declared frames, a stride fixed by frame size, items 6 and 7, AC5 and AC6.
- Q4, which modalities. Answered: lead decision under the operator's delegation of 2026-11-05. Every greyscale image with no window pair and no VOI LUT Sequence, item 10.
- Q5, blank and constant frames. Answered: lead decision under the operator's delegation of 2026-11-05. Background black, constant mid grey, item 9, AC8.
- Q6, per image or per series. Answered: operator, 2026-11-05, per series. Up to 16 frames spread through the series, first and last included, item 11.
- Q7, before the first paint or provisional. Answered: lead decision under the operator's delegation of 2026-11-05. Before the first paint, item 15, with the cost measured by AC17.
- Q8, mixed series and invalidation. Answered: lead decision under the operator's delegation of 2026-11-05. Skip without replacement, per-image fallback when nothing contributes, and the cache cleared only by a new open, items 12 and 18.
- Q9, position in the viewer order. Answered: lead decision under the operator's delegation of 2026-11-05. Library half now and in parallel, viewer half after F-213 and before F-214.
