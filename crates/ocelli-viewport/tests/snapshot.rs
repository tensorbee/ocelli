//! F-050 camera, display-area and presentation snapshots.
//!
//! The hand matrix is the inverse of `image_to_canvas` for one fitted view.
//! It is not copied from a previous run of this file.
//!
//! DICOM PS3.3 C.11.1 rescale slope 1 and intercept -1024.
//! DICOM PS3.3 C.11.2.1.2 window centre 40 and width 400, LINEAR.
//! DICOM PS3.3 C.11.2.1.1 entry count 0 means 65,536.
//! DICOM PS3.3 C.11.6 shape IDENTITY or INVERSE. Absent is neither.
//! DICOM PS3.3 C.7.6.2.1.1 index `(i, j)` is the pixel centre.

use ocelli_cache::{FrameKey, SeriesId};
use ocelli_core::{Canvas, Pt};
use ocelli_pixel::{
    ImageDimensions, ImageOrientationPatient, ImagePlane, ImagePositionPatient, LutChain,
    LutDescriptor, ModalityTransform, PhotometricInterpretation, PixelError,
    PresentationLutEvidence, PresentationLutShape, VoiFunction, VoiTransform,
    lut_descriptor_clones,
};
use ocelli_viewport::boundary::{
    FLAG_ERROR, FLAG_LOADING, FLAG_REFINING, KIND_FIT, KIND_FLIP_HORIZONTAL, KIND_PAN, KIND_ROTATE,
    KIND_SCROLL, KIND_SET_WINDOW, KIND_ZOOM,
};
use ocelli_viewport::commands::{STATE_BYTES, StackViewport};
use ocelli_viewport::presentation::{Colormap, Presentation};
use ocelli_viewport::scroll::StackSequence;
use ocelli_viewport::snapshot::SnapshotError;
use ocelli_viewport::view::{Radians, View, Zoom};

const COLUMNS: u32 = 512;
const ROWS: u32 = 512;

/// Inverse of image-to-canvas for the hand record below.
///
/// Fitted, fit scale 1, anchor `(400, 300)`, zoom 2, pan `(12.5, -4)`,
/// no rotation, no flip, spacing 1, shift 0, grid 512 by 512.
/// Scale is 2. Translation is `(412.5, 296)`. Centre is `(256, 256)`.
/// `a = e = 0.5`, `c = 256 - 0.5 * 412.5 = 49.75`, `f = 256 - 0.5 * 296 = 108`.
/// `b = -q / det` and `d = -r / det`. At zero radians `q` and `r` are `+0`,
/// so those two elements are `-0`.
const HAND_MATRIX: [f32; 16] = [
    0.5, -0.0, 0.0, 0.0, -0.0, 0.5, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 49.75, 108.0, 0.0, 1.0,
];

macro_rules! require_ok {
    ($value:expr) => {{
        let result = $value;
        assert!(
            result.is_ok(),
            "expected Ok, got {:?}",
            result.as_ref().err()
        );
        let Ok(value) = result else { return };
        value
    }};
}

fn identity(id: u16) -> Result<StackViewport, ocelli_viewport::commands::CommandError> {
    StackViewport::try_identity(id)
}

fn hand_record() -> serde_json::Value {
    serde_json::json!({
        "magic": "OCS1",
        "version": 1,
        "viewportId": 1,
        "displayArea": {
            "cssWidth": 800.0,
            "cssHeight": 600.0,
            "columns": COLUMNS,
            "rows": ROWS,
            "columnMillimetres": 1.0,
            "rowMillimetres": 1.0,
            "shiftColumn": 0.0,
            "shiftRow": 0.0,
            "fitted": true,
            "anchorX": 400.0,
            "anchorY": 300.0,
            "fitScale": 1.0
        },
        "camera": {
            "panX": 12.5,
            "panY": -4.0,
            "zoom": 2.0,
            "radians": 0.0,
            "flipHorizontal": false,
            "flipVertical": false
        },
        "presentation": {
            "modality": { "kind": "rescale", "slope": 1.0, "intercept": -1024.0 },
            "voi": {
                "kind": "window",
                "center": 40.0,
                "width": 400.0,
                "function": "linear",
                "ymin": 0.0,
                "ymax": 255.0
            },
            "photometric": "monochrome2",
            "evidence": { "kind": "shape", "shape": "inverse" },
            "colormap": "grey"
        }
    })
}

fn to_bytes(value: &serde_json::Value) -> Vec<u8> {
    let built = serde_json::to_vec(value);
    assert!(built.is_ok());
    let Ok(bytes) = built else { return Vec::new() };
    bytes
}

fn text_of(bytes: &[u8]) -> String {
    let built = String::from_utf8(bytes.to_vec());
    assert!(built.is_ok());
    built.unwrap_or_default()
}

fn must_replace(text: &str, needle: &str, replacement: &str) -> String {
    assert!(text.contains(needle), "missing {needle}");
    text.replacen(needle, replacement, 1)
}

fn assert_matrix(got: [f32; 16], expect: [f32; 16]) {
    for (index, (left, right)) in got.iter().zip(expect.iter()).enumerate() {
        assert_eq!(left.to_bits(), right.to_bits(), "camera component {index}");
    }
}

fn index_bits(view: &View, columns: u32, rows: u32, x: f64, y: f64) -> (u64, u64) {
    let built = view.canvas_to_index(columns, rows);
    assert!(built.is_ok());
    let Ok(transform) = built else { return (0, 0) };
    let point = transform.apply(Pt::<Canvas>::new(x, y, 0.0));
    (point.x.to_bits(), point.y.to_bits())
}

fn push_header(out: &mut Vec<u8>, id: u16, kind: u16, len: u32) {
    out.extend_from_slice(&kind.to_le_bytes());
    out.extend_from_slice(&id.to_le_bytes());
    out.extend_from_slice(&len.to_le_bytes());
}

fn push_f64(out: &mut Vec<u8>, value: f64) {
    out.extend_from_slice(&value.to_le_bytes());
}

fn push_u32(out: &mut Vec<u8>, value: u32) {
    out.extend_from_slice(&value.to_le_bytes());
}

fn apply(viewport: &mut StackViewport, bytes: &[u8], count: u32) {
    assert_eq!(viewport.apply_commands(bytes), Ok(count));
}

fn f32_at(bytes: &[u8], at: usize) -> f32 {
    let mut buf = [0_u8; 4];
    let Some(end) = at.checked_add(4) else {
        return f32::from_bits(0x7fc0_0000);
    };
    let Some(slot) = bytes.get(at..end) else {
        return f32::from_bits(0x7fc0_0000);
    };
    buf.copy_from_slice(slot);
    f32::from_le_bytes(buf)
}

fn rescale(slope: f32, intercept: f32) -> Result<ModalityTransform, PixelError> {
    ModalityTransform::new(None, Some(slope), Some(intercept))
}

fn window(center: f32, width: f32) -> Result<VoiTransform, PixelError> {
    VoiTransform::new(
        None,
        &[center],
        &[width],
        0,
        VoiFunction::Linear,
        0.0,
        255.0,
    )
}

fn voi_lut(descriptor: LutDescriptor) -> Result<VoiTransform, PixelError> {
    VoiTransform::new(Some(descriptor), &[], &[], 0, VoiFunction::Linear, 0.0, 1.0)
}

fn modality_lut(descriptor: LutDescriptor) -> Result<ModalityTransform, PixelError> {
    ModalityTransform::new(Some(descriptor), None, None)
}

fn chain(
    modality: ModalityTransform,
    voi: VoiTransform,
    photometric: PhotometricInterpretation,
    evidence: PresentationLutEvidence,
) -> Result<LutChain, PixelError> {
    LutChain::new(modality, voi, photometric, evidence)
}

fn hold(
    id: u16,
    modality: ModalityTransform,
    voi: VoiTransform,
    photometric: PhotometricInterpretation,
    evidence: PresentationLutEvidence,
) -> Result<StackViewport, PixelError> {
    let built = chain(modality, voi, photometric, evidence)?;
    Ok(StackViewport::new(
        id,
        Presentation::new(built, photometric, evidence, Colormap::Grey),
        StackSequence::new(&[]),
    ))
}

fn lut(count: u16, first: i32, bits: u8, values: Vec<f32>) -> Result<LutDescriptor, PixelError> {
    LutDescriptor::new(count, first, bits, values)
}

fn assert_unchanged(viewport: &StackViewport, before: &[u8]) {
    let after = require_snapshot(viewport);
    assert_eq!(after, before);
}

fn require_snapshot(viewport: &StackViewport) -> Vec<u8> {
    let built = viewport.snapshot();
    assert!(built.is_ok());
    built.unwrap_or_default()
}

fn refuse(viewport: &mut StackViewport, bytes: &[u8], error: SnapshotError) {
    let before = require_snapshot(viewport);
    assert_eq!(viewport.restore(bytes), Err(error));
    assert_unchanged(viewport, &before);
}

#[test]
fn hand_matrix_is_the_restored_camera() {
    let bytes = to_bytes(&hand_record());
    let mut viewport = require_ok!(identity(4));
    assert!(viewport.restore(&bytes).is_ok());
    assert_matrix(viewport.view().camera_matrix(COLUMNS, ROWS), HAND_MATRIX);
    assert_eq!(viewport.id(), 4);
    let window = viewport.presentation().chain().voi().window();
    assert_eq!(window, Some((40.0, 400.0, VoiFunction::Linear)));
    assert_eq!(
        viewport.presentation().chain().modality().rescale(),
        Some((1.0, -1024.0))
    );
    assert!(viewport.presentation().explicit_inverse());
}

#[test]
fn round_trip_window() {
    let mut viewport = require_ok!(identity(1));
    let mut bytes = Vec::new();
    push_header(&mut bytes, 1, KIND_FIT, 24);
    push_f64(&mut bytes, 800.0);
    push_f64(&mut bytes, 600.0);
    push_u32(&mut bytes, COLUMNS);
    push_u32(&mut bytes, ROWS);
    push_header(&mut bytes, 1, KIND_PAN, 16);
    push_f64(&mut bytes, 12.5);
    push_f64(&mut bytes, -4.0);
    push_header(&mut bytes, 1, KIND_ZOOM, 8);
    push_f64(&mut bytes, 2.0);
    push_header(&mut bytes, 1, KIND_ROTATE, 8);
    push_f64(&mut bytes, std::f64::consts::FRAC_PI_2);
    push_header(&mut bytes, 1, KIND_FLIP_HORIZONTAL, 1);
    bytes.push(1);
    push_header(&mut bytes, 1, KIND_SET_WINDOW, 17);
    push_f64(&mut bytes, 40.0);
    push_f64(&mut bytes, 400.0);
    bytes.push(0);
    apply(&mut viewport, &bytes, 6);

    let record = require_snapshot(&viewport);
    let text = text_of(&record);
    assert!(text.starts_with("{\"magic\":\"OCS1\",\"version\":1"));
    assert!(!text.contains("matrix"));
    let matrix = viewport.view().camera_matrix(COLUMNS, ROWS);
    let index = index_bits(viewport.view(), COLUMNS, ROWS, 40.0, 25.0);

    let mut moved = Vec::new();
    push_header(&mut moved, 1, KIND_PAN, 16);
    push_f64(&mut moved, 80.0);
    push_f64(&mut moved, 80.0);
    apply(&mut viewport, &moved, 1);
    let moved_matrix = viewport.view().camera_matrix(COLUMNS, ROWS);
    let moved_translation = moved_matrix.get(12).copied().unwrap_or(f32::NAN);
    let kept_translation = matrix.get(12).copied().unwrap_or(f32::NAN);
    assert_ne!(moved_translation.to_bits(), kept_translation.to_bits());

    assert!(viewport.restore(&record).is_ok());
    assert_matrix(viewport.view().camera_matrix(COLUMNS, ROWS), matrix);
    assert_eq!(
        index_bits(viewport.view(), COLUMNS, ROWS, 40.0, 25.0),
        index
    );
    assert_eq!(
        viewport.presentation().chain().voi().window(),
        Some((40.0, 400.0, VoiFunction::Linear))
    );
    assert!(viewport.restore(&record).is_ok());
    assert_eq!(require_snapshot(&viewport), record);
}

#[test]
fn round_trip_voi_sequence() {
    let descriptor = require_ok!(lut(3, 0, 8, vec![0.0, 128.0, 255.0]));
    let mut viewport = require_ok!(hold(
        1,
        require_ok!(rescale(1.0, 0.0)),
        require_ok!(voi_lut(descriptor)),
        PhotometricInterpretation::Monochrome2,
        PresentationLutEvidence::Absent,
    ));
    let record = require_snapshot(&viewport);
    let text = text_of(&record);
    assert!(text.contains("\"kind\":\"lut\""));
    assert!(!text.contains("\"kind\":\"window\""));
    assert!(viewport.presentation().chain().voi().window().is_none());

    let mut windowed = require_ok!(identity(1));
    assert!(windowed.restore(&record).is_ok());
    assert!(windowed.presentation().chain().voi().window().is_none());
    let restored = windowed.presentation().chain().voi().lut();
    assert!(restored.is_some());
    let Some(restored) = restored else { return };
    assert_eq!(restored.declared_entry_count(), 3);
    assert_eq!(restored.values(), &[0.0, 128.0, 255.0]);

    let mut state = vec![0_u8; STATE_BYTES];
    assert!(windowed.write_state(&mut state).is_ok());
    assert_eq!(STATE_BYTES, 96);
    assert_eq!(f32_at(&state, 64).to_bits(), 0.0_f32.to_bits());
    assert_eq!(f32_at(&state, 68).to_bits(), 0.0_f32.to_bits());

    let window_record = require_snapshot(&require_ok!(identity(1)));
    assert!(viewport.restore(&window_record).is_ok());
    assert!(viewport.presentation().chain().voi().window().is_some());
}

#[test]
fn round_trip_modality_sequence() {
    let descriptor = require_ok!(lut(3, -1024, 16, vec![0.0, 1.0, 2.0]));
    let viewport = require_ok!(hold(
        1,
        require_ok!(modality_lut(descriptor)),
        require_ok!(window(40.0, 400.0)),
        PhotometricInterpretation::Monochrome2,
        PresentationLutEvidence::Absent,
    ));
    assert!(
        viewport
            .presentation()
            .chain()
            .modality()
            .rescale()
            .is_none()
    );
    let record = require_snapshot(&viewport);
    let text = text_of(&record);
    assert!(!text.contains("\"slope\""));
    let mut restored = require_ok!(identity(1));
    assert!(restored.restore(&record).is_ok());
    assert!(
        restored
            .presentation()
            .chain()
            .modality()
            .rescale()
            .is_none()
    );
    let lut_ref = restored.presentation().chain().modality().lut();
    assert!(lut_ref.is_some());
    let Some(lut_ref) = lut_ref else { return };
    assert_eq!(lut_ref.values(), &[0.0, 1.0, 2.0]);
    assert_eq!(lut_ref.declared_entry_count(), 3);
    assert_eq!(lut_ref.first_mapped_input(), -1024);
}

#[test]
fn round_trip_explicit_inverse() {
    let viewport = require_ok!(hold(
        1,
        require_ok!(rescale(1.0, 0.0)),
        require_ok!(window(40.0, 400.0)),
        PhotometricInterpretation::Monochrome2,
        PresentationLutEvidence::Shape(PresentationLutShape::Inverse),
    ));
    assert!(viewport.presentation().explicit_inverse());
    assert!(viewport.presentation().chain().inverts());
    let record = require_snapshot(&viewport);
    let mut restored = require_ok!(identity(1));
    assert!(restored.restore(&record).is_ok());
    assert!(restored.presentation().explicit_inverse());
    assert_eq!(
        restored.presentation().evidence(),
        PresentationLutEvidence::Shape(PresentationLutShape::Inverse)
    );
}

#[test]
fn round_trip_absent_monochrome1() {
    let viewport = require_ok!(hold(
        1,
        require_ok!(rescale(1.0, 0.0)),
        require_ok!(window(40.0, 400.0)),
        PhotometricInterpretation::Monochrome1,
        PresentationLutEvidence::Absent,
    ));
    assert!(!viewport.presentation().explicit_inverse());
    assert!(viewport.presentation().chain().inverts());
    let record = require_snapshot(&viewport);
    let mut restored = require_ok!(identity(1));
    assert!(restored.restore(&record).is_ok());
    assert!(!restored.presentation().explicit_inverse());
    assert!(restored.presentation().chain().inverts());
    assert_eq!(
        restored.presentation().evidence(),
        PresentationLutEvidence::Absent
    );
    assert_eq!(
        restored.presentation().photometric(),
        PhotometricInterpretation::Monochrome1
    );
}

#[test]
fn refuses_unknown_version() {
    let mut record = hand_record();
    let Some(root) = record.as_object_mut() else {
        return;
    };
    let _ = root.insert("version".to_string(), serde_json::json!(2));
    let mut viewport = require_ok!(identity(1));
    refuse(
        &mut viewport,
        &to_bytes(&record),
        SnapshotError::UnknownVersion(2),
    );
}

#[test]
fn refuses_unknown_field() {
    let mut record = hand_record();
    let Some(root) = record.as_object_mut() else {
        return;
    };
    let _ = root.insert("matrix".to_string(), serde_json::json!(1));
    let mut viewport = require_ok!(identity(1));
    refuse(&mut viewport, &to_bytes(&record), SnapshotError::Json);
}

#[test]
fn refuses_bad_magic() {
    let mut record = hand_record();
    let Some(root) = record.as_object_mut() else {
        return;
    };
    let _ = root.insert("magic".to_string(), serde_json::json!("NOPE"));
    let mut viewport = require_ok!(identity(1));
    refuse(&mut viewport, &to_bytes(&record), SnapshotError::BadMagic);
}

#[test]
fn refuses_malformed_json() {
    let mut viewport = require_ok!(identity(1));
    refuse(&mut viewport, b"{", SnapshotError::Json);
}

#[test]
fn refuses_trailing_byte() {
    let mut bytes = to_bytes(&hand_record());
    bytes.push(b'x');
    let mut viewport = require_ok!(identity(1));
    refuse(&mut viewport, &bytes, SnapshotError::Json);
}

#[test]
fn refuses_non_finite_literal() {
    let text = text_of(&to_bytes(&hand_record()));
    let text = if text.contains("\"zoom\":2.0") {
        must_replace(&text, "\"zoom\":2.0", "\"zoom\":1e9999")
    } else {
        must_replace(&text, "\"zoom\":2", "\"zoom\":1e9999")
    };
    let mut viewport = require_ok!(identity(1));
    refuse(&mut viewport, text.as_bytes(), SnapshotError::NonFinite);
}

#[test]
fn refuses_unfitted_non_finite_anchor() {
    let text = text_of(&to_bytes(&hand_record()));
    let text = must_replace(&text, "\"fitted\":true", "\"fitted\":false");
    let text = if text.contains("\"anchorX\":400.0") {
        must_replace(&text, "\"anchorX\":400.0", "\"anchorX\":1e9999")
    } else {
        must_replace(&text, "\"anchorX\":400", "\"anchorX\":1e9999")
    };
    let mut viewport = require_ok!(identity(1));
    refuse(&mut viewport, text.as_bytes(), SnapshotError::NonFinite);
}

#[test]
fn refuses_bad_enum() {
    let mut record = hand_record();
    let Some(root) = record.as_object_mut() else {
        return;
    };
    let Some(presentation) = root.get_mut("presentation") else {
        return;
    };
    let Some(presentation) = presentation.as_object_mut() else {
        return;
    };
    let _ = presentation.insert("photometric".to_string(), serde_json::json!("cmyk"));
    let mut viewport = require_ok!(identity(1));
    refuse(&mut viewport, &to_bytes(&record), SnapshotError::BadEnum);
}

#[test]
fn refuses_bad_lut() {
    let mut record = hand_record();
    let Some(root) = record.as_object_mut() else {
        return;
    };
    let Some(presentation) = root.get_mut("presentation") else {
        return;
    };
    let Some(presentation) = presentation.as_object_mut() else {
        return;
    };
    let _ = presentation.insert(
        "voi".to_string(),
        serde_json::json!({
            "kind": "lut",
            "entryCount": 3,
            "firstMappedInput": 0,
            "bitsPerEntry": 8,
            "values": [1.0]
        }),
    );
    let mut viewport = require_ok!(identity(1));
    refuse(&mut viewport, &to_bytes(&record), SnapshotError::BadLut);
}

#[test]
fn refuses_chain() {
    let mut record = hand_record();
    let Some(root) = record.as_object_mut() else {
        return;
    };
    let Some(presentation) = root.get_mut("presentation") else {
        return;
    };
    let Some(presentation) = presentation.as_object_mut() else {
        return;
    };
    let _ = presentation.insert(
        "evidence".to_string(),
        serde_json::json!({ "kind": "sequence" }),
    );
    let mut viewport = require_ok!(identity(1));
    refuse(
        &mut viewport,
        &to_bytes(&record),
        SnapshotError::ChainUnavailable,
    );
}

#[test]
fn canvas_world_and_index_are_distinct() {
    // C.7.6.2.1.1. Column spacing 0.5, row spacing 2. Index (2, 3) is
    // patient (11, 26, 30) from IPP (10, 20, 30). The unfitted view uses
    // the same millimetres, so canvas (3.5, 3.0) is that index.
    let plane = require_ok!(ImagePlane::new(
        require_ok!(ImagePositionPatient::new([10.0, 20.0, 30.0])),
        require_ok!(ImageOrientationPatient::new(
            [1.0, 0.0, 0.0],
            [0.0, 1.0, 0.0]
        )),
        require_ok!(ocelli_pixel::PixelSpacing::new(2.0, 0.5)),
        require_ok!(ImageDimensions::new(8, 9)),
    ));
    let record = display_record(DisplayInput {
        zoom: 1.0,
        pan_x: 0.0,
        column_mm: 0.5,
        row_mm: 2.0,
        shift_column: 0.0,
        columns: 9,
        rows: 8,
    });
    let mut viewport = require_ok!(identity(1));
    assert!(viewport.restore(&to_bytes(&record)).is_ok());
    let world = require_ok!(viewport.view().canvas_to_world(&plane));
    let point = world.apply(Pt::<Canvas>::new(3.5, 3.0, 0.0));
    assert!((point.x - 11.0).abs() <= 1e-6);
    assert!((point.y - 26.0).abs() <= 1e-6);
    assert!((point.z - 30.0).abs() <= 1e-6);

    let mut reference = View::new();
    reference.set_spacing(0.5, 2.0);
    let reference_world = require_ok!(reference.canvas_to_world(&plane));
    let reference_point = reference_world.apply(Pt::<Canvas>::new(3.5, 3.0, 0.0));
    assert_eq!(point.x.to_bits(), reference_point.x.to_bits());
    assert_eq!(point.y.to_bits(), reference_point.y.to_bits());
    assert_eq!(point.z.to_bits(), reference_point.z.to_bits());

    let again = require_snapshot(&viewport);
    assert!(viewport.restore(&again).is_ok());
    let world_again = require_ok!(viewport.view().canvas_to_world(&plane));
    let point_again = world_again.apply(Pt::<Canvas>::new(3.5, 3.0, 0.0));
    assert_eq!(point.x.to_bits(), point_again.x.to_bits());
    assert_eq!(point.y.to_bits(), point_again.y.to_bits());
    assert_eq!(point.z.to_bits(), point_again.z.to_bits());

    // Zoom 2 makes one canvas pixel half an index step. A pan of 3 canvas
    // pixels is 1.5 mm on this plane, so it is not a patient translation of 3.
    let panned = display_record(DisplayInput {
        zoom: 2.0,
        pan_x: 3.0,
        column_mm: 0.5,
        row_mm: 2.0,
        shift_column: 0.0,
        columns: 9,
        rows: 8,
    });
    assert!(viewport.restore(&to_bytes(&panned)).is_ok());
    let panned_world = require_ok!(viewport.view().canvas_to_world(&plane));
    let panned_point = panned_world.apply(Pt::<Canvas>::new(3.5, 3.0, 0.0));
    assert_ne!(panned_point.x.to_bits(), (point.x + 3.0).to_bits());

    let shifted = display_record(DisplayInput {
        zoom: 1.0,
        pan_x: 0.0,
        column_mm: 0.5,
        row_mm: 2.0,
        shift_column: 4.0,
        columns: 9,
        rows: 8,
    });
    assert!(viewport.restore(&to_bytes(&shifted)).is_ok());
    let shifted_world = require_ok!(viewport.view().canvas_to_world(&plane));
    let shifted_point = shifted_world.apply(Pt::<Canvas>::new(3.5, 3.0, 0.0));
    assert_ne!(shifted_point.x.to_bits(), point.x.to_bits());

    let respaced = display_record(DisplayInput {
        zoom: 1.0,
        pan_x: 0.0,
        column_mm: 4.5,
        row_mm: 2.0,
        shift_column: 0.0,
        columns: 9,
        rows: 8,
    });
    assert!(viewport.restore(&to_bytes(&respaced)).is_ok());
    let respaced_world = require_ok!(viewport.view().canvas_to_world(&plane));
    let respaced_point = respaced_world.apply(Pt::<Canvas>::new(3.5, 3.0, 0.0));
    assert_ne!(respaced_point.x.to_bits(), shifted_point.x.to_bits());
}

struct DisplayInput {
    zoom: f64,
    pan_x: f64,
    column_mm: f64,
    row_mm: f64,
    shift_column: f64,
    columns: u32,
    rows: u32,
}

fn display_record(input: DisplayInput) -> serde_json::Value {
    serde_json::json!({
        "magic": "OCS1",
        "version": 1,
        "viewportId": 1,
        "displayArea": {
            "cssWidth": 800.0,
            "cssHeight": 600.0,
            "columns": input.columns,
            "rows": input.rows,
            "columnMillimetres": input.column_mm,
            "rowMillimetres": input.row_mm,
            "shiftColumn": input.shift_column,
            "shiftRow": 0.0,
            "fitted": false,
            "anchorX": 400.0,
            "anchorY": 300.0,
            "fitScale": 4.0
        },
        "camera": {
            "panX": input.pan_x,
            "panY": 0.0,
            "zoom": input.zoom,
            "radians": 0.0,
            "flipHorizontal": false,
            "flipVertical": false
        },
        "presentation": {
            "modality": { "kind": "rescale", "slope": 1.0, "intercept": 0.0 },
            "voi": {
                "kind": "window",
                "center": 40.0,
                "width": 400.0,
                "function": "linear",
                "ymin": 0.0,
                "ymax": 255.0
            },
            "photometric": "monochrome2",
            "evidence": { "kind": "absent" },
            "colormap": "grey"
        }
    })
}

#[test]
fn runtime_scroll_and_flags_survive() {
    let mut viewport = require_ok!(identity(1));
    let keys = [
        FrameKey::new(SeriesId(1), 0, 0),
        FrameKey::new(SeriesId(1), 1, 0),
        FrameKey::new(SeriesId(1), 2, 0),
    ];
    viewport.replace_stack(StackSequence::new(&keys));
    let mut scroll = Vec::new();
    push_header(&mut scroll, 1, KIND_SCROLL, 4);
    scroll.extend_from_slice(&1_i32.to_le_bytes());
    apply(&mut viewport, &scroll, 1);
    viewport.set_refining(true);
    viewport.set_generation(9);
    let record = require_snapshot(&viewport);

    let mut pan = Vec::new();
    push_header(&mut pan, 1, KIND_PAN, 16);
    push_f64(&mut pan, 5.0);
    push_f64(&mut pan, 6.0);
    apply(&mut viewport, &pan, 1);
    assert!(viewport.restore(&record).is_ok());
    assert_eq!(viewport.scroll().index(), Some(1));
    assert_eq!(viewport.scroll().len(), 3);
    assert_eq!(viewport.generation(), 9);
    assert_eq!(viewport.flags() & FLAG_ERROR, 0);
    assert_eq!(viewport.flags(), FLAG_LOADING | FLAG_REFINING);
}

#[test]
fn declared_entry_count_zero_means_65536() {
    // PS3.3 C.11.2.1.1. The descriptor value 0 is 65,536 entries. The last
    // stored value is 255 and the rest are 0. Both survive the round trip.
    let mut values = vec![0.0_f32; 65_536];
    let Some(last) = values.last_mut() else {
        return;
    };
    *last = 255.0;
    let descriptor = require_ok!(lut(0, 0, 8, values));
    assert_eq!(descriptor.declared_entry_count(), 0);
    assert_eq!(descriptor.values().len(), 65_536);
    let viewport = require_ok!(hold(
        1,
        require_ok!(rescale(1.0, -1024.0)),
        require_ok!(voi_lut(descriptor)),
        PhotometricInterpretation::Monochrome2,
        PresentationLutEvidence::Absent,
    ));
    let record = require_snapshot(&viewport);
    let mut restored = require_ok!(identity(1));
    assert!(restored.restore(&record).is_ok());
    let lut_ref = restored.presentation().chain().voi().lut();
    assert!(lut_ref.is_some());
    let Some(lut_ref) = lut_ref else { return };
    assert_eq!(lut_ref.declared_entry_count(), 0);
    assert_eq!(lut_ref.values().len(), 65_536);
    let Some(last) = lut_ref.values().last() else {
        return;
    };
    assert_eq!(last.to_bits(), 255.0_f32.to_bits());
    assert!(restored.presentation().chain().voi().window().is_none());
}

#[test]
fn snapshot_does_not_clone_a_lut_descriptor() {
    let before = lut_descriptor_clones();
    let descriptor = require_ok!(lut(3, 0, 8, vec![1.0, 2.0, 3.0]));
    let viewport = require_ok!(hold(
        1,
        require_ok!(rescale(1.0, 0.0)),
        require_ok!(voi_lut(descriptor)),
        PhotometricInterpretation::Monochrome2,
        PresentationLutEvidence::Absent,
    ));
    assert_eq!(lut_descriptor_clones(), before);
    let record = require_snapshot(&viewport);
    assert_eq!(lut_descriptor_clones(), before);
    let mut restored = require_ok!(identity(1));
    assert!(restored.restore(&record).is_ok());
    assert_eq!(lut_descriptor_clones(), before);
}

#[test]
fn property_family_round_trips() {
    let flips = [false, true];
    let angles = [0.0_f64, std::f64::consts::FRAC_PI_2, 0.3];
    let zooms = [1.0_f64, 2.0];
    let spacings = [(1.0_f64, 1.0), (0.0, 1.0), (1.0, 0.0)];
    let mut cases = 0_u32;
    for fitted in [false, true] {
        for flip_horizontal in flips {
            for flip_vertical in flips {
                for radians in angles {
                    for zoom in zooms {
                        for (column_mm, row_mm) in spacings {
                            cases = cases.saturating_add(1);
                            round_trip_case(
                                fitted,
                                flip_horizontal,
                                flip_vertical,
                                radians,
                                zoom,
                                column_mm,
                                row_mm,
                            );
                        }
                    }
                }
            }
        }
    }
    assert_eq!(cases, 144);
}

fn round_trip_case(
    fitted: bool,
    flip_horizontal: bool,
    flip_vertical: bool,
    radians: f64,
    zoom: f64,
    column_mm: f64,
    row_mm: f64,
) {
    let record = serde_json::json!({
        "magic": "OCS1",
        "version": 1,
        "viewportId": 1,
        "displayArea": {
            "cssWidth": 800.0,
            "cssHeight": 600.0,
            "columns": 32,
            "rows": 16,
            "columnMillimetres": column_mm,
            "rowMillimetres": row_mm,
            "shiftColumn": 0.25,
            "shiftRow": -0.5,
            "fitted": fitted,
            "anchorX": 100.0,
            "anchorY": 80.0,
            "fitScale": if fitted { 1.5 } else { 4.0 }
        },
        "camera": {
            "panX": 1.5,
            "panY": -2.0,
            "zoom": zoom,
            "radians": radians,
            "flipHorizontal": flip_horizontal,
            "flipVertical": flip_vertical
        },
        "presentation": {
            "modality": { "kind": "rescale", "slope": 1.0, "intercept": 0.0 },
            "voi": {
                "kind": "window",
                "center": 40.0,
                "width": 400.0,
                "function": "linear_exact",
                "ymin": 0.0,
                "ymax": 255.0
            },
            "photometric": "monochrome2",
            "evidence": { "kind": "shape", "shape": "identity" },
            "colormap": "stops"
        }
    });
    let built = StackViewport::try_identity(1);
    assert!(built.is_ok());
    let Ok(mut viewport) = built else { return };
    let bytes = to_bytes(&record);
    assert!(viewport.restore(&bytes).is_ok());
    let matrix = viewport.view().camera_matrix(32, 16);
    assert!(matrix.iter().all(|value| value.is_finite()));
    let saved = require_snapshot(&viewport);
    assert!(viewport.restore(&saved).is_ok());
    assert_matrix(viewport.view().camera_matrix(32, 16), matrix);
    assert_eq!(require_snapshot(&viewport), saved);

    let zero = 0.0_f64.to_bits();
    if fitted || column_mm.to_bits() == zero || row_mm.to_bits() == zero {
        return;
    }
    let mut reference = View::new();
    reference.set_spacing(column_mm, row_mm);
    reference.set_index_shift(0.25, -0.5);
    reference.set_zoom(Zoom::new(zoom));
    reference.set_rotation(Radians::new(radians));
    reference.set_flip_horizontal(flip_horizontal);
    reference.set_flip_vertical(flip_vertical);
    assert!(reference.set_pan(Pt::<Canvas>::new(1.5, -2.0, 0.0)).is_ok());
    assert_matrix(matrix, reference.camera_matrix(32, 16));
    let left = index_bits(viewport.view(), 32, 16, 3.0, 4.0);
    let right = index_bits(&reference, 32, 16, 3.0, 4.0);
    assert_eq!(left, right);
}
