//! Labelmap fill and outline.
//!
//! DICOM PS3.3 C.8.20.2 says a segmentation pixel is a segment number and
//! that 0 is background. The colour table is addressed from segment 1.
//! Fill writes that RGBA8. Outline is the 4-connected boundary, one pixel,
//! and replaces the fill with alpha 255. The image VOI chain is not used.

use alloc::vec::Vec;

/// One displayed colour. The bytes are the texel.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rgba8 {
    /// Red.
    pub r: u8,
    /// Green.
    pub g: u8,
    /// Blue.
    pub b: u8,
    /// Straight alpha. 0 is clear and 255 is opaque.
    pub a: u8,
}

impl Rgba8 {
    fn bytes(self) -> [u8; 4] {
        [self.r, self.g, self.b, self.a]
    }
}

/// Colours for segment 1, then segment 2, and so on. There is no entry for 0.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LabelColourLut {
    from_segment_one: Vec<Rgba8>,
}

impl LabelColourLut {
    /// Build a table whose first entry is segment 1.
    pub fn from_segment_one(colours: Vec<Rgba8>) -> Self {
        Self {
            from_segment_one: colours,
        }
    }

    fn colour(&self, segment: u16) -> Option<Rgba8> {
        let number = usize::from(segment);
        let index = number.checked_sub(1)?;
        self.from_segment_one.get(index).copied()
    }
}

/// One row-major label grid. Index 0 is background.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LabelGrid {
    /// Columns.
    pub width: u32,
    /// Rows.
    pub height: u32,
    /// `width * height` segment numbers.
    pub indices: Vec<u16>,
}

/// Which parts of a labelled pixel are drawn.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LabelStyle {
    /// Write the LUT colour on labelled pixels that are not outline.
    pub fill: bool,
    /// Replace the 4-connected boundary with the same RGB and alpha 255.
    pub outline: bool,
}

/// Why a labelmap was not written.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LabelmapError {
    /// Width or height is 0, or does not fit the neighbour arithmetic.
    Empty,
    /// The index buffer is not `width * height`.
    Length,
    /// A labelled pixel has no colour. Nothing was written.
    MissingColour,
    /// Fill and outline are both off. Nothing was written.
    NothingToDraw,
    /// The destination is not four bytes per pixel.
    OutputLength,
}

const OUTLINE_ALPHA: u8 = 255;

/// Write fill and outline into `out`.
///
/// A refusal leaves `out` unchanged. Background pixels are transparent.
///
/// # Errors
///
/// [`LabelmapError`] when the grid, the table, the style or the destination
/// cannot be drawn. The destination is not modified in that case.
pub fn present_labelmap(
    grid: &LabelGrid,
    lut: &LabelColourLut,
    style: LabelStyle,
    out: &mut [u8],
) -> Result<(), LabelmapError> {
    if !style.fill && !style.outline {
        return Err(LabelmapError::NothingToDraw);
    }
    if grid.width == 0 || grid.height == 0 {
        return Err(LabelmapError::Empty);
    }
    let width = usize::try_from(grid.width).map_err(|_| LabelmapError::Empty)?;
    let height = usize::try_from(grid.height).map_err(|_| LabelmapError::Empty)?;
    let pixels = width.checked_mul(height).ok_or(LabelmapError::Empty)?;
    if grid.indices.len() != pixels {
        return Err(LabelmapError::Length);
    }
    let bytes = pixels.checked_mul(4).ok_or(LabelmapError::Empty)?;
    if out.len() != bytes {
        return Err(LabelmapError::OutputLength);
    }
    let mut pending = alloc::vec![0_u8; bytes];
    for y in 0..grid.height {
        for x in 0..grid.width {
            let offset = offset_of(grid, x, y)?;
            let segment = grid
                .indices
                .get(offset)
                .copied()
                .ok_or(LabelmapError::Length)?;
            let painted = if segment == 0 {
                Rgba8 {
                    r: 0,
                    g: 0,
                    b: 0,
                    a: 0,
                }
            } else {
                let colour = lut.colour(segment).ok_or(LabelmapError::MissingColour)?;
                let boundary = neighbour_differs(grid, x, y, segment)?;
                paint(colour, boundary, style)
            };
            let start = offset.checked_mul(4).ok_or(LabelmapError::Empty)?;
            let end = start.checked_add(4).ok_or(LabelmapError::Empty)?;
            let Some(slot) = pending.get_mut(start..end) else {
                return Err(LabelmapError::OutputLength);
            };
            slot.copy_from_slice(&painted.bytes());
        }
    }
    out.copy_from_slice(&pending);
    Ok(())
}

fn paint(colour: Rgba8, boundary: bool, style: LabelStyle) -> Rgba8 {
    if boundary && style.outline {
        return Rgba8 {
            r: colour.r,
            g: colour.g,
            b: colour.b,
            a: OUTLINE_ALPHA,
        };
    }
    if style.fill {
        return colour;
    }
    Rgba8 {
        r: 0,
        g: 0,
        b: 0,
        a: 0,
    }
}

fn offset_of(grid: &LabelGrid, x: u32, y: u32) -> Result<usize, LabelmapError> {
    let row = y.checked_mul(grid.width).ok_or(LabelmapError::Empty)?;
    let offset = row.checked_add(x).ok_or(LabelmapError::Empty)?;
    usize::try_from(offset).map_err(|_| LabelmapError::Empty)
}

fn neighbour_differs(grid: &LabelGrid, x: u32, y: u32, here: u16) -> Result<bool, LabelmapError> {
    let steps: [(i32, i32); 4] = [(1, 0), (-1, 0), (0, 1), (0, -1)];
    let x_i = i32::try_from(x).map_err(|_| LabelmapError::Empty)?;
    let y_i = i32::try_from(y).map_err(|_| LabelmapError::Empty)?;
    let width_i = i32::try_from(grid.width).map_err(|_| LabelmapError::Empty)?;
    let height_i = i32::try_from(grid.height).map_err(|_| LabelmapError::Empty)?;
    for (step_x, step_y) in steps {
        let next_x = x_i.checked_add(step_x).ok_or(LabelmapError::Empty)?;
        let next_y = y_i.checked_add(step_y).ok_or(LabelmapError::Empty)?;
        if next_x < 0 || next_y < 0 || next_x >= width_i || next_y >= height_i {
            return Ok(true);
        }
        let next_x_u = u32::try_from(next_x).map_err(|_| LabelmapError::Empty)?;
        let next_y_u = u32::try_from(next_y).map_err(|_| LabelmapError::Empty)?;
        let offset = offset_of(grid, next_x_u, next_y_u)?;
        let neighbour = grid
            .indices
            .get(offset)
            .copied()
            .ok_or(LabelmapError::Length)?;
        if neighbour != here {
            return Ok(true);
        }
    }
    Ok(false)
}
