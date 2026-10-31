//! Viewports that share one `FrameOfReferenceUID` keep one patient point.
//!
//! The UID is compared as a string. A missing UID moves nobody. The index
//! is `HeldPlane::index_through_point`. This module creates no device.

use alloc::vec::Vec;

use ocelli_core::{Pt, World};
use ocelli_volume::{FrameOfReferenceUid, HeldPlane, PlaneSyncError};

/// Registered planes. Slots stay in registration order.
#[derive(Clone, Debug, Default)]
pub struct SyncScene {
    slots: Vec<Slot>,
}

#[derive(Clone, Debug)]
struct Slot {
    id: u16,
    uid: Option<FrameOfReferenceUid>,
    plane: HeldPlane,
}

/// One viewport whose index changed.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SliceChange {
    /// The registered viewport.
    pub id: u16,
    /// The index now stored on that slot.
    pub index: f64,
}

/// The scene did not move a plane.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SyncError {
    /// The source is unknown, its UID is missing, or a same-UID plane
    /// cannot name an index. No slot was written.
    Unavailable,
}

impl SyncScene {
    /// An empty scene.
    pub fn new() -> Self {
        Self { slots: Vec::new() }
    }

    /// Store `plane` on `id`. A repeated id replaces that slot only.
    pub fn register(&mut self, id: u16, uid: Option<FrameOfReferenceUid>, plane: HeldPlane) {
        if let Some(slot) = self.slots.iter_mut().find(|slot| slot.id == id) {
            slot.uid = uid;
            slot.plane = plane;
            return;
        }
        self.slots.push(Slot { id, uid, plane });
    }

    /// The plane stored for `id`, if it was registered.
    pub fn plane(&self, id: u16) -> Option<HeldPlane> {
        self.slots
            .iter()
            .find(|slot| slot.id == id)
            .map(|slot| slot.plane)
    }

    /// Move every plane that shares the source UID onto `point`.
    ///
    /// The change list is registration order and omits an index that did
    /// not change. A different UID stays. A missing peer UID stays.
    ///
    /// # Errors
    ///
    /// [`SyncError::Unavailable`] when `source` was not registered, when
    /// its UID is missing, or when any same-UID plane refuses the point.
    /// No slot is written.
    pub fn set_point(
        &mut self,
        source: u16,
        point: Pt<World>,
    ) -> Result<Vec<SliceChange>, SyncError> {
        let Some(source_at) = self.slots.iter().position(|slot| slot.id == source) else {
            return Err(SyncError::Unavailable);
        };
        let Some(uid) = self.slots.get(source_at).and_then(|slot| slot.uid.clone()) else {
            return Err(SyncError::Unavailable);
        };
        let mut staged = Vec::new();
        for (at, slot) in self.slots.iter().enumerate() {
            let Some(peer) = slot.uid.as_ref() else {
                continue;
            };
            if peer != &uid {
                continue;
            }
            match slot.plane.index_through_point(point) {
                Ok(index) => staged.push((at, index)),
                Err(PlaneSyncError::Unavailable) => return Err(SyncError::Unavailable),
            }
        }
        let mut changes = Vec::new();
        for (at, index) in &staged {
            let Some(slot) = self.slots.get(*at) else {
                return Err(SyncError::Unavailable);
            };
            if same_index(slot.plane.index, *index) {
                continue;
            }
            changes.push(SliceChange {
                id: slot.id,
                index: *index,
            });
        }
        for change in &changes {
            for slot in &mut self.slots {
                if slot.id == change.id {
                    slot.plane.index = change.index;
                    break;
                }
            }
        }
        Ok(changes)
    }
}

/// `==` on the two indexes, including the two zeros, without comparing floats.
fn same_index(stored: f64, next: f64) -> bool {
    let bits = (stored - next).to_bits();
    bits == 0 || bits == (-0.0_f64).to_bits()
}
