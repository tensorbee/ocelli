//! F-063 labelmap fill and outline.
//!
//! DICOM PS3.3 C.8.20.2: a segmentation pixel is a segment number. Pixel
//! value 0 is background and is not a segment number. Segment numbers start
//! at 1.
//!
//! The colour and the outline rule are the approved F-063 plan. Fill writes
//! the segment RGBA8. Outline is the one-pixel 4-connected boundary. A
//! neighbour outside the grid counts as background. An outline pixel keeps
//! the segment RGB and uses alpha 255. A refusal leaves the destination
//! unchanged.

use ocelli_seg::{LabelColourLut, LabelGrid, LabelStyle, LabelmapError, Rgba8, present_labelmap};

/// Segment 1 in the hand fixture. Not a VOI grey.
fn segment_one() -> Rgba8 {
    Rgba8 {
        r: 10,
        g: 20,
        b: 30,
        a: 128,
    }
}

fn lut_one() -> LabelColourLut {
    LabelColourLut::from_segment_one(vec![segment_one()])
}

/// 3 by 3, every pixel segment 1.
///
/// Row-major index 4 is the centre. Its four neighbours are also segment 1,
/// so it is fill. Every other pixel has a neighbour outside the grid, so it
/// is outline.
fn grid_all_one() -> LabelGrid {
    LabelGrid {
        width: 3,
        height: 3,
        indices: vec![1, 1, 1, 1, 1, 1, 1, 1, 1],
    }
}

fn style(fill: bool, outline: bool) -> LabelStyle {
    LabelStyle { fill, outline }
}

fn pixel(bytes: &[u8], index: usize) -> Option<[u8; 4]> {
    let start = index.checked_mul(4)?;
    let end = start.checked_add(4)?;
    let slot = bytes.get(start..end)?;
    let mut out = [0_u8; 4];
    out.copy_from_slice(slot);
    Some(out)
}

fn paint(
    grid: &LabelGrid,
    lut: &LabelColourLut,
    drawn: LabelStyle,
    out: &mut [u8],
) -> Result<(), LabelmapError> {
    present_labelmap(grid, lut, drawn, out)
}

#[test]
fn labelmap_fill_and_outline_use_the_segment_colour() {
    // Centre byte offset is 4 * 4 = 16. Fill alpha stays 128.
    // An edge, index 1, is outline because the neighbour above the grid is
    // background. Outline alpha is 255. A mutation that leaves 128 there fails.
    let mut out = vec![0_u8; 36];
    assert!(paint(&grid_all_one(), &lut_one(), style(true, true), &mut out).is_ok());
    assert_eq!(pixel(&out, 4), Some([10, 20, 30, 128]));
    assert_eq!(pixel(&out, 0), Some([10, 20, 30, 255]));
    assert_eq!(pixel(&out, 1), Some([10, 20, 30, 255]));
    assert_eq!(pixel(&out, 5), Some([10, 20, 30, 255]));
    assert_eq!(pixel(&out, 7), Some([10, 20, 30, 255]));
}

#[test]
fn outline_off_keeps_the_fill_alpha() {
    let mut out = vec![0_u8; 36];
    assert!(paint(&grid_all_one(), &lut_one(), style(true, false), &mut out).is_ok());
    assert_eq!(pixel(&out, 0), Some([10, 20, 30, 128]));
    assert_eq!(pixel(&out, 4), Some([10, 20, 30, 128]));
}

#[test]
fn fill_off_draws_only_the_boundary() {
    let mut out = vec![0_u8; 36];
    assert!(paint(&grid_all_one(), &lut_one(), style(false, true), &mut out).is_ok());
    assert_eq!(pixel(&out, 4), Some([0, 0, 0, 0]));
    assert_eq!(pixel(&out, 0), Some([10, 20, 30, 255]));
    assert_eq!(pixel(&out, 1), Some([10, 20, 30, 255]));
}

#[test]
fn a_missing_colour_writes_nothing() {
    // Index 0 is segment 1 and would be paintable. Index 1 is segment 2 and
    // has no LUT entry. A per-pixel commit would change the first four bytes.
    let grid = LabelGrid {
        width: 2,
        height: 1,
        indices: vec![1, 2],
    };
    let mut out = vec![0xA5_u8; 8];
    let before = out.clone();
    assert_eq!(
        paint(&grid, &lut_one(), style(true, true), &mut out),
        Err(LabelmapError::MissingColour)
    );
    assert_eq!(out, before);
}

#[test]
fn a_refusal_writes_nothing() {
    let mut out = vec![0xA5_u8; 36];
    let before = out.clone();
    let short = LabelGrid {
        width: 3,
        height: 3,
        indices: vec![1, 1],
    };
    assert_eq!(
        paint(&short, &lut_one(), style(true, true), &mut out),
        Err(LabelmapError::Length)
    );
    assert_eq!(out, before);

    let mut narrow = vec![0xA5_u8; 35];
    let narrow_before = narrow.clone();
    assert_eq!(
        paint(&grid_all_one(), &lut_one(), style(true, true), &mut narrow),
        Err(LabelmapError::OutputLength)
    );
    assert_eq!(narrow, narrow_before);

    let mut idle = vec![0xA5_u8; 36];
    let idle_before = idle.clone();
    assert_eq!(
        paint(&grid_all_one(), &lut_one(), style(false, false), &mut idle),
        Err(LabelmapError::NothingToDraw)
    );
    assert_eq!(idle, idle_before);

    let empty = LabelGrid {
        width: 0,
        height: 3,
        indices: Vec::new(),
    };
    let mut empty_out = vec![0xA5_u8; 4];
    let empty_before = empty_out.clone();
    assert_eq!(
        paint(&empty, &lut_one(), style(true, true), &mut empty_out),
        Err(LabelmapError::Empty)
    );
    assert_eq!(empty_out, empty_before);
}

#[test]
fn a_diagonal_neighbour_does_not_make_an_outline() {
    // A plus of segment 1. The centre's four orthogonal neighbours match, so
    // it stays fill. Each corner is background. An 8-connected outline would
    // paint the centre at alpha 255 because those corners differ.
    //
    // 0 1 0
    // 1 1 1
    // 0 1 0
    let grid = LabelGrid {
        width: 3,
        height: 3,
        indices: vec![0, 1, 0, 1, 1, 1, 0, 1, 0],
    };
    let mut out = vec![0xA5_u8; 36];
    assert!(paint(&grid, &lut_one(), style(true, true), &mut out).is_ok());
    assert_eq!(pixel(&out, 4), Some([10, 20, 30, 128]));
    assert_eq!(pixel(&out, 1), Some([10, 20, 30, 255]));
    assert_eq!(pixel(&out, 3), Some([10, 20, 30, 255]));
    assert_eq!(pixel(&out, 5), Some([10, 20, 30, 255]));
    assert_eq!(pixel(&out, 7), Some([10, 20, 30, 255]));
    assert_eq!(pixel(&out, 0), Some([0, 0, 0, 0]));
    assert_eq!(pixel(&out, 2), Some([0, 0, 0, 0]));
    assert_eq!(pixel(&out, 6), Some([0, 0, 0, 0]));
    assert_eq!(pixel(&out, 8), Some([0, 0, 0, 0]));
}

#[test]
fn background_is_not_a_segment() {
    // PS3.3 C.8.20.2. The zero beside segment 1 stays transparent. It does
    // not take the neighbour colour and the LUT has no entry 0.
    let grid = LabelGrid {
        width: 2,
        height: 1,
        indices: vec![1, 0],
    };
    let mut out = vec![0xA5_u8; 8];
    assert!(paint(&grid, &lut_one(), style(true, true), &mut out).is_ok());
    assert_eq!(pixel(&out, 0), Some([10, 20, 30, 255]));
    assert_eq!(pixel(&out, 1), Some([0, 0, 0, 0]));
}
