//! F-057. Upload a ready brick when its bytes change.
//!
//! The upload unit is one brick from [`Volume::ready_bricks`], not a slice
//! and not a zero-filled brick. A brick that list omits has no texture.
//! Absence is the lack of a texture. Stored zero is a plausible pixel, so
//! this module does not invent it.
//!
//! Level 0 only. A coarser level appended after level 0 is not uploaded.
//! The stamp is the brick's bytes. An equal stamp does not upload again.

use std::collections::BTreeMap;

use ocelli_cache::{Brick, BrickKey, VolumeId};
use ocelli_volume::{Volume, VolumeError};

use crate::brick_upload::BrickUploadError;
use crate::caps::Tier;

/// What one `sync_volume` did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProgressReport {
    /// Slices whose presence bit is set.
    pub present_slices: u32,
    /// `dims[2]`, the number of slices the volume holds.
    pub slice_count: u32,
    /// Bricks uploaded by this call.
    pub uploaded: u32,
    /// Grid bricks that this call did not upload.
    pub skipped: u32,
}

/// Why a progressive sync uploaded nothing, or stopped part way.
///
/// An upload refusal keeps the cut brick. A tier C refusal cuts nothing.
#[derive(Debug, PartialEq, thiserror::Error)]
pub enum ProgressError {
    /// The device refused the brick, or tier C refused before a cut.
    #[error("{source}")]
    Upload {
        /// The brick `upload_brick` gave back. `None` on tier C.
        brick: Option<Brick>,
        /// The upload refusal.
        #[source]
        source: BrickUploadError,
    },
    /// [`Volume::cut_ready_brick`] refused. That brick was not uploaded.
    #[error(transparent)]
    Volume(#[from] VolumeError),
    /// The brick grid product does not fit in the progress count.
    #[error("the brick grid product does not fit in the progress count")]
    Grid,
}

/// Walk ready bricks and upload the ones whose bytes changed.
///
/// Tier C returns [`BrickUploadError::Unavailable`] before a cut and before
/// any stamp write. `upload` is `RenderGraph::upload_brick` for a real
/// device. The floor test supplies its own and asserts it is not called.
///
/// # Errors
///
/// [`ProgressError::Upload`] on tier C and on an upload refusal.
/// [`ProgressError::Volume`] when the cut refuses a brick `ready_bricks` listed.
/// [`ProgressError::Grid`] when the grid product does not fit in `u32`.
pub fn sync_ready_bricks(
    tier: Tier,
    stamps: &mut BTreeMap<BrickKey, Vec<u8>>,
    volume_id: VolumeId,
    volume: &Volume,
    mut upload: impl FnMut(BrickKey, Brick) -> Result<(), ProgressError>,
) -> Result<ProgressReport, ProgressError> {
    if tier == Tier::Cpu {
        return Err(ProgressError::Upload {
            brick: None,
            source: BrickUploadError::Unavailable,
        });
    }
    let count = grid_count(volume.brick_grid().grid).ok_or(ProgressError::Grid)?;
    let mut uploaded = 0u32;
    for coord in volume.ready_bricks() {
        let brick = volume.cut_ready_brick(coord)?;
        let key = BrickKey {
            volume: volume_id,
            level: 0,
            brick: coord,
        };
        if stamps.get(&key).is_some_and(|held| held == &brick.samples) {
            continue;
        }
        let bytes = brick.samples.clone();
        upload(key, brick)?;
        stamps.insert(key, bytes);
        uploaded = uploaded.checked_add(1).ok_or(ProgressError::Grid)?;
    }
    report(volume, count, uploaded)
}

fn grid_count(grid: [u32; 3]) -> Option<u32> {
    let [x, y, z] = grid;
    x.checked_mul(y)?.checked_mul(z)
}

fn report(volume: &Volume, count: u32, uploaded: u32) -> Result<ProgressReport, ProgressError> {
    let skipped = count.checked_sub(uploaded).ok_or(ProgressError::Grid)?;
    Ok(ProgressReport {
        present_slices: volume.present_count(),
        slice_count: volume.dims[2],
        uploaded,
        skipped,
    })
}
