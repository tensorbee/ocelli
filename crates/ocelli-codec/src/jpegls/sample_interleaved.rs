//! 8-bit sample-interleaved JPEG-LS, ISO/IEC 14495-1 with `ILV = 2`.
//!
//! One regular context model and one run index serve the whole pixel. The
//! three components update that model in order. That is the coding of the
//! lossless RGB corpus row, whose bytes must equal the uncompressed ramp.

use crate::CodecError;

const COMPONENTS: usize = 3;
const CONTEXTS: usize = 365;
const RESET: u32 = 64;
const LIMIT: u32 = 32;
const QBPP: u32 = 8;
const MAX_SAMPLE: i32 = 255;
const A_INIT: u32 = 4;
const T1: i32 = 3;
const T2: i32 = 7;
const T3: i32 = 21;

/// ISO/IEC 14495-1 Table C.1. `J[i]` is the run segment length `2^J[i]`.
const J: [u32; 32] = [
    0, 0, 0, 0, 1, 1, 1, 1, 2, 2, 2, 2, 3, 3, 3, 3, 4, 4, 5, 5, 6, 6, 7, 7, 8, 9, 10, 11, 12, 13,
    14, 15,
];

/// Decode one 8-bit, `NEAR = 0`, three-component sample-interleaved scan.
///
/// `rows` and `columns` are the SOF55 dimensions already checked against the
/// descriptor. The returned buffer is interleaved RGB in row-major order.
pub(super) fn decode(scan: &[u8], rows: u16, columns: u16) -> Result<Vec<u8>, CodecError> {
    let rows = usize::from(rows);
    let columns = usize::from(columns);
    if rows == 0 || columns == 0 {
        return Err(CodecError::InvalidCodestream);
    }
    let sample_count = rows
        .checked_mul(columns)
        .and_then(|pixels| pixels.checked_mul(COMPONENTS))
        .ok_or(CodecError::InvalidCodestream)?;
    let line = columns
        .checked_add(2)
        .ok_or(CodecError::InvalidCodestream)?;
    let mut reader = Bits::new(scan);
    let mut regular = [Regular::new(); CONTEXTS];
    let mut run = RunContext::new();
    let mut run_index = 0usize;
    let mut previous = vec![[0u8; COMPONENTS]; line];
    let mut current = vec![[0u8; COMPONENTS]; line];
    let mut out = vec![0u8; sample_count];
    let mut offset = 0usize;

    for _row in 0..rows {
        let last = read_pixel(&previous, columns)?;
        write_pixel(&mut previous, columns + 1, last)?;
        let first = read_pixel(&previous, 1)?;
        write_pixel(&mut current, 0, first)?;
        let mut column = 0usize;
        while column < columns {
            let neighbors = Neighbors {
                left: read_pixel(&current, column)?,
                above_left: read_pixel(&previous, column)?,
                above: read_pixel(&previous, column + 1)?,
                above_right: read_pixel(&previous, column + 2)?,
            };
            let gradients = gradients(&neighbors)?;
            if gradients == [0, 0, 0] {
                column = advance_run(
                    &mut reader,
                    &mut RunStep {
                        run: &mut run,
                        run_index: &mut run_index,
                        current: &mut current,
                        previous: &previous,
                        column,
                        columns,
                        value: neighbors.left,
                    },
                )?;
                continue;
            }
            let pixel = decode_regular(&mut reader, &mut regular, &neighbors, gradients)?;
            write_pixel(&mut current, column + 1, pixel)?;
            column += 1;
        }
        for index in 0..columns {
            let pixel = read_pixel(&current, index + 1)?;
            write_rgb(&mut out, offset, pixel)?;
            offset = offset
                .checked_add(COMPONENTS)
                .ok_or(CodecError::InvalidCodestream)?;
        }
        std::mem::swap(&mut previous, &mut current);
    }
    if offset != out.len() {
        return Err(CodecError::InvalidCodestream);
    }
    Ok(out)
}

struct Neighbors {
    left: [u8; COMPONENTS],
    above: [u8; COMPONENTS],
    above_left: [u8; COMPONENTS],
    above_right: [u8; COMPONENTS],
}

fn gradients(neighbors: &Neighbors) -> Result<[i32; COMPONENTS], CodecError> {
    let mut values = [0i32; COMPONENTS];
    for component in 0..COMPONENTS {
        let left = component_value(neighbors.left, component)?;
        let above = component_value(neighbors.above, component)?;
        let above_left = component_value(neighbors.above_left, component)?;
        let above_right = component_value(neighbors.above_right, component)?;
        let ident = context_id(
            quantize(above_right - above),
            quantize(above - above_left),
            quantize(above_left - left),
        )?;
        let slot = values
            .get_mut(component)
            .ok_or(CodecError::InvalidCodestream)?;
        *slot = ident;
    }
    Ok(values)
}

/// The line pair and run state for one run-mode step.
struct RunStep<'a> {
    run: &'a mut RunContext,
    run_index: &'a mut usize,
    current: &'a mut [[u8; COMPONENTS]],
    previous: &'a [[u8; COMPONENTS]],
    column: usize,
    columns: usize,
    value: [u8; COMPONENTS],
}

fn advance_run(reader: &mut Bits<'_>, step: &mut RunStep<'_>) -> Result<usize, CodecError> {
    let remaining = step
        .columns
        .checked_sub(step.column)
        .ok_or(CodecError::InvalidCodestream)?;
    let mut filled = 0usize;
    // A full `J` segment advances the run index, including one that ends the
    // line. A one-bit that only fills the remainder does not.
    while reader.bit()? == 1 {
        let segment = 1usize
            .checked_shl(segment_order(*step.run_index)?)
            .ok_or(CodecError::InvalidCodestream)?;
        let room = remaining
            .checked_sub(filled)
            .ok_or(CodecError::InvalidCodestream)?;
        let count = segment.min(room);
        filled = filled
            .checked_add(count)
            .ok_or(CodecError::InvalidCodestream)?;
        if count == segment && *step.run_index < 31 {
            *step.run_index += 1;
        }
        if filled == remaining {
            break;
        }
    }
    if filled != remaining {
        let extra = reader.read(segment_order(*step.run_index)?)?;
        filled = filled
            .checked_add(usize::try_from(extra).map_err(|_| CodecError::InvalidCodestream)?)
            .ok_or(CodecError::InvalidCodestream)?;
        if filled > remaining {
            return Err(CodecError::InvalidCodestream);
        }
    }
    for offset in 0..filled {
        let index = step
            .column
            .checked_add(1)
            .and_then(|base| base.checked_add(offset))
            .ok_or(CodecError::InvalidCodestream)?;
        write_pixel(step.current, index, step.value)?;
    }
    let column = step
        .column
        .checked_add(filled)
        .ok_or(CodecError::InvalidCodestream)?;
    if column == step.columns {
        return Ok(column);
    }
    let above = read_pixel(step.previous, column + 1)?;
    let order = segment_order(*step.run_index)?;
    let limit = LIMIT
        .checked_sub(order)
        .and_then(|value| value.checked_sub(1))
        .ok_or(CodecError::InvalidCodestream)?;
    let mut pixel = [0u8; COMPONENTS];
    for component in 0..COMPONENTS {
        let k = step.run.golomb_k()?;
        let mapped = reader.golomb(k, limit)?;
        let error = step.run.unmap(mapped, k)?;
        step.run.update(error, mapped)?;
        let above_value = component_value(above, component)?;
        let left_value = component_value(step.value, component)?;
        let sign = if above_value < left_value { -1 } else { 1 };
        let value = reconstruct(above_value, error * sign)?;
        pixel = with_component(pixel, component, value)?;
    }
    write_pixel(step.current, column + 1, pixel)?;
    if *step.run_index > 0 {
        *step.run_index -= 1;
    }
    column.checked_add(1).ok_or(CodecError::InvalidCodestream)
}

fn decode_regular(
    reader: &mut Bits<'_>,
    regular: &mut [Regular; CONTEXTS],
    neighbors: &Neighbors,
    gradients: [i32; COMPONENTS],
) -> Result<[u8; COMPONENTS], CodecError> {
    let mut pixel = [0u8; COMPONENTS];
    for component in 0..COMPONENTS {
        let ident = *gradients
            .get(component)
            .ok_or(CodecError::InvalidCodestream)?;
        let negative = ident < 0;
        let index =
            usize::try_from(ident.unsigned_abs()).map_err(|_| CodecError::InvalidCodestream)?;
        let slot = regular
            .get_mut(index)
            .ok_or(CodecError::InvalidCodestream)?;
        let k = slot.golomb_k()?;
        let bias = if negative {
            slot.c.checked_neg()
        } else {
            Some(slot.c)
        }
        .ok_or(CodecError::InvalidCodestream)?;
        let predicted = predict(
            component_value(neighbors.left, component)?,
            component_value(neighbors.above, component)?,
            component_value(neighbors.above_left, component)?,
        )
        .checked_add(bias)
        .ok_or(CodecError::InvalidCodestream)?
        .clamp(0, MAX_SAMPLE);
        let mapped = reader.golomb(k, LIMIT)?;
        let mut error = unmap_regular(mapped)?;
        if k == 0 && slot.mapping_inverted()? {
            error = invert_mapping(error)?;
        }
        slot.update(error)?;
        if negative {
            error = error.checked_neg().ok_or(CodecError::InvalidCodestream)?;
        }
        pixel = with_component(pixel, component, reconstruct(predicted, error)?)?;
    }
    Ok(pixel)
}

fn quantize(gradient: i32) -> i32 {
    if gradient <= -T3 {
        -4
    } else if gradient <= -T2 {
        -3
    } else if gradient <= -T1 {
        -2
    } else if gradient < 0 {
        -1
    } else if gradient == 0 {
        0
    } else if gradient < T1 {
        1
    } else if gradient < T2 {
        2
    } else if gradient < T3 {
        3
    } else {
        4
    }
}

fn predict(left: i32, above: i32, diagonal: i32) -> i32 {
    let low = left.min(above);
    let high = left.max(above);
    if diagonal >= high {
        low
    } else if diagonal <= low {
        high
    } else {
        left + above - diagonal
    }
}

fn context_id(first: i32, second: i32, third: i32) -> Result<i32, CodecError> {
    first
        .checked_mul(9)
        .and_then(|value| value.checked_add(second))
        .and_then(|value| value.checked_mul(9))
        .and_then(|value| value.checked_add(third))
        .ok_or(CodecError::InvalidCodestream)
}

fn unmap_regular(mapped: u32) -> Result<i32, CodecError> {
    if mapped & 1 == 0 {
        i32::try_from(mapped / 2).map_err(|_| CodecError::InvalidCodestream)
    } else {
        let magnitude = mapped
            .checked_add(1)
            .map(|value| value / 2)
            .ok_or(CodecError::InvalidCodestream)?;
        i32::try_from(magnitude)
            .map(|value| -value)
            .map_err(|_| CodecError::InvalidCodestream)
    }
}

fn invert_mapping(error: i32) -> Result<i32, CodecError> {
    error
        .checked_neg()
        .and_then(|value| value.checked_sub(1))
        .ok_or(CodecError::InvalidCodestream)
}

fn reconstruct(predicted: i32, error: i32) -> Result<u8, CodecError> {
    let wrapped = predicted.wrapping_add(error) & 0xFF;
    u8::try_from(wrapped).map_err(|_| CodecError::InvalidCodestream)
}

fn segment_order(run_index: usize) -> Result<u32, CodecError> {
    J.get(run_index)
        .copied()
        .ok_or(CodecError::InvalidCodestream)
}

fn component_value(pixel: [u8; COMPONENTS], component: usize) -> Result<i32, CodecError> {
    let [red, green, blue] = pixel;
    let value = match component {
        0 => red,
        1 => green,
        2 => blue,
        _ => return Err(CodecError::InvalidCodestream),
    };
    Ok(i32::from(value))
}

fn with_component(
    pixel: [u8; COMPONENTS],
    component: usize,
    value: u8,
) -> Result<[u8; COMPONENTS], CodecError> {
    let [red, green, blue] = pixel;
    Ok(match component {
        0 => [value, green, blue],
        1 => [red, value, blue],
        2 => [red, green, value],
        _ => return Err(CodecError::InvalidCodestream),
    })
}

fn read_pixel(line: &[[u8; COMPONENTS]], index: usize) -> Result<[u8; COMPONENTS], CodecError> {
    line.get(index)
        .copied()
        .ok_or(CodecError::InvalidCodestream)
}

fn write_pixel(
    line: &mut [[u8; COMPONENTS]],
    index: usize,
    pixel: [u8; COMPONENTS],
) -> Result<(), CodecError> {
    let slot = line.get_mut(index).ok_or(CodecError::InvalidCodestream)?;
    *slot = pixel;
    Ok(())
}

fn write_rgb(out: &mut [u8], offset: usize, pixel: [u8; COMPONENTS]) -> Result<(), CodecError> {
    let end = offset
        .checked_add(COMPONENTS)
        .ok_or(CodecError::InvalidCodestream)?;
    let dest = out
        .get_mut(offset..end)
        .ok_or(CodecError::InvalidCodestream)?;
    let [red, green, blue] = pixel;
    let [slot_red, slot_green, slot_blue] = dest else {
        return Err(CodecError::InvalidCodestream);
    };
    *slot_red = red;
    *slot_green = green;
    *slot_blue = blue;
    Ok(())
}

#[derive(Clone, Copy)]
struct Regular {
    a: u32,
    b: i32,
    c: i32,
    n: u32,
}

impl Regular {
    const fn new() -> Self {
        Self {
            a: A_INIT,
            b: 0,
            c: 0,
            n: 1,
        }
    }

    fn golomb_k(self) -> Result<u32, CodecError> {
        let mut k = 0u32;
        while k < 16 {
            let shifted = self.n.checked_shl(k).ok_or(CodecError::InvalidCodestream)?;
            if shifted >= self.a {
                return Ok(k);
            }
            k += 1;
        }
        Err(CodecError::InvalidCodestream)
    }

    fn mapping_inverted(self) -> Result<bool, CodecError> {
        let n = i32::try_from(self.n).map_err(|_| CodecError::InvalidCodestream)?;
        let biased = self
            .b
            .checked_mul(2)
            .and_then(|value| value.checked_add(n))
            .and_then(|value| value.checked_sub(1))
            .ok_or(CodecError::InvalidCodestream)?;
        Ok(biased < 0)
    }

    fn update(&mut self, error: i32) -> Result<(), CodecError> {
        self.a = self
            .a
            .checked_add(error.unsigned_abs())
            .ok_or(CodecError::InvalidCodestream)?;
        self.b = self
            .b
            .checked_add(error)
            .ok_or(CodecError::InvalidCodestream)?;
        if self.n == RESET {
            self.a >>= 1;
            self.b >>= 1;
            self.n >>= 1;
        }
        self.n = self.n.checked_add(1).ok_or(CodecError::InvalidCodestream)?;
        let n = i32::try_from(self.n).map_err(|_| CodecError::InvalidCodestream)?;
        let low = self.b.checked_add(n).ok_or(CodecError::InvalidCodestream)?;
        if low <= 0 {
            self.b = low;
            let floor = n.checked_neg().ok_or(CodecError::InvalidCodestream)?;
            if self.b <= floor {
                self.b = floor.checked_add(1).ok_or(CodecError::InvalidCodestream)?;
            }
            if self.c > -128 {
                self.c -= 1;
            }
        } else if self.b > 0 {
            self.b = self.b.checked_sub(n).ok_or(CodecError::InvalidCodestream)?;
            if self.b > 0 {
                self.b = 0;
            }
            if self.c < 127 {
                self.c += 1;
            }
        }
        Ok(())
    }
}

struct RunContext {
    a: u32,
    n: u32,
    nn: u32,
}

impl RunContext {
    const fn new() -> Self {
        Self {
            a: A_INIT,
            n: 1,
            nn: 0,
        }
    }

    fn golomb_k(&self) -> Result<u32, CodecError> {
        let mut n_test = self.n;
        let mut k = 0u32;
        while n_test < self.a {
            n_test = n_test.checked_shl(1).ok_or(CodecError::InvalidCodestream)?;
            k += 1;
            if k > 32 {
                return Err(CodecError::InvalidCodestream);
            }
        }
        Ok(k)
    }

    fn unmap(&self, mapped: u32, k: u32) -> Result<i32, CodecError> {
        let odd = mapped & 1 == 1;
        let magnitude = if odd {
            mapped
                .checked_add(1)
                .map(|value| value / 2)
                .ok_or(CodecError::InvalidCodestream)?
        } else {
            mapped / 2
        };
        let magnitude = i32::try_from(magnitude).map_err(|_| CodecError::InvalidCodestream)?;
        let n = i32::try_from(self.n).map_err(|_| CodecError::InvalidCodestream)?;
        let nn = i32::try_from(self.nn).map_err(|_| CodecError::InvalidCodestream)?;
        let negative_is_odd =
            k != 0 || nn.checked_mul(2).ok_or(CodecError::InvalidCodestream)? >= n;
        if negative_is_odd == odd {
            Ok(-magnitude)
        } else {
            Ok(magnitude)
        }
    }

    fn update(&mut self, error: i32, mapped: u32) -> Result<(), CodecError> {
        if error < 0 {
            self.nn = self
                .nn
                .checked_add(1)
                .ok_or(CodecError::InvalidCodestream)?;
        }
        let contribution = mapped
            .checked_add(1)
            .map(|value| value >> 1)
            .ok_or(CodecError::InvalidCodestream)?;
        self.a = self
            .a
            .checked_add(contribution)
            .ok_or(CodecError::InvalidCodestream)?;
        if self.n == RESET {
            self.a >>= 1;
            self.n >>= 1;
            self.nn >>= 1;
        }
        self.n = self.n.checked_add(1).ok_or(CodecError::InvalidCodestream)?;
        Ok(())
    }
}

struct Bits<'a> {
    data: &'a [u8],
    pos: usize,
    buf: u64,
    bits: u32,
}

impl<'a> Bits<'a> {
    const fn new(data: &'a [u8]) -> Self {
        Self {
            data,
            pos: 0,
            buf: 0,
            bits: 0,
        }
    }

    fn push(&mut self, value: u32, count: u32) -> Result<(), CodecError> {
        let shifted = self
            .buf
            .checked_shl(count)
            .ok_or(CodecError::InvalidCodestream)?;
        let mask = 1u64
            .checked_shl(count)
            .map(|value| value.wrapping_sub(1))
            .ok_or(CodecError::InvalidCodestream)?;
        self.buf = shifted | (u64::from(value) & mask);
        self.bits = self
            .bits
            .checked_add(count)
            .ok_or(CodecError::InvalidCodestream)?;
        Ok(())
    }

    fn refill(&mut self) -> Result<(), CodecError> {
        // `0xFF` followed by a clear high bit is stuffed data: eight bits, then
        // the low seven of the next byte. A set high bit is a marker, so stop
        // and leave both bytes unread.
        while self.bits <= 16 {
            let Some(byte) = self.data.get(self.pos).copied() else {
                return Ok(());
            };
            if byte == 0xFF
                && self
                    .data
                    .get(self.pos + 1)
                    .is_some_and(|next| next & 0x80 != 0)
            {
                return Ok(());
            }
            self.pos += 1;
            self.push(u32::from(byte), 8)?;
            if byte == 0xFF
                && let Some(stuffed) = self.data.get(self.pos).copied()
                && stuffed & 0x80 == 0
            {
                self.pos += 1;
                self.push(u32::from(stuffed & 0x7F), 7)?;
            }
        }
        Ok(())
    }

    fn read(&mut self, count: u32) -> Result<u32, CodecError> {
        if count == 0 {
            return Ok(0);
        }
        if count >= 32 {
            return Err(CodecError::InvalidCodestream);
        }
        if self.bits < count {
            self.refill()?;
        }
        if self.bits < count {
            return Err(CodecError::InvalidCodestream);
        }
        self.bits -= count;
        let mask = 1u64
            .checked_shl(count)
            .map(|value| value.wrapping_sub(1))
            .ok_or(CodecError::InvalidCodestream)?;
        u32::try_from((self.buf >> self.bits) & mask).map_err(|_| CodecError::InvalidCodestream)
    }

    fn bit(&mut self) -> Result<u32, CodecError> {
        self.read(1)
    }

    fn high_bits(&mut self) -> Result<u32, CodecError> {
        let mut count = 0u32;
        while self.bit()? == 0 {
            count = count.checked_add(1).ok_or(CodecError::InvalidCodestream)?;
            if count > 64 {
                return Err(CodecError::InvalidCodestream);
            }
        }
        Ok(count)
    }

    fn golomb(&mut self, k: u32, limit: u32) -> Result<u32, CodecError> {
        let high = self.high_bits()?;
        let threshold = limit
            .checked_sub(QBPP + 1)
            .ok_or(CodecError::InvalidCodestream)?;
        if high >= threshold {
            return self
                .read(QBPP)?
                .checked_add(1)
                .ok_or(CodecError::InvalidCodestream);
        }
        if k == 0 {
            return Ok(high);
        }
        let remainder = self.read(k)?;
        high.checked_shl(k)
            .and_then(|value| value.checked_add(remainder))
            .ok_or(CodecError::InvalidCodestream)
    }
}
