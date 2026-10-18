//! The stack camera.
//!
//! HLD section 16 gives [`Pt<Canvas>`] with a top-left origin and y down.
//! HLD section 17.4 gives the sixteen `f32` values [`View::camera_matrix`]
//! fills. HLD section 22 gives `Dirty.camera`. Fit, rotation and flip are
//! the operations the HLD names a camera for and does not specify. They
//! follow the F-045 plan.

use ocelli_core::{Canvas, Pt};

/// A scale about the fitted centre. Distinct from [`Radians`] so the two
/// cannot be swapped at a call.
#[derive(Debug, Clone, Copy)]
pub struct Zoom(f64);

impl Zoom {
    /// `1` leaves the scale `fit` chose.
    #[must_use]
    pub const fn new(value: f64) -> Self {
        Self(value)
    }

    /// The stored factor.
    #[must_use]
    pub const fn get(self) -> f64 {
        self.0
    }
}

/// A rotation in radians, clockwise on the y-down canvas.
#[derive(Debug, Clone, Copy)]
pub struct Radians(f64);

impl Radians {
    /// Store `value` with no range reduction at the call.
    #[must_use]
    pub const fn new(value: f64) -> Self {
        Self(value)
    }

    /// The stored angle.
    #[must_use]
    pub const fn get(self) -> f64 {
        self.0
    }
}

/// Why [`View::set_pan`] refused the point.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ViewError {
    /// `Pt<Canvas>.z` was not zero. A slice index cannot hide in a pan.
    PanZ,
}

/// Pan, zoom, rotation, flip, spacing and the index shift of one stack view.
///
/// Image space origin is the top-left corner of texel `(0, 0)`. The centre
/// is `(columns / 2, rows / 2)`. A point moves in this order: spacing, flip
/// about the centre, clockwise rotation about the centre, scale
/// `fit_scale * zoom`, then the letterbox anchor, the pan and the index
/// shift. The shift is axis-aligned in canvas pixels.
#[derive(Debug, Clone, Copy)]
pub struct View {
    pan_x: f64,
    pan_y: f64,
    zoom: f64,
    radians: f64,
    flip_horizontal: bool,
    flip_vertical: bool,
    column_mm: f64,
    row_mm: f64,
    shift_col: f64,
    shift_row: f64,
    fitted: bool,
    anchor_x: f64,
    anchor_y: f64,
    fit_scale: f64,
}

impl Default for View {
    fn default() -> Self {
        Self::new()
    }
}

impl View {
    /// Identity. Spacing is 1 mm by 1 mm. The index shift is 0. Nothing is fitted.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            pan_x: 0.0,
            pan_y: 0.0,
            zoom: 1.0,
            radians: 0.0,
            flip_horizontal: false,
            flip_vertical: false,
            column_mm: 1.0,
            row_mm: 1.0,
            shift_col: 0.0,
            shift_row: 0.0,
            fitted: false,
            anchor_x: 0.0,
            anchor_y: 0.0,
            fit_scale: 1.0,
        }
    }

    /// Replace the pan. `z` of zero, including negative zero, is accepted.
    ///
    /// # Errors
    ///
    /// [`ViewError::PanZ`] when `z` is not zero. The stored pan is unchanged.
    pub fn set_pan(&mut self, pan: Pt<Canvas>) -> Result<(), ViewError> {
        if pan.z.abs().to_bits() != 0.0_f64.to_bits() {
            return Err(ViewError::PanZ);
        }
        self.pan_x = pan.x;
        self.pan_y = pan.y;
        Ok(())
    }

    /// Replace the zoom factor. This does not refit and does not pan.
    pub const fn set_zoom(&mut self, zoom: Zoom) {
        self.zoom = zoom.0;
    }

    /// Replace the clockwise rotation. This does not refit.
    pub const fn set_rotation(&mut self, radians: Radians) {
        self.radians = radians.0;
    }

    /// Reflect columns about the image centre.
    pub const fn set_flip_horizontal(&mut self, flip: bool) {
        self.flip_horizontal = flip;
    }

    /// Reflect rows about the image centre.
    pub const fn set_flip_vertical(&mut self, flip: bool) {
        self.flip_vertical = flip;
    }

    /// Column millimetres, then row millimetres.
    ///
    /// Zero is accepted. A singleton axis uses it, and this function does
    /// not substitute 1.
    pub const fn set_spacing(&mut self, column_mm: f64, row_mm: f64) {
        self.column_mm = column_mm;
        self.row_mm = row_mm;
    }

    /// Image-pixel shift. F-049 is the caller that stores a measured one.
    ///
    /// The shift is applied after scale, in canvas pixels, and is not rotated.
    /// A zero spacing makes that axis contribute no shift.
    pub const fn set_index_shift(&mut self, column: f64, row: f64) {
        self.shift_col = column;
        self.shift_row = row;
    }

    /// Letterbox the transformed image inside the canvas.
    ///
    /// Pan becomes zero and zoom becomes 1. Rotation, flip, spacing and the
    /// index shift stay. The anchor becomes the canvas centre. Before the
    /// first call the anchor is the image centre, so a default view is the
    /// identity at spacing 1.
    pub fn fit(&mut self, canvas_w: f64, canvas_h: f64, columns: u32, rows: u32) {
        let (bounds_w, bounds_h) = self.physical_bounds(columns, rows);
        let chosen = axis_scale(canvas_w, bounds_w).min(axis_scale(canvas_h, bounds_h));
        self.fit_scale = if chosen.is_finite() { chosen } else { 1.0 };
        self.anchor_x = canvas_w * 0.5;
        self.anchor_y = canvas_h * 0.5;
        self.fitted = true;
        self.pan_x = 0.0;
        self.pan_y = 0.0;
        self.zoom = 1.0;
    }

    /// `fit_scale * zoom`.
    #[must_use]
    pub const fn scale(&self) -> f64 {
        self.fit_scale * self.zoom
    }

    /// Top-left of the displayed bounds, in canvas pixels.
    #[must_use]
    pub fn offset(&self, columns: u32, rows: u32) -> (f64, f64) {
        let (bounds_w, bounds_h) = self.physical_bounds(columns, rows);
        let scale = self.scale();
        let (anchor_x, anchor_y) = self.anchor(columns, rows);
        (
            anchor_x - bounds_w * scale * 0.5,
            anchor_y - bounds_h * scale * 0.5,
        )
    }

    /// Where an image-space point lands on the canvas.
    #[must_use]
    pub fn image_to_canvas(
        &self,
        image_x: f64,
        image_y: f64,
        columns: u32,
        rows: u32,
    ) -> (f64, f64) {
        let (centre_x, centre_y) = centre(columns, rows);
        let dx = flip_sign(self.flip_horizontal) * self.column_mm * (image_x - centre_x);
        let dy = flip_sign(self.flip_vertical) * self.row_mm * (image_y - centre_y);
        let (rotated_x, rotated_y) = clockwise(dx, dy, self.radians);
        let scale = self.scale();
        let (translate_x, translate_y) = self.translation(columns, rows);
        (
            translate_x + scale * rotated_x,
            translate_y + scale * rotated_y,
        )
    }

    /// Column-major matrix from a canvas pixel to an image coordinate.
    ///
    /// This is the inverse of [`Self::image_to_canvas`]. The fragment shader
    /// multiplies `(at.x, at.y, 0, 1)` by it. A zero spacing on one axis
    /// keeps the other axis and that axis's shift. A map with no extent
    /// writes the image centre. Neither path writes a non-finite value.
    #[must_use]
    pub fn camera_matrix(&self, columns: u32, rows: u32) -> [f32; 16] {
        let (centre_x, centre_y) = centre(columns, rows);
        let (p, q, r, u) = self.basis();
        let det = p * u - q * r;
        let (translate_x, translate_y) = self.translation(columns, rows);
        let (a, b, d, e) = if linear_is_invertible(p, q, r, u, det, translate_x, translate_y) {
            (u / det, -q / det, -r / det, p / det)
        } else if let Some(inverse) = rank_one_inverse(p, q, r, u) {
            inverse
        } else {
            return centre_matrix(centre_x, centre_y);
        };
        let c = centre_x - a * translate_x - b * translate_y;
        let f = centre_y - d * translate_x - e * translate_y;
        if !c.is_finite()
            || !f.is_finite()
            || !a.is_finite()
            || !b.is_finite()
            || !d.is_finite()
            || !e.is_finite()
        {
            return centre_matrix(centre_x, centre_y);
        }
        [
            f32_from_f64(a),
            f32_from_f64(d),
            0.0,
            0.0,
            f32_from_f64(b),
            f32_from_f64(e),
            0.0,
            0.0,
            0.0,
            0.0,
            1.0,
            0.0,
            f32_from_f64(c),
            f32_from_f64(f),
            0.0,
            1.0,
        ]
    }

    fn basis(&self) -> (f64, f64, f64, f64) {
        let scale = self.scale();
        let (column_x, column_y) = clockwise(
            flip_sign(self.flip_horizontal) * self.column_mm,
            0.0,
            self.radians,
        );
        let (row_x, row_y) = clockwise(
            0.0,
            flip_sign(self.flip_vertical) * self.row_mm,
            self.radians,
        );
        (
            scale * column_x,
            scale * row_x,
            scale * column_y,
            scale * row_y,
        )
    }

    fn translation(&self, columns: u32, rows: u32) -> (f64, f64) {
        let (anchor_x, anchor_y) = self.anchor(columns, rows);
        let scale = self.scale();
        (
            anchor_x + self.pan_x + self.shift_col * self.column_mm * scale,
            anchor_y + self.pan_y + self.shift_row * self.row_mm * scale,
        )
    }

    fn anchor(&self, columns: u32, rows: u32) -> (f64, f64) {
        if self.fitted {
            (self.anchor_x, self.anchor_y)
        } else {
            centre(columns, rows)
        }
    }

    /// Axis-aligned size, in millimetres, of the rotated image.
    ///
    /// Flip does not change the corner set of a centred rectangle, so it is
    /// not applied here. The four corners go through [`clockwise`], which is
    /// the same function a point uses.
    fn physical_bounds(&self, columns: u32, rows: u32) -> (f64, f64) {
        let half_w = f64_from_u32(columns) * self.column_mm * 0.5;
        let half_h = f64_from_u32(rows) * self.row_mm * 0.5;
        let mut max_x = 0.0_f64;
        let mut max_y = 0.0_f64;
        for (x, y) in [
            (half_w, half_h),
            (half_w, -half_h),
            (-half_w, half_h),
            (-half_w, -half_h),
        ] {
            let (rotated_x, rotated_y) = clockwise(x, y, self.radians);
            max_x = max_x.max(rotated_x.abs());
            max_y = max_y.max(rotated_y.abs());
        }
        (max_x * 2.0, max_y * 2.0)
    }
}

fn centre(columns: u32, rows: u32) -> (f64, f64) {
    (f64_from_u32(columns) * 0.5, f64_from_u32(rows) * 0.5)
}

fn flip_sign(flip: bool) -> f64 {
    if flip { -1.0 } else { 1.0 }
}

fn axis_scale(canvas: f64, extent: f64) -> f64 {
    if extent.abs().to_bits() == 0.0_f64.to_bits() {
        f64::INFINITY
    } else {
        canvas / extent
    }
}

fn linear_is_invertible(p: f64, q: f64, r: f64, u: f64, det: f64, tx: f64, ty: f64) -> bool {
    det.is_finite()
        && det.abs().to_bits() != 0.0_f64.to_bits()
        && p.is_finite()
        && q.is_finite()
        && r.is_finite()
        && u.is_finite()
        && tx.is_finite()
        && ty.is_finite()
}

/// `M^T / ||M||^2` for a singular non-zero 2 by 2 matrix.
///
/// A zero column spacing makes the column of `M` zero and leaves the row.
/// Dividing by the squared length keeps that row and drops the empty axis
/// onto the image centre. The zero matrix has no extent, so the caller
/// uses the centre instead.
fn rank_one_inverse(p: f64, q: f64, r: f64, u: f64) -> Option<(f64, f64, f64, f64)> {
    let length_sq = p * p + q * q + r * r + u * u;
    if !length_sq.is_finite() || length_sq.abs().to_bits() == 0.0_f64.to_bits() {
        return None;
    }
    Some((p / length_sq, r / length_sq, q / length_sq, u / length_sq))
}

fn centre_matrix(centre_x: f64, centre_y: f64) -> [f32; 16] {
    [
        0.0,
        0.0,
        0.0,
        0.0,
        0.0,
        0.0,
        0.0,
        0.0,
        0.0,
        0.0,
        1.0,
        0.0,
        f32_from_f64(centre_x),
        f32_from_f64(centre_y),
        0.0,
        1.0,
    ]
}

/// Clockwise on a y-down plane.
///
/// A quarter turn is `sin = 1` and `cos = 0`, which sends `(dx, dy)` to
/// `(-dy, dx)`.
fn clockwise(dx: f64, dy: f64, radians: f64) -> (f64, f64) {
    let (sin, cos) = sin_cos(radians);
    (dx * cos - dy * sin, dx * sin + dy * cos)
}

/// `(sin, cos)`. Exact multiples of a quarter turn use exact 0 and 1.
///
/// `cos(π/2)` is not a zero bit. The quarter-turn fixture lands on `(2, 0)`
/// only when that residual is removed. Any other angle uses glam's libm,
/// because `f64::sin_cos` is not available to a `no_std` crate.
fn sin_cos(radians: f64) -> (f64, f64) {
    if let Some(exact) = exact_quarter(radians) {
        return exact;
    }
    let unit = glam::DVec2::from_angle(radians);
    (unit.y, unit.x)
}

/// `(sin, cos)` when `radians` is an exact multiple of π/2.
fn exact_quarter(radians: f64) -> Option<(f64, f64)> {
    let quarter = core::f64::consts::FRAC_PI_2;
    let ratio = radians / quarter;
    if !ratio.is_finite() {
        return None;
    }
    let whole = trunc_toward_zero(ratio);
    if (ratio - whole).abs().to_bits() != 0.0_f64.to_bits() {
        return None;
    }
    let reduced = whole - 4.0 * trunc_toward_zero(whole / 4.0);
    let quadrant = if reduced < 0.0 {
        reduced + 4.0
    } else {
        reduced
    };
    if quadrant.to_bits() == 0.0_f64.to_bits() {
        Some((0.0, 1.0))
    } else if quadrant.to_bits() == 1.0_f64.to_bits() {
        Some((1.0, 0.0))
    } else if quadrant.to_bits() == 2.0_f64.to_bits() {
        Some((0.0, -1.0))
    } else if quadrant.to_bits() == 3.0_f64.to_bits() {
        Some((-1.0, 0.0))
    } else {
        None
    }
}

/// Truncate toward zero. `f64::trunc` is std-only.
fn trunc_toward_zero(value: f64) -> f64 {
    if !value.is_finite() {
        return value;
    }
    let bits = value.to_bits();
    let exponent = i32::try_from((bits >> 52) & 0x7ff).unwrap_or(0);
    let unbiased = exponent - 1023;
    if unbiased < 0 {
        return f64::from_bits(bits & (1_u64 << 63));
    }
    if unbiased >= 52 {
        return value;
    }
    let fraction_bits = u32::try_from(52 - unbiased).unwrap_or(0);
    let clear = (1_u64 << fraction_bits) - 1;
    f64::from_bits(bits & !clear)
}

/// Every `u32` is an exact `f64`. Splitting avoids a truncating cast.
fn f64_from_u32(value: u32) -> f64 {
    let high = u16::try_from(value >> 16).unwrap_or(0);
    let low = u16::try_from(value & 0xffff).unwrap_or(0);
    f64::from(high) * 65_536.0 + f64::from(low)
}

/// Narrow one camera component to the `f32` uniform of HLD section 17.4.
///
/// The view math stays `f64`. Rounding is ties-to-even. A magnitude past the
/// finite `f32` range becomes the signed infinity. Subnormal `f64` values are
/// below every `f32` subnormal and become a signed zero.
fn f32_from_f64(value: f64) -> f32 {
    if value.is_nan() {
        return f32::from_bits(0x7fc0_0000);
    }
    let bits = value.to_bits();
    let sign = u32::try_from(bits >> 63).unwrap_or(0);
    let exponent = u32::try_from((bits >> 52) & 0x7ff).unwrap_or(0);
    let fraction = bits & 0x000f_ffff_ffff_ffff;
    if exponent == 0x7ff {
        return f32::from_bits((sign << 31) | 0x7f80_0000);
    }
    if exponent == 0 {
        return f32::from_bits(sign << 31);
    }
    let target = i32::try_from(exponent).unwrap_or(0) - 1023 + 127;
    let significand = fraction | (1_u64 << 52);
    if target >= 255 {
        return f32::from_bits((sign << 31) | 0x7f80_0000);
    }
    if target <= 0 {
        let rounded = rounded_shift(significand, 30 - target);
        if rounded >= (1_u64 << 23) {
            return f32::from_bits((sign << 31) | (1_u32 << 23));
        }
        return f32::from_bits((sign << 31) | u32::try_from(rounded).unwrap_or(0));
    }
    let rounded = rounded_shift(significand, 29);
    let mut out_exponent = u32::try_from(target).unwrap_or(0);
    let mantissa = if rounded >= (1_u64 << 24) {
        out_exponent = out_exponent.saturating_add(1);
        0
    } else {
        u32::try_from(rounded & 0x7f_ffff).unwrap_or(0)
    };
    if out_exponent >= 255 {
        return f32::from_bits((sign << 31) | 0x7f80_0000);
    }
    f32::from_bits((sign << 31) | (out_exponent << 23) | mantissa)
}

/// `sig / 2^shift`, rounded ties-to-even.
fn rounded_shift(sig: u64, shift: i32) -> u64 {
    if shift <= 0 {
        return sig;
    }
    let Ok(places) = u32::try_from(shift) else {
        return 0;
    };
    if places == 0 || places >= 64 {
        return 0;
    }
    let main = sig >> places;
    let half = 1_u64 << (places - 1);
    let remainder = sig & ((1_u64 << places) - 1);
    let round_up = remainder > half || (remainder == half && (main & 1) == 1);
    if round_up {
        main.saturating_add(1)
    } else {
        main
    }
}
