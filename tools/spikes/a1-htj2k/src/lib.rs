//! THROWAWAY SPIKE CODE. F-X006, Appendix A gate A1.
//!
//! Decodes the three HTJ2K corpus codestreams through `openjp2` and hands the
//! canonical 12288 bytes back to a caller. Two callers exist: `src/main.rs`,
//! which is the native decode, and `run.mjs`, which instantiates the
//! `wasm32-unknown-unknown` build under node. Both reach the SAME function, so
//! the only difference between the two measurements is the target.
//!
//! Not held to the gate set, per `/spike` step 2. It is not a workspace member,
//! so `gate clippy` and `gate test` never see it. `gate unsafe`,
//! `gate provenance` and `gate content` read `git ls-files` and do see it, and
//! `gate lint` sees `run.mjs`. It is written to pass all four.
//!
//! `gate prose` does NOT reach this directory, and an earlier revision of this
//! comment said it did. `scripts/prose_check.py` includes a path only if it is
//! in `INCLUDE_EXACT` or it ends in `.md` and starts with one of its
//! `INCLUDE_PREFIXES`. `tools/spikes/` is on neither list and holds no `.md`,
//! so an em-dash here is unchecked. `docs/spikes/A1-htj2k-openjp2.md` IS
//! covered, which is a different fact.
//!
//! # THREE THINGS THAT ARE DELIBERATE AND LOOK LIKE OVERSIGHTS
//!
//! **The dependency is `jpeg2k` and the library under test is still `openjp2`.**
//! `openjp2` 0.6.1 exposes no safe in-memory stream. `Stream::new_file` is
//! behind its `file-io` feature, which needs `libc` and a filesystem, and the
//! only other route is `Stream::new_custom` with `extern "C"` callbacks and a
//! `*mut c_void` user pointer, which cannot be dereferenced without `unsafe` in
//! a tracked file. `jpeg2k` with `default-features = false` and
//! `features = ["openjp2"]` is a thin safe wrapper whose only JPEG 2000
//! dependency is `openjp2 = 0.6.1`, and it is exactly what
//! `dicom-transfer-syntax-registry` 0.10's `openjp2` feature resolves to. So
//! the code being measured is the code A1 names, reached the way the codec
//! registry would reach it. `cargo tree` output is recorded in the answer file.
//!
//! **No pointer is ever dereferenced in Rust, and there is no `as` cast.** The
//! decoded buffer lives in a `thread_local!` `RefCell<Vec<u8>>`, so there is no
//! `static mut`. The wasm driver reads it with
//! `new Uint8Array(instance.exports.memory.buffer, out_ptr(), out_len())`,
//! which is ordinary JavaScript, and asserts `out_len()` is 12288 first.
//!
//! **Errors leave as raw integers.** `CodecError` is F-005's to shape and
//! E2.6's to populate, and this spike must not pre-empt either.

// The C allocator `openjp2` 0.6.1 needs and `wasm32-unknown-unknown` does not
// have. See `wasm_libc.rs`. Native targets get it from the system libc.
#[cfg(target_arch = "wasm32")]
mod wasm_libc;

use core::cell::RefCell;

/// `1.2.840.10008.1.2.4.201`, HTJ2K reversible, LRCP.
pub const CASE_LOSSLESS: u32 = 0;
/// `1.2.840.10008.1.2.4.202`, HTJ2K reversible, RPCL.
pub const CASE_LOSSLESS_RPCL: u32 = 1;
/// `1.2.840.10008.1.2.4.203`, HTJ2K irreversible.
pub const CASE_LOSSY: u32 = 2;
/// `1.2.840.10008.1.2.4.90`, JPEG 2000 Part 1 reversible. A CONTROL, not part
/// of A1's question. It runs the same build through a different code path, and
/// it is what separates "openjp2 does not work on wasm32" from "openjp2's
/// HTJ2K path does not work on wasm32".
pub const CASE_J2K_LOSSLESS: u32 = 3;
/// `1.2.840.10008.1.2.4.91`, JPEG 2000 Part 1 irreversible. A CONTROL.
pub const CASE_J2K_LOSSY: u32 = 4;

/// Raw decoder error codes. Not `CodecError`, deliberately. See the header.
pub const OK: u32 = 0;
pub const ERR_UNKNOWN_CASE: u32 = 1;
pub const ERR_DECODE_REFUSED: u32 = 2;
pub const ERR_COMPONENT_COUNT: u32 = 3;
pub const ERR_GEOMETRY: u32 = 4;
pub const ERR_SAMPLE_COUNT: u32 = 5;
pub const ERR_SAMPLE_RANGE: u32 = 6;

/// The canonical form, from `scripts/corpus_synth.py`: 64 rows by 96 columns,
/// unsigned 16-bit, so 6144 samples and 12288 little-endian bytes.
pub const ROWS: u32 = 64;
pub const COLS: u32 = 96;
pub const SAMPLES: usize = 6144;
pub const CANONICAL_BYTES: usize = 12288;

// Embedded at compile time from ignored `tools/spikes/out/`, written by
// `tools/spikes/common/extract.py`. Nothing is written into linear memory from
// outside, so nothing needs an inbound pointer.
const HTJ2K_LOSSLESS: &[u8] = include_bytes!("../../out/htj2k_lossless.j2c");
const HTJ2K_LOSSLESS_RPCL: &[u8] =
    include_bytes!("../../out/htj2k_lossless_rpcl.j2c");
const HTJ2K_LOSSY: &[u8] = include_bytes!("../../out/htj2k_lossy.j2c");
const J2K_LOSSLESS: &[u8] = include_bytes!("../../out/j2k_lossless.j2c");
const J2K_LOSSY: &[u8] = include_bytes!("../../out/j2k_lossy.j2c");

thread_local! {
    static OUT: RefCell<Vec<u8>> = const { RefCell::new(Vec::new()) };
}

fn codestream(case: u32) -> Option<&'static [u8]> {
    match case {
        CASE_LOSSLESS => Some(HTJ2K_LOSSLESS),
        CASE_LOSSLESS_RPCL => Some(HTJ2K_LOSSLESS_RPCL),
        CASE_LOSSY => Some(HTJ2K_LOSSY),
        CASE_J2K_LOSSLESS => Some(J2K_LOSSLESS),
        CASE_J2K_LOSSY => Some(J2K_LOSSY),
        _ => None,
    }
}

/// Reduce a decoded image to the canonical little-endian bytes.
///
/// Every shape assumption is checked and reported as a distinct code, because a
/// geometry surprise presents as a total mismatch and would read as a decoder
/// defect. No arithmetic is applied to a sample: `openjp2` returns one `i32`
/// per component sample and each is converted by `u16::try_from`, which refuses
/// rather than truncating. A lossy 9/7 reconstruction that overshot the
/// component range is therefore reported, never silently clamped.
fn to_canonical(image: &jpeg2k::Image) -> Result<Vec<u8>, u32> {
    let components = image.components();
    if components.len() != 1 {
        return Err(ERR_COMPONENT_COUNT);
    }
    let component = &components[0];
    if component.width() != COLS || component.height() != ROWS {
        return Err(ERR_GEOMETRY);
    }

    let data = component.data();
    if data.len() != SAMPLES {
        return Err(ERR_SAMPLE_COUNT);
    }

    let mut out = Vec::with_capacity(CANONICAL_BYTES);
    for sample in data {
        let value = u16::try_from(*sample).map_err(|_| ERR_SAMPLE_RANGE)?;
        out.extend_from_slice(&value.to_le_bytes());
    }
    Ok(out)
}

/// Decode one embedded codestream. The native caller.
pub fn decode_case(case: u32) -> Result<Vec<u8>, u32> {
    let src = codestream(case).ok_or(ERR_UNKNOWN_CASE)?;
    let image = jpeg2k::Image::from_bytes(src).map_err(|_| ERR_DECODE_REFUSED)?;
    to_canonical(&image)
}

/// Decode into the module-owned buffer. Returns `OK` or an error code.
///
/// **On `wasm32-unknown-unknown` this function does not return.** `openjp2`
/// 0.6.1 traps inside `jpeg2k::Image::from_bytes`, during the tile teardown
/// that `from_bytes` performs before it hands back an image: `src/tcd.rs` calls
/// `std::alloc::dealloc` on `decoded_data` with no null check, at lines 1432
/// and 2510. A host allocator tolerates a null free. The `dlmalloc` behind
/// `wasm32-unknown-unknown` does not, and the module traps with
/// `memory access out of bounds`.
///
/// The store into `OUT` is written after the decode and before this function
/// returns, so a trap anywhere in the decode leaves `out_len()` at zero. The
/// driver reads that zero and reports NO PIXELS rather than reading whatever
/// happened to be at `out_ptr()`. A gate answer built on bytes left behind by a
/// trapped decode would be worse than no answer.
#[no_mangle]
pub extern "C" fn decode(case: u32) -> u32 {
    OUT.with(|cell| {
        cell.borrow_mut().clear();
    });
    let Some(src) = codestream(case) else {
        return ERR_UNKNOWN_CASE;
    };
    let Ok(image) = jpeg2k::Image::from_bytes(src) else {
        return ERR_DECODE_REFUSED;
    };
    match to_canonical(&image) {
        Ok(bytes) => {
            OUT.with(|cell| {
                *cell.borrow_mut() = bytes;
            });
            OK
        }
        Err(code) => code,
    }
}

/// Length of the decoded buffer. The driver asserts this is 12288.
#[no_mangle]
pub extern "C" fn out_len() -> u32 {
    OUT.with(|cell| u32::try_from(cell.borrow().len()).unwrap_or(0))
}

/// Offset of the decoded buffer in linear memory.
///
/// `<*const u8>::addr()` and `u32::try_from`, not an `as` cast. On
/// `wasm32-unknown-unknown` a `usize` is 32 bits so the conversion cannot lose
/// anything. On a 64-bit host it can fail, and it returns 0 rather than a
/// truncated address. The driver refuses a zero pointer, and the native caller
/// uses `decode_case` directly and never reaches here.
#[no_mangle]
pub extern "C" fn out_ptr() -> u32 {
    OUT.with(|cell| u32::try_from(cell.borrow().as_ptr().addr()).unwrap_or(0))
}
