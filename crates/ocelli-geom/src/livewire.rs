//! 4-connected livewire over a caller-supplied cost grid.
//!
//! The path cost is the sum of the costs of the pixels entered. The start
//! pixel adds nothing. `z` is ignored and every returned point stores 0.
//! The mask marks visited pixels and does not fill an interior. The rules
//! are the local plan in `.claude/plans/F-077-design.md`. The HLD names
//! interactive segmentation and does not define this cost.

use alloc::vec::Vec;

use glam::DVec2;
use ocelli_core::Pt;

use crate::GeomError;

/// Column `x` and row `y` as a finite integer strictly inside `limit`.
///
/// Every `u32` is an exact `f64`, so a value past the last legal index is
/// outside the grid. The index is counted by subtracting one, which avoids
/// a cast.
fn integer_index(value: f64, limit: u32) -> Result<u32, GeomError> {
    if !value.is_finite() {
        return Err(GeomError::Frame);
    }
    if value.total_cmp(&0.0).is_lt() {
        return Err(GeomError::Degenerate);
    }
    let truncated = DVec2::new(value, 0.0).trunc().x;
    if truncated.total_cmp(&value).is_ne() {
        return Err(GeomError::Degenerate);
    }
    if limit == 0 {
        return Err(GeomError::Degenerate);
    }
    let last = f64::from(limit - 1);
    if value.total_cmp(&last).is_gt() {
        return Err(GeomError::Degenerate);
    }
    let mut remaining = value;
    let mut index = 0u32;
    while index < limit {
        if remaining.total_cmp(&0.0).is_eq() {
            return Ok(index);
        }
        remaining -= 1.0;
        index = match index.checked_add(1) {
            Some(next) => next,
            None => return Err(GeomError::Degenerate),
        };
    }
    Err(GeomError::Degenerate)
}

fn usize_from_u32(value: u32) -> Result<usize, GeomError> {
    usize::try_from(value).map_err(|_| GeomError::Degenerate)
}

fn cell_count(width: u32, height: u32) -> Result<usize, GeomError> {
    if width == 0 || height == 0 {
        return Err(GeomError::Degenerate);
    }
    let width_us = usize_from_u32(width)?;
    let height_us = usize_from_u32(height)?;
    width_us.checked_mul(height_us).ok_or(GeomError::Degenerate)
}

fn flat_index(x: u32, y: u32, width: u32) -> Result<usize, GeomError> {
    let x_us = usize_from_u32(x)?;
    let y_us = usize_from_u32(y)?;
    let width_us = usize_from_u32(width)?;
    y_us.checked_mul(width_us)
        .and_then(|row| row.checked_add(x_us))
        .ok_or(GeomError::Degenerate)
}

fn point_from_index<S>(index: usize, width: u32) -> Result<Pt<S>, GeomError> {
    let width_us = usize_from_u32(width)?;
    if width_us == 0 {
        return Err(GeomError::Degenerate);
    }
    let x_us = index % width_us;
    let y_us = index / width_us;
    let Ok(x) = u32::try_from(x_us) else {
        return Err(GeomError::Degenerate);
    };
    let Ok(y) = u32::try_from(y_us) else {
        return Err(GeomError::Degenerate);
    };
    Ok(Pt::new(f64::from(x), f64::from(y), 0.0))
}

fn validate_costs(costs: &[f64], cells: usize) -> Result<(), GeomError> {
    if costs.len() != cells {
        return Err(GeomError::Degenerate);
    }
    for &cost in costs {
        if !cost.is_finite() {
            return Err(GeomError::Frame);
        }
        if cost.total_cmp(&0.0).is_lt() {
            return Err(GeomError::Degenerate);
        }
    }
    Ok(())
}

fn cheapest_unsettled(distance: &[f64], settled: &[bool]) -> Option<usize> {
    let mut best_index = None;
    let mut best_cost = 0.0;
    for (index, &cost) in distance.iter().enumerate() {
        let settled_here = settled.get(index).copied().unwrap_or(true);
        if settled_here || !cost.is_finite() {
            continue;
        }
        let replace = match best_index {
            None => true,
            Some(_) => cost.total_cmp(&best_cost).is_lt(),
        };
        if replace {
            best_index = Some(index);
            best_cost = cost;
        }
    }
    best_index
}

/// Enter `neighbor`. A candidate replaces the stored cost only when it is
/// strictly smaller. An equal cost keeps the parent already stored.
fn relax(
    costs: &[f64],
    distance: &mut [f64],
    parent: &mut [Option<usize>],
    settled: &[bool],
    current: usize,
    neighbor: usize,
) -> Result<(), GeomError> {
    if settled.get(neighbor).copied() == Some(true) {
        return Ok(());
    }
    let Some(step) = costs.get(neighbor).copied() else {
        return Err(GeomError::Degenerate);
    };
    let Some(current_dist) = distance.get(current).copied() else {
        return Err(GeomError::Degenerate);
    };
    let candidate = current_dist + step;
    if !candidate.is_finite() {
        return Err(GeomError::Frame);
    }
    let Some(known) = distance.get(neighbor).copied() else {
        return Err(GeomError::Degenerate);
    };
    if !candidate.total_cmp(&known).is_lt() {
        return Ok(());
    }
    let Some(slot) = distance.get_mut(neighbor) else {
        return Err(GeomError::Degenerate);
    };
    *slot = candidate;
    let Some(slot) = parent.get_mut(neighbor) else {
        return Err(GeomError::Degenerate);
    };
    *slot = Some(current);
    Ok(())
}

fn relax_neighbours(
    costs: &[f64],
    distance: &mut [f64],
    parent: &mut [Option<usize>],
    settled: &[bool],
    current: usize,
    width: u32,
    cells: usize,
) -> Result<(), GeomError> {
    let width_us = usize_from_u32(width)?;
    if width_us == 0 {
        return Err(GeomError::Degenerate);
    }
    let x = current % width_us;
    let y = current / width_us;
    if x > 0 {
        relax(costs, distance, parent, settled, current, current - 1)?;
    }
    if x + 1 < width_us {
        relax(costs, distance, parent, settled, current, current + 1)?;
    }
    if y > 0 {
        let above = current.checked_sub(width_us).ok_or(GeomError::Degenerate)?;
        relax(costs, distance, parent, settled, current, above)?;
    }
    if let Some(below) = current.checked_add(width_us)
        && below < cells
    {
        relax(costs, distance, parent, settled, current, below)?;
    }
    Ok(())
}

fn assemble_path<S>(
    parent: &[Option<usize>],
    goal: usize,
    start: usize,
    width: u32,
    cells: usize,
) -> Result<Vec<Pt<S>>, GeomError> {
    let mut chain = Vec::new();
    let mut cursor = Some(goal);
    let mut guard = cells;
    while let Some(index) = cursor {
        if guard == 0 {
            return Err(GeomError::Degenerate);
        }
        guard -= 1;
        chain.push(index);
        let Some(next) = parent.get(index).copied() else {
            return Err(GeomError::Degenerate);
        };
        if next == Some(index) {
            return Err(GeomError::Degenerate);
        }
        cursor = next;
    }
    chain.reverse();
    if chain.first().copied() != Some(start) {
        return Err(GeomError::Degenerate);
    }
    let mut path = Vec::new();
    for index in chain {
        path.push(point_from_index(index, width)?);
    }
    Ok(path)
}

fn search<S>(
    costs: &[f64],
    width: u32,
    start: usize,
    goal: usize,
    cells: usize,
) -> Result<Vec<Pt<S>>, GeomError> {
    let mut distance = alloc::vec![f64::INFINITY; cells];
    let mut parent: Vec<Option<usize>> = alloc::vec![None; cells];
    let mut settled = alloc::vec![false; cells];
    let Some(origin) = distance.get_mut(start) else {
        return Err(GeomError::Degenerate);
    };
    *origin = 0.0;
    let mut guard = cells;
    loop {
        if guard == 0 {
            break;
        }
        guard -= 1;
        let Some(current) = cheapest_unsettled(&distance, &settled) else {
            break;
        };
        let Some(flag) = settled.get_mut(current) else {
            return Err(GeomError::Degenerate);
        };
        *flag = true;
        if current == goal {
            break;
        }
        relax_neighbours(
            costs,
            &mut distance,
            &mut parent,
            &settled,
            current,
            width,
            cells,
        )?;
    }
    let Some(goal_dist) = distance.get(goal).copied() else {
        return Err(GeomError::Degenerate);
    };
    if !goal_dist.is_finite() {
        return Err(GeomError::Degenerate);
    }
    assemble_path(&parent, goal, start, width, cells)
}

/// One 4-connected path from `start` to `goal`.
///
/// `costs` is row-major, one finite non-negative value per pixel. `x` and
/// `y` are pixel indices. `z` is ignored.
///
/// # Errors
///
/// [`GeomError::Frame`] when a cost or an index is non-finite.
/// [`GeomError::Degenerate`] when a dimension is 0, the buffer length does
/// not match, a cost is negative, or an index is not an in-range integer.
/// A start that equals the goal is one point, not an error.
pub fn livewire_path<S>(
    costs: &[f64],
    width: u32,
    height: u32,
    start: Pt<S>,
    goal: Pt<S>,
) -> Result<Vec<Pt<S>>, GeomError> {
    let cells = cell_count(width, height)?;
    validate_costs(costs, cells)?;
    let start_x = integer_index(start.x, width)?;
    let start_y = integer_index(start.y, height)?;
    let goal_x = integer_index(goal.x, width)?;
    let goal_y = integer_index(goal.y, height)?;
    if start_x == goal_x && start_y == goal_y {
        return Ok(alloc::vec![Pt::new(
            f64::from(start_x),
            f64::from(start_y),
            0.0,
        )]);
    }
    let start_index = flat_index(start_x, start_y, width)?;
    let goal_index = flat_index(goal_x, goal_y, width)?;
    search(costs, width, start_index, goal_index, cells)
}

/// One byte per pixel. A visited path pixel is 1 and every other pixel is 0.
///
/// The interior is not filled. An empty path is refused and no mask is
/// returned.
///
/// # Errors
///
/// [`GeomError::Frame`] when a path index is non-finite.
/// [`GeomError::Degenerate`] when the path is empty, a dimension is 0, or a
/// path pixel is outside the grid.
pub fn livewire_mask<S>(width: u32, height: u32, path: &[Pt<S>]) -> Result<Vec<u8>, GeomError> {
    if path.is_empty() {
        return Err(GeomError::Degenerate);
    }
    let cells = cell_count(width, height)?;
    let mut indices = Vec::new();
    for point in path {
        let x = integer_index(point.x, width)?;
        let y = integer_index(point.y, height)?;
        indices.push(flat_index(x, y, width)?);
    }
    let mut mask = alloc::vec![0u8; cells];
    for index in indices {
        let Some(slot) = mask.get_mut(index) else {
            return Err(GeomError::Degenerate);
        };
        *slot = 1;
    }
    Ok(mask)
}
