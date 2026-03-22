// ============================================================
// Winehouse Engine — Volumetric Composite Pass
// Bilateral upscale from half-res volumetric to full-res HDR.
// Weights half-res samples by depth similarity to avoid halo
// artifacts at geometry edges. Additive blend into the scene.
// ============================================================

@group(0) @binding(0) var t_hdr:        texture_2d<f32>;
@group(0) @binding(1) var t_volumetric: texture_2d<f32>;
@group(0) @binding(2) var t_depth:      texture_depth_2d;
@group(0) @binding(3) var s_linear:     sampler;

@vertex
fn vs_main(@builtin(vertex_index) vi: u32) -> @builtin(position) vec4<f32> {
    let uv = vec2<f32>(f32((vi << 1u) & 2u), f32(vi & 2u));
    return vec4<f32>(uv * 2.0 - 1.0, 0.0, 1.0);
}

@fragment
fn fs_main(@builtin(position) frag_coord: vec4<f32>) -> @location(0) vec4<f32> {
    let full_dims = vec2<f32>(textureDimensions(t_depth));
    let half_dims = vec2<f32>(textureDimensions(t_volumetric));
    let uv = frag_coord.xy / full_dims;

    // Full-res HDR — no upscaling needed
    let hdr_color = textureSample(t_hdr, s_linear, uv);

    // Full-res depth at this pixel (reference for bilateral weights)
    let full_texel  = vec2<i32>(frag_coord.xy);
    let full_depth  = textureLoad(t_depth, full_texel, 0);

    // ── Bilateral 2×2 gather ─────────────────────────────────────────────────
    // The half-res pixel that maps to this full-res pixel
    let half_base = vec2<i32>(full_texel / 2);
    let half_max  = vec2<i32>(half_dims) - vec2<i32>(1);

    // Depth difference threshold (sigma_d): tighter = sharper edge preservation
    let sigma_d = 0.1;

    var vol_sum    = vec3<f32>(0.0);
    var weight_sum = 0.0;

    // 2×2 neighbourhood in half-res space
    for (var dy = 0; dy <= 1; dy++) {
        for (var dx = 0; dx <= 1; dx++) {
            let half_texel = clamp(half_base + vec2<i32>(dx, dy), vec2<i32>(0), half_max);

            // Sample corresponding full-res depth for this half-res pixel
            let full_ref_texel = clamp(half_texel * 2, vec2<i32>(0), vec2<i32>(full_dims) - vec2<i32>(1));
            let sample_depth   = textureLoad(t_depth, full_ref_texel, 0);

            // Bilateral weight: Gaussian on depth difference
            let depth_diff = abs(full_depth - sample_depth);
            let w = exp(-depth_diff / sigma_d);

            let vol_sample = textureLoad(t_volumetric, half_texel, 0).rgb;
            vol_sum    += vol_sample * w;
            weight_sum += w;
        }
    }

    // Normalise — fallback to 1.0 weight avoids div-by-zero on sky
    let vol_color = vol_sum / max(weight_sum, 0.0001);

    // Additive composite
    return vec4<f32>(hdr_color.rgb + vol_color, 1.0);
}
