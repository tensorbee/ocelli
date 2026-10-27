//! The boundary's byte layout, declared once.
//!
//! F-104. Every number the TypeScript shell reads from the boundary is
//! declared here: the command ids and payload lengths, the enum bytes, the
//! state readback's size and offsets, the event ring's header and stride, and
//! the flag bits. `crates/ocelli-viewport/tests/typegen.rs` writes
//! `packages/core/src/generated.ts` from these items, so the two languages
//! cannot disagree about a number.
//!
//! The state and ring layouts are `#[repr(C)]` and `bytemuck::Pod`, and their
//! offsets come from `offset_of!` rather than literals. `bytemuck::bytes_of`
//! copies host bytes. HLD 17.1 requires little-endian, so a big-endian target
//! is a compile failure rather than a swapped window.

#[cfg(not(target_endian = "little"))]
compile_error!("the boundary wire format is little-endian");

use bytemuck::{Pod, Zeroable};
use ocelli_cache::{
    KIND_BUDGET_CHANGED, KIND_DISPLACED, KIND_EVICTED, KIND_PRESSURE, KIND_REFUSED, RingEvent,
};

/// `Fit` in the packed stack channel. Kind 0 is invalid.
pub const KIND_FIT: u16 = 1;
/// `Pan`.
pub const KIND_PAN: u16 = 2;
/// `Zoom`.
pub const KIND_ZOOM: u16 = 3;
/// `Rotate`.
pub const KIND_ROTATE: u16 = 4;
/// `FlipHorizontal`.
pub const KIND_FLIP_HORIZONTAL: u16 = 5;
/// `FlipVertical`.
pub const KIND_FLIP_VERTICAL: u16 = 6;
/// `SetWindow`.
pub const KIND_SET_WINDOW: u16 = 7;
/// `ApplyPreset`.
pub const KIND_APPLY_PRESET: u16 = 8;
/// `SetInvert`.
pub const KIND_SET_INVERT: u16 = 9;
/// `SetColormap`.
pub const KIND_SET_COLORMAP: u16 = 10;
/// `Scroll`.
pub const KIND_SCROLL: u16 = 11;

/// One command in the packed stack channel.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CommandSpec {
    /// The wire kind. Kind 0 is invalid.
    pub kind: u16,
    /// The name the TypeScript surface publishes.
    pub name: &'static str,
    /// Payload bytes after the eight-byte envelope.
    pub payload_bytes: u32,
}

/// The eleven commands, in the order the TypeScript surface publishes them.
pub const COMMAND_TABLE: [CommandSpec; 11] = [
    CommandSpec {
        kind: KIND_FIT,
        name: "Fit",
        payload_bytes: 24,
    },
    CommandSpec {
        kind: KIND_PAN,
        name: "Pan",
        payload_bytes: 16,
    },
    CommandSpec {
        kind: KIND_ZOOM,
        name: "Zoom",
        payload_bytes: 8,
    },
    CommandSpec {
        kind: KIND_ROTATE,
        name: "Rotate",
        payload_bytes: 8,
    },
    CommandSpec {
        kind: KIND_FLIP_HORIZONTAL,
        name: "FlipHorizontal",
        payload_bytes: 1,
    },
    CommandSpec {
        kind: KIND_FLIP_VERTICAL,
        name: "FlipVertical",
        payload_bytes: 1,
    },
    CommandSpec {
        kind: KIND_SET_WINDOW,
        name: "SetWindow",
        payload_bytes: 17,
    },
    CommandSpec {
        kind: KIND_APPLY_PRESET,
        name: "ApplyPreset",
        payload_bytes: 1,
    },
    CommandSpec {
        kind: KIND_SET_INVERT,
        name: "SetInvert",
        payload_bytes: 1,
    },
    CommandSpec {
        kind: KIND_SET_COLORMAP,
        name: "SetColormap",
        payload_bytes: 1,
    },
    CommandSpec {
        kind: KIND_SCROLL,
        name: "Scroll",
        payload_bytes: 4,
    },
];

/// The payload length this table gives `kind`, or `None` for a kind it does
/// not name. This is the only lookup the decoder uses.
#[must_use]
pub(crate) fn payload_len_of(kind: u16) -> Option<u32> {
    COMMAND_TABLE
        .iter()
        .find(|spec| spec.kind == kind)
        .map(|spec| spec.payload_bytes)
}

/// Wire byte for `VoiFunction::Linear`.
pub const VOI_FUNCTION_LINEAR: u8 = 0;
/// Wire byte for `VoiFunction::LinearExact`.
pub const VOI_FUNCTION_LINEAR_EXACT: u8 = 1;
/// Wire byte for `VoiFunction::Sigmoid`.
pub const VOI_FUNCTION_SIGMOID: u8 = 2;

/// The VOI functions and their wire bytes.
pub const VOI_FUNCTION_BYTES: [(&str, u8); 3] = [
    ("linear", VOI_FUNCTION_LINEAR),
    ("linear_exact", VOI_FUNCTION_LINEAR_EXACT),
    ("sigmoid", VOI_FUNCTION_SIGMOID),
];

/// Wire byte for `WindowPreset::SoftTissue`.
pub const PRESET_SOFT_TISSUE: u8 = 0;
/// Wire byte for `WindowPreset::Lung`.
pub const PRESET_LUNG: u8 = 1;
/// Wire byte for `WindowPreset::Bone`.
pub const PRESET_BONE: u8 = 2;

/// The window presets and their wire bytes.
pub const PRESET_BYTES: [(&str, u8); 3] = [
    ("soft_tissue", PRESET_SOFT_TISSUE),
    ("lung", PRESET_LUNG),
    ("bone", PRESET_BONE),
];

/// Wire byte for `Colormap::Grey`.
pub const COLORMAP_GREY: u8 = 0;
/// Wire byte for `Colormap::Stops`.
pub const COLORMAP_STOPS: u8 = 1;

/// The colour tables and their wire bytes.
pub const COLORMAP_BYTES: [(&str, u8); 2] = [("grey", COLORMAP_GREY), ("stops", COLORMAP_STOPS)];

/// Wire byte for an identity presentation shape.
pub const INVERT_IDENTITY: u8 = 0;
/// Wire byte for an inverse presentation shape.
pub const INVERT_INVERSE: u8 = 1;

/// The explicit presentation shapes and their wire bytes.
pub const INVERT_BYTES: [(&str, u8); 2] =
    [("identity", INVERT_IDENTITY), ("inverse", INVERT_INVERSE)];

/// Section 17.4 flag bit 0. Explicit inverse. `MONOCHROME1` is not this bit.
pub const FLAG_INVERT: u32 = 1;
/// Section 17.4 flag bit 1. A load is in progress.
pub const FLAG_LOADING: u32 = 2;
/// Section 17.4 flag bit 2. The viewport is in error.
pub const FLAG_ERROR: u32 = 4;
/// Section 17.4 flag bit 3. A reduced frame is on screen.
pub const FLAG_REFINING: u32 = 8;

/// Section 17.4's readback, field for field. 96 bytes.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Pod, Zeroable)]
pub struct ViewportState {
    /// Column-major camera, sixteen `f32`.
    pub camera: [f32; 16],
    /// VOI window centre.
    pub voi_center: f32,
    /// VOI window width.
    pub voi_width: f32,
    /// Selected slice.
    pub slice_index: u32,
    /// Stored sequence length.
    pub num_slices: u32,
    /// The four flag bits, see the `FLAG_` constants.
    pub flags: u32,
    /// HLD section 17.4's `_pad`.
    pub padding: [u32; 3],
}

/// Section 17.3's ring header, field for field. 16 bytes.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Pod, Zeroable)]
pub struct RingHeader {
    /// Pending events in the copied prefix.
    pub head: u32,
    /// Always zero in a copy, so the events are a linear prefix.
    pub tail: u32,
    /// The ring's capacity.
    pub capacity: u32,
    /// Events the producer dropped.
    pub dropped: u32,
}

/// Presented frame. Not a cache kind.
pub const KIND_PRESENTED: u32 = 6;

/// The six ring kinds and the names the TypeScript catalogue publishes.
pub const RING_EVENT_KINDS: [(&str, u32); 6] = [
    ("evicted", KIND_EVICTED),
    ("displaced", KIND_DISPLACED),
    ("refused", KIND_REFUSED),
    ("pressure", KIND_PRESSURE),
    ("budget_changed", KIND_BUDGET_CHANGED),
    ("presented", KIND_PRESENTED),
];

/// `RingHeader` in bytes.
pub const EVENT_HEADER_BYTES: usize = core::mem::size_of::<RingHeader>();
/// Offset of `head`, which a copy carries the pending count in.
pub const EVENT_RING_PENDING_OFFSET: usize = core::mem::offset_of!(RingHeader, head);
/// Offset of `tail`, which a copy carries zero in.
pub const EVENT_RING_TAIL_OFFSET: usize = core::mem::offset_of!(RingHeader, tail);
/// Offset of `capacity`.
pub const EVENT_RING_CAPACITY_OFFSET: usize = core::mem::offset_of!(RingHeader, capacity);
/// Offset of `dropped`.
pub const EVENT_RING_DROPPED_OFFSET: usize = core::mem::offset_of!(RingHeader, dropped);

/// `RingEvent` in bytes. Section 17.3's 48-byte stride.
pub const EVENT_STRIDE: usize = core::mem::size_of::<RingEvent>();
/// Offset of an event's `kind`.
pub const EVENT_KIND_OFFSET: usize = core::mem::offset_of!(RingEvent, kind);
/// Offset of an event's `viewport`.
pub const EVENT_VIEWPORT_OFFSET: usize = core::mem::offset_of!(RingEvent, viewport);
/// Offset of an event's `seq`.
pub const EVENT_SEQ_OFFSET: usize = core::mem::offset_of!(RingEvent, seq);
/// Offset of an event's payload.
pub const EVENT_PAYLOAD_OFFSET: usize = core::mem::offset_of!(RingEvent, payload);
/// An event's payload in bytes.
pub const EVENT_PAYLOAD_BYTES: usize = EVENT_STRIDE - EVENT_PAYLOAD_OFFSET;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn state_layout_matches_hld_17_4() {
        assert_eq!(core::mem::size_of::<ViewportState>(), 96);
        assert_eq!(core::mem::offset_of!(ViewportState, camera), 0);
        assert_eq!(core::mem::offset_of!(ViewportState, voi_center), 64);
        assert_eq!(core::mem::offset_of!(ViewportState, voi_width), 68);
        assert_eq!(core::mem::offset_of!(ViewportState, slice_index), 72);
        assert_eq!(core::mem::offset_of!(ViewportState, num_slices), 76);
        assert_eq!(core::mem::offset_of!(ViewportState, flags), 80);
        assert_eq!(core::mem::offset_of!(ViewportState, padding), 84);
    }

    #[test]
    fn ring_layout_matches_hld_17_3() {
        assert_eq!(EVENT_HEADER_BYTES, 16);
        assert_eq!(EVENT_STRIDE, 48);
        assert_eq!(EVENT_PAYLOAD_BYTES, 32);
        assert_eq!(EVENT_KIND_OFFSET, 0);
        assert_eq!(EVENT_VIEWPORT_OFFSET, 4);
        assert_eq!(EVENT_SEQ_OFFSET, 8);
        assert_eq!(EVENT_PAYLOAD_OFFSET, 16);
        assert_eq!(EVENT_RING_PENDING_OFFSET, 0);
        assert_eq!(EVENT_RING_TAIL_OFFSET, 4);
        assert_eq!(EVENT_RING_CAPACITY_OFFSET, 8);
        assert_eq!(EVENT_RING_DROPPED_OFFSET, 12);
    }

    #[test]
    fn command_table_is_unique_and_contiguous() {
        assert_eq!(COMMAND_TABLE.len(), 11);
        for (index, spec) in COMMAND_TABLE.iter().enumerate() {
            let expected = u16::try_from(index + 1).unwrap_or(0);
            assert_eq!(spec.kind, expected, "{}", spec.name);
            assert_eq!(payload_len_of(spec.kind), Some(spec.payload_bytes));
            for other in COMMAND_TABLE.iter().skip(index + 1) {
                assert_ne!(spec.name, other.name);
                assert_ne!(spec.kind, other.kind);
            }
        }
        assert_eq!(payload_len_of(0), None);
        assert_eq!(payload_len_of(12), None);
    }

    #[test]
    fn enum_bytes_are_unique() {
        let tables: [&[(&str, u8)]; 4] = [
            &VOI_FUNCTION_BYTES,
            &PRESET_BYTES,
            &COLORMAP_BYTES,
            &INVERT_BYTES,
        ];
        for table in tables {
            for (index, (name, byte)) in table.iter().enumerate() {
                for (other_name, other_byte) in table.iter().skip(index + 1) {
                    assert_ne!(name, other_name);
                    assert_ne!(byte, other_byte);
                }
            }
        }
    }

    #[test]
    fn ring_kinds_are_unique() {
        assert_eq!(RING_EVENT_KINDS.len(), 6);
        assert_eq!(KIND_PRESENTED, 6);
        for (index, (name, kind)) in RING_EVENT_KINDS.iter().enumerate() {
            for (other_name, other_kind) in RING_EVENT_KINDS.iter().skip(index + 1) {
                assert_ne!(name, other_name);
                assert_ne!(kind, other_kind);
            }
        }
    }
}
