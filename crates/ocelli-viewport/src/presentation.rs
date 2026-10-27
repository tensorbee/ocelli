//! Window presets, invert and the colormap.
//!
//! HLD section 18 says the LUT chain exists once, in `ocelli-pixel`. A preset
//! calls [`VoiTransform::new`] and does not add a formula. HLD section 18.3's
//! soft-tissue window is centre 40 and width 400. Lung and bone are the
//! centre and width pairs named by the F-046 plan.

use alloc::vec::Vec;

use ocelli_pixel::{
    LutChain, LutDescriptor, ModalityTransform, PhotometricInterpretation, PixelError,
    PresentationLutEvidence, PresentationLutShape, VoiFunction, VoiTransform,
};

/// The three named windows. Each one is `LINEAR`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WindowPreset {
    /// Centre 40, width 400. HLD section 18.3.
    SoftTissue,
    /// Centre -600, width 1500. A plan number, not a clinical standard.
    Lung,
    /// Centre 300, width 1500. A plan number, not a clinical standard.
    Bone,
}

impl WindowPreset {
    const fn center(self) -> f32 {
        match self {
            Self::SoftTissue => 40.0,
            Self::Lung => -600.0,
            Self::Bone => 300.0,
        }
    }

    const fn width(self) -> f32 {
        match self {
            Self::SoftTissue => 400.0,
            Self::Lung => 1500.0,
            Self::Bone => 1500.0,
        }
    }
}

/// `Grey` or `Stops`. The render crate maps these onto its preset tables.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Colormap {
    /// Display value `d` in `0..255` maps to RGB `(d, d, d)`.
    Grey,
    /// Texels at 0, 128 and 255. Every other index is black.
    Stops,
}

/// Why a presentation change was refused. The stored presentation is unchanged.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PresentationError {
    /// A preset would replace a modality or VOI LUT sequence, or a manual
    /// window would replace a VOI LUT sequence.
    Sequence,
    /// The width is outside [`VoiTransform::new`]'s domain for this function.
    InvalidWindow,
    /// The chain could not be rebuilt. This is not a window-domain refusal.
    Unavailable,
}

/// The current LUT chain and colormap for one viewport.
///
/// A preset or [`Self::set_window`] rebuilds stage 2 and keeps the modality
/// transform, the presentation evidence and the output range. [`Self::set_invert`]
/// stores an explicit shape and rebuilds the chain. It does not combine that
/// shape with `MONOCHROME1`. A window change does not change the colormap.
#[derive(Debug, Clone, PartialEq)]
pub struct Presentation {
    chain: LutChain,
    photometric: PhotometricInterpretation,
    evidence: PresentationLutEvidence,
    colormap: Colormap,
}

impl Presentation {
    /// Store a chain that has already been built.
    ///
    /// `photometric` and `evidence` are kept so a later change can rebuild
    /// through [`LutChain::new`]. They are the values that built `chain`.
    #[must_use]
    pub const fn new(
        chain: LutChain,
        photometric: PhotometricInterpretation,
        evidence: PresentationLutEvidence,
        colormap: Colormap,
    ) -> Self {
        Self {
            chain,
            photometric,
            evidence,
            colormap,
        }
    }

    /// Replace stage 2 with this preset's centre, width and `LINEAR`.
    ///
    /// # Errors
    ///
    /// [`PresentationError::Sequence`] when stage 1 is a modality LUT sequence
    /// or stage 2 is a VOI LUT sequence. The stored presentation is left in
    /// place. [`PresentationError::InvalidWindow`] when [`VoiTransform::new`]
    /// refuses the preset width. [`PresentationError::Unavailable`] when the
    /// rebuilt chain is refused for any other reason.
    pub fn apply_preset(&mut self, preset: WindowPreset) -> Result<(), PresentationError> {
        if self.chain.modality().rescale().is_none() {
            return Err(PresentationError::Sequence);
        }
        self.replace_window(preset.center(), preset.width(), VoiFunction::Linear)
    }

    /// Replace stage 2 with a caller-supplied window.
    ///
    /// The output range stays the range the chain already has. The colormap
    /// stays where [`Self::set_colormap`] left it.
    ///
    /// # Errors
    ///
    /// [`PresentationError::Sequence`] when stage 2 is a VOI LUT sequence.
    /// A modality LUT sequence is kept and is not a refusal here.
    /// [`PresentationError::InvalidWindow`] when the width is outside
    /// [`VoiTransform::new`]'s domain. [`PresentationError::Unavailable`] when
    /// the rebuilt chain is refused for any other reason. Any refusal leaves
    /// the chain, the photometric interpretation, the presentation evidence
    /// and the colormap in place.
    pub fn set_window(
        &mut self,
        center: f32,
        width: f32,
        function: VoiFunction,
    ) -> Result<(), PresentationError> {
        self.replace_window(center, width, function)
    }

    /// Set an explicit presentation shape and rebuild the chain.
    ///
    /// `true` stores [`PresentationLutShape::Inverse`]. `false` stores
    /// [`PresentationLutShape::Identity`]. The photometric interpretation is
    /// not consulted again, so this does not combine with `MONOCHROME1`.
    ///
    /// # Errors
    ///
    /// [`PresentationError::Unavailable`] when the rebuilt chain is refused.
    /// The stored chain and its evidence stay as they were.
    pub fn set_invert(&mut self, invert: bool) -> Result<(), PresentationError> {
        let shape = if invert {
            PresentationLutShape::Inverse
        } else {
            PresentationLutShape::Identity
        };
        let evidence = PresentationLutEvidence::Shape(shape);
        let voi = self.chain.voi().clone();
        self.rebuild(voi, evidence)
    }

    /// Store the colormap. This does not rebuild the chain.
    pub fn set_colormap(&mut self, colormap: Colormap) {
        self.colormap = colormap;
    }

    /// The chain a renderer reads. This crate does not call `set_voi`.
    #[must_use]
    pub const fn chain(&self) -> &LutChain {
        &self.chain
    }

    /// The colormap a renderer maps onto a preset table.
    #[must_use]
    pub const fn colormap(&self) -> Colormap {
        self.colormap
    }

    /// Photometric interpretation stored for the next rebuild.
    #[must_use]
    pub const fn photometric(&self) -> PhotometricInterpretation {
        self.photometric
    }

    /// Presentation LUT evidence stored for the next rebuild.
    ///
    /// `Absent` on `MONOCHROME1` is not [`PresentationLutEvidence::Shape`] of
    /// inverse, even when both invert.
    #[must_use]
    pub const fn evidence(&self) -> PresentationLutEvidence {
        self.evidence
    }

    /// Read the chain's selections. This does not rebuild them.
    pub(crate) fn encode(&self) -> Result<EncodedPresentation, crate::snapshot::SnapshotError> {
        Ok(EncodedPresentation {
            modality: encode_modality(self.chain.modality())?,
            voi: encode_voi(self.chain.voi())?,
            photometric: self.photometric,
            evidence: self.evidence,
            colormap: self.colormap,
        })
    }

    /// Rebuild through [`ModalityTransform::new`], [`VoiTransform::new`] and
    /// [`LutChain::new`]. The arithmetic stays in `ocelli-pixel`.
    pub(crate) fn decode(
        encoded: EncodedPresentation,
    ) -> Result<Self, crate::snapshot::SnapshotError> {
        let modality = decode_modality(encoded.modality)?;
        let voi = decode_voi(encoded.voi)?;
        let chain = match LutChain::new(modality, voi, encoded.photometric, encoded.evidence) {
            Ok(chain) => chain,
            Err(_) => return Err(crate::snapshot::SnapshotError::ChainUnavailable),
        };
        Ok(Self::new(
            chain,
            encoded.photometric,
            encoded.evidence,
            encoded.colormap,
        ))
    }

    /// Whether the stored evidence is an explicit inverse shape.
    ///
    /// `MONOCHROME1` is not this flag. Flag bit 0 of the readback state is.
    #[must_use]
    pub const fn explicit_inverse(&self) -> bool {
        matches!(
            self.evidence,
            PresentationLutEvidence::Shape(PresentationLutShape::Inverse)
        )
    }

    fn replace_window(
        &mut self,
        center: f32,
        width: f32,
        function: VoiFunction,
    ) -> Result<(), PresentationError> {
        if self.chain.voi().window().is_none() {
            return Err(PresentationError::Sequence);
        }
        let (ymin, ymax) = self.chain.voi().output_range();
        let voi = match VoiTransform::new(None, &[center], &[width], 0, function, ymin, ymax) {
            Ok(voi) => voi,
            Err(PixelError::InvalidWindow) => return Err(PresentationError::InvalidWindow),
            Err(_) => return Err(PresentationError::Unavailable),
        };
        let evidence = self.evidence;
        self.rebuild(voi, evidence)
    }

    fn rebuild(
        &mut self,
        voi: VoiTransform,
        evidence: PresentationLutEvidence,
    ) -> Result<(), PresentationError> {
        let chain = match LutChain::new(
            self.chain.modality().clone(),
            voi,
            self.photometric,
            evidence,
        ) {
            Ok(chain) => chain,
            Err(PixelError::InvalidWindow) => return Err(PresentationError::InvalidWindow),
            Err(_) => return Err(PresentationError::Unavailable),
        };
        self.chain = chain;
        self.evidence = evidence;
        Ok(())
    }
}

/// A LUT descriptor carried by a snapshot, before [`LutDescriptor::new`].
#[derive(Clone, Debug)]
pub(crate) struct EncodedLut {
    pub declared_entry_count: u16,
    pub first_mapped_input: i32,
    pub bits_per_entry: u8,
    pub values: Vec<f32>,
}

/// Stage 1 as selected. A sequence does not also carry rescale.
#[derive(Clone, Debug)]
pub(crate) enum EncodedModality {
    Rescale { slope: f32, intercept: f32 },
    Lut(EncodedLut),
}

/// Stage 2 as selected. A sequence does not also carry a window.
#[derive(Clone, Debug)]
pub(crate) enum EncodedVoi {
    Window {
        center: f32,
        width: f32,
        function: VoiFunction,
        ymin: f32,
        ymax: f32,
    },
    Lut(EncodedLut),
}

/// The presentation fields a snapshot round-trips.
#[derive(Clone, Debug)]
pub(crate) struct EncodedPresentation {
    pub modality: EncodedModality,
    pub voi: EncodedVoi,
    pub photometric: PhotometricInterpretation,
    pub evidence: PresentationLutEvidence,
    pub colormap: Colormap,
}

fn encode_modality(
    modality: &ModalityTransform,
) -> Result<EncodedModality, crate::snapshot::SnapshotError> {
    if let Some(lut) = modality.lut() {
        return Ok(EncodedModality::Lut(encode_lut(lut)?));
    }
    let Some((slope, intercept)) = modality.rescale() else {
        return Err(crate::snapshot::SnapshotError::ChainUnavailable);
    };
    if !slope.is_finite() || !intercept.is_finite() {
        return Err(crate::snapshot::SnapshotError::NonFinite);
    }
    Ok(EncodedModality::Rescale { slope, intercept })
}

fn encode_voi(voi: &VoiTransform) -> Result<EncodedVoi, crate::snapshot::SnapshotError> {
    if let Some(lut) = voi.lut() {
        return Ok(EncodedVoi::Lut(encode_lut(lut)?));
    }
    let Some((center, width, function)) = voi.window() else {
        return Err(crate::snapshot::SnapshotError::ChainUnavailable);
    };
    let (ymin, ymax) = voi.output_range();
    if !center.is_finite() || !width.is_finite() || !ymin.is_finite() || !ymax.is_finite() {
        return Err(crate::snapshot::SnapshotError::NonFinite);
    }
    Ok(EncodedVoi::Window {
        center,
        width,
        function,
        ymin,
        ymax,
    })
}

fn encode_lut(lut: &LutDescriptor) -> Result<EncodedLut, crate::snapshot::SnapshotError> {
    if lut.values().iter().any(|value| !value.is_finite()) {
        return Err(crate::snapshot::SnapshotError::NonFinite);
    }
    Ok(EncodedLut {
        declared_entry_count: lut.declared_entry_count(),
        first_mapped_input: lut.first_mapped_input(),
        bits_per_entry: lut.bits_per_entry(),
        values: lut.values().to_vec(),
    })
}

fn decode_modality(
    encoded: EncodedModality,
) -> Result<ModalityTransform, crate::snapshot::SnapshotError> {
    match encoded {
        EncodedModality::Rescale { slope, intercept } => {
            if !slope.is_finite() || !intercept.is_finite() {
                return Err(crate::snapshot::SnapshotError::NonFinite);
            }
            match ModalityTransform::new(None, Some(slope), Some(intercept)) {
                Ok(modality) => Ok(modality),
                Err(_) => Err(crate::snapshot::SnapshotError::ChainUnavailable),
            }
        }
        EncodedModality::Lut(lut) => {
            let descriptor = decode_lut(lut)?;
            match ModalityTransform::new(Some(descriptor), None, None) {
                Ok(modality) => Ok(modality),
                Err(_) => Err(crate::snapshot::SnapshotError::ChainUnavailable),
            }
        }
    }
}

fn decode_voi(encoded: EncodedVoi) -> Result<VoiTransform, crate::snapshot::SnapshotError> {
    match encoded {
        EncodedVoi::Window {
            center,
            width,
            function,
            ymin,
            ymax,
        } => {
            if !center.is_finite() || !width.is_finite() || !ymin.is_finite() || !ymax.is_finite() {
                return Err(crate::snapshot::SnapshotError::NonFinite);
            }
            match VoiTransform::new(None, &[center], &[width], 0, function, ymin, ymax) {
                Ok(voi) => Ok(voi),
                Err(PixelError::NonFiniteLutValue) => {
                    Err(crate::snapshot::SnapshotError::NonFinite)
                }
                Err(_) => Err(crate::snapshot::SnapshotError::ChainUnavailable),
            }
        }
        EncodedVoi::Lut(lut) => {
            let descriptor = decode_lut(lut)?;
            match VoiTransform::new(Some(descriptor), &[], &[], 0, VoiFunction::Linear, 0.0, 1.0) {
                Ok(voi) => Ok(voi),
                Err(_) => Err(crate::snapshot::SnapshotError::ChainUnavailable),
            }
        }
    }
}

fn decode_lut(lut: EncodedLut) -> Result<LutDescriptor, crate::snapshot::SnapshotError> {
    if lut.values.iter().any(|value| !value.is_finite()) {
        return Err(crate::snapshot::SnapshotError::NonFinite);
    }
    match LutDescriptor::new(
        lut.declared_entry_count,
        lut.first_mapped_input,
        lut.bits_per_entry,
        lut.values,
    ) {
        Ok(descriptor) => Ok(descriptor),
        Err(PixelError::NonFiniteLutValue) => Err(crate::snapshot::SnapshotError::NonFinite),
        Err(_) => Err(crate::snapshot::SnapshotError::BadLut),
    }
}
