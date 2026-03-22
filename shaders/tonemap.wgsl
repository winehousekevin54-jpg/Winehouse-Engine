// ============================================================
// Winehouse Engine — Tonemap + Bloom Composite (Phase 4)
// ACES Filmic tonemap, gamma correction, bloom additive blend.
// ============================================================

@group(0) @binding(0) var hdr_tex:      texture_2d<f32>;
@group(0) @binding(1) var bloom_tex:    texture_2d<f32>;
@group(0) @binding(2) var linear_samp:  sampler;

// ACES Filmic approximation (Hill, 2015)
fn aces_filmic(x: vec3<f32>) -> vec3<f32> {
    let a = 2.51;
    let b = vec3(0.03);
    let c = 2.43;
    let d = vec3(0.59);
    let e = vec3(0.14);
    return clamp((x * (a * x + b)) / (x * (c * x + d) + e), vec3(0.0), vec3(1.0));
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
    let hdr_dims   = vec2<f32>(textureDimensions(hdr_tex));
    let bloom_dims = vec2<f32>(textureDimensions(bloom_tex));
    let uv         = frag_coord.xy / hdr_dims;

    let hdr   = textureSample(hdr_tex,   linear_samp, uv).rgb;
    let bloom = textureSample(bloom_tex, linear_samp, uv).rgb;

    // Additive bloom composite
    var color = hdr + bloom * 0.25;

    // ACES tonemap
    color = aces_filmic(color);

    // Gamma correction (sRGB ≈ pow(x, 1/2.2))
    color = pow(max(color, vec3(0.0)), vec3(1.0 / 2.2));

    return vec4<f32>(color, 1.0);
}
