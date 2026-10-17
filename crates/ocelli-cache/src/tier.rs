//! HLD section 8's three tiers, and the pressure signal it gives them.
//!
//! Section 8 is "one budget, three tiers, explicit eviction", and it separates
//! the three because "evicting a texture and evicting a frame have very
//! different costs". The tier is carried by the cache rather than by the value,
//! because that cost belongs to the budget an entry was admitted against and
//! not to the entry.
//!
//! **There is no type holding all three.** The three tiers hold three different
//! value types, so one struct over them would need three type parameters with
//! one instantiation each. Three tiers is three `Lru` values.

/// Which of HLD section 8's three budgets a cache keeps.
///
/// `Lru::new` takes it and `Pressure` reports it. An `Admission` does not carry
/// it, because a caller emitting an eviction event already knows which cache it
/// called `insert` on.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CacheTier {
    /// Section 8: "Encoded bytes are transient and dropped as soon as a frame
    /// decodes." That drop is `Lru::remove`, which is not an eviction.
    Encoded,
    /// Section 8: "Decoded frames sit in an LRU sized by the caller."
    Decoded,
    /// Section 8: "GPU textures are their own tier with their own pressure
    /// signal, because evicting a texture and evicting a frame have very
    /// different costs."
    Gpu,
}

/// One tier's pressure signal, which section 8 gives the GPU tier by name and
/// which the other two need for the same reason.
///
/// This is a reading rather than a use, so taking it does not make anything
/// more recently used.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Pressure {
    /// The budget this reading is about.
    pub tier: CacheTier,
    /// The bytes the live entries were admitted at. Never above `budget`.
    pub used: usize,
    /// The bytes the caller allowed this tier.
    pub budget: usize,
}

/// Twice a byte count. `used * TWICE` against `budget` is the half boundary.
const TWICE: u64 = 2;
/// Ten times a byte count. `used * TEN` against `budget * NINE` is nine tenths.
const TEN: u64 = 10;
/// Nine times a budget. The other half of the nine-tenths comparison.
const NINE: u64 = 9;

/// The pressure bands a tier reports crossings between.
///
/// Half and nine tenths, fixed by the S12 design round. The comparisons are
/// integer, `used * 2` against `budget` and `used * 10` against `budget * 9`,
/// so a test can reproduce a boundary without rounding.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PressureBand {
    /// Below half the budget. A budget of zero with nothing in it is here.
    Low,
    /// At or above half, and below nine tenths.
    Elevated,
    /// At or above nine tenths. A budget of zero that still holds bytes is
    /// here, because that tier is over a budget it was told is off.
    Critical,
}

impl Pressure {
    /// Which band this reading sits in.
    ///
    /// The products are `u64`, so a budget that fits in a 32-bit `usize` cannot
    /// wrap the comparison. A budget of zero is `Low` when `used` is zero and
    /// `Critical` otherwise, which keeps the zero-budget tier that
    /// `Lru::would_admit` calls off from looking merely empty once it holds
    /// bytes.
    #[must_use]
    pub fn band(&self) -> PressureBand {
        if self.budget == 0 {
            return if self.used == 0 {
                PressureBand::Low
            } else {
                PressureBand::Critical
            };
        }
        let used = u64::try_from(self.used).unwrap_or(u64::MAX);
        let budget = u64::try_from(self.budget).unwrap_or(u64::MAX);
        let ten_used = used.saturating_mul(TEN);
        let nine_budgets = budget.saturating_mul(NINE);
        if ten_used >= nine_budgets {
            return PressureBand::Critical;
        }
        let twice_used = used.saturating_mul(TWICE);
        if twice_used >= budget {
            PressureBand::Elevated
        } else {
            PressureBand::Low
        }
    }
}
