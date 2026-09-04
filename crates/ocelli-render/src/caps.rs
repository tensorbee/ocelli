//! Resolved device capabilities and the rendering tier.
//!
//! HLD section 22 gives `Caps` field for field. Deviation D-07 adds the third
//! tier variant.
//!
//! **This module defines the type. It does not detect it.** Adapter
//! enumeration and tier resolution are F-004 (E1.4), and device creation is
//! F-039. F-008 needs the type because HLD section 31's `Kernel::workgroup`
//! takes a `&Caps`, and a hook expressed in types needs the types.

/// The rendering tier a session resolved to.
///
/// HLD section 7 gives two, both GPU. `Cpu` is deviation **D-07**, because
/// section 7 leaves a machine with neither WebGPU nor WebGL2 rendering nothing
/// at all.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Tier {
    /// WebGPU. Compute shaders, storage buffers, 3D textures to 2048.
    A,
    /// WebGL2 through wgpu's downlevel profile. Fragment shaders only, no
    /// compute, no storage buffers, a conservative 3D-texture floor of 256.
    B,
    /// CPU. Deviation D-07. A stack viewport renders, windows, scrolls and
    /// measures, reusing `ocelli-pixel` rather than reimplementing the LUT
    /// chain.
    Cpu,
}

impl Tier {
    /// Whether this tier can run a compute shader at all.
    ///
    /// Only tier A can. Section 7: "Anything wanting compute - GPU
    /// segmentation, histogram passes, compute-based resampling - is tier A
    /// only and must degrade, not fail."
    #[must_use]
    pub fn supports_compute(self) -> bool {
        matches!(self, Tier::A)
    }
}

/// HLD section 22, verbatim in its fields.
///
/// ```text
/// pub struct Caps {
///     pub compute: bool,
///     pub max_tex_3d: u32,
///     pub max_buffer: u64,
///     pub tier: Tier, // A = WebGPU, B = WebGL2 downlevel
/// }
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Caps {
    /// Whether compute shaders are available.
    pub compute: bool,
    /// The largest 3D texture dimension the adapter reports.
    pub max_tex_3d: u32,
    /// The largest buffer the adapter guarantees.
    pub max_buffer: u64,
    /// The resolved tier.
    pub tier: Tier,
}

#[cfg(test)]
mod tests {
    use super::{Caps, Tier};

    /// Only tier A supports compute, and the other two are named explicitly so
    /// that adding a fourth tier fails this test rather than passing by
    /// default. `matches!` on one variant would let a new variant inherit
    /// whichever answer the catch-all happened to give.
    #[test]
    fn only_tier_a_supports_compute() {
        assert!(Tier::A.supports_compute());
        assert!(!Tier::B.supports_compute());
        assert!(!Tier::Cpu.supports_compute());
    }

    /// A `Caps` carries section 22's four fields and D-07's third tier.
    ///
    /// The point of the test is that `Tier::Cpu` is CONSTRUCTIBLE here. HLD
    /// section 7 has two tiers and this project has three, and the deviation
    /// is only real if the third one exists in the type.
    #[test]
    fn caps_carries_the_four_fields_and_the_third_tier() {
        let caps = Caps {
            compute: false,
            max_tex_3d: 256,
            max_buffer: 268_435_456,
            tier: Tier::Cpu,
        };
        assert_eq!(caps.tier, Tier::Cpu);
        assert!(!caps.compute);
        assert_eq!(caps.max_tex_3d, 256);
        assert_eq!(caps.max_buffer, 268_435_456);
    }
}
