//! Packed stack commands. HLD section 17.1.
//!
//! The envelope is little-endian `[u16 kind][u16 viewport_id][u32 payload_len]`.
//! Kind 0 is invalid. Ids 1 to 11 follow `STACK_COMMANDS`. A batch is checked
//! before any of it is written. `View`, presentation and scroll stay as they
//! were when any command in the batch is refused.

use ocelli_core::{Canvas, Pt};
use ocelli_pixel::{
    LutChain, ModalityTransform, PhotometricInterpretation, PresentationLutEvidence, VoiFunction,
    VoiTransform,
};

use crate::presentation::{Colormap, Presentation, PresentationError, WindowPreset};
use crate::scroll::StackSequence;
use crate::view::{self, Radians, View, ViewError, Zoom};

/// Commands in one batch.
pub const MAX_COMMANDS: usize = 64;

/// Bytes in one command buffer, header and payload included.
pub const MAX_COMMAND_BYTES: usize = 4096;

/// Readback bytes. HLD section 17.4.
pub const STATE_BYTES: usize = 96;

const KIND_FIT: u16 = 1;
const KIND_PAN: u16 = 2;
const KIND_ZOOM: u16 = 3;
const KIND_ROTATE: u16 = 4;
const KIND_FLIP_HORIZONTAL: u16 = 5;
const KIND_FLIP_VERTICAL: u16 = 6;
const KIND_SET_WINDOW: u16 = 7;
const KIND_APPLY_PRESET: u16 = 8;
const KIND_SET_INVERT: u16 = 9;
const KIND_SET_COLORMAP: u16 = 10;
const KIND_SCROLL: u16 = 11;

/// Why [`StackViewport::apply_commands`] wrote nothing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommandError {
    /// The buffer is longer than [`MAX_COMMAND_BYTES`], or it holds more than
    /// [`MAX_COMMANDS`] commands.
    Limit,
    /// A header or a payload ended before its declared length.
    Truncated,
    /// A length did not fit in the address calculation.
    Overflow,
    /// The kind is 0 or any id this table does not name.
    UnknownKind,
    /// `payload_len` is not the length this kind carries.
    BadLength,
    /// The viewport id is not this stack.
    UnknownViewport,
    /// An enum byte was outside its domain.
    BadEnum,
    /// A float was NaN or infinite, or it did not narrow to a finite `f32`.
    NonFinite,
    /// In-process pan z was not zero. The wire pan has no z.
    PanZ,
    /// [`PresentationError::Sequence`].
    Sequence,
    /// [`PresentationError::InvalidWindow`].
    InvalidWindow,
    /// [`PresentationError::Unavailable`].
    Unavailable,
    /// [`StackViewport::write_state`] was given a buffer other than 96 bytes.
    StateLength,
}

#[derive(Clone, Copy)]
enum Parsed {
    Fit {
        width: f64,
        height: f64,
        columns: u32,
        rows: u32,
    },
    Pan {
        x: f64,
        y: f64,
    },
    Zoom {
        factor: f64,
    },
    Rotate {
        radians: f64,
    },
    FlipHorizontal {
        enabled: bool,
    },
    FlipVertical {
        enabled: bool,
    },
    SetWindow {
        centre: f32,
        width: f32,
        function: VoiFunction,
    },
    ApplyPreset {
        preset: WindowPreset,
    },
    SetInvert {
        invert: bool,
    },
    SetColormap {
        colormap: Colormap,
    },
    Scroll {
        step: i32,
    },
}

/// One stack viewport the command channel mutates.
#[derive(Debug, Clone)]
pub struct StackViewport {
    id: u16,
    view: View,
    presentation: Presentation,
    scroll: StackSequence,
    columns: u32,
    rows: u32,
    css_width: f64,
    css_height: f64,
    refining: bool,
    loading: bool,
    error: bool,
    generation: u32,
}

impl StackViewport {
    /// Identity modality, a width-1 linear window, and an empty stack.
    ///
    /// # Errors
    ///
    /// [`CommandError::Unavailable`] when that chain is refused. The inputs
    /// are inside the documented domain, so a refusal is a constructor change.
    pub fn try_identity(id: u16) -> Result<Self, CommandError> {
        let modality = ModalityTransform::new(None, Some(1.0), Some(0.0))
            .map_err(|_| CommandError::Unavailable)?;
        let voi = VoiTransform::new(None, &[40.0], &[1.0], 0, VoiFunction::Linear, 0.0, 255.0)
            .map_err(|_| CommandError::Unavailable)?;
        let chain = LutChain::new(
            modality,
            voi,
            PhotometricInterpretation::Monochrome2,
            PresentationLutEvidence::Absent,
        )
        .map_err(|_| CommandError::Unavailable)?;
        let presentation = Presentation::new(
            chain,
            PhotometricInterpretation::Monochrome2,
            PresentationLutEvidence::Absent,
            Colormap::Grey,
        );
        Ok(Self::new(id, presentation, StackSequence::new(&[])))
    }

    /// Empty camera, the caller's presentation and scroll, and no CSS size yet.
    #[must_use]
    pub fn new(id: u16, presentation: Presentation, scroll: StackSequence) -> Self {
        Self {
            id,
            view: View::new(),
            presentation,
            scroll,
            columns: 0,
            rows: 0,
            css_width: 0.0,
            css_height: 0.0,
            refining: false,
            loading: false,
            error: false,
            generation: 0,
        }
    }

    /// The viewport id carried in every command header.
    #[must_use]
    pub const fn id(&self) -> u16 {
        self.id
    }

    /// The camera a measurement reads. Commands are the writer.
    #[must_use]
    pub const fn view(&self) -> &View {
        &self.view
    }

    /// The presentation a renderer reads.
    #[must_use]
    pub const fn presentation(&self) -> &Presentation {
        &self.presentation
    }

    /// The scroll sequence.
    #[must_use]
    pub const fn scroll(&self) -> &StackSequence {
        &self.scroll
    }

    /// Install an ordered source sequence without replacing camera or presentation.
    pub fn replace_stack(&mut self, sequence: StackSequence) {
        self.loading = !sequence.is_empty();
        self.scroll = sequence;
    }

    /// The selected frame is unavailable until its bulk input commits.
    pub const fn set_loading(&mut self, loading: bool) {
        self.loading = loading;
    }

    /// CSS width last written by Fit.
    #[must_use]
    pub const fn css_width(&self) -> f64 {
        self.css_width
    }

    /// CSS height last written by Fit.
    #[must_use]
    pub const fn css_height(&self) -> f64 {
        self.css_height
    }

    /// Columns last written by Fit. Zero before the first Fit.
    #[must_use]
    pub const fn columns(&self) -> u32 {
        self.columns
    }

    /// Rows last written by Fit.
    #[must_use]
    pub const fn rows(&self) -> u32 {
        self.rows
    }

    /// Flag bit 3. F-X003 sets this while a reduced frame is on screen.
    pub const fn set_refining(&mut self, refining: bool) {
        self.refining = refining;
    }

    /// Generation copied into a Presented event.
    pub const fn set_generation(&mut self, generation: u32) {
        self.generation = generation;
    }

    /// Generation last stored.
    #[must_use]
    pub const fn generation(&self) -> u32 {
        self.generation
    }

    /// Section 17.4 flags. Bit 0 invert, bit 1 loading, bit 2 error, bit 3 refining.
    #[must_use]
    pub const fn flags(&self) -> u32 {
        let invert = if self.presentation.explicit_inverse() {
            1
        } else {
            0
        };
        let loading = if self.loading { 2 } else { 0 };
        let error = if self.error { 4 } else { 0 };
        let refining = if self.refining { 8 } else { 0 };
        invert | loading | error | refining
    }

    /// Apply every command, or apply none.
    ///
    /// # Errors
    ///
    /// A structural or semantic [`CommandError`]. The stored view, presentation
    /// and scroll are unchanged.
    pub fn apply_commands(&mut self, bytes: &[u8]) -> Result<u32, CommandError> {
        if bytes.len() > MAX_COMMAND_BYTES {
            return Err(CommandError::Limit);
        }
        let (parsed, count) = parse_all(bytes, self.id)?;
        let mut view = self.view;
        let mut presentation = self.presentation.clone();
        let mut scroll = self.scroll.clone();
        let mut fitted = Fitted {
            columns: self.columns,
            rows: self.rows,
            css_width: self.css_width,
            css_height: self.css_height,
        };
        for index in 0..count {
            let Some(command) = parsed.get(index).copied() else {
                return Err(CommandError::Overflow);
            };
            apply_one(
                command,
                &mut view,
                &mut presentation,
                &mut scroll,
                &mut fitted,
            )?;
        }
        self.view = view;
        self.presentation = presentation;
        self.scroll = scroll;
        self.columns = fitted.columns;
        self.rows = fitted.rows;
        self.css_width = fitted.css_width;
        self.css_height = fitted.css_height;
        u32::try_from(count).map_err(|_| CommandError::Overflow)
    }

    /// Copy section 17.4 into `out`.
    ///
    /// Camera at 0, centre at 64, width at 68, slice at 72, count at 76,
    /// flags at 80, and padding at 84. A VOI sequence has no window, so those
    /// two floats are zero.
    ///
    /// # Errors
    ///
    /// [`CommandError::StateLength`] when `out` is not 96 bytes.
    pub fn write_state(&self, out: &mut [u8]) -> Result<(), CommandError> {
        if out.len() != STATE_BYTES {
            return Err(CommandError::StateLength);
        }
        let camera = self.view.camera_matrix(self.columns, self.rows);
        for (index, value) in camera.iter().enumerate() {
            let at = index.saturating_mul(4);
            write_f32(out, at, *value);
        }
        let (centre, width) = self
            .presentation
            .chain()
            .voi()
            .window()
            .map(|(centre, width, _function)| (centre, width))
            .unwrap_or((0.0, 0.0));
        write_f32(out, 64, centre);
        write_f32(out, 68, width);
        let slice = self.scroll.index().unwrap_or(0);
        write_u32(out, 72, slice);
        write_u32(out, 76, self.scroll.len());
        write_u32(out, 80, self.flags());
        write_u32(out, 84, 0);
        write_u32(out, 88, 0);
        write_u32(out, 92, 0);
        Ok(())
    }
}

struct Fitted {
    columns: u32,
    rows: u32,
    css_width: f64,
    css_height: f64,
}

fn apply_one(
    command: Parsed,
    view: &mut View,
    presentation: &mut Presentation,
    scroll: &mut StackSequence,
    fitted: &mut Fitted,
) -> Result<(), CommandError> {
    match command {
        Parsed::Fit {
            width,
            height,
            columns,
            rows,
        } => {
            view.fit(width, height, columns, rows);
            fitted.columns = columns;
            fitted.rows = rows;
            fitted.css_width = width;
            fitted.css_height = height;
        }
        Parsed::Pan { x, y } => match view.set_pan(Pt::<Canvas>::new(x, y, 0.0)) {
            Ok(()) => {}
            Err(ViewError::PanZ) => return Err(CommandError::PanZ),
        },
        Parsed::Zoom { factor } => view.set_zoom(Zoom::new(factor)),
        Parsed::Rotate { radians } => view.set_rotation(Radians::new(radians)),
        Parsed::FlipHorizontal { enabled } => view.set_flip_horizontal(enabled),
        Parsed::FlipVertical { enabled } => view.set_flip_vertical(enabled),
        Parsed::SetWindow {
            centre,
            width,
            function,
        } => presentation
            .set_window(centre, width, function)
            .map_err(CommandError::from)?,
        Parsed::ApplyPreset { preset } => {
            presentation
                .apply_preset(preset)
                .map_err(CommandError::from)?;
        }
        Parsed::SetInvert { invert } => {
            presentation
                .set_invert(invert)
                .map_err(CommandError::from)?;
        }
        Parsed::SetColormap { colormap } => presentation.set_colormap(colormap),
        Parsed::Scroll { step } => scroll.scroll_by(step),
    }
    Ok(())
}

impl From<PresentationError> for CommandError {
    fn from(error: PresentationError) -> Self {
        match error {
            PresentationError::Sequence => Self::Sequence,
            PresentationError::InvalidWindow => Self::InvalidWindow,
            PresentationError::Unavailable => Self::Unavailable,
        }
    }
}

fn parse_all(bytes: &[u8], viewport: u16) -> Result<([Parsed; MAX_COMMANDS], usize), CommandError> {
    let mut parsed = [Parsed::Scroll { step: 0 }; MAX_COMMANDS];
    let mut count = 0;
    let mut offset = 0;
    while offset < bytes.len() {
        if count == MAX_COMMANDS {
            return Err(CommandError::Limit);
        }
        let (command, next) = parse_one(bytes, offset, viewport)?;
        let Some(slot) = parsed.get_mut(count) else {
            return Err(CommandError::Limit);
        };
        *slot = command;
        count = count.saturating_add(1);
        offset = next;
    }
    Ok((parsed, count))
}

fn parse_one(bytes: &[u8], offset: usize, viewport: u16) -> Result<(Parsed, usize), CommandError> {
    let kind = read_u16(bytes, offset)?;
    let id = read_u16(bytes, offset.saturating_add(2))?;
    let payload_len = read_u32(bytes, offset.saturating_add(4))?;
    if id != viewport {
        return Err(CommandError::UnknownViewport);
    }
    let header = offset.checked_add(8).ok_or(CommandError::Overflow)?;
    let len = usize::try_from(payload_len).map_err(|_| CommandError::Overflow)?;
    let end = header.checked_add(len).ok_or(CommandError::Overflow)?;
    if end > bytes.len() {
        return Err(CommandError::Truncated);
    }
    let payload = bytes.get(header..end).ok_or(CommandError::Truncated)?;
    let expected = payload_len_of(kind).ok_or(CommandError::UnknownKind)?;
    if payload_len != expected {
        return Err(CommandError::BadLength);
    }
    let command = decode(kind, payload)?;
    Ok((command, end))
}

fn payload_len_of(kind: u16) -> Option<u32> {
    match kind {
        KIND_FIT => Some(24),
        KIND_PAN => Some(16),
        KIND_ZOOM | KIND_ROTATE => Some(8),
        KIND_FLIP_HORIZONTAL | KIND_FLIP_VERTICAL | KIND_APPLY_PRESET | KIND_SET_INVERT
        | KIND_SET_COLORMAP => Some(1),
        KIND_SET_WINDOW => Some(17),
        KIND_SCROLL => Some(4),
        _ => None,
    }
}

fn decode(kind: u16, payload: &[u8]) -> Result<Parsed, CommandError> {
    match kind {
        KIND_FIT => Ok(Parsed::Fit {
            width: finite_f64(read_f64(payload, 0)?)?,
            height: finite_f64(read_f64(payload, 8)?)?,
            columns: read_u32(payload, 16)?,
            rows: read_u32(payload, 20)?,
        }),
        KIND_PAN => Ok(Parsed::Pan {
            x: finite_f64(read_f64(payload, 0)?)?,
            y: finite_f64(read_f64(payload, 8)?)?,
        }),
        KIND_ZOOM => Ok(Parsed::Zoom {
            factor: finite_f64(read_f64(payload, 0)?)?,
        }),
        KIND_ROTATE => Ok(Parsed::Rotate {
            radians: finite_f64(read_f64(payload, 0)?)?,
        }),
        KIND_FLIP_HORIZONTAL => Ok(Parsed::FlipHorizontal {
            enabled: flag(first(payload)?)?,
        }),
        KIND_FLIP_VERTICAL => Ok(Parsed::FlipVertical {
            enabled: flag(first(payload)?)?,
        }),
        KIND_SET_WINDOW => {
            let centre = finite_f32(read_f64(payload, 0)?)?;
            let width = finite_f32(read_f64(payload, 8)?)?;
            let function = voi_function(byte_at(payload, 16)?)?;
            Ok(Parsed::SetWindow {
                centre,
                width,
                function,
            })
        }
        KIND_APPLY_PRESET => Ok(Parsed::ApplyPreset {
            preset: preset(first(payload)?)?,
        }),
        KIND_SET_INVERT => Ok(Parsed::SetInvert {
            invert: flag(first(payload)?)?,
        }),
        KIND_SET_COLORMAP => Ok(Parsed::SetColormap {
            colormap: colormap(first(payload)?)?,
        }),
        KIND_SCROLL => Ok(Parsed::Scroll {
            step: read_i32(payload, 0)?,
        }),
        _ => Err(CommandError::UnknownKind),
    }
}

fn finite_f64(value: f64) -> Result<f64, CommandError> {
    if value.is_finite() {
        Ok(value)
    } else {
        Err(CommandError::NonFinite)
    }
}

fn finite_f32(value: f64) -> Result<f32, CommandError> {
    let narrowed = view::f32_from_f64(finite_f64(value)?);
    if narrowed.is_finite() {
        Ok(narrowed)
    } else {
        Err(CommandError::NonFinite)
    }
}

fn flag(value: u8) -> Result<bool, CommandError> {
    match value {
        0 => Ok(false),
        1 => Ok(true),
        _ => Err(CommandError::BadEnum),
    }
}

fn voi_function(value: u8) -> Result<VoiFunction, CommandError> {
    match value {
        0 => Ok(VoiFunction::Linear),
        1 => Ok(VoiFunction::LinearExact),
        2 => Ok(VoiFunction::Sigmoid),
        _ => Err(CommandError::BadEnum),
    }
}

fn preset(value: u8) -> Result<WindowPreset, CommandError> {
    match value {
        0 => Ok(WindowPreset::SoftTissue),
        1 => Ok(WindowPreset::Lung),
        2 => Ok(WindowPreset::Bone),
        _ => Err(CommandError::BadEnum),
    }
}

fn colormap(value: u8) -> Result<Colormap, CommandError> {
    match value {
        0 => Ok(Colormap::Grey),
        1 => Ok(Colormap::Stops),
        _ => Err(CommandError::BadEnum),
    }
}

fn first(payload: &[u8]) -> Result<u8, CommandError> {
    payload.first().copied().ok_or(CommandError::Truncated)
}

fn byte_at(payload: &[u8], at: usize) -> Result<u8, CommandError> {
    payload.get(at).copied().ok_or(CommandError::Truncated)
}

fn read_u16(bytes: &[u8], at: usize) -> Result<u16, CommandError> {
    let end = at.checked_add(2).ok_or(CommandError::Overflow)?;
    let slot = bytes.get(at..end).ok_or(CommandError::Truncated)?;
    let mut buf = [0_u8; 2];
    buf.copy_from_slice(slot);
    Ok(u16::from_le_bytes(buf))
}

fn read_u32(bytes: &[u8], at: usize) -> Result<u32, CommandError> {
    let end = at.checked_add(4).ok_or(CommandError::Overflow)?;
    let slot = bytes.get(at..end).ok_or(CommandError::Truncated)?;
    let mut buf = [0_u8; 4];
    buf.copy_from_slice(slot);
    Ok(u32::from_le_bytes(buf))
}

fn read_i32(bytes: &[u8], at: usize) -> Result<i32, CommandError> {
    let end = at.checked_add(4).ok_or(CommandError::Overflow)?;
    let slot = bytes.get(at..end).ok_or(CommandError::Truncated)?;
    let mut buf = [0_u8; 4];
    buf.copy_from_slice(slot);
    Ok(i32::from_le_bytes(buf))
}

fn read_f64(bytes: &[u8], at: usize) -> Result<f64, CommandError> {
    let end = at.checked_add(8).ok_or(CommandError::Overflow)?;
    let slot = bytes.get(at..end).ok_or(CommandError::Truncated)?;
    let mut buf = [0_u8; 8];
    buf.copy_from_slice(slot);
    Ok(f64::from_le_bytes(buf))
}

fn write_f32(out: &mut [u8], at: usize, value: f32) {
    let Some(end) = at.checked_add(4) else {
        return;
    };
    let Some(slot) = out.get_mut(at..end) else {
        return;
    };
    slot.copy_from_slice(&value.to_le_bytes());
}

fn write_u32(out: &mut [u8], at: usize, value: u32) {
    let Some(end) = at.checked_add(4) else {
        return;
    };
    let Some(slot) = out.get_mut(at..end) else {
        return;
    };
    slot.copy_from_slice(&value.to_le_bytes());
}
