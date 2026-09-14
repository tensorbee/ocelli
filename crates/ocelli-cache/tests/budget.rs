//! Budget fixtures for HLD section 20's `Lru`, in bytes.
//!
//! **Every number asserted here is hand-computed above the assertion that uses
//! it**, from the frame geometry DICOM PS3.3 C.7.6.3.1's Image Pixel macro
//! defines and from the footprint rules the budget is actually kept against.
//! None of them was read back from the implementation, which is HLD 27.2 R2.
//!
//! PS3.3 C.7.6.3.1 is the Image Pixel module, which defines `Rows`
//! (0028,0010), `Columns` (0028,0011), `Samples per Pixel` (0028,0002) and
//! `Bits Allocated` (0028,0100). `ocelli-codec`'s `frame_bits` multiplies those
//! four for an uncompressed frame and returns bits. **The division by eight to
//! reach bytes is this file's**, because a budget is kept in bytes. Nothing
//! here decodes anything, and a frame is a size here for the same reason.

use ocelli_cache::{Budgeted, CacheTier, Lru};

/// A decoded greyscale frame, carried by its geometry rather than by a length,
/// so the length below is derived the way the standard derives it.
#[derive(Debug, PartialEq, Eq)]
struct DecodedFrame {
    rows: usize,
    columns: usize,
    samples_per_pixel: usize,
    bits_allocated: usize,
}

impl Budgeted for DecodedFrame {
    /// PS3.3 C.7.6.3.1. An uncompressed frame occupies
    /// `Rows * Columns * SamplesPerPixel * BitsAllocated / 8` bytes.
    fn bytes(&self) -> usize {
        self.rows * self.columns * self.samples_per_pixel * self.bits_allocated / 8
    }
}

impl DecodedFrame {
    /// The 512 by 512 sixteen-bit greyscale frame the fixtures below use.
    fn ct() -> Self {
        Self {
            rows: 512,
            columns: 512,
            samples_per_pixel: 1,
            bits_allocated: 16,
        }
    }
}

/// A whole sixteen-bit series, which is what HLD section 7's bricking bullet
/// weighs against the guaranteed maximum buffer size.
#[derive(Debug, PartialEq, Eq)]
struct DecodedSeries {
    rows: usize,
    columns: usize,
    slices: usize,
    bits_allocated: usize,
}

impl Budgeted for DecodedSeries {
    /// The same four PS3.3 C.7.6.3.1 attributes, over every slice of a series.
    /// At one sample per pixel that is `Rows * Columns * Slices *
    /// BitsAllocated / 8` bytes, which is the product HLD section 7's bricking
    /// bullet takes. `Slices` is this fixture's count and not a PS3.3
    /// attribute.
    fn bytes(&self) -> usize {
        self.rows * self.columns * self.slices * self.bits_allocated / 8
    }
}

/// A GPU texture, carried by the two row lengths that differ, because which of
/// them `bytes` reports is the whole point of the second fixture.
#[derive(Debug, PartialEq, Eq)]
struct GpuTexture {
    /// The bytes one row of the decoded source occupies.
    source_bytes_per_row: usize,
    /// The bytes one row of the allocation occupies, after padding.
    allocated_bytes_per_row: usize,
    rows: usize,
}

impl GpuTexture {
    /// A 512-row texture whose row holds 300 bytes of samples and whose
    /// allocation pads that row to 512 bytes.
    fn padded() -> Self {
        Self {
            source_bytes_per_row: 300,
            allocated_bytes_per_row: 512,
            rows: 512,
        }
    }
}

impl Budgeted for GpuTexture {
    /// The allocated footprint, not the source footprint. A texture uploaded
    /// through a buffer copy is padded to the copy's row alignment, and the
    /// padding is memory the device holds whether or not a sample lives in it.
    fn bytes(&self) -> usize {
        self.allocated_bytes_per_row * self.rows
    }
}

/// The frame length, hand-computed.
///
/// `512 * 512 = 262_144` samples. Sixteen bits allocated is two bytes per
/// sample, so `262_144 * 2 = 524_288` bytes.
const FRAME_BYTES: usize = 524_288;

/// Four frames exactly. `524_288 * 4 = 2_097_152`, which is 2 MiB.
const FOUR_FRAME_BUDGET: usize = 2_097_152;

/// One padded texture exactly. `512 * 512 = 262_144`. See
/// `texture_bytes_are_the_allocated_footprint_not_the_source_footprint`.
const ONE_TEXTURE_BUDGET: usize = 262_144;

/// The same texture's source footprint. `300 * 512 = 153_600`.
const TEXTURE_SOURCE_BYTES: usize = 153_600;

/// HLD section 7's guaranteed maximum buffer size, 256 MiB.
/// `256 * 1024 * 1024 = 268_435_456`.
const MAX_BUFFER_BYTES: usize = 268_435_456;

/// HLD section 7's series, hand-computed.
///
/// `512 * 512 = 262_144` samples per slice, `262_144 * 600 = 157_286_400`
/// samples in the series, and two bytes each is `314_572_800` bytes. Section 7
/// rounds that to "roughly 300 MB".
const SERIES_BYTES: usize = 314_572_800;

/// An 8-bit RGB frame of the same geometry. `512 * 512 = 262_144` pixels, and
/// three samples of one byte each is `786_432` bytes.
const RGB_FRAME_BYTES: usize = 786_432;

#[test]
fn a_frame_is_rows_by_columns_by_samples_by_bytes_per_sample() {
    assert_eq!(DecodedFrame::ct().bytes(), FRAME_BYTES);

    // The same geometry with three samples per pixel, which is the only shape
    // that notices Samples per Pixel going missing from the product. Every
    // other fixture here is greyscale, where the factor is one.
    let rgb = DecodedFrame {
        rows: 512,
        columns: 512,
        samples_per_pixel: 3,
        bits_allocated: 8,
    };
    assert_eq!(rgb.bytes(), RGB_FRAME_BYTES);
}

#[test]
fn a_two_mib_budget_holds_exactly_four_frames_and_the_fifth_evicts_the_first() {
    let mut cache: Lru<u8, DecodedFrame> = Lru::new(CacheTier::Decoded, FOUR_FRAME_BUDGET);

    for key in 0..4 {
        let admission = cache.insert(key, DecodedFrame::ct());
        assert!(admission.evicted.is_empty());
        assert!(admission.refused.is_none());
    }

    // Four frames of 524_288 bytes are 2_097_152 bytes, which is the whole
    // budget, so nothing is free and nothing has been evicted.
    assert_eq!(cache.len(), 4);
    assert_eq!(cache.pressure().used, FOUR_FRAME_BUDGET);
    assert_eq!(cache.pressure().budget, FOUR_FRAME_BUDGET);

    let admission = cache.insert(4, DecodedFrame::ct());

    // The fifth frame needs 524_288 bytes and the budget has none free, so
    // exactly one frame leaves, and it is the one inserted first.
    assert_eq!(admission.evicted, vec![(0, DecodedFrame::ct())]);
    assert!(admission.displaced.is_none());
    assert!(admission.refused.is_none());
    assert_eq!(cache.len(), 4);
    assert_eq!(cache.pressure().used, FOUR_FRAME_BUDGET);
    assert!(!cache.contains_key(&0));
}

#[test]
fn texture_bytes_are_the_allocated_footprint_not_the_source_footprint() {
    // A copy into a texture is aligned to 256 bytes per row, so a 300-byte row
    // pads to the next multiple of 256, which is 512.
    //
    //   source:    300 * 512 = 153_600 bytes
    //   allocated: 512 * 512 = 262_144 bytes
    //
    // A `bytes` reporting 153_600 would leave 262_144 - 153_600 = 108_544
    // bytes of a full budget reported as free, and the device would have none.
    let texture = GpuTexture::padded();
    assert_eq!(
        texture.source_bytes_per_row * texture.rows,
        TEXTURE_SOURCE_BYTES
    );
    assert_eq!(texture.bytes(), ONE_TEXTURE_BUDGET);

    let mut cache: Lru<u8, GpuTexture> = Lru::new(CacheTier::Gpu, ONE_TEXTURE_BUDGET);
    let admission = cache.insert(0, texture);
    assert!(admission.refused.is_none());

    // The budget is full at one texture, and not 108_544 bytes short of full.
    assert_eq!(cache.pressure().used, ONE_TEXTURE_BUDGET);
    assert_eq!(
        cache.pressure().budget - cache.pressure().used,
        0,
        "a budget kept in source sizes reports {} bytes that do not exist",
        ONE_TEXTURE_BUDGET - TEXTURE_SOURCE_BYTES
    );

    let admission = cache.insert(1, GpuTexture::padded());
    assert_eq!(admission.evicted, vec![(0, GpuTexture::padded())]);
    assert_eq!(cache.len(), 1);
    assert_eq!(cache.pressure().used, ONE_TEXTURE_BUDGET);
}

#[test]
fn the_section_7_series_does_not_fit_a_256_mib_budget() {
    // HLD section 7: "A 512×512×600 sixteen-bit CT series is roughly 300 MB
    // against a guaranteed maximum buffer size of 256 MiB, so chunked upload
    // is the normal path, not an optimisation."
    //
    //   series: 512 * 512 * 600 * 2 = 314_572_800 bytes
    //   budget: 256 * 1024 * 1024   = 268_435_456 bytes
    //
    // 314_572_800 > 268_435_456, so the whole series is not an entry a budget
    // of this size can ever hold, whatever is or is not in the cache.
    // 314_572_800 - 268_435_456 = 46_137_344 bytes over, checked at compile
    // time because both sides are constants.
    const { assert!(SERIES_BYTES - MAX_BUFFER_BYTES == 46_137_344) };

    let series = DecodedSeries {
        rows: 512,
        columns: 512,
        slices: 600,
        bits_allocated: 16,
    };
    assert_eq!(series.bytes(), SERIES_BYTES);

    let mut cache: Lru<u8, DecodedSeries> = Lru::new(CacheTier::Gpu, MAX_BUFFER_BYTES);
    assert!(!cache.would_admit(SERIES_BYTES));
    assert!(cache.would_admit(MAX_BUFFER_BYTES));

    let admission = cache.insert(0, series);
    assert_eq!(
        admission.refused,
        Some((
            0,
            DecodedSeries {
                rows: 512,
                columns: 512,
                slices: 600,
                bits_allocated: 16,
            }
        ))
    );
    assert!(admission.evicted.is_empty());
    assert_eq!(cache.pressure().used, 0);
    assert!(cache.is_empty());
}
