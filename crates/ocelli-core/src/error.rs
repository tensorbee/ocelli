//! The error model. HLD section 23, and section 4's crate table.
//!
//! Section 4 puts "error model" in this crate's row, so this is where it
//! lives, not in `ocelli-wasm`. Section 23 is the whole policy:
//!
//! > thiserror in the core crates, the boundary maps everything to a stable
//! > numeric code and a message.
//!
//! and
//!
//! > Error codes are stable and versioned. The shell switches on the code, the
//! > message is for humans and may change.
//!
//! Two consequences shape everything below.
//!
//! **The message is not here.** "May change" means a message that crosses the
//! boundary becomes a contract the moment a consumer matches on it, and
//! `core::fmt` machinery plus format strings are exactly the weight
//! `ci/wasm-size-budget.json` exists to notice. The human text lives in
//! `packages/core/src/errors.ts`, keyed on the code. `thiserror`'s
//! `#[error("...")]` strings are still carried for `Display` in native builds
//! and tests, where they cost nothing that matters.
//!
//! **The wire shape is thirty-two bytes.** That is the `payload` of section
//! 17.3's `Event`, so an error and a log line ride the event ring with no
//! second layout and no deserialisation. `Event.kind` is what separates the
//! two kinds, so the discriminant is not repeated inside the payload.
//!
//! No allocation anywhere in this module. No `String`, no `alloc`. Operands
//! are `u64`.

use core::fmt;

/// A stable, versioned numeric error code. HLD section 23.
///
/// **The numbering is owned by `ci/error-codes.json`**, and
/// `scripts/error_code_check.py` refuses a renumbering, a reused number and a
/// code that either side of the boundary cannot name. The registry also
/// declares the per-crate ranges, so a second crate cannot quietly pick an
/// overlapping block.
///
/// **Only variants with a live producer today appear here**, which is three.
/// A fourth arrives with the story whose code it is. Reserving a space of
/// plausible future codes would be inventing producers.
///
/// `0` is never a valid code, so a zeroed buffer cannot decode as a real one.
#[repr(u16)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum ErrorCode {
    /// A Rust panic reached the panic hook. The instance is poisoned.
    ///
    /// Section 23: "Once a Rust panic aborts inside WebAssembly, that module
    /// instance's memory may be inconsistent and it must not be reused."
    /// This code is written into the panic record of
    /// `crates/ocelli-wasm/src/panic.rs`, which the shell reads out of linear
    /// memory after the trap without calling any export.
    Panicked = 1,

    /// A feature cannot run on the resolved tier and declares no fallback.
    ///
    /// Maps `ocelli_compute::ComputeError::Unavailable`. Deviation D-07's
    /// rule is that such a feature reports unavailable and never silently
    /// produces a different answer, and this is that sentence as a number, so
    /// a tier C session reports it in the encoding a tier A session would use.
    Unavailable = 700,

    /// A kernel asked for a workgroup the device cannot dispatch.
    ///
    /// Maps `ocelli_compute::ComputeError::Workgroup`. HLD section 31:
    /// "Workgroup sizes come from `Caps`, never hardcoded."
    Workgroup = 701,
}

impl ErrorCode {
    /// The stable number, written as a `match` rather than an `as` cast.
    ///
    /// HLD section 27.3 makes every cast a human review item, and an enum
    /// discriminant cast is the one place a renumbering would be invisible in
    /// a diff. The `match` puts every number on a line of its own, where
    /// `scripts/error_code_check.py` can read it and a reviewer can see it.
    #[must_use]
    pub const fn number(self) -> u16 {
        match self {
            Self::Panicked => 1,
            Self::Unavailable => 700,
            Self::Workgroup => 701,
        }
    }

    /// The code a number names, or `None` when this build does not know it.
    ///
    /// `None` is not a failure. A newer core may send a code an older
    /// consumer has never heard of, and the honest answer is that the number
    /// is unknown rather than that the record is corrupt.
    #[must_use]
    pub const fn from_number(number: u16) -> Option<Self> {
        match number {
            1 => Some(Self::Panicked),
            700 => Some(Self::Unavailable),
            701 => Some(Self::Workgroup),
            _ => None,
        }
    }
}

/// How bad an error is. Byte 2 of an error [`Record`].
///
/// `0` is not a variant, so a zeroed payload is not a valid record whichever
/// kind it claims to be. That matters because byte 2 holds a [`LogLevel`] for
/// a log record, and [`LogLevel`] reserves `0` for the same reason. One byte,
/// one rule, one decoder.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Severity {
    /// The call failed and the instance is fine. The caller may try again.
    Recoverable = 1,
    /// The instance is poisoned. Section 23, and no further call is made.
    Fatal = 2,
}

impl Severity {
    /// The stable number. A `match`, for [`ErrorCode::number`]'s reason.
    #[must_use]
    pub const fn number(self) -> u8 {
        match self {
            Self::Recoverable => 1,
            Self::Fatal => 2,
        }
    }
}

/// How loud a log line is. Byte 2 of a log [`Record`].
///
/// The HLD has no logging section, so these are this repository's, recorded
/// in `docs/lld/errors.md`. `0` is reserved so a zeroed payload is not a valid
/// line.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LogLevel {
    /// Something failed.
    Error = 1,
    /// Something is wrong and the operation continued.
    Warn = 2,
    /// A lifecycle event worth seeing by default.
    Info = 3,
    /// Detail for somebody debugging this code.
    Debug = 4,
    /// Per-frame detail. Off unless somebody asked for it.
    Trace = 5,
}

impl LogLevel {
    /// The stable number. A `match`, for [`ErrorCode::number`]'s reason.
    #[must_use]
    pub const fn number(self) -> u8 {
        match self {
            Self::Error => 1,
            Self::Warn => 2,
            Self::Info => 3,
            Self::Debug => 4,
            Self::Trace => 5,
        }
    }
}

/// Why a thirty-two byte payload is not a [`Record`].
///
/// Section 23's first clause is "thiserror in the core crates", and this is
/// the crate's `thiserror` enum. The strings are for `Display` on the host,
/// in tests and in the two native entry points. They do not cross the
/// boundary, which is what item B of the F-005 design plan is about.
///
/// Deviation D-15: the workspace entry is
/// `thiserror = { version = "2", default-features = false }`, because this
/// crate is `no_std` and thiserror's default feature is `std`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum RecordError {
    /// Byte 0 and byte 1 are zero. `0` is never a valid code.
    #[error("code 0 is never a valid error code, so this is not a record")]
    ZeroCode,

    /// Byte 2 is zero. Neither a [`Severity`] nor a [`LogLevel`] uses `0`.
    #[error("severity or level 0 is reserved, so this is not a record")]
    ZeroSeverityOrLevel,

    /// Byte 3 is above three. There are three operand slots.
    #[error("arity {0} exceeds the three operand slots a record carries")]
    Arity(u8),

    /// A constructor was handed more operands than the layout holds.
    #[error("{0} operands were offered and a record carries at most three")]
    TooManyOperands(usize),
}

/// The thirty-two bytes an error or a log line occupies.
///
/// **One layout for both kinds.** The discriminant that separates them is
/// `Event.kind` in HLD section 17.3's header, so it is not repeated here.
/// One layout, two kinds, one encoder and one decoder on each side of the
/// boundary.
///
/// ```text
/// offset  size  field
/// 0       2     code               u16, little-endian, never 0
/// 2       1     severity_or_level  u8, never 0
/// 3       1     arity              how many of a0..a2 are meaningful
/// 4       4     reserved           u32, a producer writes 0
/// 8       8     a0                 u64
/// 16      8     a1                 u64
/// 24      8     a2                 u64
///                                  total 32
/// ```
///
/// **The `u32` at offset 4 is padding with a name.** Without it the three
/// operands would sit at payload offsets 4, 12 and 20. Section 17.3's `Event`
/// is `#[repr(C)]` over `u32, u32, u64, [u8; 32]`, which puts `payload` at
/// struct offset 16 and gives the stated 48-byte stride, so payload offset 8
/// is struct offset 24 and every operand is eight-byte aligned in linear
/// memory. Nothing here reads a `u64` through a pointer, so this is not
/// correctness today. It is that an unaligned `u64` in a `#[repr(C)]`
/// boundary struct is what a later story adds a `bytemuck::Pod` derive to and
/// discovers at the worst moment.
///
/// **`reserved` is not asserted zero by the consumer.** A consumer that
/// refused a nonzero value could not read a record written by a newer core,
/// and one that ignored it would lose the same information twice. It is
/// surfaced, exactly as section 17.3 says `dropped` is surfaced.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Record {
    code: u16,
    severity_or_level: u8,
    arity: u8,
    reserved: u32,
    operands: [u64; 3],
}

impl Record {
    /// The wire size, in bytes. Section 17.3's `Event::payload` is `[u8; 32]`.
    pub const BYTES: usize = 32;

    /// An error record.
    ///
    /// Fails rather than truncating when handed more than three operands,
    /// because silently dropping the fourth is the kind of quiet loss this
    /// project treats as the dangerous defect.
    ///
    /// # Errors
    ///
    /// [`RecordError::TooManyOperands`] when `operands` is longer than three.
    pub fn error(
        code: ErrorCode,
        severity: Severity,
        operands: &[u64],
    ) -> Result<Self, RecordError> {
        Self::build(code.number(), severity.number(), operands)
    }

    /// A log record.
    ///
    /// The code space is the same registry. Log-specific codes arrive with
    /// the stories that produce them, for [`ErrorCode`]'s reason.
    ///
    /// # Errors
    ///
    /// [`RecordError::TooManyOperands`] when `operands` is longer than three.
    pub fn log(code: ErrorCode, level: LogLevel, operands: &[u64]) -> Result<Self, RecordError> {
        Self::build(code.number(), level.number(), operands)
    }

    fn build(code: u16, severity_or_level: u8, operands: &[u64]) -> Result<Self, RecordError> {
        if code == 0 {
            return Err(RecordError::ZeroCode);
        }
        if severity_or_level == 0 {
            return Err(RecordError::ZeroSeverityOrLevel);
        }
        // Written as a `match` on the length rather than `len() as u8`,
        // because the workspace denies `cast_possible_truncation` and an
        // arity that silently wrapped would be a record claiming operands it
        // does not carry.
        let arity: u8 = match operands.len() {
            0 => 0,
            1 => 1,
            2 => 2,
            3 => 3,
            other => return Err(RecordError::TooManyOperands(other)),
        };
        let mut slots = [0_u64; 3];
        for (slot, value) in slots.iter_mut().zip(operands.iter()) {
            *slot = *value;
        }
        Ok(Self {
            code,
            severity_or_level,
            arity,
            reserved: 0,
            operands: slots,
        })
    }

    /// The stable numeric code. Never zero.
    #[must_use]
    pub const fn code(self) -> u16 {
        self.code
    }

    /// The code this build can name, or `None` for a code it cannot.
    #[must_use]
    pub const fn error_code(self) -> Option<ErrorCode> {
        ErrorCode::from_number(self.code)
    }

    /// Byte 2. A [`Severity`] for an error, a [`LogLevel`] for a log line.
    #[must_use]
    pub const fn severity_or_level(self) -> u8 {
        self.severity_or_level
    }

    /// How many of the three operand slots carry meaning. At most three.
    #[must_use]
    pub const fn arity(self) -> u8 {
        self.arity
    }

    /// The three operand slots, whatever the arity says.
    #[must_use]
    pub const fn operands(self) -> [u64; 3] {
        self.operands
    }

    /// The `u32` at offset 4. A producer writes zero.
    #[must_use]
    pub const fn reserved(self) -> u32 {
        self.reserved
    }

    /// Whether this record carries something the reader does not understand.
    ///
    /// Reported, not swallowed and not fatal. Section 17.3's rule for
    /// `dropped`, applied to the one field reserved for a newer producer.
    #[must_use]
    pub const fn carries_unknown_reserved(self) -> bool {
        self.reserved != 0
    }

    /// The thirty-two bytes, little-endian throughout.
    #[must_use]
    pub const fn encode(self) -> [u8; Self::BYTES] {
        let [c0, c1] = self.code.to_le_bytes();
        let [r0, r1, r2, r3] = self.reserved.to_le_bytes();
        let [a0, a1, a2] = self.operands;
        let [p0, p1, p2, p3, p4, p5, p6, p7] = a0.to_le_bytes();
        let [q0, q1, q2, q3, q4, q5, q6, q7] = a1.to_le_bytes();
        let [s0, s1, s2, s3, s4, s5, s6, s7] = a2.to_le_bytes();
        [
            c0,
            c1,
            self.severity_or_level,
            self.arity,
            r0,
            r1,
            r2,
            r3,
            p0,
            p1,
            p2,
            p3,
            p4,
            p5,
            p6,
            p7,
            q0,
            q1,
            q2,
            q3,
            q4,
            q5,
            q6,
            q7,
            s0,
            s1,
            s2,
            s3,
            s4,
            s5,
            s6,
            s7,
        ]
    }

    /// Read thirty-two bytes back.
    ///
    /// Destructured rather than indexed, because the workspace runs clippy
    /// with `-D warnings` and `indexing_slicing` is a warning, and because the
    /// pattern puts the layout table above in the source where a reviewer
    /// reads it.
    ///
    /// An unregistered code decodes. See [`Record::error_code`].
    ///
    /// # Errors
    ///
    /// [`RecordError::ZeroCode`], [`RecordError::ZeroSeverityOrLevel`] or
    /// [`RecordError::Arity`], which are the three ways a run of bytes is not
    /// a record.
    pub const fn decode(bytes: &[u8; Self::BYTES]) -> Result<Self, RecordError> {
        let [
            c0,
            c1,
            severity_or_level,
            arity,
            r0,
            r1,
            r2,
            r3,
            p0,
            p1,
            p2,
            p3,
            p4,
            p5,
            p6,
            p7,
            q0,
            q1,
            q2,
            q3,
            q4,
            q5,
            q6,
            q7,
            s0,
            s1,
            s2,
            s3,
            s4,
            s5,
            s6,
            s7,
        ] = *bytes;

        let code = u16::from_le_bytes([c0, c1]);
        if code == 0 {
            return Err(RecordError::ZeroCode);
        }
        if severity_or_level == 0 {
            return Err(RecordError::ZeroSeverityOrLevel);
        }
        if arity > 3 {
            return Err(RecordError::Arity(arity));
        }
        Ok(Self {
            code,
            severity_or_level,
            arity,
            reserved: u32::from_le_bytes([r0, r1, r2, r3]),
            operands: [
                u64::from_le_bytes([p0, p1, p2, p3, p4, p5, p6, p7]),
                u64::from_le_bytes([q0, q1, q2, q3, q4, q5, q6, q7]),
                u64::from_le_bytes([s0, s1, s2, s3, s4, s5, s6, s7]),
            ],
        })
    }
}

impl fmt::Display for Record {
    /// Code and operands, never a sentence.
    ///
    /// The sentence is `packages/core/src/errors.ts`'s job, per section 23's
    /// "the message is for humans and may change". This exists so a native
    /// entry point and a test have something to print.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "ocelli record code {} level {} arity {}",
            self.code, self.severity_or_level, self.arity
        )
    }
}

#[cfg(test)]
mod tests {
    use super::{ErrorCode, LogLevel, Record, RecordError, Severity};

    /// The expected bytes are WRITTEN OUT BY HAND from the layout table in
    /// the `Record` doc comment, which is derived from HLD section 17.3's
    /// `Event::payload`. They are never produced by calling the encoder.
    ///
    /// HLD 27.2 R2: an agent asked to test a function will assert what it
    /// does, not what it should do. This story contains no pixel arithmetic
    /// and therefore no DICOM section for a fixture to cite, so R3's spirit is
    /// served by applying R2's rule to a layout instead of to a formula.
    ///
    /// Working, for `code = 1`, `severity = Fatal = 2`, `arity = 1`,
    /// `reserved = 0`, `a0 = 0x0102_0304_0506_0708`:
    ///
    /// - offset 0, `u16` 1 little-endian, is `01 00`
    /// - offset 2, `Severity::Fatal`, is `02`
    /// - offset 3, one meaningful operand, is `01`
    /// - offset 4, `reserved` 0, is `00 00 00 00`
    /// - offset 8, `u64` `0x0102_0304_0506_0708` little-endian, is that byte
    ///   order reversed, `08 07 06 05 04 03 02 01`
    /// - offsets 16 and 24, both operands zero, are sixteen `00` bytes
    const ERROR_BYTES: [u8; 32] = [
        0x01, 0x00, 0x02, 0x01, 0x00, 0x00, 0x00, 0x00, 0x08, 0x07, 0x06, 0x05, 0x04, 0x03, 0x02,
        0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00,
    ];

    /// The same table for a log line, hand-written the same way.
    ///
    /// `code = 700`, `level = Info = 3`, `arity = 3`, `reserved = 0`, operands
    /// `1`, `2` and `u64::MAX`.
    ///
    /// - offset 0, `u16` 700 is `0x02BC`, little-endian `BC 02`
    /// - offset 2, `LogLevel::Info`, is `03`
    /// - offset 3, three meaningful operands, is `03`
    /// - offset 8, `u64` 1, is `01` then seven `00`
    /// - offset 16, `u64` 2, is `02` then seven `00`
    /// - offset 24, `u64::MAX`, is eight `FF`
    const LOG_BYTES: [u8; 32] = [
        0xBC, 0x02, 0x03, 0x03, 0x00, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x02, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
        0xFF, 0xFF,
    ];

    /// One byte changed, without indexing. The workspace runs clippy with
    /// `-D warnings` and `indexing_slicing` is a warning, so a test that
    /// indexes is a test that fails the gate.
    fn with_byte(bytes: [u8; 32], offset: usize, value: u8) -> [u8; 32] {
        let mut copy = bytes;
        if let Some(slot) = copy.get_mut(offset) {
            *slot = value;
        }
        copy
    }

    /// Every assertion below goes through `Result::map` rather than through
    /// `unwrap` or `expect`, which the workspace denies in every target
    /// including tests. HLD section 23 is the reason: a panic poisons the
    /// instance, so there is no "this cannot fail" anywhere in this crate.

    #[test]
    fn an_error_encodes_to_the_hand_written_layout() {
        assert_eq!(
            Record::error(
                ErrorCode::Panicked,
                Severity::Fatal,
                &[0x0102_0304_0506_0708]
            )
            .map(Record::encode),
            Ok(ERROR_BYTES)
        );
    }

    #[test]
    fn an_error_decodes_from_the_hand_written_layout() {
        let decoded = Record::decode(&ERROR_BYTES);
        assert_eq!(decoded.map(Record::code), Ok(1));
        assert_eq!(
            decoded.map(Record::error_code),
            Ok(Some(ErrorCode::Panicked))
        );
        assert_eq!(
            decoded.map(Record::severity_or_level),
            Ok(Severity::Fatal.number())
        );
        assert_eq!(decoded.map(Record::arity), Ok(1));
        assert_eq!(decoded.map(Record::reserved), Ok(0));
        assert_eq!(
            decoded.map(Record::operands),
            Ok([0x0102_0304_0506_0708, 0, 0])
        );
        assert_eq!(decoded.map(Record::carries_unknown_reserved), Ok(false));
    }

    #[test]
    fn a_log_line_encodes_to_the_hand_written_layout() {
        assert_eq!(
            Record::log(ErrorCode::Unavailable, LogLevel::Info, &[1, 2, u64::MAX])
                .map(Record::encode),
            Ok(LOG_BYTES)
        );
    }

    #[test]
    fn a_log_line_decodes_from_the_hand_written_layout() {
        let decoded = Record::decode(&LOG_BYTES);
        assert_eq!(decoded.map(Record::code), Ok(700));
        assert_eq!(
            decoded.map(Record::error_code),
            Ok(Some(ErrorCode::Unavailable))
        );
        assert_eq!(
            decoded.map(Record::severity_or_level),
            Ok(LogLevel::Info.number())
        );
        assert_eq!(decoded.map(Record::arity), Ok(3));
        assert_eq!(decoded.map(Record::operands), Ok([1, 2, u64::MAX]));
    }

    /// The numbers are the registry's, and `ci/error-codes.json` holds the
    /// same three. Asserted as literals here as well, so a renumbering goes
    /// red in the crate that owns the enum and not only in the guard.
    #[test]
    fn the_three_codes_carry_their_registered_numbers() {
        assert_eq!(ErrorCode::Panicked.number(), 1);
        assert_eq!(ErrorCode::Unavailable.number(), 700);
        assert_eq!(ErrorCode::Workgroup.number(), 701);
        assert_eq!(ErrorCode::from_number(1), Some(ErrorCode::Panicked));
        assert_eq!(ErrorCode::from_number(700), Some(ErrorCode::Unavailable));
        assert_eq!(ErrorCode::from_number(701), Some(ErrorCode::Workgroup));
        assert_eq!(ErrorCode::from_number(0), None);
    }

    /// An unregistered number is not a decode failure. A newer core may send
    /// one, and "unknown" is the honest answer rather than "corrupt".
    #[test]
    fn an_unregistered_code_decodes_and_names_no_variant() {
        let bytes = with_byte(with_byte(ERROR_BYTES, 0, 0xFE), 1, 0xFF);
        let decoded = Record::decode(&bytes);
        assert_eq!(decoded.map(Record::code), Ok(0xFFFE));
        assert_eq!(decoded.map(Record::error_code), Ok(None));
    }

    /// A zeroed payload is not a record, and the two refusals that make that
    /// true are checked separately so neither is the only thing standing.
    #[test]
    fn a_zeroed_payload_is_not_a_record() {
        assert_eq!(Record::decode(&[0_u8; 32]), Err(RecordError::ZeroCode));
    }

    #[test]
    fn a_zero_severity_or_level_is_not_a_record() {
        assert_eq!(
            Record::decode(&with_byte(ERROR_BYTES, 2, 0)),
            Err(RecordError::ZeroSeverityOrLevel)
        );
    }

    #[test]
    fn an_arity_above_three_is_not_a_record() {
        assert_eq!(
            Record::decode(&with_byte(ERROR_BYTES, 3, 4)),
            Err(RecordError::Arity(4))
        );
    }

    /// `reserved` is surfaced, not refused and not dropped. Section 17.3's
    /// rule for `dropped`, applied to the field a newer producer may use.
    #[test]
    fn a_nonzero_reserved_decodes_and_is_reported() {
        let decoded = Record::decode(&with_byte(ERROR_BYTES, 4, 0x2A));
        assert_eq!(decoded.map(Record::reserved), Ok(0x2A));
        assert_eq!(decoded.map(Record::carries_unknown_reserved), Ok(true));
        assert_eq!(decoded.map(Record::code), Ok(1));
        assert_eq!(
            decoded.map(Record::operands),
            Ok([0x0102_0304_0506_0708, 0, 0])
        );
    }

    #[test]
    fn a_fourth_operand_is_refused_rather_than_dropped() {
        assert_eq!(
            Record::error(ErrorCode::Panicked, Severity::Fatal, &[1, 2, 3, 4]),
            Err(RecordError::TooManyOperands(4))
        );
    }

    /// A round-trip, on top of the two hand-written vectors and not instead of
    /// them. A round-trip alone passes for any self-consistent layout,
    /// including the wrong one.
    #[test]
    fn every_field_survives_a_round_trip() {
        let built = Record::log(ErrorCode::Workgroup, LogLevel::Trace, &[u64::MAX, 0, 7]);
        assert_eq!(built.map(|r| Record::decode(&r.encode())), Ok(built));
    }

    /// `Display` is what a native entry point or a test prints. It carries no
    /// sentence, because section 23 puts the sentence on the shell side, and a
    /// second copy here would be a second thing to keep in step.
    ///
    /// Asserted rather than left uncovered: an impl nothing executes is the
    /// defect class a green suite cannot report on.
    #[test]
    fn display_carries_the_numbers_and_no_sentence() {
        assert_eq!(
            Record::error(ErrorCode::Unavailable, Severity::Recoverable, &[])
                .map(|record| format!("{record}")),
            Ok("ocelli record code 700 level 1 arity 0".to_owned())
        );
    }

    #[test]
    fn severity_and_level_share_byte_two_and_neither_uses_zero() {
        assert_eq!(Severity::Recoverable.number(), 1);
        assert_eq!(Severity::Fatal.number(), 2);
        assert_eq!(LogLevel::Error.number(), 1);
        assert_eq!(LogLevel::Warn.number(), 2);
        assert_eq!(LogLevel::Info.number(), 3);
        assert_eq!(LogLevel::Debug.number(), 4);
        assert_eq!(LogLevel::Trace.number(), 5);
    }
}
