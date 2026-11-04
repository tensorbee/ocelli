//! The label a worker reports for the tier `resolve` kept.
//!
//! A retained adapter wins over a cached label. The CPU canvas path is not a
//! tier, so this function never reads it.

use ocelli_render::Tier;

pub(crate) fn tier_name(tier: Tier) -> &'static str {
    match tier {
        Tier::A => "A",
        Tier::B => "B",
        Tier::Cpu => "C",
    }
}

/// `adapter` is the tier of an opened adapter. `cached` is the label stored
/// when no adapter is open.
pub(crate) fn stored_tier_label(
    adapter: Option<Tier>,
    cached: Option<&'static str>,
) -> Option<&'static str> {
    if let Some(tier) = adapter {
        return Some(tier_name(tier));
    }
    cached
}

#[cfg(test)]
mod tests {
    use super::{stored_tier_label, tier_name};
    use ocelli_render::Tier;

    #[test]
    fn an_open_adapter_wins_over_a_cached_label() {
        let label = stored_tier_label(Some(Tier::A), Some("C"));
        assert_eq!(label, Some("A"));
        assert_eq!(stored_tier_label(Some(Tier::B), Some("A")), Some("B"));
        assert_eq!(stored_tier_label(Some(Tier::Cpu), None), Some("C"));
    }

    #[test]
    fn a_cached_label_is_used_only_when_no_adapter_is_open() {
        assert_eq!(stored_tier_label(None, Some("B")), Some("B"));
        assert!(stored_tier_label(None, None).is_none());
        assert_eq!(tier_name(Tier::Cpu), "C");
    }
}
