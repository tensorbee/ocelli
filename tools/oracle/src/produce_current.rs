//! Current-view candidate producer.
//!
//! One Ocelli image for every stack row and volume orientation the reference
//! run rendered. Colour, palette and YBR are written and not scored. A view
//! this process cannot build is omitted. Omission is absence, not a pass.
//! The sprint profile records the candidate gate. Pre-push requires
//! `--require-comparison`.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use dicom_core::PrimitiveValue;
use ocelli_cache::DecodedFrame;
use ocelli_codec::DecodePhotometricInterpretation;
use ocelli_core::{Pt, Stored, World};
use ocelli_dicom::{DispatchPath, Tag, parse_part10};
use ocelli_pixel::{
    ByteOrder, ColorTransform, DecodedLayout, DecodedPhotometric, ImageDimensions,
    ImageOrientationPatient, ImagePlane, ImagePositionPatient, LutDescriptor, PaletteColorLut,
    PhotometricInterpretation, PixelDataEncoding, PixelRepresentation, PlanarConfiguration,
    SampleLayout, StoredBits, StoredPixelDescription,
};
use ocelli_render::{CpuRaster, RasterColor, RasterGrid, RasterQuality};
use ocelli_viewport::view::{View, f32_from_f64};
use ocelli_volume::{
    FrameOfReferenceUid, ObliquePlane, OrthoImage, PlaneSample, SliceEvidence, Volume,
    VolumeCandidate, VolumeError,
};
use serde_json::{Map, Value};

use crate::sidecar::{Run, Sidecar, ViewKind};

use super::RowStop;

const RED_DESCRIPTOR: Tag = Tag(0x0028, 0x1101);
const GREEN_DESCRIPTOR: Tag = Tag(0x0028, 0x1102);
const BLUE_DESCRIPTOR: Tag = Tag(0x0028, 0x1103);
const RED_DATA: Tag = Tag(0x0028, 0x1201);
const GREEN_DATA: Tag = Tag(0x0028, 0x1202);
const BLUE_DATA: Tag = Tag(0x0028, 0x1203);

/// Cornerstone `MINIMUM_SLAB_THICKNESS`. The null slab is this thick.
/// The mapper takes one sample on the camera side of the focal plane.
/// It does not accumulate along the slab.
const REFERENCE_SLAB_MM: f64 = 0.05;

/// Cornerstone `MAXIMUM_RAY_DISTANCE`. Parallel clipping is this pair,
/// and the screen quad sits on the near plane. Float32 cancellation of
/// this magnitude against the slab plane is what places the sample.
const REFERENCE_RAY_LIMIT_MM: f64 = 1.0e6;

/// Divisor for a pixel the volume quad only partly covers.
/// A pixel whose four samples are all inside keeps its centre colour.
const REFERENCE_COVERAGE_SAMPLES: u32 = 4;

/// The same four offsets the stack raster uses, measured from the integer
/// pixel the volume plane samples. They are not CSS pixel-centre offsets.
const REFERENCE_COVERAGE_OFFSETS: [(f64, f64); 4] = [
    (-0.125, -0.375),
    (0.375, -0.125),
    (-0.375, 0.125),
    (0.125, 0.375),
];

/// Directories `compare produce` reads and writes.
///
/// `candidate_run` must not be the reference run and must not contain it.
pub struct CurrentPaths {
    /// Reference-half directory containing `run.json`.
    pub reference_run: PathBuf,
    /// Permitted corpus root. Manifest paths are joined under it.
    pub corpus_root: PathBuf,
    /// Ocelli candidate frames for the current reference views.
    pub candidate_run: PathBuf,
}

/// Counts for one current-view production.
///
/// Identifiers stay out of [`Debug`]. [`Self::reason`] reads one of them.
#[derive(Clone, PartialEq, Eq)]
pub struct CurrentSummary {
    /// `ran`, or the refusal that stopped the entry before a commit.
    pub resources: &'static str,
    /// Views whose `.raw` and sidecar were committed.
    pub written: usize,
    /// Declared views that were not committed.
    pub omitted: usize,
    /// Reference rows with `ok: false`. They stay in `run.json`.
    pub unsupported: usize,
    cases: Vec<CurrentCase>,
}

#[derive(Clone, PartialEq, Eq)]
struct CurrentCase {
    id: String,
    reason: &'static str,
    wrote: bool,
}

struct Prepared {
    id: String,
    rgba: Vec<u8>,
    digest: String,
    sidecar: Value,
}

struct PixelLoad {
    bytes: Vec<u8>,
    decoded_rgb: bool,
    encoding: PixelDataEncoding,
    palette: Option<PaletteColorLut>,
}

struct BuiltVolume {
    volume: Volume,
    slope: f64,
    intercept: f64,
    photometric: String,
}

/// Patient plane sampled once per canvas pixel.
///
/// The origin is the centre of pixel `(0, 0)`. PS3.3 C.7.6.2.1.1.
struct PatientPlane {
    oblique: ObliquePlane,
    origin: [f64; 3],
    row: [f64; 3],
    column: [f64; 3],
}

/// Right-handed view basis. The origin of view space is the camera position.
struct LookBasis {
    x_axis: [f64; 3],
    y_axis: [f64; 3],
    z_axis: [f64; 3],
    eye: [f64; 3],
}

/// One slab plane in view space, already narrowed to the uniform precision.
struct SlabPlane {
    normal: [f32; 3],
    distance: f32,
}

/// Float32 ray and the constant slab shift for one parallel reformat.
struct ReferenceCamera {
    toward_camera: [f64; 3],
    shift: f64,
    focal: [f64; 3],
    basis: LookBasis,
    /// Rows of `vecVCToIS`. A column vector is `row · v`.
    vec_vc_to_is: [[f32; 3]; 3],
    origin_vc: [f32; 3],
    near_z: f32,
    planes: [SlabPlane; 2],
    dims: [u32; 3],
    dims_f32: [f32; 3],
}

struct RayHit {
    tmin: f32,
    /// Mapper travel after the volume box and the two slab planes.
    travel: f32,
    texel: [u32; 3],
}

struct QuadBounds {
    left: f64,
    right: f64,
    top: f64,
    bottom: f64,
}

enum Emission {
    Mono,
    Colour,
    Palette,
    Ybr,
    Skip,
}

impl CurrentSummary {
    /// Static reason for one view identifier.
    #[must_use]
    pub fn reason(&self, id: &str) -> Option<&str> {
        self.cases
            .iter()
            .find(|case| case.id == id)
            .map(|case| case.reason)
    }

    /// Whether that view's image was committed.
    #[must_use]
    pub fn wrote(&self, id: &str) -> bool {
        self.cases.iter().any(|case| case.id == id && case.wrote)
    }

    fn reason_counts(&self) -> BTreeMap<&str, usize> {
        let mut counts = BTreeMap::new();
        for case in &self.cases {
            *counts.entry(case.reason).or_insert(0) += 1;
        }
        counts
    }
}

impl std::fmt::Debug for CurrentSummary {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("CurrentSummary")
            .field("resources", &self.resources)
            .field("written", &self.written)
            .field("omitted", &self.omitted)
            .field("unsupported", &self.unsupported)
            .field("reasons", &self.reason_counts())
            .finish()
    }
}

/// Write the current-view candidate directory.
///
/// `run.json` is written only after every committed `.raw` and sidecar.
/// A refusal leaves any previous candidate directory in place and does not
/// publish a partial `run.json`.
#[must_use]
pub fn produce_current(paths: &CurrentPaths) -> CurrentSummary {
    if !paths.reference_run.join("run.json").is_file() {
        return refused("reference-run-absent");
    }
    if directories_conflict(&paths.reference_run, &paths.candidate_run) {
        return refused("output-directory-refused");
    }
    if !paths.corpus_root.is_dir() {
        return refused("corpus-absent");
    }
    let run = match Run::load(&paths.reference_run) {
        Ok(run) => run,
        Err(_) => return refused("reference-run-unreadable"),
    };
    let Some(staging) = staging_path(&paths.candidate_run) else {
        return refused("output-directory-refused");
    };
    if directories_conflict(&paths.reference_run, &staging) {
        return refused("output-directory-refused");
    }
    if reset_directory(&staging).is_err() {
        return refused("candidate-write-refused");
    }
    match prepare_and_commit(paths, &run, &staging) {
        Ok(summary) => summary,
        Err(()) => {
            abort_directory(&staging);
            refused("candidate-write-refused")
        }
    }
}

fn refused(resources: &'static str) -> CurrentSummary {
    CurrentSummary {
        resources,
        written: 0,
        omitted: 0,
        unsupported: 0,
        cases: Vec::new(),
    }
}

fn prepare_and_commit(
    paths: &CurrentPaths,
    run: &Run,
    staging: &Path,
) -> Result<CurrentSummary, ()> {
    let mut prepared = Vec::new();
    let mut cases = Vec::new();
    render_stacks(paths, run, &mut prepared, &mut cases);
    render_volumes(paths, run, &mut prepared, &mut cases);
    let digests = digests_of(&prepared);
    let json = rewrite_run(&run.json, &digests);
    for frame in &prepared {
        write_frame(staging, frame)?;
    }
    let text = serde_json::to_vec_pretty(&json).map_err(|_| ())?;
    fs::write(staging.join("run.json"), text).map_err(|_| ())?;
    replace_directory(staging, &paths.candidate_run)?;
    let written = prepared.len();
    let omitted = run.views.len().saturating_sub(written);
    Ok(CurrentSummary {
        resources: "ran",
        written,
        omitted,
        unsupported: unsupported_rows(&json),
        cases,
    })
}

fn render_stacks(
    paths: &CurrentPaths,
    run: &Run,
    prepared: &mut Vec<Prepared>,
    cases: &mut Vec<CurrentCase>,
) {
    let mut ids: Vec<&str> = run
        .views
        .values()
        .filter(|view| view.kind == ViewKind::Stack)
        .map(|view| view.id.as_str())
        .collect();
    ids.sort_unstable();
    for id in ids {
        let Some(sidecar) = run.sidecars.get(id) else {
            omit(cases, id, "sidecar-absent");
            continue;
        };
        match render_stack(&paths.corpus_root, sidecar) {
            Ok((rgba, reason)) => push_prepared(prepared, cases, id, rgba, &sidecar.json, reason),
            Err(reason) => omit(cases, id, reason),
        }
    }
}

fn render_volumes(
    paths: &CurrentPaths,
    run: &Run,
    prepared: &mut Vec<Prepared>,
    cases: &mut Vec<CurrentCase>,
) {
    let Some(volumes) = run.json.pointer("/volumes").and_then(Value::as_array) else {
        return;
    };
    for volume in volumes {
        if volume.get("outcome").and_then(Value::as_str) == Some("refused-as-declared") {
            continue;
        }
        let Some(frames) = volume.get("frames").and_then(Value::as_array) else {
            continue;
        };
        let ids: Vec<&str> = frames
            .iter()
            .filter_map(|frame| frame.get("id").and_then(Value::as_str))
            .collect();
        let built = {
            let first = ids.first().and_then(|id| run.sidecars.get(*id));
            if !first.is_some_and(|sidecar| members_are_written(sidecar, prepared)) {
                for id in &ids {
                    omit(cases, id, "volume-member-omitted");
                }
                continue;
            }
            first.map(|sidecar| build_volume(&paths.corpus_root, run, sidecar))
        };
        for id in ids {
            let Some(sidecar) = run.sidecars.get(id) else {
                omit(cases, id, "sidecar-absent");
                continue;
            };
            if !members_are_written(sidecar, prepared) {
                omit(cases, id, "volume-member-omitted");
                continue;
            }
            let result = match &built {
                Some(Ok(built)) => render_reformat(built, sidecar),
                Some(Err(reason)) => Err(*reason),
                None => Err("volume-sidecar-absent"),
            };
            match result {
                Ok(rgba) => push_prepared(prepared, cases, id, rgba, &sidecar.json, "mono"),
                Err(reason) => omit(cases, id, reason),
            }
        }
    }
}

fn render_stack(corpus: &Path, sidecar: &Sidecar) -> Result<(Vec<u8>, &'static str), &'static str> {
    match emission(&sidecar.json) {
        Emission::Skip => Err("photometric-unsupported"),
        Emission::Mono => render_mono_stack(corpus, sidecar).map(|rgba| (rgba, "mono")),
        Emission::Colour => render_colour(corpus, sidecar).map(|rgba| (rgba, "colour-not-scored")),
        Emission::Palette => {
            render_colour(corpus, sidecar).map(|rgba| (rgba, "palette-not-scored"))
        }
        Emission::Ybr => render_colour(corpus, sidecar).map(|rgba| (rgba, "ybr-not-scored")),
    }
}

fn emission(sidecar: &Value) -> Emission {
    match super::stack_selection_reason(sidecar) {
        Some("palette-not-scored") => Emission::Palette,
        Some("ybr-not-scored") => Emission::Ybr,
        Some("colour-not-scored") => Emission::Colour,
        Some("class-not-monochrome") | None => Emission::Mono,
        Some(_) => Emission::Skip,
    }
}

fn render_mono_stack(corpus: &Path, sidecar: &Sidecar) -> Result<Vec<u8>, &'static str> {
    super::render_row(corpus, sidecar, None)
        .map(|(rgba, _, _)| rgba)
        .map_err(stop_reason)
}

fn render_colour(corpus: &Path, sidecar: &Sidecar) -> Result<Vec<u8>, &'static str> {
    let load = load_colour(corpus, sidecar)?;
    let description = colour_description(sidecar, load.decoded_rgb)?;
    let stored = unpack_described(&load.bytes, &description)?;
    let transform = ColorTransform::resolve(
        description.layout,
        load.encoding,
        if load.decoded_rgb {
            DecodedPhotometric::Rgb
        } else {
            DecodedPhotometric::Preserved
        },
        if load.decoded_rgb || matches!(load.encoding, PixelDataEncoding::Encapsulated) {
            DecodedLayout::Interleaved
        } else {
            DecodedLayout::Preserved
        },
        load.palette,
    )
    .map_err(|_| "colour-transform-refused")?;
    let (rows, columns) = sidecar.source_grid().map_err(|_| "pixel-layout-refused")?;
    raster_colour(sidecar, &transform, &stored, columns, rows)
}

fn load_colour(corpus: &Path, sidecar: &Sidecar) -> Result<PixelLoad, &'static str> {
    let bytes = super::frame_container(corpus, sidecar, true).map_err(stop_reason)?;
    let relative = sidecar
        .json
        .pointer("/row/path")
        .and_then(Value::as_str)
        .ok_or("corpus-path-refused")?;
    let file = super::open_corpus(corpus, relative).map_err(stop_reason)?;
    let parsed = parse_part10(&file).map_err(|_| "dicom-parse-refused")?;
    let decoded_rgb = reported_rgb(&parsed, sidecar)?;
    let encoding = match parsed.transfer_syntax().dispatch_path() {
        DispatchPath::Encapsulated => PixelDataEncoding::Encapsulated,
        DispatchPath::ImplicitVrLittleEndian
        | DispatchPath::ExplicitVrLittleEndian
        | DispatchPath::ExplicitVrBigEndian
        | DispatchPath::DeflatedExplicitVrLittleEndian => PixelDataEncoding::Native,
    };
    let palette = if decoded_rgb {
        None
    } else {
        palette_from(&parsed, sidecar)?
    };
    Ok(PixelLoad {
        bytes,
        decoded_rgb,
        encoding,
        palette,
    })
}

fn reported_rgb(
    parsed: &ocelli_dicom::ParsedDicom,
    sidecar: &Sidecar,
) -> Result<bool, &'static str> {
    if !matches!(
        parsed.transfer_syntax().dispatch_path(),
        DispatchPath::Encapsulated
    ) {
        return Ok(false);
    }
    let facts = super::pixel_facts(sidecar).map_err(stop_reason)?;
    let desc = super::frame_desc(sidecar, &facts).map_err(stop_reason)?;
    let registry = super::codecs()?;
    Ok(
        registry.decode_photometric_interpretation(parsed.transfer_syntax().uid(), &desc)
            == Ok(DecodePhotometricInterpretation::Rgb),
    )
}

fn colour_description(
    sidecar: &Sidecar,
    decoded_rgb: bool,
) -> Result<StoredPixelDescription, &'static str> {
    let facts = super::pixel_facts(sidecar).map_err(stop_reason)?;
    let (photometric, samples, planar) = if decoded_rgb {
        (
            PhotometricInterpretation::Rgb,
            3,
            Some(PlanarConfiguration::Interleaved),
        )
    } else {
        let photometric = photometric_named(photometric_name(sidecar)?)?;
        let samples = colour_samples(sidecar, photometric)?;
        let planar = colour_planar(sidecar, samples)?;
        (photometric, samples, planar)
    };
    let layout =
        SampleLayout::new(samples, photometric, planar).map_err(|_| "pixel-layout-refused")?;
    let representation = match facts.representation {
        0 => PixelRepresentation::Unsigned,
        _ => PixelRepresentation::Signed,
    };
    let bits = StoredBits::new(
        facts.bits_allocated,
        facts.bits_stored,
        facts.high_bit,
        representation,
    )
    .map_err(|_| "pixel-layout-refused")?;
    let dimensions =
        ImageDimensions::new(facts.rows, facts.columns).map_err(|_| "pixel-layout-refused")?;
    Ok(StoredPixelDescription::new(dimensions, layout, bits))
}

fn unpack_described(
    bytes: &[u8],
    description: &StoredPixelDescription,
) -> Result<Vec<Stored>, &'static str> {
    let count = description
        .sample_count()
        .map_err(|_| "pixel-data-length")?;
    let mut stored = vec![Stored(0.0); count];
    description
        .unpack(bytes, ByteOrder::LittleEndian, &mut stored)
        .map_err(|_| "pixel-data-length")?;
    Ok(stored)
}

fn photometric_name(sidecar: &Sidecar) -> Result<&str, &'static str> {
    sidecar
        .json
        .pointer("/attributes/photometricInterpretation")
        .and_then(Value::as_str)
        .ok_or("photometric-unsupported")
}

fn photometric_named(name: &str) -> Result<PhotometricInterpretation, &'static str> {
    match name {
        "MONOCHROME1" => Ok(PhotometricInterpretation::Monochrome1),
        "MONOCHROME2" => Ok(PhotometricInterpretation::Monochrome2),
        "PALETTE COLOR" => Ok(PhotometricInterpretation::PaletteColor),
        "RGB" => Ok(PhotometricInterpretation::Rgb),
        "YBR_FULL" => Ok(PhotometricInterpretation::YbrFull),
        "YBR_FULL_422" => Ok(PhotometricInterpretation::YbrFull422),
        "YBR_PARTIAL_422" => Ok(PhotometricInterpretation::YbrPartial422),
        "YBR_ICT" => Ok(PhotometricInterpretation::YbrIct),
        "YBR_RCT" => Ok(PhotometricInterpretation::YbrRct),
        _ => Err("photometric-unsupported"),
    }
}

fn colour_samples(
    sidecar: &Sidecar,
    photometric: PhotometricInterpretation,
) -> Result<u8, &'static str> {
    let samples = match photometric {
        PhotometricInterpretation::Monochrome1
        | PhotometricInterpretation::Monochrome2
        | PhotometricInterpretation::PaletteColor => 1_u16,
        _ => super::json_u16(sidecar, "/attributes/samplesPerPixel").unwrap_or(3),
    };
    u8::try_from(samples).map_err(|_| "pixel-layout-refused")
}

fn colour_planar(
    sidecar: &Sidecar,
    samples: u8,
) -> Result<Option<PlanarConfiguration>, &'static str> {
    if samples == 1 {
        return Ok(None);
    }
    match sidecar
        .json
        .pointer("/attributes/planarConfiguration")
        .and_then(Value::as_u64)
    {
        None | Some(0) => Ok(Some(PlanarConfiguration::Interleaved)),
        Some(1) => Ok(Some(PlanarConfiguration::Planar)),
        Some(_) => Err("pixel-layout-refused"),
    }
}

fn palette_from(
    parsed: &ocelli_dicom::ParsedDicom,
    sidecar: &Sidecar,
) -> Result<Option<PaletteColorLut>, &'static str> {
    if photometric_name(sidecar)? != "PALETTE COLOR" {
        return Ok(None);
    }
    let red = read_lut(parsed, RED_DESCRIPTOR, RED_DATA)?;
    let green = read_lut(parsed, GREEN_DESCRIPTOR, GREEN_DATA)?;
    let blue = read_lut(parsed, BLUE_DESCRIPTOR, BLUE_DATA)?;
    PaletteColorLut::new(red, green, blue)
        .map(Some)
        .map_err(|_| "palette-refused")
}

fn read_lut(
    parsed: &ocelli_dicom::ParsedDicom,
    descriptor: Tag,
    data: Tag,
) -> Result<LutDescriptor, &'static str> {
    let descriptor = element_primitive(parsed, descriptor)?;
    let [count, first, bits] = three_integers(descriptor)?;
    let entry_count = entry_count(count)?;
    let first_mapped = i32::try_from(first).map_err(|_| "palette-refused")?;
    let bits = u8::try_from(bits).map_err(|_| "palette-refused")?;
    let resolved = if entry_count == 0 {
        65_536
    } else {
        usize::from(entry_count)
    };
    let values = lut_values(element_primitive(parsed, data)?, resolved)?;
    LutDescriptor::new(entry_count, first_mapped, bits, values).map_err(|_| "palette-refused")
}

fn element_primitive(
    parsed: &ocelli_dicom::ParsedDicom,
    tag: Tag,
) -> Result<&PrimitiveValue, &'static str> {
    parsed
        .object()
        .element(tag)
        .map_err(|_| "palette-refused")?
        .value()
        .primitive()
        .ok_or("palette-refused")
}

fn three_integers(primitive: &PrimitiveValue) -> Result<[i64; 3], &'static str> {
    let values = signed_integers(primitive)?;
    if values.len() != 3 {
        return Err("palette-refused");
    }
    Ok([
        *values.first().ok_or("palette-refused")?,
        *values.get(1).ok_or("palette-refused")?,
        *values.get(2).ok_or("palette-refused")?,
    ])
}

fn entry_count(value: i64) -> Result<u16, &'static str> {
    if value == 0 {
        return Ok(0);
    }
    u16::try_from(value).map_err(|_| "palette-refused")
}

fn lut_values(primitive: &PrimitiveValue, count: usize) -> Result<Vec<f32>, &'static str> {
    let words = match primitive {
        PrimitiveValue::U16(values) => values.iter().copied().map(u32::from).collect(),
        PrimitiveValue::I16(values) => {
            let mut words = Vec::new();
            for value in values {
                let word = u32::try_from(i32::from(*value)).map_err(|_| "palette-refused")?;
                words.push(word);
            }
            words
        }
        PrimitiveValue::U8(bytes) => words_from_bytes(bytes, count)?,
        _ => return Err("palette-refused"),
    };
    if words.len() != count {
        return Err("palette-refused");
    }
    let mut values = Vec::new();
    values
        .try_reserve(words.len())
        .map_err(|_| "palette-refused")?;
    for word in words {
        values.push(f32_from_f64(f64::from(word)));
    }
    Ok(values)
}

fn words_from_bytes(bytes: &[u8], count: usize) -> Result<Vec<u32>, &'static str> {
    if bytes.len() == count {
        return Ok(bytes.iter().copied().map(u32::from).collect());
    }
    let expected = count.checked_mul(2).ok_or("palette-refused")?;
    if bytes.len() != expected {
        return Err("palette-refused");
    }
    let mut words = Vec::new();
    words.try_reserve(count).map_err(|_| "palette-refused")?;
    for pair in bytes.chunks_exact(2) {
        let low = u16::from(*pair.first().ok_or("palette-refused")?);
        let high = u16::from(*pair.get(1).ok_or("palette-refused")?);
        words.push(u32::from(low | (high << 8)));
    }
    Ok(words)
}

fn signed_integers(primitive: &PrimitiveValue) -> Result<Vec<i64>, &'static str> {
    match primitive {
        PrimitiveValue::U8(values) => Ok(values.iter().copied().map(i64::from).collect()),
        PrimitiveValue::U16(values) => Ok(values.iter().copied().map(i64::from).collect()),
        PrimitiveValue::I16(values) => Ok(values.iter().copied().map(i64::from).collect()),
        PrimitiveValue::I32(values) => Ok(values.iter().copied().map(i64::from).collect()),
        PrimitiveValue::U32(values) => Ok(values.iter().copied().map(i64::from).collect()),
        _ => Err("palette-refused"),
    }
}

/// A volume frame is loadable only when every member stack sidecar was
/// written into the same candidate directory. `Run::load` resolves the
/// reformat's categories from those sidecars and refuses a missing one.
fn members_are_written(sidecar: &Sidecar, prepared: &[Prepared]) -> bool {
    let Some(members) = sidecar
        .json
        .pointer("/volume/members")
        .and_then(Value::as_array)
    else {
        return false;
    };
    if members.is_empty() {
        return false;
    }
    members.iter().all(|member| {
        let Some(name) = member.get("stackSidecar").and_then(Value::as_str) else {
            return false;
        };
        let stem = name.strip_suffix(".json").unwrap_or(name);
        prepared.iter().any(|frame| frame.id == stem)
    })
}

fn build_volume(corpus: &Path, run: &Run, sidecar: &Sidecar) -> Result<BuiltVolume, &'static str> {
    let members = sidecar
        .json
        .pointer("/volume/members")
        .and_then(Value::as_array)
        .ok_or("volume-members-absent")?;
    let mut slices = Vec::new();
    let mut slope = None;
    let mut intercept = None;
    let mut photometric = None;
    for member in members {
        let name = member
            .get("stackSidecar")
            .and_then(Value::as_str)
            .ok_or("volume-member-absent")?;
        let stack = member_sidecar(run, name)?;
        if slope.is_none() {
            slope = stack.json.pointer("/image/slope").and_then(super::json_f64);
            intercept = stack
                .json
                .pointer("/image/intercept")
                .and_then(super::json_f64);
            photometric = stack
                .json
                .pointer("/attributes/photometricInterpretation")
                .and_then(Value::as_str)
                .map(str::to_owned);
        }
        slices.push(SliceEvidence::new(decoded_slice(corpus, &stack)?));
    }
    let orientation = slices
        .first()
        .ok_or("volume-members-absent")?
        .frame()
        .plane()
        .orientation;
    let uid = sidecar
        .json
        .pointer("/volume/id")
        .and_then(Value::as_str)
        .unwrap_or(sidecar.id.as_str());
    let candidate = VolumeCandidate::new(FrameOfReferenceUid(uid.to_owned()), orientation, slices);
    // `Volume::construct` keeps the 0.5 percent gap rule. cornerstone3D 5.8.2
    // places slice k at the endpoint grid and does not look at an interior
    // gap. The file positions are what the constructor refuses. This grid is
    // the one the reference rendered.
    let volume = match Volume::construct(candidate.clone()) {
        Ok(volume) => volume,
        Err(VolumeError::NonUniformSpacing { .. }) => {
            let moved = endpoint_grid(&candidate.slices)?;
            Volume::construct(VolumeCandidate::new(
                candidate.frame_of_reference.clone(),
                candidate.orientation,
                moved,
            ))
            .map_err(volume_reason)?
        }
        Err(error) => return Err(volume_reason(error)),
    };
    Ok(BuiltVolume {
        volume,
        slope: slope.ok_or("missing-rescale")?,
        intercept: intercept.ok_or("missing-rescale")?,
        photometric: photometric.ok_or("photometric-unsupported")?,
    })
}

/// Place each slice on the line through the first and last positions.
///
/// Spacing is `|d_last - d_first| / (n - 1)`, the endpoint span. Slice k
/// keeps the samples it arrived with and moves to `origin + k * spacing`
/// along the slice normal. A zero span stays a refusal. The constructor
/// then sees a uniform series.
fn endpoint_grid(slices: &[SliceEvidence]) -> Result<Vec<SliceEvidence>, &'static str> {
    let count = slices.len();
    let steps = u32::try_from(count.saturating_sub(1)).unwrap_or(0);
    if steps == 0 {
        return Err("volume-spacing-refused");
    }
    let normal = slices
        .first()
        .ok_or("volume-members-absent")?
        .frame()
        .plane()
        .slice_normal();
    let mut ranked = Vec::with_capacity(count);
    for (arrival, evidence) in slices.iter().enumerate() {
        let distance = evidence.frame().plane().position_vector().dot(normal);
        ranked.push((arrival, distance));
    }
    ranked.sort_by(|left, right| {
        left.1
            .partial_cmp(&right.1)
            .unwrap_or(core::cmp::Ordering::Equal)
    });
    let first_arrival = ranked.first().map(|ranked| ranked.0).unwrap_or(0);
    let last_arrival = ranked.last().map(|ranked| ranked.0).unwrap_or(0);
    let origin = slices
        .get(first_arrival)
        .ok_or("volume-members-absent")?
        .frame()
        .plane()
        .position_vector();
    let end = slices
        .get(last_arrival)
        .ok_or("volume-members-absent")?
        .frame()
        .plane()
        .position_vector()
        .dot(normal);
    let span = end - origin.dot(normal);
    let spacing = span / f64::from(steps);
    if !spacing.is_finite() || spacing.abs() <= 1.0e-9 {
        return Err("volume-spacing-refused");
    }
    let mut placed = Vec::with_capacity(count);
    for (step, (arrival, _)) in ranked.iter().copied().enumerate() {
        let frame = slices.get(arrival).ok_or("volume-members-absent")?.frame();
        let plane = frame.plane();
        let shift = spacing * f64::from(u32::try_from(step).unwrap_or(u32::MAX));
        let position = ImagePositionPatient::new([
            origin.x + normal.x * shift,
            origin.y + normal.y * shift,
            origin.z + normal.z * shift,
        ])
        .map_err(|_| "volume-spacing-refused")?;
        let moved = ImagePlane::new(
            position,
            plane.orientation,
            plane.spacing(),
            plane.dimensions(),
        )
        .map_err(|_| "volume-spacing-refused")?;
        placed.push(SliceEvidence::new(DecodedFrame::new(
            frame.description(),
            moved,
            frame.samples().to_vec(),
        )));
    }
    Ok(placed)
}

fn volume_reason(error: VolumeError) -> &'static str {
    match error {
        VolumeError::NonUniformSpacing { .. } => "volume-spacing-refused",
        _ => "volume-construct-refused",
    }
}

fn member_sidecar(run: &Run, name: &str) -> Result<Sidecar, &'static str> {
    let id = name.strip_suffix(".json").unwrap_or(name);
    if let Some(sidecar) = run.sidecars.get(id) {
        return Ok(sidecar.clone());
    }
    let path = run.directory.join(name);
    let text = fs::read(&path).map_err(|_| "volume-member-absent")?;
    let json: Value = serde_json::from_slice(&text).map_err(|_| "volume-member-absent")?;
    let kind = json
        .pointer("/kind")
        .and_then(Value::as_str)
        .ok_or("volume-member-absent")?;
    let kind = ViewKind::parse(kind).map_err(|_| "volume-member-absent")?;
    Ok(Sidecar {
        id: id.to_owned(),
        kind,
        json,
    })
}

fn decoded_slice(corpus: &Path, sidecar: &Sidecar) -> Result<DecodedFrame, &'static str> {
    let bytes = super::frame_container(corpus, sidecar, false).map_err(stop_reason)?;
    let facts = super::pixel_facts(sidecar).map_err(stop_reason)?;
    let photometric = super::photometric_of(sidecar).map_err(stop_reason)?;
    let layout = SampleLayout::new(1, photometric, None).map_err(|_| "pixel-layout-refused")?;
    let representation = match facts.representation {
        0 => PixelRepresentation::Unsigned,
        _ => PixelRepresentation::Signed,
    };
    let bits = StoredBits::new(
        facts.bits_allocated,
        facts.bits_stored,
        facts.high_bit,
        representation,
    )
    .map_err(|_| "pixel-layout-refused")?;
    let dimensions =
        ImageDimensions::new(facts.rows, facts.columns).map_err(|_| "pixel-layout-refused")?;
    let description = StoredPixelDescription::new(dimensions, layout, bits);
    let (column_mm, row_mm) = super::spacing_mm(&sidecar.json, facts.rows).map_err(stop_reason)?;
    let plane = super::image_plane(&sidecar.json, facts.rows, facts.columns, row_mm, column_mm)
        .map_err(stop_reason)?;
    Ok(DecodedFrame::new(description, plane, bytes))
}

/// One nearest reformat, shifted by the reference ray, then quad coverage.
///
/// The shift is the float32 slab entry along the view normal.
/// `Volume::reslice_nearest` reads that plane. A ray miss is absent.
/// A face hit reads that voxel. The displayed byte is the thin composite
/// of the 1024 colour table. The quad blend runs after that colour.
fn render_reformat(built: &BuiltVolume, sidecar: &Sidecar) -> Result<Vec<u8>, &'static str> {
    let (width, height) = sidecar.frame_size().map_err(|_| "canvas-extent")?;
    let plane = patient_plane(sidecar, width, height)?;
    let camera = reference_camera(sidecar, &built.volume)?;
    let shifted = offset_along_normal(&plane, camera.toward_camera, camera.shift)?;
    let mut image = built
        .volume
        .reslice_nearest(shifted.oblique)
        .map_err(|_| "volume-reslice-refused")?;
    apply_reference_ray(&mut image, &built.volume, &plane, &camera)?;
    let (stored, absent) = plane_samples(&image)?;
    let synthetic = synthetic_sidecar(sidecar, built, &plane, &image)?;
    let view = stack_view(&synthetic)?;
    let mut rgba = draw_mono(&synthetic, &view, &stored, image.width, image.height)?;
    paint_reference_tone(
        &mut rgba,
        sidecar,
        built,
        &plane,
        &camera,
        &view,
        SampleGrid {
            stored: &stored,
            absent: &absent,
            columns: image.width,
            rows: image.height,
        },
    )?;
    paint_absent(&view, &absent, image.width, image.height, &mut rgba);
    cover_reference_quad(&mut rgba, &built.volume, &plane, &camera, width, height)?;
    Ok(rgba)
}

/// Replace the stack ramp with the reference volume composite.
///
/// cornerstone3D 5.8.2 samples a 1024-wide colour table with OpenGL `LINEAR`.
/// The thin composite then multiplies by `1 - pow(0, raySteps)`. SwiftShader
/// resolves that power as `2^(-127 * raySteps)`. `raySteps` is this ray's
/// clipped travel divided by `(spacingX + spacingY + spacingZ) / 6`.
/// The stack ramp stays in `draw_mono` for every non-volume view.
struct SampleGrid<'a> {
    stored: &'a [Stored],
    absent: &'a [bool],
    columns: u32,
    rows: u32,
}

fn paint_reference_tone(
    rgba: &mut [u8],
    sidecar: &Sidecar,
    built: &BuiltVolume,
    plane: &PatientPlane,
    camera: &ReferenceCamera,
    view: &View,
    grid: SampleGrid<'_>,
) -> Result<(), &'static str> {
    let (width, height) = sidecar.frame_size().map_err(|_| "canvas-extent")?;
    let SampleGrid {
        stored,
        absent,
        columns,
        rows,
    } = grid;
    if columns != width || rows != height {
        return Err("canvas-extent");
    }
    let Some((center, window_width)) = linear_window(sidecar)? else {
        return Ok(());
    };
    let distance = sample_distance(&built.volume)?;
    let slope = f32_from_f64(built.slope);
    let intercept = f32_from_f64(built.intercept);
    let invert = built.photometric == "MONOCHROME1";
    let mut row = 0_u32;
    while row < rows {
        let mut column = 0_u32;
        while column < columns {
            let offset = sample_offset(columns, column, row).ok_or("canvas-extent")?;
            let missing = absent.get(offset).copied().unwrap_or(true);
            if !missing {
                let sample = stored.get(offset).copied().ok_or("volume-sample-absent")?;
                let scalar = sample.0 * slope + intercept;
                let point = plane_point(plane, column, row);
                if let Some(hit) = cast_ray(camera, point) {
                    let byte = reference_volume_byte(
                        scalar,
                        center,
                        window_width,
                        invert,
                        hit.travel,
                        distance,
                    );
                    let (x, y) = view.image_to_canvas(
                        f64::from(column) + 0.5,
                        f64::from(row) + 0.5,
                        columns,
                        rows,
                    );
                    if let Some((pixel_x, pixel_y)) = canvas_pixel(x, y, columns, rows) {
                        write_grey(rgba, pixel_x, pixel_y, width, byte);
                    }
                }
            }
            column = column.saturating_add(1);
        }
        row = row.saturating_add(1);
    }
    Ok(())
}

fn linear_window(sidecar: &Sidecar) -> Result<Option<(f32, f32)>, &'static str> {
    let function = sidecar
        .json
        .pointer("/voi/voiLutFunction")
        .and_then(Value::as_str);
    match function {
        None | Some("LINEAR") => {}
        Some("LINEAR_EXACT" | "SIGMOID") => return Ok(None),
        Some(_) => return Err("voi-function-unknown"),
    }
    let center = sidecar
        .json
        .pointer("/voi/windowCenter")
        .and_then(super::json_f64)
        .ok_or("voi-absent")?;
    let width = sidecar
        .json
        .pointer("/voi/windowWidth")
        .and_then(super::json_f64)
        .ok_or("voi-absent")?;
    if !center.is_finite() || !width.is_finite() || width <= 1.0 {
        return Err("voi-window-refused");
    }
    Ok(Some((f32_from_f64(center), f32_from_f64(width))))
}

fn sample_distance(volume: &Volume) -> Result<f32, &'static str> {
    let [x, y, z] = volume.spacing;
    let distance = (f32_from_f64(x) + f32_from_f64(y) + f32_from_f64(z)) / 6.0;
    if !distance.is_finite() || distance <= 0.0 {
        return Err("volume-spacing-refused");
    }
    Ok(distance)
}

/// Normalised LINEAR window. PS3.3 C.11.2.1.2.1 with the half-pixel adjustment
/// cornerstone applies before the colour texture.
fn linear_s(scalar: f32, center: f32, width: f32) -> f32 {
    let adjusted_center = center - 0.5;
    let adjusted_width = width - 1.0;
    if !scalar.is_finite() || !adjusted_width.is_finite() || adjusted_width <= 0.0 {
        return 0.0;
    }
    let lower = adjusted_center - adjusted_width / 2.0;
    if scalar <= lower {
        return 0.0;
    }
    let upper = adjusted_center + adjusted_width / 2.0;
    if scalar > upper {
        return 1.0;
    }
    (scalar - lower) / adjusted_width
}

fn vtk_texel(index: u32) -> f32 {
    let limited = index.min(1023);
    let rising = 255.0 * f32::from(u16::try_from(limited).unwrap_or(0)) / 1023.0;
    f32::from(half_even_byte(rising)) / 255.0
}

/// OpenGL `LINEAR` of the 1024 table. `s` is clamped to `[0, 1]`.
/// Texel `i` is `half_even(255 * i / 1023) / 255`, and the address is
/// `s * 1024 - 0.5` with both neighbours clamped to `[0, 1023]`.
fn gl_linear_unit(s: f32) -> f32 {
    let coord = if s.is_finite() {
        s.clamp(0.0, 1.0)
    } else {
        0.0
    };
    let x = coord * 1024.0 - 0.5;
    if x <= 0.0 {
        return vtk_texel(0);
    }
    let mut lo = 0_u32;
    let mut hi = 1024_u32;
    while lo.saturating_add(1) < hi {
        let mid = lo + (hi - lo) / 2;
        if f32::from(u16::try_from(mid).unwrap_or(0)) <= x {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    let lo = lo.min(1023);
    let next = lo.saturating_add(1).min(1023);
    let frac = (x - f32::from(u16::try_from(lo).unwrap_or(0))).clamp(0.0, 1.0);
    vtk_texel(lo) * (1.0 - frac) + vtk_texel(next) * frac
}

fn reference_volume_byte(
    scalar: f32,
    center: f32,
    width: f32,
    invert: bool,
    travel: f32,
    sample_distance: f32,
) -> u8 {
    let mut unit = gl_linear_unit(linear_s(scalar, center, width));
    if invert {
        unit = 1.0 - unit;
    }
    let steps = travel / sample_distance;
    let alpha = 1.0 - (-127.0 * steps).exp2();
    half_even_byte(unit * alpha * 255.0)
}

/// `Uint8ClampedArray` rounds half to even when the table is stored.
fn half_even_byte(value: f32) -> u8 {
    let rounded = if !value.is_finite() || value <= 0.0 {
        0.0
    } else if value >= 255.0 {
        255.0
    } else {
        let floor = value.floor();
        let fraction = value - floor;
        if fraction > 0.5 {
            floor + 1.0
        } else if fraction < 0.5 {
            floor
        } else {
            let half = floor * 0.5;
            let even = (half.floor() - half).to_bits() == 0.0_f32.to_bits();
            if even { floor } else { floor + 1.0 }
        }
    };
    let mut candidate = 0_u8;
    loop {
        if (f32::from(candidate) - rounded).abs() <= 1.0e-3 {
            return candidate;
        }
        if candidate == 255 {
            return 255;
        }
        candidate = candidate.saturating_add(1);
    }
}

/// The reference camera's patient plane, one sample per canvas pixel.
///
/// `viewUp` is the negated column cosine, so increasing row index moves down
/// the canvas. The row cosine is `viewUp` crossed with `viewPlaneNormal`,
/// which is the right vector on this side of the plane. PS3.3 C.7.6.2.1.1.
/// Spacing is `2 * parallelScale / height`. `Volume::reslice_nearest` rounds
/// the index and `sample_at` reads that voxel.
fn patient_plane(sidecar: &Sidecar, width: u32, height: u32) -> Result<PatientPlane, &'static str> {
    if width == 0 || height == 0 {
        return Err("canvas-extent");
    }
    let focal = super::json_triple(&sidecar.json, "/camera/focalPoint")
        .ok_or("camera-placement-refused")?;
    let view_up =
        super::json_triple(&sidecar.json, "/camera/viewUp").ok_or("camera-orientation-refused")?;
    let normal = super::json_triple(&sidecar.json, "/camera/viewPlaneNormal")
        .ok_or("camera-orientation-refused")?;
    let parallel = sidecar
        .json
        .pointer("/camera/parallelScale")
        .and_then(super::json_f64)
        .ok_or("camera-scale-refused")?;
    if !parallel.is_finite() || parallel <= 0.0 {
        return Err("camera-scale-refused");
    }
    let millimetres = 2.0 * parallel / f64::from(height);
    if !millimetres.is_finite() || millimetres <= 0.0 {
        return Err("camera-scale-refused");
    }
    let column = [-view_up[0], -view_up[1], -view_up[2]];
    let row = cross(view_up, normal);
    let orientation =
        ImageOrientationPatient::new(row, column).map_err(|_| "camera-orientation-refused")?;
    let row_direction = orientation.row();
    let column_direction = orientation.column();
    let row = [row_direction.x, row_direction.y, row_direction.z];
    let column = [column_direction.x, column_direction.y, column_direction.z];
    let centre_column = (f64::from(width) - 1.0) / 2.0;
    let centre_row = (f64::from(height) - 1.0) / 2.0;
    let origin = [
        focal[0] - row[0] * millimetres * centre_column - column[0] * millimetres * centre_row,
        focal[1] - row[1] * millimetres * centre_column - column[1] * millimetres * centre_row,
        focal[2] - row[2] * millimetres * centre_column - column[2] * millimetres * centre_row,
    ];
    Ok(PatientPlane {
        oblique: ObliquePlane {
            origin: Pt::<World>::new(origin[0], origin[1], origin[2]),
            row: row_direction,
            column: column_direction,
            columns: width,
            rows: height,
            column_mm: millimetres,
            row_mm: millimetres,
        },
        origin,
        row,
        column,
    })
}

fn cross(left: [f64; 3], right: [f64; 3]) -> [f64; 3] {
    [
        left[1] * right[2] - left[2] * right[1],
        left[2] * right[0] - left[0] * right[2],
        left[0] * right[1] - left[1] * right[0],
    ]
}

fn dot(left: [f64; 3], right: [f64; 3]) -> f64 {
    left[0] * right[0] + left[1] * right[1] + left[2] * right[2]
}

fn finite_triple(value: [f64; 3]) -> bool {
    value[0].is_finite() && value[1].is_finite() && value[2].is_finite()
}

fn normalize(value: [f64; 3]) -> Result<[f64; 3], &'static str> {
    let length = (value[0] * value[0] + value[1] * value[1] + value[2] * value[2]).sqrt();
    if !length.is_finite() || length <= 0.0 {
        return Err("camera-orientation-refused");
    }
    Ok([value[0] / length, value[1] / length, value[2] / length])
}

fn look_basis(eye: [f64; 3], focal: [f64; 3], up: [f64; 3]) -> Result<LookBasis, &'static str> {
    if !finite_triple(eye) || !finite_triple(focal) || !finite_triple(up) {
        return Err("camera-orientation-refused");
    }
    let z_axis = normalize([eye[0] - focal[0], eye[1] - focal[1], eye[2] - focal[2]])?;
    let x_axis = normalize(cross(up, z_axis))?;
    let y_axis = normalize(cross(z_axis, x_axis))?;
    Ok(LookBasis {
        x_axis,
        y_axis,
        z_axis,
        eye,
    })
}

fn view_point(basis: &LookBasis, point: [f64; 3]) -> [f64; 3] {
    let delta = [
        point[0] - basis.eye[0],
        point[1] - basis.eye[1],
        point[2] - basis.eye[2],
    ];
    [
        dot(basis.x_axis, delta),
        dot(basis.y_axis, delta),
        dot(basis.z_axis, delta),
    ]
}

fn direction_column(volume: &Volume, axis: usize) -> [f64; 3] {
    let column = volume.direction.col(axis);
    [column.x, column.y, column.z]
}

fn narrow_triple(value: [f64; 3]) -> [f32; 3] {
    [
        f32_from_f64(value[0]),
        f32_from_f64(value[1]),
        f32_from_f64(value[2]),
    ]
}

/// One rounding after the sum. A float32 running sum is one ULP off on the
/// coronal face, and that ULP is the difference between a hit and a miss.
fn dot_rounded(row: [f32; 3], vector: [f32; 3]) -> f32 {
    let sum = f64::from(row[0]) * f64::from(vector[0])
        + f64::from(row[1]) * f64::from(vector[1])
        + f64::from(row[2]) * f64::from(vector[2]);
    f32_from_f64(sum)
}

fn mat_vec(matrix: [[f32; 3]; 3], vector: [f32; 3]) -> [f32; 3] {
    [
        dot_rounded(matrix[0], vector),
        dot_rounded(matrix[1], vector),
        dot_rounded(matrix[2], vector),
    ]
}

/// Inverse of a 3 by 3 matrix in `f64`, after the caller has narrowed and widened.
///
/// gl-matrix inverts the float32 uniform with float64 arithmetic and stores
/// float32. A float32 LU inverse is a different matrix.
fn inverse3(matrix: [[f64; 3]; 3]) -> Result<[[f64; 3]; 3], &'static str> {
    let det = matrix[0][0] * (matrix[1][1] * matrix[2][2] - matrix[1][2] * matrix[2][1])
        - matrix[0][1] * (matrix[1][0] * matrix[2][2] - matrix[1][2] * matrix[2][0])
        + matrix[0][2] * (matrix[1][0] * matrix[2][1] - matrix[1][1] * matrix[2][0]);
    if !det.is_finite() || det.to_bits() == 0 || det.to_bits() == 1_u64 << 63 {
        return Err("volume-reslice-refused");
    }
    let inverse = [
        [
            (matrix[1][1] * matrix[2][2] - matrix[1][2] * matrix[2][1]) / det,
            (matrix[0][2] * matrix[2][1] - matrix[0][1] * matrix[2][2]) / det,
            (matrix[0][1] * matrix[1][2] - matrix[0][2] * matrix[1][1]) / det,
        ],
        [
            (matrix[1][2] * matrix[2][0] - matrix[1][0] * matrix[2][2]) / det,
            (matrix[0][0] * matrix[2][2] - matrix[0][2] * matrix[2][0]) / det,
            (matrix[0][2] * matrix[1][0] - matrix[0][0] * matrix[1][2]) / det,
        ],
        [
            (matrix[1][0] * matrix[2][1] - matrix[1][1] * matrix[2][0]) / det,
            (matrix[0][1] * matrix[2][0] - matrix[0][0] * matrix[2][1]) / det,
            (matrix[0][0] * matrix[1][1] - matrix[0][1] * matrix[1][0]) / det,
        ],
    ];
    if !inverse.iter().flatten().all(|value| value.is_finite()) {
        return Err("volume-reslice-refused");
    }
    Ok(inverse)
}

fn is_to_view(basis: &LookBasis, volume: &Volume) -> Result<[[f32; 3]; 3], &'static str> {
    let scales = [
        f64::from(volume.dims[0]) * volume.spacing[0],
        f64::from(volume.dims[1]) * volume.spacing[1],
        f64::from(volume.dims[2]) * volume.spacing[2],
    ];
    if !scales.iter().all(|scale| scale.is_finite() && *scale > 0.0) {
        return Err("volume-reslice-refused");
    }
    let i_axis = direction_column(volume, 0);
    let j_axis = direction_column(volume, 1);
    let k_axis = direction_column(volume, 2);
    let rows = [basis.x_axis, basis.y_axis, basis.z_axis];
    let wide = [
        [
            dot(rows[0], i_axis) * scales[0],
            dot(rows[0], j_axis) * scales[1],
            dot(rows[0], k_axis) * scales[2],
        ],
        [
            dot(rows[1], i_axis) * scales[0],
            dot(rows[1], j_axis) * scales[1],
            dot(rows[1], k_axis) * scales[2],
        ],
        [
            dot(rows[2], i_axis) * scales[0],
            dot(rows[2], j_axis) * scales[1],
            dot(rows[2], k_axis) * scales[2],
        ],
    ];
    let narrowed = [
        narrow_triple(wide[0]),
        narrow_triple(wide[1]),
        narrow_triple(wide[2]),
    ];
    let widened = [
        [
            f64::from(narrowed[0][0]),
            f64::from(narrowed[0][1]),
            f64::from(narrowed[0][2]),
        ],
        [
            f64::from(narrowed[1][0]),
            f64::from(narrowed[1][1]),
            f64::from(narrowed[1][2]),
        ],
        [
            f64::from(narrowed[2][0]),
            f64::from(narrowed[2][1]),
            f64::from(narrowed[2][2]),
        ],
    ];
    let inverse = inverse3(widened)?;
    let result = [
        narrow_triple(inverse[0]),
        narrow_triple(inverse[1]),
        narrow_triple(inverse[2]),
    ];
    if !result.iter().flatten().all(|value| value.is_finite()) {
        return Err("volume-reslice-refused");
    }
    Ok(result)
}

fn parallel_quad_view_z() -> Result<f32, &'static str> {
    let near = -REFERENCE_RAY_LIMIT_MM;
    let far = REFERENCE_RAY_LIMIT_MM;
    let span = near - far;
    if !span.is_finite() || span.to_bits() == 0 || span.to_bits() == 1_u64 << 63 {
        return Err("camera-scale-refused");
    }
    let inverse_span = 1.0 / span;
    let m22 = 2.0 * inverse_span;
    let m23 = (far + near) * inverse_span;
    if m22.to_bits() == 0 || m22.to_bits() == 1_u64 << 63 {
        return Err("camera-scale-refused");
    }
    let z = (-1.0 - m23) / m22;
    if !z.is_finite() {
        return Err("camera-scale-refused");
    }
    Ok(f32_from_f64(z))
}

fn slab_plane(basis: &LookBasis, focal: [f64; 3], normal_world: [f64; 3], sign: f64) -> SlabPlane {
    let normal_w = [
        normal_world[0] * sign,
        normal_world[1] * sign,
        normal_world[2] * sign,
    ];
    let origin_w = [
        focal[0] - normal_world[0] * REFERENCE_SLAB_MM * sign,
        focal[1] - normal_world[1] * REFERENCE_SLAB_MM * sign,
        focal[2] - normal_world[2] * REFERENCE_SLAB_MM * sign,
    ];
    let normal = narrow_triple([
        dot(basis.x_axis, normal_w),
        dot(basis.y_axis, normal_w),
        dot(basis.z_axis, normal_w),
    ]);
    let position = view_point(basis, origin_w);
    let distance = f32_from_f64(
        -(position[0] * f64::from(normal[0])
            + position[1] * f64::from(normal[1])
            + position[2] * f64::from(normal[2])),
    );
    SlabPlane { normal, distance }
}

fn signed_zero(value: f32) -> bool {
    let bits = value.to_bits();
    bits == 0 || bits == 0x8000_0000
}

fn volume_span(origin_is: [f32; 3], dir_is: [f32; 3]) -> (f32, f32) {
    let inverse = [
        1.0_f32 / dir_is[0],
        1.0_f32 / dir_is[1],
        1.0_f32 / dir_is[2],
    ];
    let toward_zero = [
        (0.0_f32 - origin_is[0]) * inverse[0],
        (0.0_f32 - origin_is[1]) * inverse[1],
        (0.0_f32 - origin_is[2]) * inverse[2],
    ];
    let toward_one = [
        (1.0_f32 - origin_is[0]) * inverse[0],
        (1.0_f32 - origin_is[1]) * inverse[1],
        (1.0_f32 - origin_is[2]) * inverse[2],
    ];
    let first = [
        toward_zero[0].min(toward_one[0]),
        toward_zero[1].min(toward_one[1]),
        toward_zero[2].min(toward_one[2]),
    ];
    let last = [
        toward_zero[0].max(toward_one[0]),
        toward_zero[1].max(toward_one[1]),
        toward_zero[2].max(toward_one[2]),
    ];
    (
        first[0].max(first[1]).max(first[2]),
        last[0].min(last[1]).min(last[2]),
    )
}

fn clip_span(
    origin: [f32; 3],
    mut tmin: f32,
    mut tmax: f32,
    planes: &[SlabPlane; 2],
) -> (f32, f32) {
    for plane in planes {
        let ratio = -plane.normal[2];
        let equation = origin[0] * plane.normal[0]
            + origin[1] * plane.normal[1]
            + origin[2] * plane.normal[2]
            + plane.distance;
        if signed_zero(ratio) {
            if equation < 0.0 {
                tmin = tmax;
            }
            continue;
        }
        let result = -equation / ratio;
        if ratio < 0.0 {
            tmax = tmax.min(result);
        } else {
            tmin = tmin.max(result);
        }
    }
    (tmin, tmax)
}

fn floor_below(value: f32, limit: u32) -> Option<u32> {
    if !value.is_finite() || value < 0.0 || limit == 0 {
        return None;
    }
    let widened = f64::from(value);
    if widened >= f64::from(limit) {
        return None;
    }
    let mut lo = 0_u32;
    let mut hi = limit;
    while lo.saturating_add(1) < hi {
        let mid = lo + (hi - lo) / 2;
        if f64::from(mid) <= widened {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    if f64::from(lo) <= widened && lo < limit {
        Some(lo)
    } else {
        None
    }
}

fn clamp_texel(coord: f32, dim: f32, limit: u32) -> Option<u32> {
    if !coord.is_finite() || !dim.is_finite() || dim <= 0.0 || limit == 0 {
        return None;
    }
    let edge = 0.5_f32 / dim;
    let high = 1.0_f32 - edge;
    let clamped = if coord < edge {
        edge
    } else if coord > high {
        high
    } else {
        coord
    };
    floor_below(clamped * dim, limit)
}

fn cast_ray(camera: &ReferenceCamera, point: [f64; 3]) -> Option<RayHit> {
    let view = view_point(&camera.basis, point);
    let origin = [f32_from_f64(view[0]), f32_from_f64(view[1]), camera.near_z];
    let delta = [
        origin[0] - camera.origin_vc[0],
        origin[1] - camera.origin_vc[1],
        origin[2] - camera.origin_vc[2],
    ];
    let origin_is = mat_vec(camera.vec_vc_to_is, delta);
    let dir_is = [
        -camera.vec_vc_to_is[0][2],
        -camera.vec_vc_to_is[1][2],
        -camera.vec_vc_to_is[2][2],
    ];
    let (tmin, tmax) = volume_span(origin_is, dir_is);
    let (tmin, tmax) = clip_span(origin, tmin, tmax, &camera.planes);
    let tmin = if tmin < 0.0 { 0.0_f32 } else { tmin };
    if !tmin.is_finite() || !tmax.is_finite() || tmax <= tmin {
        return None;
    }
    let travel = tmax - tmin;
    let sample = [origin[0], origin[1], origin[2] - tmin];
    let sample_delta = [
        sample[0] - camera.origin_vc[0],
        sample[1] - camera.origin_vc[1],
        sample[2] - camera.origin_vc[2],
    ];
    let sample_is = mat_vec(camera.vec_vc_to_is, sample_delta);
    let texel = [
        clamp_texel(sample_is[0], camera.dims_f32[0], camera.dims[0])?,
        clamp_texel(sample_is[1], camera.dims_f32[1], camera.dims[1])?,
        clamp_texel(sample_is[2], camera.dims_f32[2], camera.dims[2])?,
    ];
    Some(RayHit {
        tmin,
        travel,
        texel,
    })
}

fn slab_shift(camera: &ReferenceCamera) -> Result<f64, &'static str> {
    let view = view_point(&camera.basis, camera.focal);
    let hit = cast_ray(camera, camera.focal).ok_or("volume-reslice-refused")?;
    let sample_z = f64::from(camera.near_z - hit.tmin);
    let shift = sample_z - view[2];
    if !shift.is_finite() {
        return Err("camera-placement-refused");
    }
    Ok(shift)
}

fn reference_camera(sidecar: &Sidecar, volume: &Volume) -> Result<ReferenceCamera, &'static str> {
    let eye =
        super::json_triple(&sidecar.json, "/camera/position").ok_or("camera-placement-refused")?;
    let focal = super::json_triple(&sidecar.json, "/camera/focalPoint")
        .ok_or("camera-placement-refused")?;
    let up =
        super::json_triple(&sidecar.json, "/camera/viewUp").ok_or("camera-orientation-refused")?;
    let toward_camera = super::json_triple(&sidecar.json, "/camera/viewPlaneNormal")
        .ok_or("camera-orientation-refused")?;
    let basis = look_basis(eye, focal, up)?;
    let vec_vc_to_is = is_to_view(&basis, volume)?;
    let corner = volume.voxel_world([-0.5, -0.5, -0.5]);
    let origin_vc = narrow_triple(view_point(&basis, [corner.x, corner.y, corner.z]));
    let near_z = parallel_quad_view_z()?;
    let planes = [
        slab_plane(&basis, focal, toward_camera, 1.0),
        slab_plane(&basis, focal, toward_camera, -1.0),
    ];
    let dims = volume.dims;
    let dims_f32 = [
        f32_from_f64(f64::from(dims[0])),
        f32_from_f64(f64::from(dims[1])),
        f32_from_f64(f64::from(dims[2])),
    ];
    let mut camera = ReferenceCamera {
        toward_camera,
        shift: 0.0,
        focal,
        basis,
        vec_vc_to_is,
        origin_vc,
        near_z,
        planes,
        dims,
        dims_f32,
    };
    camera.shift = slab_shift(&camera)?;
    Ok(camera)
}

fn offset_along_normal(
    plane: &PatientPlane,
    toward_camera: [f64; 3],
    shift: f64,
) -> Result<PatientPlane, &'static str> {
    if !shift.is_finite() || !finite_triple(toward_camera) {
        return Err("camera-placement-refused");
    }
    let origin = [
        plane.origin[0] + toward_camera[0] * shift,
        plane.origin[1] + toward_camera[1] * shift,
        plane.origin[2] + toward_camera[2] * shift,
    ];
    if !finite_triple(origin) {
        return Err("camera-placement-refused");
    }
    let mut oblique = plane.oblique;
    oblique.origin = Pt::<World>::new(origin[0], origin[1], origin[2]);
    Ok(PatientPlane {
        oblique,
        origin,
        row: plane.row,
        column: plane.column,
    })
}

fn plane_point(plane: &PatientPlane, x: u32, y: u32) -> [f64; 3] {
    let column = f64::from(x);
    let row = f64::from(y);
    [
        plane.origin[0]
            + plane.row[0] * plane.oblique.column_mm * column
            + plane.column[0] * plane.oblique.row_mm * row,
        plane.origin[1]
            + plane.row[1] * plane.oblique.column_mm * column
            + plane.column[1] * plane.oblique.row_mm * row,
        plane.origin[2]
            + plane.row[2] * plane.oblique.column_mm * column
            + plane.column[2] * plane.oblique.row_mm * row,
    ]
}

fn offset_point(point: [f64; 3], direction: [f64; 3], distance: f64) -> [f64; 3] {
    [
        point[0] + direction[0] * distance,
        point[1] + direction[1] * distance,
        point[2] + direction[2] * distance,
    ]
}

fn continuous_index(volume: &Volume, point: [f64; 3]) -> [f64; 3] {
    let delta = [
        point[0] - volume.origin.x,
        point[1] - volume.origin.y,
        point[2] - volume.origin.z,
    ];
    let i_axis = direction_column(volume, 0);
    let j_axis = direction_column(volume, 1);
    let k_axis = direction_column(volume, 2);
    [
        dot(i_axis, delta) / volume.spacing[0],
        dot(j_axis, delta) / volume.spacing[1],
        dot(k_axis, delta) / volume.spacing[2],
    ]
}

fn floor_nonneg_u32(index: f64) -> Option<u32> {
    if !index.is_finite() || index < 0.0 || index >= f64::from(u32::MAX) {
        return None;
    }
    let mut lo = 0_u32;
    let mut hi = u32::MAX;
    while lo.saturating_add(1) < hi {
        let mid = lo + (hi - lo) / 2;
        if f64::from(mid) <= index {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    if f64::from(lo) <= index {
        Some(lo)
    } else {
        None
    }
}

/// Same rounding as `nearest_index`, then the volume bounds.
/// A result of `dim` is absent. The private helper stays in `ocelli-volume`.
fn nearest_voxel(index: f64, dim: u32) -> Option<u32> {
    if !index.is_finite() || dim == 0 {
        return None;
    }
    if index < 0.0 {
        if index > -0.5 { Some(0) } else { None }
    } else {
        let base = floor_nonneg_u32(index)?;
        let fraction = index - f64::from(base);
        let rounded = if fraction >= 0.5 {
            base.checked_add(1)?
        } else {
            base
        };
        if rounded >= dim { None } else { Some(rounded) }
    }
}

fn stored_at_voxel(
    volume: &Volume,
    plane: &PatientPlane,
    texel: [u32; 3],
) -> Result<PlaneSample, &'static str> {
    let world = volume.voxel_world([
        f64::from(texel[0]),
        f64::from(texel[1]),
        f64::from(texel[2]),
    ]);
    let probe = ObliquePlane {
        origin: world,
        row: plane.oblique.row,
        column: plane.oblique.column,
        columns: 1,
        rows: 1,
        column_mm: plane.oblique.column_mm,
        row_mm: plane.oblique.row_mm,
    };
    let image = volume
        .reslice_nearest(probe)
        .map_err(|_| "volume-reslice-refused")?;
    image.samples.first().copied().ok_or("volume-sample-absent")
}

fn sample_offset(width: u32, x: u32, y: u32) -> Option<usize> {
    usize::try_from(y)
        .ok()?
        .checked_mul(usize::try_from(width).ok()?)?
        .checked_add(usize::try_from(x).ok()?)
}

fn apply_reference_ray(
    image: &mut OrthoImage,
    volume: &Volume,
    plane: &PatientPlane,
    camera: &ReferenceCamera,
) -> Result<(), &'static str> {
    if image.width != plane.oblique.columns || image.height != plane.oblique.rows {
        return Err("canvas-extent");
    }
    let mut y = 0_u32;
    while y < image.height {
        let mut x = 0_u32;
        while x < image.width {
            correct_ray_sample(image, volume, plane, camera, x, y)?;
            x = x.saturating_add(1);
        }
        y = y.saturating_add(1);
    }
    Ok(())
}

fn correct_ray_sample(
    image: &mut OrthoImage,
    volume: &Volume,
    plane: &PatientPlane,
    camera: &ReferenceCamera,
    x: u32,
    y: u32,
) -> Result<(), &'static str> {
    let point = plane_point(plane, x, y);
    let replacement = match cast_ray(camera, point) {
        None => Some(PlaneSample::Absent),
        Some(hit) => {
            let shifted = offset_point(point, camera.toward_camera, camera.shift);
            let index = continuous_index(volume, shifted);
            let nearest = [
                nearest_voxel(index[0], volume.dims[0]),
                nearest_voxel(index[1], volume.dims[1]),
                nearest_voxel(index[2], volume.dims[2]),
            ];
            let same = nearest[0] == Some(hit.texel[0])
                && nearest[1] == Some(hit.texel[1])
                && nearest[2] == Some(hit.texel[2]);
            if same {
                None
            } else {
                Some(stored_at_voxel(volume, plane, hit.texel)?)
            }
        }
    };
    let Some(sample) = replacement else {
        return Ok(());
    };
    let offset = sample_offset(image.width, x, y).ok_or("canvas-extent")?;
    let slot = image
        .samples
        .get_mut(offset)
        .ok_or("volume-sample-absent")?;
    *slot = sample;
    Ok(())
}

fn include_corner(
    bounds: &mut QuadBounds,
    volume: &Volume,
    plane: &PatientPlane,
    focal: [f64; 3],
    index: [f64; 3],
    width: u32,
    height: u32,
) {
    let world = volume.voxel_world(index);
    let delta = [world.x - focal[0], world.y - focal[1], world.z - focal[2]];
    let centre_x = (f64::from(width) - 1.0) / 2.0;
    let centre_y = (f64::from(height) - 1.0) / 2.0;
    let cx = centre_x + dot(delta, plane.row) / plane.oblique.column_mm;
    let cy = centre_y + dot(delta, plane.column) / plane.oblique.row_mm;
    if cx < bounds.left {
        bounds.left = cx;
    }
    if cx > bounds.right {
        bounds.right = cx;
    }
    if cy < bounds.top {
        bounds.top = cy;
    }
    if cy > bounds.bottom {
        bounds.bottom = cy;
    }
}

fn quad_bounds(
    volume: &Volume,
    plane: &PatientPlane,
    focal: [f64; 3],
    width: u32,
    height: u32,
) -> Result<QuadBounds, &'static str> {
    if !plane.oblique.column_mm.is_finite()
        || !plane.oblique.row_mm.is_finite()
        || plane.oblique.column_mm <= 0.0
        || plane.oblique.row_mm <= 0.0
    {
        return Err("camera-scale-refused");
    }
    let mut bounds = QuadBounds {
        left: f64::INFINITY,
        right: f64::NEG_INFINITY,
        top: f64::INFINITY,
        bottom: f64::NEG_INFINITY,
    };
    let i0 = -0.5;
    let j0 = -0.5;
    let k0 = -0.5;
    let i1 = f64::from(volume.dims[0]) - 0.5;
    let j1 = f64::from(volume.dims[1]) - 0.5;
    let k1 = f64::from(volume.dims[2]) - 0.5;
    for i in [i0, i1] {
        for j in [j0, j1] {
            for k in [k0, k1] {
                include_corner(&mut bounds, volume, plane, focal, [i, j, k], width, height);
            }
        }
    }
    if !bounds.left.is_finite()
        || !bounds.right.is_finite()
        || !bounds.top.is_finite()
        || !bounds.bottom.is_finite()
    {
        return Err("volume-reslice-refused");
    }
    Ok(bounds)
}

fn coverage_hits(x: u32, y: u32, bounds: &QuadBounds) -> u32 {
    let mut hits = 0_u32;
    let px = f64::from(x);
    let py = f64::from(y);
    for (offset_x, offset_y) in REFERENCE_COVERAGE_OFFSETS {
        let sx = px + offset_x;
        let sy = py + offset_y;
        if sx > bounds.left && sx <= bounds.right && sy > bounds.top && sy <= bounds.bottom {
            hits = hits.saturating_add(1);
        }
    }
    hits
}

fn reference_coverage_byte(value: u8, hits: u32) -> u8 {
    let scaled =
        u32::from(value).saturating_mul(hits).saturating_add(2) / REFERENCE_COVERAGE_SAMPLES;
    u8::try_from(scaled).unwrap_or(u8::MAX)
}

fn blend_coverage(rgba: &mut [u8], x: u32, y: u32, width: u32, hits: u32) {
    let covered = u32::try_from(REFERENCE_COVERAGE_OFFSETS.len()).unwrap_or(0);
    if hits == covered {
        return;
    }
    let Some(offset) = sample_offset(width, x, y).and_then(|pixel| pixel.checked_mul(4)) else {
        return;
    };
    let Some(pixel) = rgba.get_mut(offset..offset.saturating_add(4)) else {
        return;
    };
    if pixel.len() != 4 {
        return;
    }
    if hits == 0 {
        pixel.copy_from_slice(&[0, 0, 0, 255]);
        return;
    }
    let red = reference_coverage_byte(pixel.first().copied().unwrap_or(0), hits);
    let green = reference_coverage_byte(pixel.get(1).copied().unwrap_or(0), hits);
    let blue = reference_coverage_byte(pixel.get(2).copied().unwrap_or(0), hits);
    pixel.copy_from_slice(&[red, green, blue, 255]);
}

fn cover_reference_quad(
    rgba: &mut [u8],
    volume: &Volume,
    plane: &PatientPlane,
    camera: &ReferenceCamera,
    width: u32,
    height: u32,
) -> Result<(), &'static str> {
    let expected = usize::try_from(width)
        .ok()
        .and_then(|axis| usize::try_from(height).ok()?.checked_mul(axis))
        .and_then(|count| count.checked_mul(4))
        .ok_or("canvas-extent")?;
    if rgba.len() != expected {
        return Err("canvas-extent");
    }
    let bounds = quad_bounds(volume, plane, camera.focal, width, height)?;
    let mut y = 0_u32;
    while y < height {
        let mut x = 0_u32;
        while x < width {
            let hits = coverage_hits(x, y, &bounds);
            blend_coverage(rgba, x, y, width, hits);
            x = x.saturating_add(1);
        }
        y = y.saturating_add(1);
    }
    Ok(())
}

fn plane_samples(image: &OrthoImage) -> Result<(Vec<Stored>, Vec<bool>), &'static str> {
    let mut stored = Vec::new();
    let mut absent = Vec::new();
    stored
        .try_reserve(image.samples.len())
        .map_err(|_| "pixel-data-length")?;
    absent
        .try_reserve(image.samples.len())
        .map_err(|_| "pixel-data-length")?;
    let mut present = false;
    for sample in &image.samples {
        match sample {
            // `PlaneSample` already holds the container sample as `f32`.
            // `Stored` is that same unit. Slope and intercept are applied later.
            PlaneSample::Present(value) => {
                stored.push(Stored(*value));
                absent.push(false);
                present = true;
            }
            // The placeholder is overwritten with the background after the LUT.
            // A missing corner is outside the volume.
            PlaneSample::Absent => {
                stored.push(Stored(0.0));
                absent.push(true);
            }
        }
    }
    if !present {
        return Err("volume-sample-absent");
    }
    Ok((stored, absent))
}

fn synthetic_sidecar(
    volume_sidecar: &Sidecar,
    built: &BuiltVolume,
    plane: &PatientPlane,
    image: &OrthoImage,
) -> Result<Sidecar, &'static str> {
    let (width, height) = volume_sidecar.frame_size().map_err(|_| "canvas-extent")?;
    let camera = volume_sidecar
        .json
        .get("camera")
        .cloned()
        .ok_or("camera-placement-refused")?;
    let voi = volume_sidecar
        .json
        .get("voi")
        .cloned()
        .ok_or("voi-absent")?;
    let parallel = camera
        .pointer("/parallelScale")
        .and_then(super::json_f64)
        .ok_or("camera-scale-refused")?;
    let (vertical, horizontal) = canvas_scale(parallel, height, image.row_mm, image.column_mm)?;
    let mut attributes = Map::new();
    attributes.insert(
        "photometricInterpretation".to_owned(),
        Value::String(built.photometric.clone()),
    );
    attributes.insert(
        "pixelSpacing".to_owned(),
        Value::Array(vec![
            super::json_number(image.row_mm),
            super::json_number(image.column_mm),
        ]),
    );
    let [rx, ry, rz] = plane.row;
    let [cx, cy, cz] = plane.column;
    let [ox, oy, oz] = plane.origin;
    attributes.insert(
        "imageOrientationPatient".to_owned(),
        Value::Array(vec![
            super::json_number(rx),
            super::json_number(ry),
            super::json_number(rz),
            super::json_number(cx),
            super::json_number(cy),
            super::json_number(cz),
        ]),
    );
    attributes.insert(
        "imagePositionPatient".to_owned(),
        Value::Array(vec![
            super::json_number(ox),
            super::json_number(oy),
            super::json_number(oz),
        ]),
    );
    let mut image_block = Map::new();
    image_block.insert("rows".to_owned(), Value::from(image.height));
    image_block.insert("columns".to_owned(), Value::from(image.width));
    image_block.insert("slope".to_owned(), super::json_number(built.slope));
    image_block.insert("intercept".to_owned(), super::json_number(built.intercept));
    image_block.insert("color".to_owned(), Value::Bool(false));
    let mut frame = Map::new();
    frame.insert("width".to_owned(), Value::from(width));
    frame.insert("height".to_owned(), Value::from(height));
    let mut scale = Map::new();
    scale.insert("vertical".to_owned(), super::json_number(vertical));
    scale.insert("horizontal".to_owned(), super::json_number(horizontal));
    let mut object = Map::new();
    object.insert("kind".to_owned(), Value::String("stack".to_owned()));
    object.insert("frame".to_owned(), Value::Object(frame));
    object.insert("image".to_owned(), Value::Object(image_block));
    object.insert("attributes".to_owned(), Value::Object(attributes));
    object.insert("voi".to_owned(), voi);
    object.insert("camera".to_owned(), camera);
    object.insert(
        "canvasPixelsPerSourcePixel".to_owned(),
        Value::Object(scale),
    );
    Ok(Sidecar {
        id: volume_sidecar.id.clone(),
        kind: ViewKind::Stack,
        json: Value::Object(object),
    })
}

fn canvas_scale(
    parallel: f64,
    height: u32,
    row_mm: f64,
    column_mm: f64,
) -> Result<(f64, f64), &'static str> {
    if height == 0 || !parallel.is_finite() || parallel <= 0.0 {
        return Err("camera-scale-refused");
    }
    let millimetres = 2.0 * parallel / f64::from(height);
    if !millimetres.is_finite() || millimetres <= 0.0 {
        return Err("camera-scale-refused");
    }
    let vertical = row_mm / millimetres;
    let horizontal = column_mm / millimetres;
    if !vertical.is_finite() || !horizontal.is_finite() || vertical <= 0.0 || horizontal <= 0.0 {
        return Err("camera-scale-refused");
    }
    Ok((vertical, horizontal))
}

fn raster_colour(
    sidecar: &Sidecar,
    transform: &ColorTransform,
    stored: &[Stored],
    columns: u32,
    rows: u32,
) -> Result<Vec<u8>, &'static str> {
    let view = stack_view(sidecar)?;
    let (width, height) = sidecar.frame_size().map_err(|_| "canvas-extent")?;
    let mut raster =
        CpuRaster::open(f64::from(width), f64::from(height), 1.0).map_err(|_| "canvas-extent")?;
    if raster.device() != [width, height] {
        return Err("canvas-extent");
    }
    raster
        .reserve_source(columns, rows)
        .map_err(|_| "canvas-extent")?;
    let mut rgba = rgba_buffer(width, height)?;
    let drawn = raster
        .raster_color_into(
            &view,
            RasterColor::supported(transform).map_err(|_| "raster-refused")?,
            stored,
            RasterGrid {
                source: [columns, rows],
                target: [width, height],
            },
            RasterQuality::Settled {
                generation: raster.generation(),
            },
            &mut rgba,
        )
        .map_err(|_| "raster-refused")?;
    if !drawn.wrote || drawn.extent != [width, height] {
        return Err("raster-idle");
    }
    Ok(rgba)
}

fn draw_mono(
    sidecar: &Sidecar,
    view: &View,
    stored: &[Stored],
    columns: u32,
    rows: u32,
) -> Result<Vec<u8>, &'static str> {
    let (width, height) = sidecar.frame_size().map_err(|_| "canvas-extent")?;
    let mut raster =
        CpuRaster::open(f64::from(width), f64::from(height), 1.0).map_err(|_| "canvas-extent")?;
    if raster.device() != [width, height] {
        return Err("canvas-extent");
    }
    let presentation = super::presentation_of(sidecar).map_err(stop_reason)?;
    let mut rgba = rgba_buffer(width, height)?;
    let drawn = raster
        .raster_into(
            view,
            &presentation,
            stored,
            RasterGrid {
                source: [columns, rows],
                target: [width, height],
            },
            RasterQuality::Settled {
                generation: raster.generation(),
            },
            &mut rgba,
        )
        .map_err(|_| "raster-refused")?;
    if !drawn.wrote || drawn.extent != [width, height] {
        return Err("raster-idle");
    }
    Ok(rgba)
}

fn stack_view(sidecar: &Sidecar) -> Result<View, &'static str> {
    super::executed_camera(sidecar)
        .map(|executed| executed.view)
        .map_err(stop_reason)
}

/// Outside the volume the canvas stays the reference background.
///
/// `image_to_canvas` places the source pixel centre. A sample `sample_at`
/// refused is not a stored zero run through the LUT.
fn paint_absent(view: &View, absent: &[bool], columns: u32, rows: u32, rgba: &mut [u8]) {
    let mut index = 0_usize;
    let mut row = 0_u32;
    while row < rows {
        let mut column = 0_u32;
        while column < columns {
            if absent.get(index) == Some(&true) {
                let (x, y) = view.image_to_canvas(
                    f64::from(column) + 0.5,
                    f64::from(row) + 0.5,
                    columns,
                    rows,
                );
                if let Some((pixel_x, pixel_y)) = canvas_pixel(x, y, columns, rows) {
                    write_black(rgba, pixel_x, pixel_y, columns);
                }
            }
            index = index.saturating_add(1);
            column = column.saturating_add(1);
        }
        row = row.saturating_add(1);
    }
}

fn canvas_pixel(x: f64, y: f64, columns: u32, rows: u32) -> Option<(u32, u32)> {
    Some((rounded_index(x, columns)?, rounded_index(y, rows)?))
}

fn rounded_index(coord: f64, limit: u32) -> Option<u32> {
    if !coord.is_finite() || limit == 0 {
        return None;
    }
    let rounded = (coord - 0.5).round();
    if rounded < 0.0 || rounded >= f64::from(limit) {
        return None;
    }
    let mut lo = 0_u32;
    let mut hi = limit;
    while lo < hi {
        let mid = lo + (hi - lo) / 2;
        if f64::from(mid) < rounded {
            lo = mid.saturating_add(1);
        } else {
            hi = mid;
        }
    }
    if (f64::from(lo) - rounded).abs() > 0.1 || lo >= limit {
        return None;
    }
    Some(lo)
}

fn write_black(rgba: &mut [u8], x: u32, y: u32, width: u32) {
    write_grey(rgba, x, y, width, 0);
}

fn write_grey(rgba: &mut [u8], x: u32, y: u32, width: u32, byte: u8) {
    let offset = usize::try_from(y)
        .ok()
        .zip(usize::try_from(x).ok())
        .zip(usize::try_from(width).ok())
        .and_then(|((row, column), span)| {
            row.checked_mul(span)
                .and_then(|start| start.checked_add(column))
                .and_then(|pixel| pixel.checked_mul(4))
        });
    let Some(offset) = offset else {
        return;
    };
    let Some(pixel) = rgba.get_mut(offset..offset.saturating_add(4)) else {
        return;
    };
    if pixel.len() == 4 {
        pixel.copy_from_slice(&[byte, byte, byte, 255]);
    }
}

fn rgba_buffer(width: u32, height: u32) -> Result<Vec<u8>, &'static str> {
    let pixels = usize::try_from(width)
        .ok()
        .and_then(|axis| {
            usize::try_from(height)
                .ok()
                .and_then(|other| axis.checked_mul(other))
        })
        .and_then(|count| count.checked_mul(4))
        .ok_or("canvas-extent")?;
    Ok(vec![0_u8; pixels])
}

fn push_prepared(
    prepared: &mut Vec<Prepared>,
    cases: &mut Vec<CurrentCase>,
    id: &str,
    rgba: Vec<u8>,
    json: &Value,
    reason: &'static str,
) {
    if !super::identifier_ok(id) {
        omit(cases, id, "identifier-refused");
        return;
    }
    let digest = super::sha256_hex(&rgba);
    let sidecar = published_sidecar(json, &digest);
    prepared.push(Prepared {
        id: id.to_owned(),
        rgba,
        digest,
        sidecar,
    });
    cases.push(CurrentCase {
        id: id.to_owned(),
        reason,
        wrote: true,
    });
}

fn published_sidecar(json: &Value, digest: &str) -> Value {
    let mut json = json.clone();
    if let Some(frame) = json.get_mut("frame").and_then(Value::as_object_mut) {
        frame.remove("statistics");
        frame.insert("sha256".to_owned(), Value::String(digest.to_owned()));
    }
    json
}

fn omit(cases: &mut Vec<CurrentCase>, id: &str, reason: &'static str) {
    cases.push(CurrentCase {
        id: id.to_owned(),
        reason,
        wrote: false,
    });
}

fn digests_of(prepared: &[Prepared]) -> BTreeMap<String, String> {
    prepared
        .iter()
        .map(|frame| (frame.id.clone(), frame.digest.clone()))
        .collect()
}

fn rewrite_run(reference: &Value, written: &BTreeMap<String, String>) -> Value {
    let mut json = reference.clone();
    if let Some(rows) = json.get_mut("rows").and_then(Value::as_array_mut) {
        rows.retain(|row| keep_row(row, written));
    }
    if let Some(volumes) = json.get_mut("volumes").and_then(Value::as_array_mut) {
        for volume in volumes.iter_mut() {
            rewrite_volume(volume, written);
        }
    }
    json
}

fn keep_row(row: &Value, written: &BTreeMap<String, String>) -> bool {
    if row.get("ok").and_then(Value::as_bool) != Some(true) {
        return true;
    }
    row.get("id")
        .and_then(Value::as_str)
        .is_some_and(|id| written.contains_key(id))
}

fn rewrite_volume(volume: &mut Value, written: &BTreeMap<String, String>) {
    if volume.get("outcome").and_then(Value::as_str) == Some("refused-as-declared") {
        return;
    }
    let Some(frames) = volume.get_mut("frames").and_then(Value::as_array_mut) else {
        return;
    };
    frames.retain(|frame| {
        frame
            .get("id")
            .and_then(Value::as_str)
            .is_some_and(|id| written.contains_key(id))
    });
    for frame in frames.iter_mut() {
        let Some(id) = frame.get("id").and_then(Value::as_str).map(str::to_owned) else {
            continue;
        };
        let Some(digest) = written.get(&id).cloned() else {
            continue;
        };
        if let Some(object) = frame.as_object_mut() {
            object.insert("sha256".to_owned(), Value::String(digest));
        }
    }
}

fn unsupported_rows(json: &Value) -> usize {
    json.pointer("/rows")
        .and_then(Value::as_array)
        .map_or(0, |rows| {
            rows.iter()
                .filter(|row| row.get("ok").and_then(Value::as_bool) == Some(false))
                .count()
        })
}

fn write_frame(directory: &Path, frame: &Prepared) -> Result<(), ()> {
    let raw = directory.join(format!("{}.raw", frame.id));
    let json_path = directory.join(format!("{}.json", frame.id));
    let text = serde_json::to_vec_pretty(&frame.sidecar).map_err(|_| ())?;
    fs::write(&raw, &frame.rgba).map_err(|_| ())?;
    if fs::write(&json_path, text).is_err() {
        let _ = fs::remove_file(&raw);
        return Err(());
    }
    Ok(())
}

fn stop_reason(stop: RowStop) -> &'static str {
    stop.reason
}

fn directories_conflict(reference: &Path, candidate: &Path) -> bool {
    let Ok(reference) = reference.canonicalize() else {
        return false;
    };
    if candidate.exists() {
        let Ok(candidate) = candidate.canonicalize() else {
            return true;
        };
        return reference == candidate
            || candidate.starts_with(&reference)
            || reference.starts_with(&candidate);
    }
    let Ok(candidate) = absolute_path(candidate) else {
        return true;
    };
    candidate.starts_with(&reference) || reference.starts_with(&candidate)
}

fn absolute_path(path: &Path) -> Result<PathBuf, ()> {
    if path.is_absolute() {
        return Ok(path.to_path_buf());
    }
    Ok(std::env::current_dir().map_err(|_| ())?.join(path))
}

fn staging_path(candidate: &Path) -> Option<PathBuf> {
    let name = candidate.file_name()?.to_str()?;
    let mut staging = candidate.to_path_buf();
    staging.set_file_name(format!("{name}.ocelli-partial"));
    Some(staging)
}

fn reset_directory(path: &Path) -> Result<(), ()> {
    if path.exists() {
        fs::remove_dir_all(path).map_err(|_| ())?;
    }
    fs::create_dir_all(path).map_err(|_| ())
}

fn abort_directory(path: &Path) {
    let _ = fs::remove_dir_all(path);
}

fn replace_directory(staging: &Path, candidate: &Path) -> Result<(), ()> {
    let Some(backup) = backup_path(candidate) else {
        abort_directory(staging);
        return Err(());
    };
    if candidate.exists() && fs::rename(candidate, &backup).is_err() {
        abort_directory(staging);
        return Err(());
    }
    if fs::rename(staging, candidate).is_err() {
        if backup.exists() {
            let _ = fs::rename(&backup, candidate);
        }
        abort_directory(staging);
        return Err(());
    }
    if backup.exists() {
        let _ = fs::remove_dir_all(&backup);
    }
    Ok(())
}

fn backup_path(candidate: &Path) -> Option<PathBuf> {
    let name = candidate.file_name()?.to_str()?;
    let mut backup = candidate.to_path_buf();
    backup.set_file_name(format!("{name}.ocelli-previous"));
    Some(backup)
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::{Path, PathBuf};

    use serde_json::Value;
    use sha2::{Digest, Sha256};

    use super::{CurrentPaths, produce_current};

    fn workspace_root() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
    }

    fn equipped() -> Option<(PathBuf, PathBuf)> {
        let root = workspace_root();
        let reference = root.join("tools/oracle/out");
        let corpus = root.join("corpus/data");
        if reference.join("run.json").is_file() && corpus.is_dir() {
            Some((reference, corpus))
        } else {
            None
        }
    }

    fn sha256_file(path: &Path) -> String {
        let bytes = fs::read(path).unwrap_or_default();
        let digest = Sha256::digest(&bytes);
        digest.iter().map(|byte| format!("{byte:02x}")).collect()
    }

    fn temp_dir(label: &str) -> PathBuf {
        let path =
            std::env::temp_dir().join(format!("ocelli-fx021-{label}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).unwrap_or(());
        path
    }

    fn copy_run(reference: &Path, dest: &Path, ids: &[&str]) -> Result<(), String> {
        fs::create_dir_all(dest).map_err(|error| error.to_string())?;
        let run_text = fs::read(reference.join("run.json")).map_err(|error| error.to_string())?;
        let mut run: Value =
            serde_json::from_slice(&run_text).map_err(|error| error.to_string())?;
        let wanted: std::collections::BTreeSet<&str> = ids.iter().copied().collect();
        if let Some(rows) = run.get_mut("rows").and_then(Value::as_array_mut) {
            rows.retain(|row| {
                row.get("ok").and_then(Value::as_bool) != Some(true)
                    || row
                        .get("id")
                        .and_then(Value::as_str)
                        .is_some_and(|id| wanted.contains(id))
            });
        }
        run.as_object_mut()
            .ok_or_else(|| "run.json is not an object".to_owned())?
            .insert("volumes".to_owned(), Value::Array(Vec::new()));
        for id in ids {
            fs::copy(
                reference.join(format!("{id}.json")),
                dest.join(format!("{id}.json")),
            )
            .map_err(|error| error.to_string())?;
            fs::copy(
                reference.join(format!("{id}.raw")),
                dest.join(format!("{id}.raw")),
            )
            .map_err(|error| error.to_string())?;
        }
        let text = serde_json::to_vec_pretty(&run).map_err(|error| error.to_string())?;
        fs::write(dest.join("run.json"), text).map_err(|error| error.to_string())?;
        Ok(())
    }

    fn copy_series_volume(
        reference: &Path,
        dest: &Path,
        volume_id: &str,
    ) -> Result<Vec<String>, String> {
        fs::create_dir_all(dest).map_err(|error| error.to_string())?;
        let run_text = fs::read(reference.join("run.json")).map_err(|error| error.to_string())?;
        let mut run: Value =
            serde_json::from_slice(&run_text).map_err(|error| error.to_string())?;
        let volumes = run
            .get("volumes")
            .and_then(Value::as_array)
            .cloned()
            .ok_or_else(|| "volumes are absent".to_owned())?;
        let volume = volumes
            .iter()
            .find(|item| item.get("id").and_then(Value::as_str) == Some(volume_id))
            .cloned()
            .ok_or_else(|| format!("{volume_id} is absent"))?;
        let frame_ids = frame_ids_of(&volume)?;
        let first = frame_ids
            .first()
            .ok_or_else(|| format!("{volume_id} has no frame"))?;
        let sidecar_text =
            fs::read(reference.join(format!("{first}.json"))).map_err(|error| error.to_string())?;
        let sidecar: Value =
            serde_json::from_slice(&sidecar_text).map_err(|error| error.to_string())?;
        let members = member_stems(&sidecar)?;
        let mut kept = Vec::new();
        for item in volumes {
            let id = item.get("id").and_then(Value::as_str);
            let outcome = item.get("outcome").and_then(Value::as_str);
            if id == Some(volume_id) || outcome == Some("refused-as-declared") {
                kept.push(item);
            }
        }
        if let Some(rows) = run.get_mut("rows").and_then(Value::as_array_mut) {
            rows.retain(|row| {
                row.get("ok").and_then(Value::as_bool) != Some(true)
                    || row
                        .get("id")
                        .and_then(Value::as_str)
                        .is_some_and(|id| members.iter().any(|member| member == id))
            });
        }
        run.as_object_mut()
            .ok_or_else(|| "run.json is not an object".to_owned())?
            .insert("volumes".to_owned(), Value::Array(kept));
        for id in members.iter().chain(frame_ids.iter()) {
            fs::copy(
                reference.join(format!("{id}.json")),
                dest.join(format!("{id}.json")),
            )
            .map_err(|error| error.to_string())?;
            fs::copy(
                reference.join(format!("{id}.raw")),
                dest.join(format!("{id}.raw")),
            )
            .map_err(|error| error.to_string())?;
        }
        let text = serde_json::to_vec_pretty(&run).map_err(|error| error.to_string())?;
        fs::write(dest.join("run.json"), text).map_err(|error| error.to_string())?;
        Ok(frame_ids)
    }

    fn frame_ids_of(volume: &Value) -> Result<Vec<String>, String> {
        let frames = volume
            .get("frames")
            .and_then(Value::as_array)
            .ok_or_else(|| "frames are absent".to_owned())?;
        let mut ids = Vec::new();
        for frame in frames {
            let id = frame
                .get("id")
                .and_then(Value::as_str)
                .ok_or_else(|| "a volume frame has no id".to_owned())?;
            ids.push(id.to_owned());
        }
        Ok(ids)
    }

    fn member_stems(sidecar: &Value) -> Result<Vec<String>, String> {
        let members = sidecar
            .pointer("/volume/members")
            .and_then(Value::as_array)
            .ok_or_else(|| "volume members are absent".to_owned())?;
        let mut stems = Vec::new();
        for member in members {
            let name = member
                .get("stackSidecar")
                .and_then(Value::as_str)
                .ok_or_else(|| "a volume member has no stack sidecar".to_owned())?;
            stems.push(name.strip_suffix(".json").unwrap_or(name).to_owned());
        }
        Ok(stems)
    }

    fn declared_frames(path: &Path, volume_id: &str) -> Result<Vec<String>, String> {
        let text = fs::read(path.join("run.json")).map_err(|error| error.to_string())?;
        let run: Value = serde_json::from_slice(&text).map_err(|error| error.to_string())?;
        let volumes = run
            .get("volumes")
            .and_then(Value::as_array)
            .ok_or_else(|| "volumes are absent".to_owned())?;
        let volume = volumes
            .iter()
            .find(|item| item.get("id").and_then(Value::as_str) == Some(volume_id))
            .ok_or_else(|| format!("{volume_id} is absent"))?;
        frame_ids_of(volume)
    }

    fn set_window(path: &Path, center: f64) -> Result<(), String> {
        let text = fs::read(path).map_err(|error| error.to_string())?;
        let mut json: Value = serde_json::from_slice(&text).map_err(|error| error.to_string())?;
        let voi = json
            .get_mut("voi")
            .and_then(Value::as_object_mut)
            .ok_or_else(|| "voi is absent".to_owned())?;
        let number = serde_json::Number::from_f64(center)
            .ok_or_else(|| "window center is not finite".to_owned())?;
        voi.insert("windowCenter".to_owned(), Value::Number(number));
        let text = serde_json::to_vec_pretty(&json).map_err(|error| error.to_string())?;
        fs::write(path, text).map_err(|error| error.to_string())?;
        Ok(())
    }

    #[test]
    fn a_missing_reference_writes_nothing() {
        let dir = temp_dir("missing");
        let summary = produce_current(&CurrentPaths {
            reference_run: dir.join("reference"),
            corpus_root: dir.join("corpus"),
            candidate_run: dir.join("candidate"),
        });
        assert_eq!(summary.resources, "reference-run-absent");
        assert!(!summary.wrote("anything"));
        assert!(!dir.join("candidate").join("run.json").exists());
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn the_candidate_may_not_be_the_reference() {
        let dir = temp_dir("overlap");
        let reference = dir.join("reference");
        fs::create_dir_all(&reference).unwrap_or(());
        let _ = fs::write(reference.join("run.json"), b"{}");
        let summary = produce_current(&CurrentPaths {
            reference_run: reference.clone(),
            corpus_root: dir.join("corpus"),
            candidate_run: reference.clone(),
        });
        assert_eq!(summary.resources, "output-directory-refused");
        assert!(reference.join("run.json").is_file());
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn window_change_moves_the_candidate_and_a_copy_does_not() -> Result<(), String> {
        let Some((reference, corpus)) = equipped() else {
            let dir = temp_dir("window-unrun");
            let summary = produce_current(&CurrentPaths {
                reference_run: dir.join("missing"),
                corpus_root: corpus_fallback(),
                candidate_run: dir.join("candidate"),
            });
            assert_eq!(summary.resources, "reference-run-absent");
            let _ = fs::remove_dir_all(&dir);
            return Ok(());
        };
        let id = "synthetic__ct_unsigned_16";
        let dir = temp_dir("window");
        let first_reference = dir.join("reference-a");
        let second_reference = dir.join("reference-b");
        copy_run(&reference, &first_reference, &[id])?;
        copy_run(&reference, &second_reference, &[id])?;
        let reference_raw = first_reference.join(format!("{id}.raw"));
        let before = sha256_file(&reference_raw);
        set_window(&second_reference.join(format!("{id}.json")), 4_000.0)?;
        let first = dir.join("candidate-a");
        let second = dir.join("candidate-b");
        let produced = produce_current(&CurrentPaths {
            reference_run: first_reference,
            corpus_root: corpus.clone(),
            candidate_run: first.clone(),
        });
        assert_eq!(produced.resources, "ran", "{produced:?}");
        assert!(produced.wrote(id), "{produced:?}");
        let moved = produce_current(&CurrentPaths {
            reference_run: second_reference,
            corpus_root: corpus,
            candidate_run: second.clone(),
        });
        assert_eq!(moved.resources, "ran", "{moved:?}");
        let first_digest = sha256_file(&first.join(format!("{id}.raw")));
        let second_digest = sha256_file(&second.join(format!("{id}.raw")));
        assert_ne!(
            first_digest, second_digest,
            "the producer must read the window it is given"
        );
        assert_eq!(sha256_file(&reference_raw), before);
        let copied = dir.join("copied.raw");
        fs::copy(&reference_raw, &copied).map_err(|error| error.to_string())?;
        let copied_again = dir.join("copied-again.raw");
        fs::copy(&reference_raw, &copied_again).map_err(|error| error.to_string())?;
        assert_eq!(sha256_file(&copied), sha256_file(&copied_again));
        let _ = fs::remove_dir_all(&dir);
        Ok(())
    }

    #[test]
    fn a_deleted_view_is_absent_and_a_missing_raw_is_structural() -> Result<(), String> {
        let Some((reference, corpus)) = equipped() else {
            return Ok(());
        };
        let id = "synthetic__ct_unsigned_16";
        let other = "synthetic__cr_monochrome1";
        let dir = temp_dir("absent");
        let reference_copy = dir.join("reference");
        copy_run(&reference, &reference_copy, &[id, other])?;
        let candidate = dir.join("candidate");
        let summary = produce_current(&CurrentPaths {
            reference_run: reference_copy.clone(),
            corpus_root: corpus,
            candidate_run: candidate.clone(),
        });
        assert_eq!(summary.resources, "ran", "{summary:?}");
        assert!(summary.wrote(id) && summary.wrote(other), "{summary:?}");
        let loaded_reference =
            crate::sidecar::Run::load(&reference_copy).map_err(|error| error.to_string())?;
        let mut run_text =
            fs::read(candidate.join("run.json")).map_err(|error| error.to_string())?;
        let mut run: Value =
            serde_json::from_slice(&run_text).map_err(|error| error.to_string())?;
        if let Some(rows) = run.get_mut("rows").and_then(Value::as_array_mut) {
            rows.retain(|row| row.get("id").and_then(Value::as_str) != Some(other));
        }
        run_text = serde_json::to_vec_pretty(&run).map_err(|error| error.to_string())?;
        fs::write(candidate.join("run.json"), &run_text).map_err(|error| error.to_string())?;
        let _ = fs::remove_file(candidate.join(format!("{other}.raw")));
        let _ = fs::remove_file(candidate.join(format!("{other}.json")));
        let loaded = crate::sidecar::Run::load(&candidate).map_err(|error| error.to_string())?;
        assert!(loaded_reference.views.contains_key(other));
        assert!(!loaded.views.contains_key(other));
        let _ = fs::remove_file(candidate.join(format!("{id}.raw")));
        let error = crate::sidecar::Run::load(&candidate).map_err(|error| error.to_string());
        let Err(message) = error else {
            return Err("a declared id with no raw loaded".to_owned());
        };
        assert!(
            message.contains(id) && message.contains("`.raw`"),
            "{message}"
        );
        let _ = fs::remove_dir_all(&dir);
        Ok(())
    }

    #[test]
    fn colour_is_emitted_and_not_recorded_as_pass() -> Result<(), String> {
        let Some((reference, corpus)) = equipped() else {
            return Ok(());
        };
        let rows = [
            "synthetic__sc_rgb_interleaved",
            "synthetic__sc_rgb_planar",
            "syntax__reference_rgb8",
            "synthetic__sc_palette_color",
            "synthetic__sc_palette_color_16",
            "syntax__jpeg_baseline_rgb8",
            "syntax__jpegls_lossless_rgb8",
            "real__us_cmb_crc__00000001",
        ];
        let dir = temp_dir("colour");
        let reference_copy = dir.join("reference");
        copy_run(&reference, &reference_copy, &rows)?;
        let candidate = dir.join("candidate");
        let summary = produce_current(&CurrentPaths {
            reference_run: reference_copy,
            corpus_root: corpus,
            candidate_run: candidate.clone(),
        });
        assert_eq!(summary.resources, "ran", "{summary:?}");
        for id in [
            "synthetic__sc_rgb_interleaved",
            "synthetic__sc_rgb_planar",
            "syntax__reference_rgb8",
        ] {
            assert!(summary.wrote(id), "{id} {summary:?}");
            assert_eq!(summary.reason(id), Some("colour-not-scored"));
            assert!(candidate.join(format!("{id}.raw")).is_file());
        }
        for id in [
            "synthetic__sc_palette_color",
            "synthetic__sc_palette_color_16",
        ] {
            assert!(summary.wrote(id), "{id} {summary:?}");
            assert_eq!(summary.reason(id), Some("palette-not-scored"));
        }
        assert!(summary.wrote("syntax__jpeg_baseline_rgb8"), "{summary:?}");
        assert_eq!(
            summary.reason("syntax__jpeg_baseline_rgb8"),
            Some("ybr-not-scored")
        );
        assert!(summary.wrote("syntax__jpegls_lossless_rgb8"), "{summary:?}");
        assert_eq!(
            summary.reason("syntax__jpegls_lossless_rgb8"),
            Some("colour-not-scored")
        );
        assert_eq!(summary.reason("real__us_cmb_crc__00000001"), Some("mono"));
        let _ = fs::remove_dir_all(&dir);
        Ok(())
    }

    #[test]
    fn uniform_and_endpoint_reformats_match_the_reference() -> Result<(), String> {
        let Some((reference, corpus)) = equipped() else {
            return Ok(());
        };
        let uniform = "volume__synthetic__ct_series_uniform";
        let nonuniform = "volume__synthetic__ct_series_nonuniform";
        let dir = temp_dir("volumes");
        let uniform_reference = dir.join("uniform-reference");
        let nonuniform_reference = dir.join("nonuniform-reference");
        let uniform_frames = copy_series_volume(&reference, &uniform_reference, uniform)?;
        let nonuniform_frames = copy_series_volume(&reference, &nonuniform_reference, nonuniform)?;
        assert_eq!(uniform_frames.len(), 3);
        assert_eq!(nonuniform_frames.len(), 3);
        let uniform_candidate = dir.join("uniform-candidate");
        let produced = produce_current(&CurrentPaths {
            reference_run: uniform_reference,
            corpus_root: corpus.clone(),
            candidate_run: uniform_candidate.clone(),
        });
        assert_eq!(produced.resources, "ran", "{produced:?}");
        for id in &uniform_frames {
            assert!(produced.wrote(id), "{id} {produced:?}");
            assert!(uniform_candidate.join(format!("{id}.raw")).is_file());
            assert_ne!(produced.reason(id), Some("pass"));
        }
        let loaded =
            crate::sidecar::Run::load(&uniform_candidate).map_err(|error| error.to_string())?;
        for id in &uniform_frames {
            assert!(loaded.views.contains_key(id), "{id}");
        }
        assert!(
            declared_frames(&uniform_candidate, "volume__real__ct_cmb_mml")?.is_empty(),
            "the declared refusal keeps an empty frame list"
        );
        assert_eq!(volume_outcome(&uniform_candidate, uniform)?, "built");
        assert_eq!(
            volume_outcome(&uniform_candidate, "volume__real__ct_cmb_mml")?,
            "refused-as-declared"
        );
        let nonuniform_candidate = dir.join("nonuniform-candidate");
        let omitted = produce_current(&CurrentPaths {
            reference_run: nonuniform_reference.clone(),
            corpus_root: corpus,
            candidate_run: nonuniform_candidate.clone(),
        });
        assert_eq!(omitted.resources, "ran", "{omitted:?}");
        for id in &nonuniform_frames {
            assert!(omitted.wrote(id), "{id} {omitted:?}");
            assert_ne!(omitted.reason(id), Some("pass"));
            let expected = fs::read(nonuniform_reference.join(format!("{id}.raw")))
                .map_err(|error| error.to_string())?;
            let actual = fs::read(nonuniform_candidate.join(format!("{id}.raw")))
                .map_err(|error| error.to_string())?;
            assert_eq!(actual.len(), expected.len(), "{id}");
            let mismatches = actual
                .iter()
                .zip(expected.iter())
                .filter(|(left, right)| left != right)
                .count();
            assert_eq!(mismatches, 0, "{id} has {mismatches} differing bytes");
        }
        assert_eq!(
            declared_frames(&nonuniform_candidate, nonuniform)?,
            nonuniform_frames
        );
        assert_eq!(volume_outcome(&nonuniform_candidate, nonuniform)?, "built");
        let _ = fs::remove_dir_all(&dir);
        Ok(())
    }

    #[test]
    fn uniform_reformat_bytes_match_the_reference() -> Result<(), String> {
        let Some((reference, corpus)) = equipped() else {
            return Ok(());
        };
        let ids = [
            "volume__synthetic__ct_series_uniform__AXIAL",
            "volume__synthetic__ct_series_uniform__CORONAL",
            "volume__synthetic__ct_series_uniform__SAGITTAL",
        ];
        let dir = temp_dir("uniform-bytes");
        let reference_copy = dir.join("reference");
        copy_series_volume(
            &reference,
            &reference_copy,
            "volume__synthetic__ct_series_uniform",
        )?;
        let candidate = dir.join("candidate");
        let summary = produce_current(&CurrentPaths {
            reference_run: reference_copy.clone(),
            corpus_root: corpus,
            candidate_run: candidate.clone(),
        });
        assert_eq!(summary.resources, "ran", "{summary:?}");
        for id in ids {
            assert!(summary.wrote(id), "{id} {summary:?}");
            let expected = fs::read(reference_copy.join(format!("{id}.raw")))
                .map_err(|error| error.to_string())?;
            let actual =
                fs::read(candidate.join(format!("{id}.raw"))).map_err(|error| error.to_string())?;
            assert_eq!(actual.len(), expected.len(), "{id}");
            let mismatches = actual
                .iter()
                .zip(expected.iter())
                .filter(|(left, right)| left != right)
                .count();
            assert_eq!(mismatches, 0, "{id} has {mismatches} differing bytes");
        }
        let _ = fs::remove_dir_all(&dir);
        Ok(())
    }

    #[test]
    fn a_missing_member_omits_the_volume_frame() -> Result<(), String> {
        let Some((reference, corpus)) = equipped() else {
            return Ok(());
        };
        let volume_id = "volume__synthetic__ct_series_uniform";
        let dir = temp_dir("member");
        let reference_copy = dir.join("reference");
        let frames = copy_series_volume(&reference, &reference_copy, volume_id)?;
        let member = reference_copy.join("synthetic__ct_series_uniform__slice_000.json");
        let text = fs::read(&member).map_err(|error| error.to_string())?;
        let mut sidecar: Value =
            serde_json::from_slice(&text).map_err(|error| error.to_string())?;
        let row = sidecar
            .get_mut("row")
            .and_then(Value::as_object_mut)
            .ok_or_else(|| "row is absent".to_owned())?;
        row.insert(
            "path".to_owned(),
            Value::String("missing/not-a-file.dcm".to_owned()),
        );
        let text = serde_json::to_vec_pretty(&sidecar).map_err(|error| error.to_string())?;
        fs::write(&member, text).map_err(|error| error.to_string())?;
        let candidate = dir.join("candidate");
        let summary = produce_current(&CurrentPaths {
            reference_run: reference_copy,
            corpus_root: corpus,
            candidate_run: candidate.clone(),
        });
        assert_eq!(summary.resources, "ran", "{summary:?}");
        for id in &frames {
            assert!(!summary.wrote(id), "{id} {summary:?}");
            assert_eq!(summary.reason(id), Some("volume-member-omitted"));
        }
        assert!(declared_frames(&candidate, volume_id)?.is_empty());
        crate::sidecar::Run::load(&candidate).map_err(|error| error.to_string())?;
        let _ = fs::remove_dir_all(&dir);
        Ok(())
    }

    #[test]
    fn real_mr_reformats_match_the_endpoint_grid() -> Result<(), String> {
        let Some((reference, corpus)) = equipped() else {
            return Ok(());
        };
        let volume_id = "volume__real__mr_eay131";
        let dir = temp_dir("mr-spacing");
        let reference_copy = dir.join("reference");
        let frames = copy_series_volume(&reference, &reference_copy, volume_id)?;
        let candidate = dir.join("candidate");
        let summary = produce_current(&CurrentPaths {
            reference_run: reference_copy.clone(),
            corpus_root: corpus.clone(),
            candidate_run: candidate.clone(),
        });
        assert_eq!(summary.resources, "ran", "{summary:?}");
        for id in &frames {
            assert!(summary.wrote(id), "{id} {summary:?}");
            assert_ne!(summary.reason(id), Some("pass"));
            let expected = fs::read(reference_copy.join(format!("{id}.raw")))
                .map_err(|error| error.to_string())?;
            let actual =
                fs::read(candidate.join(format!("{id}.raw"))).map_err(|error| error.to_string())?;
            assert_class_one(&actual, &expected, id);
        }
        assert_eq!(declared_frames(&candidate, volume_id)?, frames);
        assert_eq!(volume_outcome(&candidate, volume_id)?, "built");
        let _ = fs::remove_dir_all(&dir);
        Ok(())
    }

    #[test]
    fn a_left_aligned_high_bit_matches_the_unshifted_reference() -> Result<(), String> {
        let Some((reference, corpus)) = equipped() else {
            return Ok(());
        };
        let id = "synthetic__ct_signed_12in16_left";
        let dir = temp_dir("high-bit");
        let reference_copy = dir.join("reference");
        copy_run(&reference, &reference_copy, &[id])?;
        let candidate = dir.join("candidate");
        let summary = produce_current(&CurrentPaths {
            reference_run: reference_copy.clone(),
            corpus_root: corpus,
            candidate_run: candidate.clone(),
        });
        assert_eq!(summary.resources, "ran", "{summary:?}");
        assert!(summary.wrote(id), "{summary:?}");
        let produced =
            fs::read(candidate.join(format!("{id}.raw"))).map_err(|error| error.to_string())?;
        let reference_raw = fs::read(reference_copy.join(format!("{id}.raw")))
            .map_err(|error| error.to_string())?;
        assert_eq!(produced.len(), reference_raw.len(), "{id}");
        let mismatches = produced
            .iter()
            .zip(reference_raw.iter())
            .filter(|(left, right)| left != right)
            .count();
        assert_eq!(mismatches, 0, "{id} has {mismatches} differing bytes");
        let reference_views =
            crate::sidecar::Run::load(&reference_copy).map_err(|error| error.to_string())?;
        let candidate_views =
            crate::sidecar::Run::load(&candidate).map_err(|error| error.to_string())?;
        assert!(reference_views.views.contains_key(id));
        assert!(candidate_views.views.contains_key(id));
        let _ = fs::remove_dir_all(&dir);
        Ok(())
    }

    #[test]
    fn parallel_scale_moves_the_reformat_and_a_copy_does_not() -> Result<(), String> {
        let Some((reference, corpus)) = equipped() else {
            return Ok(());
        };
        let axial = "volume__synthetic__ct_series_uniform__AXIAL";
        let dir = temp_dir("reformat-scale");
        let reference_copy = dir.join("reference");
        copy_series_volume(
            &reference,
            &reference_copy,
            "volume__synthetic__ct_series_uniform",
        )?;
        let first_dir = dir.join("first");
        let first = produce_current(&CurrentPaths {
            reference_run: reference_copy.clone(),
            corpus_root: corpus.clone(),
            candidate_run: first_dir.clone(),
        });
        assert_eq!(first.resources, "ran", "{first:?}");
        assert!(first.wrote(axial), "{first:?}");
        let reference_hash = sha256_file(&reference_copy.join(format!("{axial}.raw")));
        let first_hash = sha256_file(&first_dir.join(format!("{axial}.raw")));
        assert_eq!(
            first_hash, reference_hash,
            "axial reformat disagrees with the reference frame"
        );
        set_parallel_scale(&reference_copy.join(format!("{axial}.json")), 23.0)?;
        let second_dir = dir.join("second");
        let second = produce_current(&CurrentPaths {
            reference_run: reference_copy,
            corpus_root: corpus,
            candidate_run: second_dir.clone(),
        });
        assert_eq!(second.resources, "ran", "{second:?}");
        assert!(second.wrote(axial), "{second:?}");
        let second_hash = sha256_file(&second_dir.join(format!("{axial}.raw")));
        assert_ne!(
            second_hash, first_hash,
            "parallelScale did not move the reformat"
        );
        let published = fs::read(second_dir.join(format!("{axial}.json")))
            .map_err(|error| error.to_string())?;
        let published: Value =
            serde_json::from_slice(&published).map_err(|error| error.to_string())?;
        let scale = published
            .pointer("/camera/parallelScale")
            .and_then(Value::as_f64);
        assert!((scale.unwrap_or(0.0) - 23.0).abs() < 1e-9);
        let _ = fs::remove_dir_all(&dir);
        Ok(())
    }

    fn set_parallel_scale(path: &Path, scale: f64) -> Result<(), String> {
        let text = fs::read(path).map_err(|error| error.to_string())?;
        let mut sidecar: Value =
            serde_json::from_slice(&text).map_err(|error| error.to_string())?;
        let camera = sidecar
            .get_mut("camera")
            .and_then(Value::as_object_mut)
            .ok_or_else(|| "camera is absent".to_owned())?;
        let number =
            serde_json::Number::from_f64(scale).ok_or_else(|| "scale is not finite".to_owned())?;
        camera.insert("parallelScale".to_owned(), Value::Number(number));
        let text = serde_json::to_vec_pretty(&sidecar).map_err(|error| error.to_string())?;
        fs::write(path, text).map_err(|error| error.to_string())?;
        Ok(())
    }

    fn volume_outcome(path: &Path, volume_id: &str) -> Result<String, String> {
        let text = fs::read(path.join("run.json")).map_err(|error| error.to_string())?;
        let json: Value = serde_json::from_slice(&text).map_err(|error| error.to_string())?;
        let volumes = json
            .get("volumes")
            .and_then(Value::as_array)
            .ok_or_else(|| "volumes are absent".to_owned())?;
        for volume in volumes {
            if volume.get("id").and_then(Value::as_str) == Some(volume_id) {
                return volume
                    .get("outcome")
                    .and_then(Value::as_str)
                    .map(str::to_owned)
                    .ok_or_else(|| format!("{volume_id} has no outcome"));
            }
        }
        Err(format!("{volume_id} is absent"))
    }

    fn corpus_fallback() -> PathBuf {
        workspace_root().join("corpus/data")
    }

    /// Section 25.1 class one. A one-code edge residual stays inside the bound.
    /// Placement that paints a different silhouette does not.
    fn assert_class_one(candidate: &[u8], reference: &[u8], id: &str) {
        use crate::tolerance::{
            MONOCHROME_MAX_ABS_DIFF, MONOCHROME_SIGNED_MEAN_BIAS,
            MONOCHROME_WITHIN_ONE_LSB_FRACTION,
        };
        assert_eq!(candidate.len(), reference.len(), "{id}");
        let mut within_one = 0_u32;
        let mut over_max = 0_u32;
        let mut pixels = 0_u32;
        let mut informative = 0_u32;
        let mut signed = 0_i32;
        let mut worst = 0_u8;
        for (left, right) in candidate.chunks(4).zip(reference.chunks(4)) {
            let (Some(got), Some(want)) = (left.first(), right.first()) else {
                continue;
            };
            pixels = pixels.saturating_add(1);
            let delta = got.abs_diff(*want);
            if delta <= 1 {
                within_one = within_one.saturating_add(1);
            }
            if delta > MONOCHROME_MAX_ABS_DIFF {
                over_max = over_max.saturating_add(1);
            }
            if delta > worst {
                worst = delta;
            }
            let same_extreme = left
                .iter()
                .zip(right.iter())
                .all(|(item, other)| *item == *other && (*item == 0 || *item == u8::MAX));
            if !same_extreme {
                informative = informative.saturating_add(1);
                signed = signed.saturating_add(i32::from(*got) - i32::from(*want));
            }
        }
        let fraction = if pixels == 0 {
            0.0
        } else {
            f64::from(within_one) / f64::from(pixels)
        };
        let mean = if informative == 0 {
            0.0
        } else {
            f64::from(signed) / f64::from(informative)
        };
        assert!(
            fraction >= MONOCHROME_WITHIN_ONE_LSB_FRACTION && over_max == 0,
            "{id} fraction {fraction} over {over_max} worst {worst}"
        );
        assert!(
            mean.abs() <= MONOCHROME_SIGNED_MEAN_BIAS,
            "{id} signed mean {mean} over {informative} informative pixels"
        );
    }
}
