//! Volume assembly, geometry, reslicing.
//!
//! Targets: wasm32 yes, native yes. See `docs/hld/03-architecture-and-crates.md`.
//!
//! F-033 assembles a volume from decoded frames. Spacing comes from
//! `ImagePositionPatient` projected on the slice normal, DICOM PS3.3
//! C.7.6.2.1.1, and never from `SpacingBetweenSlices` or `SliceThickness`,
//! which are not read here at all.

#![cfg_attr(not(test), no_std)]

extern crate alloc;

use alloc::string::String;
use alloc::vec::Vec;

use glam::{DMat3, DVec3};
use ocelli_cache::{
    BrickGrid, BrickSample, BrickWidth, Budgeted, CacheTier, DecodedFrame, Lru, Pressure, SeriesId,
};
use ocelli_core::{Pt, World};
use ocelli_pixel::{ImageOrientationPatient, PixelRepresentation};

/// The crate's own name. The scaffold test asserts it matches Cargo's, which
/// is the one mistake a copy-pasted crate skeleton actually makes.
pub const CRATE_NAME: &str = env!("CARGO_PKG_NAME");

/// A `FrameOfReferenceUID` (0020,0052) value, kept as the UID string.
///
/// Grouping compares it for equality and does nothing else with it, so it is
/// not parsed. Two series share a spatial frame only if they share this UID.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct FrameOfReferenceUid(pub String);

/// What one voxel holds, section 19's `VoxelKind`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VoxelKind {
    /// Unsigned eight-bit.
    U8,
    /// Signed sixteen-bit.
    I16,
    /// Unsigned sixteen-bit.
    U16,
    /// Thirty-two-bit float.
    F32,
}

impl VoxelKind {
    /// The size of one voxel in this kind.
    pub fn bytes_per_voxel(self) -> usize {
        match self {
            Self::U8 => 1,
            Self::I16 | Self::U16 => 2,
            Self::F32 => 4,
        }
    }
}

impl From<VoxelKind> for BrickWidth {
    /// Sixteen-bit samples share one edge, signed or unsigned.
    fn from(kind: VoxelKind) -> Self {
        match kind {
            VoxelKind::U8 => Self::EightBit,
            VoxelKind::I16 | VoxelKind::U16 => Self::SixteenBit,
            VoxelKind::F32 => Self::Float32,
        }
    }
}

impl From<VoxelKind> for BrickSample {
    /// Signed and unsigned sixteen-bit samples stay distinct.
    ///
    /// [`BrickWidth`] collapses them because they share an edge. The upload
    /// format cannot, which is PS3.3 C.7.6.3 Pixel Representation.
    fn from(kind: VoxelKind) -> Self {
        match kind {
            VoxelKind::U8 => Self::U8,
            VoxelKind::I16 => Self::I16,
            VoxelKind::U16 => Self::U16,
            VoxelKind::F32 => Self::F32,
        }
    }
}

/// One slice's evidence for assembly: a decoded frame and its plane.
#[derive(Clone, Debug, PartialEq)]
pub struct SliceEvidence {
    frame: DecodedFrame,
}

impl SliceEvidence {
    /// Wrap one decoded frame as slice evidence.
    pub fn new(frame: DecodedFrame) -> Self {
        Self { frame }
    }

    /// The decoded frame this evidence carries.
    pub fn frame(&self) -> &DecodedFrame {
        &self.frame
    }
}

/// A set of frames that could form one volume, per PS3.3.
///
/// Grouped by `FrameOfReferenceUID` **and** `ImageOrientationPatient`,
/// because a series carrying a localiser has two orientations and is not one
/// volume.
#[derive(Clone, Debug, PartialEq)]
pub struct VolumeCandidate {
    /// The shared frame of reference.
    pub frame_of_reference: FrameOfReferenceUid,
    /// The shared orientation.
    pub orientation: ImageOrientationPatient,
    /// The slices, in whatever order they arrived.
    pub slices: Vec<SliceEvidence>,
}

impl VolumeCandidate {
    /// Assemble a candidate from its grouping key and its slices.
    pub fn new(
        frame_of_reference: FrameOfReferenceUid,
        orientation: ImageOrientationPatient,
        slices: Vec<SliceEvidence>,
    ) -> Self {
        Self {
            frame_of_reference,
            orientation,
            slices,
        }
    }
}

/// Group decoded frames into candidate volumes by frame of reference and
/// orientation, preserving first-seen order.
pub fn group_volumes(inputs: Vec<(FrameOfReferenceUid, DecodedFrame)>) -> Vec<VolumeCandidate> {
    let mut groups: Vec<VolumeCandidate> = Vec::new();
    for (frame_of_reference, frame) in inputs {
        let orientation = frame.plane().orientation;
        let existing = groups.iter_mut().find(|group: &&mut VolumeCandidate| {
            group.frame_of_reference == frame_of_reference && group.orientation == orientation
        });
        match existing {
            Some(group) => group.slices.push(SliceEvidence::new(frame)),
            None => groups.push(VolumeCandidate::new(
                frame_of_reference,
                orientation,
                alloc::vec![SliceEvidence::new(frame)],
            )),
        }
    }
    groups
}

/// Why a candidate is not a volume.
#[derive(Clone, Copy, Debug, PartialEq, thiserror::Error)]
pub enum VolumeError {
    /// Fewer than two slices, so no spacing can be derived.
    #[error("a volume needs at least two slices to derive spacing")]
    TooFewSlices,
    /// Two slices share a projected position.
    #[error("two slices share the projected position {at}")]
    DuplicatePosition {
        /// The shared projected position, in millimetres.
        at: f64,
    },
    /// The gaps are not uniform within tolerance.
    #[error("slice spacing is not uniform: median {median}, worst gap {worst} at index {index}")]
    NonUniformSpacing {
        /// The median gap, in millimetres.
        median: f64,
        /// The gap furthest from the median.
        worst: f64,
        /// The index of that gap in sorted order.
        index: usize,
    },
    /// The inter-slice vector is not parallel to the slice normal.
    #[error("the inter-slice vector is not parallel to the slice normal, cosine {cosine}")]
    ShearedVolume {
        /// The cosine between the inter-slice vector and the normal.
        cosine: f64,
    },
    /// Mixed orientations reached assembly. Frames of reference are
    /// grouping's concern in `group_volumes`, because slice evidence
    /// carries no frame of reference for assembly to compare.
    #[error("the slices do not share one orientation")]
    NotOneVolume,
    /// Slices disagree on dimensions or voxel kind.
    #[error("the slices do not share dimensions and voxel kind")]
    InconsistentDimensions,
    /// Slices disagree on in-plane spacing or sample layout.
    #[error("the slices do not share in-plane spacing and a one-sample layout")]
    InconsistentSpacing,
    /// A stored description whose kind `VoxelKind` cannot name.
    #[error("a {allocated}-bit stored description with signed {signed} has no voxel kind")]
    UnsupportedVoxelKind {
        /// Bits Allocated of the refused description.
        allocated: u8,
        /// Whether Pixel Representation declared signed samples.
        signed: bool,
    },
    /// A slice write outside the assembled slice range.
    #[error("slice {index} is outside the volume's {count} slices")]
    SliceOutOfRange {
        /// The slice index offered.
        index: u32,
        /// How many slices the volume has.
        count: u32,
    },
    /// A slice write whose byte length is not one slice.
    #[error("a slice is {expected} bytes and the write offered {actual}")]
    SliceSizeMismatch {
        /// One slice's byte length.
        expected: usize,
        /// The byte length offered.
        actual: usize,
    },
}

/// One multiscale level. Level 0 is full resolution, and E8.8 adds entries.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Level {
    /// Dimensions at this level.
    pub dims: [u32; 3],
    /// Spacing at this level, in millimetres.
    pub spacing: [f64; 3],
    /// Byte offset of this level in the store.
    pub offset: usize,
}

/// Per-slice presence for progressive assembly, one bit per slice.
///
/// Section 19 names `bitvec::BitVec`. This is a hand-rolled newtype over
/// `Vec<u64>` instead, recorded as deviation D-25, because `bitvec` is not a
/// workspace dependency and the set needs only set, test and count.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SlicePresence {
    words: Vec<u64>,
    slices: u32,
}

impl SlicePresence {
    /// An empty presence set for `slices` slices.
    pub fn new(slices: u32) -> Self {
        let words = usize::try_from(slices.div_ceil(64)).unwrap_or(0);
        Self {
            words: alloc::vec![0; words],
            slices,
        }
    }

    /// Mark slice `n` present. Outside the range is a no-op by index check.
    pub fn set(&mut self, n: u32) {
        if n >= self.slices {
            return;
        }
        let word = usize::try_from(n / 64).unwrap_or(usize::MAX);
        if let Some(held) = self.words.get_mut(word) {
            *held |= 1_u64 << (n % 64);
        }
    }

    /// Whether slice `n` has landed.
    pub fn test(&self, n: u32) -> bool {
        if n >= self.slices {
            return false;
        }
        let word = usize::try_from(n / 64).unwrap_or(usize::MAX);
        self.words
            .get(word)
            .is_some_and(|held| held & (1_u64 << (n % 64)) != 0)
    }

    /// How many slices have landed.
    pub fn count(&self) -> u32 {
        self.words
            .iter()
            .map(|word| word.count_ones())
            .sum::<u32>()
            .min(self.slices)
    }
}

/// One contiguous voxel allocation, x fastest then y then z.
///
/// Slice `n` starts at `n * dims[0] * dims[1] * bytes_per_voxel`, which is
/// section 19's layout rule with one implementation. A per-slice `Vec` is
/// never introduced, because the whole point is a single upload region.
#[derive(Clone, Debug, PartialEq)]
pub struct VoxelStore {
    data: Vec<u8>,
    dims: [u32; 3],
    kind: VoxelKind,
}

impl VoxelStore {
    /// A cleared store sized from dimensions and voxel kind.
    pub fn new(dims: [u32; 3], kind: VoxelKind) -> Self {
        let voxels = dims.iter().fold(1_usize, |total, dim| {
            total.saturating_mul(usize::try_from(*dim).unwrap_or(0))
        });
        Self {
            data: alloc::vec![0; voxels.saturating_mul(kind.bytes_per_voxel())],
            dims,
            kind,
        }
    }

    /// The whole allocation's byte length.
    pub fn len(&self) -> usize {
        self.data.len()
    }

    /// Whether the allocation is empty.
    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    fn slice_len(&self) -> usize {
        let plane =
            usize::try_from(self.dims[0]).unwrap_or(0) * usize::try_from(self.dims[1]).unwrap_or(0);
        plane * self.kind.bytes_per_voxel()
    }

    /// Slice `n`'s bytes, when `n` is inside the volume.
    pub fn slice(&self, n: u32) -> Option<&[u8]> {
        if n >= self.dims[2] {
            return None;
        }
        let len = self.slice_len();
        let start = usize::try_from(n).unwrap_or(usize::MAX).checked_mul(len)?;
        self.data.get(start..start.checked_add(len)?)
    }

    /// Slice `n`'s bytes for writing.
    ///
    /// # Errors
    ///
    /// [`VolumeError::SliceOutOfRange`] when `n` is outside the volume.
    pub fn slice_mut(&mut self, n: u32) -> Result<&mut [u8], VolumeError> {
        if n >= self.dims[2] {
            return Err(VolumeError::SliceOutOfRange {
                index: n,
                count: self.dims[2],
            });
        }
        let len = self.slice_len();
        let start = usize::try_from(n).unwrap_or(usize::MAX).saturating_mul(len);
        let end = start.saturating_add(len);
        match self.data.get_mut(start..end) {
            Some(slice) => Ok(slice),
            None => Err(VolumeError::SliceOutOfRange {
                index: n,
                count: self.dims[2],
            }),
        }
    }
}

/// Section 19's volume struct.
#[derive(Clone, Debug, PartialEq)]
pub struct Volume {
    /// Voxel dimensions, x then y then z.
    pub dims: [u32; 3],
    /// Millimetre spacing: between columns, between rows, between slices.
    pub spacing: [f64; 3],
    /// The first slice's `ImagePositionPatient`, the centre of voxel zero.
    pub origin: Pt<World>,
    /// The direction matrix from `ImageOrientationPatient` and the normal.
    pub direction: DMat3,
    /// What one voxel holds.
    pub voxel: VoxelKind,
    /// The one contiguous allocation.
    pub data: VoxelStore,
    /// The multiscale level axis, exactly one entry in this story.
    pub levels: Vec<Level>,
    /// Per-slice presence, for progressive assembly.
    pub present: SlicePresence,
    /// The arrival index of each slice in sorted order. `order[n]` is the
    /// index into the candidate's `slices` of the slice at sorted position
    /// `n`, so a caller places each arriving slice at its sorted position
    /// without deriving the projection a second time outside this crate.
    pub order: Vec<usize>,
}

impl Volume {
    /// The brick grid of this volume at level 0.
    ///
    /// Enumeration goes through [`BrickGrid`]. This volume does not keep a
    /// second decomposition.
    pub fn brick_grid(&self) -> BrickGrid {
        BrickGrid::new(self.dims, BrickWidth::from(self.voxel))
    }

    /// Assemble a candidate into a volume, or refuse it with the reason.
    ///
    /// Slices sort by projected position. Spacing between slices is the
    /// median projected gap, refused when any gap departs from it by more
    /// than 0.5 per cent, which is decision E of the S12 design round: at
    /// 3 mm that is 15 micrometres, far below a reconstruction increment and
    /// far above `f64` noise in a dot product.
    ///
    /// # Errors
    ///
    /// [`VolumeError::TooFewSlices`] under two slices.
    /// [`VolumeError::NotOneVolume`] when evidence disagrees with the
    /// candidate's orientation. [`VolumeError::InconsistentDimensions`] when
    /// slices disagree on dimensions or kind.
    /// [`VolumeError::DuplicatePosition`] on a shared projected position.
    /// [`VolumeError::NonUniformSpacing`] past the tolerance.
    /// [`VolumeError::ShearedVolume`] when an inter-slice vector is not
    /// parallel to the normal, detected from geometry and never from a tilt
    /// tag.
    pub fn assemble(candidate: VolumeCandidate) -> Result<Self, VolumeError> {
        if candidate.slices.len() < 2 {
            return Err(VolumeError::TooFewSlices);
        }
        let first = candidate.slices.first().ok_or(VolumeError::TooFewSlices)?;
        let first_plane = first.frame.plane();
        let (dims, voxel) = describe(first)?;
        for evidence in &candidate.slices {
            let plane = evidence.frame.plane();
            if plane.orientation != candidate.orientation {
                return Err(VolumeError::NotOneVolume);
            }
            let (held_dims, held_kind) = describe(evidence)?;
            if held_dims != dims || held_kind != voxel {
                return Err(VolumeError::InconsistentDimensions);
            }
            // One volume has one spacing and one slice length. Evidence
            // that disagrees on in-plane spacing, or carries more than one
            // sample per pixel so its frames could never be placed whole,
            // is not one volume, and the answer must not depend on which
            // slice arrived first.
            if plane.spacing() != first_plane.spacing()
                || evidence.frame.description().layout.samples_per_pixel() != 1
            {
                return Err(VolumeError::InconsistentSpacing);
            }
        }
        let normal: DVec3 = first_plane.slice_normal();
        let mut projected: Vec<(f64, DVec3, usize)> = candidate
            .slices
            .iter()
            .enumerate()
            .map(|(arrival, evidence)| {
                let plane = evidence.frame.plane();
                (
                    plane.position_vector().dot(normal),
                    plane.position_vector(),
                    arrival,
                )
            })
            .collect();
        projected.sort_by(|left, right| {
            left.0
                .partial_cmp(&right.0)
                .unwrap_or(core::cmp::Ordering::Equal)
        });
        for pair in projected.windows(2) {
            if let [lower, upper] = pair {
                // The duplicate bound is 1e-9 mm, a nanometre. Projected
                // positions are f64 dot products of millimetre values, whose
                // noise is far below this, and a real duplicate is exact, so
                // the bound separates the two without a judgement call.
                if (upper.0 - lower.0).abs() <= 1e-9 {
                    return Err(VolumeError::DuplicatePosition { at: upper.0 });
                }
                let vector = upper.1 - lower.1;
                let lateral = vector - normal * (upper.0 - lower.0);
                // The shear bound is 1e-6 mm of lateral step, a micrometre,
                // on the same reasoning: geometry noise is far below it and
                // a real shear step is millimetres.
                if lateral.length() > 1e-6 {
                    return Err(VolumeError::ShearedVolume {
                        cosine: vector.normalize().dot(normal).abs(),
                    });
                }
            }
        }
        let gaps: Vec<f64> = projected
            .windows(2)
            .map(|pair| {
                let upper = pair.get(1).map(|held| held.0).unwrap_or(0.0);
                let lower = pair.first().map(|held| held.0).unwrap_or(0.0);
                upper - lower
            })
            .collect();
        let median = median(&gaps);
        let mut worst = median;
        let mut worst_index = 0_usize;
        let mut worst_departure = 0.0_f64;
        for (index, gap) in gaps.iter().enumerate() {
            let departure = (gap - median).abs();
            if departure > worst_departure {
                worst_departure = departure;
                worst = *gap;
                worst_index = index;
            }
        }
        if worst_departure > median.abs() * 0.005 {
            return Err(VolumeError::NonUniformSpacing {
                median,
                worst,
                index: worst_index,
            });
        }
        let count = u32::try_from(projected.len()).unwrap_or(u32::MAX);
        let volume_dims = [dims[0], dims[1], count];
        let spacing = [
            first_plane.spacing().column_mm(),
            first_plane.spacing().row_mm(),
            median,
        ];
        let origin_position = projected
            .first()
            .map(|(_, position, _)| *position)
            .unwrap_or(DVec3::ZERO);
        let order: Vec<usize> = projected.iter().map(|(_, _, arrival)| *arrival).collect();
        let orientation = first_plane.orientation;
        let direction = DMat3::from_cols(orientation.row(), orientation.column(), normal);
        Ok(Self {
            dims: volume_dims,
            spacing,
            origin: Pt::new(origin_position.x, origin_position.y, origin_position.z),
            direction,
            voxel,
            data: VoxelStore::new(volume_dims, voxel),
            levels: alloc::vec![Level {
                dims: volume_dims,
                spacing,
                offset: 0,
            }],
            present: SlicePresence::new(count),
            order,
        })
    }

    /// Write one slice and mark it present. Idempotent: writing a slice
    /// twice replaces its contents and counts it once.
    ///
    /// # Errors
    ///
    /// [`VolumeError::SliceOutOfRange`] outside the volume, and
    /// [`VolumeError::SliceSizeMismatch`] when the write is not one slice.
    pub fn place(&mut self, n: u32, samples: &[u8]) -> Result<(), VolumeError> {
        let expected = self.data.slice_len();
        if samples.len() != expected {
            return Err(VolumeError::SliceSizeMismatch {
                expected,
                actual: samples.len(),
            });
        }
        self.data.slice_mut(n)?.copy_from_slice(samples);
        self.present.set(n);
        Ok(())
    }

    /// How many slices have landed, which drives the loading indicator.
    pub fn present_count(&self) -> u32 {
        self.present.count()
    }

    /// Whether every slice has landed.
    pub fn is_complete(&self) -> bool {
        self.present_count() == self.dims[2]
    }

    /// Brick coordinates whose slices have all landed, x fastest.
    ///
    /// That order is [`BrickGrid::index_of`]. A brick whose z range includes
    /// an absent slice is left out. Uploading it would invent zeros the study
    /// does not contain. The renderer does not read [`SlicePresence`].
    pub fn ready_bricks(&self) -> Vec<[u32; 3]> {
        let grid = self.brick_grid();
        let [gx, gy, gz] = grid.grid;
        let mut ready = Vec::new();
        let mut z = 0u32;
        while z < gz {
            let mut y = 0u32;
            while y < gy {
                let mut x = 0u32;
                while x < gx {
                    let brick = [x, y, z];
                    if brick_is_ready(&grid, &self.present, brick) {
                        ready.push(brick);
                    }
                    x += 1;
                }
                y += 1;
            }
            z += 1;
        }
        ready
    }
}

/// Every slice of `brick` has landed, and the brick has a real extent.
fn brick_is_ready(grid: &BrickGrid, present: &SlicePresence, brick: [u32; 3]) -> bool {
    let Ok(origin) = grid.origin_of(brick) else {
        return false;
    };
    let Ok(extent) = grid.extent_of(brick) else {
        return false;
    };
    let [ex, ey, ez] = extent;
    if ex == 0 || ey == 0 || ez == 0 {
        return false;
    }
    let [_, _, oz] = origin;
    let Some(end) = oz.checked_add(ez) else {
        return false;
    };
    let mut slice = oz;
    while slice < end {
        if !present.test(slice) {
            return false;
        }
        slice += 1;
    }
    true
}

impl Budgeted for Volume {
    /// The contiguous allocation. The level axis and the presence set are
    /// bounded by it and are not counted separately.
    fn bytes(&self) -> usize {
        self.data.len()
    }
}

fn describe(evidence: &SliceEvidence) -> Result<([u32; 2], VoxelKind), VolumeError> {
    let description = evidence.frame.description();
    // A kind `VoxelKind` cannot name refuses rather than borrowing the kind
    // with the same byte width. Signed 8-bit has no variant, and a 32-bit
    // stored container is an integer in this system, not a float, so 32-bit
    // frames refuse here until a description can say float.
    let kind = match (
        description.bits.bits_allocated(),
        description.bits.pixel_representation(),
    ) {
        (8, PixelRepresentation::Unsigned) => VoxelKind::U8,
        (16, PixelRepresentation::Signed) => VoxelKind::I16,
        (16, PixelRepresentation::Unsigned) => VoxelKind::U16,
        (allocated, representation) => {
            return Err(VolumeError::UnsupportedVoxelKind {
                allocated,
                signed: representation == PixelRepresentation::Signed,
            });
        }
    };
    Ok((
        [
            description.dimensions.columns(),
            description.dimensions.rows(),
        ],
        kind,
    ))
}

fn median(gaps: &[f64]) -> f64 {
    let mut sorted = gaps.to_vec();
    sorted.sort_by(|left, right| {
        left.partial_cmp(right)
            .unwrap_or(core::cmp::Ordering::Equal)
    });
    let middle = sorted.len() / 2;
    if sorted.len() % 2 == 1 {
        sorted.get(middle).copied().unwrap_or(0.0)
    } else {
        let lower = sorted.get(middle.saturating_sub(1)).copied().unwrap_or(0.0);
        let upper = sorted.get(middle).copied().unwrap_or(0.0);
        (lower + upper) / 2.0
    }
}

/// One assembled volume's identity in the decoded tier.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct VolumeKey {
    /// The series the volume was assembled from.
    pub series: SeriesId,
}

/// Assembled volumes in the decoded tier, in their own `Lru` with their own
/// budget, per decision F of the S12 design round. A volume is a decoded
/// thing that is not a frame, and sharing the frame budget would let one
/// volume evict every frame a viewer is holding.
pub struct VolumeCache {
    volumes: Lru<VolumeKey, Volume>,
}

impl VolumeCache {
    /// An empty volume cache holding at most `budget` bytes.
    pub fn new(budget: usize) -> Self {
        Self {
            volumes: Lru::new(CacheTier::Decoded, budget),
        }
    }

    /// Insert a volume, returning what admission displaced or evicted.
    pub fn insert(
        &mut self,
        key: VolumeKey,
        volume: Volume,
    ) -> ocelli_cache::Admission<VolumeKey, Volume> {
        self.volumes.insert(key, volume)
    }

    /// Read a volume. Reading is a use.
    pub fn get(&mut self, key: &VolumeKey) -> Option<&Volume> {
        self.volumes.get(key)
    }

    /// Explicitly release a volume.
    pub fn remove(&mut self, key: &VolumeKey) -> Option<Volume> {
        self.volumes.remove(key)
    }

    /// This cache's pressure signal.
    pub fn pressure(&self) -> Pressure {
        self.volumes.pressure()
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn scaffold_declares_its_own_name() {
        assert_eq!(super::CRATE_NAME, env!("CARGO_PKG_NAME"));
        assert!(super::CRATE_NAME.starts_with("ocelli"));
    }
}
