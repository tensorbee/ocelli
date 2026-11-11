# Series open

**F-IDs that contributed:** F-192, F-214
**Last updated:** 2026-11-10

`assemble_display` orders a bag of already parsed frames into display groups. It does not build a volume and it does not scan a directory.

## Public boundary

```rust
pub fn assemble_display(frames: &[ParsedFrame]) -> Result<Vec<DisplaySeries>, SeriesOpenError>
pub fn declared_frame_count(parsed: &ParsedDicom) -> Result<u32, SeriesOpenError>
pub fn cine_timing(metadata: &MetadataSet) -> Result<CineTiming, SeriesOpenError>
```

`DisplaySeries` keeps the spatial order, the cine timing, the playback sequencing, the spatial slots, and two reports. Those reports are shear and a non-uniform through-plane step. Accessors return those pieces. A slot is one projected position. Frames that share it stay in the slot.

`ParsedFrame` borrows one parsed object and a frame index. It does not store a path.

## Grouping

The group key is the Series Instance UID, the Frame of Reference UID, orientation agreement, and in-plane spacing agreement. A missing UID does not join a present UID. Frames of one parsed object share one missing UID. Two parsed objects that both omit a UID do not join each other. Orientation uses the row and column cosines and `DIRECTION_COSINE_TOLERANCE`. Spacing uses the one `spacing_agrees` helper. A 90 degree in-plane rotation is a different group even when the slice normal matches.

A frame with no Image Position Patient is unpositioned. It joins the single positioned group that shares its series and frame of reference. When several orientation groups share that pair, or when none do, the frame stays in an unpositioned companion. Unpositioned frames that share that pair share one companion. A present orientation or pixel spacing that cannot be read is still an error. Any other geometry failure is an error for that input.

## Order

Positioned frames sort by the projection of Image Position Patient onto the first positioned frame's slice normal. That projection is DICOM PS3.3 C.7.6.2.1.1. Equal projections within the existing geometry tolerance stay one slot. Inside a slot the order is Stack ID, then In-Stack Position Number, then Temporal Position Index, then instance number, then SOP Instance UID bytes, then frame index, then input index. Unpositioned frames sort after positioned frames by instance number, then SOP Instance UID bytes, then frame index, then input index.

The group reports `Projected` when any frame has a position. Otherwise it reports `InstanceNumber`.

Shear and a non-uniform through-plane step are reports. `StackGeometry::derive` still refuses those cases for a volume.

## Cine

Timing is read from the first frame in display order, from the top-level cine module. The first valid source wins.

| Tag | Result |
|-----|--------|
| (0018,1065) | Frame time vector, only when its length equals the group length |
| (0018,1063) | One interval in milliseconds |
| (0008,2144) | Frames per second |
| (0018,0040) | Frames per second |

An absent or zero-length source is skipped. A vector whose length differs from the group is skipped before its values are read. PS3.3 C.7.6.5.1.2 says the first vector component is zero. A later component that is non-finite, zero, or negative is an error and does not fall through. Frame Time, the recommended rate, and cine rate use the same rule for their single value. When every source is absent, timing is `Unavailable`. The library does not invent a frame rate.

`cine_timing` reads the timing of one object alone, from its metadata projection, with the same sources, order and refusals and the object's declared frame count as the length a Frame Time Vector must have. It does not read Preferred Playback Sequencing, so a bad value there never hides a good Frame Time. PS3.3 C.7.6.5.1.1 defines Frame Time as the interval between frames of one multi-frame image, so on an object of one frame a Frame Time of zero is `Unavailable` and not an error. A malformed frame declaration or functional-group sequence is `SeriesOpenReason::Metadata`, and a present but unusable timing value is `SeriesOpenReason::Cine`. The folder viewer's inspect reply reports it as `cine` and `frameTimeMs`, and its Play waits that Frame Time only between frames of the same file.

Preferred Playback Sequencing `(0018,1244)` is US. Value 0 is `Loop` and value 1 is `Sweep`. An absent value is `Loop`. Any other value or value representation is an error. The example page labels `Sweep` as Bounce. The design text named the strings LOOP and SWEEP. PS3.6 gives the attribute value representation US, and that is what the reader accepts.

## Example viewer

`POST /series-open` accepts the token `synthetic` or a multipart body of Part 10 objects. The synthetic body is three objects, because two groups and an axial order of two slices do not fit in two objects. The page shows the group count, the order, the timing source, the interval, and a Loop or Bounce control. A shown position is the frame list index and the frame inside that object. A parse failure names the part. An assembly failure names the frame list index. Playback of pixels stays on the folder page.

Pixel Data is not copied into the metadata projection. The projection is the one frame open already uses.
