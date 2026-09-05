//! The panic path. HLD section 23, and it is the crux of this story.
//!
//! Section 23's callout, verbatim:
//!
//! > **A PANIC POISONS THE INSTANCE** Once a Rust panic aborts inside
//! > WebAssembly, that module instance's memory may be inconsistent and it
//! > must not be reused. So: panic = "abort" in release, no exported function
//! > may let a panic escape as a normal error path, and on panic the worker
//! > instance is torn down and rebuilt, with the JavaScript shell
//! > reconstructing viewport state from its own copy.
//!
//! ## Recovery is not resumption, and that is measured rather than assumed
//!
//! `rustc --print cfg --target wasm32-unknown-unknown` reports
//! `panic="abort"`. That is the TARGET's default, so it holds in **every**
//! profile and not only in the release profile section 15.2 sets it in.
//! `catch_unwind` compiles on wasm32 and never catches. So a boundary that
//! caught a panic and turned it into an error code is not merely forbidden by
//! section 23, it does not work.
//!
//! What happens instead: the instance traps, and the shell reads a fixed-size
//! record out of linear memory **without calling any export**, marks the
//! viewport in error, discards the worker and builds a new one. "Without
//! poisoning the wasm instance" in the sprint's done line therefore means that
//! the mapping from a panic to a JS error must not itself require touching a
//! poisoned instance.
//!
//! ## Why this file contains no `unsafe`
//!
//! The obvious implementation of a fixed record is a `static mut`, and that
//! would have to be refused: HLD 27.2 R5 allows `unsafe` in
//! `crates/ocelli-wasm/src/ring.rs` and `crates/ocelli-core/src/cast.rs`, and
//! this is neither. A `static` written through `AtomicU8::store` needs only a
//! shared reference, so the hook mutates a static with no `unsafe`, no
//! `Mutex` that could deadlock inside a panic, and no `UnsafeCell`.
//!
//! The address is taken with `core::ptr::from_ref` and `<*const T>::addr()`,
//! which gives an address with no pointer-to-integer cast, then narrowed with
//! `u32::try_from`, which gives the narrowing with no
//! `cast_possible_truncation`.

use core::fmt::Write as _;
use core::sync::atomic::{AtomicU8, AtomicU32, Ordering};

use ocelli_core::ErrorCode;

/// Written last, and the shell's only test for whether a record is present.
///
/// The four ASCII bytes `OCP1`, little-endian, so a hex dump of linear memory
/// is readable and so an accidental small integer sitting in this word does
/// not read as a panic. `0` means no panic has been recorded.
pub const PANIC_MAGIC: u32 = u32::from_le_bytes(*b"OCP1");

/// The layout version. Bumped when the field order below changes, so an older
/// shell reading a newer record can say so rather than mis-parse it.
pub const PANIC_RECORD_VERSION: u32 = 1;

/// Message bytes the record holds. A longer message is truncated.
pub const MESSAGE_CAPACITY: usize = 512;

/// Header bytes before the message: magic, version, code, length.
pub const HEADER_BYTES: usize = 16;

/// The whole record, in bytes. `panic_record_len` returns this.
pub const PANIC_RECORD_BYTES: usize = HEADER_BYTES + MESSAGE_CAPACITY;

/// The record, as one `#[repr(C)]` static.
///
/// **One struct and not five statics.** Rust gives no layout guarantee across
/// separate statics, so five of them could sit anywhere in linear memory
/// relative to each other and the single cached pointer the shell holds would
/// address only the first. `#[repr(C)]` over four `AtomicU32` and then the
/// message array fixes the offsets:
///
/// ```text
/// offset  size  field
/// 0       4     magic     u32, 0 until a panic, then PANIC_MAGIC
/// 4       4     version   u32, PANIC_RECORD_VERSION
/// 8       4     code      u32, an ocelli_core::ErrorCode number
/// 12      4     msg_len   u32, bytes written, at most MESSAGE_CAPACITY
/// 16      512   msg       UTF-8, not NUL terminated
///                         total 528
/// ```
#[repr(C)]
struct PanicSlot {
    magic: AtomicU32,
    version: AtomicU32,
    code: AtomicU32,
    msg_len: AtomicU32,
    msg: [AtomicU8; MESSAGE_CAPACITY],
}

static PANIC_SLOT: PanicSlot = PanicSlot {
    magic: AtomicU32::new(0),
    version: AtomicU32::new(0),
    code: AtomicU32::new(0),
    msg_len: AtomicU32::new(0),
    msg: [const { AtomicU8::new(0) }; MESSAGE_CAPACITY],
};

/// A `core::fmt::Write` sink that truncates and returns `Ok`.
///
/// **Returning `Err` on overflow would be a defect here.** `write!` would then
/// return an error the hook has to handle, and the only honest handling inside
/// a panic hook is another panic. So this fills what it can, records that it
/// stopped, and says everything went fine, which is true of everything the
/// caller can act on.
///
/// A truncation can land inside a UTF-8 sequence. The bytes are handed to the
/// shell as bytes and decoded leniently there, which turns a split code point
/// into one replacement character rather than into a thrown exception.
struct MessageBuffer {
    bytes: [u8; MESSAGE_CAPACITY],
    len: usize,
    truncated: bool,
}

impl MessageBuffer {
    const fn new() -> Self {
        Self {
            bytes: [0_u8; MESSAGE_CAPACITY],
            len: 0,
            truncated: false,
        }
    }

    /// The bytes actually written. Never longer than [`MESSAGE_CAPACITY`].
    fn filled(&self) -> &[u8] {
        self.bytes.get(..self.len).unwrap_or(&[])
    }

    /// Whether anything was dropped for want of room.
    ///
    /// Nothing in the hook branches on this today, because a truncated message
    /// is still the message and there is nothing better to do with the fact
    /// inside a panic. The shell derives the same answer from `msg_len`
    /// reaching capacity. It is here so the truncation is a property a test
    /// can assert directly rather than infer.
    #[cfg(test)]
    const fn truncated(&self) -> bool {
        self.truncated
    }
}

impl core::fmt::Write for MessageBuffer {
    fn write_str(&mut self, text: &str) -> core::fmt::Result {
        for byte in text.as_bytes() {
            match self.bytes.get_mut(self.len) {
                Some(slot) => {
                    *slot = *byte;
                    self.len = self.len.saturating_add(1);
                }
                None => {
                    self.truncated = true;
                    return Ok(());
                }
            }
        }
        Ok(())
    }
}

/// Fill the record's body, leaving the magic unwritten.
///
/// Split from [`seal`] so the ordering rule is a thing the tests can drive
/// rather than a comment. A hook that panics part way through leaves the magic
/// at `0`, and the shell then reads the record as **absent** rather than as
/// present with garbage in it.
fn fill(code: u32, message: &MessageBuffer) {
    let filled = message.filled();
    for (slot, byte) in PANIC_SLOT.msg.iter().zip(filled.iter()) {
        slot.store(*byte, Ordering::Relaxed);
    }
    // `filled` is at most MESSAGE_CAPACITY, which is 512, so this conversion
    // cannot fail. `unwrap_or` rather than `expect`, because there is no
    // "this cannot fail" in this crate and a length that somehow did not fit
    // should read as a full buffer rather than end the process.
    let len = u32::try_from(filled.len()).unwrap_or(u32::MAX);
    PANIC_SLOT.msg_len.store(len, Ordering::Relaxed);
    PANIC_SLOT.code.store(code, Ordering::Relaxed);
    PANIC_SLOT
        .version
        .store(PANIC_RECORD_VERSION, Ordering::Relaxed);
}

/// Write the magic. Last, always.
fn seal() {
    PANIC_SLOT.magic.store(PANIC_MAGIC, Ordering::Release);
}

/// Format a panic and store it. The whole of the hook's work.
fn record(info: &std::panic::PanicHookInfo<'_>) {
    let mut message = MessageBuffer::new();

    // `payload()` is `&dyn Any`. The two shapes std ever produces are a
    // `&'static str` for a literal and a `String` for a formatted one.
    let payload = info.payload();
    let text = payload
        .downcast_ref::<&str>()
        .copied()
        .or_else(|| payload.downcast_ref::<String>().map(String::as_str))
        .unwrap_or("panic with a payload this build cannot render");

    // Every `write!` here is infallible, because `MessageBuffer` truncates
    // rather than failing. The results are discarded deliberately.
    let _ = message.write_str(text);
    if let Some(location) = info.location() {
        let _ = write!(
            message,
            " at {}:{}:{}",
            location.file(),
            location.line(),
            location.column()
        );
    }

    fill(ErrorCode::Panicked.number().into(), &message);
    seal();

    #[cfg(all(target_arch = "wasm32", debug_assertions))]
    console_echo(&message);
}

/// The development-only console echo. HLD section 23's first bullet:
///
/// > console_error_panic_hook in development builds only, it costs binary size
/// > and leaks symbol names.
///
/// **That names a behaviour, not a crate.** `console_error_panic_hook` is not
/// in section 15.2's dependency list, and `wasm-bindgen` declares the one
/// import needed with no crate added. Gated on `debug_assertions`, which is
/// what "development builds" means and which needs no feature flag, so the
/// artefact `bin/ocelli.sh wasm` ships carries neither the import nor the
/// symbol names.
///
/// **The record itself is written in both profiles.** The echo is the part
/// that is development-only. A production panic the shell could not describe
/// is exactly the silent blank canvas section 23's last bullet forbids.
#[cfg(all(target_arch = "wasm32", debug_assertions))]
fn console_echo(message: &MessageBuffer) {
    if let Ok(text) = core::str::from_utf8(message.filled()) {
        console::error(text);
    }
}

#[cfg(all(target_arch = "wasm32", debug_assertions))]
mod console {
    use wasm_bindgen::prelude::wasm_bindgen;

    #[wasm_bindgen]
    extern "C" {
        #[wasm_bindgen(js_namespace = console, js_name = error)]
        pub fn error(message: &str);
    }
}

/// Install the panic hook. Called once by a worker before any other call.
///
/// Idempotent in effect: calling it twice replaces the hook with an equivalent
/// one. The one allocation in this module is the `Box` that
/// `std::panic::set_hook` requires, and it happens here, at worker start,
/// before a frame has been rendered.
pub fn install() {
    std::panic::set_hook(Box::new(record));
}

/// The address of the panic record in linear memory.
///
/// The shell calls this immediately after instantiating the module and caches
/// the integer. **It never calls it again**, which is the whole reason this
/// design works: after the trap the shell needs no call into the instance at
/// all, so section 23's "must not be reused" is honoured literally rather than
/// approximately. An export that returned the message after the trap would be
/// reusing a poisoned instance to ask why it was poisoned.
///
/// **A cached pointer is safe and a cached view is not.** Memory growth
/// relocates the JavaScript `ArrayBuffer` and detaches every view over it. It
/// does not move the linear address of a static. So this integer stays correct
/// for the life of the instance, and the `DataView` over it is built fresh at
/// every read. That is HLD section 17.2's rule, and
/// `packages/core/src/panic.ts` is it as code.
///
/// Returns `0` where the address does not fit a `u32`, and the shell treats
/// `0` as "no record available" rather than as a valid address.
///
/// **On a 64-bit host that `0` is the ordinary answer**, because a host
/// address rarely fits a `u32`. That is not a defect and it is not papered
/// over: this export exists for wasm32, where `usize` is 32 bits and the
/// conversion is always exact. The native entry points do not read a record
/// out of linear memory, because they have none. [`record_address`] is what
/// the host tests use.
#[must_use]
pub fn record_ptr() -> u32 {
    u32::try_from(record_address()).unwrap_or(0)
}

/// The record's address as a `usize`, on whatever target this is.
///
/// `core::ptr::from_ref` and `<*const T>::addr()` rather than an `as` cast.
/// The workspace denies `cast_possible_truncation` and HLD section 27.3 makes
/// every cast a human review item, so a design that needs none is worth two
/// extra calls.
#[must_use]
pub fn record_address() -> usize {
    core::ptr::from_ref(&PANIC_SLOT).addr()
}

/// The record's length in bytes. Always [`PANIC_RECORD_BYTES`].
///
/// Read once alongside [`record_ptr`] and cached with it. It is an export
/// rather than a constant on the shell side so that a shell and a core built
/// from different commits disagree loudly at startup instead of quietly at
/// the moment of a panic.
#[must_use]
pub fn record_len() -> u32 {
    u32::try_from(PANIC_RECORD_BYTES).unwrap_or(u32::MAX)
}

#[cfg(test)]
mod tests {
    //! **Host only.** `cargo test` runs the dev profile on the host, where
    //! `panic = "unwind"` and `catch_unwind` therefore works. wasm32 is
    //! `panic = "abort"` in every profile and cannot run any of this, which is
    //! why `scripts/panic_probe.mjs` and the `panic` gate exist: a native test
    //! proves the hook's formatting and nothing at all about the wasm32 trap.

    use core::fmt::Write as _;
    use std::sync::{Mutex, MutexGuard};

    use super::{
        MESSAGE_CAPACITY, MessageBuffer, PANIC_MAGIC, PANIC_RECORD_BYTES, PANIC_RECORD_VERSION,
        PANIC_SLOT, fill, record, record_address, record_len, seal,
    };
    use core::sync::atomic::Ordering;
    use ocelli_core::ErrorCode;

    /// The record is one global, and `cargo test` runs tests in parallel
    /// threads inside one process. Without this every test here would race
    /// every other one, and the failures would be intermittent, which is the
    /// worst kind.
    static SERIAL: Mutex<()> = Mutex::new(());

    /// Take the lock and clear the record. `lock()` returns a `Result` and
    /// `unwrap` is denied, so a poisoned lock is recovered from rather than
    /// escalated: a poisoned lock here means an earlier test in this module
    /// failed, and that test has already reported itself.
    fn exclusive() -> MutexGuard<'static, ()> {
        let guard = match SERIAL.lock() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        };
        PANIC_SLOT.magic.store(0, Ordering::Relaxed);
        PANIC_SLOT.version.store(0, Ordering::Relaxed);
        PANIC_SLOT.code.store(0, Ordering::Relaxed);
        PANIC_SLOT.msg_len.store(0, Ordering::Relaxed);
        for slot in &PANIC_SLOT.msg {
            slot.store(0, Ordering::Relaxed);
        }
        guard
    }

    /// The message bytes currently in the record, copied out.
    fn stored_message() -> Vec<u8> {
        let len = PANIC_SLOT.msg_len.load(Ordering::Relaxed);
        PANIC_SLOT
            .msg
            .iter()
            .take(usize::try_from(len).unwrap_or(0))
            .map(|slot| slot.load(Ordering::Relaxed))
            .collect()
    }

    /// The layout numbers are asserted as literals, because they are a wire
    /// contract with `packages/core/src/panic.ts` and that file asserts the
    /// same numbers. Two implementations, one layout, and neither derived from
    /// the other.
    #[test]
    fn the_record_layout_is_the_one_the_shell_reads() {
        assert_eq!(super::HEADER_BYTES, 16);
        assert_eq!(MESSAGE_CAPACITY, 512);
        assert_eq!(PANIC_RECORD_BYTES, 528);
        assert_eq!(record_len(), 528);
        assert_eq!(PANIC_RECORD_VERSION, 1);
        // "OCP1" little-endian: O=0x4F, C=0x43, P=0x50, 1=0x31, so the u32 is
        // 0x3150_434F. Written out by hand rather than by calling the
        // constructor that produced it.
        assert_eq!(PANIC_MAGIC, 0x3150_434F);
    }

    /// The record has an address. Asserted on `record_address` and not on
    /// `record_ptr`, because on a 64-bit host the `u32` narrowing legitimately
    /// returns 0 and asserting the export would be asserting the target rather
    /// than the record. `scripts/panic_probe.mjs` asserts the export, on
    /// wasm32, where it is the thing that has to be right.
    #[test]
    fn the_record_has_an_address() {
        assert_ne!(record_address(), 0);
    }

    /// The writer truncates at capacity, returns `Ok`, and records the
    /// truncated length. Driven with an input longer than the buffer.
    #[test]
    fn the_message_writer_truncates_and_still_succeeds() {
        let mut buffer = MessageBuffer::new();
        let long = "a".repeat(MESSAGE_CAPACITY + 64);
        assert_eq!(buffer.write_str(&long), Ok(()));
        assert_eq!(buffer.filled().len(), MESSAGE_CAPACITY);
        assert!(buffer.truncated());
        // A second write on a full buffer still succeeds and still adds
        // nothing, because the hook has no way to handle an error.
        assert_eq!(buffer.write_str("more"), Ok(()));
        assert_eq!(buffer.filled().len(), MESSAGE_CAPACITY);
    }

    #[test]
    fn a_short_message_is_not_marked_truncated() {
        let mut buffer = MessageBuffer::new();
        assert_eq!(buffer.write_str("short"), Ok(()));
        assert_eq!(buffer.filled(), b"short");
        assert!(!buffer.truncated());
    }

    /// The ordering rule of the design plan's item C step 2, driven rather
    /// than asserted in a comment. A hook that panics between `fill` and
    /// `seal` leaves the magic unwritten, and the shell reads that as no
    /// record rather than as a valid one with garbage in it.
    #[test]
    fn a_record_with_no_magic_reads_as_absent() {
        let _guard = exclusive();
        let mut buffer = MessageBuffer::new();
        let _ = buffer.write_str("torn");
        fill(7, &buffer);

        assert_eq!(PANIC_SLOT.magic.load(Ordering::Relaxed), 0);
        assert_eq!(PANIC_SLOT.code.load(Ordering::Relaxed), 7);
        assert_eq!(stored_message(), b"torn");

        seal();
        assert_eq!(PANIC_SLOT.magic.load(Ordering::Relaxed), PANIC_MAGIC);
    }

    /// An invariant that does not hold, computed at runtime.
    ///
    /// **A test of the panic hook has to produce a panic, and the workspace
    /// denies both `clippy::panic`, which covers `panic!` and
    /// `std::panic::panic_any`, and `clippy::unwrap_used` and
    /// `clippy::expect_used`.** Those denials are HLD section 23's, and they
    /// are right: a panic reachable from an exported function is a defect.
    /// None of them is switched off here and no `#[allow]` is added.
    ///
    /// So the panic arrives the way a panic actually arrives in this
    /// codebase, through an assertion that fails. `core::hint::black_box` is
    /// what makes the condition runtime data rather than a constant, which is
    /// both honest about the intent and what stops
    /// `clippy::assertions_on_constants` from reading it as `assert!(false)`.
    fn an_invariant_that_does_not_hold() -> bool {
        core::hint::black_box(false)
    }

    /// A panic caught on the host leaves a well-formed record.
    ///
    /// The previous hook is restored, so a later failing test in this process
    /// still reports itself normally.
    #[test]
    fn a_real_panic_leaves_a_well_formed_record() {
        let _guard = exclusive();
        let previous = std::panic::take_hook();
        std::panic::set_hook(Box::new(record));

        let outcome = std::panic::catch_unwind(|| {
            assert!(
                an_invariant_that_does_not_hold(),
                "deliberate panic, F-005 host test"
            );
        });

        // Snapshot BEFORE any assertion. The hook is a process-global, and a
        // failing assertion is itself a panic, so an assertion that ran while
        // the hook was still installed would overwrite the record it is
        // about. Restore the hook, copy the record out, then assert.
        std::panic::set_hook(previous);
        let magic = PANIC_SLOT.magic.load(Ordering::Relaxed);
        let version = PANIC_SLOT.version.load(Ordering::Relaxed);
        let code = PANIC_SLOT.code.load(Ordering::Relaxed);
        let length = PANIC_SLOT.msg_len.load(Ordering::Relaxed);
        let message = String::from_utf8_lossy(&stored_message()).into_owned();

        assert!(outcome.is_err());
        assert_eq!(magic, PANIC_MAGIC);
        assert_eq!(version, PANIC_RECORD_VERSION);
        assert_eq!(code, u32::from(ErrorCode::Panicked.number()));
        assert_ne!(length, 0);
        assert!(
            message.contains("deliberate panic, F-005 host test"),
            "the payload is missing from {message}"
        );
        assert!(
            message.contains("crates/ocelli-wasm/src/panic.rs"),
            "the file is missing from {message}"
        );
        assert!(
            message.contains(" at "),
            "the location separator is missing from {message}"
        );
    }

    /// A panic whose message is longer than the buffer still leaves a record,
    /// with the length clamped to capacity.
    #[test]
    fn an_over_long_panic_message_is_truncated_in_the_record() {
        let _guard = exclusive();
        let previous = std::panic::take_hook();
        std::panic::set_hook(Box::new(record));

        let long = "z".repeat(MESSAGE_CAPACITY * 2);
        let outcome = std::panic::catch_unwind(move || {
            assert!(an_invariant_that_does_not_hold(), "{long}");
        });

        // Snapshot first, for the reason the test above gives.
        std::panic::set_hook(previous);
        let magic = PANIC_SLOT.magic.load(Ordering::Relaxed);
        let length = PANIC_SLOT.msg_len.load(Ordering::Relaxed);

        assert!(outcome.is_err());
        assert_eq!(length, u32::try_from(MESSAGE_CAPACITY).unwrap_or(u32::MAX));
        assert_eq!(magic, PANIC_MAGIC);
    }
}
