# Overlays

**F-IDs that contributed:** F-193
**Last updated:** 2026-11-04

`read_overlays` reads top-level overlay groups and the display shutter for one frame. It does not decode Pixel Data and it does not draw.

## Public boundary

```rust
pub fn read_overlays(parsed: &ParsedDicom, frame: u32) -> Result<OverlaySet, OverlayError>
```

`OverlayPlane::bit` takes a 1-based image row and column. The origin shift is image equals overlay plus origin minus 1. A coordinate outside the image is clear. A set bit that lands outside the image stays in the plane and increments `outside_count`.

`Shutter::hides` is true when any shape hides the pixel. Rectangular edges are inclusive. A circle includes a pixel whose squared distance equals the squared radius. A polygon includes its boundary. Shapes combine by intersection of their openings.

`OverlaySet` also reports the image rows and columns from `(0028,0010)` and `(0028,0011)`.

## Groups

Even groups from `6000` through `601E` are walked. A group with no rows attribute is skipped. A later group is still read. A present rows attribute with a missing required sibling is an error. A refusal returns no planes.

Number of Frames in Overlay absent means one frame. Image Frame Origin absent means 1. Origin 0 is an error. The selected plane is the requested frame plus 1 minus the origin. A later image frame with no covering overlay contributes nothing. A frame index at or past the declared frame count is `FrameOutOfRange`.

A 1-bit overlay is packed low bit first and its bit position must be 0. Overlay Data may be OB or OW. OW words are expanded little-endian into the same packed bits. A 16-bit overlay is accepted only when Overlay Bits Allocated is 16, the bit position is 0 through 15, and Overlay Data is OW. The bit is taken from those words. Pixel Data is not read when Overlay Data is absent. Bits allocated 8 is an error.

## Per-frame groups

If the requested frame's item in `(5200,9230)` contains a tag whose group is even and in `6000` through `601E`, the read returns `FrameOverlayUnsupported`. Nested items count.

## Shutters

Shutter Shape may name rectangular, circular, polygonal and bitmap shapes. Rectangular edges are `(0018,1602)`, `(0018,1604)`, `(0018,1606)` and `(0018,1608)`.

Circle centre is `(0018,1610)` with two integer components, row then column. The radius is `(0018,1612)`. Polygon vertices are `(0018,1620)`, pairs of row then column. The design text named `(0018,1614)` as a third circle attribute. PS3.6 does not define that tag for this module, so the reader does not look for it.

A polygon with fewer than three vertices, an odd component count, a zero-length edge, or a crossing of non-adjacent edges is refused. A zero-length edge is refused even when every pair of edges is adjacent. Adjacent edges that only share a vertex are not a crossing. Adjacent edges that overlap along the same line are refused.

A bitmap shutter uses Shutter Overlay Group `(0018,1623)`. The group must be even and in `6000` through `601E`. The overlay type is G, the bits are packed, the origin is `1\1`, and the rows and columns match the image. Value 1 means the shutter applies. That group is removed from the graphic list.

Shutter Presentation Value absent or empty stays absent. A present value of 0 stays present. The value lives on `OverlaySet` and is not invented.

## Example viewer

`POST /overlay` accepts the token `synthetic` or one Part 10 object. The query `frame` selects the frame and defaults to 0. The synthetic object is 4 by 4. Image `(1, 1)` is set on the graphic overlay and hidden by the rectangular shutter. The presentation is null. The mask is sixteen graphic bits and the first is 1. The page says the neutral value is absent. It does not print 0 for a missing attribute. The mask is printed as digits. The design text says the preview uses the presentation value when it is present. The page prints that value as its own sentence and does not paint it into the mask digits. A chosen file is posted and discarded.

Tiers A, B and C do not draw this module.
