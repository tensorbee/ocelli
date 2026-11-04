//! Patient clip planes for the composite volume ray, F-062.
//!
//! The kept side is `dot(point - plane_origin, normal) >= 0`, including the
//! plane. A discarded sample adds nothing. Bricks are not dropped. At most
//! six planes are stored. An empty list leaves every sample kept, so the
//! measured ray stays until a caller sets a plane.
//!
//! The patient point is the same formula as `Volume::voxel_world`. The origin
//! is the centre of voxel `(0, 0, 0)`. Column 0 of the direction scales by
//! the column spacing and the x index, column 1 by the row spacing and the y
//! index, and column 2 by the slice spacing and the z index.

use ocelli_viewport::view::f32_from_f64;

/// Bytes of the clip uniform. The prefix is 64 bytes and six slots follow.
pub(crate) const CLIP_UNIFORM_BYTES: usize = 256;

const PLANE_SLOTS: usize = 6;
const FRAME_BYTES: usize = 64;
const SLOT_BYTES: usize = 32;

/// Why a caller list was not stored.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub(crate) enum ClipError {
    /// The list has more than six planes.
    #[error("a volume ray accepts at most six clip planes")]
    Count,
    /// A component is NaN or infinite, or it does not narrow to a finite f32.
    #[error("a clip plane component is not finite")]
    NotFinite,
    /// The f64 length is zero, or the f32 image of the normal has length zero.
    #[error("a clip plane normal has length zero")]
    ZeroNormal,
}

/// One plane in the patient frame.
///
/// The fields stay public because the caller owns the value it passes in.
/// `set_clip_planes` copies a validated list and does not keep this struct.
#[derive(Debug, Clone, Copy)]
pub struct ClipPlane {
    /// A point on the plane, in patient millimetres.
    pub origin_mm: [f64; 3],
    /// The normal. The side it points toward is kept. Length zero is refused.
    pub normal: [f64; 3],
}

/// Patient origin and direction columns remembered at a successful bind.
///
/// The shader uniform is `f32`. The `f64` values are checked in `try_new`
/// and are not stored again.
#[derive(Debug, Clone, Copy)]
pub(crate) struct PatientFrame {
    origin_f32: [f32; 3],
    direction_f32: [[f32; 3]; 3],
}

impl PatientFrame {
    /// Origin at 0 mm and an identity direction. Both are exact in `f32`.
    #[must_use]
    pub(crate) fn identity() -> Self {
        Self {
            origin_f32: [0.0, 0.0, 0.0],
            direction_f32: [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]],
        }
    }

    /// Copy a caller frame when every component survives the `f32` narrowing.
    ///
    /// A non-zero `f64` that becomes a zero `f32` is refused. The shader would
    /// otherwise collapse that axis onto the origin and test the wrong point.
    #[must_use]
    pub(crate) fn try_new(origin_mm: [f64; 3], direction: [[f64; 3]; 3]) -> Option<Self> {
        let origin_f32 = narrow_point(origin_mm)?;
        let direction_f32 = [
            narrow_point(column(direction, 0)?)?,
            narrow_point(column(direction, 1)?)?,
            narrow_point(column(direction, 2)?)?,
        ];
        Some(Self {
            origin_f32,
            direction_f32,
        })
    }
}

#[derive(Clone, Copy)]
struct PlaneSlot {
    origin_mm: [f64; 3],
    normal: [f64; 3],
    origin_f32: [f32; 3],
    normal_f32: [f32; 3],
    active: bool,
}

impl PlaneSlot {
    fn inactive() -> Self {
        Self {
            origin_mm: [0.0, 0.0, 0.0],
            normal: [0.0, 0.0, 0.0],
            origin_f32: [0.0, 0.0, 0.0],
            normal_f32: [0.0, 0.0, 0.0],
            active: false,
        }
    }

    fn try_new(plane: &ClipPlane) -> Result<Self, ClipError> {
        let (ox, oy, oz) = xyz(plane.origin_mm);
        let (nx, ny, nz) = xyz(plane.normal);
        if !ox.is_finite()
            || !oy.is_finite()
            || !oz.is_finite()
            || !nx.is_finite()
            || !ny.is_finite()
            || !nz.is_finite()
        {
            return Err(ClipError::NotFinite);
        }
        if length_is_zero(nx, ny, nz) {
            return Err(ClipError::ZeroNormal);
        }
        let origin_f32 = match narrow_point(plane.origin_mm) {
            Some(point) => point,
            None => return Err(ClipError::NotFinite),
        };
        let normal_f32 = narrow_normal(plane.normal)?;
        Ok(Self {
            origin_mm: plane.origin_mm,
            normal: plane.normal,
            origin_f32,
            normal_f32,
            active: true,
        })
    }

    fn same(&self, other: &Self) -> bool {
        self.active == other.active
            && same_f64(self.origin_mm, other.origin_mm)
            && same_f64(self.normal, other.normal)
    }
}

/// The planes stored for one viewport. Empty until a successful set.
#[derive(Clone, Copy)]
pub(crate) struct ClipList {
    slots: [PlaneSlot; PLANE_SLOTS],
}

impl ClipList {
    /// No planes. Every finite patient point is kept.
    #[must_use]
    pub(crate) fn empty() -> Self {
        Self {
            slots: [PlaneSlot::inactive(); PLANE_SLOTS],
        }
    }

    /// Validate `planes` and copy them.
    ///
    /// A refusal builds no list. Finiteness is tested before the length of
    /// the normal. The `f64` length is tested before the `f32` narrowing.
    ///
    /// # Errors
    ///
    /// [`ClipError::Count`] above six planes.
    /// [`ClipError::NotFinite`] for a non-finite component, a component that
    /// does not narrow to a finite `f32`, or an origin component that
    /// collapses to a zero `f32`.
    /// [`ClipError::ZeroNormal`] when the `f64` length is zero or the `f32`
    /// image of the normal has length zero.
    pub(crate) fn from_planes(planes: &[ClipPlane]) -> Result<Self, ClipError> {
        if planes.len() > PLANE_SLOTS {
            return Err(ClipError::Count);
        }
        let mut slots = [PlaneSlot::inactive(); PLANE_SLOTS];
        for (index, plane) in planes.iter().enumerate() {
            let Some(slot) = slots.get_mut(index) else {
                return Err(ClipError::Count);
            };
            *slot = PlaneSlot::try_new(plane)?;
        }
        Ok(Self { slots })
    }

    /// Whether every active plane keeps `point`.
    ///
    /// An empty list returns true. The comparison includes the plane.
    /// The shader repeats this test in `f32`. This copy is the host fixture.
    #[cfg(test)]
    #[must_use]
    pub(crate) fn keeps(&self, point: [f64; 3]) -> bool {
        let (px, py, pz) = xyz(point);
        for slot in &self.slots {
            if !slot.active {
                continue;
            }
            let (ox, oy, oz) = xyz(slot.origin_mm);
            let (nx, ny, nz) = xyz(slot.normal);
            let facing = (px - ox).mul_add(nx, (py - oy).mul_add(ny, (pz - oz) * nz));
            match facing.partial_cmp(&0.0) {
                Some(std::cmp::Ordering::Equal | std::cmp::Ordering::Greater) => {}
                Some(std::cmp::Ordering::Less) | None => return false,
            }
        }
        true
    }

    pub(crate) fn same(&self, other: &Self) -> bool {
        self.slots
            .iter()
            .zip(other.slots.iter())
            .all(|(slot, held)| slot.same(held))
    }

    /// Little-endian bytes for the clip uniform.
    ///
    /// Bytes 0 to 63 are the patient origin and the three direction columns,
    /// each a `vec4` with `w` zero. Each of the six slots is an origin `vec4`
    /// and a normal `vec4`. The normal `w` is 1 when the slot is active and
    /// 0 when it is disabled.
    #[must_use]
    pub(crate) fn uniform_bytes(&self, frame: &PatientFrame) -> [u8; CLIP_UNIFORM_BYTES] {
        let mut bytes = [0_u8; CLIP_UNIFORM_BYTES];
        write_vec4(&mut bytes, 0, frame.origin_f32, 0.0);
        for (index, column) in frame.direction_f32.iter().enumerate() {
            let offset = 16_usize.saturating_mul(index.saturating_add(1));
            write_vec4(&mut bytes, offset, *column, 0.0);
        }
        for (index, slot) in self.slots.iter().enumerate() {
            if !slot.active {
                continue;
            }
            let base = FRAME_BYTES.saturating_add(index.saturating_mul(SLOT_BYTES));
            write_vec4(&mut bytes, base, slot.origin_f32, 0.0);
            write_vec4(&mut bytes, base.saturating_add(16), slot.normal_f32, 1.0);
        }
        bytes
    }
}

fn column(direction: [[f64; 3]; 3], index: usize) -> Option<[f64; 3]> {
    direction.get(index).copied()
}

fn xyz(value: [f64; 3]) -> (f64, f64, f64) {
    (
        value.first().copied().unwrap_or(f64::NAN),
        value.get(1).copied().unwrap_or(f64::NAN),
        value.get(2).copied().unwrap_or(f64::NAN),
    )
}

fn xyz_f32(value: [f32; 3]) -> (f32, f32, f32) {
    (
        value.first().copied().unwrap_or(f32::NAN),
        value.get(1).copied().unwrap_or(f32::NAN),
        value.get(2).copied().unwrap_or(f32::NAN),
    )
}

fn same_f64(left: [f64; 3], right: [f64; 3]) -> bool {
    let (lx, ly, lz) = xyz(left);
    let (rx, ry, rz) = xyz(right);
    lx.to_bits() == rx.to_bits() && ly.to_bits() == ry.to_bits() && lz.to_bits() == rz.to_bits()
}

fn f64_is_zero(value: f64) -> bool {
    let bits = value.to_bits();
    bits == 0 || bits == (1_u64 << 63)
}

fn f32_is_zero(value: f32) -> bool {
    let bits = value.to_bits();
    bits == 0 || bits == (1_u32 << 31)
}

fn length_is_zero(x: f64, y: f64, z: f64) -> bool {
    let sum = x.mul_add(x, y.mul_add(y, z * z));
    let bits = sum.to_bits();
    bits == 0 || bits == (1_u64 << 63)
}

/// Narrow one patient component. A non-zero value that flushes to zero is
/// not representable in the uniform.
fn narrow_scalar(value: f64) -> Option<f32> {
    if !value.is_finite() {
        return None;
    }
    let image = f32_from_f64(value);
    if !image.is_finite() {
        return None;
    }
    if !f64_is_zero(value) && f32_is_zero(image) {
        return None;
    }
    Some(image)
}

fn narrow_point(point: [f64; 3]) -> Option<[f32; 3]> {
    let (x, y, z) = xyz(point);
    Some([narrow_scalar(x)?, narrow_scalar(y)?, narrow_scalar(z)?])
}

/// Narrow a normal. A component that flushes to zero is kept when another
/// component stays non-zero. The normal is refused when every narrowed
/// component is zero. A square that underflows is not that test.
fn narrow_normal(normal: [f64; 3]) -> Result<[f32; 3], ClipError> {
    let (x, y, z) = xyz(normal);
    let image = [
        match finite_f32(x) {
            Some(value) => value,
            None => return Err(ClipError::NotFinite),
        },
        match finite_f32(y) {
            Some(value) => value,
            None => return Err(ClipError::NotFinite),
        },
        match finite_f32(z) {
            Some(value) => value,
            None => return Err(ClipError::NotFinite),
        },
    ];
    let (ix, iy, iz) = xyz_f32(image);
    if f32_is_zero(ix) && f32_is_zero(iy) && f32_is_zero(iz) {
        return Err(ClipError::ZeroNormal);
    }
    Ok(image)
}

fn finite_f32(value: f64) -> Option<f32> {
    if !value.is_finite() {
        return None;
    }
    let image = f32_from_f64(value);
    if !image.is_finite() {
        return None;
    }
    Some(image)
}

fn write_f32(bytes: &mut [u8], offset: usize, value: f32) {
    let Some(end) = offset.checked_add(4) else {
        return;
    };
    if let Some(slot) = bytes.get_mut(offset..end) {
        slot.copy_from_slice(&value.to_le_bytes());
    }
}

fn write_vec4(bytes: &mut [u8], offset: usize, xyz: [f32; 3], w: f32) {
    let (x, y, z) = xyz_f32(xyz);
    write_f32(bytes, offset, x);
    write_f32(bytes, offset.saturating_add(4), y);
    write_f32(bytes, offset.saturating_add(8), z);
    write_f32(bytes, offset.saturating_add(12), w);
}

#[cfg(test)]
mod tests {
    use glam::DMat3;
    use ocelli_core::{Pt, World};
    use ocelli_volume::{Level, SlicePresence, Volume, VoxelKind, VoxelStore};

    use super::{ClipError, ClipList, ClipPlane, PatientFrame};

    fn volume_at(spacing: [f64; 3], dims: [u32; 3]) -> Volume {
        Volume {
            dims,
            spacing,
            origin: Pt::<World>::new(0.0, 0.0, 0.0),
            direction: DMat3::IDENTITY,
            voxel: VoxelKind::U8,
            data: VoxelStore::new(dims, VoxelKind::U8),
            levels: vec![Level {
                dims,
                spacing,
                offset: 0,
            }],
            present: SlicePresence::new(match dims {
                [_, _, depth] => depth,
            }),
            order: Vec::new(),
        }
    }

    fn plane_z(origin_z: f64, normal_z: f64) -> ClipPlane {
        ClipPlane {
            origin_mm: [0.0, 0.0, origin_z],
            normal: [0.0, 0.0, normal_z],
        }
    }

    fn point_of(volume: &Volume, index: [f64; 3]) -> [f64; 3] {
        let point = volume.voxel_world(index);
        [point.x, point.y, point.z]
    }

    fn f32_at(bytes: &[u8], offset: usize) -> f32 {
        let Some(end) = offset.checked_add(4) else {
            return f32::NAN;
        };
        let Some(chunk) = bytes.get(offset..end) else {
            return f32::NAN;
        };
        let Ok(array) = <[u8; 4]>::try_from(chunk) else {
            return f32::NAN;
        };
        f32::from_le_bytes(array)
    }

    #[test]
    fn an_empty_list_keeps_both_samples() {
        let volume = volume_at([1.0, 1.0, 10.0], [1, 1, 2]);
        let near = point_of(&volume, [0.0, 0.0, 0.0]);
        let far = point_of(&volume, [0.0, 0.0, 1.0]);
        assert_eq!(near[2].to_bits(), 0.0_f64.to_bits());
        assert_eq!(far[2].to_bits(), 10.0_f64.to_bits());
        let empty = ClipList::empty();
        assert!(empty.keeps(near));
        assert!(empty.keeps(far));
        // Stored 200 displays as 200 under LINEAR_EXACT centre 127.5 width 255.
        // Flat alpha is 0.5. Two resident steps from clear are 0.75 of that
        // colour. One kept step is 0.5 of it. 0.75 * 200/255 is 150/255.
        assert_eq!(
            ((200.0_f32 / 255.0) * 0.75).to_bits(),
            (150.0_f32 / 255.0).to_bits()
        );
        assert_eq!(
            ((200.0_f32 / 255.0) * 0.5).to_bits(),
            (100.0_f32 / 255.0).to_bits()
        );
    }

    #[test]
    fn a_plane_at_five_millimetres_drops_zero_and_keeps_ten() {
        let volume = volume_at([1.0, 1.0, 10.0], [1, 1, 2]);
        let near = point_of(&volume, [0.0, 0.0, 0.0]);
        let far = point_of(&volume, [0.0, 0.0, 1.0]);
        let list = match ClipList::from_planes(&[plane_z(5.0, 1.0)]) {
            Ok(list) => list,
            Err(_) => unreachable!("one finite normal"),
        };
        assert!(!list.keeps(near));
        assert!(list.keeps(far));
        let both = match ClipList::from_planes(&[plane_z(5.0, 1.0), plane_z(8.0, -1.0)]) {
            Ok(list) => list,
            Err(_) => unreachable!("two finite normals"),
        };
        assert!(!both.keeps(near));
        assert!(!both.keeps(far));
        let between = point_of(&volume, [0.0, 0.0, 0.5]);
        assert_eq!(between[2].to_bits(), 5.0_f64.to_bits());
        assert!(both.keeps(between));
        let frame = PatientFrame::identity();
        let bytes = list.uniform_bytes(&frame);
        assert_eq!(bytes.len(), 256);
        assert_eq!(f32_at(&bytes, 56).to_bits(), 1.0_f32.to_bits());
        assert_eq!(f32_at(&bytes, 72).to_bits(), 5.0_f32.to_bits());
        assert_eq!(f32_at(&bytes, 88).to_bits(), 1.0_f32.to_bits());
        assert_eq!(f32_at(&bytes, 92).to_bits(), 1.0_f32.to_bits());
        assert_eq!(f32_at(&bytes, 124).to_bits(), 0.0_f32.to_bits());
    }

    #[test]
    fn a_sample_on_the_plane_is_kept() {
        let volume = volume_at([1.0, 1.0, 1.0], [1, 1, 1]);
        let point = point_of(&volume, [0.0, 0.0, 0.0]);
        assert_eq!(point[2].to_bits(), 0.0_f64.to_bits());
        let list = match ClipList::from_planes(&[plane_z(0.0, 1.0)]) {
            Ok(list) => list,
            Err(_) => unreachable!("one finite normal"),
        };
        let facing = point[2] - 0.0;
        assert_eq!(facing.to_bits(), 0.0_f64.to_bits());
        assert!(list.keeps(point));
    }

    #[test]
    fn a_bad_list_leaves_the_previous_planes() {
        let previous = match ClipList::from_planes(&[plane_z(5.0, 1.0)]) {
            Ok(list) => list,
            Err(_) => unreachable!("one finite normal"),
        };
        let volume = volume_at([1.0, 1.0, 10.0], [1, 1, 2]);
        let near = point_of(&volume, [0.0, 0.0, 0.0]);
        let far = point_of(&volume, [0.0, 0.0, 1.0]);
        let mut seven = Vec::new();
        for _ in 0..7 {
            seven.push(plane_z(5.0, 1.0));
        }
        assert!(matches!(
            ClipList::from_planes(&seven),
            Err(ClipError::Count)
        ));
        assert!(matches!(
            ClipList::from_planes(&[ClipPlane {
                origin_mm: [f64::NAN, 0.0, 0.0],
                normal: [0.0, 0.0, 1.0],
            }]),
            Err(ClipError::NotFinite)
        ));
        assert!(matches!(
            ClipList::from_planes(&[ClipPlane {
                origin_mm: [0.0, 0.0, 0.0],
                normal: [f64::INFINITY, 0.0, 0.0],
            }]),
            Err(ClipError::NotFinite)
        ));
        assert!(matches!(
            ClipList::from_planes(&[ClipPlane {
                origin_mm: [0.0, 0.0, 0.0],
                normal: [1.0e40, 0.0, 0.0],
            }]),
            Err(ClipError::NotFinite)
        ));
        assert!(matches!(
            ClipList::from_planes(&[plane_z(0.0, 0.0)]),
            Err(ClipError::ZeroNormal)
        ));
        assert!(matches!(
            ClipList::from_planes(&[ClipPlane {
                origin_mm: [0.0, 0.0, 0.0],
                normal: [-0.0, 0.0, 0.0],
            }]),
            Err(ClipError::ZeroNormal)
        ));
        assert!(matches!(
            ClipList::from_planes(&[ClipPlane {
                origin_mm: [0.0, 0.0, 0.0],
                normal: [1.0e-46, 0.0, 0.0],
            }]),
            Err(ClipError::ZeroNormal)
        ));
        assert!(matches!(
            ClipList::from_planes(&[ClipPlane {
                origin_mm: [1.0e-46, 0.0, 0.0],
                normal: [0.0, 0.0, 1.0],
            }]),
            Err(ClipError::NotFinite)
        ));
        assert!(!previous.keeps(near));
        assert!(previous.keeps(far));
    }

    #[test]
    fn a_normal_whose_square_underflows_is_kept() {
        // 1e-23 is a normal f32. Its square is below every f32 subnormal.
        assert!(
            ClipList::from_planes(&[ClipPlane {
                origin_mm: [0.0, 0.0, 0.0],
                normal: [1.0e-23, 0.0, 0.0],
            }])
            .is_ok()
        );
    }

    #[test]
    fn six_planes_fit_and_the_identity_frame_narrows() {
        let mut planes = Vec::new();
        for _ in 0..6 {
            planes.push(plane_z(0.0, 1.0));
        }
        assert!(ClipList::from_planes(&planes).is_ok());
        let frame = PatientFrame::try_new(
            [0.0, 0.0, 0.0],
            [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]],
        );
        assert!(frame.is_some());
        assert!(
            PatientFrame::try_new(
                [f64::NAN, 0.0, 0.0],
                [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]],
            )
            .is_none()
        );
    }

    #[test]
    fn the_same_bits_match_and_a_new_plane_does_not() {
        let first = match ClipList::from_planes(&[plane_z(5.0, 1.0)]) {
            Ok(list) => list,
            Err(_) => unreachable!("one finite normal"),
        };
        let second = match ClipList::from_planes(&[plane_z(5.0, 1.0)]) {
            Ok(list) => list,
            Err(_) => unreachable!("one finite normal"),
        };
        let moved = match ClipList::from_planes(&[plane_z(6.0, 1.0)]) {
            Ok(list) => list,
            Err(_) => unreachable!("one finite normal"),
        };
        assert!(first.same(&second));
        assert!(!first.same(&moved));
        assert!(first.same(&first));
        assert!(!ClipList::empty().same(&first));
    }
}
