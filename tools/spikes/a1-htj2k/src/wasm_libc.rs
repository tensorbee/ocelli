//! THROWAWAY SPIKE CODE. F-X006, Appendix A gate A1.
//!
//! `openjp2` 0.6.1 is a C2Rust port and its `src/malloc.rs` declares
//! `extern "C" { malloc, calloc, realloc, free, memcpy }` with no `cfg`. On a
//! native target the system libc supplies them. `wasm32-unknown-unknown` has no
//! libc, so they are undefined and the module does not link.
//!
//! This file is the smallest thing that tests whether supplying them from the
//! DOWNSTREAM crate is enough. It is a measurement, not a proposed fix, and the
//! answer file records what it showed.
//!
//! **It contains no `unsafe`, and that is the whole trick.** A conventional
//! shim over `std::alloc` needs raw-pointer writes. This one keeps every
//! allocation as an owned `Box<[u64]>` in a side table keyed by address, so
//! `realloc` copies between two owned slices with `copy_from_slice` and never
//! reads through a raw pointer. `u64` and not `u8` because C code stores
//! `int32_t` and pointers in these blocks and a `Box<[u8]>` guarantees only
//! byte alignment.
//!
//! `free` drops the entry. `memcpy` is not defined here because
//! `compiler_builtins` already supplies it on this target, which the link error
//! confirms by not naming it.

use core::cell::RefCell;
use core::ffi::c_void;
use core::ptr;
use std::collections::HashMap;

thread_local! {
    /// address -> (requested byte size, the owned block)
    static BLOCKS: RefCell<HashMap<usize, (usize, Box<[u64]>)>> =
        RefCell::new(HashMap::new());
}

/// Whole 8-byte words needed to hold `bytes`, at least one.
fn words(bytes: usize) -> usize {
    bytes.div_ceil(8).max(1)
}

/// Allocate a zeroed, 8-byte-aligned block and record it. Returns null on a
/// zero request, which is what `openjp2`'s own wrappers already expect.
fn allocate(bytes: usize) -> *mut c_void {
    if bytes == 0 {
        return ptr::null_mut();
    }
    let mut block: Box<[u64]> = vec![0_u64; words(bytes)].into_boxed_slice();
    let address = block.as_mut_ptr().expose_provenance();
    BLOCKS.with(|table| {
        table.borrow_mut().insert(address, (bytes, block));
    });
    ptr::with_exposed_provenance_mut::<c_void>(address)
}

#[no_mangle]
pub extern "C" fn malloc(size: usize) -> *mut c_void {
    allocate(size)
}

#[no_mangle]
pub extern "C" fn calloc(count: usize, size: usize) -> *mut c_void {
    match count.checked_mul(size) {
        Some(bytes) => allocate(bytes),
        None => ptr::null_mut(),
    }
}

#[no_mangle]
pub extern "C" fn free(block: *mut c_void) {
    if block.is_null() {
        return;
    }
    let address = block.addr();
    BLOCKS.with(|table| {
        table.borrow_mut().remove(&address);
    });
}

#[no_mangle]
pub extern "C" fn realloc(block: *mut c_void, size: usize) -> *mut c_void {
    if block.is_null() {
        return allocate(size);
    }
    let address = block.addr();
    let taken = BLOCKS.with(|table| table.borrow_mut().remove(&address));
    let Some((old_bytes, old)) = taken else {
        // Not ours. Refusing is the only honest answer and a decode that
        // reaches here is reported as a failure rather than corrupted.
        return ptr::null_mut();
    };
    if size == 0 {
        return ptr::null_mut();
    }
    let mut new: Box<[u64]> = vec![0_u64; words(size)].into_boxed_slice();
    let carried = words(old_bytes.min(size));
    new[..carried].copy_from_slice(&old[..carried]);
    let new_address = new.as_mut_ptr().expose_provenance();
    BLOCKS.with(|table| {
        table.borrow_mut().insert(new_address, (size, new));
    });
    ptr::with_exposed_provenance_mut::<c_void>(new_address)
}
