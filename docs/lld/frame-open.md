# Frame open

**F-IDs that contributed:** F-191
**Last updated:** 2026-11-04

`open_frame` reads one Part 10 object and returns the inputs `LutChain`
already accepts. It does not map samples. The example helper maps the
built-in fixture with `LutChain::map_into` and then fills padding.

## Public boundary

```rust
pub fn open_frame(parsed: &ParsedDicom, frame: u32) -> Result<OpenedFrame, FrameOpenError>
pub fn fill_padding(
    display: &mut [Display],
    mask: &[bool],
    photometric: PhotometricInterpretation,
    chain: &LutChain,
) -> Result<(), FrameOpenError>
```

`OpenedFrame` keeps the modality transform, the window input, the photometric
interpretation, the presentation evidence, the colour selection, the padding
range, the lossy flag, the burned-in flag and the private creators. Accessors
return those pieces. `with_codec_rgb(true)` records that a codec already
emitted RGB. A second call, or a call on monochrome or palette samples, is
`SecondColourConversion`.

## What it reads

Rescale, modality LUT, window, VOI LUT, photometric interpretation,
presentation LUT evidence, padding, lossy image compression, burned-in
annotation and private creators.

Resolution uses `MultiframeMetadata::resolve` with top-level fallback. A
per-frame functional group wins over the shared group, which wins over the
top-level attribute. A missing window and a missing VOI LUT stay absent. A
zero-length value is absent.

A missing rescale slope and a missing rescale intercept together are the
identity, slope 1 and intercept 0. That matches PS3.3 C.11.1 for objects that
do not carry the pair. One of the two without the other is an attribute error.
A modality LUT still wins, and the identity is not invented in that case.

The first item of a LUT sequence is the one selected. The selected window
index is 0. The display range passed to `VoiTransform` is 0 to 255. When the
VOI LUT Function code is absent, the function is LINEAR.

LUT Descriptor may be US or SS. The first and third values are always
unsigned. The second value of a modality descriptor follows Pixel
Representation. The second value of a VOI descriptor follows PS3.3 C.11.2.1.1.
It is unsigned when a modality LUT is present. It follows Pixel Representation
when rescale is absent. When rescale is present it is signed if the pixels are
signed or either rescale term is negative.

## Padding

DICOM PS3.3 C.7.5.1.1.2. The range is inclusive and is applied to stored
integers before the modality stage. MONOCHROME2, palette and colour require
the padding value to be less than or equal to the limit. MONOCHROME1 requires
the value to be greater than or equal to the limit. The other order is
`PaddingOrder`. A limit without a value is `PaddingLimitWithoutValue`.

`padding_mask` reports which stored integers fall inside the range.
`fill_padding` runs after `LutChain` and writes one constant at those
indices. The constant is the black end of that same chain.
`LutChain::inverts` is the only inversion decision. `MONOCHROME1` black is
`ymin` when the chain inverts and `ymax` when it does not. `MONOCHROME2`
black is the other end. Colour and palette use `ymin`. The range is the
chain's own output range. The write is not a window.

Stored 0 under LINEAR_EXACT centre 100 and width 200 is already `ymin`, so
that pair cannot show a missing fill. The fixture therefore also fills a
padding value of 50, which the same window maps to 63.75, and requires the
filled sample to be `ymin`.

## Projection

Pixel Data `(7FE0,0010)`, Float Pixel Data `(7FE0,0008)` and Double Float
Pixel Data `(7FE0,0009)` are skipped, including an encapsulated pixel sequence.
The metadata projection used by frame open does not retain those bytes.
`MetadataSet::from_object` still copies an explicit OW payload for other
callers, and it still refuses an encapsulated sequence. Frame open does not
use that function.

## Colour, flags and creators

`PALETTE COLOR` is `ColourSelection::Palette`. RGB and the YBR family are
`ColourSelection::Colour` with `codec_already_rgb` false until
`with_codec_rgb(true)`.

Lossy Image Compression `00` is lossless and `01` is lossy. Any other present
code is `LossyCode`. Burned In Annotation is `YES` or `NO`. Empty and absent
stay absent.

A private creator is an odd group element `(gggg,00xx)` with `xx` from `0x10`
to `0xFF`. The record is the creator string. Private values are not copied
into it.

## Example helper

`examples/example-server` binds `127.0.0.1` and prints only its base URL.
`POST /frame-open` with the body `synthetic` returns the built-in fixture,
including display 63.75 and padding count 1. Any other body is a Part 10
object. That response returns colour, flags, the window label, creator slots
and a padding count of 1 when a range is declared and 0 when it is not. It
does not return a sample array. The body cap is 1 GiB. A larger declared
length is refused before the body is read.

`npm run dev` starts this helper and Vite. The example page section Frame
open is the control.
