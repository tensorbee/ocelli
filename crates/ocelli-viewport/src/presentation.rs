//! Window presets, invert and the colormap.
//!
//! HLD section 18 says the LUT chain exists once, in `ocelli-pixel`. A preset
//! calls [`VoiTransform::new`] and does not add a formula. HLD section 18.3's
//! soft-tissue window is centre 40 and width 400. Lung and bone are the
//! centre and width pairs named by the F-046 plan.

use ocelli_pixel::{
    LutChain, PhotometricInterpretation, PixelError, PresentationLutEvidence, PresentationLutShape,
    VoiFunction, VoiTransform,
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
