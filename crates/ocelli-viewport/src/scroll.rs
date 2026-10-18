//! Stack scroll in stored order.
//!
//! HLD section 8 is the decoded LRU this type reads. HLD section 22 names
//! `Dirty.slice` for a change of index. A prefetch insert is not that change.
//! HLD section 30 is volume brick prefetch and is not this path. The radius
//! is one, from the F-047 plan.

use alloc::vec::Vec;

use ocelli_cache::{FrameKey, ImageCache};

/// Why [`StackSequence::samples_for_upload`] returned no samples.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScrollError {
    /// The sequence has no frames, so it has no index.
    Empty,
    /// The selected key is not in the image cache. No neighbour is returned.
    NotResident,
}

/// One stack in the order the caller stored it.
///
/// Instance number, slice location and image position do not reorder the
/// list. `scroll_by` clamps. It does not wrap. Prefetch is the index minus
/// one and plus one, when those indices exist.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StackSequence {
    keys: Vec<FrameKey>,
    index: u32,
}

impl StackSequence {
    /// Store `keys` in the order given. The first frame is selected.
    #[must_use]
    pub fn new(keys: &[FrameKey]) -> Self {
        Self {
            keys: keys.to_vec(),
            index: 0,
        }
    }

    /// The selected index, or [`None`] when the sequence is empty.
    #[must_use]
    pub fn index(&self) -> Option<u32> {
        if self.keys.is_empty() {
            None
        } else {
            Some(self.index)
        }
    }

    /// The selected frame. [`None`] when the sequence is empty.
    #[must_use]
    pub fn displayed_key(&self) -> Option<FrameKey> {
        self.key_at(self.index)
    }

    /// The neighbour below in slot 0 and the neighbour above in slot 1.
    ///
    /// A missing side is [`None`]. The selected key is not included. An
    /// empty sequence is `[None, None]`.
    #[must_use]
    pub fn prefetch_keys(&self) -> [Option<FrameKey>; 2] {
        let Some(index) = self.index() else {
            return [None, None];
        };
        let below = index.checked_sub(1).and_then(|lower| self.key_at(lower));
        let above = index.checked_add(1).and_then(|higher| self.key_at(higher));
        [below, above]
    }

    /// Add `delta` and clamp to the stored list. An empty sequence stays empty.
    pub fn scroll_by(&mut self, delta: i32) {
        let Some(len) = u32::try_from(self.keys.len()).ok() else {
            return;
        };
        let Some(last) = len.checked_sub(1) else {
            return;
        };
        let next = i64::from(self.index).saturating_add(i64::from(delta));
        let clamped = next.clamp(0, i64::from(last));
        self.index = u32::try_from(clamped).unwrap_or(0);
    }

    /// Samples for the selected key only.
    ///
    /// # Errors
    ///
    /// [`ScrollError::Empty`] when the sequence has no frames.
    /// [`ScrollError::NotResident`] when that key is absent. The bytes of a
    /// neighbour are not returned.
    pub fn samples_for_upload<'a>(
        &self,
        cache: &'a mut ImageCache,
    ) -> Result<&'a [u8], ScrollError> {
        let Some(key) = self.displayed_key() else {
            return Err(ScrollError::Empty);
        };
        match cache.get(&key) {
            Some(frame) => Ok(frame.samples()),
            None => Err(ScrollError::NotResident),
        }
    }

    fn key_at(&self, index: u32) -> Option<FrameKey> {
        // `usize::from(u32)` is not implemented. A `u32` that does not fit
        // the target `usize` is not an index this list can name.
        let index = usize::try_from(index).ok()?;
        self.keys.get(index).copied()
    }
}
