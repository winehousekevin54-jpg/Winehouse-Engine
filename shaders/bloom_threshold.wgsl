// ============================================================
// Winehouse Engine — Bloom Threshold Pass (Phase 4)
// Extracts HDR pixels above luminance threshold.
// ============================================================

@group(0) @binding(0) var hdr_tex:       texture_2d<f32>;
@group(0) @binding(1) var linear_samp:   sampler;

const THRESHOLD: f32 = 1.0;  // HDR luminance cutoff

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
    let dims  = vec2<f32>(textureDimensions(hdr_tex));
    let uv    = frag_coord.xy / dims;
    let color = textureSample(hdr_tex, linear_samp, uv).rgb;

    // Luminance (perceptual weights)
    let lum    = dot(color, vec3<f32>(0.2126, 0.7152, 0.0722));
    let bright = max(lum - THRESHOLD, 0.0);

    // Soft knee for smooth transition
    let factor = bright / max(lum, 0.0001);
    return vec4<f32>(color * factor, 1.0);
}
