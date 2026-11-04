//! F-194 stack interpolation fixtures.
//!
//! The 2 by 2 image is `[[0, 100], [200, 255]]`. At `(0.5, 0.5)` each
//! weight is one quarter, and `0.25 * (0 + 100 + 200 + 255)` is `138.75`.
//! That value is compared before `display_index`.

use ocelli_render::TierSample;
use ocelli_render::{
    CpuRaster, Interpolation, InterpolationError, RasterError, Tier, blend_display, blend_on_tier,
    tier_b_fragment_module, write_sample,
};

const SAMPLES: [f32; 4] = [0.0, 100.0, 200.0, 255.0];

fn bits(value: f32) -> u32 {
    value.to_bits()
}

fn opened(result: Result<f32, InterpolationError>) -> f32 {
    assert!(result.is_ok());
    match result {
        Ok(value) => value,
        Err(_) => unreachable!("the coordinate is inside the fixture"),
    }
}

#[test]
fn interpolation_nearest_follows_texel_of() {
    let value = opened(blend_display(2, &SAMPLES, 1.0, 0.0, Interpolation::Nearest));
    assert_eq!(bits(value), bits(100.0));
    let origin = opened(blend_display(2, &SAMPLES, 0.5, 0.5, Interpolation::Nearest));
    assert_eq!(bits(origin), bits(0.0));
}

#[test]
fn interpolation_linear_centre_is_one_quarter_of_each_texel() {
    let value = opened(blend_display(2, &SAMPLES, 0.5, 0.5, Interpolation::Linear));
    assert_eq!(bits(value), bits(138.75));
    for corner in SAMPLES {
        assert_ne!(bits(value), bits(corner));
    }
    // (0.25, 0.75) weights 0.1875, 0.0625, 0.5625, 0.1875.
    // 0*0.1875 + 100*0.0625 + 200*0.5625 + 255*0.1875 = 166.5625.
    // Swapping the two middle corners yields 116.5625, so the centre
    // fixture alone cannot see that swap.
    let off = opened(blend_display(
        2,
        &SAMPLES,
        0.25,
        0.75,
        Interpolation::Linear,
    ));
    assert_eq!(bits(off), bits(166.5625));
}

#[test]
fn interpolation_linear_negative_zero_is_the_texel_centre() {
    let value = opened(blend_display(1, &[5.0], -0.0, 0.0, Interpolation::Linear));
    assert_eq!(bits(value), bits(5.0));
}

#[test]
fn interpolation_linear_last_texel_centre_keeps_that_texel() {
    // Centre-origin 1 is the centre of texel 1. The missing neighbour has
    // weight 0, so the sample is 255 and not Outside.
    let centre = opened(blend_display(2, &SAMPLES, 1.0, 1.0, Interpolation::Linear));
    assert_eq!(bits(centre), bits(255.0));
    let edge = opened(blend_display(2, &SAMPLES, 1.0, 0.5, Interpolation::Linear));
    assert_eq!(bits(edge), bits(177.5));
}

#[test]
fn interpolation_outside_is_not_a_clamped_edge() {
    let left = blend_display(2, &SAMPLES, -0.1, 0.5, Interpolation::Nearest);
    assert!(matches!(left, Err(InterpolationError::Outside)));
    let past = blend_display(2, &SAMPLES, 2.0, 0.5, Interpolation::Linear);
    assert!(matches!(past, Err(InterpolationError::Outside)));
    let footprint = blend_display(2, &SAMPLES, 1.5, 0.5, Interpolation::Linear);
    assert!(matches!(footprint, Err(InterpolationError::Outside)));
    let nearest_edge = opened(blend_display(2, &SAMPLES, 1.5, 0.5, Interpolation::Nearest));
    assert_eq!(bits(nearest_edge), bits(100.0));
}

#[test]
fn interpolation_mode_two_leaves_the_sample_unchanged() {
    let mut slot = 7.0_f32;
    let refused = write_sample(2, 2, &SAMPLES, 0.5, 0.5, &mut slot);
    assert!(matches!(refused, Err(InterpolationError::Unavailable)));
    assert_eq!(bits(slot), bits(7.0));
}

#[test]
fn interpolation_cpu_mode_two_keeps_the_selected_mode() {
    let built = CpuRaster::open(1.0, 1.0, 1.0);
    assert!(built.is_ok());
    let mut raster = match built {
        Ok(raster) => raster,
        Err(_) => unreachable!("the canvas is finite"),
    };
    assert_eq!(raster.interpolation(), Interpolation::Nearest);
    assert!(raster.set_interpolation(1).is_ok());
    assert_eq!(raster.interpolation(), Interpolation::Linear);
    let refused = raster.set_interpolation(2);
    assert!(matches!(refused, Err(RasterError::Unavailable)));
    assert_eq!(raster.interpolation(), Interpolation::Linear);
}

#[test]
fn interpolation_tier_b_linear_refusal_is_not_nearest() {
    let request = TierSample {
        width: 2,
        samples: &SAMPLES,
        x: 0.5,
        y: 0.5,
        mode: Interpolation::Linear,
    };
    let refused = blend_on_tier(&request, Tier::B, false);
    assert!(matches!(refused, Err(InterpolationError::Unavailable)));
    assert!(available_rows());
}

fn available_rows() -> bool {
    for tier in [Tier::A, Tier::B, Tier::Cpu] {
        if !ocelli_render::available(Interpolation::Nearest, tier)
            || !ocelli_render::available(Interpolation::Linear, tier)
        {
            return false;
        }
    }
    true
}

#[test]
fn interpolation_shader_has_the_shared_weights_and_texture_load() {
    let shader = tier_b_fragment_module();
    assert!(shader.contains("textureLoad(stored_tex, texel, 0).r"));
    assert!(shader.contains(
        "(1.0 - tx) * (1.0 - ty) * p00 + tx * (1.0 - ty) * p10 + (1.0 - tx) * ty * p01 + tx * ty * p11"
    ));
    assert!(shader.contains("let mode = u32(camera.scale.y);"));
    assert!(
        shader
            .contains("(need_x && x0 + 1.0 >= f32(dims.x)) || (need_y && y0 + 1.0 >= f32(dims.y))")
    );
    assert!(!shader.contains("sampler"));
}
