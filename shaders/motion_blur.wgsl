// ============================================================
// Winehouse Engine — Motion Blur (Phase C)
// Velocity-based cinematic motion blur.
// Samples along the per-pixel velocity vector (UV-space).
// ============================================================

struct MotionBlurUniforms {
    viewport:     vec2<f32>,
    max_blur_px:  f32,
    sample_count: f32,
}

@group(0) @binding(0) var<uniform> mb:         MotionBlurUniforms;
@group(0) @binding(1) var          t_color:    texture_2d<f32>;
@group(0) @binding(2) var          t_velocity: texture_2d<f32>;
@group(0) @binding(3) var          t_depth:    texture_depth_2d;
@group(0) @binding(4) var          s_linear:   sampler;

@vertex
fn vs_main(@builtin(vertex_index) idx: u32) -> @builtin(position) vec4<f32> {
    var pos = array<vec2<f32>, 3>(
        vec2<f32>(-1.0, -1.0),
        vec2<f32>( 3.0, -1.0),
        vec2<f32>(-1.0,  3.0),
    );
    return vec4<f32>(pos[idx], 0.0, 1.0);
}

@fragment
fn fs_main(@builtin(position) frag_coord: vec4<f32>) -> @location(0) vec4<f32> {
    let pixel = vec2<i32>(frag_coord.xy);
    let uv    = frag_coord.xy / mb.viewport;

    // Sky pixels: no motion blur
    let depth = textureLoad(t_depth, pixel, 0);
    if depth >= 1.0 {
        return textureSampleLevel(t_color, s_linear, uv, 0.0);
    }

    // Velocity is stored as UV-space delta (see gbuffer.wgsl: (curr_ndc - prev_ndc) * 0.5)
    let vel_uv  = textureLoad(t_velocity, pixel, 0).xy;

    // Convert UV-space velocity to pixel-space length
    let vel_px  = vel_uv * mb.viewport;
    let vel_len = length(vel_px);

    // Skip nearly-stationary pixels
    if vel_len < 0.5 {
        return textureLoad(t_color, pixel, 0);
    }

    let vel_dir     = vel_px / vel_len;
    let clamped_len = min(vel_len, mb.max_blur_px);
    let n           = max(i32(mb.sample_count), 3);

    var color      = vec4<f32>(0.0);
    var weight_sum = 0.0;

    for (var i: i32 = 0; i < n; i++) {
        // Distribute samples symmetrically: t in [-0.5, 0.5]
        let t         = (f32(i) / f32(n - 1)) - 0.5;
        let offset_px = vel_dir * clamped_len * t;
        let sample_uv = clamp((frag_coord.xy + offset_px) / mb.viewport, vec2(0.0), vec2(1.0));

        // Trapezoid weighting: upweight centre samples to reduce ghosting
        let w = 1.0 - abs(t);
        color      += textureSampleLevel(t_color, s_linear, sample_uv, 0.0) * w;
        weight_sum += w;
    }

    return color / max(weight_sum, 0.0001);
}
