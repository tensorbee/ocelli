# F-239 server review, pass 1

**Reviewed:** staged tree `3f1f607d4d526c6b7f63292e6a88afaedba2c584`,
baseline `933e84f7cadedd9a0afbdfa54ec4fa947790a188`, HEAD `055b8633`.
Executable production was frozen at `33a823f4`. The subsequent `7dedcd92`
added a preview control and `3f1f607d` changed comments. Their explicit deltas
were read and composed into an immutable review copy.

**Author / reviewer:** Codex participant `/root/glm_codec_merge` authored the
ten server/accessor/preview-test paths. Reviewer `/root/f244_review` authored
none of them. The untracked GLM `tests/colour.rs` was excluded from all these
source identities. The reviewer's earlier `tests/browser/colour.mjs` is also
excluded from this review and its independence claim.

**Result:** 3 defects, 0 smells, 0 nitpicks.

## Acceptance challenged

AC1 to AC13, AC19, AC21 and AC22 at the native server boundary, including
unchanged cache, decode, geometry and codec callers. Synthetic inputs came
from an independent pydicom generator and the literal PS3.3 2026d matrix,
planar and palette rules. Expected 4:2:2 bytes were calculated by an exact
Fraction inverse of the standard's stated forward matrix. Sample masking and
scaling patterns were computed independently in Python. No renderer output
seeded those fixtures.

Primary sections read: PS3.3 2026d C.7.6.3.1.2, C.7.6.3.1.3,
C.7.6.3.1.5, C.7.6.3.1.6, C.7.9, C.11.2.1.2 and C.11.6. The
[Image Pixel Module](https://dicom.nema.org/medical/dicom/current/output/chtml/part03/sect_C.7.6.3.html)
defines the layouts and palette inputs. The
[LINEAR section](https://dicom.nema.org/medical/dicom/current/output/chtml/part03/sect_C.11.2.html)
provides the formula reused by the approved per-channel parity policy.
File VOI is defined for monochrome by the standard, while F239's colour
window is the explicitly approved cornerstone parity behavior.

The immutable review workspace is
`/private/tmp/ocelli-f239-server-review-p1`. All ten authored paths were
checked against the frozen tree, byte-identical or an exact production prefix
followed only by appended reviewer controls. The source inventory is
`/private/tmp/f239-server-review-source.json`. Review instrumentation and
mutations stayed in that copy. No author source was remediated by this reviewer.

## Evidence

All receipts below are new commands under `/private/tmp/ocelli-s46-codex`,
with exact arguments, direct process exits and logs. Native Cargo work used
two jobs and `/private/tmp/f239-review-target`. Historical author receipts
were read but were not counted as commands executed by this reviewer.

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|--------------------------|---------------------------|------------------------------|----------------------|
| `test ocelli-folder-viewer --lib f239_review` | Frozen executable source plus independent palette admission controls | Now, `f239-server-review-admission` | 101. Actual charge 6824, claim 840. Pinned A becomes 410 after B is admitted |
| `test ocelli-folder-viewer --test review_colour` | Independent synthetic inputs, source restored after controlled mutations | Now, `f239-server-review-public-restored-ten` | 0, ten selected tests pass |
| `--lib f239_review_byte_gate` | Actual render admission beside an existing one-byte claim | Now, `f239-server-review-three-byte-restored` | 0, colour waits then valid retry renders |
| `--test review_colour roi_has_one_containment` | Review-only counter inside the actual geometry provider | Now, `f239-server-review-roi-onewalk-restored` | 0, two bounding-box pixels cause two containment calls total |
| Duplicate the colour containment call | Same counted consumer | Now, `f239-server-review-roi-second-walk-mutant` | 101, four calls rather than two |
| Remove the channel multiplier from `grey_bytes` | Same actual admission consumer | Now, `f239-server-review-three-byte-mutant` | 101, render exceeds admitted bytes |
| Claimed over-twice scratch release | Large planar frame followed by two-pixel RGB using one scratch owner | Now, `f239-server-review-band-retention` | 101, 1966050 bytes retained before and after |
| Actual bounded retention and accounting | 16- and 32-bit planar containers, each with 12 stored bits | Now, `f239-server-review-retained-bound` | 0. 1966050 and 2359260 bytes retained and fully counted |

The public ten-test selection covers packed 4:2:2 across rows, masked 12-bit
planar samples crossing a 65536-pixel band boundary, big-endian planar input,
32-bit containers with 12 stored bits, codec-reported RGB, RLE interleaved
output, YBR scaling before conversion, raw RGB point values and population ROI
statistics, named refusals, independent length calibration, three native
frames, colour windows, inversion ties and valid recovery.

JPEG's consumer test asserts the approved AC9 property that the viewer retains
the registered decoder's reported RGB bytes. It does not establish independent
JPEG decoder arithmetic. Other pixel expectations are literal standard/plan
values or independent pattern calculations.

Failed setup/control attempts are retained separately. The first fixture
inspection used pydicom's default YBR conversion rather than `raw=True`. It was
corrected before controls ran. The first public review test omitted the frame
argument of `inspect_json` and did not compile. The first containment counter
used `std` in the no-std geometry crate and did not compile. Corrected controls
passed. None of those compiler/setup failures is mutation sensitivity.

## Defects

### D1, palette admission reserves less than the frame owns

**Where:** `examples/folder-viewer/src/held.rs:1689`, `frame_sample_cost` and
its claim/reuse callers. The new exact final charge is in
`preview.rs`, `HeldFrame::charged_bytes`.

**What:** the predecode claim still accounts for monochrome value facts and
unit storage only. It does not make room for the new boxed colour recipe and
the palette's owned lookup vectors. A two-pixel frame with three standard
256-entry, 16-bit palette tables costs 6824 bytes but its claim is 840.

A real cache lifecycle control holds palette A through an active read, admits
palette B with budget `2 * A_frame_charge + B_file_capacity - 1`, and previews
A again. B succeeds, but A answers 410 instead of 200 from `held`. Final
charging at store time is too late to preserve the earlier admission invariant.

**Why it is wrong:** F239 AC15 and its ownership contract require actual
palette and recipe allocations to enter admission. F216's existing cache
protocol protects a frame that a response is reading. The old scalar-facts
regression already states why reserving only the later final charge is
insufficient.

**Scope and regression:** every palette large enough to exceed the scalar
allowance can trigger this on first decode or reused-buffer admission. Keep
both the predecode-bound check and pinned-A/admit-B lifecycle check. The
independent fixture uses two indices, 0 and 255, red `i * 257`, green
`(255 - i) * 257`, and blue `128 * 257`, with no patient attributes.

The lead accepted D1 and assigned the author a repair. This pass does not
approve that remediation.

### D2, the plan promises colour scratch trimming that is absent

**Where:** `.claude/plans/F-239-design.md:372`, colour scratch ownership row.
`colour.rs:117` owns the vectors, and `serve.rs` trims the grey reply only.

**What:** the approved row says a band larger than a later frame is freed by
the existing over-twice rule. Rendering a 257 by 259 planar frame and then a
2 by 1 RGB frame through the same scratch retains 1966050 bytes throughout.
The buffers are bounded and their actual capacities are counted. They do not
shrink as the plan claims.

**Scope and regression:** this is a false memory-policy claim, not an
unbounded-allocation finding. The lead chose retained, bounded storage per
conversion workspace and a factual plan correction, with no trimming
optimization, tolerance or byte-budget change. The replacement positive
control verifies retention, band bounds and `workspace_bytes` accounting.
A 32-bit container with 12 stored bits is accepted and needs more packed
scratch than a 16-bit container, so a universal 1.875 MiB claim would also be
incorrect. The tested 32-bit shape retains 2359260 bytes.

### D3, public field comments still give monochrome semantics for colour

**Where:** `preview.rs:161`, `:175` and `:224`.

**What:** `Preview.inverted` says the display bytes were flipped after the
chain. For the approved tie fixture, the actual non-inverted RGB byte is 128
and the inverted byte is 126. Flipping byte 128 would give 127. The code is
correct and the field comment is false for colour. `window_function` says an
absent field keeps the file function, even though colour uses LINEAR and
ignores the synthetic file's SIGMOID. `window_count` is described as the count
of file pairs even though colour deliberately reports 0 with file pairs present.

**Scope and regression:** document the colour exceptions on the public fields,
not just on the enclosing struct. The independent inverse-tie and ignored-file
VOI fixtures confirm the behavior that the comments need to describe. No
arithmetic change is requested.

## Smells

None reported.

## Nitpicks

None reported.

## Verified clean

The native output-length path distinguishes two stored samples per native
4:2:2 pixel from three decoded RGB components. RLE decoder layout overrides
the header before physical unpacking, and a JPEG RGB report avoids a second
YBR conversion. Multi-row packed input and later planar bands produce the
independently derived bytes, including unused high-bit masking, both byte
orders and a wider container. The colour stage runs before the parity window,
and point and ROI values remain before that window and inversion. ROI integer
statistics and the one-walk count agree with AC13.

The three-byte reply charge is used by the actual admission caller. A colour
reply waits alongside another claim and recovers after that claim drops.
Its controlled multiplier mutation is detected. Palette final charges include
both owned vectors in each LUT descriptor. All three conversion scratch
capacities enter workspace accounting. Those checks do not excuse D1's earlier
admission gap.

Named refusals do not echo an arbitrary photometric value. Length calibration
remains available when a palette recipe is refused. Native multiframe selection
and following valid requests recover as specified.

## Limitations and handoff

This is a bounded server review, not approval of the full viewer story or the
reviewer's earlier browser harness. No browser, GPU, oracle, full workspace
sweep or feature/sprint profile was run in this pass. Page, README and Use
integration are the root's separate work. The author reports one old page
sentence test awaits that union, which is not attributed to the new server.
The GLM test file remains excluded and needs its own source pin and review.

The native JPEG/HTJ2K codec implementations and the earlier library slice were
not reopened as whole reviews. Their server composition and the new heap
accessors were checked. Full RSS and AC29 to AC32 measurements remain external.

Source freeze5 `b5ae6518` is a new repair tree and is not covered by this pass.
It needs a separate bounded independent repair review. The initial RED
receipts, fixture generator, test sources and source inventory remain under
`/private/tmp`. No synthetic DICOM belongs in git.
