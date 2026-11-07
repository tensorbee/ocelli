//! Monochrome frames of one Part 10 object.
//!
//! Windowed samples go through `LutChain`. The grey byte is
//! `display_index`, the same address the colour table uses.
//! A window override replaces the file window with another call of that chain.
//! A Modality LUT Sequence is refused, because this preview does not apply one.
//! A VOI LUT Sequence is refused unless that override replaces it.
//! Enhanced multi-frame rescale and window values come from the functional
//! groups, with the top-level attributes as the fallback.
//! Invert flips the display byte after the chain.

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
    DecodePhotometricInterpretation, FrameDesc, FrameDescInput, PixelDataVr, PixelRepresentation,
    RawDecoder, Registry, register_htj2k_decoders, register_jpeg_decoders,
    register_jpeg2000_decoders, register_jpegls_decoders, register_jpegxl_decoders,
    register_native_and_rle_decoders,
};
use ocelli_core::Stored;
use ocelli_dicom::{
    DispatchPath, EncapsulatedFrameIndex, FrameGeometry, FunctionalGroupSource, MetadataElement,
    MetadataSet, MetadataValue, MultiframeMetadata, ParseError, TopLevelFallback,
    parse_decimal_string, parse_part10,
};
use ocelli_pixel::{
    ByteOrder, ImageDimensions, ImagePlane, LutChain, ModalityTransform, PhotometricInterpretation,
    PresentationLutEvidence, PresentationLutShape, SampleLayout, StoredBits,
    StoredPixelDescription, StoredSampleTable, VoiFunction, VoiTransform,
};
use ocelli_render::display_index;
use ocelli_viewport::presentation::WindowPreset;
use ocelli_viewport::view::f32_from_f64;

use crate::corners::{
    age_text, dicom_date_text, dicom_time_text, exposure_mas, patient_orientation_letters,
    person_name_text,
};
use crate::measure::{CalibrationInputs, FactorInput, SpacingInput, choose_spacing};
use crate::value::{ValueFacts, value_facts};

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

/// One grey frame ready to draw.
#[derive(Debug, Clone, PartialEq)]
pub struct Preview {
    /// Columns.
    pub width: u32,
    /// Rows.
    pub height: u32,
    /// Number of Frames, or 1 when that attribute is absent.
    pub frames: u32,
    /// The file had no window, so the grey bytes use the modality range.
    pub full_range: bool,
    /// One display byte per pixel, row major.
    pub grey: Vec<u8>,
    /// Zero-based frame that was decoded.
    pub frame: u32,
    /// Window center that was applied, in modality units.
    pub window_center: f32,
    /// Window width that was applied.
    pub window_width: f32,
    /// VOI LUT Function that was applied.
    pub window_function: &'static str,
    /// The display bytes were flipped after the LUT chain.
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
    /// numeric window or the full range.
    pub window_index: Option<u32>,
    /// Number of Window Center and Window Width pairs the file gives this
    /// frame, 0 when it gives none or they cannot be paired.
    pub window_count: u32,
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
}

/// Which frame to decode, and an optional replacement window.
///
/// Absent window fields keep the file window, including the full-range
/// fallback. Invert is applied to the grey byte after `display_index`.
#[derive(Debug, Clone, Copy, Default)]
pub struct FrameRequest {
    /// Zero-based frame index.
    pub frame: u32,
    /// Flip each grey byte as `255 - byte`.
    pub invert: bool,
    /// Replacement Window Center. Set together with [`Self::window_width`].
    pub window_center: Option<f32>,
    /// Replacement Window Width.
    pub window_width: Option<f32>,
    /// Replacement VOI LUT Function. Absent keeps the file function.
    pub window_function: Option<VoiFunction>,
    /// Named window from `WindowPreset`. It refuses a VOI LUT Sequence
    /// and it is not combined with a numeric window.
    pub window_preset: Option<WindowPreset>,
    /// Zero-based index into the file's Window Center and Window Width
    /// pairs. Absent is index 0. It is not combined with a preset or a
    /// numeric window, and it refuses a VOI LUT Sequence.
    pub window_index: Option<u32>,
}

impl PreviewError {
    /// A stable token for the page. The sentence in [`std::fmt::Display`] is the one a person reads.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::NotPart10 => "not-part10",
            Self::Attribute => "attribute",
            Self::NotMonochrome => "not-monochrome",
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
    /// The frame is not monochrome.
    #[error("this preview shows monochrome frames only")]
    NotMonochrome,
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
        &mut |_| Vec::new(),
    )?;
    let mut scratch = Vec::new();
    let mut grey = Vec::new();
    render_held(&held, request, &mut scratch, &mut grey)
}

/// Refuse a window header that cannot name one window.
///
/// # Errors
///
/// Returns [`PreviewError::Window`] when center and width are not paired,
/// a preset is combined with a numeric window or a function, a file window
/// index is combined with a preset or a numeric window, or a number is not
/// finite.
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
/// The samples are decoder output. Grey bytes are not stored, because a
/// window change has to run `LutChain` again.
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
    /// Full-range center and width when the file has no window. Otherwise an error.
    pub(crate) full_range: Result<(f32, f32), PreviewError>,
    /// Request-independent inputs of the value at a point, or the refusal
    /// that blocks every point. [`render_held`] does not read it. Boxed, so
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
    /// The bytes the cache charges for this frame.
    ///
    /// The capacity of the sample buffer, the struct itself, and the heap
    /// bytes of the value facts when the frame has them.
    pub(crate) fn charged_bytes(&self) -> usize {
        let value = self.value.as_ref().map_or(0, |facts| facts.heap_bytes());
        self.samples
            .capacity()
            .saturating_add(size_of::<Self>())
            .saturating_add(value)
    }
}

/// Chain inputs that do not depend on the request.
pub(crate) struct Recipe {
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
    /// No Window Center or Window Width. LINEAR_EXACT over the modality range.
    Full,
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
    A: FnMut(usize) -> Vec<u8>,
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
    if samples != 1 {
        return Err(PreviewError::NotMonochrome);
    }
    let bits_allocated = required_u16(object, BITS_ALLOCATED)?;
    let bits_stored = required_u16(object, BITS_STORED)?;
    let high_bit = required_u16(object, HIGH_BIT)?;
    let representation = pixel_representation(object)?;
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
    let layout =
        SampleLayout::new(1, photometric, None).map_err(|_| PreviewError::NotMonochrome)?;
    let dimensions = ImageDimensions::new(height, width).map_err(|_| PreviewError::Attribute)?;
    let description = StoredPixelDescription::new(dimensions, layout, stored_bits);
    let count = description
        .sample_count()
        .map_err(|_| PreviewError::Attribute)?;
    let bytes_per_sample = usize::from(stored_bits.bits_allocated() / 8);
    let expected = count
        .checked_mul(bytes_per_sample)
        .ok_or(PreviewError::Decode)?;
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
) -> HeldFrame {
    let prepared = prepare_held(object, &parts.description, &samples, frame_index);
    HeldFrame {
        samples,
        description: parts.description,
        byte_order: ByteOrder::LittleEndian,
        recipe: prepared.recipe,
        facts: prepared.facts,
        full_range: prepared.full_range,
        value: prepared.value,
        calibration: prepared.calibration,
        frames: parts.frames,
        width: parts.width,
        height: parts.height,
    }
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
    A: FnMut(usize) -> Vec<u8>,
{
    let object = parsed.object();
    let (parts, desc) = frame_parts(object, frame)?;
    if desc.output_len() != parts.expected {
        return Err(PreviewError::Decode);
    }
    let frame_index = usize::try_from(frame).map_err(|_| PreviewError::Frame)?;
    prepare_samples(samples, parts.expected, acquire)?;
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
    Ok(held_from_samples(object, parts, taken, frame_index))
}

/// Size the sample buffer to the decoded frame.
///
/// A buffer from `acquire` is used when it already has room for the frame.
/// `resize` then writes every byte, so a recycled buffer does not keep its
/// previous contents.
fn prepare_samples<A>(
    samples: &mut Vec<u8>,
    output_len: usize,
    acquire: &mut A,
) -> Result<(), PreviewError>
where
    A: FnMut(usize) -> Vec<u8>,
{
    if samples.capacity() < output_len {
        let got = acquire(output_len);
        if got.capacity() >= output_len {
            *samples = got;
        }
    }
    samples.clear();
    samples.resize(output_len, 0);
    if samples.len() != output_len {
        return Err(PreviewError::Decode);
    }
    Ok(())
}

struct Prepared {
    recipe: Result<Recipe, PreviewError>,
    facts: Result<FrameFacts, PreviewError>,
    full_range: Result<(f32, f32), PreviewError>,
    value: Result<Box<ValueFacts>, PreviewError>,
    calibration: Result<CalibrationInputs, PreviewError>,
}

fn prepare_held(
    object: &DefaultDicomObject,
    description: &StoredPixelDescription,
    samples: &[u8],
    frame_index: usize,
) -> Prepared {
    let photometric = description.layout.photometric_interpretation();
    let metadata = match project_metadata(object) {
        Ok(metadata) => metadata,
        Err(error) => {
            return Prepared {
                recipe: Err(error),
                facts: Err(error),
                full_range: Err(error),
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
                full_range: Err(PreviewError::Attribute),
                value: Err(PreviewError::Attribute),
                calibration: Err(PreviewError::Attribute),
            };
        }
    };
    let facts = frame_facts(&metadata, &multi, frame_index);
    // Spacing is not stage 1, so a refused rescale below never refuses a length.
    let calibration = calibration_inputs(&metadata, &multi, frame_index);
    if let Err(error) = modality_blocked(&multi, frame_index) {
        return Prepared {
            recipe: Err(error),
            facts,
            full_range: Err(error),
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
                full_range: Err(error),
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
                full_range: Err(PreviewError::Window),
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
    let (file_window, full_range) =
        file_window_of(&multi, &modality, description, samples, frame_index);
    let ct = optional_text(object, MODALITY).ok().flatten().as_deref() == Some("CT");
    Prepared {
        recipe: Ok(Recipe {
            modality,
            photometric,
            evidence,
            voi_lut,
            file_window,
            ct,
        }),
        facts,
        full_range,
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

fn file_window_of(
    multi: &MultiframeMetadata<'_>,
    modality: &ModalityTransform,
    description: &StoredPixelDescription,
    samples: &[u8],
    frame_index: usize,
) -> (
    Result<FileWindow, PreviewError>,
    Result<(f32, f32), PreviewError>,
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
        return match full_range(modality, &stored) {
            Ok(pair) => (Ok(FileWindow::Full), Ok(pair)),
            Err(error) => (Err(error), Err(error)),
        };
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
    grey: &mut Vec<u8>,
) -> Result<Preview, PreviewError> {
    let recipe = match &held.recipe {
        Ok(recipe) => recipe,
        Err(error) => return Err(*error),
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
    let grey = std::mem::take(grey);
    Ok(Preview {
        width: held.width,
        height: held.height,
        frames: held.frames,
        full_range: window.full_range,
        grey,
        frame: request.frame,
        window_center: center,
        window_width: width,
        window_function: function_label(function),
        inverted: request.invert,
        facts,
        calibration_source,
        calibration_mm,
        window_index: window.index,
        window_count: window_count(recipe),
    })
}

/// The pairs the file gives this frame, or 0 when it gives none or its
/// window cannot be used.
fn window_count(recipe: &Recipe) -> u32 {
    match &recipe.file_window {
        Ok(FileWindow::Explicit { centers, .. }) => {
            u32::try_from(centers.len()).unwrap_or(u32::MAX)
        }
        Ok(FileWindow::Full) | Err(_) => 0,
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
    full_range: bool,
    /// The file window pair drawn, or `None` for a preset, a numeric window
    /// or the full range.
    index: Option<u32>,
}

impl ChosenWindow {
    fn one(center: f32, width: f32, function: VoiFunction, full_range: bool) -> Self {
        Self {
            centers: vec![center],
            widths: vec![width],
            selected: 0,
            function,
            full_range,
            index: None,
        }
    }
}

fn select_window(
    held: &HeldFrame,
    recipe: &Recipe,
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
            false,
        ));
    }
    if request.window_center.is_some() && request.window_width.is_some() {
        let center = request.window_center.ok_or(PreviewError::Window)?;
        let width = request.window_width.ok_or(PreviewError::Window)?;
        if !center.is_finite() || !width.is_finite() {
            return Err(PreviewError::Window);
        }
        let function = request.window_function.unwrap_or(VoiFunction::Linear);
        return Ok(ChosenWindow::one(center, width, function, false));
    }
    if voi_present(recipe)? {
        return Err(PreviewError::Window);
    }
    match recipe.file_window.as_ref().map_err(|error| *error)? {
        FileWindow::Full => {
            // An index names a stored pair, and this file stores none.
            if request.window_index.is_some() {
                return Err(PreviewError::Window);
            }
            let (center, width) = held.full_range?;
            // A function sent alone replaces the full-range default. The
            // default is LINEAR_EXACT only when the request names none.
            let function = request.window_function.unwrap_or(VoiFunction::LinearExact);
            Ok(ChosenWindow::one(center, width, function, true))
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
            let index = request.window_index.unwrap_or(0);
            let selected = usize::try_from(index).map_err(|_| PreviewError::Window)?;
            // An empty slot has no number to draw. It is refused here rather
            // than skipped, so the pairs after it keep their positions.
            if !matches!(
                (centers.get(selected), widths.get(selected)),
                (Some(Some(_)), Some(Some(_)))
            ) {
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
                full_range: false,
                index: Some(index),
            })
        }
    }
}

fn voi_present(recipe: &Recipe) -> Result<bool, PreviewError> {
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
        _ => Err(PreviewError::NotMonochrome),
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
        // XYB is three samples, so this preview refuses it as not-monochrome
        // before decode. The arm exists so the match stays exhaustive.
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
    let Ok(rows) = required_u16(object, ROWS) else {
        return false;
    };
    let Ok(columns) = required_u16(object, COLUMNS) else {
        return false;
    };
    let Ok(samples) = required_u16(object, SAMPLES_PER_PIXEL) else {
        return false;
    };
    let Ok(bits) = required_u16(object, BITS_ALLOCATED) else {
        return false;
    };
    let Ok(frames) = frame_count(object) else {
        return false;
    };
    if bits == 0 || !bits.is_multiple_of(8) {
        return false;
    }
    let Some(bytes_per) = usize::from(bits / 8).checked_mul(usize::from(samples)) else {
        return false;
    };
    let Some(one) = usize::from(rows)
        .checked_mul(usize::from(columns))
        .and_then(|count| count.checked_mul(bytes_per))
    else {
        return false;
    };
    let Ok(frames) = usize::try_from(frames) else {
        return false;
    };
    let Some(all) = one.checked_mul(frames) else {
        return false;
    };
    all > 0 && pixels.len() == all
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
    if decoded.len() != desc.output_len() {
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
            decode_encapsulated(parsed, element, desc, frame_count, frame_index, decoded)?;
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
    element: &InMemElement<StandardDataDictionary>,
    desc: &FrameDesc,
    frame_count: usize,
    frame_index: usize,
    decoded: &mut [u8],
) -> Result<(), PreviewError> {
    let offsets = element.value().offset_table().ok_or(PreviewError::Decode)?;
    let fragments = element.value().fragments().ok_or(PreviewError::Decode)?;
    let borrowed: Vec<&[u8]> = fragments.iter().map(Vec::as_slice).collect();
    let index = EncapsulatedFrameIndex::from_basic(frame_count, offsets, &borrowed)
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
    if produced != DecodePhotometricInterpretation::Preserved {
        return Err(PreviewError::NotMonochrome);
    }
    registry
        .decode(uid, &encoded, desc, decoded)
        .map_err(|_| PreviewError::Decode)
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
    project_elements(object.iter())
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

fn full_range(modality: &ModalityTransform, stored: &[Stored]) -> Result<(f32, f32), PreviewError> {
    let mut low: Option<f32> = None;
    let mut high: Option<f32> = None;
    for sample in stored {
        let value = modality.apply(*sample).0;
        if !value.is_finite() {
            return Err(PreviewError::Window);
        }
        low = Some(match low {
            Some(held) if held < value => held,
            Some(_) | None => value,
        });
        high = Some(match high {
            Some(held) if held > value => held,
            Some(_) | None => value,
        });
    }
    let (Some(low), Some(high)) = (low, high) else {
        return Err(PreviewError::Window);
    };
    let span = high - low;
    if !span.is_finite() {
        return Err(PreviewError::Window);
    }
    if span.to_bits() == 0.0_f32.to_bits() {
        return Ok((low, 1.0));
    }
    let center = (low + high) / 2.0;
    if !center.is_finite() {
        return Err(PreviewError::Window);
    }
    Ok((center, span))
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
    Ok(FrameFacts {
        row_spacing,
        column_spacing,
        spacing_source,
        edge_left,
        edge_right,
        edge_top,
        edge_bottom,
        edge_source,
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

fn inspect_facts(object: &DefaultDicomObject, frame: usize) -> FrameFacts {
    let Ok(metadata) = project_metadata(object) else {
        return FrameFacts::empty();
    };
    let Ok(multi) = MultiframeMetadata::new(&metadata) else {
        return FrameFacts::empty();
    };
    frame_facts(&metadata, &multi, frame).unwrap_or_else(|_| FrameFacts::empty())
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
    let object = parsed.object();
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
    let frame_index = usize::try_from(frame).map_err(|_| PreviewError::Frame)?;
    if frames > 0 && frame_index >= usize::try_from(frames).unwrap_or(usize::MAX) {
        return Err(PreviewError::Frame);
    }
    let facts = inspect_facts(object, frame_index);
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
    push_json_token(&mut out, "rowSpacing", facts.row_spacing.as_deref());
    push_json_token(&mut out, "columnSpacing", facts.column_spacing.as_deref());
    push_field(&mut out, "spacingSource", facts.spacing_source, false);
    push_field(&mut out, "edgeLeft", &facts.edge_left, false);
    push_field(&mut out, "edgeRight", &facts.edge_right, false);
    push_field(&mut out, "edgeTop", &facts.edge_top, false);
    push_field(&mut out, "edgeBottom", &facts.edge_bottom, false);
    push_field(&mut out, "edgeSource", facts.edge_source, false);
    push_field(
        &mut out,
        "frameOfReferenceUid",
        &sendable_uid(optional_text(object, FRAME_OF_REFERENCE).ok().flatten()),
        false,
    );
    push_corner_fields(&mut out, object);
    push_window_fields(&mut out, object, frame_index);
    out.push_str(",\"planes\":[");
    for (index, token) in plane_tokens(object, frames).iter().enumerate() {
        if index > 0 {
            out.push(',');
        }
        match token {
            Some(text) => json_string(&mut out, text),
            None => out.push_str("null"),
        }
    }
    out.push_str("]}");
    Ok(out)
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
fn push_window_fields(out: &mut String, object: &DefaultDicomObject, frame: usize) {
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
    let listed = project_metadata(object)
        .ok()
        .and_then(|metadata| window_rows(&metadata, frame));
    let (rows, luts, per_frame) = listed.unwrap_or_default();
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
    out.push_str("],\"windowsPerFrame\":");
    out.push_str(if per_frame { "true" } else { "false" });
}

/// One listed file window.
struct WindowRow {
    /// The stored Window Center as a JSON number, or `None` for an empty slot.
    center: Option<String>,
    width: Option<String>,
    explanation: String,
    label: String,
}

/// The rows, the VOI LUT explanations and the per-frame flag, or `None`
/// when the functional groups or an attribute cannot be read.
fn window_rows(
    metadata: &MetadataSet,
    frame: usize,
) -> Option<(Vec<WindowRow>, Vec<String>, bool)> {
    let multi = MultiframeMetadata::new(metadata).ok()?;
    let centers = resolved_slots(&multi, frame, FRAME_VOI_LUT_SEQUENCE, WINDOW_CENTER).ok()?;
    let widths = resolved_slots(&multi, frame, FRAME_VOI_LUT_SEQUENCE, WINDOW_WIDTH).ok()?;
    let explanations =
        resolved_slots(&multi, frame, FRAME_VOI_LUT_SEQUENCE, WINDOW_EXPLANATION).ok()?;
    let per_frame = matches!(
        multi
            .resolve(
                frame,
                FRAME_VOI_LUT_SEQUENCE,
                WINDOW_CENTER,
                TopLevelFallback::Allowed
            )
            .ok()?
            .map(|found| found.source()),
        Some(FunctionalGroupSource::PerFrame(_))
    );
    let blank = |slots: &[String]| slots.iter().all(String::is_empty);
    let mut rows = Vec::new();
    // Pairs that cannot be paired, or that carry no number at all, are not
    // a list of windows. The preview refuses or full-ranges those files.
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
    Some((rows, luts, per_frame))
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
fn plane_tokens(object: &DefaultDicomObject, frames: u32) -> Vec<Option<String>> {
    let count = usize::try_from(frames).map_or(FRAME_LIST_CAP, |count| count.min(FRAME_LIST_CAP));
    let Ok(metadata) = project_metadata(object) else {
        return vec![None; count];
    };
    let Ok(multi) = MultiframeMetadata::new(&metadata) else {
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
mod tests {
    use super::{
        borrowed_decode_ticks, borrowed_pixels_fit, file_order_pixels, preview_frame,
        split_pixel_payload,
    };
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
