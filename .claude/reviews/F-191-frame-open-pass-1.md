# F-191 review, pass 1

**Reviewed**: uncommitted F-191 tree on `sprint/s42` at `c6eec1f`, before the rescale, padding and descriptor remediation
**Author / reviewer**: implementation by the sprint host. Review by Claude Code, read only, with no shell. A second independent reader was still running when this record was written.
**Result**: 5 defects, 4 smells, 5 nitpicks

## Acceptance challenged

AC1 through AC4 were checked against the existing constructors. AC5's MONOCHROME1 fill was checked against `LutChain`, which inverts before the fill. AC8's creator record was checked against the request to resolve `(0009,1001)`. The rescale identity used by the folder page was checked against `ModalityTransform::new`.

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|--------------------------|---------------------------|------------------------------|----------------------|
| Read of `frame_open.rs`, `lut.rs`, `preview.rs` | Working tree before remediation | Executed by Claude, read only | Defects below |
| LINEAR_EXACT arm changed to `Linear` | `stored_50_maps` | Prior receipt, reverted | Exit 101, `left: Linear right: LinearExact` |
| MONOCHROME1 fill arm changed to `ymin` | `monochrome1_padding` | Prior receipt, reverted | Exit 101, bits 0 versus 1132396544. That only pinned the old rule |

## Defects

### D1, MONOCHROME1 padding is white after the chain

**Where**: `crates/ocelli-dicom/src/frame_open.rs` fill, at the time of review
**What**: `fill_padding` wrote `ymax` for MONOCHROME1 after `LutChain::map_into`.
**Why it is wrong**: `PresentationTransform` inverts MONOCHROME1 when no shape is declared, so black in the display buffer is `ymin`. PS3.3 C.11.6. `ymax` is white.
**Evidence**: `lut.rs` selects `Inverse` for MONOCHROME1 when evidence is absent. The test asserted `ymax`, so it pinned the wrong end.
**Scope and regression**: every MONOCHROME1 fill after the chain. The plan text said `ymax` and did not account for the chain's inverse.

### D2, absent rescale refuses the frame

**Where**: `modality_of` calling `ModalityTransform::new(None, None, None)`
**What**: a missing slope and intercept became `FrameOpenError::Chain`.
**Why it is wrong**: PS3.3 C.11.1 leaves those objects as the identity. The folder page uses slope 1 and intercept 0. The plan says a zero-length rescale is absent in that same sense.
**Evidence**: `ModalityTransform::new` returns `MissingRescale` when both options are empty and no LUT is present. Every fixture supplied a rescale, so the suite stayed green.
**Scope and regression**: MR, ultrasound, RGB and most objects without the pair.

### D3, VOI LUT second value uses Pixel Representation

**Where**: `window_of` passing pixel representation into the VOI descriptor
**What**: unsigned CT pixels with intercept -1024 read `0xFC00` as 64512.
**Why it is wrong**: PS3.3 C.11.2.1.1. The second value is SS when rescale output may be signed. It is US when a modality LUT is present. Pixel Representation applies to the modality descriptor, C.11.1.1.1.
**Evidence**: standard text for C.11.2.1.1. No fixture used a negative first input.
**Scope and regression**: a VOI LUT on a rescaled CT frame collapses to the first entry.

### D4, an SS LUT descriptor is refused

**Where**: `lut_descriptor` accepting only `Unsigned16`
**What**: Explicit VR SS for `(0028,3002)` became an attribute error.
**Why it is wrong**: the descriptor VR is US or SS. The first and third values stay unsigned. The explicit VR follows the second value.
**Evidence**: C.11.1.1.1 note on one VR for a multi-valued element. No SS fixture.
**Scope and regression**: signed modality and VOI descriptors in explicit VR.

### D5, private elements are not resolved

**Where**: `private_creators_of`
**What**: the API returned creator records and no lookup from `(gggg,xxee)` to that creator.
**Why it is wrong**: the plan says unknown private elements are listed with their creator, and AC8 says `(0009,1001)` resolves to `(0009,0010)`.
**Evidence**: the test asserted the creator string only. Deleting `(0009,1001)` left it green.
**Scope and regression**: callers cannot resolve a private element. The approved signature has no element list, so the lookup is the resolution.

## Smells

### S1, fill range is caller supplied

A 16-bit VOI LUT has an output range of 0 to 65535. A caller that passes 0 and 255 writes the wrong constant. The helper uses `output_range()`.

### S2, pixel-data skip is not discriminating

Deleting the tag skip still leaves the suite green. Replacing the pixel-sequence arm with an error also stays green, because no fixture has pixel data.

### S3, closed refusals are untested

Burned-in `MAYBE`, burned-in `NO`, a padding limit without a value, and `with_codec_rgb` on colour, twice, and on monochrome could change without a red test.

### S4, a non-text creator refuses the frame

An UN creator element failed `one_code` and refused the object. PS3.5 allows UN when the VR was not known. The design says private values are not interpreted.

## Nitpicks

Reserved odd groups 0001, 0003, 0005, 0007 and FFFF were accepted as private. Per-attribute functional-group precedence can mix levels on a malformed object. Posted `padding_count` counts ranges while the synthetic path counts samples, and the page labels them differently. CORS `*` is broader than the Vite proxy. `bin/example-dev.sh` ignores `CARGO_TARGET_DIR`.

## Verified clean

LINEAR and LINEAR_EXACT call `VoiTransform::new` and are separate arms. An absent window stays absent. Modality and VOI sequences win over rescale and window. Padding order is refused rather than swapped. Per-frame windows beat a missing top-level window. Lossy codes are closed. Palette is not a grey chain. Errors carry a tag and a closed reason. The helper binds `127.0.0.1`, prints only its URL, caps the body before reading it, and takes 63.75 from `LutChain`. The page does not show a file name and keeps the last success. No wasm-bindgen and no unsafe in the touched files.

## Limitations and handoff

Read only. The standard passages for C.11.2.1.1 were checked against the 2025 text after this report. Remediation belongs to a later tree and needs its own pass.
