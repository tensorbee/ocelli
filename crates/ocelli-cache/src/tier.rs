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
