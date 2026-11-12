//! Monochrome and colour frames of one Part 10 object.
//!
//! Windowed samples go through `LutChain`. The grey byte is
//! `display_index`, the same address the colour table uses.
//! A window override replaces the file window with another call of that chain.
//! A Modality LUT Sequence is refused, because this preview does not apply one.
//! A VOI LUT Sequence is refused unless that override replaces it.
//! Enhanced multi-frame rescale and window values come from the functional
//! groups, with the top-level attributes as the fallback.
//! Monochrome invert flips the display byte after the chain. Colour uses
//! the library's per-channel window and invert before display-byte rounding.

use std::cell::Cell;
use std::collections::HashSet;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{LazyLock, Mutex, OnceLock};
use std::thread::ThreadId;
use std::time::Instant;

use dicom_core::dictionary::{DataDictionary, DataDictionaryEntry};
use dicom_core::header::{DataElement, EmptyObject, Header};
use dicom_core::{DicomValue, PrimitiveValue, Tag, VR};
use dicom_object::mem::{InMemDicomObject, InMemElement};
use dicom_object::{DefaultDicomObject, StandardDataDictionary};
use ocelli_codec::{
    DecodePhotometricInterpretation, DecodeSampleLayout, Decoder, FrameDesc, FrameDescInput,
    PixelDataVr, PixelRepresentation, RawDecoder, Registry, register_htj2k_decoders,
    register_jpeg_decoders, register_jpeg2000_decoders, register_jpegls_decoders,
    register_jpegxl_decoders, register_native_and_rle_decoders,
};
use ocelli_core::Stored;
use ocelli_dicom::{
    CineTiming, DispatchPath, EncapsulatedFrameIndex, FrameGeometry, FunctionalGroupSource,
    MetadataElement, MetadataSet, MetadataValue, MultiframeMetadata, ParseError, SeriesOpenReason,
    TopLevelFallback, check_conformance, cine_timing, parse_decimal_string, parse_part10,
};
use ocelli_pixel::{
    AutoWindow, ByteOrder, DecodedLayout, DecodedPhotometric, ImageDimensions, ImagePlane,
    LutChain, ModalityTransform, PhotometricInterpretation, PixelDataEncoding, PlanarConfiguration,
    PresentationLutEvidence, PresentationLutShape, SampleLayout, StoredBits,
    StoredPixelDescription, StoredRange, StoredSampleTable, VoiFunction, VoiTransform, auto_window,
};
use ocelli_render::display_index;
use ocelli_viewport::presentation::WindowPreset;
use ocelli_viewport::view::f32_from_f64;

use crate::colour::{ColourRecipe, ColourScratch, ColourUnavailableReason};

use crate::corners::{
    age_text, dicom_date_text, dicom_time_text, exposure_mas, fd_text, patient_orientation_letters,
    person_name_text,
};
use crate::measure::{CalibrationInputs, FactorInput, SpacingInput, choose_spacing};
use crate::value::{ValueFacts, stored_padding, value_facts};

const ROWS: Tag = Tag(0x0028, 0x0010);
const COLUMNS: Tag = Tag(0x0028, 0x0011);
const BITS_ALLOCATED: Tag = Tag(0x0028, 0x0100);
const BITS_STORED: Tag = Tag(0x0028, 0x0101);
const HIGH_BIT: Tag = Tag(0x0028, 0x0102);
const PIXEL_REPRESENTATION: Tag = Tag(0x0028, 0x0103);
const SAMPLES_PER_PIXEL: Tag = Tag(0x0028, 0x0002);
const PHOTOMETRIC: Tag = Tag(0x0028, 0x0004);
const NUMBER_OF_FRAMES: Tag = Tag(0x0028, 0x0008);
const RESCALE_SLOPE: Tag = Tag(0x0028, 0x1053);
const RESCALE_INTERCEPT: Tag = Tag(0x0028, 0x1052);
const WINDOW_CENTER: Tag = Tag(0x0028, 0x1050);
const WINDOW_WIDTH: Tag = Tag(0x0028, 0x1051);
const VOI_FUNCTION: Tag = Tag(0x0028, 0x1056);
const WINDOW_EXPLANATION: Tag = Tag(0x0028, 0x1055);
const LUT_EXPLANATION: Tag = Tag(0x0028, 0x3003);
const MODALITY_LUT_SEQUENCE: Tag = Tag(0x0028, 0x3000);
const VOI_LUT_SEQUENCE: Tag = Tag(0x0028, 0x3010);
const FRAME_VOI_LUT_SEQUENCE: Tag = Tag(0x0028, 0x9132);
pub(crate) const PIXEL_VALUE_TRANSFORMATION: Tag = Tag(0x0028, 0x9145);
const PRESENTATION_SHAPE: Tag = Tag(0x2050, 0x0020);
const PRESENTATION_SEQUENCE: Tag = Tag(0x2050, 0x0010);
const PIXEL_DATA: Tag = Tag(0x7FE0, 0x0010);
const PIXEL_SPACING: Tag = Tag(0x0028, 0x0030);
const IMAGER_PIXEL_SPACING: Tag = Tag(0x0018, 0x1164);
/// Estimated Radiographic Magnification Factor, PS3.3 C.8.7.5.
const MAGNIFICATION_FACTOR: Tag = Tag(0x0018, 0x1114);
const IMAGE_ORIENTATION: Tag = Tag(0x0020, 0x0037);
const PIXEL_MEASURES_SEQUENCE: Tag = Tag(0x0028, 0x9110);
const PLANE_ORIENTATION_SEQUENCE: Tag = Tag(0x0020, 0x9116);
const SERIES_UID: Tag = Tag(0x0020, 0x000E);
const SERIES_NUMBER: Tag = Tag(0x0020, 0x0011);
pub(crate) const MODALITY: Tag = Tag(0x0008, 0x0060);
const SERIES_DESCRIPTION: Tag = Tag(0x0008, 0x103E);
const INSTANCE_NUMBER: Tag = Tag(0x0020, 0x0013);
const SOP_INSTANCE: Tag = Tag(0x0008, 0x0018);
/// SOP Class UID, PS3.3 C.12.1. The page reads it to tell an SR or Key
/// Object Selection document from an image.
const SOP_CLASS: Tag = Tag(0x0008, 0x0016);
/// Study Instance UID, PS3.3 C.7.2.1.
const STUDY_UID: Tag = Tag(0x0020, 0x000D);
const PATIENT_NAME: Tag = Tag(0x0010, 0x0010);
const PATIENT_ID: Tag = Tag(0x0010, 0x0020);
const STUDY_DATE: Tag = Tag(0x0008, 0x0020);
const STUDY_DESCRIPTION: Tag = Tag(0x0008, 0x1030);
const ACCESSION: Tag = Tag(0x0008, 0x0050);
const FRAME_OF_REFERENCE: Tag = Tag(0x0020, 0x0052);
const PATIENT_BIRTH_DATE: Tag = Tag(0x0010, 0x0030);
const PATIENT_SEX: Tag = Tag(0x0010, 0x0040);
const PATIENT_AGE: Tag = Tag(0x0010, 0x1010);
const STUDY_TIME: Tag = Tag(0x0008, 0x0030);
const INSTITUTION: Tag = Tag(0x0008, 0x0080);
const SLICE_LOCATION: Tag = Tag(0x0020, 0x1041);
const SLICE_THICKNESS: Tag = Tag(0x0018, 0x0050);
const KVP: Tag = Tag(0x0018, 0x0060);
const EXPOSURE: Tag = Tag(0x0018, 0x1152);
const EXPOSURE_MICRO: Tag = Tag(0x0018, 0x1153);
const REPETITION_TIME: Tag = Tag(0x0018, 0x0080);
const ECHO_TIME: Tag = Tag(0x0018, 0x0081);
/// The enhanced CT and MR functional groups the corner reads, PS3.3 2026c
/// C.8.15.3.8, C.8.15.3.9, C.8.13.5.2 and C.8.13.5.4. The group attribute is
/// not always the classic tag: the echo group carries Effective Echo Time.
const CT_EXPOSURE_SEQUENCE: Tag = Tag(0x0018, 0x9321);
const CT_XRAY_DETAILS_SEQUENCE: Tag = Tag(0x0018, 0x9325);
const MR_TIMING_SEQUENCE: Tag = Tag(0x0018, 0x9112);
const MR_ECHO_SEQUENCE: Tag = Tag(0x0018, 0x9114);
const EXPOSURE_MAS: Tag = Tag(0x0018, 0x9332);
const EFFECTIVE_ECHO_TIME: Tag = Tag(0x0018, 0x9082);
/// Patient Orientation, PS3.3 C.7.6.1.1.1.
const PATIENT_ORIENTATION: Tag = Tag(0x0020, 0x0020);
/// Anatomical Orientation Type, whose `QUADRUPED` value PS3.3 C.7.6.1.1.1
/// gives letters of its own.
const ANATOMICAL_ORIENTATION_TYPE: Tag = Tag(0x0010, 0x2210);

/// The most images the page lists for one file, and the most planes
/// `inspect_json` writes for it. The page's `FRAME_LIST_CAP` has the same
/// value, and a page test pins the two.
pub const FRAME_LIST_CAP: usize = 65536;

/// The longest UID, in characters. PS3.5 Table 6.2-1: UI is "64 bytes
/// maximum", and PS3.5 9.1 counts every digit, separator and padding.
pub(crate) const UID_MAX_CHARS: usize = 64;
const MAX_PIXELS: u32 = 8_192 * 8_192;
const MAX_HEADER_ROWS: usize = 4096;
const VALUE_CAP: usize = 1024;
const BULK_TEXT_CAP: usize = 64;

/// One display frame ready to draw.
#[derive(Debug, Clone, PartialEq)]
pub struct Preview {
    /// Columns.
    pub width: u32,
    /// Rows.
    pub height: u32,
    /// Number of Frames, or 1 when that attribute is absent.
    pub frames: u32,
    /// Which automatic window drew the bytes. Always None for colour.
    pub auto: AutoSource,
    /// One grey or three RGB display bytes per pixel, row major.
    pub grey: Vec<u8>,
    /// One grey byte or three packed RGB bytes per pixel.
    pub channels: u8,
    /// A wide palette was explicitly read as 8-bit entries.
    pub palette_fallback: bool,
    /// The file requested ICC colour management, not applied in F-239.
    pub icc: bool,
    /// Zero-based frame that was decoded.
    pub frame: u32,
    /// Window center in modality units for monochrome, display units for colour.
    pub window_center: f32,
    /// Window width that was applied.
    pub window_width: f32,
    /// VOI LUT Function that was applied.
    pub window_function: &'static str,
    /// Inverted presentation: monochrome flips display bytes, while colour
    /// inverts floating channels after its window and before byte rounding.
    pub inverted: bool,
    /// Spacing and edge letters for the shown frame.
    pub facts: FrameFacts,
    /// The calibration source the length tool chooses for this frame,
    /// `pixel-spacing`, `imager-spacing`, `detector` or `uncalibrated`, or
    /// empty when that choice refuses. The scale bar reads it.
    pub calibration_source: &'static str,
    /// Millimetres per row and per column of that choice, row first. Absent
    /// when uncalibrated or refused.
    pub calibration_mm: Option<(f64, f64)>,
    /// The file window pair that was drawn, or `None` for a preset, a
    /// numeric window or an automatic window. Always `None` for colour.
    pub window_index: Option<u32>,
    /// Number of Window Center and Window Width pairs the file gives this
    /// frame, 0 when it gives none or they cannot be paired. Always 0 for
    /// colour, which does not offer the file's window pairs.
    pub window_count: u32,
}

/// The automatic window a preview was drawn at, F-227.
///
/// A monochrome file that stores no window and no VOI LUT Sequence takes the
/// automatic window of `ocelli_pixel`. `Series` is the one window of its
/// series that the page sent back in `X-Ocelli-Series-Window`, `Image` is the
/// window of this image alone. An image with any window of its own is always
/// `None`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AutoSource {
    /// The file window, a preset or a numeric window.
    None,
    /// The series window the request carried.
    Series,
    /// This image's own automatic window.
    Image,
}

impl AutoSource {
    /// The `X-Ocelli-Auto-Window` token.
    #[must_use]
    pub const fn token(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::Series => "series",
            Self::Image => "image",
        }
    }
}

/// The five acquisition tokens of one frame and the mAs link that answered.
///
/// PS3.3 2026c C.7.6.2, C.7.6.16.2.1, C.8.2.1, C.8.3.1, C.8.15.3.8,
/// C.8.15.3.9, C.8.13.5.2 and C.8.13.5.4, resolved for the shown frame
/// under the precedence rule.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CornerTokens {
    /// Slice Thickness of the shown frame, top level or the Pixel Measures
    /// group, as a DS token.
    pub slice_thickness: Option<String>,
    /// kVp of the shown frame, top level or the CT X-Ray Details group.
    pub kvp: Option<String>,
    /// mAs of the shown frame, the Exposure chain or the CT Exposure group,
    /// as the field's own token.
    pub exposure_mas: Option<String>,
    /// `Exposure`, `ExposureInuAs` or `ExposureInmAs`, the link that
    /// answered, or empty when none did.
    pub exposure_source: &'static str,
    /// Repetition Time of the shown frame, top level or the MR Timing group.
    pub repetition_time: Option<String>,
    /// Echo Time of the shown frame, top level Echo Time or the MR Echo
    /// group's Effective Echo Time.
    pub echo_time: Option<String>,
}

impl CornerTokens {
    pub(crate) fn empty() -> Self {
        Self {
            slice_thickness: None,
            kvp: None,
            exposure_mas: None,
            exposure_source: "",
            repetition_time: None,
            echo_time: None,
        }
    }
}

/// Spacing and patient-direction letters for one frame.
///
/// Spacing tokens are row then column. A token that is already a JSON
/// number is kept. Any other finite DS is the parsed value. Letters come
/// from Image Orientation (Patient), and from Patient Orientation only when
/// Image Orientation (Patient) is absent or has zero length. They are empty
/// when the attribute that decides cannot be used, or when Anatomical
/// Orientation Type is `QUADRUPED`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FrameFacts {
    /// Adjacent row spacing, when the file has a usable pair.
    pub row_spacing: Option<String>,
    /// Adjacent column spacing.
    pub column_spacing: Option<String>,
    /// `PixelSpacing`, `ImagerPixelSpacing`, or empty.
    pub spacing_source: &'static str,
    /// Patient direction at the left edge.
    pub edge_left: String,
    /// Patient direction at the right edge.
    pub edge_right: String,
    /// Patient direction at the top edge.
    pub edge_top: String,
    /// Patient direction at the bottom edge.
    pub edge_bottom: String,
    /// `ImageOrientationPatient` or `PatientOrientation`, the attribute the
    /// letters came from, or empty when there are no letters.
    pub edge_source: &'static str,
    /// The acquisition tokens of the shown frame, or `None` when no field
    /// yields a token at any level, which includes a field that is present
    /// but unusable, stopped by the top-level rule, or inside a malformed
    /// group. Boxed, because the frame cache charges [`HeldFrame`] by its
    /// struct size and the tokens' own bytes then ride the heap the way
    /// the edge letters' do.
    pub corners: Option<Box<CornerTokens>>,
}

/// Which frame to decode, and an optional replacement window.
///
/// Absent window fields keep the monochrome file window or automatic
/// window, and use centre 128 and width 256 for colour. Monochrome invert
/// follows `display_index`, while colour invert precedes byte rounding.
#[derive(Debug, Clone, Copy, Default)]
pub struct FrameRequest {
    /// Zero-based frame index.
    pub frame: u32,
    /// Invert the presentation, per channel on colour.
    pub invert: bool,
    /// Replacement Window Center. Set together with [`Self::window_width`].
    pub window_center: Option<f32>,
    /// Replacement Window Width.
    pub window_width: Option<f32>,
    /// Replacement VOI LUT Function. Absent keeps the monochrome file
    /// function and uses LINEAR on colour. Other functions refuse on colour.
    pub window_function: Option<VoiFunction>,
    /// Named window from `WindowPreset`. It refuses a VOI LUT Sequence
    /// and it is not combined with a numeric window.
    pub window_preset: Option<WindowPreset>,
    /// Zero-based index into the file's Window Center and Window Width
    /// pairs. Absent is index 0. It is not combined with a preset or a
    /// numeric window, and it refuses a VOI LUT Sequence.
    pub window_index: Option<u32>,
    /// Centre and width of the series window, F-227, exactly as
    /// `/series-window` wrote them and the page sent them back. Applied only
    /// to an image that stores no window of its own, in place of that image's
    /// own automatic window. It is not combined with a preset, a numeric
    /// window or a file window index.
    pub series_window: Option<(f32, f32)>,
}

impl PreviewError {
    /// A stable token for the page. The sentence in [`std::fmt::Display`] is the one a person reads.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::NotPart10 => "not-part10",
            Self::Attribute => "attribute",
            Self::ColourUnavailable(_) => "colour-unavailable",
            Self::ColourNotApplicable => "colour-not-applicable",
            Self::TooLarge => "too-large",
            Self::Decode => "decode",
            Self::Window => "window",
            Self::Frame => "frame",
            Self::Point => "point",
            Self::Calibration => "calibration",
            Self::SelfIntersecting => "self-intersecting",
            Self::Degenerate => "degenerate",
            Self::Modality => "modality",
            Self::Region => "region",
        }
    }
}

/// Why a file could not be shown.
///
/// No variant carries bytes, names, or sample values.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum PreviewError {
    /// The bytes are not a Part 10 object this build can read.
    #[error("the file is not a DICOM Part 10 object this preview can read")]
    NotPart10,
    /// An image attribute is missing or not usable.
    #[error("an image attribute is missing or not usable")]
    Attribute,
    /// This build cannot draw the colour shape, for a closed reason.
    #[error("{0}")]
    ColourUnavailable(ColourUnavailableReason),
    /// A scalar window operation was requested on colour.
    #[error("this window operation does not apply to a colour image")]
    ColourNotApplicable,
    /// The frame is larger than the preview will allocate.
    #[error("the frame is larger than this preview will draw")]
    TooLarge,
    /// The codec registry or the frame decode refused the pixels.
    #[error("the frame could not be decoded")]
    Decode,
    /// The LUT chain refused the window, a LUT sequence this preview does not
    /// apply, or the presentation.
    #[error("the window or presentation could not be applied")]
    Window,
    /// The requested frame index is outside Number of Frames.
    #[error("the requested frame is not in this file")]
    Frame,
    /// A requested point is outside the image.
    #[error("the point is outside this image")]
    Point,
    /// A present spacing attribute cannot be used, so nothing is measured.
    #[error("the spacing in this file cannot be used for a measurement")]
    Calibration,
    /// A freeform outline crosses itself.
    #[error("This outline crosses itself, so its area is not measured.")]
    SelfIntersecting,
    /// The shape has no length, no angle arm or no sample to measure.
    #[error("the shape has nothing to measure")]
    Degenerate,
    /// A window preset on an image whose Modality is not `CT`.
    #[error("these presets are in Hounsfield units and apply to CT only")]
    Modality,
    /// A region window rectangle holds no pixel centre, or only padding.
    #[error("the region holds no pixel with a value to window")]
    Region,
}

/// Decode frame 0 with the file window.
///
/// # Errors
///
/// Returns [`PreviewError`] before a grey byte is written when the file,
/// the frame, or the window cannot be shown.
pub fn preview_first_frame(bytes: &[u8]) -> Result<Preview, PreviewError> {
    preview_frame(bytes, FrameRequest::default())
}

/// Decode one frame and map it through the LUT chain.
///
/// # Errors
///
/// Returns [`PreviewError`] before a grey byte is written when the file,
/// the frame, or the window cannot be shown.
pub fn preview_frame(bytes: &[u8], request: FrameRequest) -> Result<Preview, PreviewError> {
    validate_frame_request(request)?;
    let mut native = Vec::new();
    let mut samples = Vec::new();
    let held = decode_held(
        bytes,
        request.frame,
        &mut native,
        &mut samples,
        &mut |_| {},
        &mut |_, _| Vec::new(),
    )?;
    let mut scratch = Vec::new();
    let mut grey = Vec::new();
    render_held(
        &held,
        request,
        &mut scratch,
        &mut ColourScratch::default(),
        &mut grey,
    )
}

/// Refuse a window header that cannot name one window.
///
/// # Errors
///
/// Returns [`PreviewError::Window`] when center and width are not paired,
/// a preset is combined with a numeric window or a function, a file window
/// index is combined with a preset or a numeric window, a series window is
/// combined with any of those or with a function, a series window has no
/// positive finite width, or a number is not finite.
pub(crate) fn validate_frame_request(request: FrameRequest) -> Result<(), PreviewError> {
    if request.window_center.is_some() != request.window_width.is_some() {
        return Err(PreviewError::Window);
    }
    if request.window_index.is_some()
        && (request.window_preset.is_some()
            || request.window_center.is_some()
            || request.window_width.is_some())
    {
        return Err(PreviewError::Window);
    }
    if request.window_preset.is_some()
        && (request.window_center.is_some()
            || request.window_width.is_some()
            || request.window_function.is_some())
    {
        return Err(PreviewError::Window);
    }
    if request
        .window_center
        .is_some_and(|center| !center.is_finite())
        || request.window_width.is_some_and(|width| !width.is_finite())
    {
        return Err(PreviewError::Window);
    }
    if let Some((center, width)) = request.series_window {
        // One window per request. A series window beside another named one
        // would have to be silently preferred, so it is refused instead. A
        // function alone would draw the series centre and width under another
        // function and still say series, review pass 16 N2.
        if request.window_preset.is_some()
            || request.window_center.is_some()
            || request.window_width.is_some()
            || request.window_index.is_some()
            || request.window_function.is_some()
        {
            return Err(PreviewError::Window);
        }
        if !center.is_finite() || !width.is_finite() || width <= 0.0 {
            return Err(PreviewError::Window);
        }
    }
    Ok(())
}

/// How many times this thread has started a decode.
///
/// A render that calls [`decode_held`] again counts as a fresh decode.
pub(crate) fn decode_ticks() -> u64 {
    DECODE_TICKS.with(Cell::get)
}

thread_local! {
    static DECODE_TICKS: Cell<u64> = const { Cell::new(0) };
    static PHASE: Cell<PhaseRecord> = const { Cell::new(PhaseRecord::ZERO) };
}

/// Threads inside `decode_held` right now.
static DECODES_NOW: AtomicUsize = AtomicUsize::new(0);
/// Highest `DECODES_NOW` this process has seen.
static DECODES_PEAK: AtomicUsize = AtomicUsize::new(0);
/// Distinct threads that have entered `decode_held`.
static DECODE_THREADS: LazyLock<Mutex<HashSet<ThreadId>>> =
    LazyLock::new(|| Mutex::new(HashSet::new()));

fn bump_peak(slot: &AtomicUsize, value: usize) {
    let mut current = slot.load(Ordering::Relaxed);
    while value > current {
        match slot.compare_exchange_weak(current, value, Ordering::Relaxed, Ordering::Relaxed) {
            Ok(_) => return,
            Err(observed) => current = observed,
        }
    }
}

fn lock_threads() -> std::sync::MutexGuard<'static, HashSet<ThreadId>> {
    DECODE_THREADS
        .lock()
        .unwrap_or_else(|poison| poison.into_inner())
}

/// Threads that have decoded, the peak overlapping decodes, and how many are in one now.
pub(crate) fn decode_schedule() -> (usize, usize, usize) {
    let threads = lock_threads().len();
    (
        threads,
        DECODES_PEAK.load(Ordering::Relaxed),
        DECODES_NOW.load(Ordering::Relaxed),
    )
}

/// Counts one `decode_held` until it returns.
struct DecodeInFlight;

impl DecodeInFlight {
    fn enter() -> Self {
        let now = DECODES_NOW
            .fetch_add(1, Ordering::Relaxed)
            .saturating_add(1);
        bump_peak(&DECODES_PEAK, now);
        lock_threads().insert(std::thread::current().id());
        Self
    }
}

impl Drop for DecodeInFlight {
    fn drop(&mut self) {
        let mut current = DECODES_NOW.load(Ordering::Relaxed);
        loop {
            let next = current.saturating_sub(1);
            match DECODES_NOW.compare_exchange_weak(
                current,
                next,
                Ordering::Relaxed,
                Ordering::Relaxed,
            ) {
                Ok(_) => return,
                Err(observed) => current = observed,
            }
        }
    }
}

#[cfg(test)]
thread_local! {
    static BORROWED_DECODES: Cell<u64> = const { Cell::new(0) };
}

/// One preview's phase times, in microseconds. Zero unless profiling.
#[derive(Clone, Copy)]
pub(crate) struct PhaseRecord {
    pub lookup_us: u128,
    pub unpack_us: u128,
    pub chain_us: u128,
    pub voi_us: u128,
    pub grey_us: u128,
    pub fused_us: u128,
    pub write_us: u128,
    pub pixels: usize,
}

impl PhaseRecord {
    const ZERO: Self = Self {
        lookup_us: 0,
        unpack_us: 0,
        chain_us: 0,
        voi_us: 0,
        grey_us: 0,
        fused_us: 0,
        write_us: 0,
        pixels: 0,
    };
}

/// `OCELLI_PROFILE` splits a held window step. Unset costs one flag read.
pub(crate) fn profiling() -> bool {
    static FLAG: OnceLock<bool> = OnceLock::new();
    *FLAG.get_or_init(|| std::env::var_os("OCELLI_PROFILE").is_some())
}

pub(crate) fn reset_phases() {
    PHASE.with(|cell| cell.set(PhaseRecord::ZERO));
}

pub(crate) fn note_lookup(micros: u128) {
    PHASE.with(|cell| {
        let mut phase = cell.get();
        phase.lookup_us = micros;
        cell.set(phase);
    });
}

pub(crate) fn note_write(micros: u128) {
    PHASE.with(|cell| {
        let mut phase = cell.get();
        phase.write_us = micros;
        cell.set(phase);
    });
}

pub(crate) fn phases() -> PhaseRecord {
    PHASE.with(Cell::get)
}

/// Copy a worker's phase times onto this thread. Lookup and write stay put.
pub(crate) fn merge_worker_phases(from: PhaseRecord) {
    PHASE.with(|cell| {
        let mut phase = cell.get();
        phase.unpack_us = from.unpack_us;
        phase.chain_us = from.chain_us;
        phase.voi_us = from.voi_us;
        phase.grey_us = from.grey_us;
        phase.fused_us = from.fused_us;
        phase.pixels = from.pixels;
        cell.set(phase);
    });
}

fn note_render(update: impl FnOnce(&mut PhaseRecord)) {
    PHASE.with(|cell| {
        let mut phase = cell.get();
        update(&mut phase);
        cell.set(phase);
    });
}

fn note_decode() {
    DECODE_TICKS.with(|cell| cell.set(cell.get().wrapping_add(1)));
}

#[cfg(test)]
fn borrowed_decode_ticks() -> u64 {
    BORROWED_DECODES.with(Cell::get)
}

fn note_borrowed_decode() {
    #[cfg(test)]
    BORROWED_DECODES.with(|cell| cell.set(cell.get().wrapping_add(1)));
}

/// One decoded frame held so a later window does not decode it again.
///
/// The samples are decoder output. Display bytes are not stored, because a
/// window change maps the decoded samples through their recipe again.
pub(crate) struct HeldFrame {
    /// Decoder output for this frame. Its capacity is charged in
    /// [`HeldFrame::charged_bytes`].
    pub(crate) samples: Vec<u8>,
    /// Image Pixel evidence for [`Self::samples`].
    pub(crate) description: StoredPixelDescription,
    /// Byte order of [`Self::samples`]. Native words are already little endian.
    pub(crate) byte_order: ByteOrder,
    /// Request-independent chain inputs, or the refusal that blocks every window.
    pub(crate) recipe: Result<Recipe, PreviewError>,
    /// Spacing and edge letters. Returned only after the chain is built.
    pub(crate) facts: Result<FrameFacts, PreviewError>,
    /// This image's own automatic window, F-227, computed once when the file
    /// has no window. Otherwise an error, and never read.
    pub(crate) auto: Result<AutoWindow, PreviewError>,
    /// Request-independent scalar value facts. Colour uses its recipe for
    /// RGB readouts and keeps ColourNotApplicable here. [`render_held`] does
    /// not read these facts. Boxed, so
    /// a refused frame does not carry the facts' size inline. The facts'
    /// heap bytes are charged in [`HeldFrame::charged_bytes`].
    pub(crate) value: Result<Box<ValueFacts>, PreviewError>,
    /// The spacing attributes a measurement reads, as the file stores them,
    /// or the refusal that blocks every measurement. Nothing is derived
    /// here. `select_calibration` runs per request. The inputs own no heap,
    /// so [`HeldFrame::charged_bytes`] counts them in the struct size.
    pub(crate) calibration: Result<CalibrationInputs, PreviewError>,
    /// Number of Frames, or 1 when that attribute is absent.
    pub(crate) frames: u32,
    /// Columns.
    pub(crate) width: u32,
    /// Rows.
    pub(crate) height: u32,
}

impl HeldFrame {
    pub(crate) fn channels(&self) -> usize {
        if matches!(
            self.description.layout.photometric_interpretation(),
            PhotometricInterpretation::Monochrome1 | PhotometricInterpretation::Monochrome2
        ) {
            1
        } else {
            3
        }
    }

    /// The bytes the cache charges for this frame.
    ///
    /// The capacity of the sample buffer, the struct itself, and the heap
    /// bytes of the value facts when the frame has them.
    pub(crate) fn charged_bytes(&self) -> usize {
        let value = self.value.as_ref().map_or(0, |facts| facts.heap_bytes());
        let colour = match &self.recipe {
            Ok(Recipe::Colour(recipe)) => recipe.heap_bytes(),
            _ => 0,
        };
        self.samples
            .capacity()
            .saturating_add(size_of::<Self>())
            .saturating_add(value)
            .saturating_add(colour)
    }
}

/// Existing monochrome admission allowance, unchanged by colour support.
pub(crate) fn monochrome_held_overhead() -> usize {
    size_of::<HeldFrame>()
        .saturating_add(size_of::<crate::value::ValueFacts>())
        .saturating_add(crate::value::UNIT_MAX_BYTES)
}

/// Chain inputs that do not depend on the request.
pub(crate) enum Recipe {
    Monochrome(MonochromeRecipe),
    Colour(Box<ColourRecipe>),
}

pub(crate) struct MonochromeRecipe {
    /// Modality stage. A Modality LUT Sequence never reaches this value.
    pub(crate) modality: ModalityTransform,
    /// Photometric Interpretation of the frame.
    pub(crate) photometric: PhotometricInterpretation,
    /// Presentation LUT evidence. Checked after the window branch.
    pub(crate) evidence: Result<PresentationLutEvidence, PreviewError>,
    /// Whether a VOI LUT Sequence is present. A numeric override does not read it.
    pub(crate) voi_lut: Result<bool, PreviewError>,
    /// The file window, or the refusal that applies only when the file window is used.
    pub(crate) file_window: Result<FileWindow, PreviewError>,
    /// Modality (0008,0060) is `CT`. The presets are Hounsfield numbers, so
    /// they are refused on any other image. A Modality that is absent or not
    /// text is not `CT`.
    pub(crate) ct: bool,
}

/// The file window, computed once.
pub(crate) enum FileWindow {
    /// No Window Center or Window Width, F-227. The automatic window of
    /// `ocelli_pixel`, LINEAR_EXACT from the 1st to the 99th percentile.
    Auto {
        /// This frame's Pixel Padding Value range in stored space, PS3.3
        /// C.7.5.1.1.2, or the refusal that leaves the frame out of a series
        /// window. The series pool reads it.
        padding: Result<Option<StoredRange>, PreviewError>,
    },
    /// The file's centers, widths, and function.
    ///
    /// PS3.3 C.11.2.1.2.2 pairs the values by position, so an empty value
    /// keeps its slot as `None` rather than moving the pairs after it.
    Explicit {
        /// Window Center values, one slot per stored value.
        centers: Vec<Option<f32>>,
        /// Window Width values, one slot per stored value.
        widths: Vec<Option<f32>>,
        /// VOI LUT Function from the file, or LINEAR when it is absent.
        function: VoiFunction,
    },
}

/// Decode one frame into samples and a window recipe.
///
/// `acquire` receives the requested sample bytes and the held frame
/// overhead, including any prepared colour box and palette allocations.
/// `on_samples` runs once the sample buffer has its capacity and before
/// those bytes are decoded. The caller can count that buffer while it is
/// still outside the cache.
///
/// # Errors
///
/// Returns the refusals that happen before a sample can be held: Part 10,
/// image attributes, size, photometric interpretation, frame index, and decode.
pub(crate) fn decode_held<F, A>(
    bytes: &[u8],
    frame: u32,
    native: &mut Vec<u8>,
    samples: &mut Vec<u8>,
    on_samples: &mut F,
    acquire: &mut A,
) -> Result<HeldFrame, PreviewError>
where
    F: FnMut(usize),
    A: FnMut(usize, usize) -> Vec<u8>,
{
    let _in_flight = DecodeInFlight::enter();
    note_decode();
    if let Some((header, range)) = split_pixel_payload(bytes)
        && let Some(pixels) = bytes.get(range)
        && let Ok(parsed) = parse_part10(&header)
        && native_dispatch(parsed.transfer_syntax().dispatch_path())
        && borrowed_pixels_fit(parsed.object(), pixels)
    {
        return finish_decode(
            &parsed,
            Some(pixels),
            frame,
            native,
            samples,
            on_samples,
            acquire,
        );
    }
    let parsed = parse_part10(bytes).map_err(part10_error)?;
    finish_decode(&parsed, None, frame, native, samples, on_samples, acquire)
}

struct FrameParts {
    frames: u32,
    width: u32,
    height: u32,
    expected: usize,
    description: StoredPixelDescription,
}

fn frame_parts(
    object: &DefaultDicomObject,
    frame: u32,
    encoding: PixelDataEncoding,
) -> Result<(FrameParts, FrameDesc), PreviewError> {
    let rows = required_u16(object, ROWS)?;
    let columns = required_u16(object, COLUMNS)?;
    let height = u32::from(rows);
    let width = u32::from(columns);
    let pixel_count = height.checked_mul(width).ok_or(PreviewError::TooLarge)?;
    if pixel_count == 0 || pixel_count > MAX_PIXELS {
        return Err(PreviewError::TooLarge);
    }
    let photometric = photometric(object)?;
    let samples = required_u16(object, SAMPLES_PER_PIXEL)?;
    let colour = !matches!(
        photometric,
        PhotometricInterpretation::Monochrome1 | PhotometricInterpretation::Monochrome2
    );
    let bits_allocated = required_u16(object, BITS_ALLOCATED)?;
    let bits_stored = required_u16(object, BITS_STORED)?;
    let high_bit = required_u16(object, HIGH_BIT)?;
    let representation = pixel_representation(object)?;
    if colour
        && (representation != ocelli_pixel::PixelRepresentation::Unsigned
            || !(8..=16).contains(&bits_stored))
    {
        return Err(PreviewError::ColourUnavailable(
            ColourUnavailableReason::Samples,
        ));
    }
    let frames = frame_count(object)?;
    if frame >= frames {
        return Err(PreviewError::Frame);
    }
    let vr = pixel_vr(object)?;
    let desc = FrameDesc::new(FrameDescInput {
        rows,
        columns,
        samples_per_pixel: samples,
        bits_allocated,
        bits_stored,
        high_bit,
        pixel_representation: match representation {
            ocelli_pixel::PixelRepresentation::Unsigned => PixelRepresentation::Unsigned,
            ocelli_pixel::PixelRepresentation::Signed => PixelRepresentation::Signed,
        },
        photometric_interpretation: photometric_name(photometric).to_owned(),
        pixel_data_vr: vr,
    })
    .map_err(|_| PreviewError::Attribute)?;
    let stored_bits = StoredBits::new(
        u8::try_from(bits_allocated).map_err(|_| PreviewError::Attribute)?,
        u8::try_from(bits_stored).map_err(|_| PreviewError::Attribute)?,
        u8::try_from(high_bit).map_err(|_| PreviewError::Attribute)?,
        representation,
    )
    .map_err(|_| PreviewError::Attribute)?;
    let planar = if samples == 3 {
        Some(if encoding == PixelDataEncoding::Encapsulated {
            PlanarConfiguration::Interleaved
        } else {
            match required_u16(object, Tag(0x0028, 0x0006))? {
                0 => PlanarConfiguration::Interleaved,
                1 => PlanarConfiguration::Planar,
                _ => {
                    return Err(PreviewError::ColourUnavailable(
                        ColourUnavailableReason::Space,
                    ));
                }
            }
        })
    } else {
        None
    };
    let layout = SampleLayout::new(
        u8::try_from(samples).map_err(|_| PreviewError::Attribute)?,
        photometric,
        planar,
    )
    .map_err(|_| PreviewError::ColourUnavailable(ColourUnavailableReason::Space))?;
    let dimensions = ImageDimensions::new(height, width).map_err(|_| PreviewError::Attribute)?;
    let description = StoredPixelDescription::new(dimensions, layout, stored_bits);
    let expected = if encoding == PixelDataEncoding::Encapsulated {
        desc.output_len()
    } else {
        description
            .sample_count()
            .map_err(|_| PreviewError::ColourUnavailable(ColourUnavailableReason::ChromaColumns))?
            .checked_mul(usize::from(stored_bits.bits_allocated() / 8))
            .ok_or(PreviewError::Decode)?
    };
    Ok((
        FrameParts {
            frames,
            width,
            height,
            expected,
            description,
        },
        desc,
    ))
}

fn held_from_samples(
    object: &DefaultDicomObject,
    parts: FrameParts,
    samples: Vec<u8>,
    frame_index: usize,
    decoded: DecodeFacts,
    colour: Option<(MetadataSet, Result<ColourRecipe, PreviewError>)>,
) -> Result<HeldFrame, PreviewError> {
    let prepared = prepare_held(object, &parts.description, &samples, frame_index, colour);
    let mut description = parts.description;
    if decoded.photometric == DecodedPhotometric::Rgb {
        description.layout = SampleLayout::new(
            3,
            PhotometricInterpretation::Rgb,
            Some(PlanarConfiguration::Interleaved),
        )
        .map_err(|_| PreviewError::Attribute)?;
    } else if decoded.layout == DecodedLayout::Interleaved
        && description.layout.samples_per_pixel() == 3
    {
        description.layout = SampleLayout::new(
            3,
            description.layout.photometric_interpretation(),
            Some(PlanarConfiguration::Interleaved),
        )
        .map_err(|_| PreviewError::Attribute)?;
    }
    Ok(HeldFrame {
        samples,
        description,
        byte_order: ByteOrder::LittleEndian,
        recipe: prepared.recipe,
        facts: prepared.facts,
        auto: prepared.auto,
        value: prepared.value,
        calibration: prepared.calibration,
        frames: parts.frames,
        width: parts.width,
        height: parts.height,
    })
}

fn finish_decode<F, A>(
    parsed: &ocelli_dicom::ParsedDicom,
    source: Option<&[u8]>,
    frame: u32,
    native: &mut Vec<u8>,
    samples: &mut Vec<u8>,
    on_samples: &mut F,
    acquire: &mut A,
) -> Result<HeldFrame, PreviewError>
where
    F: FnMut(usize),
    A: FnMut(usize, usize) -> Vec<u8>,
{
    let object = parsed.object();
    let encoding = if parsed.transfer_syntax().dispatch_path() == DispatchPath::Encapsulated {
        PixelDataEncoding::Encapsulated
    } else {
        PixelDataEncoding::Native
    };
    let (mut parts, desc) = frame_parts(object, frame, encoding)?;
    let decoded = decode_facts(parsed, &desc)?;
    parts.expected = decoded.len;
    if decoded.len == 0 {
        return Err(PreviewError::Decode);
    }
    let frame_index = usize::try_from(frame).map_err(|_| PreviewError::Frame)?;
    // Prepare owned colour inputs once, before the cache decides which
    // unread frame makes room. A palette's actual capacity is part of that
    // admission, not an extra discovered only when the frame is stored.
    let colour = if matches!(
        parts.description.layout.photometric_interpretation(),
        PhotometricInterpretation::Monochrome1 | PhotometricInterpretation::Monochrome2
    ) {
        None
    } else {
        let metadata = project_metadata(object)?;
        let recipe = ColourRecipe::new(
            &metadata,
            parts.description,
            decoded.encoding,
            decoded.photometric,
            decoded.layout,
        );
        Some((metadata, recipe))
    };
    let overhead = colour
        .as_ref()
        .map_or_else(monochrome_held_overhead, |(_, recipe)| {
            size_of::<HeldFrame>()
                .saturating_add(recipe.as_ref().map_or(0, ColourRecipe::heap_bytes))
        });
    prepare_samples(samples, parts.expected, overhead, acquire)?;
    on_samples(samples.capacity());
    decode_frame(
        parsed,
        &desc,
        parts.frames,
        frame_index,
        source,
        native,
        samples,
    )?;
    let taken = std::mem::take(samples);
    held_from_samples(object, parts, taken, frame_index, decoded, colour)
}

/// Size the sample buffer to the decoded frame.
///
/// A buffer from `acquire` is used when it already has room for the frame.
/// `resize` then writes every byte, so a recycled buffer does not keep its
/// previous contents.
fn prepare_samples<A>(
    samples: &mut Vec<u8>,
    output_len: usize,
    overhead: usize,
    acquire: &mut A,
) -> Result<(), PreviewError>
where
    A: FnMut(usize, usize) -> Vec<u8>,
{
    if samples.capacity() < output_len {
        let got = acquire(output_len, overhead);
        if got.capacity() >= output_len {
            *samples = got;
        }
    }
    samples.clear();
    // Admission reserved this exact sample length. Avoid Vec's minimum
    // fresh capacity making a tiny colour frame larger than that claim.
    samples
        .try_reserve_exact(output_len)
        .map_err(|_| PreviewError::TooLarge)?;
    samples.resize(output_len, 0);
    if samples.len() != output_len {
        return Err(PreviewError::Decode);
    }
    Ok(())
}

struct Prepared {
    recipe: Result<Recipe, PreviewError>,
    facts: Result<FrameFacts, PreviewError>,
    auto: Result<AutoWindow, PreviewError>,
    value: Result<Box<ValueFacts>, PreviewError>,
    calibration: Result<CalibrationInputs, PreviewError>,
}

fn prepare_held(
    object: &DefaultDicomObject,
    description: &StoredPixelDescription,
    samples: &[u8],
    frame_index: usize,
    colour: Option<(MetadataSet, Result<ColourRecipe, PreviewError>)>,
) -> Prepared {
    let photometric = description.layout.photometric_interpretation();
    let (metadata, colour_recipe) = match colour {
        Some((metadata, recipe)) => (Ok(metadata), Some(recipe)),
        None => (project_metadata(object), None),
    };
    let metadata = match metadata {
        Ok(metadata) => metadata,
        Err(error) => {
            return Prepared {
                recipe: Err(error),
                facts: Err(error),
                auto: Err(error),
                value: Err(error),
                calibration: Err(error),
            };
        }
    };
    let multi = match MultiframeMetadata::new(&metadata) {
        Ok(multi) => multi,
        Err(_) => {
            return Prepared {
                recipe: Err(PreviewError::Attribute),
                facts: Err(PreviewError::Attribute),
                auto: Err(PreviewError::Attribute),
                value: Err(PreviewError::Attribute),
                calibration: Err(PreviewError::Attribute),
            };
        }
    };
    let facts = frame_facts(&metadata, &multi, frame_index);
    // Spacing is not stage 1, so a refused rescale below never refuses a length.
    let calibration = calibration_inputs(&metadata, &multi, frame_index);
    if let Some(recipe) = colour_recipe {
        return Prepared {
            recipe: recipe.map(|recipe| Recipe::Colour(Box::new(recipe))),
            facts,
            calibration,
            auto: Err(PreviewError::ColourNotApplicable),
            value: Err(PreviewError::ColourNotApplicable),
        };
    }
    if let Err(error) = modality_blocked(&multi, frame_index) {
        return Prepared {
            recipe: Err(error),
            facts,
            auto: Err(error),
            value: Err(error),
            calibration,
        };
    }
    let (slope, intercept) = match rescale(&multi, frame_index) {
        Ok(pair) => pair,
        Err(error) => {
            return Prepared {
                recipe: Err(error),
                facts,
                auto: Err(error),
                value: Err(error),
                calibration,
            };
        }
    };
    let modality = match ModalityTransform::new(None, Some(slope), Some(intercept)) {
        Ok(modality) => modality,
        Err(_) => {
            return Prepared {
                recipe: Err(PreviewError::Window),
                facts,
                auto: Err(PreviewError::Window),
                value: Err(PreviewError::Window),
                calibration,
            };
        }
    };
    // The value's stage 1 is this transform, so it refuses exactly when the
    // preview's stage 1 does and never over a window or presentation input.
    let value = value_facts(
        &metadata,
        object,
        &multi,
        frame_index,
        description,
        &modality,
    )
    .map(Box::new);
    let voi_lut = group_has(
        &multi,
        frame_index,
        FRAME_VOI_LUT_SEQUENCE,
        VOI_LUT_SEQUENCE,
    );
    let evidence = presentation(object);
    // F-210's padding reader, the one `value_facts` uses, read here on its
    // own so a unit the value refuses does not hide the padding.
    let padding = stored_padding(&metadata, description)
        .map_err(|_| PreviewError::Window)
        .and_then(stored_range);
    let (file_window, auto) = file_window_of(
        &multi,
        &modality,
        description,
        samples,
        frame_index,
        padding,
    );
    let ct = optional_text(object, MODALITY).ok().flatten().as_deref() == Some("CT");
    Prepared {
        recipe: Ok(Recipe::Monochrome(MonochromeRecipe {
            modality,
            photometric,
            evidence,
            voi_lut,
            file_window,
            ct,
        })),
        facts,
        auto,
        value,
        calibration,
    }
}

/// PS3.3 C.11.1. A Modality LUT Sequence replaces rescale, so identity slope is wrong.
fn modality_blocked(
    multi: &MultiframeMetadata<'_>,
    frame_index: usize,
) -> Result<(), PreviewError> {
    if group_has(
        multi,
        frame_index,
        PIXEL_VALUE_TRANSFORMATION,
        MODALITY_LUT_SEQUENCE,
    )? {
        return Err(PreviewError::Window);
    }
    Ok(())
}

/// The padding range as stored-space `f32` bounds for the automatic window.
///
/// `read_padding` gives one US or SS value and limit, so every bound fits
/// 16 bits and is exact in `f32`. A bound outside that is refused rather
/// than rounded, the rule F-210 applies to the value at the pointer.
fn stored_range(
    padding: Option<ocelli_dicom::PaddingRange>,
) -> Result<Option<StoredRange>, PreviewError> {
    let Some(range) = padding else {
        return Ok(None);
    };
    let low = exact_bound(range.value())?;
    let high = match range.limit() {
        Some(limit) => exact_bound(limit)?,
        None => low,
    };
    Ok(Some(StoredRange {
        low: Stored(low),
        high: Stored(high),
    }))
}

/// A US or SS padding bound as the `f32` it is. Anything wider is refused.
fn exact_bound(value: i32) -> Result<f32, PreviewError> {
    if let Ok(narrow) = i16::try_from(value) {
        return Ok(f32::from(narrow));
    }
    u16::try_from(value)
        .map(f32::from)
        .map_err(|_| PreviewError::Window)
}

fn file_window_of(
    multi: &MultiframeMetadata<'_>,
    modality: &ModalityTransform,
    description: &StoredPixelDescription,
    samples: &[u8],
    frame_index: usize,
    padding: Result<Option<StoredRange>, PreviewError>,
) -> (
    Result<FileWindow, PreviewError>,
    Result<AutoWindow, PreviewError>,
) {
    let centers = match decimal_slots(multi, frame_index, FRAME_VOI_LUT_SEQUENCE, WINDOW_CENTER) {
        Ok(centers) => centers,
        Err(error) => return (Err(error), Err(error)),
    };
    let widths = match decimal_slots(multi, frame_index, FRAME_VOI_LUT_SEQUENCE, WINDOW_WIDTH) {
        Ok(widths) => widths,
        Err(error) => return (Err(error), Err(error)),
    };
    // Every slot empty carries no number, the same as an absent attribute.
    if centers.iter().all(Option::is_none) && widths.iter().all(Option::is_none) {
        let count = match description.sample_count() {
            Ok(count) => count,
            Err(_) => return (Err(PreviewError::Attribute), Err(PreviewError::Attribute)),
        };
        let mut stored = vec![Stored(0.0); count];
        if description
            .unpack(samples, ByteOrder::LittleEndian, &mut stored)
            .is_err()
        {
            return (Err(PreviewError::Decode), Err(PreviewError::Decode));
        }
        // F-227. The per-image window is a pool of one, the same library
        // function the series window folds. A padding range that cannot be
        // read is not guessed around, so this image's own window refuses,
        // while a series window the page sends can still draw it.
        let auto = padding.and_then(|range| {
            auto_window(&stored, modality, range).map_err(|_| PreviewError::Window)
        });
        return (Ok(FileWindow::Auto { padding }), auto);
    }
    if centers.is_empty() || centers.len() != widths.len() {
        return (Err(PreviewError::Window), Err(PreviewError::Window));
    }
    match voi_function(multi, frame_index) {
        Ok(function) => (
            Ok(FileWindow::Explicit {
                centers,
                widths,
                function,
            }),
            Err(PreviewError::Window),
        ),
        Err(error) => (Err(error), Err(error)),
    }
}

/// Map a held frame through one window.
///
/// `scratch` grows to the frame and keeps its spare capacity.
///
/// # Errors
///
/// Returns [`PreviewError`] when the held recipe or this window cannot be drawn.
pub(crate) fn render_held(
    held: &HeldFrame,
    request: FrameRequest,
    scratch: &mut Vec<Stored>,
    colour: &mut ColourScratch,
    grey: &mut Vec<u8>,
) -> Result<Preview, PreviewError> {
    let recipe = match &held.recipe {
        Ok(recipe) => recipe,
        Err(error) => return Err(*error),
    };
    let recipe = match recipe {
        Recipe::Monochrome(recipe) => recipe,
        Recipe::Colour(recipe) => {
            let (center, width) = crate::colour::draw(held, recipe, request, colour, grey)?;
            return finish_preview(
                held,
                request,
                std::mem::take(grey),
                RenderedWindow {
                    center,
                    width,
                    function: VoiFunction::Linear,
                    auto: AutoSource::None,
                    index: None,
                    count: 0,
                },
                3,
                recipe.palette_fallback,
                recipe.icc.present(),
            );
        }
    };
    let window = select_window(held, recipe, request)?;
    let evidence = recipe.evidence?;
    let chain_started = profiling().then(Instant::now);
    // PS3.3 C.11.2.1.2.2. The whole list goes in and `VoiTransform::new`
    // selects the pair. The pair reported is the one it selected, read back
    // through `window`, so the folder viewer never re-selects.
    let voi = VoiTransform::new(
        None,
        &window.centers,
        &window.widths,
        window.selected,
        window.function,
        0.0,
        255.0,
    )
    .map_err(|_| PreviewError::Window)?;
    let (center, width, function) = voi.window().ok_or(PreviewError::Window)?;
    let chain = LutChain::new(recipe.modality.clone(), voi, recipe.photometric, evidence)
        .map_err(|_| PreviewError::Window)?;
    let chain_us = chain_started.map(|started| started.elapsed().as_micros());
    // PS3.3 C.11.2.1.2 and C.11.2.1.3. The table is `LutChain::apply` once per
    // stored integer. `display_index` quantises those entries, then each
    // sample selects one byte. Wider stored widths keep the per-sample loop.
    let table_started = profiling().then(Instant::now);
    let table = chain.stored_sample_table(held.description.bits);
    let table_us = table_started.map(|started| started.elapsed().as_micros());
    if let Some(table) = table {
        grey_from_table(held, &table, request.invert, grey, chain_us, table_us)?;
    } else {
        grey_from_samples(held, &chain, request.invert, scratch, grey, chain_us)?;
    }
    finish_preview(
        held,
        request,
        std::mem::take(grey),
        RenderedWindow {
            center,
            width,
            function,
            auto: window.auto,
            index: window.index,
            count: window_count(recipe),
        },
        1,
        false,
        false,
    )
}

struct RenderedWindow {
    center: f32,
    width: f32,
    function: VoiFunction,
    auto: AutoSource,
    index: Option<u32>,
    count: u32,
}

fn finish_preview(
    held: &HeldFrame,
    request: FrameRequest,
    grey: Vec<u8>,
    window: RenderedWindow,
    channels: u8,
    palette_fallback: bool,
    icc: bool,
) -> Result<Preview, PreviewError> {
    let facts = held.facts.clone()?;
    // F-211's choice, the one `/measure` makes for this frame, so the scale
    // bar and a length cannot disagree. A refusal shows no bar.
    let chosen = held
        .calibration
        .as_ref()
        .ok()
        .and_then(|inputs| choose_spacing(inputs).ok());
    let calibration_source = chosen.map_or("", |spacing| spacing.source);
    let calibration_mm = chosen
        .and_then(|spacing| spacing.mm)
        .map(|mm| (mm.row, mm.column));
    Ok(Preview {
        width: held.width,
        height: held.height,
        frames: held.frames,
        auto: window.auto,
        grey,
        channels,
        palette_fallback,
        icc,
        frame: request.frame,
        window_center: window.center,
        window_width: window.width,
        window_function: function_label(window.function),
        inverted: request.invert,
        facts,
        calibration_source,
        calibration_mm,
        window_index: window.index,
        window_count: window.count,
    })
}

/// The pairs the file gives this frame, or 0 when it gives none or its
/// window cannot be used.
fn window_count(recipe: &MonochromeRecipe) -> u32 {
    match &recipe.file_window {
        Ok(FileWindow::Explicit { centers, .. }) => {
            u32::try_from(centers.len()).unwrap_or(u32::MAX)
        }
        Ok(FileWindow::Auto { .. }) | Err(_) => 0,
    }
}

/// Quantise the stored-sample table once, then select a byte per sample.
fn grey_from_table(
    held: &HeldFrame,
    table: &StoredSampleTable,
    invert: bool,
    grey: &mut Vec<u8>,
    chain_us: Option<u128>,
    table_us: Option<u128>,
) -> Result<(), PreviewError> {
    let bake_started = profiling().then(Instant::now);
    let mut indexed = Vec::with_capacity(table.len());
    for display in table.entries() {
        let mut byte = display_byte(display.0)?;
        if invert {
            byte = 255 - byte;
        }
        indexed.push(byte);
    }
    let bake_us = bake_started.map(|started| started.elapsed().as_micros());
    let count = held
        .description
        .sample_count()
        .map_err(|_| PreviewError::Attribute)?;
    fit_exact(grey, count)?;
    let map_started = profiling().then(Instant::now);
    table
        .map_containers(
            held.byte_order,
            &held.samples,
            &indexed,
            grey.as_mut_slice(),
        )
        .map_err(|_| PreviewError::Decode)?;
    let map_us = map_started.map(|started| started.elapsed().as_micros());
    if profiling() {
        note_render(|phase| {
            phase.unpack_us = 0;
            phase.chain_us = chain_us.unwrap_or(0);
            phase.voi_us = table_us.unwrap_or(0);
            phase.grey_us = bake_us.unwrap_or(0);
            phase.fused_us = map_us.unwrap_or(0);
            phase.pixels = grey.len();
        });
    }
    Ok(())
}

/// Per-sample chain for a stored width the table does not cover.
fn grey_from_samples(
    held: &HeldFrame,
    chain: &LutChain,
    invert: bool,
    scratch: &mut Vec<Stored>,
    grey: &mut Vec<u8>,
    chain_us: Option<u128>,
) -> Result<(), PreviewError> {
    let count = held
        .description
        .sample_count()
        .map_err(|_| PreviewError::Attribute)?;
    if scratch.len() != count {
        scratch.resize(count, Stored(0.0));
    }
    let unpack_started = profiling().then(Instant::now);
    held.description
        .unpack(&held.samples, held.byte_order, scratch)
        .map_err(|_| PreviewError::Decode)?;
    let unpack_us = unpack_started.map(|started| started.elapsed().as_micros());
    let fused_started = profiling().then(Instant::now);
    fit_exact(grey, scratch.len())?;
    for (cell, sample) in grey.iter_mut().zip(scratch.iter().copied()) {
        let mut byte = display_byte(chain.apply(sample).0)?;
        if invert {
            byte = 255 - byte;
        }
        *cell = byte;
    }
    let fused_us = fused_started.map(|started| started.elapsed().as_micros());
    if profiling() {
        note_render(|phase| {
            phase.unpack_us = unpack_us.unwrap_or(0);
            phase.chain_us = chain_us.unwrap_or(0);
            phase.voi_us = 0;
            phase.grey_us = 0;
            phase.fused_us = fused_us.unwrap_or(0);
            phase.pixels = scratch.len();
        });
    }
    Ok(())
}

/// Grow `buf` to `len` bytes without a doubled spare capacity.
fn fit_exact(buf: &mut Vec<u8>, len: usize) -> Result<(), PreviewError> {
    if buf.capacity() < len {
        let extra = len.checked_sub(buf.len()).ok_or(PreviewError::Decode)?;
        buf.reserve_exact(extra);
    }
    buf.resize(len, 0);
    Ok(())
}

/// `display_index` into the 8-bit preview. PS3.3 C.11.2.1.2's output byte.
fn display_byte(display: f32) -> Result<u8, PreviewError> {
    u8::try_from(display_index(display, 255)).map_err(|_| PreviewError::Window)
}

/// The window lists one request draws, and which pair of them.
struct ChosenWindow {
    /// Every Window Center the request offers `VoiTransform::new`.
    centers: Vec<f32>,
    /// Every Window Width, paired with [`Self::centers`] by position.
    widths: Vec<f32>,
    /// The pair `VoiTransform::new` selects.
    selected: usize,
    function: VoiFunction,
    auto: AutoSource,
    /// The file window pair drawn, or `None` for a preset, a numeric window
    /// or an automatic window.
    index: Option<u32>,
}

impl ChosenWindow {
    fn one(center: f32, width: f32, function: VoiFunction, auto: AutoSource) -> Self {
        Self {
            centers: vec![center],
            widths: vec![width],
            selected: 0,
            function,
            auto,
            index: None,
        }
    }
}

fn select_window(
    held: &HeldFrame,
    recipe: &MonochromeRecipe,
    request: FrameRequest,
) -> Result<ChosenWindow, PreviewError> {
    if let Some(preset) = request.window_preset {
        // The LUT refusal comes first, so a preset on a file with a VOI LUT
        // Sequence is the window error whatever its Modality.
        if voi_present(recipe)? {
            return Err(PreviewError::Window);
        }
        if !recipe.ct {
            return Err(PreviewError::Modality);
        }
        return Ok(ChosenWindow::one(
            preset.center(),
            preset.width(),
            VoiFunction::Linear,
            AutoSource::None,
        ));
    }
    if request.window_center.is_some() && request.window_width.is_some() {
        let center = request.window_center.ok_or(PreviewError::Window)?;
        let width = request.window_width.ok_or(PreviewError::Window)?;
        if !center.is_finite() || !width.is_finite() {
            return Err(PreviewError::Window);
        }
        let function = request.window_function.unwrap_or(VoiFunction::Linear);
        return Ok(ChosenWindow::one(center, width, function, AutoSource::None));
    }
    if voi_present(recipe)? {
        return Err(PreviewError::Window);
    }
    match recipe.file_window.as_ref().map_err(|error| *error)? {
        FileWindow::Auto { .. } => {
            // An index names a stored pair, and this file stores none.
            if request.window_index.is_some() {
                return Err(PreviewError::Window);
            }
            // F-227. The series window is the one the page was given for this
            // image's series, used as sent. Without it the image draws at its
            // own automatic window, and the reply says which.
            let (center, width, auto) = match request.series_window {
                Some((center, width)) => (center, width, AutoSource::Series),
                None => {
                    let own = held.auto?;
                    (own.center, own.width, AutoSource::Image)
                }
            };
            // A function sent alone replaces the LINEAR_EXACT the automatic
            // window is checked under. The default holds when it names none.
            let function = request.window_function.unwrap_or(VoiFunction::LinearExact);
            Ok(ChosenWindow::one(center, width, function, auto))
        }
        FileWindow::Explicit {
            centers,
            widths,
            function,
        } => {
            let mut function = *function;
            if let Some(choice) = request.window_function {
                function = choice;
            }
            // With no index the default is the first slot holding both a
            // centre and a width, so a file whose first value is empty still
            // opens drawn. An explicit index is taken as sent.
            let usable = |at: usize| {
                matches!(
                    (centers.get(at), widths.get(at)),
                    (Some(Some(_)), Some(Some(_)))
                )
            };
            let selected = match request.window_index {
                Some(index) => usize::try_from(index).map_err(|_| PreviewError::Window)?,
                None => (0..centers.len()).find(|at| usable(*at)).unwrap_or(0),
            };
            let index = u32::try_from(selected).map_err(|_| PreviewError::Window)?;
            // An empty slot has no number to draw. It is refused here rather
            // than skipped, so the pairs after it keep their positions.
            if !usable(selected) {
                return Err(PreviewError::Window);
            }
            // An empty slot that is not the selected one is never read by
            // `VoiTransform::new`, which validates only the selected pair.
            let fill = |slots: &[Option<f32>]| -> Vec<f32> {
                slots.iter().map(|slot| slot.unwrap_or(f32::NAN)).collect()
            };
            Ok(ChosenWindow {
                centers: fill(centers),
                widths: fill(widths),
                selected,
                function,
                auto: AutoSource::None,
                index: Some(index),
            })
        }
    }
}

fn voi_present(recipe: &MonochromeRecipe) -> Result<bool, PreviewError> {
    recipe.voi_lut
}

fn part10_error(_error: ParseError) -> PreviewError {
    // `ParseError` is non-exhaustive. Every refusal is the same preview answer.
    PreviewError::NotPart10
}

fn photometric(object: &DefaultDicomObject) -> Result<PhotometricInterpretation, PreviewError> {
    match required_text(object, PHOTOMETRIC)?.as_str() {
        "MONOCHROME1" => Ok(PhotometricInterpretation::Monochrome1),
        "MONOCHROME2" => Ok(PhotometricInterpretation::Monochrome2),
        "RGB" => Ok(PhotometricInterpretation::Rgb),
        "YBR_FULL" => Ok(PhotometricInterpretation::YbrFull),
        "YBR_FULL_422" => Ok(PhotometricInterpretation::YbrFull422),
        "YBR_PARTIAL_422" => Ok(PhotometricInterpretation::YbrPartial422),
        "YBR_ICT" => Ok(PhotometricInterpretation::YbrIct),
        "YBR_RCT" => Ok(PhotometricInterpretation::YbrRct),
        "PALETTE COLOR" => Ok(PhotometricInterpretation::PaletteColor),
        _ => Err(PreviewError::ColourUnavailable(
            ColourUnavailableReason::Space,
        )),
    }
}

fn photometric_name(photometric: PhotometricInterpretation) -> &'static str {
    match photometric {
        PhotometricInterpretation::Monochrome1 => "MONOCHROME1",
        PhotometricInterpretation::Monochrome2 => "MONOCHROME2",
        PhotometricInterpretation::PaletteColor => "PALETTE COLOR",
        PhotometricInterpretation::Rgb => "RGB",
        PhotometricInterpretation::YbrFull => "YBR_FULL",
        PhotometricInterpretation::YbrFull422 => "YBR_FULL_422",
        PhotometricInterpretation::YbrPartial422 => "YBR_PARTIAL_422",
        PhotometricInterpretation::YbrIct => "YBR_ICT",
        PhotometricInterpretation::YbrRct => "YBR_RCT",
        // XYB is not an admitted viewer space. Keep the match exhaustive.
        PhotometricInterpretation::Xyb => "XYB",
    }
}

fn pixel_representation(
    object: &DefaultDicomObject,
) -> Result<ocelli_pixel::PixelRepresentation, PreviewError> {
    match required_u16(object, PIXEL_REPRESENTATION)? {
        0 => Ok(ocelli_pixel::PixelRepresentation::Unsigned),
        1 => Ok(ocelli_pixel::PixelRepresentation::Signed),
        _ => Err(PreviewError::Attribute),
    }
}

fn frame_count(object: &DefaultDicomObject) -> Result<u32, PreviewError> {
    let Some(raw) = optional_text(object, NUMBER_OF_FRAMES)? else {
        return Ok(1);
    };
    let value = parse_decimal_string(&raw).ok_or(PreviewError::Attribute)?;
    whole_u32(value).ok_or(PreviewError::Attribute)
}

/// A finite whole number in `1..=u32::MAX`, compared by bits so a fraction is refused.
fn whole_u32(value: f64) -> Option<u32> {
    if !value.is_finite() || value < 1.0 {
        return None;
    }
    if value.fract().to_bits() != 0.0_f64.to_bits() {
        return None;
    }
    let mut lo = 1_u32;
    let mut hi = u32::MAX;
    while hi - lo > 1 {
        let mid = lo + (hi - lo) / 2;
        if f64::from(mid) < value {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    if f64::from(lo).to_bits() == value.to_bits() {
        Some(lo)
    } else if f64::from(hi).to_bits() == value.to_bits() {
        Some(hi)
    } else {
        None
    }
}

fn pixel_vr(object: &DefaultDicomObject) -> Result<PixelDataVr, PreviewError> {
    let element = object
        .element(PIXEL_DATA)
        .map_err(|_| PreviewError::Attribute)?;
    match element.vr() {
        VR::OB => Ok(PixelDataVr::Ob),
        VR::OW => Ok(PixelDataVr::Ow),
        _ => Err(PreviewError::Attribute),
    }
}

fn native_dispatch(path: DispatchPath) -> bool {
    !matches!(path, DispatchPath::Encapsulated)
}

/// The borrowed pixel value is the whole element, every frame, already in
/// transfer-syntax byte order. A parsed copy of those bytes is not required.
fn borrowed_pixels_fit(object: &DefaultDicomObject, pixels: &[u8]) -> bool {
    let Ok((parts, _)) = frame_parts(object, 0, PixelDataEncoding::Native) else {
        return false;
    };
    let Ok(frames) = usize::try_from(parts.frames) else {
        return false;
    };
    parts
        .expected
        .checked_mul(frames)
        .is_some_and(|all| all > 0 && pixels.len() == all)
}

#[derive(Clone, Copy)]
struct DecodeFacts {
    len: usize,
    encoding: PixelDataEncoding,
    photometric: DecodedPhotometric,
    layout: DecodedLayout,
}

fn decode_facts(
    parsed: &ocelli_dicom::ParsedDicom,
    desc: &FrameDesc,
) -> Result<DecodeFacts, PreviewError> {
    let path = parsed.transfer_syntax().dispatch_path();
    if path != DispatchPath::Encapsulated {
        return Ok(DecodeFacts {
            len: native_decoder(path)?.decoded_output_len(desc),
            encoding: PixelDataEncoding::Native,
            photometric: DecodedPhotometric::Preserved,
            layout: DecodedLayout::Preserved,
        });
    }
    let registry = codecs()?;
    let uid = parsed.transfer_syntax().uid();
    Ok(DecodeFacts {
        len: registry
            .decoded_output_len(uid, desc)
            .map_err(|_| PreviewError::Decode)?,
        encoding: PixelDataEncoding::Encapsulated,
        photometric: match registry
            .decode_photometric_interpretation(uid, desc)
            .map_err(|_| PreviewError::Decode)?
        {
            DecodePhotometricInterpretation::Preserved => DecodedPhotometric::Preserved,
            DecodePhotometricInterpretation::Rgb => DecodedPhotometric::Rgb,
        },
        layout: match registry
            .decode_sample_layout(uid, desc)
            .map_err(|_| PreviewError::Decode)?
        {
            DecodeSampleLayout::Preserved => DecodedLayout::Preserved,
            DecodeSampleLayout::Interleaved => DecodedLayout::Interleaved,
        },
    })
}

fn decode_frame(
    parsed: &ocelli_dicom::ParsedDicom,
    desc: &FrameDesc,
    frames: u32,
    frame_index: usize,
    pixels: Option<&[u8]>,
    native: &mut Vec<u8>,
    decoded: &mut [u8],
) -> Result<(), PreviewError> {
    if decoded.len() != decode_facts(parsed, desc)?.len {
        return Err(PreviewError::Decode);
    }
    let object = parsed.object();
    let frame_count = usize::try_from(frames).map_err(|_| PreviewError::Attribute)?;
    if let Some(pixels) = pixels {
        let decoder = native_decoder(parsed.transfer_syntax().dispatch_path())?;
        let index = decoder
            .index_native_value(pixels, frame_count, desc)
            .map_err(|_| PreviewError::Decode)?;
        index
            .decode_frame(frame_index, decoded)
            .map_err(|_| PreviewError::Decode)?;
        note_borrowed_decode();
        return Ok(());
    }
    let element = object
        .element(PIXEL_DATA)
        .map_err(|_| PreviewError::Attribute)?;
    match parsed.transfer_syntax().dispatch_path() {
        DispatchPath::Encapsulated => {
            decode_encapsulated(parsed, desc, frame_count, frame_index, decoded)?;
        }
        path => {
            let borrowed = native_byte_slice(element, path, native)?;
            let decoder = native_decoder(path)?;
            let index = decoder
                .index_native_value(borrowed, frame_count, desc)
                .map_err(|_| PreviewError::Decode)?;
            index
                .decode_frame(frame_index, decoded)
                .map_err(|_| PreviewError::Decode)?;
        }
    }
    Ok(())
}

fn native_order_matches(path: DispatchPath) -> bool {
    match path {
        DispatchPath::ExplicitVrBigEndian => cfg!(target_endian = "big"),
        DispatchPath::ImplicitVrLittleEndian
        | DispatchPath::ExplicitVrLittleEndian
        | DispatchPath::DeflatedExplicitVrLittleEndian => cfg!(target_endian = "little"),
        DispatchPath::Encapsulated => false,
    }
}

fn borrowed_native_bytes(element: &InMemElement<StandardDataDictionary>) -> Option<&[u8]> {
    let DicomValue::Primitive(value) = element.value() else {
        return None;
    };
    match value.to_bytes() {
        std::borrow::Cow::Borrowed(bytes) => Some(bytes),
        std::borrow::Cow::Owned(_) => None,
    }
}

fn native_decoder(path: DispatchPath) -> Result<RawDecoder, PreviewError> {
    match path {
        DispatchPath::ImplicitVrLittleEndian => Ok(RawDecoder::implicit_vr_little_endian()),
        DispatchPath::ExplicitVrLittleEndian | DispatchPath::DeflatedExplicitVrLittleEndian => {
            Ok(RawDecoder::explicit_vr_little_endian())
        }
        DispatchPath::ExplicitVrBigEndian => Ok(RawDecoder::big_endian()),
        DispatchPath::Encapsulated => Err(PreviewError::Decode),
    }
}

fn decode_encapsulated(
    parsed: &ocelli_dicom::ParsedDicom,
    desc: &FrameDesc,
    frame_count: usize,
    frame_index: usize,
    decoded: &mut [u8],
) -> Result<(), PreviewError> {
    // F-244. One constructor chooses the Extended or Basic Offset Table and
    // the transfer syntax's frame delimiter, PS3.5 A.4 and PS3.3 C.7.6.3.1.8.
    let index = EncapsulatedFrameIndex::from_parsed(parsed, frame_count)
        .map_err(|_| PreviewError::Decode)?;
    let parts = index
        .frame_fragments(frame_index)
        .map_err(|_| PreviewError::Decode)?;
    let mut encoded = Vec::new();
    for part in parts {
        encoded.extend_from_slice(part);
    }
    let registry = codecs()?;
    let uid = parsed.transfer_syntax().uid();
    let produced = registry
        .decode_photometric_interpretation(uid, desc)
        .map_err(|_| PreviewError::Decode)?;
    let _ = produced;
    registry.decode(uid, &encoded, desc, decoded).map_err(|_| {
        if matches!(
            desc.photometric_interpretation(),
            "MONOCHROME1" | "MONOCHROME2"
        ) {
            PreviewError::Decode
        } else {
            PreviewError::ColourUnavailable(ColourUnavailableReason::Codestream)
        }
    })
}

fn native_byte_slice<'a>(
    element: &'a InMemElement<StandardDataDictionary>,
    path: DispatchPath,
    native: &'a mut Vec<u8>,
) -> Result<&'a [u8], PreviewError> {
    match element.value() {
        DicomValue::Primitive(PrimitiveValue::U8(bytes)) => Ok(bytes.as_slice()),
        DicomValue::Primitive(PrimitiveValue::U16(words)) => {
            // A second buffer of the frame stays resident after the copy is
            // dropped. When the host endian matches the transfer syntax, the
            // parsed words are already the bytes the decoder reads.
            if native_order_matches(path)
                && let Some(borrowed) = borrowed_native_bytes(element)
            {
                return Ok(borrowed);
            }
            fill_native_words(words, path, native)?;
            Ok(native.as_slice())
        }
        DicomValue::Primitive(_) | DicomValue::Sequence(_) | DicomValue::PixelSequence(_) => {
            Err(PreviewError::Decode)
        }
    }
}

/// Copy `U16` pixels into `native` at exactly two bytes per word.
///
/// The buffer stays with the caller. Growing a fresh vector by doubling would
/// leave the abandoned capacities resident after the copy is dropped.
fn fill_native_words(
    words: &[u16],
    path: DispatchPath,
    native: &mut Vec<u8>,
) -> Result<(), PreviewError> {
    let bytes_len = words.len().checked_mul(2).ok_or(PreviewError::Decode)?;
    native.clear();
    if native.capacity() < bytes_len {
        native.reserve_exact(bytes_len);
    }
    for word in words {
        let encoded = match path {
            DispatchPath::ExplicitVrBigEndian => word.to_be_bytes(),
            DispatchPath::ImplicitVrLittleEndian
            | DispatchPath::ExplicitVrLittleEndian
            | DispatchPath::DeflatedExplicitVrLittleEndian
            | DispatchPath::Encapsulated => word.to_le_bytes(),
        };
        native.extend_from_slice(&encoded);
    }
    Ok(())
}

fn codecs() -> Result<Registry, PreviewError> {
    let mut registry = Registry::new();
    register_native_and_rle_decoders(&mut registry).map_err(|_| PreviewError::Decode)?;
    register_jpeg_decoders(&mut registry).map_err(|_| PreviewError::Decode)?;
    register_jpeg2000_decoders(&mut registry).map_err(|_| PreviewError::Decode)?;
    register_jpegls_decoders(&mut registry).map_err(|_| PreviewError::Decode)?;
    register_htj2k_decoders(&mut registry).map_err(|_| PreviewError::Decode)?;
    register_jpegxl_decoders(&mut registry).map_err(|_| PreviewError::Decode)?;
    Ok(registry)
}

fn project_metadata(object: &DefaultDicomObject) -> Result<MetadataSet, PreviewError> {
    #[cfg(test)]
    note_inspect_work(optional_text(object, SOP_INSTANCE).ok().flatten(), true);
    project_elements(object.iter())
}

/// Whole-object metadata projections and plane-list builds, by the SOP
/// Instance UID of the object they were made for, on every thread. A test
/// gives its file a UID of its own, so tests running at once and a decode
/// worker's thread do not hide or add work.
#[cfg(test)]
static INSPECT_WORK: LazyLock<Mutex<std::collections::HashMap<String, (u64, u64)>>> =
    LazyLock::new(|| Mutex::new(std::collections::HashMap::new()));

#[cfg(test)]
fn note_inspect_work(uid: Option<String>, projection: bool) {
    let Some(uid) = uid else {
        return;
    };
    let mut work = INSPECT_WORK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let entry = work.entry(uid).or_insert((0, 0));
    if projection {
        entry.0 = entry.0.saturating_add(1);
    } else {
        entry.1 = entry.1.saturating_add(1);
    }
}

/// The projections and plane-list builds made for the object whose SOP
/// Instance UID is `uid`, from any thread.
#[cfg(test)]
pub(crate) fn inspect_work(uid: &str) -> (u64, u64) {
    INSPECT_WORK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .get(uid)
        .copied()
        .unwrap_or((0, 0))
}

fn project_item(
    object: &InMemDicomObject<StandardDataDictionary>,
) -> Result<MetadataSet, PreviewError> {
    project_elements(object.iter())
}

fn project_elements<'a>(
    elements: impl Iterator<Item = &'a InMemElement<StandardDataDictionary>>,
) -> Result<MetadataSet, PreviewError> {
    let mut metadata = MetadataSet::new();
    for element in elements {
        let tag = element.header().tag;
        if tag == PIXEL_DATA {
            continue;
        }
        let projected = match element.value() {
            DicomValue::Primitive(value) => MetadataElement::from_primitive(element.vr(), value),
            DicomValue::Sequence(sequence) => {
                let mut items = Vec::new();
                for item in sequence.items() {
                    items.push(project_item(item)?);
                }
                MetadataElement::sequence(items)
            }
            DicomValue::PixelSequence(_) => continue,
        };
        metadata
            .insert(tag, projected)
            .map_err(|_| PreviewError::Attribute)?;
    }
    Ok(metadata)
}

fn group_has(
    multi: &MultiframeMetadata<'_>,
    frame: usize,
    group: Tag,
    attribute: Tag,
) -> Result<bool, PreviewError> {
    Ok(multi
        .resolve(frame, group, attribute, TopLevelFallback::Allowed)
        .map_err(|_| PreviewError::Attribute)?
        .is_some())
}

fn resolved_decimals(
    multi: &MultiframeMetadata<'_>,
    frame: usize,
    group: Tag,
    attribute: Tag,
) -> Result<Vec<f32>, PreviewError> {
    let found = multi
        .resolve(frame, group, attribute, TopLevelFallback::Allowed)
        .map_err(|_| PreviewError::Attribute)?;
    let Some(found) = found else {
        return Ok(Vec::new());
    };
    // A zero-length element is present and carries no number. It means the
    // same as an absent attribute. Any other non-text value is refused.
    if matches!(found.element().value(), MetadataValue::Empty) {
        return Ok(Vec::new());
    }
    let Some(parts) = found.element().semantic_text() else {
        return Err(PreviewError::Attribute);
    };
    let mut values = Vec::new();
    for part in parts {
        if part.is_empty() {
            continue;
        }
        let parsed = parse_decimal_string(part).ok_or(PreviewError::Attribute)?;
        let narrow = f32_from_f64(parsed);
        if !narrow.is_finite() {
            return Err(PreviewError::Attribute);
        }
        values.push(narrow);
    }
    Ok(values)
}

/// Every stored value of a multi-valued attribute, empty ones included.
///
/// PS3.3 C.11.2.1.2.2 pairs Window Center, Window Width and Window Center &
/// Width Explanation by position, so a reader that dropped an empty value
/// would move every later pair onto the wrong label. Absent and zero-length
/// give no slot. A value that is not text is refused.
fn resolved_slots(
    multi: &MultiframeMetadata<'_>,
    frame: usize,
    group: Tag,
    attribute: Tag,
) -> Result<Vec<String>, PreviewError> {
    let found = multi
        .resolve(frame, group, attribute, TopLevelFallback::Allowed)
        .map_err(|_| PreviewError::Attribute)?;
    let Some(found) = found else {
        return Ok(Vec::new());
    };
    if matches!(found.element().value(), MetadataValue::Empty) {
        return Ok(Vec::new());
    }
    let Some(parts) = found.element().semantic_text() else {
        return Err(PreviewError::Attribute);
    };
    Ok(parts.into_iter().map(str::to_owned).collect())
}

/// [`resolved_slots`] read as decimals. An empty value is `None` in its
/// slot. Any other value that is not a finite decimal is refused.
fn decimal_slots(
    multi: &MultiframeMetadata<'_>,
    frame: usize,
    group: Tag,
    attribute: Tag,
) -> Result<Vec<Option<f32>>, PreviewError> {
    let mut slots = Vec::new();
    for part in resolved_slots(multi, frame, group, attribute)? {
        if part.is_empty() {
            slots.push(None);
            continue;
        }
        let parsed = parse_decimal_string(&part).ok_or(PreviewError::Attribute)?;
        let narrow = f32_from_f64(parsed);
        if !narrow.is_finite() {
            return Err(PreviewError::Attribute);
        }
        slots.push(Some(narrow));
    }
    Ok(slots)
}

pub(crate) fn resolved_text(
    multi: &MultiframeMetadata<'_>,
    frame: usize,
    group: Tag,
    attribute: Tag,
) -> Result<Option<String>, PreviewError> {
    let found = multi
        .resolve(frame, group, attribute, TopLevelFallback::Allowed)
        .map_err(|_| PreviewError::Attribute)?;
    let Some(found) = found else {
        return Ok(None);
    };
    // A zero-length element is present and carries no token. It means the
    // same as an absent attribute. Any other non-text value is refused.
    if matches!(found.element().value(), MetadataValue::Empty) {
        return Ok(None);
    }
    let Some(parts) = found.element().semantic_text() else {
        return Err(PreviewError::Attribute);
    };
    Ok(parts
        .into_iter()
        .find(|part| !part.is_empty())
        .map(str::to_owned))
}

fn function_label(function: VoiFunction) -> &'static str {
    match function {
        VoiFunction::Linear => "LINEAR",
        VoiFunction::LinearExact => "LINEAR_EXACT",
        VoiFunction::Sigmoid => "SIGMOID",
    }
}

fn rescale(multi: &MultiframeMetadata<'_>, frame: usize) -> Result<(f32, f32), PreviewError> {
    let slope = resolved_decimals(multi, frame, PIXEL_VALUE_TRANSFORMATION, RESCALE_SLOPE)?;
    let intercept = resolved_decimals(multi, frame, PIXEL_VALUE_TRANSFORMATION, RESCALE_INTERCEPT)?;
    match (slope.first().copied(), intercept.first().copied()) {
        (None, None) => Ok((1.0, 0.0)),
        (Some(slope), Some(intercept)) => {
            if slope.is_finite() && intercept.is_finite() {
                Ok((slope, intercept))
            } else {
                Err(PreviewError::Window)
            }
        }
        _ => Err(PreviewError::Window),
    }
}

fn voi_function(multi: &MultiframeMetadata<'_>, frame: usize) -> Result<VoiFunction, PreviewError> {
    match resolved_text(multi, frame, FRAME_VOI_LUT_SEQUENCE, VOI_FUNCTION)? {
        None => Ok(VoiFunction::Linear),
        Some(text) if text == "LINEAR" => Ok(VoiFunction::Linear),
        Some(text) if text == "LINEAR_EXACT" => Ok(VoiFunction::LinearExact),
        Some(text) if text == "SIGMOID" => Ok(VoiFunction::Sigmoid),
        Some(_) => Err(PreviewError::Window),
    }
}

fn presentation(object: &DefaultDicomObject) -> Result<PresentationLutEvidence, PreviewError> {
    if object.element(PRESENTATION_SEQUENCE).is_ok() {
        return Ok(PresentationLutEvidence::Sequence);
    }
    match optional_text(object, PRESENTATION_SHAPE)? {
        None => Ok(PresentationLutEvidence::Absent),
        Some(text) if text == "IDENTITY" => Ok(PresentationLutEvidence::Shape(
            PresentationLutShape::Identity,
        )),
        Some(text) if text == "INVERSE" => Ok(PresentationLutEvidence::Shape(
            PresentationLutShape::Inverse,
        )),
        Some(_) => Err(PreviewError::Window),
    }
}

fn required_u16(object: &DefaultDicomObject, tag: Tag) -> Result<u16, PreviewError> {
    object
        .element(tag)
        .map_err(|_| PreviewError::Attribute)?
        .to_int::<u16>()
        .map_err(|_| PreviewError::Attribute)
}

fn required_text(object: &DefaultDicomObject, tag: Tag) -> Result<String, PreviewError> {
    optional_text(object, tag)?.ok_or(PreviewError::Attribute)
}

pub(crate) fn optional_text(
    object: &DefaultDicomObject,
    tag: Tag,
) -> Result<Option<String>, PreviewError> {
    let Ok(element) = object.element(tag) else {
        return Ok(None);
    };
    let raw = element.to_str().map_err(|_| PreviewError::Attribute)?;
    let text = raw.trim_matches(' ').trim_end_matches('\0');
    if text.is_empty() {
        Ok(None)
    } else {
        Ok(Some(text.to_owned()))
    }
}

/// Read the VOI LUT Function token used by the page.
///
/// # Errors
///
/// Returns [`PreviewError::Window`] when the token is not one of the three
/// functions in PS3.3 C.11.2.
pub fn window_function_from_token(text: &str) -> Result<VoiFunction, PreviewError> {
    match text {
        "LINEAR" => Ok(VoiFunction::Linear),
        "LINEAR_EXACT" => Ok(VoiFunction::LinearExact),
        "SIGMOID" => Ok(VoiFunction::Sigmoid),
        _ => Err(PreviewError::Window),
    }
}

/// Read the preset token used by the page.
///
/// The names are the same three strings as `PRESET_BYTES` in
/// `ocelli-viewport`. The centre and width come from [`WindowPreset`].
///
/// # Errors
///
/// Returns [`PreviewError::Window`] when the token is not one of the three.
pub fn window_preset_from_token(text: &str) -> Result<WindowPreset, PreviewError> {
    match text {
        "soft_tissue" => Ok(WindowPreset::SoftTissue),
        "lung" => Ok(WindowPreset::Lung),
        "bone" => Ok(WindowPreset::Bone),
        _ => Err(PreviewError::Window),
    }
}

impl FrameFacts {
    fn empty() -> Self {
        Self {
            row_spacing: None,
            column_spacing: None,
            spacing_source: "",
            edge_left: String::new(),
            edge_right: String::new(),
            edge_top: String::new(),
            edge_bottom: String::new(),
            edge_source: "",
            corners: None,
        }
    }
}

fn frame_facts(
    metadata: &MetadataSet,
    multi: &MultiframeMetadata<'_>,
    frame: usize,
) -> Result<FrameFacts, PreviewError> {
    if frame >= multi.frame_count() {
        return Ok(FrameFacts::empty());
    }
    let pixel = spacing_pair(multi, frame, PIXEL_MEASURES_SEQUENCE, PIXEL_SPACING)?;
    let imager = top_level_pair(metadata, IMAGER_PIXEL_SPACING);
    // A Pixel Spacing that is present but not a pair stays empty. It does not
    // fall through to Imager Pixel Spacing.
    let (row_spacing, column_spacing, spacing_source) = match pixel {
        SpacingRead::Pair(row, column) => (Some(row), Some(column), "PixelSpacing"),
        SpacingRead::Unusable => (None, None, ""),
        SpacingRead::Absent => {
            if let Some((row, column)) = imager {
                (Some(row), Some(column), "ImagerPixelSpacing")
            } else {
                (None, None, "")
            }
        }
    };
    let (letters, edge_source) = edge_letters(metadata, multi, frame)?;
    let [edge_left, edge_right, edge_top, edge_bottom] = letters;
    // F-267. The acquisition corners of the shown frame, under the
    // precedence rule: the top level first, then the frame's own group and
    // the shared group through one `resolve` call.
    let slice_thickness = corner_token(
        metadata,
        multi,
        frame,
        SLICE_THICKNESS,
        PIXEL_MEASURES_SEQUENCE,
        SLICE_THICKNESS,
        VR::DS,
    );
    let kvp = corner_token(
        metadata,
        multi,
        frame,
        KVP,
        CT_XRAY_DETAILS_SEQUENCE,
        KVP,
        VR::DS,
    );
    let repetition_time = corner_token(
        metadata,
        multi,
        frame,
        REPETITION_TIME,
        MR_TIMING_SEQUENCE,
        REPETITION_TIME,
        VR::DS,
    );
    let echo_time = corner_token(
        metadata,
        multi,
        frame,
        ECHO_TIME,
        MR_ECHO_SEQUENCE,
        EFFECTIVE_ECHO_TIME,
        VR::FD,
    );
    let (exposure_mas, exposure_source) = corner_exposure_mas(metadata, multi, frame);
    let corners = if slice_thickness.is_some()
        || kvp.is_some()
        || exposure_mas.is_some()
        || repetition_time.is_some()
        || echo_time.is_some()
    {
        Some(Box::new(CornerTokens {
            slice_thickness,
            kvp,
            exposure_mas,
            exposure_source,
            repetition_time,
            echo_time,
        }))
    } else {
        None
    };
    Ok(FrameFacts {
        row_spacing,
        column_spacing,
        spacing_source,
        edge_left,
        edge_right,
        edge_top,
        edge_bottom,
        edge_source,
        corners,
    })
}

/// Left, right, top and bottom letters and the attribute they came from.
///
/// Image Orientation (Patient) decides whenever it is present, and one that
/// cannot be used gives no letters rather than reading Patient Orientation.
/// Only an absent or zero-length one reads Patient Orientation, from the top
/// level. PS3.3 C.7.6.1.1.1: a `QUADRUPED` Anatomical Orientation Type uses
/// letters that overlap the human set, so it gives no letters at all.
fn edge_letters(
    metadata: &MetadataSet,
    multi: &MultiframeMetadata<'_>,
    frame: usize,
) -> Result<([String; 4], &'static str), PreviewError> {
    let none = || {
        (
            [String::new(), String::new(), String::new(), String::new()],
            "",
        )
    };
    let quadruped = metadata
        .get(ANATOMICAL_ORIENTATION_TYPE)
        .and_then(MetadataElement::semantic_text)
        .is_some_and(|parts| parts.first().is_some_and(|text| *text == "QUADRUPED"));
    if quadruped {
        return Ok(none());
    }
    if let Some(parts) = token_list(multi, frame, PLANE_ORIENTATION_SEQUENCE, IMAGE_ORIENTATION)? {
        return Ok(match orientation_letters(&parts) {
            Some(letters) => (letters, "ImageOrientationPatient"),
            None => none(),
        });
    }
    if image_orientation_present(multi, frame)? {
        return Ok(none());
    }
    let Some(element) = metadata.get(PATIENT_ORIENTATION) else {
        return Ok(none());
    };
    let Some(parts) = element.semantic_text() else {
        return Ok(none());
    };
    let values: Vec<&str> = parts.into_iter().filter(|part| !part.is_empty()).collect();
    Ok(match patient_orientation_letters(&values) {
        Some(letters) => (letters, "PatientOrientation"),
        None => none(),
    })
}

/// Image Orientation (Patient) is present with a value that is not zero length.
///
/// It is asked only after [`token_list`] found no text, so a true answer is a
/// value that is not text. [`token_list`] reports that the same as an absent
/// attribute, and here it must still decide.
fn image_orientation_present(
    multi: &MultiframeMetadata<'_>,
    frame: usize,
) -> Result<bool, PreviewError> {
    let found = multi
        .resolve(
            frame,
            PLANE_ORIENTATION_SEQUENCE,
            IMAGE_ORIENTATION,
            TopLevelFallback::Allowed,
        )
        .map_err(|_| PreviewError::Attribute)?;
    Ok(found.is_some_and(|found| !matches!(found.element().value(), MetadataValue::Empty)))
}

enum SpacingRead {
    Absent,
    Unusable,
    Pair(String, String),
}

fn spacing_pair(
    multi: &MultiframeMetadata<'_>,
    frame: usize,
    group: Tag,
    attribute: Tag,
) -> Result<SpacingRead, PreviewError> {
    let found = multi
        .resolve(frame, group, attribute, TopLevelFallback::Allowed)
        .map_err(|_| PreviewError::Attribute)?;
    let Some(found) = found else {
        return Ok(SpacingRead::Absent);
    };
    if matches!(found.element().value(), MetadataValue::Empty) {
        return Ok(SpacingRead::Unusable);
    }
    let Some(parts) = found.element().semantic_text() else {
        return Ok(SpacingRead::Unusable);
    };
    let parts: Vec<&str> = parts.into_iter().filter(|part| !part.is_empty()).collect();
    let Some(row) = parts.first() else {
        return Ok(SpacingRead::Unusable);
    };
    let Some(column) = parts.get(1) else {
        return Ok(SpacingRead::Unusable);
    };
    if parts.len() != 2 {
        return Ok(SpacingRead::Unusable);
    }
    match (clean_decimal(row), clean_decimal(column)) {
        (Some(row), Some(column)) => Ok(SpacingRead::Pair(row, column)),
        _ => Ok(SpacingRead::Unusable),
    }
}

fn top_level_pair(metadata: &MetadataSet, tag: Tag) -> Option<(String, String)> {
    let element = metadata.get(tag)?;
    let parts = element.semantic_text()?;
    let row = parts.first()?;
    let column = parts.get(1)?;
    if parts.len() != 2 {
        return None;
    }
    Some((clean_decimal(row)?, clean_decimal(column)?))
}

/// One top-level corner read.
enum TopLevelRead {
    /// No element, a present zero-length one, or text whose one component
    /// trims away. The groups decide.
    Absent,
    /// A token from the one-component trim rule.
    Token(String),
    /// Present, but no token came from it. The read stops here.
    Stop,
}

/// The top level of one corner field, from the metadata projection.
///
/// PS3.3 2026c C.7.6.2, C.8.2.1 and C.8.3.1 carry the classic homes. An
/// absent element, a present empty one, and a text element whose one
/// component is empty after the trim are absent and fall to the groups. A
/// text element yields a token only when it has exactly one component,
/// with no empty-part filter, after every trailing space and NUL is
/// trimmed, through `clean_decimal`. A multi-valued element and a
/// non-text element yield no token and stop the read, the measurement rule
/// `spacing_pair` keeps: a present attribute that is not one usable value
/// refuses rather than reading the next source. There is no VR check at
/// the top level, the projected text rule is the only gate, the same gate
/// `corner_text` applies for the scan.
fn top_level_read(metadata: &MetadataSet, tag: Tag) -> TopLevelRead {
    let Some(element) = metadata.get(tag) else {
        return TopLevelRead::Absent;
    };
    if matches!(element.value(), MetadataValue::Empty) {
        return TopLevelRead::Absent;
    }
    let Some(parts) = element.semantic_text() else {
        return TopLevelRead::Stop;
    };
    if parts.len() != 1 {
        return TopLevelRead::Stop;
    }
    let Some(text) = parts
        .first()
        .map(|text| text.trim_end_matches([' ', '\u{0}']))
    else {
        return TopLevelRead::Stop;
    };
    if text.is_empty() {
        return TopLevelRead::Absent;
    }
    match clean_decimal(text) {
        Some(token) => TopLevelRead::Token(token),
        None => TopLevelRead::Stop,
    }
}

/// One link of the mAs chain, the same read without the stop.
///
/// `exposure_mas` falls from an unusable Exposure to a usable Exposure in
/// µAs, so every kind of unusable top-level value falls through here
/// instead of stopping, a multi-value and a non-text value alike.
fn chain_text(metadata: &MetadataSet, tag: Tag) -> Option<String> {
    let element = metadata.get(tag)?;
    if matches!(element.value(), MetadataValue::Empty) {
        return None;
    }
    let parts = element.semantic_text()?;
    if parts.len() != 1 {
        return None;
    }
    let text = parts.first()?.trim_end_matches([' ', '\u{0}']);
    if text.is_empty() {
        None
    } else {
        Some(text.to_owned())
    }
}

/// One resolved group element's token, the PS3.6 2026c Table 6-1 VR first.
///
/// The VR is checked before any value read, because an OD element projects
/// to `Float64` exactly as an FD one does and a text VR other than DS
/// projects to `Text` exactly as DS does, so the value variant alone
/// cannot tell the VR. A DS element reads its one component, trimmed of
/// every trailing space and NUL, through `clean_decimal`, and an FD
/// element its one value through `fd_text`. Anything else, including a
/// group `resolve` error, is no token and never a refusal.
fn group_token(
    multi: &MultiframeMetadata<'_>,
    frame: usize,
    group: Tag,
    attribute: Tag,
    vr: VR,
) -> Option<String> {
    let found = match multi.resolve(frame, group, attribute, TopLevelFallback::Disallowed) {
        Ok(Some(found)) => found,
        _ => return None,
    };
    let element = found.element();
    if element.vr() != vr {
        return None;
    }
    match element.value() {
        MetadataValue::Text(values) if values.len() == 1 => {
            let text = values
                .first()
                .map(|text| text.trim_end_matches([' ', '\u{0}']))?;
            if text.is_empty() {
                None
            } else {
                clean_decimal(text)
            }
        }
        MetadataValue::Float64(values) if values.len() == 1 => {
            values.first().copied().and_then(fd_text)
        }
        _ => None,
    }
}

/// One corner field for one frame, the top level first, then the groups.
///
/// The top-level tag and the group attribute are named separately, because
/// the echo group carries Effective Echo Time where the classic top level
/// carries Echo Time. A present top-level value that yields no token stops
/// the read, and an absent or trim-empty one falls to the per-frame and
/// shared groups through one `resolve` call, which orders per-frame first
/// and never falls from a present per-frame element to the shared one.
fn corner_token(
    metadata: &MetadataSet,
    multi: &MultiframeMetadata<'_>,
    frame: usize,
    top_level: Tag,
    group: Tag,
    attribute: Tag,
    group_vr: VR,
) -> Option<String> {
    match top_level_read(metadata, top_level) {
        TopLevelRead::Token(token) => Some(token),
        TopLevelRead::Stop => None,
        TopLevelRead::Absent => group_token(multi, frame, group, attribute, group_vr),
    }
}

/// The mAs corner, the Exposure chain and then the CT Exposure group.
///
/// `exposure_mas` orders Exposure then Exposure in µAs over 1000 and never
/// computes one from tube current and exposure time. The group's Exposure
/// in mAs is the chain's last source, `ExposureInmAs` its token, the
/// PS3.6 2026c keyword of (0018,9332) in the pattern of the other two
/// sources.
fn corner_exposure_mas(
    metadata: &MetadataSet,
    multi: &MultiframeMetadata<'_>,
    frame: usize,
) -> (Option<String>, &'static str) {
    let exposure = chain_text(metadata, EXPOSURE);
    let micro = chain_text(metadata, EXPOSURE_MICRO);
    let (mas, source) = exposure_mas(exposure.as_deref(), micro.as_deref());
    if mas.is_some() {
        return (mas, source);
    }
    match group_token(multi, frame, CT_EXPOSURE_SEQUENCE, EXPOSURE_MAS, VR::FD) {
        Some(token) => (Some(token), "ExposureInmAs"),
        None => (None, ""),
    }
}

/// The spacing a measurement reads for one frame, PS3.3 C.7.6.2 and C.8.11.4.
///
/// Pixel Spacing resolves through the Pixel Measures Sequence with the top
/// level as the fallback, as [`frame_facts`] reads it. Imager Pixel
/// Spacing and the Estimated Radiographic Magnification Factor (PS3.3
/// C.8.7.5) are top level. A present attribute that is not one usable value
/// of its VR is `Unusable`, so a measurement refuses it rather than reading
/// the next source. A zero-length factor is absent, as a zero-length
/// rescale is.
fn calibration_inputs(
    metadata: &MetadataSet,
    multi: &MultiframeMetadata<'_>,
    frame: usize,
) -> Result<CalibrationInputs, PreviewError> {
    if frame >= multi.frame_count() {
        return Err(PreviewError::Frame);
    }
    let pixel_spacing = match spacing_pair(multi, frame, PIXEL_MEASURES_SEQUENCE, PIXEL_SPACING)? {
        SpacingRead::Absent => SpacingInput::Absent,
        SpacingRead::Unusable => SpacingInput::Unusable,
        SpacingRead::Pair(row, column) => decimal_pair(&row, &column),
    };
    let imager_spacing = if metadata.get(IMAGER_PIXEL_SPACING).is_none() {
        SpacingInput::Absent
    } else {
        match top_level_pair(metadata, IMAGER_PIXEL_SPACING) {
            Some((row, column)) => decimal_pair(&row, &column),
            None => SpacingInput::Unusable,
        }
    };
    let magnification = match metadata.get(MAGNIFICATION_FACTOR) {
        None => FactorInput::Absent,
        Some(element) if matches!(element.value(), MetadataValue::Empty) => FactorInput::Absent,
        Some(element) => {
            let parts: Vec<&str> = element
                .semantic_text()
                .unwrap_or_default()
                .into_iter()
                .filter(|part| !part.is_empty())
                .collect();
            match (parts.as_slice(), parts.first()) {
                ([_], Some(text)) => parse_decimal_string(text.trim())
                    .map_or(FactorInput::Unusable, FactorInput::Value),
                _ => FactorInput::Unusable,
            }
        }
    };
    Ok(CalibrationInputs {
        pixel_spacing,
        imager_spacing,
        magnification,
    })
}

/// A pair [`clean_decimal`] already accepted, parsed into the stored numbers.
fn decimal_pair(row: &str, column: &str) -> SpacingInput {
    match (parse_decimal_string(row), parse_decimal_string(column)) {
        (Some(row), Some(column)) => SpacingInput::Pair { row, column },
        _ => SpacingInput::Unusable,
    }
}

fn token_list(
    multi: &MultiframeMetadata<'_>,
    frame: usize,
    group: Tag,
    attribute: Tag,
) -> Result<Option<Vec<String>>, PreviewError> {
    let found = multi
        .resolve(frame, group, attribute, TopLevelFallback::Allowed)
        .map_err(|_| PreviewError::Attribute)?;
    let Some(found) = found else {
        return Ok(None);
    };
    if matches!(found.element().value(), MetadataValue::Empty) {
        return Ok(None);
    }
    let Some(parts) = found.element().semantic_text() else {
        return Ok(None);
    };
    Ok(Some(
        parts
            .into_iter()
            .filter(|part| !part.is_empty())
            .map(str::to_owned)
            .collect(),
    ))
}

/// F-206's letters from the six Image Orientation (Patient) values, or `None`.
fn orientation_letters(parts: &[String]) -> Option<[String; 4]> {
    if parts.len() != 6 {
        return None;
    }
    let mut values = Vec::new();
    for part in parts {
        let parsed = clean_decimal(part).and_then(|text| parse_decimal_string(&text))?;
        values.push(parsed);
    }
    let component = |index: usize| values.get(index).copied().unwrap_or(0.0);
    let right_x = component(0);
    let right_y = component(1);
    let right_z = component(2);
    let bottom_x = component(3);
    let bottom_y = component(4);
    let bottom_z = component(5);
    // A cosine that is not a unit vector has no patient edge.
    // 1e-4 covers a DS that is not written to full precision.
    if !unit_cosine(right_x, right_y, right_z) || !unit_cosine(bottom_x, bottom_y, bottom_z) {
        return None;
    }
    let right = direction_letter(right_x, right_y, right_z);
    let bottom = direction_letter(bottom_x, bottom_y, bottom_z);
    Some([opposite(&right), right, opposite(&bottom), bottom])
}

fn unit_cosine(x: f64, y: f64, z: f64) -> bool {
    let length_sq = x * x + y * y + z * z;
    (length_sq - 1.0).abs() <= 1.0e-4
}

fn direction_letter(x: f64, y: f64, z: f64) -> String {
    let ax = x.abs();
    let ay = y.abs();
    let az = z.abs();
    let letter = if ax >= ay && ax >= az {
        if x.is_sign_negative() { "R" } else { "L" }
    } else if ay >= az {
        if y.is_sign_negative() { "A" } else { "P" }
    } else if z.is_sign_negative() {
        "F"
    } else {
        "H"
    };
    letter.to_owned()
}

fn opposite(letter: &str) -> String {
    letter
        .chars()
        .map(|ch| match ch {
            'L' => 'R',
            'R' => 'L',
            'A' => 'P',
            'P' => 'A',
            'H' => 'F',
            'F' => 'H',
            other => other,
        })
        .collect()
}

fn clean_decimal(text: &str) -> Option<String> {
    let trimmed = text.trim();
    let parsed = parse_decimal_string(trimmed)?;
    if json_number_token(trimmed) {
        return Some(trimmed.to_owned());
    }
    let rendered = format!("{parsed}");
    json_number_token(&rendered).then_some(rendered)
}

fn json_number_token(text: &str) -> bool {
    let bytes = text.as_bytes();
    let mut index = 0;
    if bytes.first() == Some(&b'-') {
        index = 1;
    }
    let Some(first) = bytes.get(index) else {
        return false;
    };
    if *first == b'0' {
        index += 1;
    } else if first.is_ascii_digit() {
        index += 1;
        while bytes.get(index).is_some_and(u8::is_ascii_digit) {
            index += 1;
        }
    } else {
        return false;
    }
    if bytes.get(index) == Some(&b'.') {
        index += 1;
        let start = index;
        while bytes.get(index).is_some_and(u8::is_ascii_digit) {
            index += 1;
        }
        if index == start {
            return false;
        }
    }
    if matches!(bytes.get(index), Some(b'e' | b'E')) {
        index += 1;
        if matches!(bytes.get(index), Some(b'+' | b'-')) {
            index += 1;
        }
        let start = index;
        while bytes.get(index).is_some_and(u8::is_ascii_digit) {
            index += 1;
        }
        if index == start {
            return false;
        }
    }
    index == bytes.len()
}

fn push_json_token(out: &mut String, name: &str, token: Option<&str>) {
    out.push_str(",\"");
    out.push_str(name);
    out.push_str("\":");
    match token {
        Some(text) => out.push_str(text),
        None => out.push_str("null"),
    }
}

/// Part 10 bytes with the top-level pixel value removed.
///
/// Folder open parses every file for the series list. Copying each pixel
/// value and then dropping it leaves the region resident on this host, and a
/// later frame of a different size does not reuse it. The list reads no pixel,
/// so the value is replaced with an empty element. `None` means the encoding
/// was not walked and the caller parses `bytes` unchanged.
fn bytes_without_pixel_payload(bytes: &[u8]) -> Option<Vec<u8>> {
    split_pixel_payload(bytes).map(|(header, _range)| header)
}

fn split_pixel_payload(bytes: &[u8]) -> Option<(Vec<u8>, std::ops::Range<usize>)> {
    let (dataset_at, explicit, little) = dataset_origin(bytes)?;
    let mut out = Vec::new();
    out.extend_from_slice(bytes.get(..dataset_at)?);
    let mut cursor = dataset_at;
    let mut pixel_range = None;
    while cursor < bytes.len() {
        let tag = read_tag(bytes, cursor, little)?;
        if tag.0 != 0xFFFE && tag.1 == 0 && tag.0 != 0x0002 {
            return None;
        }
        if tag == (0x7FE0, 0x0010) {
            if pixel_range.is_some() {
                return None;
            }
            let (header_len, value_end) = pixel_value_span(bytes, cursor, explicit, little)?;
            let value_at = cursor.checked_add(header_len)?;
            if value_end < value_at {
                return None;
            }
            let header = bytes.get(cursor..value_at)?;
            out.extend_from_slice(header);
            let length_at = out.len().checked_sub(4)?;
            let tail = out.get_mut(length_at..)?;
            tail.fill(0);
            pixel_range = Some(value_at..value_end);
            cursor = value_end;
            continue;
        }
        let next = element_end(bytes, cursor, explicit, little, 0)?;
        if next <= cursor {
            return None;
        }
        out.extend_from_slice(bytes.get(cursor..next)?);
        cursor = next;
    }
    let range = pixel_range?;
    // A length that runs past the file is not a pixel value. The caller then
    // parses the whole object instead of borrowing a range it cannot see.
    if range.is_empty() || range.end > bytes.len() || out.len() >= bytes.len() {
        return None;
    }
    Some((out, range))
}

fn dataset_origin(bytes: &[u8]) -> Option<(usize, bool, bool)> {
    if bytes.get(128..132) != Some(b"DICM") {
        return None;
    }
    let mut cursor = 132_usize;
    let mut syntax = String::new();
    while cursor < bytes.len() {
        let tag = read_tag(bytes, cursor, true)?;
        if tag.0 != 0x0002 {
            break;
        }
        let (value_at, value_end) = explicit_value_span(bytes, cursor, true)?;
        if tag == TRANSFER_SYNTAX {
            let raw = bytes.get(value_at..value_end)?;
            syntax = std::str::from_utf8(raw)
                .ok()?
                .trim_end_matches(['\0', ' '])
                .to_owned();
        }
        cursor = value_end;
    }
    // Only the three uncompressed syntaxes whose element headers this walker
    // reads. Every other UID, including deflated Explicit VR Little Endian
    // (1.2.840.10008.1.2.1.99), returns None so the caller parses the file.
    let (explicit, little) = match syntax.as_str() {
        "1.2.840.10008.1.2" => (false, true),
        "1.2.840.10008.1.2.1" => (true, true),
        "1.2.840.10008.1.2.2" => (true, false),
        _ => return None,
    };
    Some((cursor, explicit, little))
}

fn pixel_value_span(
    bytes: &[u8],
    at: usize,
    explicit: bool,
    little: bool,
) -> Option<(usize, usize)> {
    if !explicit {
        let length = read_u32(bytes, at.checked_add(4)?, little)?;
        let value_at = at.checked_add(8)?;
        if length == UNDEFINED_LENGTH {
            return None;
        }
        let value_end = value_at.checked_add(usize::try_from(length).ok()?)?;
        return Some((8, value_end));
    }
    let vr = bytes.get(at.checked_add(4)?..at.checked_add(6)?)?;
    if !long_vr(vr) {
        return None;
    }
    let length = read_u32(bytes, at.checked_add(8)?, little)?;
    let value_at = at.checked_add(12)?;
    if length == UNDEFINED_LENGTH {
        return None;
    }
    let value_end = value_at.checked_add(usize::try_from(length).ok()?)?;
    Some((12, value_end))
}

fn element_end(bytes: &[u8], at: usize, explicit: bool, little: bool, depth: u32) -> Option<usize> {
    if depth > 8 {
        return None;
    }
    let tag = read_tag(bytes, at, little)?;
    if tag.0 == 0xFFFE {
        let length = read_u32(bytes, at.checked_add(4)?, little)?;
        if length == UNDEFINED_LENGTH {
            return undefined_item_end(bytes, at.checked_add(8)?, explicit, little, depth);
        }
        return at
            .checked_add(8)?
            .checked_add(usize::try_from(length).ok()?);
    }
    if !explicit {
        let length = read_u32(bytes, at.checked_add(4)?, little)?;
        let value_at = at.checked_add(8)?;
        if length == UNDEFINED_LENGTH {
            return undefined_sequence_end(bytes, value_at, explicit, little, depth);
        }
        return value_at.checked_add(usize::try_from(length).ok()?);
    }
    let vr = bytes.get(at.checked_add(4)?..at.checked_add(6)?)?;
    if long_vr(vr) {
        let length = read_u32(bytes, at.checked_add(8)?, little)?;
        let value_at = at.checked_add(12)?;
        if length == UNDEFINED_LENGTH {
            return undefined_sequence_end(bytes, value_at, explicit, little, depth);
        }
        return value_at.checked_add(usize::try_from(length).ok()?);
    }
    let length = usize::from(read_u16(bytes, at.checked_add(6)?, little)?);
    at.checked_add(8)?.checked_add(length)
}

fn explicit_value_span(bytes: &[u8], at: usize, little: bool) -> Option<(usize, usize)> {
    let vr = bytes.get(at.checked_add(4)?..at.checked_add(6)?)?;
    if long_vr(vr) {
        let length = read_u32(bytes, at.checked_add(8)?, little)?;
        if length == UNDEFINED_LENGTH {
            return None;
        }
        let value_at = at.checked_add(12)?;
        let value_end = value_at.checked_add(usize::try_from(length).ok()?)?;
        return Some((value_at, value_end));
    }
    let length = usize::from(read_u16(bytes, at.checked_add(6)?, little)?);
    let value_at = at.checked_add(8)?;
    Some((value_at, value_at.checked_add(length)?))
}

fn undefined_sequence_end(
    bytes: &[u8],
    mut cursor: usize,
    explicit: bool,
    little: bool,
    depth: u32,
) -> Option<usize> {
    if depth > 8 {
        return None;
    }
    while cursor < bytes.len() {
        let tag = read_tag(bytes, cursor, little)?;
        if tag == (0xFFFE, 0xE0DD) {
            let length = read_u32(bytes, cursor.checked_add(4)?, little)?;
            if length != 0 {
                return None;
            }
            return cursor.checked_add(8);
        }
        if tag != (0xFFFE, 0xE000) {
            return None;
        }
        let length = read_u32(bytes, cursor.checked_add(4)?, little)?;
        if length == UNDEFINED_LENGTH {
            cursor = undefined_item_end(bytes, cursor.checked_add(8)?, explicit, little, depth)?;
        } else {
            cursor = cursor
                .checked_add(8)?
                .checked_add(usize::try_from(length).ok()?)?;
        }
    }
    None
}

fn undefined_item_end(
    bytes: &[u8],
    mut cursor: usize,
    explicit: bool,
    little: bool,
    depth: u32,
) -> Option<usize> {
    if depth > 8 {
        return None;
    }
    while cursor < bytes.len() {
        let tag = read_tag(bytes, cursor, little)?;
        if tag == (0xFFFE, 0xE00D) {
            let length = read_u32(bytes, cursor.checked_add(4)?, little)?;
            if length != 0 {
                return None;
            }
            return cursor.checked_add(8);
        }
        let next = element_end(bytes, cursor, explicit, little, depth.checked_add(1)?)?;
        if next <= cursor {
            return None;
        }
        cursor = next;
    }
    None
}

fn long_vr(vr: &[u8]) -> bool {
    matches!(
        vr,
        b"OB"
            | b"OD"
            | b"OF"
            | b"OL"
            | b"OV"
            | b"OW"
            | b"SQ"
            | b"SV"
            | b"UC"
            | b"UN"
            | b"UR"
            | b"UT"
            | b"UV"
    )
}

fn read_tag(bytes: &[u8], at: usize, little: bool) -> Option<(u16, u16)> {
    Some((
        read_u16(bytes, at, little)?,
        read_u16(bytes, at.checked_add(2)?, little)?,
    ))
}

fn read_u16(bytes: &[u8], at: usize, little: bool) -> Option<u16> {
    let raw = bytes.get(at..at.checked_add(2)?)?;
    let bytes = [raw.first().copied()?, raw.get(1).copied()?];
    Some(if little {
        u16::from_le_bytes(bytes)
    } else {
        u16::from_be_bytes(bytes)
    })
}

fn read_u32(bytes: &[u8], at: usize, little: bool) -> Option<u32> {
    let raw = bytes.get(at..at.checked_add(4)?)?;
    let bytes = [
        raw.first().copied()?,
        raw.get(1).copied()?,
        raw.get(2).copied()?,
        raw.get(3).copied()?,
    ];
    Some(if little {
        u32::from_le_bytes(bytes)
    } else {
        u32::from_be_bytes(bytes)
    })
}

const UNDEFINED_LENGTH: u32 = 0xFFFF_FFFF;
const TRANSFER_SYNTAX: (u16, u16) = (0x0002, 0x0010);

/// Series and image attributes for the left-hand list and the corner text.
///
/// The JSON holds the values stored in the file. The caller must not write it
/// to a log.
///
/// # Errors
///
/// Returns [`PreviewError::NotPart10`] when the bytes are not a Part 10 object.
pub fn inspect_json(bytes: &[u8], frame: u32) -> Result<String, PreviewError> {
    let stripped = bytes_without_pixel_payload(bytes);
    let bytes = stripped.as_deref().unwrap_or(bytes);
    let parsed = parse_part10(bytes).map_err(part10_error)?;
    InspectParts::new(parsed.object())?.reply(frame)
}

/// Every frame's inspect reply for a multiframe whose windows come from a
/// per-frame functional group.
///
/// The page asks again only for those files, and only by id, so this is what
/// the process keeps once the posted bytes are dropped. It is computed from
/// the parsed data set, which every transfer syntax reaches the same way.
/// The frame-independent parts are kept once and only each frame's spacing,
/// edge and window fields per frame.
pub(crate) struct KeptReplies {
    parts: Shared,
    frames: Vec<(String, String)>,
}

/// The parts of an inspect reply no frame changes.
struct Shared {
    /// Up to and including `monochrome`.
    head: String,
    /// `frameOfReferenceUid`, the corner fields, `presets`, `cine` and
    /// `frameTimeMs`.
    middle: String,
    /// `windowsPerFrame` and `planes`, to the closing brace.
    tail: String,
}

impl Shared {
    fn reply(&self, facts: &str, windows: &str) -> String {
        let mut out = String::with_capacity(
            self.head.len() + facts.len() + self.middle.len() + windows.len() + self.tail.len(),
        );
        out.push_str(&self.head);
        out.push_str(facts);
        out.push_str(&self.middle);
        out.push_str(windows);
        out.push_str(&self.tail);
        out
    }

    fn bytes(&self) -> usize {
        self.head
            .capacity()
            .saturating_add(self.middle.capacity())
            .saturating_add(self.tail.capacity())
    }
}

impl KeptReplies {
    /// The reply for `frame`, byte for byte what `inspect_json` writes.
    pub(crate) fn reply(&self, frame: u32) -> Option<String> {
        let (facts, windows) = self.frames.get(usize::try_from(frame).ok()?)?;
        Some(self.parts.reply(facts, windows))
    }

    /// One frame's reply of `len` bytes, for the pool's own tests.
    #[cfg(test)]
    pub(crate) fn filler(len: usize) -> Self {
        Self {
            parts: Shared {
                head: "x".repeat(len),
                middle: String::new(),
                tail: String::new(),
            },
            frames: vec![(String::new(), String::new())],
        }
    }

    /// The bytes these replies own on the heap: every string's capacity and
    /// the frame list's slots.
    pub(crate) fn bytes(&self) -> usize {
        self.frames
            .iter()
            .map(|(facts, windows)| facts.capacity().saturating_add(windows.capacity()))
            .fold(self.parts.bytes(), usize::saturating_add)
            .saturating_add(
                self.frames
                    .capacity()
                    .saturating_mul(size_of::<(String, String)>()),
            )
    }
}

/// One `POST /file`'s inspect reply for `frame` and its [`KeptReplies`].
///
/// The bytes are parsed, the metadata projected and the plane list built
/// once, and both the reply and the kept set are taken from that one
/// [`InspectParts`]. The kept set is `None` for a file that is not a
/// per-frame multiframe, or when it would pass `cap` bytes.
pub(crate) fn inspect_post(
    bytes: &[u8],
    frame: u32,
    cap: usize,
) -> (Result<String, PreviewError>, Option<KeptReplies>) {
    let stripped = bytes_without_pixel_payload(bytes);
    let bytes = stripped.as_deref().unwrap_or(bytes);
    let parsed = match parse_part10(bytes) {
        Ok(parsed) => parsed,
        Err(error) => return (Err(part10_error(error)), None),
    };
    let parts = match InspectParts::new(parsed.object()) {
        Ok(parts) => parts,
        Err(error) => return (Err(error), None),
    };
    let reply = parts.reply(frame);
    let kept = if reply.is_ok() {
        kept_replies(parts, cap)
    } else {
        None
    };
    (reply, kept)
}

/// [`inspect_post`]'s kept set alone, for the pool's own tests.
#[cfg(test)]
pub(crate) fn per_frame_replies(bytes: &[u8], cap: usize) -> Option<KeptReplies> {
    inspect_post(bytes, 0, cap).1
}

/// [`KeptReplies`] from one post's parts, or `None` for a file that is not a
/// per-frame multiframe or when they would pass `cap` bytes.
fn kept_replies(parts: InspectParts, cap: usize) -> Option<KeptReplies> {
    if parts.frames < 2 || !parts.per_frame {
        return None;
    }
    let mut total = parts.shared.bytes();
    let mut frames = Vec::new();
    for frame in 0..parts.frames {
        let (mut facts, mut windows) = parts.frame_fields(frame).ok()?;
        facts.shrink_to_fit();
        windows.shrink_to_fit();
        total = total
            .saturating_add(facts.capacity())
            .saturating_add(windows.capacity())
            .saturating_add(size_of::<(String, String)>());
        if total > cap {
            return None;
        }
        frames.push((facts, windows));
    }
    frames.shrink_to_fit();
    let mut shared = parts.shared;
    shared.head.shrink_to_fit();
    shared.middle.shrink_to_fit();
    shared.tail.shrink_to_fit();
    Some(KeptReplies {
        parts: shared,
        frames,
    })
}

/// One parsed object's inspect reply, built so the frame-independent parts
/// and the metadata projection are made once however many frames are asked.
struct InspectParts {
    shared: Shared,
    /// Number of Frames, or 0 when it could not be read.
    frames: u32,
    /// Whether any frame's Window Center comes from a per-frame group.
    per_frame: bool,
    /// The projection the per-frame fields read, or `None` when the data set
    /// or its functional groups cannot be read, which empties those fields.
    metadata: Option<MetadataSet>,
}

impl InspectParts {
    fn reply(&self, frame: u32) -> Result<String, PreviewError> {
        let (facts, windows) = self.frame_fields(frame)?;
        Ok(self.shared.reply(&facts, &windows))
    }

    /// The spacing and edge fields, then the window list fields, of `frame`.
    fn frame_fields(&self, frame: u32) -> Result<(String, String), PreviewError> {
        let frame_index = usize::try_from(frame).map_err(|_| PreviewError::Frame)?;
        if self.frames > 0 && frame_index >= usize::try_from(self.frames).unwrap_or(usize::MAX) {
            return Err(PreviewError::Frame);
        }
        let multi = self.metadata.as_ref().and_then(|metadata| {
            MultiframeMetadata::new(metadata)
                .ok()
                .map(|multi| (metadata, multi))
        });
        let facts = multi
            .as_ref()
            .and_then(|(metadata, multi)| frame_facts(metadata, multi, frame_index).ok())
            .unwrap_or_else(FrameFacts::empty);
        let mut fields = String::new();
        push_json_token(&mut fields, "rowSpacing", facts.row_spacing.as_deref());
        push_json_token(
            &mut fields,
            "columnSpacing",
            facts.column_spacing.as_deref(),
        );
        push_field(&mut fields, "spacingSource", facts.spacing_source, false);
        push_field(&mut fields, "edgeLeft", &facts.edge_left, false);
        push_field(&mut fields, "edgeRight", &facts.edge_right, false);
        push_field(&mut fields, "edgeTop", &facts.edge_top, false);
        push_field(&mut fields, "edgeBottom", &facts.edge_bottom, false);
        push_field(&mut fields, "edgeSource", facts.edge_source, false);
        let mut windows = String::new();
        let listed = multi
            .as_ref()
            .and_then(|(_, multi)| window_rows(multi, frame_index));
        push_window_rows(&mut windows, listed.unwrap_or_default());
        Ok((fields, windows))
    }

    fn new(object: &DefaultDicomObject) -> Result<Self, PreviewError> {
        let mut sop = text_or_empty(object, SOP_INSTANCE)?;
        if sop.is_empty() {
            sop = object
                .meta()
                .media_storage_sop_instance_uid()
                .trim_end_matches('\0')
                .to_owned();
        }
        let photometric_text = text_or_empty(object, PHOTOMETRIC)?;
        let samples = optional_u32(object, SAMPLES_PER_PIXEL)?;
        let rows = optional_u32(object, ROWS)?;
        let columns = optional_u32(object, COLUMNS)?;
        let frames = frame_count(object).unwrap_or(0);
        let monochrome = (photometric_text == "MONOCHROME1" || photometric_text == "MONOCHROME2")
            && samples == 1
            && rows > 0
            && columns > 0
            && frames >= 1;
        let mut out = String::from("{");
        push_field(
            &mut out,
            "seriesUid",
            &text_or_empty(object, SERIES_UID)?,
            true,
        );
        push_field(
            &mut out,
            "seriesNumber",
            &text_or_empty(object, SERIES_NUMBER)?,
            false,
        );
        push_field(
            &mut out,
            "modality",
            &text_or_empty(object, MODALITY)?,
            false,
        );
        push_field(
            &mut out,
            "seriesDescription",
            &text_or_empty(object, SERIES_DESCRIPTION)?,
            false,
        );
        push_field(
            &mut out,
            "instanceNumber",
            &text_or_empty(object, INSTANCE_NUMBER)?,
            false,
        );
        push_field(&mut out, "sopInstanceUid", &sop, false);
        push_field(
            &mut out,
            "sopClassUid",
            &sendable_uid(optional_text(object, SOP_CLASS).ok().flatten()),
            false,
        );
        push_field(
            &mut out,
            "studyUid",
            &sendable_uid(optional_text(object, STUDY_UID).ok().flatten()),
            false,
        );
        push_field(
            &mut out,
            "patientName",
            &text_or_empty(object, PATIENT_NAME)?,
            false,
        );
        push_field(
            &mut out,
            "patientId",
            &text_or_empty(object, PATIENT_ID)?,
            false,
        );
        push_field(
            &mut out,
            "studyDate",
            &text_or_empty(object, STUDY_DATE)?,
            false,
        );
        // F-215. The stored Study Time, PS3.3 C.7.2.1, which a saved document
        // copies verbatim. F-212's `studyTime` is display text.
        push_field(
            &mut out,
            "studyTimeValue",
            &optional_text(object, STUDY_TIME)
                .ok()
                .flatten()
                .unwrap_or_default(),
            false,
        );
        push_field(
            &mut out,
            "studyDescription",
            &text_or_empty(object, STUDY_DESCRIPTION)?,
            false,
        );
        push_field(
            &mut out,
            "accession",
            &text_or_empty(object, ACCESSION)?,
            false,
        );
        push_number(&mut out, "rows", rows);
        push_number(&mut out, "columns", columns);
        push_number(&mut out, "samples", samples);
        push_field(&mut out, "photometric", &photometric_text, false);
        push_number(&mut out, "frames", frames);
        push_field(
            &mut out,
            "windowCenter",
            &text_or_empty(object, WINDOW_CENTER)?,
            false,
        );
        push_field(
            &mut out,
            "windowWidth",
            &text_or_empty(object, WINDOW_WIDTH)?,
            false,
        );
        push_field(
            &mut out,
            "windowFunction",
            &text_or_empty(object, VOI_FUNCTION)?,
            false,
        );
        out.push_str(",\"monochrome\":");
        out.push_str(if monochrome { "true" } else { "false" });
        let colour = photometric(object).ok().is_some_and(|space| match space {
            PhotometricInterpretation::Monochrome1 | PhotometricInterpretation::Monochrome2 => {
                false
            }
            PhotometricInterpretation::PaletteColor => samples == 1,
            _ => samples == 3,
        }) && rows > 0
            && columns > 0
            && frames > 0;
        out.push_str(",\"colour\":");
        out.push_str(if colour { "true" } else { "false" });
        let icc_present = object.element(Tag(0x0028, 0x2000)).is_ok();
        let metadata = project_metadata(object).ok();
        let icc_status = if colour {
            metadata
                .as_ref()
                .map(|metadata| crate::colour::read_profile(metadata).0)
                .unwrap_or(if icc_present {
                    crate::colour::IccStatus::NotApplied("malformed")
                } else {
                    crate::colour::IccStatus::Absent
                })
        } else {
            crate::colour::IccStatus::Absent
        };
        out.push_str(",\"icc\":");
        out.push_str(if icc_present { "true" } else { "false" });
        out.push_str(",\"icc_applied\":");
        out.push_str(if icc_status.applied() {
            "true"
        } else {
            "false"
        });
        push_field(&mut out, "icc_kind", icc_status.kind(), false);
        let mut middle = String::new();
        push_field(
            &mut middle,
            "frameOfReferenceUid",
            &sendable_uid(optional_text(object, FRAME_OF_REFERENCE).ok().flatten()),
            false,
        );
        push_corner_fields(&mut middle, object);
        push_presets(&mut middle, object);
        push_cine(&mut middle, metadata.as_ref());
        let per_frame = metadata
            .as_ref()
            .and_then(|metadata| MultiframeMetadata::new(metadata).ok())
            .is_some_and(|multi| windows_per_frame(&multi, frames));
        let mut tail = String::from(",\"windowsPerFrame\":");
        tail.push_str(if per_frame { "true" } else { "false" });
        tail.push_str(",\"planes\":[");
        let planes = metadata
            .as_ref()
            .map(|metadata| plane_tokens(metadata, frames))
            .unwrap_or_else(|| vec![None; frame_list_len(frames)]);
        for (index, token) in planes.iter().enumerate() {
            if index > 0 {
                tail.push(',');
            }
            match token {
                Some(text) => json_string(&mut tail, text),
                None => tail.push_str("null"),
            }
        }
        tail.push_str("]}");
        Ok(Self {
            shared: Shared {
                head: out,
                middle,
                tail,
            },
            frames,
            per_frame,
            metadata,
        })
    }
}

/// `cine` and `frameTimeMs` of one object, from [`cine_timing`].
///
/// Frame Time `(0018,1063)` is DS in milliseconds, PS3.3 C.7.6.5, and is the
/// interval between frames of this object only, C.7.6.5.1.1. `cine` is
/// `frame-time`, `vector`, `rate`, `none`, `unusable` or `unreadable`.
/// `frameTimeMs` is the interval as a JSON number for `frame-time` and `null`
/// otherwise. `unusable` is a timing value [`cine_timing`] refuses with
/// [`SeriesOpenReason::Cine`]. `unreadable` is any other refusal, or a data set
/// that cannot be projected, and says nothing about Frame Time. The page owns
/// the change to a per-second label.
fn push_cine(out: &mut String, metadata: Option<&MetadataSet>) {
    let (cine, milliseconds) = match metadata.map(cine_timing) {
        Some(Ok(CineTiming::FrameTime { milliseconds })) if milliseconds.is_finite() => {
            ("frame-time", Some(milliseconds))
        }
        Some(Ok(CineTiming::FrameTimeVector)) => ("vector", None),
        Some(Ok(CineTiming::RecommendedDisplayFrameRate { .. } | CineTiming::CineRate { .. })) => {
            ("rate", None)
        }
        Some(Ok(CineTiming::Unavailable)) => ("none", None),
        // A later source this page does not know plays at the slider, as
        // every source but Frame Time does.
        Some(Ok(_)) => ("none", None),
        Some(Err(error)) if error.reason() == SeriesOpenReason::Cine => ("unusable", None),
        Some(Err(_)) | None => ("unreadable", None),
    };
    push_field(out, "cine", cine, false);
    out.push_str(",\"frameTimeMs\":");
    match milliseconds {
        // `Display` for `f64` never uses an exponent, and the value is finite.
        Some(value) => out.push_str(&value.to_string()),
        None => out.push_str("null"),
    }
}

/// The presets offered on a CT, in the order the number keys reach them.
const CT_PRESETS: [WindowPreset; 3] = [
    WindowPreset::SoftTissue,
    WindowPreset::Lung,
    WindowPreset::Bone,
];

/// The `X-Ocelli-Window-Preset` token of a preset, the inverse of
/// [`window_preset_from_token`].
const fn preset_token(preset: WindowPreset) -> &'static str {
    match preset {
        WindowPreset::SoftTissue => "soft_tissue",
        WindowPreset::Lung => "lung",
        WindowPreset::Bone => "bone",
    }
}

const fn preset_name(preset: WindowPreset) -> &'static str {
    match preset {
        WindowPreset::SoftTissue => "Soft tissue",
        WindowPreset::Lung => "Lung",
        WindowPreset::Bone => "Bone",
    }
}

/// The window list the page numbers, for one frame.
///
/// `presets` is [`CT_PRESETS`] on a CT and empty otherwise. `windows` is one
/// row per Window Center and Window Width pair in file order, PS3.3
/// C.11.2.1.2.2, with the Window Center & Width Explanation at the same
/// position, or `Window n` counted from 1 when that slot is absent or empty.
/// `voiLuts` is one row per VOI LUT Sequence item, by its LUT Explanation or
/// `Lookup table n`. `windowsPerFrame` is whether this frame's Window Center
/// came from a per-frame functional group, so the page asks again when the
/// frame changes. A window attribute that cannot be read lists nothing and
/// never refuses the scan.
fn push_presets(out: &mut String, object: &DefaultDicomObject) {
    let ct = optional_text(object, MODALITY).ok().flatten().as_deref() == Some("CT");
    out.push_str(",\"presets\":[");
    if ct {
        for (at, preset) in CT_PRESETS.into_iter().enumerate() {
            if at > 0 {
                out.push(',');
            }
            out.push('{');
            push_field(out, "token", preset_token(preset), true);
            push_field(out, "name", preset_name(preset), false);
            // `Display` for `f32` never uses an exponent.
            out.push_str(&format!(
                ",\"center\":{},\"width\":{}}}",
                preset.center(),
                preset.width()
            ));
        }
    }
    out.push(']');
}

/// `windows` and `voiLuts` for one frame's listed rows.
fn push_window_rows(out: &mut String, (rows, luts): (Vec<WindowRow>, Vec<String>)) {
    out.push_str(",\"windows\":[");
    for (at, row) in rows.iter().enumerate() {
        if at > 0 {
            out.push(',');
        }
        out.push_str("{\"center\":");
        out.push_str(row.center.as_deref().unwrap_or("null"));
        out.push_str(",\"width\":");
        out.push_str(row.width.as_deref().unwrap_or("null"));
        push_field(out, "explanation", &row.explanation, false);
        push_field(out, "label", &row.label, false);
        out.push('}');
    }
    out.push_str("],\"voiLuts\":[");
    for (at, explanation) in luts.iter().enumerate() {
        if at > 0 {
            out.push(',');
        }
        let label = if explanation.is_empty() {
            format!("Lookup table {}", at + 1)
        } else {
            explanation.clone()
        };
        out.push('{');
        push_field(out, "explanation", explanation, true);
        push_field(out, "label", &label, false);
        out.push('}');
    }
    out.push(']');
}

/// Whether any frame's Window Center resolves from a per-frame functional
/// group. The page then asks every frame other than 0 for its own list, so
/// a file whose later frames alone carry their own VOI is not read as
/// frame 0's.
fn windows_per_frame(multi: &MultiframeMetadata<'_>, frames: u32) -> bool {
    (0..frame_list_len(frames)).any(|frame| {
        matches!(
            multi
                .resolve(
                    frame,
                    FRAME_VOI_LUT_SEQUENCE,
                    WINDOW_CENTER,
                    TopLevelFallback::Allowed
                )
                .ok()
                .flatten()
                .map(|found| found.source()),
            Some(FunctionalGroupSource::PerFrame(_))
        )
    })
}

fn frame_list_len(frames: u32) -> usize {
    usize::try_from(frames).map_or(FRAME_LIST_CAP, |count| count.min(FRAME_LIST_CAP))
}

/// One listed file window.
struct WindowRow {
    /// The stored Window Center as a JSON number, or `None` for an empty slot.
    center: Option<String>,
    width: Option<String>,
    explanation: String,
    label: String,
}

/// The rows and the VOI LUT explanations of one frame, or `None` when an
/// attribute cannot be read.
fn window_rows(
    multi: &MultiframeMetadata<'_>,
    frame: usize,
) -> Option<(Vec<WindowRow>, Vec<String>)> {
    let centers = resolved_slots(multi, frame, FRAME_VOI_LUT_SEQUENCE, WINDOW_CENTER).ok()?;
    let widths = resolved_slots(multi, frame, FRAME_VOI_LUT_SEQUENCE, WINDOW_WIDTH).ok()?;
    let explanations =
        resolved_slots(multi, frame, FRAME_VOI_LUT_SEQUENCE, WINDOW_EXPLANATION).ok()?;
    let blank = |slots: &[String]| slots.iter().all(String::is_empty);
    let mut rows = Vec::new();
    // Pairs that cannot be paired, or that carry no number at all, are not
    // a list of windows. The preview refuses those files or draws them at
    // the automatic window.
    if centers.len() == widths.len() && !(blank(&centers) && blank(&widths)) {
        for (at, (center, width)) in centers.iter().zip(&widths).enumerate() {
            let explanation = explanations.get(at).cloned().unwrap_or_default();
            let label = if explanation.is_empty() {
                format!("Window {}", at + 1)
            } else {
                explanation.clone()
            };
            rows.push(WindowRow {
                center: clean_decimal(center),
                width: clean_decimal(width),
                explanation,
                label,
            });
        }
    }
    let mut luts = Vec::new();
    if let Some(found) = multi
        .resolve(
            frame,
            FRAME_VOI_LUT_SEQUENCE,
            VOI_LUT_SEQUENCE,
            TopLevelFallback::Allowed,
        )
        .ok()?
        && let MetadataValue::Sequence(items) = found.element().value()
    {
        for item in items {
            let explanation = item
                .get(LUT_EXPLANATION)
                .and_then(MetadataElement::semantic_text)
                .and_then(|parts| parts.first().map(|part| (*part).to_owned()))
                .unwrap_or_default();
            luts.push(explanation);
        }
    }
    Some((rows, luts))
}

/// One top-level attribute read for a corner line.
enum CornerText {
    /// Absent, zero length, or only padding.
    Absent,
    /// The stored text, trimmed of its padding. A multi-valued element
    /// keeps its `\` separators.
    Text(String),
    /// Present with a value that is not text, such as a binary VR.
    NotText,
}

impl CornerText {
    fn text(&self) -> Option<&str> {
        match self {
            Self::Text(text) => Some(text),
            Self::Absent | Self::NotText => None,
        }
    }
}

/// Read one attribute for the corners without ever refusing the scan.
///
/// This is the non-failing form of the `frameOfReferenceUid` read, with a
/// value that is not text kept apart from an absent one, so a binary value
/// shows an empty corner field rather than its numbers as text.
fn corner_text(object: &DefaultDicomObject, tag: Tag) -> CornerText {
    let Ok(element) = object.element(tag) else {
        return CornerText::Absent;
    };
    match element.value() {
        DicomValue::Primitive(PrimitiveValue::Empty) => CornerText::Absent,
        DicomValue::Primitive(PrimitiveValue::Str(_) | PrimitiveValue::Strs(_)) => {
            match optional_text(object, tag).ok().flatten() {
                Some(text) => CornerText::Text(text),
                None => CornerText::Absent,
            }
        }
        DicomValue::Primitive(_) | DicomValue::Sequence(_) | DicomValue::PixelSequence(_) => {
            CornerText::NotText
        }
    }
}

/// A VM 1 DS as F-206's JSON number token, or `None` for anything else.
///
/// A multi-valued element keeps its `\` separator, which [`clean_decimal`]
/// does not parse, so it is `None` too.
fn corner_decimal(object: &DefaultDicomObject, tag: Tag) -> Option<String> {
    clean_decimal(corner_text(object, tag).text()?)
}

/// The F-212 corner fields, already formatted. None of them refuses the file.
fn push_corner_fields(out: &mut String, object: &DefaultDicomObject) {
    let name = corner_text(object, PATIENT_NAME);
    push_field(
        out,
        "patientNameText",
        &name.text().map(person_name_text).unwrap_or_default(),
        false,
    );
    let birth = corner_text(object, PATIENT_BIRTH_DATE);
    push_field(
        out,
        "birthDate",
        &birth.text().map(dicom_date_text).unwrap_or_default(),
        false,
    );
    // PS3.3 C.7.1.1. Only the enumerated values are shown.
    let sex = corner_text(object, PATIENT_SEX);
    let sex = sex
        .text()
        .filter(|value| matches!(*value, "M" | "F" | "O"))
        .unwrap_or_default();
    push_field(out, "sex", sex, false);
    let study = corner_text(object, STUDY_DATE);
    // A Patient's Age that is present but not text is not an AS, so it
    // gives no age and does not fall through to the computed one.
    let (age, age_source) = match corner_text(object, PATIENT_AGE) {
        CornerText::NotText => (String::new(), ""),
        read => age_text(read.text(), birth.text(), study.text()),
    };
    push_field(out, "age", &age, false);
    push_field(out, "ageSource", age_source, false);
    push_field(
        out,
        "studyDateText",
        &study.text().map(dicom_date_text).unwrap_or_default(),
        false,
    );
    let time = corner_text(object, STUDY_TIME);
    push_field(
        out,
        "studyTime",
        &time.text().map(dicom_time_text).unwrap_or_default(),
        false,
    );
    let institution = corner_text(object, INSTITUTION);
    push_field(
        out,
        "institution",
        institution.text().unwrap_or_default(),
        false,
    );
    for (field, tag) in [
        ("sliceLocation", SLICE_LOCATION),
        ("sliceThickness", SLICE_THICKNESS),
        ("kvp", KVP),
        ("repetitionTime", REPETITION_TIME),
        ("echoTime", ECHO_TIME),
    ] {
        push_json_token(out, field, corner_decimal(object, tag).as_deref());
    }
    let exposure = corner_text(object, EXPOSURE);
    let micro = corner_text(object, EXPOSURE_MICRO);
    let (mas, mas_source) = exposure_mas(exposure.text(), micro.text());
    push_json_token(out, "exposureMas", mas.as_deref());
    push_field(out, "exposureSource", mas_source, false);
}

/// The UID as the page may send it back, or `""`.
///
/// PS3.5 9.1: a UID is digits and `.` and at most [`UID_MAX_CHARS`]
/// characters. Anything else is reported empty, so it links nothing rather
/// than breaking the space-separated `/link` body.
fn sendable_uid(text: Option<String>) -> String {
    match text {
        Some(text)
            if text.len() <= UID_MAX_CHARS
                && text
                    .bytes()
                    .all(|byte| byte.is_ascii_digit() || byte == b'.') =>
        {
            text
        }
        _ => String::new(),
    }
}

/// One plane token per frame, up to [`FRAME_LIST_CAP`], or `None` where
/// `FrameGeometry::derive` refused the frame.
fn plane_tokens(metadata: &MetadataSet, frames: u32) -> Vec<Option<String>> {
    #[cfg(test)]
    note_inspect_work(
        metadata
            .get(SOP_INSTANCE)
            .and_then(MetadataElement::semantic_text)
            .and_then(|parts| parts.first().map(|part| (*part).to_owned())),
        false,
    );
    let count = frame_list_len(frames);
    let Ok(multi) = MultiframeMetadata::new(metadata) else {
        return vec![None; count];
    };
    (0..count)
        .map(|frame| {
            FrameGeometry::derive(&multi, frame)
                .ok()
                .map(|geometry| plane_token(&geometry.plane()))
        })
        .collect()
}

/// The thirteen numbers of a validated plane, joined by commas.
///
/// `Sx,Sy,Sz,Xx,Xy,Xz,Yx,Yy,Yz,rowSpacing,columnSpacing,rows,columns`. `S`
/// is Image Position (Patient), `X` and `Y` the validated row and column
/// cosines, and the spacings are in Pixel Spacing's order, row first, PS3.3
/// 10.7.1.3. Each `f64` is written with `{:e}`, the shortest text that reads
/// back to the same bits, which never exceeds 24 characters, and each `u32`
/// with `{}`. The page stores the token and posts it back unread.
pub fn plane_token(plane: &ImagePlane) -> String {
    let position = plane.position_vector();
    let row = plane.orientation.row();
    let column = plane.orientation.column();
    let spacing = plane.spacing();
    let dimensions = plane.dimensions();
    let numbers = [
        position.x,
        position.y,
        position.z,
        row.x,
        row.y,
        row.z,
        column.x,
        column.y,
        column.z,
        spacing.row_mm(),
        spacing.column_mm(),
    ];
    let mut out = String::new();
    for number in numbers {
        out.push_str(&format!("{number:e},"));
    }
    out.push_str(&format!("{},{}", dimensions.rows(), dimensions.columns()));
    out
}

/// File meta and data set rows for the headers table.
///
/// Pixel data is reported as a byte count. Other values are the stored text.
/// The caller must not write the JSON to a log.
///
/// # Errors
///
/// Returns [`PreviewError::NotPart10`] when the bytes are not a Part 10 object.
pub fn headers_json(bytes: &[u8]) -> Result<String, PreviewError> {
    let parsed = parse_part10(bytes).map_err(part10_error)?;
    let object = parsed.object();
    let dict = StandardDataDictionary;
    let mut rows = Vec::new();
    for element in object.meta().to_element_iter() {
        if capped(&mut rows) {
            break;
        }
        push_row(
            &mut rows,
            &dict,
            element.tag(),
            element.vr(),
            "",
            meta_text(&element),
        );
    }
    for element in object.iter() {
        push_dataset(&mut rows, &dict, element, "", 0);
    }
    Ok(rows_json(&rows))
}

/// The F-225 conformance report of one Part 10 object, as JSON.
///
/// One parse and no pixel decode. The report is `ocelli-dicom`'s own
/// serialisation and carries no value apart from a SOP Class UID under the
/// DICOM root. A finding never stops a frame from opening, because nothing
/// on the open or preview path calls this.
///
/// # Errors
///
/// Returns [`PreviewError::NotPart10`] when the bytes are not a Part 10
/// object, the same refusal [`headers_json`] makes.
pub fn conformance_json(bytes: &[u8]) -> Result<String, PreviewError> {
    let parsed = parse_part10(bytes).map_err(part10_error)?;
    Ok(check_conformance(&parsed).to_json())
}

struct HeaderRow {
    tag: String,
    vr: String,
    name: String,
    value: String,
}

fn text_or_empty(object: &DefaultDicomObject, tag: Tag) -> Result<String, PreviewError> {
    Ok(optional_text(object, tag)?.unwrap_or_default())
}

fn optional_u32(object: &DefaultDicomObject, tag: Tag) -> Result<u32, PreviewError> {
    let Ok(element) = object.element(tag) else {
        return Ok(0);
    };
    let value = element
        .to_int::<u16>()
        .map_err(|_| PreviewError::Attribute)?;
    Ok(u32::from(value))
}

pub(crate) fn push_field(out: &mut String, name: &str, value: &str, first: bool) {
    if !first {
        out.push(',');
    }
    out.push('"');
    out.push_str(name);
    out.push_str("\":");
    json_string(out, value);
}

fn push_number(out: &mut String, name: &str, value: u32) {
    out.push_str(",\"");
    out.push_str(name);
    out.push_str("\":");
    out.push_str(&value.to_string());
}

pub(crate) fn json_string(out: &mut String, text: &str) {
    out.push('"');
    for ch in text.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            control if control.is_control() => {
                out.push_str(&format!("\\u{code:04x}", code = u32::from(control)));
            }
            other => out.push(other),
        }
    }
    out.push('"');
}

fn rows_json(rows: &[HeaderRow]) -> String {
    let mut out = String::from("[");
    for (index, row) in rows.iter().enumerate() {
        if index > 0 {
            out.push(',');
        }
        out.push_str("{\"tag\":");
        json_string(&mut out, &row.tag);
        out.push_str(",\"vr\":");
        json_string(&mut out, &row.vr);
        out.push_str(",\"name\":");
        json_string(&mut out, &row.name);
        out.push_str(",\"value\":");
        json_string(&mut out, &row.value);
        out.push('}');
    }
    out.push(']');
    out
}

fn push_dataset(
    rows: &mut Vec<HeaderRow>,
    dict: &StandardDataDictionary,
    element: &InMemElement<StandardDataDictionary>,
    prefix: &str,
    depth: u32,
) {
    if capped(rows) {
        return;
    }
    let shown = format!("{prefix}{}", tag_text(element.tag()));
    push_row(
        rows,
        dict,
        element.tag(),
        element.vr(),
        &shown,
        dataset_text(element),
    );
    if depth >= 6 {
        return;
    }
    let Some(items) = element.value().items() else {
        return;
    };
    for (index, item) in items.iter().enumerate() {
        let next = format!("{shown}[{index}].");
        for child in item.iter() {
            push_dataset(rows, dict, child, &next, depth + 1);
        }
    }
}

fn push_row(
    rows: &mut Vec<HeaderRow>,
    dict: &StandardDataDictionary,
    tag: Tag,
    vr: VR,
    shown: &str,
    value: String,
) {
    if capped(rows) {
        return;
    }
    let label = if shown.is_empty() {
        tag_text(tag)
    } else {
        shown.to_owned()
    };
    rows.push(HeaderRow {
        tag: label,
        vr: VR::to_string(vr).to_owned(),
        name: dict
            .by_tag(tag)
            .map(|entry| entry.alias().to_owned())
            .unwrap_or_default(),
        value: clip_text(&value),
    });
}

fn capped(rows: &mut Vec<HeaderRow>) -> bool {
    if rows.len() < MAX_HEADER_ROWS {
        return false;
    }
    let already = rows
        .last()
        .is_some_and(|row| row.value == "further elements omitted");
    if !already {
        rows.push(HeaderRow {
            tag: String::new(),
            vr: String::new(),
            name: String::new(),
            value: "further elements omitted".to_owned(),
        });
    }
    true
}

fn tag_text(tag: Tag) -> String {
    format!(
        "({group:04X},{element:04X})",
        group = tag.group(),
        element = tag.element()
    )
}

fn clip_text(text: &str) -> String {
    if text.chars().count() <= VALUE_CAP {
        return text.to_owned();
    }
    let mut kept: String = text.chars().take(VALUE_CAP).collect();
    kept.push_str(" [truncated]");
    kept
}

fn meta_text(element: &DataElement<EmptyObject, [u8; 0]>) -> String {
    if element.tag() == PIXEL_DATA {
        return primitive_note(element.value(), true);
    }
    if let Some(note) = bulk_if_large(element.vr(), element.value()) {
        return note;
    }
    match element.to_str() {
        Ok(text) => text.into_owned(),
        Err(_) => primitive_note(element.value(), true),
    }
}

fn dataset_text(element: &InMemElement<StandardDataDictionary>) -> String {
    if element.tag() == PIXEL_DATA {
        return pixel_note(element);
    }
    if element.vr() == VR::SQ {
        let count = element
            .value()
            .items()
            .map(|items| items.len())
            .unwrap_or(0);
        return format!("{count} items");
    }
    if let DicomValue::Primitive(value) = element.value()
        && let Some(note) = bulk_if_large_primitive(element.vr(), value)
    {
        return note;
    }
    match element.to_str() {
        Ok(text) => text.into_owned(),
        Err(_) => pixel_note(element),
    }
}

fn bulk_if_large(vr: VR, value: &DicomValue<EmptyObject, [u8; 0]>) -> Option<String> {
    let DicomValue::Primitive(primitive) = value else {
        return None;
    };
    bulk_if_large_primitive(vr, primitive)
}

fn bulk_if_large_primitive(vr: VR, value: &PrimitiveValue) -> Option<String> {
    match vr {
        VR::OB | VR::OW | VR::OF | VR::OD | VR::OL | VR::OV | VR::UN => {
            let bytes = primitive_len(value)?;
            if bytes > BULK_TEXT_CAP {
                Some(format!("{bytes} bytes"))
            } else {
                None
            }
        }
        _ => None,
    }
}

fn primitive_note(value: &DicomValue<EmptyObject, [u8; 0]>, force: bool) -> String {
    let DicomValue::Primitive(primitive) = value else {
        return "0 bytes".to_owned();
    };
    match primitive_len(primitive) {
        Some(bytes) if force || bytes > BULK_TEXT_CAP => format!("{bytes} bytes"),
        Some(_) => primitive.to_str().into_owned(),
        None => "value omitted".to_owned(),
    }
}

fn pixel_note(element: &InMemElement<StandardDataDictionary>) -> String {
    if let Some(fragments) = element.value().fragments() {
        let mut total = 0_usize;
        for fragment in fragments {
            match total.checked_add(fragment.len()) {
                Some(sum) => total = sum,
                None => return "byte length overflowed".to_owned(),
            }
        }
        return format!("{total} bytes");
    }
    if let DicomValue::Primitive(value) = element.value() {
        match primitive_len(value) {
            Some(bytes) => format!("{bytes} bytes"),
            None => "value omitted".to_owned(),
        }
    } else {
        "0 bytes".to_owned()
    }
}

fn primitive_len(value: &PrimitiveValue) -> Option<usize> {
    match value {
        PrimitiveValue::Empty => Some(0),
        PrimitiveValue::U8(items) => Some(items.len()),
        PrimitiveValue::U16(items) => items.len().checked_mul(2),
        PrimitiveValue::I16(items) => items.len().checked_mul(2),
        PrimitiveValue::U32(items) => items.len().checked_mul(4),
        PrimitiveValue::I32(items) => items.len().checked_mul(4),
        PrimitiveValue::F32(items) => items.len().checked_mul(4),
        PrimitiveValue::U64(items) => items.len().checked_mul(8),
        PrimitiveValue::I64(items) => items.len().checked_mul(8),
        PrimitiveValue::F64(items) => items.len().checked_mul(8),
        PrimitiveValue::Str(_)
        | PrimitiveValue::Strs(_)
        | PrimitiveValue::Tags(_)
        | PrimitiveValue::Date(_)
        | PrimitiveValue::DateTime(_)
        | PrimitiveValue::Time(_) => None,
    }
}

#[cfg(test)]
fn file_order_pixels(object: &DefaultDicomObject, little: bool) -> Result<Vec<u8>, ()> {
    let element = object.element(PIXEL_DATA).map_err(|_| ())?;
    match element.value() {
        DicomValue::Primitive(PrimitiveValue::U8(items)) => Ok(items.to_vec()),
        DicomValue::Primitive(PrimitiveValue::U16(items)) => {
            let mut out = Vec::new();
            for word in items {
                let encoded = if little {
                    word.to_le_bytes()
                } else {
                    word.to_be_bytes()
                };
                out.extend_from_slice(&encoded);
            }
            Ok(out)
        }
        DicomValue::Primitive(_) | DicomValue::Sequence(_) | DicomValue::PixelSequence(_) => {
            Err(())
        }
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::{
        borrowed_decode_ticks, borrowed_pixels_fit, file_order_pixels, inspect_post, inspect_work,
        per_frame_replies, preview_frame, split_pixel_payload,
    };

    fn element(group: u16, element: u16, vr: &[u8; 2], value: &[u8]) -> Vec<u8> {
        let mut out = Vec::new();
        out.extend_from_slice(&group.to_le_bytes());
        out.extend_from_slice(&element.to_le_bytes());
        out.extend_from_slice(vr);
        if matches!(vr, b"SQ" | b"OW" | b"OB") {
            out.extend_from_slice(&[0, 0]);
            out.extend_from_slice(&u32::try_from(value.len()).unwrap_or(0).to_le_bytes());
        } else {
            out.extend_from_slice(&u16::try_from(value.len()).unwrap_or(0).to_le_bytes());
        }
        out.extend_from_slice(value);
        out
    }

    fn padded(text: &str) -> Vec<u8> {
        let mut bytes = text.as_bytes().to_vec();
        if bytes.len() % 2 == 1 {
            bytes.push(b' ');
        }
        bytes
    }

    fn item(content: &[u8]) -> Vec<u8> {
        let mut out = vec![0xFE, 0xFF, 0x00, 0xE0];
        out.extend_from_slice(&u32::try_from(content.len()).unwrap_or(0).to_le_bytes());
        out.extend_from_slice(content);
        out
    }

    /// An N-frame 1 by 1 enhanced CT whose every frame carries its own
    /// Frame VOI LUT item, centre 40 and width 400.
    pub(crate) fn per_frame_file(frames: usize) -> Vec<u8> {
        per_frame_file_as(frames, "2.25.213000")
    }

    /// [`per_frame_file`] with SOP Instance UID `uid`, which
    /// [`inspect_work`] counts by.
    pub(crate) fn per_frame_file_as(frames: usize, uid: &str) -> Vec<u8> {
        let mut meta = Vec::new();
        meta.extend(element(0x0002, 0x0001, b"OB", &[0, 1]));
        meta.extend(element(0x0002, 0x0010, b"UI", b"1.2.840.10008.1.2.1\0"));
        let mut bytes = vec![0_u8; 128];
        bytes.extend_from_slice(b"DICM");
        bytes.extend(element(
            0x0002,
            0x0000,
            b"UL",
            &u32::try_from(meta.len()).unwrap_or(0).to_le_bytes(),
        ));
        bytes.extend(meta);
        let voi = item(&element(
            0x0028,
            0x9132,
            b"SQ",
            &item(
                &[
                    element(0x0028, 0x1050, b"DS", &padded("40")),
                    element(0x0028, 0x1051, b"DS", &padded("400")),
                ]
                .concat(),
            ),
        ));
        let mut sop = uid.as_bytes().to_vec();
        if sop.len() % 2 == 1 {
            sop.push(0);
        }
        bytes.extend(element(0x0008, 0x0018, b"UI", &sop));
        bytes.extend(element(0x0008, 0x0060, b"CS", b"CT"));
        bytes.extend(element(0x0028, 0x0002, b"US", &1_u16.to_le_bytes()));
        bytes.extend(element(0x0028, 0x0004, b"CS", &padded("MONOCHROME2")));
        bytes.extend(element(0x0028, 0x0008, b"IS", &padded(&frames.to_string())));
        for (tag, value) in [
            (0x0010, 1_u16),
            (0x0011, 1),
            (0x0100, 16),
            (0x0101, 16),
            (0x0102, 15),
            (0x0103, 0),
        ] {
            bytes.extend(element(0x0028, tag, b"US", &value.to_le_bytes()));
        }
        bytes.extend(element(0x5200, 0x9230, b"SQ", &voi.repeat(frames)));
        bytes.extend(element(
            0x7FE0,
            0x0010,
            b"OW",
            &50_u16.to_le_bytes().repeat(frames),
        ));
        bytes
    }

    #[test]
    fn kept_replies_are_charged_at_their_heap_bytes() -> Result<(), String> {
        // Review pass 2, S1, and pass 3. The charge is every string's
        // capacity plus the frame list's slots, nothing guessed. The kept
        // replies also equal what `inspect_json` writes for each frame.
        let file = per_frame_file(3);
        let kept = per_frame_replies(&file, usize::MAX).ok_or("nothing was kept")?;
        let mut want =
            kept.parts.head.capacity() + kept.parts.middle.capacity() + kept.parts.tail.capacity();
        for (facts, windows) in &kept.frames {
            want += facts.capacity() + windows.capacity();
        }
        want += kept.frames.capacity() * std::mem::size_of::<(String, String)>();
        if kept.bytes() != want {
            return Err(format!("charged {} for {want} heap bytes", kept.bytes()));
        }
        for frame in 0..3 {
            let body = inspect_json(&file, frame).map_err(|_| "inspect refused".to_owned())?;
            if kept.reply(frame).as_deref() != Some(body.as_str()) {
                return Err(format!(
                    "frame {frame}'s kept reply differs from the body form"
                ));
            }
        }
        // N4. A one-frame file never asks by id, so it keeps nothing even
        // when its one frame's window is per frame.
        if per_frame_replies(&per_frame_file(1), usize::MAX).is_some() {
            return Err("a one-frame file kept replies".to_owned());
        }
        // N4, the cap inside the builder is what bounds a huge file's work.
        if per_frame_replies(&file, kept.bytes() - 1).is_some() {
            return Err("replies past the cap were built".to_owned());
        }
        Ok(())
    }

    #[test]
    fn per_frame_replies_build_the_shared_parts_once() -> Result<(), String> {
        // Review pass 3, D1, and pass 4, D1. One post, the reply and the kept
        // set together, projects the metadata once and builds the plane list
        // once, so the work does not grow with the frame count beyond the
        // per-frame window and spacing fields.
        let mut work = Vec::new();
        for frames in [4_usize, 12] {
            let uid = format!("2.25.21300{frames}");
            let (reply, kept) = inspect_post(&per_frame_file_as(frames, &uid), 0, usize::MAX);
            if reply.is_err() || kept.is_none() {
                return Err(format!("{frames} frames gave no reply or kept nothing"));
            }
            work.push(inspect_work(&uid));
        }
        if !matches!(work[..], [(1, 1), (1, 1)]) {
            return Err(format!(
                "projections and plane builds by frame count: {work:?}"
            ));
        }
        Ok(())
    }
    use crate::{FrameRequest, inspect_json, slice};
    use ocelli_viewport::presentation::WindowPreset;

    #[test]
    fn inspect_drops_the_pixel_payload() -> Result<(), String> {
        let mut file = slice(0).ok_or("synthetic slice 0 was absent")?;
        let pixels = 96_usize.saturating_mul(128).saturating_mul(2);
        file.extend_from_slice(&0xFFFC_u16.to_le_bytes());
        file.extend_from_slice(&0xFFFC_u16.to_le_bytes());
        file.extend_from_slice(b"OB");
        file.extend_from_slice(&0_u16.to_le_bytes());
        file.extend_from_slice(&2_u32.to_le_bytes());
        file.extend_from_slice(&[0, 0]);
        let (stripped, range) = split_pixel_payload(&file).ok_or("the pixel payload was kept")?;
        if range.len() != pixels {
            return Err(format!(
                "the borrowed pixel span was {} for {pixels} bytes",
                range.len()
            ));
        }
        if stripped.len().saturating_add(pixels) > file.len() {
            return Err(format!(
                "stripped {} bytes of a {} byte file",
                stripped.len(),
                file.len()
            ));
        }
        let marker = [0xFC_u8, 0xFF, 0xFC, 0xFF];
        if !stripped
            .windows(marker.len())
            .any(|window| window == marker)
        {
            return Err("the element after the pixels was dropped".to_owned());
        }
        let json = inspect_json(&file, 0).map_err(|_| "inspect refused the file".to_owned())?;
        if !json.contains("\"monochrome\":true") {
            return Err(json);
        }
        Ok(())
    }

    #[test]
    fn a_native_frame_is_read_from_the_file_slice() -> Result<(), String> {
        // Body at column 64 row 48 on slice 0. Stored 1064, intercept -1024,
        // so the modality value is 40. Soft tissue is LINEAR center 40 width
        // 400. PS3.3 C.11.2.1.2 gives grey 128, the same figure the preset
        // fixture pins. A decode of the emptied header would not.
        let file = slice(0).ok_or("synthetic slice 0 was absent")?;
        let before = borrowed_decode_ticks();
        let preview = preview_frame(
            &file,
            FrameRequest {
                window_preset: Some(WindowPreset::SoftTissue),
                ..FrameRequest::default()
            },
        )
        .map_err(|_| "the synthetic slice was refused".to_owned())?;
        let index = 48_usize.saturating_mul(128).saturating_add(64);
        let grey = preview
            .grey
            .get(index)
            .copied()
            .ok_or("the body pixel was absent")?;
        if grey != 128 {
            return Err(format!("the body pixel was grey {grey}"));
        }
        if borrowed_decode_ticks() == before {
            return Err("the frame was decoded from a parsed copy".to_owned());
        }
        Ok(())
    }

    #[test]
    fn a_deflated_syntax_is_not_walked() -> Result<(), String> {
        let mut file = slice(0).ok_or("synthetic slice 0 was absent")?;
        let marker = b"1.2.840.10008.1.2.1";
        let at = file
            .windows(marker.len())
            .position(|window| window == marker)
            .ok_or("the explicit little endian uid was absent")?;
        // Same length as the explicit little endian UID, so the element length
        // stays valid. This UID is not one of the three uncompressed syntaxes.
        let stand_in = b"1.2.840.10008.1.2.5";
        let Some(slot) = file.get_mut(at..at + marker.len()) else {
            return Err("the transfer syntax slot was short".to_owned());
        };
        slot.copy_from_slice(stand_in);
        if split_pixel_payload(&file).is_some() {
            return Err("a non-allowlisted syntax was walked".to_owned());
        }
        Ok(())
    }

    #[test]
    fn a_pixel_length_past_the_file_is_not_borrowed() -> Result<(), String> {
        let file = slice(0).ok_or("synthetic slice 0 was absent")?;
        let (_, range) = split_pixel_payload(&file).ok_or("the native frame was not walked")?;
        let mut short = file;
        short.truncate(range.end.saturating_sub(1));
        if split_pixel_payload(&short).is_some() {
            return Err("a range past the file was borrowed".to_owned());
        }
        Ok(())
    }

    #[test]
    fn a_short_pixel_value_does_not_fit_the_header() -> Result<(), String> {
        let file = slice(0).ok_or("synthetic slice 0 was absent")?;
        let (header, range) =
            split_pixel_payload(&file).ok_or("the native frame was not walked")?;
        let parsed =
            ocelli_dicom::parse_part10(&header).map_err(|_| "the stripped header did not parse")?;
        let Some(pixels) = file.get(range.clone()) else {
            return Err("the native frame range was not in the file".to_owned());
        };
        if !borrowed_pixels_fit(parsed.object(), pixels) {
            return Err("the native frame did not fit its header".to_owned());
        }
        let short = file
            .get(range.start..range.end.saturating_sub(2))
            .ok_or("the shortened pixel value was absent")?;
        if borrowed_pixels_fit(parsed.object(), short) {
            return Err("a short pixel value fitted the header".to_owned());
        }
        Ok(())
    }

    #[test]
    fn borrowed_bytes_match_the_parsed_pixels_for_every_accepted_corpus_file() -> Result<(), String>
    {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let data = root.join("corpus/data");
        let manifest_path = root.join("corpus/manifest.tsv");
        if !data.is_dir() {
            return Ok(());
        }
        let manifest = std::fs::read_to_string(&manifest_path)
            .map_err(|_| "tracked corpus manifest is unreadable".to_owned())?;
        let mut accepted = 0_usize;
        for (row_index, line) in manifest.lines().skip(1).enumerate() {
            if line.trim().is_empty() {
                continue;
            }
            let row = row_index + 2;
            let mut columns = line.split('\t');
            let relative = columns
                .next()
                .ok_or_else(|| format!("manifest row {row} has no path"))?;
            let _modality = columns
                .next()
                .ok_or_else(|| format!("manifest row {row} has no modality"))?;
            let syntax = columns
                .next()
                .ok_or_else(|| format!("manifest row {row} has no transfer syntax"))?
                .trim();
            let bytes = std::fs::read(data.join(relative))
                .map_err(|_| format!("corpus file for manifest row {row} is unreadable"))?;
            let walked = split_pixel_payload(&bytes);
            let allowed = matches!(
                syntax,
                "1.2.840.10008.1.2" | "1.2.840.10008.1.2.1" | "1.2.840.10008.1.2.2"
            );
            if !allowed {
                if walked.is_some() {
                    return Err(format!(
                        "manifest row {row} was walked under a syntax the allowlist excludes"
                    ));
                }
                continue;
            }
            let Some((_, range)) = walked else {
                continue;
            };
            let header = ocelli_dicom::parse_part10(&bytes)
                .map_err(|_| format!("manifest row {row} was walked but the file did not parse"))?;
            let little = syntax != "1.2.840.10008.1.2.2";
            let parsed_pixels = file_order_pixels(header.object(), little)
                .map_err(|_| format!("manifest row {row} pixel value could not be read"))?;
            let Some(borrowed) = bytes.get(range) else {
                return Err(format!("manifest row {row} borrowed a range past the file"));
            };
            // Rows, columns, samples, bits allocated and frames name the
            // expected length. A different length is not an acceptance. The
            // preview path parses the whole file instead.
            if !borrowed_pixels_fit(header.object(), borrowed) {
                continue;
            }
            if borrowed != parsed_pixels.as_slice() {
                return Err(format!(
                    "manifest row {row} borrowed pixels differ from the parsed value"
                ));
            }
            accepted = accepted.saturating_add(1);
        }
        if accepted == 0 {
            return Err("the corpus had no file the walker accepts".to_owned());
        }
        Ok(())
    }
}
