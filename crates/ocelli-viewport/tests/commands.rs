//! F-101 command batches.
//!
//! Bytes are written here, not by the parser. A later failure must leave the
//! viewport where it started, including a VOI-sequence `SetWindow`.

use ocelli_cache::{FrameKey, SeriesId};
use ocelli_core::{Canvas, Pt};
use ocelli_pixel::{
    LutChain, LutDescriptor, ModalityTransform, PhotometricInterpretation, PresentationLutEvidence,
    VoiFunction, VoiTransform,
};
use ocelli_viewport::commands::{CommandError, STATE_BYTES, StackViewport};
use ocelli_viewport::presentation::{Colormap, Presentation};
use ocelli_viewport::scroll::StackSequence;
use ocelli_viewport::view::View;

const VIEWPORT: u16 = 7;

fn modality(sequence: Option<LutDescriptor>) -> ModalityTransform {
    let built = ModalityTransform::new(sequence, Some(1.0), Some(0.0));
    assert!(built.is_ok());
    match built {
        Ok(value) => value,
        Err(_) => unreachable!("modality slope 1 is in domain"),
    }
}

fn window(sequence: Option<LutDescriptor>, width: f32) -> VoiTransform {
    let built = VoiTransform::new(
        sequence,
        &[40.0],
        &[width],
        0,
        VoiFunction::Linear,
        0.0,
        255.0,
    );
    assert!(built.is_ok());
    match built {
        Ok(value) => value,
        Err(_) => unreachable!("window width is in domain"),
    }
}

fn chain(modality: ModalityTransform, voi: VoiTransform) -> LutChain {
    let built = LutChain::new(
        modality,
        voi,
        PhotometricInterpretation::Monochrome2,
        PresentationLutEvidence::Absent,
    );
    assert!(built.is_ok());
    match built {
        Ok(value) => value,
        Err(_) => unreachable!("monochrome chain is in domain"),
    }
}

fn lut(values: Vec<f32>) -> LutDescriptor {
    let built = LutDescriptor::new(3, 0, 8, values);
    assert!(built.is_ok());
    match built {
        Ok(value) => value,
        Err(_) => unreachable!("three-entry descriptor is in domain"),
    }
}

fn stack(chain: LutChain) -> StackViewport {
    let presentation = Presentation::new(
        chain,
        PhotometricInterpretation::Monochrome2,
        PresentationLutEvidence::Absent,
        Colormap::Grey,
    );
    let keys = [
        FrameKey::new(SeriesId(1), 0, 0),
        FrameKey::new(SeriesId(1), 1, 0),
        FrameKey::new(SeriesId(1), 2, 0),
    ];
    StackViewport::new(VIEWPORT, presentation, StackSequence::new(&keys))
}

fn ordinary() -> StackViewport {
    stack(chain(modality(None), window(None, 1.0)))
}

fn voi_sequence() -> StackViewport {
    let table = lut(vec![5.0, 15.0, 25.0]);
    stack(chain(modality(None), window(Some(table), 400.0)))
}

fn header(out: &mut Vec<u8>, kind: u16, payload_len: u32) {
    out.extend_from_slice(&kind.to_le_bytes());
    out.extend_from_slice(&VIEWPORT.to_le_bytes());
    out.extend_from_slice(&payload_len.to_le_bytes());
}

fn fit(out: &mut Vec<u8>, width: f64, height: f64, columns: u32, rows: u32) {
    header(out, 1, 24);
    out.extend_from_slice(&width.to_le_bytes());
    out.extend_from_slice(&height.to_le_bytes());
    out.extend_from_slice(&columns.to_le_bytes());
    out.extend_from_slice(&rows.to_le_bytes());
}

fn pan(out: &mut Vec<u8>, x: f64, y: f64) {
    header(out, 2, 16);
    out.extend_from_slice(&x.to_le_bytes());
    out.extend_from_slice(&y.to_le_bytes());
}

fn set_window(out: &mut Vec<u8>, centre: f64, width: f64, function: u8) {
    header(out, 7, 17);
    out.extend_from_slice(&centre.to_le_bytes());
    out.extend_from_slice(&width.to_le_bytes());
    out.push(function);
}

fn scroll(out: &mut Vec<u8>, step: i32) {
    header(out, 11, 4);
    out.extend_from_slice(&step.to_le_bytes());
}

fn read_f32(bytes: &[u8], at: usize) -> f32 {
    let mut buf = [0_u8; 4];
    let Some(slot) = bytes.get(at..at + 4) else {
        return f32::NAN;
    };
    buf.copy_from_slice(slot);
    f32::from_le_bytes(buf)
}

fn read_u32(bytes: &[u8], at: usize) -> u32 {
    let mut buf = [0_u8; 4];
    let Some(slot) = bytes.get(at..at + 4) else {
        return u32::MAX;
    };
    buf.copy_from_slice(slot);
    u32::from_le_bytes(buf)
}

#[test]
fn fit_pan_and_scroll_match_direct_calls_and_a_later_refusal_rolls_back() {
    let mut bytes = Vec::new();
    fit(&mut bytes, 8.0, 8.0, 2, 4);
    pan(&mut bytes, 3.0, -4.0);
    scroll(&mut bytes, 1);
    let mut viewport = ordinary();
    assert_eq!(viewport.apply_commands(&bytes), Ok(3));
    assert_eq!(viewport.columns(), 2);
    assert_eq!(viewport.rows(), 4);
    assert_eq!(viewport.css_width().to_bits(), 8.0_f64.to_bits());
    assert_eq!(viewport.scroll().index(), Some(1));
    let landed = viewport.view().image_to_canvas(0.5, 0.5, 2, 4);
    let mut direct = View::new();
    direct.fit(8.0, 8.0, 2, 4);
    assert!(direct.set_pan(Pt::<Canvas>::new(3.0, -4.0, 0.0)).is_ok());
    let expected = direct.image_to_canvas(0.5, 0.5, 2, 4);
    assert_eq!(landed.0.to_bits(), expected.0.to_bits());
    assert_eq!(landed.1.to_bits(), expected.1.to_bits());

    let mut refused = Vec::new();
    fit(&mut refused, 8.0, 8.0, 2, 4);
    set_window(&mut refused, 40.0, 0.0, 0);
    let before = viewport.view().image_to_canvas(0.5, 0.5, 2, 4);
    let before_index = viewport.scroll().index();
    assert_eq!(
        viewport.apply_commands(&refused),
        Err(CommandError::InvalidWindow)
    );
    let after = viewport.view().image_to_canvas(0.5, 0.5, 2, 4);
    assert_eq!(after.0.to_bits(), before.0.to_bits());
    assert_eq!(after.1.to_bits(), before.1.to_bits());
    assert_eq!(viewport.scroll().index(), before_index);
}

#[test]
fn a_voi_sequence_set_window_refuses_the_whole_batch() {
    let mut bytes = Vec::new();
    fit(&mut bytes, 8.0, 8.0, 2, 4);
    set_window(&mut bytes, 40.0, 400.0, 0);
    let mut viewport = voi_sequence();
    let original = viewport.presentation().chain().clone();
    assert_eq!(viewport.apply_commands(&bytes), Err(CommandError::Sequence));
    assert_eq!(viewport.presentation().chain(), &original);
    assert_eq!(viewport.columns(), 0);
    let canvas = viewport.view().image_to_canvas(0.5, 0.5, 2, 4);
    assert_eq!(canvas.0.to_bits(), 0.5_f64.to_bits());
    assert_eq!(canvas.1.to_bits(), 0.5_f64.to_bits());
}

#[test]
fn truncation_unknown_kind_and_a_bad_enum_write_nothing() {
    let mut viewport = ordinary();
    let truncated = [1_u8, 0, 7, 0, 24, 0, 0, 0];
    assert_eq!(
        viewport.apply_commands(&truncated),
        Err(CommandError::Truncated)
    );
    let mut unknown = Vec::new();
    header(&mut unknown, 0, 0);
    assert_eq!(
        viewport.apply_commands(&unknown),
        Err(CommandError::UnknownKind)
    );
    let mut bad = Vec::new();
    header(&mut bad, 9, 1);
    bad.push(3);
    assert_eq!(viewport.apply_commands(&bad), Err(CommandError::BadEnum));
    assert_eq!(viewport.columns(), 0);
}

#[test]
fn state_offsets_follow_section_17_4_and_bit_3_is_refining() {
    let mut bytes = Vec::new();
    fit(&mut bytes, 8.0, 8.0, 2, 4);
    set_window(&mut bytes, 40.0, 400.0, 0);
    let mut viewport = ordinary();
    assert_eq!(viewport.apply_commands(&bytes), Ok(2));
    viewport.set_refining(true);
    viewport.set_generation(9);
    let mut out = [0_u8; STATE_BYTES];
    assert!(viewport.write_state(&mut out).is_ok());
    let camera = viewport.view().camera_matrix(2, 4);
    for (index, value) in camera.iter().enumerate() {
        assert_eq!(read_f32(&out, index * 4).to_bits(), value.to_bits());
    }
    assert_eq!(read_f32(&out, 64).to_bits(), 40.0_f32.to_bits());
    assert_eq!(read_f32(&out, 68).to_bits(), 400.0_f32.to_bits());
    assert_eq!(read_u32(&out, 72), 0);
    assert_eq!(read_u32(&out, 76), 3);
    assert_eq!(read_u32(&out, 80), 8);
    assert_eq!(read_u32(&out, 84), 0);
    assert_eq!(read_u32(&out, 88), 0);
    assert_eq!(read_u32(&out, 92), 0);
    assert_eq!(viewport.generation(), 9);
}

#[test]
fn every_command_id_matches_the_direct_call() {
    let mut bytes = Vec::new();
    fit(&mut bytes, 8.0, 8.0, 4, 4);
    header(&mut bytes, 3, 8);
    bytes.extend_from_slice(&2.0_f64.to_le_bytes());
    header(&mut bytes, 4, 8);
    bytes.extend_from_slice(&std::f64::consts::FRAC_PI_2.to_le_bytes());
    header(&mut bytes, 5, 1);
    bytes.push(1);
    header(&mut bytes, 6, 1);
    bytes.push(1);
    set_window(&mut bytes, 40.0, 400.0, 1);
    header(&mut bytes, 8, 1);
    bytes.push(1);
    header(&mut bytes, 9, 1);
    bytes.push(1);
    header(&mut bytes, 10, 1);
    bytes.push(1);
    scroll(&mut bytes, -1);
    let mut viewport = ordinary();
    assert_eq!(viewport.apply_commands(&bytes), Ok(10));

    let mut direct = View::new();
    direct.fit(8.0, 8.0, 4, 4);
    direct.set_zoom(ocelli_viewport::view::Zoom::new(2.0));
    direct.set_rotation(ocelli_viewport::view::Radians::new(
        std::f64::consts::FRAC_PI_2,
    ));
    direct.set_flip_horizontal(true);
    direct.set_flip_vertical(true);
    let landed = viewport.view().image_to_canvas(0.5, 0.5, 4, 4);
    let expected = direct.image_to_canvas(0.5, 0.5, 4, 4);
    assert_eq!(landed.0.to_bits(), expected.0.to_bits());
    assert_eq!(landed.1.to_bits(), expected.1.to_bits());
    assert_eq!(
        viewport.presentation().chain().voi().window(),
        Some((-600.0, 1500.0, VoiFunction::Linear))
    );
    assert!(viewport.presentation().explicit_inverse());
    assert_eq!(viewport.presentation().colormap(), Colormap::Stops);
    assert_eq!(viewport.scroll().index(), Some(0));
    assert_eq!(viewport.flags(), 1);

    let mut exact = ordinary();
    let mut one = Vec::new();
    set_window(&mut one, 10.0, 20.0, 1);
    assert_eq!(exact.apply_commands(&one), Ok(1));
    assert_eq!(
        exact.presentation().chain().voi().window(),
        Some((10.0, 20.0, VoiFunction::LinearExact))
    );
    let mut bone = Vec::new();
    header(&mut bone, 8, 1);
    bone.push(2);
    assert_eq!(exact.apply_commands(&bone), Ok(1));
    assert_eq!(
        exact.presentation().chain().voi().window(),
        Some((300.0, 1500.0, VoiFunction::Linear))
    );
}

#[test]
fn a_modality_sequence_accepts_a_window_and_refuses_a_preset() {
    let table = lut(vec![1.0, 2.0, 3.0]);
    let built = ModalityTransform::new(Some(table), Some(9.0), Some(99.0));
    assert!(built.is_ok());
    let modality = match built {
        Ok(value) => value,
        Err(_) => unreachable!("modality sequence is in domain"),
    };
    let mut viewport = stack(chain(modality, window(None, 1.0)));
    let mut window_bytes = Vec::new();
    set_window(&mut window_bytes, 40.0, 400.0, 2);
    assert_eq!(viewport.apply_commands(&window_bytes), Ok(1));
    assert_eq!(
        viewport.presentation().chain().voi().window(),
        Some((40.0, 400.0, VoiFunction::Sigmoid))
    );
    let before = viewport.presentation().chain().clone();
    let mut preset = Vec::new();
    header(&mut preset, 8, 1);
    preset.push(0);
    assert_eq!(
        viewport.apply_commands(&preset),
        Err(CommandError::Sequence)
    );
    assert_eq!(viewport.presentation().chain(), &before);
    assert_eq!(viewport.columns(), 0);
}

#[test]
fn structural_refusals_leave_the_viewport_where_it_started() {
    let mut viewport = ordinary();
    assert_eq!(viewport.apply_commands(&[]), Ok(0));

    let mut foreign = Vec::new();
    foreign.extend_from_slice(&1_u16.to_le_bytes());
    foreign.extend_from_slice(&9_u16.to_le_bytes());
    foreign.extend_from_slice(&0_u32.to_le_bytes());
    assert_eq!(
        viewport.apply_commands(&foreign),
        Err(CommandError::UnknownViewport)
    );

    let mut short = Vec::new();
    header(&mut short, 3, 4);
    short.extend_from_slice(&1.0_f32.to_le_bytes());
    assert_eq!(
        viewport.apply_commands(&short),
        Err(CommandError::BadLength)
    );

    let mut non_finite = Vec::new();
    pan(&mut non_finite, f64::NAN, 1.0);
    assert_eq!(
        viewport.apply_commands(&non_finite),
        Err(CommandError::NonFinite)
    );

    let huge = vec![0_u8; 4097];
    assert_eq!(viewport.apply_commands(&huge), Err(CommandError::Limit));

    let mut many = Vec::new();
    for _ in 0..65 {
        scroll(&mut many, 1);
    }
    assert_eq!(viewport.apply_commands(&many), Err(CommandError::Limit));
    assert_eq!(viewport.columns(), 0);
    assert_eq!(viewport.scroll().index(), Some(0));
}
