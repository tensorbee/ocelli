//! JSON for `POST /interpolate`.
//!
//! The body is the token `synthetic`. The numbers are the 2 by 2 display
//! fixture at `(0.5, 0.5)`. Nearest is the corner `texel_of` selects. Linear
//! is `138.75` before display quantisation.

use ocelli_render::{Interpolation, InterpolationError, Tier, available, blend_display};

const SAMPLES: [f32; 4] = [0.0, 100.0, 200.0, 255.0];

/// Answer the interpolation fixture.
#[must_use]
pub fn interpolate_reply(body: &[u8]) -> (u16, String) {
    if body != b"synthetic" {
        return (400, "{\"error\":\"fixture\"}".to_string());
    }
    let nearest = match blend_display(2, &SAMPLES, 0.5, 0.5, Interpolation::Nearest) {
        Ok(value) => value,
        Err(error) => return fixture_error(error),
    };
    let linear = match blend_display(2, &SAMPLES, 0.5, 0.5, Interpolation::Linear) {
        Ok(value) => value,
        Err(error) => return fixture_error(error),
    };
    (200, success_json(nearest, linear))
}

fn fixture_error(error: InterpolationError) -> (u16, String) {
    let code = match error {
        InterpolationError::Outside => "outside",
        InterpolationError::Unavailable => "unavailable",
    };
    (500, format!("{{\"error\":\"{code}\"}}"))
}

fn success_json(nearest: f32, linear: f32) -> String {
    let mut out = String::from("{\"nearest\":");
    out.push_str(&number(nearest));
    out.push_str(",\"linear\":");
    out.push_str(&number(linear));
    out.push_str(",\"tiers\":[");
    out.push_str(&tier_row("A", Tier::A));
    out.push(',');
    out.push_str(&tier_row("B", Tier::B));
    out.push(',');
    out.push_str(&tier_row("C", Tier::Cpu));
    out.push_str("]}");
    out
}

fn tier_row(name: &str, tier: Tier) -> String {
    format!(
        "{{\"tier\":\"{name}\",\"nearest\":\"{nearest}\",\"linear\":\"{linear}\"}}",
        nearest = word(available(Interpolation::Nearest, tier)),
        linear = word(available(Interpolation::Linear, tier)),
    )
}

fn word(offered: bool) -> &'static str {
    if offered { "available" } else { "unavailable" }
}

fn number(value: f32) -> String {
    if value.to_bits() == 0.0_f32.to_bits() {
        return "0".to_string();
    }
    if value.to_bits() == 138.75_f32.to_bits() {
        return "138.75".to_string();
    }
    format!("{value}")
}
