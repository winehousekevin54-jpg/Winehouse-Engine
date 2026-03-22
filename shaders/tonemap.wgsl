// ============================================================
// Winehouse Engine — Tonemap + Bloom + Color Grade + 3D LUT (Phase C)
// ACES Filmic tonemap, gamma, bloom composite, exposure,
// saturation, contrast, and optional 3D LUT colour grade.
// ============================================================

struct ColorGradeUniforms {
    exposure:     f32,
    saturation:   f32,
    contrast:     f32,
    lut_strength: f32,
}

@group(0) @binding(0) var          hdr_tex:    texture_2d<f32>;
@group(0) @binding(1) var          bloom_tex:  texture_2d<f32>;
@group(0) @binding(2) var          linear_samp: sampler;
@group(0) @binding(3) var<uniform> grades:     ColorGradeUniforms;
@group(0) @binding(4) var          lut_tex:    texture_3d<f32>;

// ACES Filmic approximation (Hill, 2015)
fn aces_filmic(x: vec3<f32>) -> vec3<f32> {
    let a = 2.51;
    let b = vec3(0.03);
    let c = 2.43;
    let d = vec3(0.59);
    let e = vec3(0.14);
    return clamp((x * (a * x + b)) / (x * (c * x + d) + e), vec3(0.0), vec3(1.0));
}

fn adjust_saturation(color: vec3<f32>, sat: f32) -> vec3<f32> {
    let lum = dot(color, vec3<f32>(0.2126, 0.7152, 0.0722));
    return mix(vec3<f32>(lum), color, sat);
}

fn adjust_contrast(color: vec3<f32>, con: f32) -> vec3<f32> {
    return clamp((color - 0.5) * con + 0.5, vec3(0.0), vec3(1.0));
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
    let hdr_dims = vec2<f32>(textureDimensions(hdr_tex));
    let uv       = frag_coord.xy / hdr_dims;

    let hdr   = textureSample(hdr_tex,   linear_samp, uv).rgb;
    let bloom = textureSample(bloom_tex, linear_samp, uv).rgb;

    // Bloom composite + exposure (applied in linear/HDR space)
    var color = (hdr + bloom * 0.25) * grades.exposure;

    // ACES tonemap
    color = aces_filmic(color);

    // Gamma correction (sRGB ≈ pow(x, 1/2.2))
    color = pow(max(color, vec3(0.0)), vec3(1.0 / 2.2));

    // Saturation & contrast adjustments (LDR / sRGB space)
    color = adjust_saturation(color, grades.saturation);
    color = adjust_contrast(color, grades.contrast);

    // 3D LUT lookup (sRGB space, trilinear)
    let lut_sample = textureSample(lut_tex, linear_samp, clamp(color, vec3(0.0), vec3(1.0))).rgb;
    color = mix(color, lut_sample, grades.lut_strength);

    return vec4<f32>(clamp(color, vec3(0.0), vec3(1.0)), 1.0);
}
