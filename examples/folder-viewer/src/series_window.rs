//! The series window, F-227.
//!
//! One automatic window for every image of a series that stores none, from
//! the operator's answer of 2026-11-05: up to 16 images spread through the
//! series, pooled once. This module chooses nothing and computes nothing of
//! its own. `ocelli_pixel::series_sample` names the images, and
//! `AutoWindowPool` folds each one through its own modality stage, PS3.3
//! C.11.1, leaving out stored zero and its own padding, PS3.3 C.7.5.1.1.2.
//! Here a sampled image is only fetched, checked for a window of its own and
//! unpacked.
//!
//! Each sampled image is read, unpacked and folded inside one decode job,
//! under its permit. The frame is unpacked into the permit's own scratch,
//! which the permit keeps for its next job inside F-216's permit term. The
//! decoded samples exist only inside the job, which parks them on the free
//! list, inside `OCELLI_CACHE_MIB`, when cached and free bytes fit the
//! budget, and frees them otherwise. The pool moves
//! into each job and back. It reserves at most 2^20 `f32` values, 4 MiB,
//! when it is built and keeps at most `AUTO_WINDOW_SAMPLE_CAP / K` of them
//! from each of `K` sampled images, whatever the frame size. At most
//! `SERIES_ROUNDS` rounds run at once, so the reservations are F-216's named
//! `P * 4 MiB` term.

use ocelli_core::Stored;
use ocelli_pixel::{AutoWindow, AutoWindowPool, series_sample};

use crate::held::FileId;
use crate::preview::{FileWindow, HeldFrame, Recipe};

/// The window of one series and how it was made.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct SeriesWindow {
    /// The pooled window.
    pub(crate) window: AutoWindow,
    /// Images `series_sample` chose.
    pub(crate) sampled: usize,
    /// Chosen images that contributed nothing: a window of their own, a VOI
    /// LUT Sequence, a refused decode, a padding range that cannot be read,
    /// a frame the pool refused, or an image the page could not offer, such
    /// as a file changed on disk. None is replaced by a neighbour.
    pub(crate) skipped: usize,
}

/// What a decode job did with one sampled image.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Folded {
    /// The image's values are in the pool.
    Added,
    /// The image contributed nothing: refused, or a window of its own.
    Skipped,
    /// The id left the cache between the two passes.
    Gone,
    /// No decode permit was free, or the peer left.
    Busy,
}

/// Why no series window was made.
#[derive(Clone, PartialEq, Eq)]
pub(crate) enum SeriesRefusal {
    /// These sampled ids are not held, every one of them, in the order they
    /// were sampled. The page posts each file again and asks once more.
    Gone(Vec<FileId>),
    /// A decode could not be scheduled.
    Busy,
    /// No sampled image contributed, so the images use their own windows.
    Nothing,
}

/// Pools made, F-227 AC14, by the viewer generation of the list's first id.
/// Counted where the pool is built, so a pool made on any other path is
/// seen, and per generation, so a test running beside on another viewer
/// cannot move another test's count.
#[cfg(test)]
static POOLS: std::sync::Mutex<Vec<(u64, u64)>> = std::sync::Mutex::new(Vec::new());

#[cfg(test)]
fn note_pool(images: &[(Option<FileId>, u32)]) {
    let Some(generation) = images.iter().find_map(|(id, _)| id.map(|id| id.generation)) else {
        return;
    };
    let mut pools = POOLS
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    match pools.iter_mut().find(|(seen, _)| *seen == generation) {
        Some((_, count)) => *count = count.saturating_add(1),
        None => pools.push((generation, 1)),
    }
}

/// Rounds running now and the most seen at once, by viewer generation,
/// counted inside the round whatever admitted it.
#[cfg(test)]
static ROUNDS: std::sync::Mutex<Vec<(u64, usize, usize)>> = std::sync::Mutex::new(Vec::new());

#[cfg(test)]
struct RoundCount(Option<u64>);

#[cfg(test)]
impl RoundCount {
    fn enter(images: &[(Option<FileId>, u32)]) -> Self {
        let generation = images.iter().find_map(|(id, _)| id.map(|id| id.generation));
        if let Some(generation) = generation {
            let mut rounds = ROUNDS
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            match rounds.iter_mut().find(|(seen, _, _)| *seen == generation) {
                Some((_, now, peak)) => {
                    *now = now.saturating_add(1);
                    *peak = (*peak).max(*now);
                }
                None => rounds.push((generation, 1, 1)),
            }
        }
        Self(generation)
    }
}

#[cfg(test)]
impl Drop for RoundCount {
    fn drop(&mut self) {
        let Some(generation) = self.0 else {
            return;
        };
        let mut rounds = ROUNDS
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if let Some((_, now, _)) = rounds.iter_mut().find(|(seen, _, _)| *seen == generation) {
            *now = now.saturating_sub(1);
        }
    }
}

/// The most series window rounds seen at once for viewer `generation`.
#[cfg(test)]
pub(crate) fn rounds_peak(generation: u64) -> usize {
    let rounds = ROUNDS
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    rounds
        .iter()
        .find(|(seen, _, _)| *seen == generation)
        .map_or(0, |(_, _, peak)| *peak)
}

/// Pools made for lists of ids of viewer `generation`.
#[cfg(test)]
pub(crate) fn pools_for(generation: u64) -> u64 {
    let pools = POOLS
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    pools
        .iter()
        .find(|(seen, _)| *seen == generation)
        .map_or(0, |(_, count)| *count)
}

/// Pool the sampled images of a series of `images`, in the page's order.
///
/// An image is its file id and frame, or `None` for one the page could not
/// offer, which is skipped when sampled. Two passes, review pass 16 D1:
/// `present` checks every sampled id first and keeps nothing, and when any is
/// gone the answer names every gone one and nothing is decoded. `load` then
/// takes the pool and one sampled id, folds that image in under a decode
/// permit, and gives the pool back. An id that leaves the cache between the
/// passes is gone too. A pool `load` does not give back, because its job did
/// not run, ends the round as busy.
/// The pool declares the sampled count up front, so a skipped image moves no
/// budget and the window stays a function of the sampled bytes alone.
///
/// # Errors
///
/// [`SeriesRefusal::Gone`] with every gone id, [`SeriesRefusal::Busy`]
/// when a decode cannot run, and [`SeriesRefusal::Nothing`] for an empty list
/// or when every sampled image is skipped.
pub(crate) fn series_window<F, G>(
    images: &[(Option<FileId>, u32)],
    mut present: F,
    mut load: G,
) -> Result<SeriesWindow, SeriesRefusal>
where
    F: FnMut(FileId, u32) -> bool,
    G: FnMut(FileId, u32, AutoWindowPool) -> (Option<AutoWindowPool>, Folded),
{
    #[cfg(test)]
    let _round = RoundCount::enter(images);
    let indices = series_sample(images.len()).map_err(|_| SeriesRefusal::Nothing)?;
    let pool = AutoWindowPool::new(indices.len()).map_err(|_| SeriesRefusal::Nothing)?;
    #[cfg(test)]
    note_pool(images);
    let mut skipped = 0_usize;
    let mut gone = Vec::new();
    let mut found = Vec::with_capacity(indices.len());
    for at in &indices {
        let &(id, frame) = images.get(*at).ok_or(SeriesRefusal::Nothing)?;
        let Some(id) = id else {
            skipped = skipped.saturating_add(1);
            continue;
        };
        if present(id, frame) {
            found.push((id, frame));
        } else {
            gone.push(id);
        }
    }
    if !gone.is_empty() {
        return Err(SeriesRefusal::Gone(gone));
    }
    let mut pool = pool;
    for (id, frame) in found {
        let (back, folded) = load(id, frame, pool);
        // The one busy check: a pool the job did not give back.
        pool = back.ok_or(SeriesRefusal::Busy)?;
        match folded {
            Folded::Added => {}
            Folded::Skipped => skipped = skipped.saturating_add(1),
            Folded::Gone => gone.push(id),
            Folded::Busy => return Err(SeriesRefusal::Busy),
        }
    }
    if !gone.is_empty() {
        return Err(SeriesRefusal::Gone(gone));
    }
    if skipped == indices.len() {
        return Err(SeriesRefusal::Nothing);
    }
    let window = pool.finish().map_err(|_| SeriesRefusal::Nothing)?;
    Ok(SeriesWindow {
        window,
        sampled: indices.len(),
        skipped,
    })
}

/// Fold one decoded frame into the pool when it stores no window of its own,
/// on the thread that holds the frame's decode permit.
///
/// The same eligibility as the per-image window: a usable recipe, no VOI LUT
/// Sequence, the automatic file window, and a padding range that could be
/// read. The frame is unpacked into `scratch`, the permit's own, which keeps
/// its capacity for the next decode under that permit.
pub(crate) fn fold(
    pool: &mut AutoWindowPool,
    held: &HeldFrame,
    scratch: &mut Vec<Stored>,
) -> Folded {
    let Ok(Recipe::Monochrome(recipe)) = held.recipe.as_ref() else {
        return Folded::Skipped;
    };
    if !matches!(recipe.voi_lut, Ok(false)) {
        return Folded::Skipped;
    }
    let Ok(FileWindow::Auto {
        padding: Ok(padding),
    }) = &recipe.file_window
    else {
        return Folded::Skipped;
    };
    let Ok(count) = held.description.sample_count() else {
        return Folded::Skipped;
    };
    if scratch.len() != count {
        scratch.resize(count, Stored(0.0));
    }
    if held
        .description
        .unpack(&held.samples, held.byte_order, scratch)
        .is_err()
    {
        return Folded::Skipped;
    }
    // A refused frame leaves the pool as it was, so it is skipped.
    match pool.add_frame(scratch, &recipe.modality, *padding) {
        Ok(()) => Folded::Added,
        Err(_) => Folded::Skipped,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn id(serial: u64) -> FileId {
        FileId {
            generation: 7,
            serial,
        }
    }

    /// Review pass 18 S2. An id present in the first pass and gone by its
    /// load is named in the refusal, never skipped or read as refused, so
    /// the page posts that file again rather than losing the series window.
    #[test]
    fn an_id_gone_between_the_passes_is_named() {
        let images = [(Some(id(1)), 0), (Some(id(2)), 0), (None, 0)];
        let made = series_window(
            &images,
            |_, _| true,
            |at, _, pool| {
                let folded = if at == id(2) {
                    Folded::Gone
                } else {
                    Folded::Skipped
                };
                (Some(pool), folded)
            },
        );
        assert!(made.is_err_and(|refusal| refusal == SeriesRefusal::Gone(vec![id(2)])));
    }

    /// A pool that a job did not give back ends the round as busy.
    #[test]
    fn a_pool_not_given_back_is_busy() {
        let images = [(Some(id(1)), 0), (Some(id(2)), 0)];
        let made = series_window(&images, |_, _| true, |_, _, _| (None, Folded::Added));
        assert!(made.is_err_and(|refusal| refusal == SeriesRefusal::Busy));
    }
}
