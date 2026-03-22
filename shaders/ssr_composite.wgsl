// ============================================================
// Winehouse Engine — SSR Composite Pass
// Blends SSR reflections into the HDR buffer.
// ============================================================

@group(0) @binding(0) var t_hdr:     texture_2d<f32>;
@group(0) @binding(1) var t_ssr:     texture_2d<f32>;
@group(0) @binding(2) var s_linear:  sampler;

@vertex
fn vs_main(@builtin(vertex_index) vi: u32) -> @builtin(position) vec4<f32> {
    let uv = vec2<f32>(f32((vi << 1u) & 2u), f32(vi & 2u));
    return vec4<f32>(uv * 2.0 - 1.0, 0.0, 1.0);
}

struct VsOut {
    @builtin(position) pos: vec4<f32>,
}

@fragment
fn fs_main(@builtin(position) frag_coord: vec4<f32>) -> @location(0) vec4<f32> {
    let dims = vec2<f32>(textureDimensions(t_hdr));
    let uv = frag_coord.xy / dims;
    let hdr_color = textureSample(t_hdr, s_linear, uv);
    let ssr_color = textureSample(t_ssr, s_linear, uv);

    // Blend: add reflected light modulated by confidence
    let result = hdr_color.rgb + ssr_color.rgb * ssr_color.a;
    return vec4<f32>(result, 1.0);
}
