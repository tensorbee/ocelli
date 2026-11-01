//! Applicable monochrome stack frames for the existing comparator.
//!
//! One settled CPU frame is written for each reference stack row this raster
//! can draw. Colour, palette, YBR, volume reformats and rows the reference
//! did not render are recorded as unsupported and are not scored. The
//! directories are the input of `ocelli-compare gate`. This module does not
//! add a command, open an adapter, or record a host sample.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use dicom_core::PrimitiveValue;
use ocelli_codec::{
    DecodePhotometricInterpretation, FrameDesc, FrameDescInput, PixelDataVr, PixelRepresentation,
    Registry, register_htj2k_decoders, register_jpeg_decoders, register_jpeg2000_decoders,
    register_jpegls_decoders, register_native_and_rle_decoders,
};
use ocelli_core::{Canvas, Index, Pt, Stored, Transform};
use ocelli_dicom::{DispatchPath, EncapsulatedFrameIndex, Tag, parse_part10};
use ocelli_pixel::{
    ByteOrder, ImageDimensions, ImageOrientationPatient, ImagePlane, ImagePositionPatient,
    LutChain, ModalityTransform, PhotometricInterpretation, PixelSpacing, PlanarConfiguration,
    PresentationLutEvidence, SampleLayout, StoredBits, StoredPixelDescription, VoiFunction,
    VoiTransform,
};
use ocelli_render::{CpuRaster, RasterGrid, RasterQuality};
use ocelli_viewport::presentation::{Colormap, Presentation};
use ocelli_viewport::view::{INDEX_TO_IMAGE_CENTRE, View, Zoom, f32_from_f64};

use crate::tolerance::WORLD_TOLERANCE_MM;
use serde_json::{Map, Value};
use sha2::{Digest, Sha256};

use crate::attribution::{Context, Register, compare_view};
use crate::report::Outcome;
use crate::sidecar::{Run, Sidecar, ViewKind};

#[path = "produce_current.rs"]
mod current;

pub use current::{CurrentPaths, CurrentSummary, produce_current};

const PIXEL_DATA: Tag = Tag(0x7FE0, 0x0010);
const TIER_A: &str = "unrun";
const INSTRUMENTS: &str = "not-run-by-producer";
const COLLECTOR: &str = "not-run-by-producer";
const HOST_MEASUREMENT: &str = "unavailable";

/// Directories the producer reads and writes.
///
/// `reference_subset` and `candidate_run` must be different from each other
/// and must not be the reference run. The producer replaces those two
/// directories when it writes.
pub struct ProducePaths {
    /// Reference-half directory containing `run.json`.
    pub reference_run: PathBuf,
    /// Permitted corpus root. Manifest paths are joined under it.
    pub corpus_root: PathBuf,
    /// Subset of the reference frames that were produced.
    pub reference_subset: PathBuf,
    /// Ocelli candidate frames for that same subset.
    pub candidate_run: PathBuf,
    /// Disposition record. It belongs under an ignored directory.
    pub dispositions: PathBuf,
}

/// Counts for one producer execution.
///
/// Case identifiers stay out of [`Debug`] so a failing assertion does not
/// print a corpus listing. [`Self::case_reason`] reads one identifier.
#[derive(Clone, PartialEq, Eq)]
pub struct ProduceSummary {
    /// `ran`, `reference-run-absent`, `reference-run-unreadable`,
    /// `corpus-absent`, or `output-directory-refused`.
    pub resources: &'static str,
    /// Monochrome stack rows the raster was asked to draw.
    pub selected: usize,
    /// Rows whose candidate frame was written.
    pub produced: usize,
    /// Rows [`compare_view`] returned a record for.
    pub compared: usize,
    /// Rows that were not selected.
    pub unsupported: usize,
    /// Production failures, pixel failures and comparator refusals.
    pub failed: usize,
    /// Rows or runs that were not executed.
    pub unrun: usize,
    /// Compared rows whose outcome is pass.
    pub pass: usize,
    /// Compared rows whose outcome is fail.
    pub outcome_fail: usize,
    /// Compared rows whose outcome is unmeasured.
    pub unmeasured: usize,
    /// Selected rows that did not produce a frame.
    pub produce_failed: usize,
    /// Written rows the comparator refused.
    pub comparator_refused: usize,
    /// Unsupported rows plus selected rows.
    pub recorded: usize,
    /// Tier A was not opened.
    pub tier_a: &'static str,
    /// Interaction instruments were not implemented.
    pub instruments: &'static str,
    /// CPU-time and wall-time collection was not configured.
    pub collector: &'static str,
    /// The representative GPU-less host was not available.
    pub host_measurement: &'static str,
    cases: Vec<CaseRecord>,
}

#[derive(Clone, PartialEq, Eq)]
struct CaseRecord {
    id: String,
    disposition: &'static str,
    reason: &'static str,
}

struct Ready {
    id: String,
    rgba: Vec<u8>,
    width: u32,
    height: u32,
    digest: String,
}

struct RowStop {
    disposition: &'static str,
    reason: &'static str,
}

impl RowStop {
    fn failed(reason: &'static str) -> Self {
        Self {
            disposition: "failed",
            reason,
        }
    }

    fn unrun(reason: &'static str) -> Self {
        Self {
            disposition: "unrun",
            reason,
        }
    }
}

impl ProduceSummary {
    /// Whether the recorded counts describe one execution.
    ///
    /// A missing reference run is one unrun and no selected row. A loaded run
    /// balances selected rows against produced, failed and unrun rows.
    #[must_use]
    pub fn accounts(&self) -> bool {
        let compared = self.compared == self.pass + self.outcome_fail + self.unmeasured;
        let produced = self.produced == self.compared + self.comparator_refused;
        let failed =
            self.failed == self.produce_failed + self.outcome_fail + self.comparator_refused;
        if self.resources != "ran" && self.resources != "corpus-absent" {
            return self.selected == 0
                && self.produced == 0
                && self.compared == 0
                && self.recorded == 0
                && self.unrun == 1
                && compared
                && produced
                && failed;
        }
        let selected = self.selected == self.produced + self.produce_failed + self.unrun;
        let recorded = self.unsupported + self.selected == self.recorded;
        selected && recorded && compared && produced && failed
    }

    /// Static reason for one view identifier.
    #[must_use]
    pub fn case_reason(&self, id: &str) -> Option<&str> {
        self.cases
            .iter()
            .find(|case| case.id == id)
            .map(|case| case.reason)
    }

    /// `unsupported`, `unrun`, `failed`, or `compared`.
    #[must_use]
    pub fn case_disposition(&self, id: &str) -> Option<&str> {
        self.cases
            .iter()
            .find(|case| case.id == id)
            .map(|case| case.disposition)
    }

    /// How many cases carry this static reason.
    #[must_use]
    pub fn reason_count(&self, reason: &str) -> usize {
        self.cases
            .iter()
            .filter(|case| case.reason == reason)
            .count()
    }

    fn reason_counts(&self) -> BTreeMap<&str, usize> {
        let mut counts = BTreeMap::new();
        for case in &self.cases {
            *counts.entry(case.reason).or_insert(0) += 1;
        }
        counts
    }
}

impl std::fmt::Debug for ProduceSummary {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("ProduceSummary")
            .field("resources", &self.resources)
            .field("selected", &self.selected)
            .field("produced", &self.produced)
            .field("compared", &self.compared)
            .field("unsupported", &self.unsupported)
            .field("failed", &self.failed)
            .field("unrun", &self.unrun)
            .field("pass", &self.pass)
            .field("outcome_fail", &self.outcome_fail)
            .field("unmeasured", &self.unmeasured)
            .field("produce_failed", &self.produce_failed)
            .field("comparator_refused", &self.comparator_refused)
            .field("recorded", &self.recorded)
            .field("tier_a", &self.tier_a)
            .field("instruments", &self.instruments)
            .field("collector", &self.collector)
            .field("host_measurement", &self.host_measurement)
            .field("reasons", &self.reason_counts())
            .finish()
    }
}

/// Why a rendered stack sidecar is outside the scored monochrome set.
///
/// `None` means the row is selected. The caller still has to reject a row
/// the reference did not render, a volume reformat, or an unsafe identifier.
#[must_use]
pub fn stack_selection_reason(sidecar: &Value) -> Option<&'static str> {
    let photometric = sidecar
        .pointer("/attributes/photometricInterpretation")
        .and_then(Value::as_str)
        .unwrap_or("");
    let samples = sidecar
        .pointer("/attributes/samplesPerPixel")
        .and_then(Value::as_u64);
    let color = sidecar.pointer("/image/color").and_then(Value::as_bool);
    let mono16 = sidecar
        .pointer("/row/categories")
        .and_then(Value::as_array)
        .is_some_and(|categories| {
            categories
                .iter()
                .any(|category| category.as_str() == Some("mono16"))
        });
    if photometric == "PALETTE COLOR" {
        return Some("palette-not-scored");
    }
    if photometric.starts_with("YBR") {
        return Some("ybr-not-scored");
    }
    if color == Some(true) || photometric == "RGB" {
        return Some("colour-not-scored");
    }
    if photometric != "MONOCHROME1" && photometric != "MONOCHROME2" {
        return Some("photometric-unsupported");
    }
    if samples != Some(1) || color != Some(false) || !mono16 {
        return Some("class-not-monochrome");
    }
    None
}

/// Produce the applicable monochrome candidate and compare each written frame.
///
/// A missing reference run, a missing corpus root, or an output directory that
/// would replace the reference run returns without claiming a comparison.
/// Pixel disagreements stay in the failed and compared counts.
#[must_use]
pub fn produce_applicable(paths: &ProducePaths) -> ProduceSummary {
    produce_with(paths, None)
}

/// Produce the same bounded stack set through the actual native tier A graph.
#[must_use]
pub fn produce_tier_a(paths: &ProducePaths) -> ProduceSummary {
    match crate::gpu_candidate::GpuStack::open() {
        Ok(mut gpu) => produce_with(paths, Some(&mut gpu)),
        Err(reason) => {
            let summary = bare(reason);
            write_dispositions(&paths.dispositions, &summary);
            summary
        }
    }
}

fn produce_with(
    paths: &ProducePaths,
    gpu: Option<&mut crate::gpu_candidate::GpuStack>,
) -> ProduceSummary {
    let tier_a = gpu.is_some();
    let execution = gpu.as_ref().map_or_else(
        || serde_json::json!({"tier": "C", "entry": "native CpuRaster", "browser": false}),
        |adapter| adapter.execution(),
    );
    if !paths.reference_run.join("run.json").is_file() {
        let summary = bare("reference-run-absent");
        write_dispositions(&paths.dispositions, &summary);
        return summary;
    }
    let reference = match Run::load(&paths.reference_run) {
        Ok(run) => run,
        Err(_) => {
            let summary = bare("reference-run-unreadable");
            write_dispositions(&paths.dispositions, &summary);
            return summary;
        }
    };
    if output_overlaps(&paths.reference_run, &paths.reference_subset)
        || output_overlaps(&paths.reference_run, &paths.candidate_run)
        || output_overlaps(&paths.reference_subset, &paths.candidate_run)
    {
        let summary = bare("output-directory-refused");
        write_dispositions(&paths.dispositions, &summary);
        return summary;
    }

    let corpus_available = paths.corpus_root.is_dir();
    let mut cases = Vec::new();
    let mut ready = Vec::new();
    classify_rows(
        &reference,
        corpus_available,
        &paths.corpus_root,
        &mut cases,
        &mut ready,
        gpu,
    );
    classify_volumes(&reference, &mut cases);

    if !corpus_available {
        let summary = finish("corpus-absent", cases);
        write_dispositions(&paths.dispositions, &summary);
        return summary;
    }
    if reset_directory(&paths.reference_subset).is_err()
        || reset_directory(&paths.candidate_run).is_err()
    {
        for frame in ready {
            cases.push(CaseRecord {
                id: frame.id,
                disposition: "failed",
                reason: "output-directory-refused",
            });
        }
        let summary = finish("ran", cases);
        write_dispositions(&paths.dispositions, &summary);
        return summary;
    }

    let mut written = Vec::new();
    for frame in ready {
        match write_pair(&reference, paths, &frame) {
            Ok(()) => written.push(frame),
            Err(reason) => cases.push(CaseRecord {
                id: frame.id,
                disposition: "failed",
                reason,
            }),
        }
    }
    let produced_ids: BTreeSet<String> = written.iter().map(|frame| frame.id.clone()).collect();
    if write_subset_runs(&reference, paths, &produced_ids, &written, &execution).is_err() {
        for frame in written {
            cases.push(CaseRecord {
                id: frame.id,
                disposition: "failed",
                reason: "output-write-refused",
            });
        }
        let summary = finish("ran", cases);
        write_dispositions(&paths.dispositions, &summary);
        return summary;
    }

    compare_written(
        &paths.reference_subset,
        &paths.candidate_run,
        &written,
        &mut cases,
    );
    let mut summary = finish("ran", cases);
    if tier_a && summary.produced > 0 {
        summary.tier_a = "ran";
    }
    write_dispositions(&paths.dispositions, &summary);
    summary
}

fn bare(resources: &'static str) -> ProduceSummary {
    ProduceSummary {
        resources,
        selected: 0,
        produced: 0,
        compared: 0,
        unsupported: 0,
        failed: 0,
        unrun: 1,
        pass: 0,
        outcome_fail: 0,
        unmeasured: 0,
        produce_failed: 0,
        comparator_refused: 0,
        recorded: 0,
        tier_a: TIER_A,
        instruments: INSTRUMENTS,
        collector: COLLECTOR,
        host_measurement: HOST_MEASUREMENT,
        cases: Vec::new(),
    }
}

fn finish(resources: &'static str, cases: Vec<CaseRecord>) -> ProduceSummary {
    let selected = cases
        .iter()
        .filter(|case| case.disposition != "unsupported")
        .count();
    let unsupported = cases.len() - selected;
    let unrun = cases
        .iter()
        .filter(|case| case.disposition == "unrun")
        .count();
    let comparator_refused = cases
        .iter()
        .filter(|case| case.reason == "comparator-refused")
        .count();
    let produce_failed = cases
        .iter()
        .filter(|case| case.disposition == "failed" && case.reason != "comparator-refused")
        .count();
    let pass = cases.iter().filter(|case| case.reason == "pass").count();
    let outcome_fail = cases
        .iter()
        .filter(|case| case.reason == "pixel-fail")
        .count();
    let unmeasured = cases
        .iter()
        .filter(|case| case.reason == "unmeasured")
        .count();
    let compared = pass + outcome_fail + unmeasured;
    ProduceSummary {
        resources,
        selected,
        produced: compared + comparator_refused,
        compared,
        unsupported,
        failed: produce_failed + outcome_fail + comparator_refused,
        unrun,
        pass,
        outcome_fail,
        unmeasured,
        produce_failed,
        comparator_refused,
        recorded: cases.len(),
        tier_a: TIER_A,
        instruments: INSTRUMENTS,
        collector: COLLECTOR,
        host_measurement: HOST_MEASUREMENT,
        cases,
    }
}

fn classify_rows(
    reference: &Run,
    corpus_available: bool,
    corpus_root: &Path,
    cases: &mut Vec<CaseRecord>,
    ready: &mut Vec<Ready>,
    mut gpu: Option<&mut crate::gpu_candidate::GpuStack>,
) {
    let Some(rows) = reference.json.pointer("/rows").and_then(Value::as_array) else {
        return;
    };
    for row in rows {
        let Some(id) = row.pointer("/id").and_then(Value::as_str) else {
            continue;
        };
        if row.pointer("/ok").and_then(Value::as_bool) != Some(true) {
            cases.push(unsupported(id, "reference-row-not-rendered"));
            continue;
        }
        let Some(sidecar) = reference.sidecars.get(id) else {
            cases.push(selected_stop(id, &RowStop::failed("sidecar-missing")));
            continue;
        };
        if let Some(reason) = stack_selection_reason(&sidecar.json) {
            cases.push(unsupported(id, reason));
            continue;
        }
        if !identifier_ok(id) {
            cases.push(selected_stop(id, &RowStop::failed("identifier-refused")));
            continue;
        }
        if !corpus_available {
            cases.push(selected_stop(id, &RowStop::unrun("corpus-absent")));
            continue;
        }
        match render_row(corpus_root, sidecar, gpu.as_deref_mut()) {
            Ok((rgba, width, height)) => {
                let digest = sha256_hex(&rgba);
                ready.push(Ready {
                    id: id.to_owned(),
                    rgba,
                    width,
                    height,
                    digest,
                });
            }
            Err(stop) => cases.push(selected_stop(id, &stop)),
        }
    }
}

fn classify_volumes(reference: &Run, cases: &mut Vec<CaseRecord>) {
    for view in reference.views.values() {
        if view.kind == ViewKind::VolumeReformat {
            cases.push(unsupported(&view.id, "volume-reformat"));
        }
    }
}

fn unsupported(id: &str, reason: &'static str) -> CaseRecord {
    CaseRecord {
        id: id.to_owned(),
        disposition: "unsupported",
        reason,
    }
}

fn selected_stop(id: &str, stop: &RowStop) -> CaseRecord {
    CaseRecord {
        id: id.to_owned(),
        disposition: stop.disposition,
        reason: stop.reason,
    }
}

fn render_row(
    corpus_root: &Path,
    sidecar: &Sidecar,
    gpu: Option<&mut crate::gpu_candidate::GpuStack>,
) -> Result<(Vec<u8>, u32, u32), RowStop> {
    let relative = sidecar
        .json
        .pointer("/row/path")
        .and_then(Value::as_str)
        .ok_or_else(|| RowStop::failed("corpus-path-refused"))?;
    let file = open_corpus(corpus_root, relative)?;
    let declared = sidecar
        .json
        .pointer("/row/sha256")
        .and_then(Value::as_str)
        .ok_or_else(|| RowStop::failed("input-digest-absent"))?;
    if sha256_hex(&file) != declared {
        return Err(RowStop::failed("input-digest-disagrees"));
    }
    let parsed = parse_part10(&file).map_err(|_| RowStop::failed("dicom-parse-refused"))?;
    let mut samples = match parsed.transfer_syntax().dispatch_path() {
        DispatchPath::Encapsulated => encapsulated_samples(&parsed, sidecar)?,
        DispatchPath::ImplicitVrLittleEndian
        | DispatchPath::ExplicitVrLittleEndian
        | DispatchPath::ExplicitVrBigEndian
        | DispatchPath::DeflatedExplicitVrLittleEndian => native_samples(&parsed, sidecar)?,
    };
    // jpeg.js reconstructs a 12-bit extended scan 15 counts above the level
    // `decode_extended` shares with DCMTK. The reference frame is that
    // reconstruction. The codec samples stay on the 12-bit level.
    apply_reference_jpeg_level(sidecar, &mut samples);
    if let Some(gpu) = gpu {
        let camera = executed_camera(sidecar)?;
        let (width, height) = sidecar
            .frame_size()
            .map_err(|_| RowStop::failed("canvas-extent"))?;
        let (rows, columns) = sidecar
            .source_grid()
            .map_err(|_| RowStop::failed("pixel-layout-refused"))?;
        let presentation = presentation_of(sidecar)?;
        let rgba = gpu
            .draw(
                &camera.view,
                &presentation,
                &samples,
                [columns, rows],
                [width, height],
            )
            .map_err(RowStop::failed)?;
        Ok((rgba, width, height))
    } else {
        raster_settled(sidecar, &samples)
    }
}

fn open_corpus(root: &Path, relative: &str) -> Result<Vec<u8>, RowStop> {
    if relative.is_empty() || Path::new(relative).is_absolute() {
        return Err(RowStop::failed("corpus-path-refused"));
    }
    if relative
        .split(['/', '\\'])
        .any(|part| part.is_empty() || part == "." || part == "..")
    {
        return Err(RowStop::failed("corpus-path-refused"));
    }
    let root = root
        .canonicalize()
        .map_err(|_| RowStop::unrun("corpus-file-absent"))?;
    let path = root.join(relative);
    if !path.is_file() {
        return Err(RowStop::unrun("corpus-file-absent"));
    }
    let full = path
        .canonicalize()
        .map_err(|_| RowStop::failed("corpus-path-refused"))?;
    if !full.starts_with(&root) {
        return Err(RowStop::failed("corpus-path-refused"));
    }
    fs::read(&full).map_err(|_| RowStop::failed("corpus-file-unreadable"))
}

fn frame_container(
    corpus_root: &Path,
    sidecar: &Sidecar,
    allow_decoder_rgb: bool,
) -> Result<Vec<u8>, RowStop> {
    let relative = sidecar
        .json
        .pointer("/row/path")
        .and_then(Value::as_str)
        .ok_or_else(|| RowStop::failed("corpus-path-refused"))?;
    let file = open_corpus(corpus_root, relative)?;
    let declared = sidecar
        .json
        .pointer("/row/sha256")
        .and_then(Value::as_str)
        .ok_or_else(|| RowStop::failed("input-digest-absent"))?;
    if sha256_hex(&file) != declared {
        return Err(RowStop::failed("input-digest-disagrees"));
    }
    let parsed = parse_part10(&file).map_err(|_| RowStop::failed("dicom-parse-refused"))?;
    match parsed.transfer_syntax().dispatch_path() {
        DispatchPath::Encapsulated => encapsulated_bytes(&parsed, sidecar, allow_decoder_rgb),
        DispatchPath::ImplicitVrLittleEndian
        | DispatchPath::ExplicitVrLittleEndian
        | DispatchPath::ExplicitVrBigEndian
        | DispatchPath::DeflatedExplicitVrLittleEndian => native_bytes(&parsed, sidecar),
    }
}

fn native_bytes(parsed: &ocelli_dicom::ParsedDicom, sidecar: &Sidecar) -> Result<Vec<u8>, RowStop> {
    let element = parsed
        .object()
        .element(PIXEL_DATA)
        .map_err(|_| RowStop::failed("pixel-data-absent"))?;
    if element.value().fragments().is_some() {
        return Err(RowStop::failed("pixel-data-length"));
    }
    let primitive = element
        .value()
        .primitive()
        .ok_or_else(|| RowStop::failed("pixel-data-absent"))?;
    let facts = pixel_facts(sidecar)?;
    let bytes = primitive_bytes(primitive, facts.bits_allocated)?;
    first_frame(bytes, facts.frame_bytes, facts.frames)
}

fn native_samples(
    parsed: &ocelli_dicom::ParsedDicom,
    sidecar: &Sidecar,
) -> Result<Vec<Stored>, RowStop> {
    let frame = native_bytes(parsed, sidecar)?;
    let facts = pixel_facts(sidecar)?;
    unpack_frame(&frame, sidecar, &facts)
}

fn encapsulated_bytes(
    parsed: &ocelli_dicom::ParsedDicom,
    sidecar: &Sidecar,
    allow_decoder_rgb: bool,
) -> Result<Vec<u8>, RowStop> {
    let element = parsed
        .object()
        .element(PIXEL_DATA)
        .map_err(|_| RowStop::failed("pixel-data-absent"))?;
    let offsets = match element.value().offset_table() {
        Some(table) => table.to_vec(),
        None => Vec::new(),
    };
    let fragments = element
        .value()
        .fragments()
        .ok_or_else(|| RowStop::failed("pixel-data-absent"))?;
    let facts = pixel_facts(sidecar)?;
    let desc = frame_desc(sidecar, &facts)?;
    let registry = codecs().map_err(RowStop::failed)?;
    let decoder_rgb = registry
        .decode_photometric_interpretation(parsed.transfer_syntax().uid(), &desc)
        == Ok(DecodePhotometricInterpretation::Rgb);
    if decoder_rgb && !allow_decoder_rgb {
        return Err(RowStop::failed("decoder-colour-output"));
    }
    let borrowed: Vec<&[u8]> = fragments.iter().map(Vec::as_slice).collect();
    let index = EncapsulatedFrameIndex::from_basic(facts.frames, &offsets, &borrowed)
        .map_err(|_| RowStop::failed("frame-index-refused"))?;
    let parts = index
        .frame_fragments(0)
        .map_err(|_| RowStop::failed("frame-index-refused"))?;
    let mut encoded = Vec::new();
    for part in parts {
        encoded.extend_from_slice(part);
    }
    let mut decoded = vec![0_u8; desc.output_len()];
    registry
        .decode(
            parsed.transfer_syntax().uid(),
            &encoded,
            &desc,
            &mut decoded,
        )
        .map_err(|_| RowStop::failed("decode-refused"))?;
    Ok(decoded)
}

fn encapsulated_samples(
    parsed: &ocelli_dicom::ParsedDicom,
    sidecar: &Sidecar,
) -> Result<Vec<Stored>, RowStop> {
    let decoded = encapsulated_bytes(parsed, sidecar, false)?;
    let facts = pixel_facts(sidecar)?;
    unpack_frame(&decoded, sidecar, &facts)
}

struct PixelFacts {
    rows: u32,
    columns: u32,
    bits_allocated: u8,
    bits_stored: u8,
    high_bit: u8,
    representation: u8,
    frames: usize,
    frame_bytes: usize,
}

fn pixel_facts(sidecar: &Sidecar) -> Result<PixelFacts, RowStop> {
    let (rows, columns) = sidecar
        .source_grid()
        .map_err(|_| RowStop::failed("pixel-layout-refused"))?;
    let bits_allocated = json_u8(sidecar, "/attributes/bitsAllocated")?;
    let bits_stored = json_u8(sidecar, "/attributes/bitsStored")?;
    let high_bit = json_u8(sidecar, "/attributes/highBit")?;
    let representation = json_u8(sidecar, "/attributes/pixelRepresentation")?;
    if representation > 1 {
        return Err(RowStop::failed("pixel-layout-refused"));
    }
    let frames = declared_frames(&sidecar.json)?;
    let stored_samples = stored_sample_count(sidecar)?;
    let per_sample = usize::from(bits_allocated / 8);
    if per_sample == 0 {
        return Err(RowStop::failed("pixel-layout-refused"));
    }
    let pixels = usize::try_from(rows)
        .ok()
        .and_then(|height| {
            usize::try_from(columns)
                .ok()
                .and_then(|width| height.checked_mul(width))
        })
        .ok_or_else(|| RowStop::failed("pixel-layout-refused"))?;
    // Native RGB is three stored samples per pixel. A 4:2:2 photometric
    // interpretation stores two. Monochrome and palette stay at one.
    // Encapsulated output length is `FrameDesc::output_len`, not this product.
    let frame_bytes = pixels
        .checked_mul(per_sample)
        .and_then(|bytes| bytes.checked_mul(stored_samples))
        .ok_or_else(|| RowStop::failed("pixel-data-length"))?;
    let stored_representation = match representation {
        0 => ocelli_pixel::PixelRepresentation::Unsigned,
        _ => ocelli_pixel::PixelRepresentation::Signed,
    };
    // The file high bit is retained on the facts. A container whose high bit
    // is not `BitsStored - 1` is still a legal width here. `StoredBits::new`
    // keeps the DICOM refusal. The unpack below uses the conforming position,
    // which is the shift the reference render applies.
    let aligned = bits_stored
        .checked_sub(1)
        .ok_or_else(|| RowStop::failed("pixel-layout-refused"))?;
    if high_bit >= bits_allocated {
        return Err(RowStop::failed("pixel-layout-refused"));
    }
    StoredBits::new(bits_allocated, bits_stored, aligned, stored_representation)
        .map_err(|_| RowStop::failed("pixel-layout-refused"))?;
    Ok(PixelFacts {
        rows,
        columns,
        bits_allocated,
        bits_stored,
        high_bit,
        representation,
        frames,
        frame_bytes,
    })
}

fn frame_desc(sidecar: &Sidecar, facts: &PixelFacts) -> Result<FrameDesc, RowStop> {
    let photometric = sidecar
        .json
        .pointer("/attributes/photometricInterpretation")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_owned();
    let representation = match facts.representation {
        0 => PixelRepresentation::Unsigned,
        _ => PixelRepresentation::Signed,
    };
    FrameDesc::new(FrameDescInput {
        rows: u16::try_from(facts.rows).map_err(|_| RowStop::failed("pixel-layout-refused"))?,
        columns: u16::try_from(facts.columns)
            .map_err(|_| RowStop::failed("pixel-layout-refused"))?,
        samples_per_pixel: json_u16(sidecar, "/attributes/samplesPerPixel").unwrap_or(1),
        bits_allocated: u16::from(facts.bits_allocated),
        bits_stored: u16::from(facts.bits_stored),
        high_bit: u16::from(facts.high_bit),
        pixel_representation: representation,
        photometric_interpretation: photometric,
        pixel_data_vr: PixelDataVr::Ob,
    })
    .map_err(|_| RowStop::failed("pixel-layout-refused"))
}

fn unpack_frame(
    frame: &[u8],
    sidecar: &Sidecar,
    facts: &PixelFacts,
) -> Result<Vec<Stored>, RowStop> {
    let photometric = photometric_of(sidecar)?;
    let layout = SampleLayout::new(1, photometric, None::<PlanarConfiguration>)
        .map_err(|_| RowStop::failed("pixel-layout-refused"))?;
    let representation = match facts.representation {
        0 => ocelli_pixel::PixelRepresentation::Unsigned,
        _ => ocelli_pixel::PixelRepresentation::Signed,
    };
    // Cornerstone reads the low `BitsStored` bits. A legacy high bit of 15
    // on a 12-bit sample would shift by 4 and miss that frame. The pixel
    // crate still refuses the legacy header. This raster does not.
    let aligned = facts
        .bits_stored
        .checked_sub(1)
        .ok_or_else(|| RowStop::failed("pixel-layout-refused"))?;
    let bits = StoredBits::new(
        facts.bits_allocated,
        facts.bits_stored,
        aligned,
        representation,
    )
    .map_err(|_| RowStop::failed("pixel-layout-refused"))?;
    let dimensions = ImageDimensions::new(facts.rows, facts.columns)
        .map_err(|_| RowStop::failed("pixel-layout-refused"))?;
    let description = StoredPixelDescription::new(dimensions, layout, bits);
    let count = description
        .sample_count()
        .map_err(|_| RowStop::failed("pixel-data-length"))?;
    let mut stored = vec![Stored(0.0); count];
    description
        .unpack(frame, ByteOrder::LittleEndian, &mut stored)
        .map_err(|_| RowStop::failed("pixel-data-length"))?;
    Ok(stored)
}

/// Camera state the raster executed, plus the observed reference flip flags.
///
/// `flip_horizontal` and `flip_vertical` are the readback fields. They are not
/// the raster's internal column coefficient. That coefficient is applied on
/// `view` and is not written back into these fields.
struct ExecutedCamera {
    view: View,
    parallel_scale: f64,
    vertical: f64,
    horizontal: f64,
    flip_horizontal: bool,
    flip_vertical: bool,
}

fn raster_settled(sidecar: &Sidecar, stored: &[Stored]) -> Result<(Vec<u8>, u32, u32), RowStop> {
    let executed = executed_camera(sidecar)?;
    let (width, height) = sidecar
        .frame_size()
        .map_err(|_| RowStop::failed("canvas-extent"))?;
    let (rows, columns) = sidecar
        .source_grid()
        .map_err(|_| RowStop::failed("pixel-layout-refused"))?;
    let mut view = executed.view;
    align_raster_window(&mut view, columns, rows)?;

    let mut raster = CpuRaster::open(f64::from(width), f64::from(height), 1.0)
        .map_err(|_| RowStop::failed("canvas-extent"))?;
    if raster.device() != [width, height] {
        return Err(RowStop::failed("canvas-extent"));
    }
    let pixels = usize::try_from(width)
        .ok()
        .and_then(|axis| {
            usize::try_from(height)
                .ok()
                .and_then(|other| axis.checked_mul(other))
        })
        .and_then(|count| count.checked_mul(4))
        .ok_or_else(|| RowStop::failed("canvas-extent"))?;
    let mut rgba = vec![0_u8; pixels];
    let presentation = presentation_of(sidecar)?;
    let drawn = raster
        .raster_into(
            &view,
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
        .map_err(|_| RowStop::failed("raster-refused"))?;
    if !drawn.wrote || drawn.extent != [width, height] {
        return Err(RowStop::failed("raster-idle"));
    }
    cover_quad_edge(&mut rgba, &view, columns, rows, width, height)?;
    Ok((rgba, width, height))
}

/// Standard 4-sample positions relative to a pixel centre.
///
/// The viewport canvas resolves those samples against the clear colour. A
/// sample inside the source quad keeps the nearest source texel. A sample
/// outside contributes black. Interior pixels, where all four samples see
/// the same texel, stay on the point sample.
const EDGE_SAMPLE_OFFSETS: [(f64, f64); 4] = [
    (-0.125, -0.375),
    (0.375, -0.125),
    (-0.375, 0.125),
    (0.125, 0.375),
];

/// Window positions snap to this many units per pixel before sampling.
///
/// Vulkan requires at least four fractional bits. The pinned software
/// rasteriser rounds the quad to that grid, and the nearest source texel is
/// read through the snapped edges. A sample that lands on the snapped
/// maximum edge is inside. A sample on the snapped minimum edge is outside.
/// `synthetic__mr_nonsquare_spacing` separates the two: continuous `409.6`
/// misses the sample at `409.625`, and the sixteenth snap includes it, while
/// the top sample at `102.375` stays out.
const WINDOW_SUBPIXELS: f64 = 16.0;

struct WindowRect {
    left: f64,
    top: f64,
    right: f64,
    bottom: f64,
}

/// Place the image edges on the sixteenth-pixel grid the reference rasteriser uses.
///
/// Spacing absorbs the snapped width and height. Pan absorbs a centre that
/// the two edges do not move equally. An edge already on the grid is unchanged,
/// so a column fit that lands on whole pixels keeps its texels.
fn align_raster_window(view: &mut View, columns: u32, rows: u32) -> Result<(), RowStop> {
    let to_canvas = view
        .index_to_canvas(columns, rows)
        .map_err(|_| RowStop::failed("camera-scale-refused"))?;
    let continuous = window_rect(&to_canvas, columns, rows, false)
        .ok_or_else(|| RowStop::failed("camera-scale-refused"))?;
    let snapped = window_rect(&to_canvas, columns, rows, true)
        .ok_or_else(|| RowStop::failed("camera-scale-refused"))?;
    let scale = view.scale();
    let snapped_width = snapped.right - snapped.left;
    let snapped_height = snapped.bottom - snapped.top;
    if !scale.is_finite()
        || scale <= 0.0
        || !snapped_width.is_finite()
        || !snapped_height.is_finite()
        || snapped_width <= 0.0
        || snapped_height <= 0.0
    {
        return Err(RowStop::failed("camera-scale-refused"));
    }
    let column_mm = snapped_width / (f64::from(columns) * scale);
    let row_mm = snapped_height / (f64::from(rows) * scale);
    if !column_mm.is_finite() || !row_mm.is_finite() || column_mm <= 0.0 || row_mm <= 0.0 {
        return Err(RowStop::failed("camera-scale-refused"));
    }
    view.set_spacing(column_mm, row_mm);
    let dx = (snapped.left + snapped.right - continuous.left - continuous.right) * 0.5;
    let dy = (snapped.top + snapped.bottom - continuous.top - continuous.bottom) * 0.5;
    view.set_pan(Pt::<Canvas>::new(dx, dy, 0.0))
        .map_err(|_| RowStop::failed("camera-scale-refused"))?;
    Ok(())
}

fn snapped_window_rect(
    to_canvas: &Transform<Index, Canvas>,
    columns: u32,
    rows: u32,
) -> Option<WindowRect> {
    window_rect(to_canvas, columns, rows, true)
}

fn window_rect(
    to_canvas: &Transform<Index, Canvas>,
    columns: u32,
    rows: u32,
    snap: bool,
) -> Option<WindowRect> {
    let width = f64::from(columns);
    let height = f64::from(rows);
    let corners = [
        (-INDEX_TO_IMAGE_CENTRE, -INDEX_TO_IMAGE_CENTRE),
        (width - INDEX_TO_IMAGE_CENTRE, -INDEX_TO_IMAGE_CENTRE),
        (-INDEX_TO_IMAGE_CENTRE, height - INDEX_TO_IMAGE_CENTRE),
        (
            width - INDEX_TO_IMAGE_CENTRE,
            height - INDEX_TO_IMAGE_CENTRE,
        ),
    ];
    let mut left = f64::MAX;
    let mut top = f64::MAX;
    let mut right = f64::MIN;
    let mut bottom = f64::MIN;
    for (index_x, index_y) in corners {
        let canvas = to_canvas.apply(Pt::<Index>::new(index_x, index_y, 0.0));
        if !canvas.x.is_finite() || !canvas.y.is_finite() {
            return None;
        }
        let x = if snap {
            snap_window(canvas.x)
        } else {
            canvas.x
        };
        let y = if snap {
            snap_window(canvas.y)
        } else {
            canvas.y
        };
        left = left.min(x);
        top = top.min(y);
        right = right.max(x);
        bottom = bottom.max(y);
    }
    Some(WindowRect {
        left,
        top,
        right,
        bottom,
    })
}

fn snap_window(value: f64) -> f64 {
    (value * WINDOW_SUBPIXELS).round() / WINDOW_SUBPIXELS
}

/// Far edges include the sample that lands on them. Near edges do not.
fn sample_inside_window(rect: &WindowRect, x: f64, y: f64) -> bool {
    x > rect.left && x <= rect.right && y > rect.top && y <= rect.bottom
}

fn cover_quad_edge(
    rgba: &mut [u8],
    view: &View,
    columns: u32,
    rows: u32,
    width: u32,
    height: u32,
) -> Result<(), RowStop> {
    let to_index = view
        .canvas_to_index(columns, rows)
        .map_err(|_| RowStop::failed("camera-scale-refused"))?;
    let to_canvas = view
        .index_to_canvas(columns, rows)
        .map_err(|_| RowStop::failed("camera-scale-refused"))?;
    let rect = snapped_window_rect(&to_canvas, columns, rows)
        .ok_or_else(|| RowStop::failed("camera-scale-refused"))?;
    let mut y = 0_u32;
    while y < height {
        let mut x = 0_u32;
        while x < width {
            let css_x = f64::from(x) + 0.5;
            let css_y = f64::from(y) + 0.5;
            let centre = to_index.apply(Pt::<Canvas>::new(css_x, css_y, 0.0));
            let centre_texel = source_texel(centre.x, centre.y, columns, rows);
            let mut hits = 0_u32;
            let mut inside_texel = None;
            for (offset_x, offset_y) in EDGE_SAMPLE_OFFSETS {
                let sample_x = css_x + offset_x;
                let sample_y = css_y + offset_y;
                if sample_inside_window(&rect, sample_x, sample_y) {
                    hits = hits.saturating_add(1);
                }
                let sample = to_index.apply(Pt::<Canvas>::new(sample_x, sample_y, 0.0));
                if let Some(texel) = source_texel(sample.x, sample.y, columns, rows) {
                    inside_texel = Some(texel);
                }
            }
            if hits != 0 && hits != 4 {
                write_coverage(
                    rgba,
                    &to_canvas,
                    (width, height),
                    (x, y),
                    if centre_texel.is_some() {
                        None
                    } else {
                        inside_texel
                    },
                    hits,
                );
            }
            x = x.saturating_add(1);
        }
        y = y.saturating_add(1);
    }
    Ok(())
}

fn source_texel(index_x: f64, index_y: f64, columns: u32, rows: u32) -> Option<(u32, u32)> {
    let column = contained_image(index_x + INDEX_TO_IMAGE_CENTRE, columns)?;
    let row = contained_image(index_y + INDEX_TO_IMAGE_CENTRE, rows)?;
    Some((column, row))
}

fn contained_image(image: f64, limit: u32) -> Option<u32> {
    if !image.is_finite() || image < 0.0 {
        return None;
    }
    if image >= f64::from(limit) {
        return None;
    }
    let mut lo = 0_u32;
    let mut hi = limit;
    while lo.saturating_add(1) < hi {
        let mid = lo + (hi - lo) / 2;
        if f64::from(mid) <= image {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    Some(lo)
}

fn write_coverage(
    rgba: &mut [u8],
    to_canvas: &Transform<Index, Canvas>,
    extent: (u32, u32),
    pixel: (u32, u32),
    lookup: Option<(u32, u32)>,
    hits: u32,
) {
    let (width, _) = extent;
    let (x, y) = pixel;
    let Some([red, green, blue, alpha]) = coverage_source(rgba, to_canvas, extent, pixel, lookup)
    else {
        return;
    };
    let Some(slot) = rgba_at_mut(rgba, width, x, y) else {
        return;
    };
    let blended = [
        coverage_byte(red, hits),
        coverage_byte(green, hits),
        coverage_byte(blue, hits),
        alpha,
    ];
    slot.copy_from_slice(&blended);
}

fn coverage_source(
    rgba: &[u8],
    to_canvas: &Transform<Index, Canvas>,
    extent: (u32, u32),
    pixel: (u32, u32),
    lookup: Option<(u32, u32)>,
) -> Option<[u8; 4]> {
    let (width, height) = extent;
    let (x, y) = pixel;
    let Some((column, row)) = lookup else {
        return rgba_at(rgba, width, x, y);
    };
    let centre = to_canvas.apply(Pt::<Index>::new(f64::from(column), f64::from(row), 0.0));
    let source_x = pixel_of_css(centre.x, width)?;
    let source_y = pixel_of_css(centre.y, height)?;
    rgba_at(rgba, width, source_x, source_y)
}

fn pixel_of_css(css: f64, limit: u32) -> Option<u32> {
    if !css.is_finite() {
        return None;
    }
    let shifted = css - 0.5;
    if shifted < 0.0 {
        return None;
    }
    let rounded = shifted.round();
    if !rounded.is_finite() || rounded < 0.0 || rounded >= f64::from(limit) {
        return None;
    }
    let mut lo = 0_u32;
    let mut hi = limit;
    while lo.saturating_add(1) < hi {
        let mid = lo + (hi - lo) / 2;
        if f64::from(mid) <= rounded {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    if (f64::from(lo) - rounded).abs() > 0.1 {
        return None;
    }
    Some(lo)
}

fn coverage_byte(value: u8, hits: u32) -> u8 {
    let scaled = u32::from(value).saturating_mul(hits).saturating_add(2) / 4;
    u8::try_from(scaled).unwrap_or(u8::MAX)
}

fn rgba_at(rgba: &[u8], width: u32, x: u32, y: u32) -> Option<[u8; 4]> {
    let index = rgba_index(width, x, y)?;
    let bytes = rgba.get(index..index.saturating_add(4))?;
    Some([
        *bytes.first()?,
        *bytes.get(1)?,
        *bytes.get(2)?,
        *bytes.get(3)?,
    ])
}

fn rgba_at_mut(rgba: &mut [u8], width: u32, x: u32, y: u32) -> Option<&mut [u8]> {
    let index = rgba_index(width, x, y)?;
    rgba.get_mut(index..index.saturating_add(4))
}

fn rgba_index(width: u32, x: u32, y: u32) -> Option<usize> {
    let width = usize::try_from(width).ok()?;
    let x = usize::try_from(x).ok()?;
    let y = usize::try_from(y).ok()?;
    y.checked_mul(width)
        .and_then(|row| row.checked_add(x))
        .and_then(|pixel| pixel.checked_mul(4))
}

/// Stored counts the pinned 12-bit JPEG decoder adds before the LUT.
///
/// jpeg.js keeps the 8-bit bias 2056 and divides it by `255 / 4095`, then
/// shifts by 4. The same fixed-point IDCT at a 12-bit level adds
/// `(2048 << 4) + 8` instead. That difference rounds to 15. Unsigned
/// samples only. A signed representation is not this reconstruction.
fn jpeg_extended_12_level_residual() -> f32 {
    let scaled = 2056.0_f64 * 4095.0 / 255.0;
    let level = 2048.0_f64 * 16.0 + 8.0;
    let residual = ((scaled - level) / 16.0).round();
    f32_from_f64(residual)
}

fn apply_reference_jpeg_level(sidecar: &Sidecar, samples: &mut [Stored]) {
    let syntax = sidecar
        .json
        .pointer("/row/transferSyntax")
        .and_then(Value::as_str);
    let bits = sidecar
        .json
        .pointer("/attributes/bitsStored")
        .and_then(Value::as_u64);
    let representation = sidecar
        .json
        .pointer("/attributes/pixelRepresentation")
        .and_then(Value::as_u64);
    if syntax != Some("1.2.840.10008.1.2.4.51") || bits != Some(12) || representation != Some(0) {
        return;
    }
    let residual = jpeg_extended_12_level_residual();
    for sample in samples {
        *sample = Stored(sample.0 + residual);
    }
}

fn presentation_of(sidecar: &Sidecar) -> Result<Presentation, RowStop> {
    if sidecar.json.pointer("/voi/source").and_then(Value::as_str) == Some("none") {
        return Err(RowStop::failed("voi-absent"));
    }
    let center = required_f64(&sidecar.json, "/voi/windowCenter", "voi-absent")?;
    let width = required_f64(&sidecar.json, "/voi/windowWidth", "voi-absent")?;
    let slope = required_f64(&sidecar.json, "/image/slope", "missing-rescale")?;
    let intercept = required_f64(&sidecar.json, "/image/intercept", "missing-rescale")?;
    if !center.is_finite() || !width.is_finite() {
        return Err(RowStop::failed("voi-window-refused"));
    }
    if !slope.is_finite() || !intercept.is_finite() {
        return Err(RowStop::failed("rescale-refused"));
    }
    let function = voi_function(&sidecar.json)?;
    let (center, width) = presented_sigmoid_window(function, center, width);
    let photometric = photometric_of(sidecar)?;
    let modality = ModalityTransform::new(
        None,
        Some(f32_from_f64(slope)),
        Some(f32_from_f64(intercept)),
    )
    .map_err(|_| RowStop::failed("rescale-refused"))?;
    let voi = VoiTransform::new(
        None,
        &[f32_from_f64(center)],
        &[f32_from_f64(width)],
        0,
        function,
        0.0,
        255.0,
    )
    .map_err(|_| RowStop::failed("voi-window-refused"))?;
    let chain = LutChain::new(modality, voi, photometric, PresentationLutEvidence::Absent)
        .map_err(|_| RowStop::failed("raster-refused"))?;
    Ok(Presentation::new(
        chain,
        photometric,
        PresentationLutEvidence::Absent,
        Colormap::Grey,
    ))
}

fn photometric_of(sidecar: &Sidecar) -> Result<PhotometricInterpretation, RowStop> {
    match sidecar
        .json
        .pointer("/attributes/photometricInterpretation")
        .and_then(Value::as_str)
    {
        Some("MONOCHROME1") => Ok(PhotometricInterpretation::Monochrome1),
        Some("MONOCHROME2") => Ok(PhotometricInterpretation::Monochrome2),
        _ => Err(RowStop::failed("photometric-unsupported")),
    }
}

/// File width of at least one is unchanged. A SIGMOID width in `(0, 1)` is
/// the window the pinned renderer recovers before C.11.2.1.3.1 runs.
///
/// Width 0.5 at centre 40 comes back as width 1.5 at centre 40. The product
/// LUT keeps the file width. This adjustment belongs to the candidate only.
fn presented_sigmoid_window(function: VoiFunction, center: f64, width: f64) -> (f64, f64) {
    if function != VoiFunction::Sigmoid || !(width > 0.0 && width < 1.0) {
        return (center, width);
    }
    let half_span = (width - 1.0) / 2.0;
    let lower = center - 0.5 - half_span;
    let upper = center - 0.5 + half_span;
    let span = upper - lower;
    ((lower + upper + 1.0) / 2.0, span.abs() + 1.0)
}

fn voi_function(sidecar: &Value) -> Result<VoiFunction, RowStop> {
    match sidecar
        .pointer("/voi/voiLutFunction")
        .and_then(Value::as_str)
    {
        None | Some("LINEAR") => Ok(VoiFunction::Linear),
        Some("LINEAR_EXACT") => Ok(VoiFunction::LinearExact),
        Some("SIGMOID") => Ok(VoiFunction::Sigmoid),
        Some(_) => Err(RowStop::failed("voi-function-unknown")),
    }
}

fn spacing_mm(sidecar: &Value, frame_height: u32) -> Result<(f64, f64), RowStop> {
    if let Some(value) = sidecar.pointer("/attributes/pixelSpacing")
        && !value.is_null()
    {
        let (row, column) =
            positive_pair(value).ok_or_else(|| RowStop::failed("spacing-refused"))?;
        return Ok((column, row));
    }
    let row = sidecar
        .pointer("/cornerstoneMetadata/imagePlaneModule/rowPixelSpacing")
        .and_then(json_f64);
    let column = sidecar
        .pointer("/cornerstoneMetadata/imagePlaneModule/columnPixelSpacing")
        .and_then(json_f64);
    if row.is_some() || column.is_some() {
        let (Some(row), Some(column)) = (row, column) else {
            return Err(RowStop::failed("spacing-refused"));
        };
        if !row.is_finite() || !column.is_finite() || row <= 0.0 || column <= 0.0 {
            return Err(RowStop::failed("spacing-refused"));
        }
        return Ok((column, row));
    }
    let vertical = sidecar
        .pointer("/canvasPixelsPerSourcePixel/vertical")
        .and_then(json_f64);
    let horizontal = sidecar
        .pointer("/canvasPixelsPerSourcePixel/horizontal")
        .and_then(json_f64);
    let parallel = sidecar.pointer("/camera/parallelScale").and_then(json_f64);
    if let (Some(vertical), Some(horizontal), Some(parallel)) = (vertical, horizontal, parallel) {
        if frame_height == 0 || !parallel.is_finite() {
            return Err(RowStop::failed("spacing-refused"));
        }
        let millimetres = 2.0 * parallel / f64::from(frame_height);
        let column_mm = horizontal * millimetres;
        let row_mm = vertical * millimetres;
        if !column_mm.is_finite() || !row_mm.is_finite() || column_mm <= 0.0 || row_mm <= 0.0 {
            return Err(RowStop::failed("spacing-refused"));
        }
        return Ok((column_mm, row_mm));
    }
    Ok((1.0, 1.0))
}

fn positive_pair(value: &Value) -> Option<(f64, f64)> {
    let items = value.as_array()?;
    if items.len() != 2 {
        return None;
    }
    let row = items.first().and_then(json_f64)?;
    let column = items.get(1).and_then(json_f64)?;
    if !row.is_finite() || !column.is_finite() || row <= 0.0 || column <= 0.0 {
        return None;
    }
    Some((row, column))
}

fn camera_flag(sidecar: &Value, pointer: &str) -> Result<bool, RowStop> {
    match sidecar.pointer(pointer) {
        None | Some(Value::Null) => Ok(false),
        Some(Value::Bool(value)) => Ok(*value),
        Some(_) => Err(RowStop::failed("camera-flip-refused")),
    }
}

fn declared_frames(sidecar: &Value) -> Result<usize, RowStop> {
    match sidecar.pointer("/attributes/numberOfFrames") {
        None | Some(Value::Null) => Ok(1),
        Some(value) => {
            let count = value.as_u64().or_else(|| {
                value
                    .as_str()
                    .and_then(|text| text.trim().parse::<u64>().ok())
            });
            match count.and_then(|count| usize::try_from(count).ok()) {
                Some(count) if count > 0 => Ok(count),
                _ => Err(RowStop::failed("pixel-data-length")),
            }
        }
    }
}

fn json_u8(sidecar: &Sidecar, pointer: &str) -> Result<u8, RowStop> {
    let number = sidecar
        .json
        .pointer(pointer)
        .and_then(Value::as_u64)
        .ok_or_else(|| RowStop::failed("pixel-layout-refused"))?;
    u8::try_from(number).map_err(|_| RowStop::failed("pixel-layout-refused"))
}

fn json_u16(sidecar: &Sidecar, pointer: &str) -> Result<u16, RowStop> {
    let number = sidecar
        .json
        .pointer(pointer)
        .and_then(Value::as_u64)
        .ok_or_else(|| RowStop::failed("pixel-layout-refused"))?;
    u16::try_from(number).map_err(|_| RowStop::failed("pixel-layout-refused"))
}

/// Stored samples on the wire for one pixel.
///
/// PS3.3 C.7.6.3.1.2 stores `YBR_FULL_422` and `YBR_PARTIAL_422` as two
/// samples per pixel even though Samples per Pixel stays 3. Every other
/// photometric interpretation uses Samples per Pixel. A missing attribute is
/// one sample, which is the monochrome files this path already draws.
fn stored_sample_count(sidecar: &Sidecar) -> Result<usize, RowStop> {
    let photometric = sidecar
        .json
        .pointer("/attributes/photometricInterpretation")
        .and_then(Value::as_str)
        .unwrap_or("");
    if photometric == "YBR_FULL_422" || photometric == "YBR_PARTIAL_422" {
        return Ok(2);
    }
    let samples = json_u16(sidecar, "/attributes/samplesPerPixel").unwrap_or(1);
    if samples == 0 {
        return Err(RowStop::failed("pixel-layout-refused"));
    }
    Ok(usize::from(samples))
}

fn json_f64(value: &Value) -> Option<f64> {
    value.as_f64().or_else(|| {
        value
            .as_array()
            .and_then(|items| items.first())
            .and_then(Value::as_f64)
    })
}

fn required_f64(sidecar: &Value, pointer: &str, reason: &'static str) -> Result<f64, RowStop> {
    sidecar
        .pointer(pointer)
        .and_then(json_f64)
        .ok_or_else(|| RowStop::failed(reason))
}

fn primitive_bytes(value: &PrimitiveValue, bits_allocated: u8) -> Result<Vec<u8>, RowStop> {
    match (bits_allocated, value) {
        (8, PrimitiveValue::U8(samples)) => Ok(samples.iter().copied().collect()),
        // PS3.5 packs an 8-bit OW pixel element into 16-bit words. Those
        // words are the stored byte pairs, little endian on the wire.
        (8, PrimitiveValue::U16(samples)) => pack_u16(samples),
        (16, PrimitiveValue::U16(samples)) => pack_u16(samples),
        (16, PrimitiveValue::I16(samples)) => pack_i16(samples),
        (16, PrimitiveValue::U8(samples)) => Ok(samples.iter().copied().collect()),
        (32, PrimitiveValue::U32(samples)) => pack_u32(samples),
        (32, PrimitiveValue::I32(samples)) => pack_i32(samples),
        (32, PrimitiveValue::U8(samples)) => Ok(samples.iter().copied().collect()),
        _ => Err(RowStop::failed("pixel-data-length")),
    }
}

fn pack_u16(words: &[u16]) -> Result<Vec<u8>, RowStop> {
    pack_width(words.len(), 2, |out| {
        for word in words {
            out.extend_from_slice(&word.to_le_bytes());
        }
    })
}

fn pack_i16(words: &[i16]) -> Result<Vec<u8>, RowStop> {
    pack_width(words.len(), 2, |out| {
        for word in words {
            out.extend_from_slice(&word.to_le_bytes());
        }
    })
}

fn pack_u32(words: &[u32]) -> Result<Vec<u8>, RowStop> {
    pack_width(words.len(), 4, |out| {
        for word in words {
            out.extend_from_slice(&word.to_le_bytes());
        }
    })
}

fn pack_i32(words: &[i32]) -> Result<Vec<u8>, RowStop> {
    pack_width(words.len(), 4, |out| {
        for word in words {
            out.extend_from_slice(&word.to_le_bytes());
        }
    })
}

fn pack_width(
    count: usize,
    width: usize,
    fill: impl FnOnce(&mut Vec<u8>),
) -> Result<Vec<u8>, RowStop> {
    let len = count
        .checked_mul(width)
        .ok_or_else(|| RowStop::failed("pixel-data-length"))?;
    let mut out = Vec::new();
    out.try_reserve(len)
        .map_err(|_| RowStop::failed("pixel-data-length"))?;
    fill(&mut out);
    Ok(out)
}

fn first_frame(mut bytes: Vec<u8>, frame_bytes: usize, frames: usize) -> Result<Vec<u8>, RowStop> {
    if frame_bytes == 0 || frames == 0 {
        return Err(RowStop::failed("pixel-data-length"));
    }
    let expected = frame_bytes
        .checked_mul(frames)
        .ok_or_else(|| RowStop::failed("pixel-data-length"))?;
    if let Some(padded) = expected.checked_add(1)
        && bytes.len() == padded
        && bytes.last().copied() == Some(0)
    {
        bytes.pop();
    }
    if bytes.len() != expected {
        return Err(RowStop::failed("pixel-data-length"));
    }
    bytes.truncate(frame_bytes);
    Ok(bytes)
}

fn codecs() -> Result<&'static Registry, &'static str> {
    static CELL: OnceLock<Result<Registry, &'static str>> = OnceLock::new();
    match CELL.get_or_init(build_registry) {
        Ok(registry) => Ok(registry),
        Err(reason) => Err(*reason),
    }
}

fn build_registry() -> Result<Registry, &'static str> {
    let mut registry = Registry::new();
    register_native_and_rle_decoders(&mut registry).map_err(|_| "registry-unavailable")?;
    register_jpeg_decoders(&mut registry).map_err(|_| "registry-unavailable")?;
    register_jpeg2000_decoders(&mut registry).map_err(|_| "registry-unavailable")?;
    register_jpegls_decoders(&mut registry).map_err(|_| "registry-unavailable")?;
    register_htj2k_decoders(&mut registry).map_err(|_| "registry-unavailable")?;
    Ok(registry)
}

fn write_pair(reference: &Run, paths: &ProducePaths, frame: &Ready) -> Result<(), &'static str> {
    let sidecar = reference
        .sidecars
        .get(&frame.id)
        .ok_or("output-write-refused")?;
    let reference_raw = paths.reference_subset.join(format!("{}.raw", frame.id));
    let reference_json = paths.reference_subset.join(format!("{}.json", frame.id));
    let candidate_raw = paths.candidate_run.join(format!("{}.raw", frame.id));
    let candidate_json = paths.candidate_run.join(format!("{}.json", frame.id));
    let written = (|| {
        let expected = usize::try_from(frame.width)
            .ok()
            .and_then(|width| {
                usize::try_from(frame.height)
                    .ok()
                    .and_then(|height| width.checked_mul(height))
            })
            .and_then(|pixels| pixels.checked_mul(4))
            .ok_or("output-write-refused")?;
        if frame.rgba.len() != expected {
            return Err("output-write-refused");
        }
        fs::copy(
            reference.directory.join(format!("{}.raw", frame.id)),
            &reference_raw,
        )
        .map_err(|_| "output-write-refused")?;
        fs::copy(
            reference.directory.join(format!("{}.json", frame.id)),
            &reference_json,
        )
        .map_err(|_| "output-write-refused")?;
        let executed = executed_camera(sidecar).map_err(|_| "output-write-refused")?;
        let json = candidate_sidecar(sidecar.json.clone(), &executed, &frame.digest);
        let text = serde_json::to_vec_pretty(&json).map_err(|_| "output-write-refused")?;
        fs::write(&candidate_raw, &frame.rgba).map_err(|_| "output-write-refused")?;
        fs::write(&candidate_json, text).map_err(|_| "output-write-refused")?;
        Ok(())
    })();
    if written.is_err() {
        for path in [
            &reference_raw,
            &reference_json,
            &candidate_raw,
            &candidate_json,
        ] {
            let _ = fs::remove_file(path);
        }
    }
    written
}

fn candidate_sidecar(mut json: Value, executed: &ExecutedCamera, digest: &str) -> Value {
    if let Some(frame) = json.get_mut("frame").and_then(Value::as_object_mut) {
        frame.remove("statistics");
        frame.insert("sha256".to_owned(), Value::String(digest.to_owned()));
    }
    if let Some(camera) = json.get_mut("camera").and_then(Value::as_object_mut) {
        camera.insert(
            "parallelScale".to_owned(),
            json_number(executed.parallel_scale),
        );
        camera.insert(
            "flipHorizontal".to_owned(),
            Value::Bool(executed.flip_horizontal),
        );
        camera.insert(
            "flipVertical".to_owned(),
            Value::Bool(executed.flip_vertical),
        );
    }
    if let Some(scale) = json
        .get_mut("canvasPixelsPerSourcePixel")
        .and_then(Value::as_object_mut)
    {
        scale.insert("vertical".to_owned(), json_number(executed.vertical));
        scale.insert("horizontal".to_owned(), json_number(executed.horizontal));
    }
    json
}

fn json_number(value: f64) -> Value {
    serde_json::Number::from_f64(value).map_or(Value::Null, Value::Number)
}

/// Fit the source, then apply `parallelScale` when it is not already that fit.
///
/// Millimetres per canvas pixel are `2 * parallelScale / canvasHeight`. A
/// centred focal point and a view-up of the negated column cosine are
/// required. A gaze parallel to the plane normal is applied on either side.
/// The positive-normal side sets the raster's horizontal coefficient, because
/// that side's reference right vector is the negated row. The sidecar flip
/// flags are the observed readback. A reference horizontal flip has already
/// moved the camera, so those flags are published as read and are not applied
/// again. Anything off that pose is refused before a sidecar is published.
fn executed_camera(sidecar: &Sidecar) -> Result<ExecutedCamera, RowStop> {
    let (width, height) = sidecar
        .frame_size()
        .map_err(|_| RowStop::failed("canvas-extent"))?;
    let (rows, columns) = sidecar
        .source_grid()
        .map_err(|_| RowStop::failed("pixel-layout-refused"))?;
    if height == 0 || width == 0 {
        return Err(RowStop::failed("canvas-extent"));
    }
    let (column_mm, row_mm) = spacing_mm(&sidecar.json, height)?;
    let flip_horizontal = camera_flag(&sidecar.json, "/camera/flipHorizontal")?;
    let flip_vertical = camera_flag(&sidecar.json, "/camera/flipVertical")?;
    let parallel = required_f64(
        &sidecar.json,
        "/camera/parallelScale",
        "camera-scale-refused",
    )?;
    if !parallel.is_finite() || parallel <= 0.0 {
        return Err(RowStop::failed("camera-scale-refused"));
    }
    let vertical = required_f64(
        &sidecar.json,
        "/canvasPixelsPerSourcePixel/vertical",
        "camera-scale-refused",
    )?;
    let horizontal = required_f64(
        &sidecar.json,
        "/canvasPixelsPerSourcePixel/horizontal",
        "camera-scale-refused",
    )?;
    let millimetres = 2.0 * parallel / f64::from(height);
    if !millimetres.is_finite() || millimetres <= 0.0 {
        return Err(RowStop::failed("camera-scale-refused"));
    }
    let expected_vertical = row_mm / millimetres;
    let expected_horizontal = column_mm / millimetres;
    if !within_world(vertical, expected_vertical) || !within_world(horizontal, expected_horizontal)
    {
        return Err(RowStop::failed("camera-scale-disagrees"));
    }
    let horizontal_basis_reversed = accept_pose(&sidecar.json, rows, columns, row_mm, column_mm)?;

    let mut view = View::new();
    view.set_spacing(column_mm, row_mm);
    view.set_flip_horizontal(horizontal_basis_reversed);
    view.set_flip_vertical(false);
    view.fit(f64::from(width), f64::from(height), columns, rows);
    let fitted = view.scale();
    let fitted_parallel = f64::from(height) / (2.0 * fitted);
    if !fitted.is_finite() || !fitted_parallel.is_finite() || fitted_parallel <= 0.0 {
        return Err(RowStop::failed("camera-scale-refused"));
    }
    if !within_world(fitted_parallel, parallel) {
        let zoom = fitted_parallel / parallel;
        if !zoom.is_finite() || zoom <= 0.0 {
            return Err(RowStop::failed("camera-scale-refused"));
        }
        view.set_zoom(Zoom::new(zoom));
    }
    Ok(ExecutedCamera {
        view,
        parallel_scale: parallel,
        vertical,
        horizontal,
        flip_horizontal,
        flip_vertical,
    })
}

/// Accept a supported pose. `Ok(true)` is the positive-normal side.
///
/// The reference right vector is `viewUp × viewPlaneNormal`, and the
/// view-plane normal points from the focal point toward the camera.
/// Supported view-up is the negated column, so that right vector equals the
/// row direction on the negative-normal side. The unflipped view already
/// maps increasing column index to the right. The positive-normal side
/// reverses it.
fn accept_pose(
    sidecar: &Value,
    rows: u32,
    columns: u32,
    row_mm: f64,
    column_mm: f64,
) -> Result<bool, RowStop> {
    let plane = image_plane(sidecar, rows, columns, row_mm, column_mm)?;
    let centre = image_centre(&plane);
    let focal = json_triple(sidecar, "/camera/focalPoint")
        .ok_or_else(|| RowStop::failed("camera-placement-refused"))?;
    let position = json_triple(sidecar, "/camera/position")
        .ok_or_else(|| RowStop::failed("camera-orientation-refused"))?;
    let view_up = json_triple(sidecar, "/camera/viewUp")
        .ok_or_else(|| RowStop::failed("camera-orientation-refused"))?;
    if !triple_within(focal, centre) {
        return Err(RowStop::failed("camera-placement-refused"));
    }
    let expected_up = (-plane.orientation.column()).to_array();
    if !triple_within(view_up, expected_up) {
        return Err(RowStop::failed("camera-orientation-refused"));
    }
    let normal = plane.slice_normal();
    let gaze_x = position[0] - focal[0];
    let gaze_y = position[1] - focal[1];
    let gaze_z = position[2] - focal[2];
    let along = gaze_x * normal.x + gaze_y * normal.y + gaze_z * normal.z;
    let lateral_x = gaze_x - normal.x * along;
    let lateral_y = gaze_y - normal.y * along;
    let lateral_z = gaze_z - normal.z * along;
    let lateral = (lateral_x * lateral_x + lateral_y * lateral_y + lateral_z * lateral_z).sqrt();
    if !along.is_finite() || along.abs() <= WORLD_TOLERANCE_MM || lateral > WORLD_TOLERANCE_MM {
        return Err(RowStop::failed("camera-orientation-refused"));
    }
    Ok(along > 0.0)
}

fn image_plane(
    sidecar: &Value,
    rows: u32,
    columns: u32,
    row_mm: f64,
    column_mm: f64,
) -> Result<ImagePlane, RowStop> {
    let position = json_triple(
        sidecar,
        "/cornerstoneMetadata/imagePlaneModule/imagePositionPatient",
    )
    .or_else(|| json_triple(sidecar, "/attributes/imagePositionPatient"))
    .ok_or_else(|| RowStop::failed("camera-placement-refused"))?;
    let (row, column) = direction_pair(sidecar)?;
    let position = ImagePositionPatient::new(position)
        .map_err(|_| RowStop::failed("camera-placement-refused"))?;
    let orientation = ImageOrientationPatient::new(row, column)
        .map_err(|_| RowStop::failed("camera-orientation-refused"))?;
    let spacing =
        PixelSpacing::new(row_mm, column_mm).map_err(|_| RowStop::failed("spacing-refused"))?;
    let dimensions =
        ImageDimensions::new(rows, columns).map_err(|_| RowStop::failed("pixel-layout-refused"))?;
    ImagePlane::new(position, orientation, spacing, dimensions)
        .map_err(|_| RowStop::failed("spacing-refused"))
}

fn direction_pair(sidecar: &Value) -> Result<([f64; 3], [f64; 3]), RowStop> {
    let row = json_triple(sidecar, "/cornerstoneMetadata/imagePlaneModule/rowCosines");
    let column = json_triple(
        sidecar,
        "/cornerstoneMetadata/imagePlaneModule/columnCosines",
    );
    if let (Some(row), Some(column)) = (row, column) {
        return Ok((row, column));
    }
    let orientation = sidecar
        .pointer("/attributes/imageOrientationPatient")
        .and_then(Value::as_array)
        .ok_or_else(|| RowStop::failed("camera-orientation-refused"))?;
    if orientation.len() != 6 {
        return Err(RowStop::failed("camera-orientation-refused"));
    }
    let mut row = [0.0; 3];
    let mut column = [0.0; 3];
    for (index, slot) in row.iter_mut().enumerate() {
        *slot = orientation
            .get(index)
            .and_then(json_f64)
            .ok_or_else(|| RowStop::failed("camera-orientation-refused"))?;
    }
    for (index, slot) in column.iter_mut().enumerate() {
        *slot = orientation
            .get(index.saturating_add(3))
            .and_then(json_f64)
            .ok_or_else(|| RowStop::failed("camera-orientation-refused"))?;
    }
    Ok((row, column))
}

fn image_centre(plane: &ImagePlane) -> [f64; 3] {
    let centre_column = (f64::from(plane.dimensions().columns()) - 1.0) / 2.0;
    let centre_row = (f64::from(plane.dimensions().rows()) - 1.0) / 2.0;
    let world = plane.position_vector()
        + plane.orientation.row() * plane.spacing().column_mm() * centre_column
        + plane.orientation.column() * plane.spacing().row_mm() * centre_row;
    world.to_array()
}

fn within_world(left: f64, right: f64) -> bool {
    (left - right).abs() <= WORLD_TOLERANCE_MM
}

fn triple_within(left: [f64; 3], right: [f64; 3]) -> bool {
    within_world(left[0], right[0])
        && within_world(left[1], right[1])
        && within_world(left[2], right[2])
}

fn json_triple(sidecar: &Value, pointer: &str) -> Option<[f64; 3]> {
    let list = sidecar.pointer(pointer)?.as_array()?;
    if list.len() != 3 {
        return None;
    }
    let mut out = [0.0; 3];
    for (slot, value) in out.iter_mut().zip(list.iter()) {
        let number = json_f64(value)?;
        if !number.is_finite() {
            return None;
        }
        *slot = number;
    }
    Some(out)
}

fn write_subset_runs(
    reference: &Run,
    paths: &ProducePaths,
    produced: &BTreeSet<String>,
    written: &[Ready],
    execution: &Value,
) -> Result<(), &'static str> {
    let digests = written
        .iter()
        .map(|frame| (frame.id.clone(), frame.digest.clone()))
        .collect::<BTreeMap<_, _>>();
    let reference_rows = subset_rows(&reference.json, produced, None);
    let candidate_rows = subset_rows(&reference.json, produced, Some(&digests));
    let low = filter_entries(
        reference,
        reference.json.pointer("/lowInformation/rows"),
        None,
        produced,
    );
    let down = filter_entries(
        reference,
        reference.json.pointer("/downsampled"),
        Some(ViewKind::Stack),
        produced,
    );
    write_run_json(
        &paths.reference_subset.join("run.json"),
        &reference.json,
        reference_rows,
        &low,
        &down,
    )?;
    // Inherited fields identify the source reference. Candidate execution is
    // recorded separately so Chromium readback cannot be mistaken for ours.
    let mut candidate_source = reference.json.clone();
    candidate_source
        .as_object_mut()
        .ok_or("output-write-refused")?
        .insert("ocelliCandidateExecution".to_owned(), execution.clone());
    write_run_json(
        &paths.candidate_run.join("run.json"),
        &candidate_source,
        candidate_rows,
        &low,
        &down,
    )?;
    Ok(())
}

fn subset_rows(
    run: &Value,
    produced: &BTreeSet<String>,
    digests: Option<&BTreeMap<String, String>>,
) -> Vec<Value> {
    let Some(rows) = run.pointer("/rows").and_then(Value::as_array) else {
        return Vec::new();
    };
    let mut kept = Vec::new();
    for row in rows {
        let Some(id) = row.pointer("/id").and_then(Value::as_str) else {
            continue;
        };
        if !produced.contains(id) {
            continue;
        }
        let mut copy = row.clone();
        if let Some(digests) = digests
            && let Some(digest) = digests.get(id)
            && let Some(object) = copy.as_object_mut()
        {
            object.insert("sha256".to_owned(), Value::String(digest.clone()));
        }
        kept.push(copy);
    }
    kept
}

fn filter_entries(
    reference: &Run,
    entries: Option<&Value>,
    default_kind: Option<ViewKind>,
    produced: &BTreeSet<String>,
) -> Vec<Value> {
    let Some(entries) = entries.and_then(Value::as_array) else {
        return Vec::new();
    };
    entries
        .iter()
        .filter(
            |entry| match reference.resolve_census_entry(entry, default_kind) {
                Ok(ids) => !ids.is_empty() && ids.iter().all(|id| produced.contains(id)),
                Err(_) => false,
            },
        )
        .cloned()
        .collect()
}

fn write_run_json(
    path: &Path,
    original: &Value,
    rows: Vec<Value>,
    low: &[Value],
    down: &[Value],
) -> Result<(), &'static str> {
    let mut json = original.clone();
    let Some(object) = json.as_object_mut() else {
        return Err("output-write-refused");
    };
    object.insert("rows".to_owned(), Value::Array(rows));
    object.insert("volumes".to_owned(), Value::Array(Vec::new()));
    object.insert("downsampled".to_owned(), Value::Array(down.to_vec()));
    if let Some(info) = object
        .get_mut("lowInformation")
        .and_then(Value::as_object_mut)
    {
        info.insert("rows".to_owned(), Value::Array(low.to_vec()));
    }
    let text = serde_json::to_vec_pretty(&json).map_err(|_| "output-write-refused")?;
    fs::write(path, text).map_err(|_| "output-write-refused")
}

fn compare_written(
    reference_dir: &Path,
    candidate_dir: &Path,
    written: &[Ready],
    cases: &mut Vec<CaseRecord>,
) {
    let reference = match Run::load(reference_dir) {
        Ok(run) => run,
        Err(_) => {
            refuse_written(written, cases);
            return;
        }
    };
    let candidate = match Run::load(candidate_dir) {
        Ok(run) => run,
        Err(_) => {
            refuse_written(written, cases);
            return;
        }
    };
    let low = match reference.low_information() {
        Ok(ids) => ids,
        Err(_) => {
            refuse_written(written, cases);
            return;
        }
    };
    let down = match reference.downsampled() {
        Ok(ids) => ids,
        Err(_) => {
            refuse_written(written, cases);
            return;
        }
    };
    let register = load_register();
    let context = Context {
        reference: &reference,
        candidate: &candidate,
        register: &register,
        low_information: &low,
        downsampled: &down,
    };
    for frame in written {
        let compared = reference
            .read_frame(&frame.id)
            .ok()
            .zip(candidate.read_frame(&frame.id).ok())
            .and_then(|(reference_frame, candidate_frame)| {
                compare_view(&context, &frame.id, &reference_frame, &candidate_frame).ok()
            });
        let Some(record) = compared else {
            cases.push(CaseRecord {
                id: frame.id.clone(),
                disposition: "failed",
                reason: "comparator-refused",
            });
            continue;
        };
        let reason = match record.outcome {
            Outcome::Pass => "pass",
            Outcome::Fail => "pixel-fail",
            Outcome::Unmeasured => "unmeasured",
            Outcome::Absent => {
                cases.push(CaseRecord {
                    id: frame.id.clone(),
                    disposition: "failed",
                    reason: "comparator-refused",
                });
                continue;
            }
        };
        cases.push(CaseRecord {
            id: frame.id.clone(),
            disposition: "compared",
            reason,
        });
    }
}

fn refuse_written(written: &[Ready], cases: &mut Vec<CaseRecord>) {
    for frame in written {
        cases.push(CaseRecord {
            id: frame.id.clone(),
            disposition: "failed",
            reason: "comparator-refused",
        });
    }
}

fn load_register() -> Register {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("reference-divergence.json");
    let Ok(text) = fs::read_to_string(&path) else {
        return Register::default();
    };
    let Ok(json) = serde_json::from_str::<Value>(&text) else {
        return Register::default();
    };
    Register::from_json(&json).unwrap_or_default()
}

fn write_dispositions(path: &Path, summary: &ProduceSummary) {
    if let Some(parent) = path.parent()
        && !parent.as_os_str().is_empty()
    {
        let _ = fs::create_dir_all(parent);
    }
    let cases = summary
        .cases
        .iter()
        .map(|case| {
            let mut object = Map::new();
            object.insert("id".to_owned(), Value::String(case.id.clone()));
            object.insert(
                "disposition".to_owned(),
                Value::String(case.disposition.to_owned()),
            );
            object.insert("reason".to_owned(), Value::String(case.reason.to_owned()));
            Value::Object(object)
        })
        .collect::<Vec<_>>();
    let mut object = Map::new();
    object.insert(
        "resources".to_owned(),
        Value::String(summary.resources.to_owned()),
    );
    insert_count(&mut object, "selected", summary.selected);
    insert_count(&mut object, "produced", summary.produced);
    insert_count(&mut object, "compared", summary.compared);
    insert_count(&mut object, "unsupported", summary.unsupported);
    insert_count(&mut object, "failed", summary.failed);
    insert_count(&mut object, "unrun", summary.unrun);
    insert_count(&mut object, "pass", summary.pass);
    insert_count(&mut object, "outcomeFail", summary.outcome_fail);
    insert_count(&mut object, "unmeasured", summary.unmeasured);
    insert_count(&mut object, "produceFailed", summary.produce_failed);
    insert_count(&mut object, "comparatorRefused", summary.comparator_refused);
    object.insert("tierA".to_owned(), Value::String(summary.tier_a.to_owned()));
    object.insert(
        "instruments".to_owned(),
        Value::String(summary.instruments.to_owned()),
    );
    object.insert(
        "collector".to_owned(),
        Value::String(summary.collector.to_owned()),
    );
    object.insert(
        "hostMeasurement".to_owned(),
        Value::String(summary.host_measurement.to_owned()),
    );
    object.insert("cases".to_owned(), Value::Array(cases));
    let Ok(text) = serde_json::to_vec_pretty(&Value::Object(object)) else {
        return;
    };
    let _ = fs::write(path, text);
}

fn insert_count(object: &mut Map<String, Value>, key: &str, count: usize) {
    let Some(number) = u64::try_from(count).ok() else {
        return;
    };
    object.insert(key.to_owned(), Value::from(number));
}

fn reset_directory(path: &Path) -> Result<(), ()> {
    if path.exists() {
        fs::remove_dir_all(path).map_err(|_| ())?;
    }
    fs::create_dir_all(path).map_err(|_| ())
}

fn output_overlaps(left: &Path, right: &Path) -> bool {
    left == right || left.starts_with(right) || right.starts_with(left)
}

fn identifier_ok(id: &str) -> bool {
    !id.is_empty()
        && id
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_' || byte == b'-')
}

fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    let mut text = String::with_capacity(digest.len().saturating_mul(2));
    for byte in digest {
        text.push_str(&format!("{byte:02x}"));
    }
    text
}

#[cfg(test)]
mod tests {
    use super::{ProducePaths, produce_applicable, stack_selection_reason};
    use serde_json::json;
    use std::fs;
    use std::path::PathBuf;

    fn classified(photo: &str, samples: u64, color: bool, mono16: bool) -> serde_json::Value {
        let mut categories = vec!["synthetic".to_owned()];
        if mono16 {
            categories.push("mono16".to_owned());
        }
        json!({
            "attributes": {
                "photometricInterpretation": photo,
                "samplesPerPixel": samples
            },
            "image": { "color": color },
            "row": { "categories": categories }
        })
    }

    #[test]
    fn mono16_is_selected_and_colour_is_not() {
        assert_eq!(
            stack_selection_reason(&classified("MONOCHROME2", 1, false, true)),
            None
        );
        assert_eq!(
            stack_selection_reason(&classified("MONOCHROME1", 1, false, true)),
            None
        );
        assert_eq!(
            stack_selection_reason(&classified("MONOCHROME2", 1, false, false)),
            Some("class-not-monochrome")
        );
        assert_eq!(
            stack_selection_reason(&classified("PALETTE COLOR", 1, true, false)),
            Some("palette-not-scored")
        );
        assert_eq!(
            stack_selection_reason(&classified("YBR_FULL_422", 3, true, false)),
            Some("ybr-not-scored")
        );
        assert_eq!(
            stack_selection_reason(&classified("RGB", 3, true, false)),
            Some("colour-not-scored")
        );
    }

    #[test]
    fn a_missing_reference_run_is_unrun() {
        let dir =
            std::env::temp_dir().join(format!("ocelli-produce-missing-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        let summary = produce_applicable(&ProducePaths {
            reference_run: dir.join("reference"),
            corpus_root: dir.join("corpus"),
            reference_subset: dir.join("subset"),
            candidate_run: dir.join("candidate"),
            dispositions: dir.join("dispositions.json"),
        });
        assert_eq!(summary.resources, "reference-run-absent");
        assert_eq!(summary.compared, 0);
        assert_eq!(summary.unrun, 1);
        assert_eq!(summary.selected, 0);
        assert_eq!(summary.tier_a, "unrun");
        assert_eq!(summary.instruments, "not-run-by-producer");
        assert_eq!(summary.collector, "not-run-by-producer");
        assert_eq!(summary.host_measurement, "unavailable");
        assert!(summary.accounts());
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn applicable_monochrome_rows_reach_the_comparator() {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
        let reference = root.join("tools/oracle/out");
        let corpus = root.join("corpus/data");
        let out = root.join("tools/oracle/compare-out");
        if !reference.join("run.json").is_file() || !corpus.is_dir() {
            let summary = produce_applicable(&ProducePaths {
                reference_run: root.join("tools/oracle/missing-run"),
                corpus_root: corpus,
                reference_subset: out.join("reference-selected"),
                candidate_run: out.join("candidate"),
                dispositions: out.join("dispositions.json"),
            });
            assert_eq!(summary.resources, "reference-run-absent");
            assert_eq!(summary.compared, 0);
            assert_eq!(summary.unrun, 1);
            assert!(summary.accounts());
            return;
        }
        let summary = produce_applicable(&ProducePaths {
            reference_run: reference,
            corpus_root: corpus,
            reference_subset: out.join("reference-selected"),
            candidate_run: out.join("candidate"),
            dispositions: out.join("dispositions.json"),
        });
        assert_eq!(summary.resources, "ran");
        assert!(summary.selected > 0);
        assert!(summary.accounts(), "{summary:?}");
        assert_eq!(summary.tier_a, "unrun");
        assert_eq!(summary.instruments, "not-run-by-producer");
        assert_eq!(summary.collector, "not-run-by-producer");
        assert_eq!(summary.host_measurement, "unavailable");
        // High bit 15 with 12 bits stored is unpacked at the conforming
        // high bit. The reference frame matches, so the row is a pass.
        // `StoredBits::new` still refuses that file high bit on its own.
        assert_eq!(
            summary.case_disposition("synthetic__ct_signed_12in16_left"),
            Some("compared"),
            "{}",
            summary
                .case_reason("synthetic__ct_signed_12in16_left")
                .unwrap_or("absent")
        );
        assert_eq!(
            summary.case_reason("synthetic__ct_signed_12in16_left"),
            Some("pass")
        );
        assert_eq!(
            summary.case_disposition("synthetic__ct_unsigned_16"),
            Some("compared"),
            "{}",
            summary
                .case_reason("synthetic__ct_unsigned_16")
                .unwrap_or("absent")
        );
        assert_eq!(
            summary.case_disposition("synthetic__ct_signed_12in16_right"),
            Some("compared"),
            "{}",
            summary
                .case_reason("synthetic__ct_signed_12in16_right")
                .unwrap_or("absent")
        );
        assert_eq!(
            summary.case_disposition("synthetic__ct_multiframe_perframe"),
            Some("compared"),
            "{}",
            summary
                .case_reason("synthetic__ct_multiframe_perframe")
                .unwrap_or("absent")
        );
        assert_eq!(
            summary.case_reason("synthetic__sc_palette_color"),
            Some("palette-not-scored")
        );
        assert_eq!(
            summary.case_reason("synthetic__sc_rgb_interleaved"),
            Some("colour-not-scored")
        );
        assert_eq!(
            summary.case_reason("synthetic__us_ybr_full_422"),
            Some("reference-row-not-rendered")
        );
        assert_eq!(
            summary.produced,
            summary.compared + summary.comparator_refused
        );
    }

    use super::{ExecutedCamera, candidate_sidecar, executed_camera, pixel_facts, raster_settled};
    use crate::sidecar::{Sidecar, ViewKind};
    use ocelli_core::Stored;

    const SAMPLES: [Stored; 4] = [Stored(0.0), Stored(40.0), Stored(0.0), Stored(40.0)];

    fn centred(parallel: f64, scale: f64, flip_horizontal: bool) -> Sidecar {
        Sidecar {
            id: "synthetic-camera".to_owned(),
            kind: ViewKind::Stack,
            json: json!({
                "frame": {"width": 8, "height": 8, "sha256": "synthetic"},
                "attributes": {
                    "rows": 2,
                    "columns": 2,
                    "pixelSpacing": [1.0, 1.0],
                    "photometricInterpretation": "MONOCHROME2",
                    "imageOrientationPatient": [1.0, 0.0, 0.0, 0.0, 1.0, 0.0]
                },
                "image": {"rows": 2, "columns": 2, "slope": 1.0, "intercept": 0.0},
                "voi": {
                    "source": "file",
                    "windowCenter": 40.0,
                    "windowWidth": 1.0,
                    "voiLutFunction": "LINEAR"
                },
                "cornerstoneMetadata": {
                    "imagePlaneModule": {
                        "imagePositionPatient": [0.0, 0.0, 0.0],
                        "rowCosines": [1.0, 0.0, 0.0],
                        "columnCosines": [0.0, 1.0, 0.0]
                    }
                },
                "camera": {
                    "position": [0.5, 0.5, 10.0],
                    "focalPoint": [0.5, 0.5, 0.0],
                    "viewUp": [0.0, -1.0, 0.0],
                    "parallelScale": parallel,
                    "flipHorizontal": flip_horizontal
                },
                "canvasPixelsPerSourcePixel": {"vertical": scale, "horizontal": scale}
            }),
        }
    }

    fn with_z(mut sidecar: Sidecar, z: f64) -> Sidecar {
        let camera = match sidecar
            .json
            .get_mut("camera")
            .and_then(|value| value.as_object_mut())
        {
            Some(camera) => camera,
            None => unreachable!("the fixture has a camera"),
        };
        camera.insert("position".to_owned(), json!([0.5, 0.5, z]));
        sidecar
    }

    fn with_view_up(mut sidecar: Sidecar, view_up: [f64; 3]) -> Sidecar {
        let camera = match sidecar
            .json
            .get_mut("camera")
            .and_then(|value| value.as_object_mut())
        {
            Some(camera) => camera,
            None => unreachable!("the fixture has a camera"),
        };
        camera.insert(
            "viewUp".to_owned(),
            json!([view_up[0], view_up[1], view_up[2]]),
        );
        sidecar
    }

    fn with_vertical_flag(mut sidecar: Sidecar, flip_vertical: bool) -> Sidecar {
        let camera = match sidecar
            .json
            .get_mut("camera")
            .and_then(|value| value.as_object_mut())
        {
            Some(camera) => camera,
            None => unreachable!("the fixture has a camera"),
        };
        camera.insert("flipVertical".to_owned(), json!(flip_vertical));
        sidecar
    }

    fn draw(sidecar: &Sidecar) -> Result<Vec<u8>, &'static str> {
        draw_samples(sidecar, &SAMPLES)
    }

    fn draw_samples(sidecar: &Sidecar, samples: &[Stored]) -> Result<Vec<u8>, &'static str> {
        match raster_settled(sidecar, samples) {
            Ok((rgba, _, _)) => Ok(rgba),
            Err(stop) => Err(stop.reason),
        }
    }

    fn first_channel(frame: &[u8], pixel: usize) -> Option<u8> {
        frame.get(pixel.saturating_mul(4)).copied()
    }

    fn published(sidecar: &Sidecar, executed: &ExecutedCamera) -> serde_json::Value {
        candidate_sidecar(sidecar.json.clone(), executed, "synthetic-digest")
    }

    #[test]
    fn zoom_changes_a_nonconstant_frame_and_the_sidecar_records_it() {
        // The negative-normal side keeps column 0 on the left. Scale does not
        // depend on that side. The signed-basis test covers the positive side.
        let fitted = with_z(centred(1.0, 4.0, false), -10.0);
        let zoomed = with_z(centred(2.0, 2.0, false), -10.0);
        let before = match draw(&fitted) {
            Ok(frame) => frame,
            Err(reason) => unreachable!("fitted camera is supported, refused as {reason}"),
        };
        let after = match draw(&zoomed) {
            Ok(frame) => frame,
            Err(reason) => unreachable!("doubled field of view is supported, refused as {reason}"),
        };
        assert_eq!(first_channel(&before, 0), Some(0));
        assert_eq!(first_channel(&before, 6), Some(255));
        assert_eq!(first_channel(&after, 0), Some(0));
        assert_eq!(first_channel(&after, 6), Some(0));
        assert_ne!(before, after);
        let executed = match executed_camera(&zoomed) {
            Ok(camera) => camera,
            Err(stop) => unreachable!("zoomed camera was drawn, then {}", stop.reason),
        };
        let sidecar = published(&zoomed, &executed);
        assert!(near(
            sidecar
                .pointer("/camera/parallelScale")
                .and_then(serde_json::Value::as_f64),
            2.0
        ));
        assert!(near(
            sidecar
                .pointer("/canvasPixelsPerSourcePixel/vertical")
                .and_then(serde_json::Value::as_f64),
            2.0
        ));
        assert!(near(
            sidecar
                .pointer("/canvasPixelsPerSourcePixel/horizontal")
                .and_then(serde_json::Value::as_f64),
            2.0
        ));
        let rendered_vertical = executed.view.scale();
        assert!((rendered_vertical - 2.0).abs() <= 1e-6);
    }

    /// Cornerstone `Viewport.flip({ flipHorizontal: true })` negates the
    /// view-plane normal, moves position to `focal + newNormal * distance`,
    /// and toggles the observed flag. View-up stays put.
    ///
    /// From the centred negative-Z readback, one horizontal flip is therefore
    /// position z = 10, view-up `[0, -1, 0]` and `flipHorizontal` true. A
    /// second flip returns z = -10 and the flag to false. Samples
    /// `[0, 40, 0, 40]` with LINEAR centre 40 and width 1 put black in column
    /// 0 and white in column 1. The moved camera already places white on the
    /// left. The flag is not a second reflection.
    #[test]
    fn a_reference_horizontal_flip_readback_is_applied_once() {
        let reset = with_z(centred(1.0, 4.0, false), -10.0);
        let flipped = centred(1.0, 4.0, true);
        let same_pose = centred(1.0, 4.0, false);
        let restored = with_z(centred(1.0, 4.0, false), -10.0);
        let reset_frame = match draw(&reset) {
            Ok(frame) => frame,
            Err(reason) => unreachable!("reset camera is supported, refused as {reason}"),
        };
        let flipped_frame = match draw(&flipped) {
            Ok(frame) => frame,
            Err(reason) => unreachable!("flipped readback is supported, refused as {reason}"),
        };
        let restored_frame = match draw(&restored) {
            Ok(frame) => frame,
            Err(reason) => unreachable!("flip back is supported, refused as {reason}"),
        };
        let same_pose_frame = match draw(&same_pose) {
            Ok(frame) => frame,
            Err(reason) => unreachable!("positive pose is supported, refused as {reason}"),
        };
        assert_eq!(first_channel(&reset_frame, 0), Some(0));
        assert_eq!(first_channel(&reset_frame, 6), Some(255));
        assert_eq!(first_channel(&flipped_frame, 0), Some(255));
        assert_eq!(first_channel(&flipped_frame, 6), Some(0));
        assert_ne!(flipped_frame, reset_frame);
        assert_eq!(flipped_frame, same_pose_frame);
        assert_eq!(restored_frame, reset_frame);

        let flipped_executed = match executed_camera(&flipped) {
            Ok(camera) => camera,
            Err(stop) => unreachable!("flipped readback was drawn, then {}", stop.reason),
        };
        let same_pose_executed = match executed_camera(&same_pose) {
            Ok(camera) => camera,
            Err(stop) => unreachable!("positive pose was drawn, then {}", stop.reason),
        };
        let restored_executed = match executed_camera(&restored) {
            Ok(camera) => camera,
            Err(stop) => unreachable!("flip back was drawn, then {}", stop.reason),
        };
        let flipped_sidecar = published(&flipped, &flipped_executed);
        let same_pose_sidecar = published(&same_pose, &same_pose_executed);
        let restored_sidecar = published(&restored, &restored_executed);
        assert_eq!(
            flipped_sidecar
                .pointer("/camera/flipHorizontal")
                .and_then(serde_json::Value::as_bool),
            Some(true)
        );
        assert_eq!(
            restored_sidecar
                .pointer("/camera/flipHorizontal")
                .and_then(serde_json::Value::as_bool),
            Some(false)
        );
        assert_eq!(
            same_pose_sidecar
                .pointer("/camera/flipHorizontal")
                .and_then(serde_json::Value::as_bool),
            Some(false)
        );
        assert!(flipped_executed.flip_horizontal);
        assert!(!same_pose_executed.flip_horizontal);
        assert!(near(
            flipped_sidecar
                .pointer("/camera/position/2")
                .and_then(serde_json::Value::as_f64),
            10.0
        ));
        assert!(near(
            restored_sidecar
                .pointer("/camera/position/2")
                .and_then(serde_json::Value::as_f64),
            -10.0
        ));
        assert!(near(
            flipped_sidecar
                .pointer("/camera/viewUp/1")
                .and_then(serde_json::Value::as_f64),
            -1.0
        ));
        assert_eq!(
            flipped_sidecar
                .pointer("/camera/flipVertical")
                .and_then(serde_json::Value::as_bool),
            Some(false)
        );

        // One `flip` call with both flags negates view-up and lands on the
        // positive-normal side with both observed flags true. A vertical-only
        // flip keeps the horizontal flag false and also negates view-up.
        // Neither pose is the supported column order.
        let combined =
            with_vertical_flag(with_view_up(centred(1.0, 4.0, true), [0.0, 1.0, 0.0]), true);
        let vertical = with_vertical_flag(
            with_view_up(centred(1.0, 4.0, false), [0.0, 1.0, 0.0]),
            true,
        );
        assert_eq!(draw(&combined), Err("camera-orientation-refused"));
        assert_eq!(draw(&vertical), Err("camera-orientation-refused"));
        assert!(executed_camera(&combined).is_err());
        assert!(executed_camera(&vertical).is_err());
    }

    /// Identity orientation, view-up `[0, -1, 0]`, samples `[0, 40, 0, 40]`.
    ///
    /// The reference right vector is `viewUp × viewPlaneNormal`. The
    /// view-plane normal points from the focal point toward the camera, so
    /// z = -10 gives right `[1, 0, 0]` and z = 10 gives right `[-1, 0, 0]`.
    /// Column 0 is black and column 1 is white under LINEAR centre 40 and
    /// width 1. Both sidecars carry `flipHorizontal` false. The positive side
    /// still swaps the columns, and the published flag stays false because
    /// that flag is the readback rather than the raster coefficient.
    #[test]
    fn each_gaze_sign_applies_its_own_horizontal_basis() {
        let positive = centred(1.0, 4.0, false);
        let negative = with_z(centred(1.0, 4.0, false), -10.0);
        let positive_frame = match draw(&positive) {
            Ok(frame) => frame,
            Err(reason) => unreachable!("positive-normal gaze is supported, refused as {reason}"),
        };
        let negative_frame = match draw(&negative) {
            Ok(frame) => frame,
            Err(reason) => unreachable!("negative-normal gaze is supported, refused as {reason}"),
        };
        assert_eq!(first_channel(&negative_frame, 0), Some(0));
        assert_eq!(first_channel(&negative_frame, 6), Some(255));
        assert_eq!(first_channel(&positive_frame, 0), Some(255));
        assert_eq!(first_channel(&positive_frame, 6), Some(0));
        assert_ne!(positive_frame, negative_frame);

        let positive_executed = match executed_camera(&positive) {
            Ok(camera) => camera,
            Err(stop) => unreachable!("positive side was drawn, then {}", stop.reason),
        };
        let negative_executed = match executed_camera(&negative) {
            Ok(camera) => camera,
            Err(stop) => unreachable!("negative side was drawn, then {}", stop.reason),
        };
        assert!(!positive_executed.flip_horizontal);
        assert!(!negative_executed.flip_horizontal);
        assert!((positive_executed.view.scale() - 4.0).abs() <= 1e-6);
        assert!((negative_executed.view.scale() - 4.0).abs() <= 1e-6);

        let positive_sidecar = published(&positive, &positive_executed);
        let negative_sidecar = published(&negative, &negative_executed);
        assert_eq!(
            positive_sidecar
                .pointer("/camera/flipHorizontal")
                .and_then(serde_json::Value::as_bool),
            Some(false)
        );
        assert_eq!(
            negative_sidecar
                .pointer("/camera/flipHorizontal")
                .and_then(serde_json::Value::as_bool),
            Some(false)
        );
        assert!(near(
            positive_sidecar
                .pointer("/camera/position/2")
                .and_then(serde_json::Value::as_f64),
            10.0
        ));
        assert!(near(
            negative_sidecar
                .pointer("/camera/position/2")
                .and_then(serde_json::Value::as_f64),
            -10.0
        ));
        assert!(near(
            positive_sidecar
                .pointer("/camera/focalPoint/0")
                .and_then(serde_json::Value::as_f64),
            0.5
        ));
        assert!(near(
            positive_sidecar
                .pointer("/camera/focalPoint/1")
                .and_then(serde_json::Value::as_f64),
            0.5
        ));
        assert!(near(
            positive_sidecar
                .pointer("/camera/focalPoint/2")
                .and_then(serde_json::Value::as_f64),
            0.0
        ));
        assert!(near(
            positive_sidecar
                .pointer("/camera/viewUp/0")
                .and_then(serde_json::Value::as_f64),
            0.0
        ));
        assert!(near(
            positive_sidecar
                .pointer("/camera/viewUp/1")
                .and_then(serde_json::Value::as_f64),
            -1.0
        ));
        assert!(near(
            positive_sidecar
                .pointer("/camera/viewUp/2")
                .and_then(serde_json::Value::as_f64),
            0.0
        ));
        assert!(near(
            negative_sidecar
                .pointer("/camera/viewUp/1")
                .and_then(serde_json::Value::as_f64),
            -1.0
        ));
        assert!(near(
            positive_sidecar
                .pointer("/camera/parallelScale")
                .and_then(serde_json::Value::as_f64),
            1.0
        ));
        assert!(near(
            positive_sidecar
                .pointer("/canvasPixelsPerSourcePixel/horizontal")
                .and_then(serde_json::Value::as_f64),
            4.0
        ));
        assert_eq!(
            negative_sidecar
                .pointer("/camera/flipVertical")
                .and_then(serde_json::Value::as_bool),
            Some(false)
        );
        assert_eq!(
            positive_sidecar
                .pointer("/camera/flipVertical")
                .and_then(serde_json::Value::as_bool),
            Some(false)
        );
    }

    fn near(value: Option<f64>, expected: f64) -> bool {
        value.is_some_and(|value| (value - expected).abs() <= 1e-6)
    }

    #[test]
    fn an_off_centre_focal_point_is_refused_before_publication() {
        let mut shifted = centred(1.0, 4.0, false);
        let camera = match shifted
            .json
            .get_mut("camera")
            .and_then(|value| value.as_object_mut())
        {
            Some(camera) => camera,
            None => unreachable!("the fixture has a camera"),
        };
        camera.insert("focalPoint".to_owned(), json!([0.0, 0.0, 0.0]));
        assert_eq!(draw(&shifted), Err("camera-placement-refused"));
        assert!(executed_camera(&shifted).is_err());
    }

    #[test]
    fn a_rotated_view_up_is_refused() {
        let mut rotated = centred(1.0, 4.0, false);
        let camera = match rotated
            .json
            .get_mut("camera")
            .and_then(|value| value.as_object_mut())
        {
            Some(camera) => camera,
            None => unreachable!("the fixture has a camera"),
        };
        camera.insert("viewUp".to_owned(), json!([1.0, 0.0, 0.0]));
        assert_eq!(draw(&rotated), Err("camera-orientation-refused"));
    }

    #[test]
    fn a_scale_that_disagrees_with_parallel_scale_is_refused() {
        let inconsistent = centred(2.0, 4.0, false);
        assert_eq!(draw(&inconsistent), Err("camera-scale-disagrees"));
    }

    /// PS3.3 C.11.2.1.2 at stored 40, centre 40, width 400.
    ///
    /// `((40 - 39.5) / 399 + 0.5) * 255` is 127.819547.... The grey table
    /// rounds a positive display to the nearest code, the same rule as the
    /// shader's `table_index`, so the byte is 128. The pinned reference
    /// quantises with `Math.floor(value * 255 + 0.5)`, and HLD 25.1's
    /// signed-mean bound rejects the one-code truncation this used to assert.
    #[test]
    fn a_fractional_linear_display_keeps_the_nearest_code() {
        let sidecar = filled_one(40.0, 400.0, "LINEAR", 1.0, 0.0);
        let frame = match draw_samples(&sidecar, &[Stored(40.0)]) {
            Ok(frame) => frame,
            Err(reason) => unreachable!("soft-tissue window is supported, refused as {reason}"),
        };
        assert_eq!(first_channel(&frame, 0), Some(128));
        assert_eq!(first_channel(&frame, 3), Some(128));
        assert_eq!(frame.get(3).copied(), Some(255));
    }

    /// A canvas pixel whose centre lies outside the fitted extent is background.
    ///
    /// Source is 1 by 1 millimetre on an 8 by 4 canvas. The fit scale is 4,
    /// so the image is 4 by 4 canvas pixels and the horizontal remainder is
    /// centred: the extent starts at x = 2. Pixel 0 has centre 0.5 and is
    /// outside. Pixel 2 has centre 2.5 and is inside. Stored 40 with LINEAR
    /// centre 40 and width 1 is white. The outside pixel stays opaque black.
    #[test]
    fn a_pixel_outside_the_centre_extent_is_background() {
        let sidecar = letterboxed_one();
        let frame = match draw_samples(&sidecar, &[Stored(40.0)]) {
            Ok(frame) => frame,
            Err(reason) => unreachable!("letterboxed fit is supported, refused as {reason}"),
        };
        assert_eq!(first_channel(&frame, 0), Some(0));
        assert_eq!(first_channel(&frame, 2), Some(255));
        assert_eq!(first_channel(&frame, 5), Some(255));
        assert_eq!(first_channel(&frame, 7), Some(0));
        assert_eq!(frame.get(3).copied(), Some(255));
    }

    /// PS3.3 C.7.6.2.1.1. Row spacing 2 and column spacing 1 on a 2 by 2 grid.
    ///
    /// The canvas is 8 by 8. The physical height limits the fit, scale is 2,
    /// and each source texel is 2 by 4 canvas pixels. The centre of texel
    /// (0, 0) is canvas (3, 2). The centre of texel (1, 1) is canvas (5, 6).
    /// LINEAR centre 40 and width 1 paints stored 0 black and stored 40 white.
    /// Pixel 0 is left of the centred extent.
    #[test]
    fn nonsquare_spacing_samples_each_texel_at_its_centre() {
        let sidecar = nonsquare_pair();
        let frame = match draw_samples(&sidecar, &SAMPLES) {
            Ok(frame) => frame,
            Err(reason) => unreachable!("nonsquare fit is supported, refused as {reason}"),
        };
        assert_eq!(grey_at(&frame, 0, 0, 8), Some(0));
        assert_eq!(grey_at(&frame, 3, 0, 8), Some(0));
        assert_eq!(grey_at(&frame, 3, 2, 8), Some(0));
        assert_eq!(grey_at(&frame, 5, 2, 8), Some(255));
        assert_eq!(grey_at(&frame, 3, 6, 8), Some(0));
        assert_eq!(grey_at(&frame, 5, 6, 8), Some(255));
        assert_eq!(grey_at(&frame, 5, 7, 8), Some(255));
    }

    /// PS3.3 C.11.2.1.3.1 at the window a file width below one recovers.
    ///
    /// Width 0.5 at centre 40 is presented as width 1.5 at centre 40. Stored
    /// 151, 158, 160 and 170 with slope 0.25 are modality 37.75, 39.5, 40 and
    /// 42.5. The recovered curve is 0.631, 53.195, 127.5 and above 254.5.
    /// Nearest codes are 1, 53, 128 and 255. The file-width curve at modality
    /// 39.5 is 4.586 and its nearest code is 5.
    #[test]
    fn sigmoid_width_below_one_follows_the_recovered_window() {
        let low = filled_one(40.0, 0.5, "SIGMOID", 0.25, 0.0);
        let shoulder = filled_one(40.0, 0.5, "SIGMOID", 0.25, 0.0);
        let mid = filled_one(40.0, 0.5, "SIGMOID", 0.25, 0.0);
        let high = filled_one(40.0, 0.5, "SIGMOID", 0.25, 0.0);
        let low = match draw_samples(&low, &[Stored(151.0)]) {
            Ok(frame) => frame,
            Err(reason) => unreachable!("sigmoid shoulder is supported, refused as {reason}"),
        };
        let shoulder = match draw_samples(&shoulder, &[Stored(158.0)]) {
            Ok(frame) => frame,
            Err(reason) => unreachable!("sigmoid sample is supported, refused as {reason}"),
        };
        let mid = match draw_samples(&mid, &[Stored(160.0)]) {
            Ok(frame) => frame,
            Err(reason) => unreachable!("sigmoid centre is supported, refused as {reason}"),
        };
        let high = match draw_samples(&high, &[Stored(170.0)]) {
            Ok(frame) => frame,
            Err(reason) => unreachable!("sigmoid shoulder is supported, refused as {reason}"),
        };
        assert_eq!(first_channel(&low, 0), Some(1));
        assert_eq!(first_channel(&shoulder, 0), Some(53));
        assert_eq!(first_channel(&mid, 0), Some(128));
        assert_eq!(first_channel(&high, 0), Some(255));
        assert_ne!(low, mid);
        assert_ne!(mid, high);
    }

    /// PS3.3 C.7.6.3.3 puts the meaningful bits under `HighBit`.
    ///
    /// High bit 15 with 12 bits stored is the legacy left-aligned header.
    /// The reference render does not shift it. The producer accepts the
    /// header and unpacks the low 12 bits. A high bit past the container
    /// is still refused. High bit 11 is the conforming header.
    #[test]
    fn a_left_aligned_high_bit_is_unshifted_like_the_reference() {
        assert!(pixel_facts(&layout_header(15)).is_ok());
        assert!(pixel_facts(&layout_header(11)).is_ok());
        match pixel_facts(&layout_header(16)) {
            Err(stop) => assert_eq!(stop.reason, "pixel-layout-refused"),
            Ok(_) => unreachable!("high bit 16 does not fit in 16 allocated bits"),
        }
    }

    struct Pose {
        columns: u32,
        rows: u32,
        canvas_w: u32,
        canvas_h: u32,
        column_mm: f64,
        row_mm: f64,
        parallel: f64,
        horizontal: f64,
        vertical: f64,
        focal: [f64; 3],
        center: f64,
        width: f64,
        function: &'static str,
        slope: f64,
        intercept: f64,
    }

    impl Pose {
        fn sidecar(&self) -> Sidecar {
            let focal = self.focal;
            Sidecar {
                id: "synthetic-residual".to_owned(),
                kind: ViewKind::Stack,
                json: json!({
                    "frame": {
                        "width": self.canvas_w,
                        "height": self.canvas_h,
                        "sha256": "synthetic"
                    },
                    "attributes": {
                        "rows": self.rows,
                        "columns": self.columns,
                        "pixelSpacing": [self.row_mm, self.column_mm],
                        "photometricInterpretation": "MONOCHROME2",
                        "imageOrientationPatient": [1.0, 0.0, 0.0, 0.0, 1.0, 0.0]
                    },
                    "image": {
                        "rows": self.rows,
                        "columns": self.columns,
                        "slope": self.slope,
                        "intercept": self.intercept
                    },
                    "voi": {
                        "source": "file",
                        "windowCenter": self.center,
                        "windowWidth": self.width,
                        "voiLutFunction": self.function
                    },
                    "cornerstoneMetadata": {
                        "imagePlaneModule": {
                            "imagePositionPatient": [0.0, 0.0, 0.0],
                            "rowCosines": [1.0, 0.0, 0.0],
                            "columnCosines": [0.0, 1.0, 0.0]
                        }
                    },
                    "camera": {
                        // Negative-normal gaze. These fixtures derive the
                        // unflipped column order. The signed-basis test owns
                        // the positive side.
                        "position": [focal[0], focal[1], -10.0],
                        "focalPoint": focal,
                        "viewUp": [0.0, -1.0, 0.0],
                        "parallelScale": self.parallel
                    },
                    "canvasPixelsPerSourcePixel": {
                        "vertical": self.vertical,
                        "horizontal": self.horizontal
                    }
                }),
            }
        }
    }

    fn filled_one(
        center: f64,
        width: f64,
        function: &'static str,
        slope: f64,
        intercept: f64,
    ) -> Sidecar {
        Pose {
            columns: 1,
            rows: 1,
            canvas_w: 2,
            canvas_h: 2,
            column_mm: 1.0,
            row_mm: 1.0,
            parallel: 0.5,
            horizontal: 2.0,
            vertical: 2.0,
            focal: [0.0, 0.0, 0.0],
            center,
            width,
            function,
            slope,
            intercept,
        }
        .sidecar()
    }

    fn letterboxed_one() -> Sidecar {
        Pose {
            columns: 1,
            rows: 1,
            canvas_w: 8,
            canvas_h: 4,
            column_mm: 1.0,
            row_mm: 1.0,
            parallel: 0.5,
            horizontal: 4.0,
            vertical: 4.0,
            focal: [0.0, 0.0, 0.0],
            center: 40.0,
            width: 1.0,
            function: "LINEAR",
            slope: 1.0,
            intercept: 0.0,
        }
        .sidecar()
    }

    fn nonsquare_pair() -> Sidecar {
        Pose {
            columns: 2,
            rows: 2,
            canvas_w: 8,
            canvas_h: 8,
            column_mm: 1.0,
            row_mm: 2.0,
            parallel: 2.0,
            horizontal: 2.0,
            vertical: 4.0,
            focal: [0.5, 1.0, 0.0],
            center: 40.0,
            width: 1.0,
            function: "LINEAR",
            slope: 1.0,
            intercept: 0.0,
        }
        .sidecar()
    }

    fn layout_header(high_bit: u64) -> Sidecar {
        Sidecar {
            id: "synthetic-layout".to_owned(),
            kind: ViewKind::Stack,
            json: json!({
                "image": {"rows": 12, "columns": 20},
                "attributes": {
                    "bitsAllocated": 16,
                    "bitsStored": 12,
                    "highBit": high_bit,
                    "pixelRepresentation": 1
                }
            }),
        }
    }

    fn grey_at(frame: &[u8], x: usize, y: usize, width: usize) -> Option<u8> {
        frame
            .get((y.saturating_mul(width).saturating_add(x)).saturating_mul(4))
            .copied()
    }

    /// The source quad resolves like a 4-sample canvas edge.
    ///
    /// One sample inside the quad keeps a quarter of the edge texel and the
    /// rest is the clear colour. `synthetic__ct_series_uniform__slice_000`
    /// puts that edge at canvas x = 42.666, so column 42 is black from a
    /// point sample and 64 in the reference. Removing the blend leaves those
    /// bytes 64 apart. `synthetic__mr_nonsquare_spacing` puts the bottom edge
    /// at 409.6, which the sixteenth-pixel snap moves onto the sample at
    /// 409.625. Without that snap the row stays at two samples.
    #[test]
    fn uniform_slice_edge_matches_the_reference_within_one_lsb() {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
        let reference = root.join("tools/oracle/out");
        let corpus = root.join("corpus/data");
        if !reference.join("run.json").is_file() || !corpus.is_dir() {
            let summary = produce_applicable(&ProducePaths {
                reference_run: root.join("tools/oracle/missing-run"),
                corpus_root: corpus,
                reference_subset: root.join("tools/oracle/compare-out/reference-selected"),
                candidate_run: root.join("tools/oracle/compare-out/candidate"),
                dispositions: root.join("tools/oracle/compare-out/dispositions.json"),
            });
            assert_eq!(summary.resources, "reference-run-absent");
            return;
        }
        let run = match crate::sidecar::Run::load(&reference) {
            Ok(run) => run,
            Err(error) => unreachable!("the reference run loads, {error}"),
        };
        for id in [
            "synthetic__ct_series_uniform__slice_000",
            "synthetic__ct_unsigned_16",
            "synthetic__cr_monochrome1",
            "synthetic__ct_signed_12in16_right",
            "synthetic__ct_multiframe_perframe",
            "synthetic__mr_nonsquare_spacing",
        ] {
            let sidecar = match run.sidecars.get(id) {
                Some(sidecar) => sidecar,
                None => unreachable!("{id} is declared in the reference run"),
            };
            let (rgba, width, _) = match super::render_row(&corpus, sidecar, None) {
                Ok(frame) => frame,
                Err(stop) => unreachable!("{id} draws, {}", stop.reason),
            };
            let raw = match fs::read(reference.join(format!("{id}.raw"))) {
                Ok(bytes) => bytes,
                Err(error) => unreachable!("{id} reference raw is present, {error}"),
            };
            assert_frame_within_one(id, &rgba, &raw, width);
        }
    }

    /// JPEG extended 12-bit is the jpeg.js level, 15 counts above DCMTK.
    ///
    /// The codec fixture stays on DCMTK. This raster adds the reference
    /// decoder's residual before the LUT. A residual of 0 leaves the
    /// informative mean about nine display counts dark, which fails 25.1.
    #[test]
    fn jpeg_extended_12_level_matches_the_reference() {
        assert_eq!(
            super::jpeg_extended_12_level_residual().to_bits(),
            15.0_f32.to_bits()
        );
        let (rgba, raw) = rendered_pair("syntax__jpeg_extended_12");
        assert_class_one_bounds("syntax__jpeg_extended_12", &rgba, &raw);
    }

    /// The decimated DX frame is sampled through the sixteenth-pixel window.
    ///
    /// Column edges already sit on that grid. Row edges do not. An integer
    /// snap selects different source rows across the interior, and 25.1's
    /// full-frame bound rejects that frame.
    #[test]
    fn dx_window_snap_matches_the_reference_bound() {
        let (rgba, raw) = rendered_pair("real__dx_varepop__00000001");
        assert_class_one_bounds("real__dx_varepop__00000001", &rgba, &raw);
    }

    fn rendered_pair(id: &str) -> (Vec<u8>, Vec<u8>) {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
        let reference = root.join("tools/oracle/out");
        let corpus = root.join("corpus/data");
        let run = match crate::sidecar::Run::load(&reference) {
            Ok(run) => run,
            Err(error) => unreachable!("the reference run loads, {error}"),
        };
        let sidecar = match run.sidecars.get(id) {
            Some(sidecar) => sidecar,
            None => unreachable!("{id} is declared in the reference run"),
        };
        let (rgba, _, _) = match super::render_row(&corpus, sidecar, None) {
            Ok(frame) => frame,
            Err(stop) => unreachable!("{id} draws, {}", stop.reason),
        };
        let raw = match fs::read(reference.join(format!("{id}.raw"))) {
            Ok(bytes) => bytes,
            Err(error) => unreachable!("{id} reference raw is present, {error}"),
        };
        (rgba, raw)
    }

    fn assert_class_one_bounds(id: &str, rgba: &[u8], raw: &[u8]) {
        use crate::tolerance::{
            MONOCHROME_MAX_ABS_DIFF, MONOCHROME_SIGNED_MEAN_BIAS,
            MONOCHROME_WITHIN_ONE_LSB_FRACTION,
        };
        assert_eq!(rgba.len(), raw.len(), "{id}");
        let mut within_one = 0_u32;
        let mut over_max = 0_u32;
        let mut pixels = 0_u32;
        let mut informative = 0_u32;
        let mut signed = 0_i32;
        let mut worst = 0_u8;
        for (left, right) in rgba.chunks(4).zip(raw.chunks(4)) {
            let (Some(candidate), Some(reference)) = (left.first(), right.first()) else {
                continue;
            };
            pixels = pixels.saturating_add(1);
            let delta = candidate.abs_diff(*reference);
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
                signed = signed.saturating_add(i32::from(*candidate) - i32::from(*reference));
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

    fn assert_frame_within_one(id: &str, rgba: &[u8], raw: &[u8], width: u32) {
        assert_eq!(rgba.len(), raw.len(), "{id}");
        let mut worst = 0_u8;
        let mut over = 0_u32;
        let mut at = (0_u32, 0_u32);
        let columns = usize::try_from(width).unwrap_or(0);
        for (index, (left, right)) in rgba.iter().zip(raw.iter()).enumerate() {
            let delta = left.abs_diff(*right);
            if delta > 1 {
                over = over.saturating_add(1);
            }
            if delta > worst {
                worst = delta;
                if columns > 0 {
                    let pixel = index / 4;
                    let x = u32::try_from(pixel % columns).unwrap_or(0);
                    let y = u32::try_from(pixel / columns).unwrap_or(0);
                    at = (x, y);
                }
            }
        }
        assert_eq!(over, 0, "{id} worst {worst} at {at:?}");
    }
}
