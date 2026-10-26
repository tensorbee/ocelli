//! Synthetic settled CPU frames handed to [`crate::attribution::compare_view`].
//!
//! These controls are not corpus rows. [`compare_settled_linear_grey`] is the
//! positive control. A stored `0` frame against the same hand grey is the
//! wrong candidate. The applicable corpus producer is [`crate::produce`].

use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

use ocelli_core::Stored;
use ocelli_pixel::{
    LutChain, ModalityTransform, PhotometricInterpretation, PresentationLutEvidence, VoiFunction,
    VoiTransform,
};
use ocelli_render::{CpuRaster, RasterGrid, RasterQuality};
use ocelli_viewport::presentation::{Colormap, Presentation};
use ocelli_viewport::view::View;
use serde_json::{Value, json};

use crate::attribution::{CompareError, Context, Register, compare_view};
use crate::frame::{Frame, FrameError};
use crate::report::ViewRecord;
use crate::sidecar::{DeclaredView, Run, Sidecar, ViewKind};

const VIEW: &str = "synthetic-cpu-linear__00000001";
const SIDE: u32 = 16;

/// Why the bounded producer did not reach a comparison.
#[derive(Debug)]
pub enum CandidateError {
    /// The CPU raster refused the synthetic grid.
    Raster(&'static str),
    /// The frame bytes do not match the declared extent.
    Frame(FrameError),
    /// [`compare_view`] refused the pair.
    Compare(CompareError),
}

impl From<FrameError> for CandidateError {
    fn from(error: FrameError) -> Self {
        Self::Frame(error)
    }
}

impl From<CompareError> for CandidateError {
    fn from(error: CompareError) -> Self {
        Self::Compare(error)
    }
}

/// Raster stored `40` through LINEAR centre `40` and width `400`, then compare.
///
/// The reference frame is the hand grey byte `128` on every pixel, the
/// nearest code to PS3.3 C.11.2.1.2's 127.81955 at centre 40 and width 400.
/// The candidate frame is the raster's settled RGBA. Both sidecars are
/// `mono16`.
///
/// # Errors
///
/// [`CandidateError`] when the raster, the frame length, or the comparator
/// refuses the pair.
pub fn compare_settled_linear_grey() -> Result<ViewRecord, CandidateError> {
    let candidate = raster_rgba(40.0)?;
    // PS3.3 C.11.2.1.2 at centre 40 and width 400 is 127.81955, and the
    // pinned reference quantises it to the nearest code, 128. The control
    // stays discriminating because stored 0 is 102, not 128.
    let reference_grey = [128_u8; 256];
    compare_rgba(&candidate, &reference_grey)
}

/// Compare one raster frame with a hand grey image of the same extent.
///
/// # Errors
///
/// [`CandidateError`] from the frame constructor or from [`compare_view`].
pub fn compare_rgba(
    candidate_rgba: &[u8],
    reference_grey: &[u8],
) -> Result<ViewRecord, CandidateError> {
    let reference = stack_run();
    let candidate = stack_run();
    let register = Register::default();
    let empty = BTreeSet::new();
    let context = Context {
        reference: &reference,
        candidate: &candidate,
        register: &register,
        low_information: &empty,
        downsampled: &empty,
    };
    let reference_frame = Frame::from_monochrome(SIDE, SIDE, reference_grey)?;
    let candidate_frame = Frame::new(SIDE, SIDE, candidate_rgba.to_vec())?;
    Ok(compare_view(
        &context,
        VIEW,
        &reference_frame,
        &candidate_frame,
    )?)
}

/// Settled RGBA for one stored value on a 16 by 16 identity view.
///
/// # Errors
///
/// [`CandidateError::Raster`] when the canvas or the draw is refused.
pub fn raster_rgba(stored: f32) -> Result<Vec<u8>, CandidateError> {
    let mut raster = CpuRaster::open(f64::from(SIDE), f64::from(SIDE), 1.0)
        .map_err(|_| CandidateError::Raster("the synthetic canvas was refused"))?;
    let view = View::new();
    let presentation = linear_presentation()?;
    let samples = vec![Stored(stored); 256];
    let target = raster.device();
    let pixels = usize::try_from(target[0])
        .ok()
        .and_then(|width| {
            usize::try_from(target[1])
                .ok()
                .map(|height| width.saturating_mul(height))
        })
        .ok_or(CandidateError::Raster(
            "the device extent is not a pixel count",
        ))?;
    let mut out = vec![0_u8; pixels.saturating_mul(4)];
    let drawn = raster
        .raster_into(
            &view,
            &presentation,
            &samples,
            RasterGrid {
                source: [SIDE, SIDE],
                target,
            },
            RasterQuality::Settled {
                generation: raster.generation(),
            },
            &mut out,
        )
        .map_err(|_| CandidateError::Raster("the settled draw was refused"))?;
    if !drawn.wrote || drawn.extent != [SIDE, SIDE] {
        return Err(CandidateError::Raster(
            "the settled draw wrote the wrong extent",
        ));
    }
    Ok(out)
}

fn linear_presentation() -> Result<Presentation, CandidateError> {
    let modality = ModalityTransform::new(None, Some(1.0), Some(0.0))
        .map_err(|_| CandidateError::Raster("modality slope is outside the domain"))?;
    let voi = VoiTransform::new(None, &[40.0], &[400.0], 0, VoiFunction::Linear, 0.0, 255.0)
        .map_err(|_| CandidateError::Raster("the LINEAR window is outside the domain"))?;
    let chain = LutChain::new(
        modality,
        voi,
        PhotometricInterpretation::Monochrome2,
        PresentationLutEvidence::Absent,
    )
    .map_err(|_| CandidateError::Raster("the monochrome chain is outside the domain"))?;
    Ok(Presentation::new(
        chain,
        PhotometricInterpretation::Monochrome2,
        PresentationLutEvidence::Absent,
        Colormap::Grey,
    ))
}

fn stack_run() -> Run {
    let sidecar = Sidecar {
        id: VIEW.to_owned(),
        kind: ViewKind::Stack,
        json: json!({
            "kind": "stack",
            "camera": {
                "position": [0.0, 0.0, 100.0],
                "focalPoint": [0.0, 0.0, 0.0],
                "viewUp": [0.0, 1.0, 0.0],
                "parallelScale": 8.0
            },
            "image": { "rows": SIDE, "columns": SIDE },
            "canvasPixelsPerSourcePixel": { "vertical": 1.0, "horizontal": 1.0 },
            "attributes": { "photometricInterpretation": "MONOCHROME2" }
        }),
    };
    let mut views = BTreeMap::new();
    views.insert(
        VIEW.to_owned(),
        DeclaredView {
            id: VIEW.to_owned(),
            kind: ViewKind::Stack,
            path: Some("synthetic/cpu-linear.dcm".to_owned()),
            subject: None,
        },
    );
    let mut sidecars = BTreeMap::new();
    sidecars.insert(VIEW.to_owned(), sidecar);
    let mut categories = BTreeMap::new();
    categories.insert(VIEW.to_owned(), vec!["mono16".to_owned()]);
    Run {
        directory: PathBuf::from("built-in-memory"),
        json: Value::Null,
        views,
        sidecars,
        raw_present: BTreeSet::new(),
        by_path: BTreeMap::new(),
        categories,
    }
}

#[cfg(test)]
mod tests {
    use crate::report::Outcome;

    use super::{compare_rgba, compare_settled_linear_grey, raster_rgba};

    #[test]
    fn the_settled_linear_frame_is_compared_not_assumed() {
        let grey = match raster_rgba(40.0) {
            Ok(value) => value,
            Err(_) => unreachable!("stored 40 is inside the LINEAR window"),
        };
        assert_eq!(grey.first().copied(), Some(128));
        let other = match raster_rgba(0.0) {
            Ok(value) => value,
            Err(_) => unreachable!("stored 0 is inside the same window"),
        };
        assert_ne!(other.first().copied(), Some(128));
        let matched = match compare_settled_linear_grey() {
            Ok(value) => value,
            Err(_) => unreachable!("the grey frame and the hand byte agree"),
        };
        assert_eq!(matched.outcome, Outcome::Pass);
        let diverged = match compare_rgba(&other, &[128_u8; 256]) {
            Ok(value) => value,
            Err(_) => unreachable!("a different frame still reaches compare_view"),
        };
        assert_eq!(diverged.outcome, Outcome::Fail);
    }

    #[test]
    fn a_stored_zero_frame_is_the_wrong_candidate() {
        let wrong = match raster_rgba(0.0) {
            Ok(value) => value,
            Err(_) => unreachable!("stored 0 is inside the LINEAR window"),
        };
        assert_ne!(wrong.first().copied(), Some(128));
        let record = match compare_rgba(&wrong, &[128_u8; 256]) {
            Ok(value) => value,
            Err(_) => unreachable!("the wrong frame still reaches compare_view"),
        };
        assert_eq!(record.outcome, Outcome::Fail);
    }
}
