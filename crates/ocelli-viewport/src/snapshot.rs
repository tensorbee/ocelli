//! Versioned JSON record of one viewport's camera, display area and presentation.
//!
//! HLD section 17.4's 96-byte readback is the inverse camera matrix plus five
//! scalars. It cannot rebuild pan, zoom, rotation, flip, spacing, the VOI
//! function, a selected LUT sequence, the colormap, or whether the presentation
//! shape was declared. This record stores those inputs. [`View::camera_matrix`]
//! stays the only matrix, and [`LutChain::new`] stays the only chain.
//!
//! DICOM PS3.3 C.11.2.1.1: a LUT descriptor's entry count of zero means 65,536.
//! PS3.3 C.11.6: presentation shape is `IDENTITY` or `INVERSE`, and an absent
//! shape is not the same state as an explicit one.

use alloc::string::{String, ToString};
use alloc::vec::Vec;
use core::fmt::Write as _;

use ocelli_pixel::{PhotometricInterpretation, PresentationLutEvidence, PresentationLutShape};
use serde::{Deserialize, Serialize};

use crate::commands::StackViewport;
use crate::presentation::{
    Colormap, EncodedLut, EncodedModality, EncodedPresentation, EncodedVoi, Presentation,
};
use crate::view::{EncodedView, View};

/// The record's magic. A reader refuses every other value.
pub const SNAPSHOT_MAGIC: &str = "OCS1";

/// The only version this crate reads. Every other version is refused.
pub const SNAPSHOT_VERSION: u16 = 1;

/// [`SnapshotError::NonFinite`].
pub const SNAPSHOT_ERROR_NON_FINITE: u32 = 1;
/// [`SnapshotError::Encode`].
pub const SNAPSHOT_ERROR_ENCODE: u32 = 2;
/// [`SnapshotError::Json`].
pub const SNAPSHOT_ERROR_JSON: u32 = 3;
/// [`SnapshotError::BadMagic`].
pub const SNAPSHOT_ERROR_BAD_MAGIC: u32 = 4;
/// [`SnapshotError::UnknownVersion`].
pub const SNAPSHOT_ERROR_UNKNOWN_VERSION: u32 = 5;
/// [`SnapshotError::BadEnum`].
pub const SNAPSHOT_ERROR_BAD_ENUM: u32 = 6;
/// [`SnapshotError::BadLut`].
pub const SNAPSHOT_ERROR_BAD_LUT: u32 = 7;
/// [`SnapshotError::ChainUnavailable`].
pub const SNAPSHOT_ERROR_CHAIN_UNAVAILABLE: u32 = 8;

/// Names and codes the TypeScript generator emits. Zero is success and is absent.
pub const SNAPSHOT_ERRORS: &[(&str, u32)] = &[
    ("NonFinite", SNAPSHOT_ERROR_NON_FINITE),
    ("Encode", SNAPSHOT_ERROR_ENCODE),
    ("Json", SNAPSHOT_ERROR_JSON),
    ("BadMagic", SNAPSHOT_ERROR_BAD_MAGIC),
    ("UnknownVersion", SNAPSHOT_ERROR_UNKNOWN_VERSION),
    ("BadEnum", SNAPSHOT_ERROR_BAD_ENUM),
    ("BadLut", SNAPSHOT_ERROR_BAD_LUT),
    ("ChainUnavailable", SNAPSHOT_ERROR_CHAIN_UNAVAILABLE),
];

/// Why a snapshot was not written, or why a restore changed nothing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SnapshotError {
    /// A camera, display-area or LUT value is NaN or infinite.
    NonFinite,
    /// `serde_json` refused to serialise the record.
    Encode,
    /// The input is not one JSON value, has a trailing byte, or names an unknown field.
    Json,
    /// `magic` is not [`SNAPSHOT_MAGIC`].
    BadMagic,
    /// `version` is not [`SNAPSHOT_VERSION`].
    UnknownVersion(u16),
    /// A kind, function, photometric, shape or colormap name is outside its domain.
    BadEnum,
    /// [`ocelli_pixel::LutDescriptor::new`] refused the descriptor.
    BadLut,
    /// [`ocelli_pixel::LutChain::new`] refused the rebuilt chain.
    ChainUnavailable,
}

impl SnapshotError {
    /// Stable nonzero code for the wasm boundary. Zero means success.
    #[must_use]
    pub const fn number(self) -> u32 {
        match self {
            Self::NonFinite => SNAPSHOT_ERROR_NON_FINITE,
            Self::Encode => SNAPSHOT_ERROR_ENCODE,
            Self::Json => SNAPSHOT_ERROR_JSON,
            Self::BadMagic => SNAPSHOT_ERROR_BAD_MAGIC,
            Self::UnknownVersion(_) => SNAPSHOT_ERROR_UNKNOWN_VERSION,
            Self::BadEnum => SNAPSHOT_ERROR_BAD_ENUM,
            Self::BadLut => SNAPSHOT_ERROR_BAD_LUT,
            Self::ChainUnavailable => SNAPSHOT_ERROR_CHAIN_UNAVAILABLE,
        }
    }
}

/// A decoded record, not yet written onto a viewport.
pub(crate) struct DecodedSnapshot {
    pub view: View,
    pub presentation: Presentation,
    pub columns: u32,
    pub rows: u32,
    pub css_width: f64,
    pub css_height: f64,
}

/// UTF-8 JSON for `viewport`. Scroll, flags and generation are not included.
pub(crate) fn encode(viewport: &StackViewport) -> Result<Vec<u8>, SnapshotError> {
    let view = viewport.view().encode()?;
    let presentation = viewport.presentation().encode()?;
    finite64(viewport.css_width())?;
    finite64(viewport.css_height())?;
    let record = Record {
        magic: SNAPSHOT_MAGIC.to_string(),
        version: SNAPSHOT_VERSION,
        viewport_id: viewport.id(),
        display_area: display_from(viewport, view),
        camera: camera_from(view),
        presentation: presentation_from(presentation),
    };
    serde_json::to_vec(&record).map_err(|_| SnapshotError::Encode)
}

/// Decode `bytes` into locals. The caller assigns them after this returns.
pub(crate) fn decode(bytes: &[u8]) -> Result<DecodedSnapshot, SnapshotError> {
    let record: Record = serde_json::from_slice(bytes).map_err(map_decode)?;
    if record.magic != SNAPSHOT_MAGIC {
        return Err(SnapshotError::BadMagic);
    }
    if record.version != SNAPSHOT_VERSION {
        return Err(SnapshotError::UnknownVersion(record.version));
    }
    finite64(record.display_area.css_width)?;
    finite64(record.display_area.css_height)?;
    let view = View::decode(view_from(&record))?;
    let presentation = Presentation::decode(presentation_from_dto(record.presentation))?;
    Ok(DecodedSnapshot {
        view,
        presentation,
        columns: record.display_area.columns,
        rows: record.display_area.rows,
        css_width: record.display_area.css_width,
        css_height: record.display_area.css_height,
    })
}

fn finite64(value: f64) -> Result<(), SnapshotError> {
    if value.is_finite() {
        Ok(())
    } else {
        Err(SnapshotError::NonFinite)
    }
}

fn map_decode(error: serde_json::Error) -> SnapshotError {
    let mut text = String::new();
    let _ = write!(text, "{error}");
    if text.contains("number out of range") {
        SnapshotError::NonFinite
    } else if text.contains("unknown variant") {
        SnapshotError::BadEnum
    } else {
        SnapshotError::Json
    }
}

fn display_from(viewport: &StackViewport, view: EncodedView) -> DisplayDto {
    DisplayDto {
        css_width: viewport.css_width(),
        css_height: viewport.css_height(),
        columns: viewport.columns(),
        rows: viewport.rows(),
        column_millimetres: view.column_mm,
        row_millimetres: view.row_mm,
        shift_column: view.shift_col,
        shift_row: view.shift_row,
        fitted: view.fitted,
        anchor_x: view.anchor_x,
        anchor_y: view.anchor_y,
        fit_scale: view.fit_scale,
    }
}

fn camera_from(view: EncodedView) -> CameraDto {
    CameraDto {
        pan_x: view.pan_x,
        pan_y: view.pan_y,
        zoom: view.zoom,
        radians: view.radians,
        flip_horizontal: view.flip_horizontal,
        flip_vertical: view.flip_vertical,
    }
}

fn view_from(record: &Record) -> EncodedView {
    EncodedView {
        pan_x: record.camera.pan_x,
        pan_y: record.camera.pan_y,
        zoom: record.camera.zoom,
        radians: record.camera.radians,
        flip_horizontal: record.camera.flip_horizontal,
        flip_vertical: record.camera.flip_vertical,
        column_mm: record.display_area.column_millimetres,
        row_mm: record.display_area.row_millimetres,
        shift_col: record.display_area.shift_column,
        shift_row: record.display_area.shift_row,
        fitted: record.display_area.fitted,
        anchor_x: record.display_area.anchor_x,
        anchor_y: record.display_area.anchor_y,
        fit_scale: record.display_area.fit_scale,
    }
}

fn presentation_from(encoded: EncodedPresentation) -> PresentationDto {
    PresentationDto {
        modality: modality_from(encoded.modality),
        voi: voi_from(encoded.voi),
        photometric: photometric_from(encoded.photometric),
        evidence: evidence_from(encoded.evidence),
        colormap: colormap_from(encoded.colormap),
    }
}

fn presentation_from_dto(dto: PresentationDto) -> EncodedPresentation {
    EncodedPresentation {
        modality: modality_dto(dto.modality),
        voi: voi_dto(dto.voi),
        photometric: photometric_dto(dto.photometric),
        evidence: evidence_dto(dto.evidence),
        colormap: colormap_dto(dto.colormap),
    }
}

fn modality_from(modality: EncodedModality) -> ModalityDto {
    match modality {
        EncodedModality::Rescale { slope, intercept } => ModalityDto::Rescale { slope, intercept },
        EncodedModality::Lut(lut) => ModalityDto::Lut {
            entry_count: lut.declared_entry_count,
            first_mapped_input: lut.first_mapped_input,
            bits_per_entry: lut.bits_per_entry,
            values: lut.values,
        },
    }
}

fn modality_dto(modality: ModalityDto) -> EncodedModality {
    match modality {
        ModalityDto::Rescale { slope, intercept } => EncodedModality::Rescale { slope, intercept },
        ModalityDto::Lut {
            entry_count,
            first_mapped_input,
            bits_per_entry,
            values,
        } => EncodedModality::Lut(EncodedLut {
            declared_entry_count: entry_count,
            first_mapped_input,
            bits_per_entry,
            values,
        }),
    }
}

fn voi_from(voi: EncodedVoi) -> VoiDto {
    match voi {
        EncodedVoi::Window {
            center,
            width,
            function,
            ymin,
            ymax,
        } => VoiDto::Window {
            center,
            width,
            function: function_from(function),
            ymin,
            ymax,
        },
        EncodedVoi::Lut(lut) => VoiDto::Lut {
            entry_count: lut.declared_entry_count,
            first_mapped_input: lut.first_mapped_input,
            bits_per_entry: lut.bits_per_entry,
            values: lut.values,
        },
    }
}

fn voi_dto(voi: VoiDto) -> EncodedVoi {
    match voi {
        VoiDto::Window {
            center,
            width,
            function,
            ymin,
            ymax,
        } => EncodedVoi::Window {
            center,
            width,
            function: function_dto(function),
            ymin,
            ymax,
        },
        VoiDto::Lut {
            entry_count,
            first_mapped_input,
            bits_per_entry,
            values,
        } => EncodedVoi::Lut(EncodedLut {
            declared_entry_count: entry_count,
            first_mapped_input,
            bits_per_entry,
            values,
        }),
    }
}

fn function_from(function: ocelli_pixel::VoiFunction) -> FunctionDto {
    match function {
        ocelli_pixel::VoiFunction::Linear => FunctionDto::Linear,
        ocelli_pixel::VoiFunction::LinearExact => FunctionDto::LinearExact,
        ocelli_pixel::VoiFunction::Sigmoid => FunctionDto::Sigmoid,
    }
}

fn function_dto(function: FunctionDto) -> ocelli_pixel::VoiFunction {
    match function {
        FunctionDto::Linear => ocelli_pixel::VoiFunction::Linear,
        FunctionDto::LinearExact => ocelli_pixel::VoiFunction::LinearExact,
        FunctionDto::Sigmoid => ocelli_pixel::VoiFunction::Sigmoid,
    }
}

fn photometric_from(value: PhotometricInterpretation) -> PhotometricDto {
    match value {
        PhotometricInterpretation::Monochrome1 => PhotometricDto::Monochrome1,
        PhotometricInterpretation::Monochrome2 => PhotometricDto::Monochrome2,
        PhotometricInterpretation::PaletteColor => PhotometricDto::PaletteColor,
        PhotometricInterpretation::Rgb => PhotometricDto::Rgb,
        PhotometricInterpretation::YbrFull => PhotometricDto::YbrFull,
        PhotometricInterpretation::YbrFull422 => PhotometricDto::YbrFull422,
        PhotometricInterpretation::YbrPartial422 => PhotometricDto::YbrPartial422,
        PhotometricInterpretation::YbrIct => PhotometricDto::YbrIct,
        PhotometricInterpretation::YbrRct => PhotometricDto::YbrRct,
    }
}

fn photometric_dto(value: PhotometricDto) -> PhotometricInterpretation {
    match value {
        PhotometricDto::Monochrome1 => PhotometricInterpretation::Monochrome1,
        PhotometricDto::Monochrome2 => PhotometricInterpretation::Monochrome2,
        PhotometricDto::PaletteColor => PhotometricInterpretation::PaletteColor,
        PhotometricDto::Rgb => PhotometricInterpretation::Rgb,
        PhotometricDto::YbrFull => PhotometricInterpretation::YbrFull,
        PhotometricDto::YbrFull422 => PhotometricInterpretation::YbrFull422,
        PhotometricDto::YbrPartial422 => PhotometricInterpretation::YbrPartial422,
        PhotometricDto::YbrIct => PhotometricInterpretation::YbrIct,
        PhotometricDto::YbrRct => PhotometricInterpretation::YbrRct,
    }
}

fn evidence_from(evidence: PresentationLutEvidence) -> EvidenceDto {
    match evidence {
        PresentationLutEvidence::Absent => EvidenceDto::Absent,
        PresentationLutEvidence::Shape(PresentationLutShape::Identity) => EvidenceDto::Shape {
            shape: ShapeDto::Identity,
        },
        PresentationLutEvidence::Shape(PresentationLutShape::Inverse) => EvidenceDto::Shape {
            shape: ShapeDto::Inverse,
        },
        PresentationLutEvidence::Sequence => EvidenceDto::Sequence,
    }
}

fn evidence_dto(evidence: EvidenceDto) -> PresentationLutEvidence {
    match evidence {
        EvidenceDto::Absent => PresentationLutEvidence::Absent,
        EvidenceDto::Shape {
            shape: ShapeDto::Identity,
        } => PresentationLutEvidence::Shape(PresentationLutShape::Identity),
        EvidenceDto::Shape {
            shape: ShapeDto::Inverse,
        } => PresentationLutEvidence::Shape(PresentationLutShape::Inverse),
        EvidenceDto::Sequence => PresentationLutEvidence::Sequence,
    }
}

fn colormap_from(colormap: Colormap) -> ColormapDto {
    match colormap {
        Colormap::Grey => ColormapDto::Grey,
        Colormap::Stops => ColormapDto::Stops,
    }
}

fn colormap_dto(colormap: ColormapDto) -> Colormap {
    match colormap {
        ColormapDto::Grey => Colormap::Grey,
        ColormapDto::Stops => Colormap::Stops,
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Record {
    magic: String,
    version: u16,
    viewport_id: u16,
    display_area: DisplayDto,
    camera: CameraDto,
    presentation: PresentationDto,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct DisplayDto {
    css_width: f64,
    css_height: f64,
    columns: u32,
    rows: u32,
    column_millimetres: f64,
    row_millimetres: f64,
    shift_column: f64,
    shift_row: f64,
    fitted: bool,
    anchor_x: f64,
    anchor_y: f64,
    fit_scale: f64,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct CameraDto {
    pan_x: f64,
    pan_y: f64,
    zoom: f64,
    radians: f64,
    flip_horizontal: bool,
    flip_vertical: bool,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct PresentationDto {
    modality: ModalityDto,
    voi: VoiDto,
    photometric: PhotometricDto,
    evidence: EvidenceDto,
    colormap: ColormapDto,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all_fields = "camelCase", deny_unknown_fields)]
enum ModalityDto {
    #[serde(rename = "rescale")]
    Rescale { slope: f32, intercept: f32 },
    #[serde(rename = "lut")]
    Lut {
        entry_count: u16,
        first_mapped_input: i32,
        bits_per_entry: u8,
        values: Vec<f32>,
    },
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all_fields = "camelCase", deny_unknown_fields)]
enum VoiDto {
    #[serde(rename = "window")]
    Window {
        center: f32,
        width: f32,
        function: FunctionDto,
        ymin: f32,
        ymax: f32,
    },
    #[serde(rename = "lut")]
    Lut {
        entry_count: u16,
        first_mapped_input: i32,
        bits_per_entry: u8,
        values: Vec<f32>,
    },
}

#[derive(Debug, Serialize, Deserialize)]
enum FunctionDto {
    #[serde(rename = "linear")]
    Linear,
    #[serde(rename = "linear_exact")]
    LinearExact,
    #[serde(rename = "sigmoid")]
    Sigmoid,
}

#[derive(Debug, Serialize, Deserialize)]
enum PhotometricDto {
    #[serde(rename = "monochrome1")]
    Monochrome1,
    #[serde(rename = "monochrome2")]
    Monochrome2,
    #[serde(rename = "palette_color")]
    PaletteColor,
    #[serde(rename = "rgb")]
    Rgb,
    #[serde(rename = "ybr_full")]
    YbrFull,
    #[serde(rename = "ybr_full_422")]
    YbrFull422,
    #[serde(rename = "ybr_partial_422")]
    YbrPartial422,
    #[serde(rename = "ybr_ict")]
    YbrIct,
    #[serde(rename = "ybr_rct")]
    YbrRct,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "kind", deny_unknown_fields)]
enum EvidenceDto {
    #[serde(rename = "absent")]
    Absent,
    #[serde(rename = "shape")]
    Shape { shape: ShapeDto },
    #[serde(rename = "sequence")]
    Sequence,
}

#[derive(Debug, Serialize, Deserialize)]
enum ShapeDto {
    #[serde(rename = "identity")]
    Identity,
    #[serde(rename = "inverse")]
    Inverse,
}

#[derive(Debug, Serialize, Deserialize)]
enum ColormapDto {
    #[serde(rename = "grey")]
    Grey,
    #[serde(rename = "stops")]
    Stops,
}

#[cfg(test)]
mod tests {
    use super::{SNAPSHOT_ERRORS, SnapshotError};

    #[test]
    fn error_numbers_are_nonzero_and_unique() {
        let samples = [
            SnapshotError::NonFinite,
            SnapshotError::Encode,
            SnapshotError::Json,
            SnapshotError::BadMagic,
            SnapshotError::UnknownVersion(2),
            SnapshotError::UnknownVersion(9),
            SnapshotError::BadEnum,
            SnapshotError::BadLut,
            SnapshotError::ChainUnavailable,
        ];
        for error in samples {
            let code = error.number();
            assert!(code != 0);
            assert!(SNAPSHOT_ERRORS.iter().any(|(_, listed)| *listed == code));
        }
        assert_eq!(
            SnapshotError::UnknownVersion(2).number(),
            SnapshotError::UnknownVersion(9).number()
        );
        let mut codes: Vec<u32> = SNAPSHOT_ERRORS.iter().map(|(_, code)| *code).collect();
        codes.sort_unstable();
        let before = codes.len();
        codes.dedup();
        assert_eq!(codes.len(), before);
        assert_eq!(codes.len(), SNAPSHOT_ERRORS.len());
    }
}
