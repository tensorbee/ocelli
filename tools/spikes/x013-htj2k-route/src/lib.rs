//! THROWAWAY SPIKE CODE. F-X013, the pure-Rust HTJ2K route measurement.
//!
//! This is deliberately not a production `Decoder`. It embeds three extracted
//! synthetic codestreams, decodes them through openjph-core 0.1.0, and exposes
//! one integer ABI so native and wasm execute the same candidate code.

use core::cell::RefCell;

use openjph_core::codestream::Codestream;
use openjph_core::file::MemInfile;

pub const CASE_LOSSLESS: u32 = 0;
pub const CASE_LOSSLESS_RPCL: u32 = 1;
pub const CASE_LOSSY: u32 = 2;

pub const OK: u32 = 0;
pub const ERR_UNKNOWN_CASE: u32 = 1;
pub const ERR_HEADER: u32 = 2;
pub const ERR_GEOMETRY: u32 = 3;
pub const ERR_DECODE: u32 = 4;
pub const ERR_LINE_COUNT: u32 = 5;
pub const ERR_LINE_WIDTH: u32 = 6;
pub const ERR_SAMPLE_RANGE: u32 = 7;
pub const ERR_BYTE_COUNT: u32 = 8;

pub const ROWS: u32 = 64;
pub const COLS: u32 = 96;
pub const CANONICAL_BYTES: usize = 12_288;

const LOSSLESS: &[u8] = include_bytes!("../../out/htj2k_lossless.j2c");
const LOSSLESS_RPCL: &[u8] = include_bytes!("../../out/htj2k_lossless_rpcl.j2c");
const LOSSY: &[u8] = include_bytes!("../../out/htj2k_lossy.j2c");

thread_local! {
    static OUT: RefCell<Vec<u8>> = const { RefCell::new(Vec::new()) };
}

fn codestream(case: u32) -> Option<&'static [u8]> {
    match case {
        CASE_LOSSLESS => Some(LOSSLESS),
        CASE_LOSSLESS_RPCL => Some(LOSSLESS_RPCL),
        CASE_LOSSY => Some(LOSSY),
        _ => None,
    }
}

/// Decode one synthetic codestream into canonical little-endian `u16` bytes.
pub fn decode_case(case: u32) -> Result<Vec<u8>, u32> {
    let source = codestream(case).ok_or(ERR_UNKNOWN_CASE)?;
    let mut input = MemInfile::new(source);
    let mut decoder = Codestream::new();
    decoder.read_headers(&mut input).map_err(|_| ERR_HEADER)?;

    let siz = decoder.access_siz();
    if siz.get_num_components() != 1
        || siz.get_recon_width(0) != COLS
        || siz.get_recon_height(0) != ROWS
        || siz.get_bit_depth(0) != 16
        || siz.is_signed(0)
    {
        return Err(ERR_GEOMETRY);
    }

    decoder.create(&mut input).map_err(|_| ERR_DECODE)?;
    let mut output = Vec::with_capacity(CANONICAL_BYTES);
    for _ in 0..ROWS {
        let line = decoder.pull(0).ok_or(ERR_LINE_COUNT)?;
        if line.len() != usize::try_from(COLS).map_err(|_| ERR_LINE_WIDTH)? {
            return Err(ERR_LINE_WIDTH);
        }
        for value in line {
            let sample = u16::try_from(value).map_err(|_| ERR_SAMPLE_RANGE)?;
            output.extend_from_slice(&sample.to_le_bytes());
        }
    }
    if decoder.pull(0).is_some() {
        return Err(ERR_LINE_COUNT);
    }
    if output.len() != CANONICAL_BYTES {
        return Err(ERR_BYTE_COUNT);
    }
    Ok(output)
}

/// Decode into the module-owned buffer and return a stable raw error code.
#[no_mangle]
pub extern "C" fn decode(case: u32) -> u32 {
    OUT.with(|cell| cell.borrow_mut().clear());
    match decode_case(case) {
        Ok(bytes) => {
            OUT.with(|cell| *cell.borrow_mut() = bytes);
            OK
        }
        Err(code) => code,
    }
}

#[no_mangle]
pub extern "C" fn out_len() -> u32 {
    OUT.with(|cell| u32::try_from(cell.borrow().len()).unwrap_or(0))
}

#[no_mangle]
pub extern "C" fn out_ptr() -> u32 {
    OUT.with(|cell| u32::try_from(cell.borrow().as_ptr().addr()).unwrap_or(0))
}
