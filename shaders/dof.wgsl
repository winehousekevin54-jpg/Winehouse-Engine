// ============================================================
// Winehouse Engine — Depth of Field (Phase C)
// Single-pass gather bokeh with inline Circle-of-Confusion.
// 16-sample Poisson disk, weighted by sample CoC.
// ============================================================

struct DofUniforms {
    viewport:       vec2<f32>,
    focal_distance: f32,
    focal_range:    f32,
    bokeh_radius:   f32,
    near:           f32,
    far:            f32,
    _pad:           f32,
}

@group(0) @binding(0) var<uniform> dof:      DofUniforms;
@group(0) @binding(1) var          t_color:  texture_2d<f32>;
@group(0) @binding(2) var          t_depth:  texture_depth_2d;
@group(0) @binding(3) var          s_linear: sampler;

fn linearize_depth(d: f32) -> f32 {
    return dof.near * dof.far / (dof.far - d * (dof.far - dof.near));
}

// Normalised Circle-of-Confusion: 0 = in-focus, 1 = max blur
fn compute_coc(lin_depth: f32) -> f32 {
    return clamp(abs(lin_depth - dof.focal_distance) / max(dof.focal_range, 0.001), 0.0, 1.0);
}

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
    let uv    = frag_coord.xy / dof.viewport;
    let texel = 1.0 / dof.viewport;

    // Centre pixel depth & CoC
    let center_raw = textureLoad(t_depth, pixel, 0);

    // Sky: no DoF
    if center_raw >= 1.0 {
        return textureSampleLevel(t_color, s_linear, uv, 0.0);
    }

    let center_lin = linearize_depth(center_raw);
    let center_coc = compute_coc(center_lin);
    let radius_px  = center_coc * dof.bokeh_radius;

    // Nearly in-focus — skip gather
    if radius_px < 0.5 {
        return textureSampleLevel(t_color, s_linear, uv, 0.0);
    }

    // 16-sample Poisson disk (Ward 1994)
    var disk: array<vec2<f32>, 16> = array<vec2<f32>, 16>(
        vec2( -0.94201624, -0.39906216),
        vec2(  0.94558609, -0.76890725),
        vec2( -0.09418410, -0.92938870),
        vec2(  0.34495938,  0.29387760),
        vec2( -0.91588581,  0.45771432),
        vec2( -0.81544232, -0.87912464),
        vec2( -0.38277543,  0.27676845),
        vec2(  0.97484398,  0.75648379),
        vec2(  0.44323325, -0.97511554),
        vec2(  0.53742981, -0.47373420),
        vec2( -0.26496911, -0.41893023),
        vec2(  0.79197514,  0.19090188),
        vec2( -0.24188840,  0.99706507),
        vec2( -0.81409955,  0.91437590),
        vec2(  0.19984126,  0.78641367),
        vec2(  0.14383161, -0.14100790),
    );

    var color_sum  = vec4<f32>(0.0);
    var weight_sum = 0.0;

    for (var i: i32 = 0; i < 16; i++) {
        let offset_uv  = disk[i] * radius_px * texel;
        let sample_uv  = clamp(uv + offset_uv, vec2(0.0), vec2(1.0));
        let sample_px  = vec2<i32>(sample_uv * dof.viewport);
        let sample_raw = textureLoad(t_depth, sample_px, 0);
        // If sample hits sky, treat it as having the centre depth (avoids huge CoC for sky taps)
        let used_raw   = select(center_raw, sample_raw, sample_raw < 1.0);
        let sample_lin = linearize_depth(used_raw);
        let sample_coc = compute_coc(sample_lin);

        // Upweight out-of-focus samples to encourage bokeh spread
        let w = max(sample_coc, 0.01);
        color_sum  += textureSampleLevel(t_color, s_linear, sample_uv, 0.0) * w;
        weight_sum += w;
    }

    return color_sum / max(weight_sum, 0.0001);
}
