// ============================================================
// Winehouse Engine — Volumetric Composite Pass
// Bilateral upscale from half-res volumetric to full-res HDR.
// Additive blend into the scene.
// ============================================================

@group(0) @binding(0) var t_hdr:        texture_2d<f32>;
@group(0) @binding(1) var t_volumetric: texture_2d<f32>;
@group(0) @binding(2) var s_linear:     sampler;

@vertex
fn vs_main(@builtin(vertex_index) vi: u32) -> @builtin(position) vec4<f32> {
    let uv = vec2<f32>(f32((vi << 1u) & 2u), f32(vi & 2u));
    return vec4<f32>(uv * 2.0 - 1.0, 0.0, 1.0);
}

@fragment
fn fs_main(@builtin(position) frag_coord: vec4<f32>) -> @location(0) vec4<f32> {
    let dims = vec2<f32>(textureDimensions(t_hdr));
    let uv = frag_coord.xy / dims;
    let hdr_color  = textureSample(t_hdr, s_linear, uv);
    let vol_color  = textureSample(t_volumetric, s_linear, uv);

    // Additive composite
    return vec4<f32>(hdr_color.rgb + vol_color.rgb, 1.0);
}
