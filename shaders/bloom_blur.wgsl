// ============================================================
// Winehouse Engine — Bloom Gaussian Blur (Phase 4)
// Separable 9-tap Gaussian — reused for horizontal AND vertical.
// Direction: [1,0] = horizontal, [0,1] = vertical
// ============================================================

struct BloomUniforms {
    direction:  vec2<f32>,  // [1,0] or [0,1]
    texel_size: vec2<f32>,  // 1.0 / texture_size
}

@group(0) @binding(0) var<uniform> bloom:       BloomUniforms;
@group(0) @binding(1) var input_tex:            texture_2d<f32>;
@group(0) @binding(2) var linear_samp:          sampler;

// 9-tap Gaussian weights (σ ≈ 1.5, normalized)
const W0: f32 = 0.0539909665;
const W1: f32 = 0.1216216216;
const W2: f32 = 0.1945945946;
const W3: f32 = 0.2270270270;

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
    let dims = vec2<f32>(textureDimensions(input_tex));
    let uv   = frag_coord.xy / dims;
    let step = bloom.direction * bloom.texel_size;

    var color = textureSample(input_tex, linear_samp, uv).rgb * W3;
    color += textureSample(input_tex, linear_samp, uv - step * 1.0).rgb * W2;
    color += textureSample(input_tex, linear_samp, uv + step * 1.0).rgb * W2;
    color += textureSample(input_tex, linear_samp, uv - step * 2.0).rgb * W1;
    color += textureSample(input_tex, linear_samp, uv + step * 2.0).rgb * W1;
    color += textureSample(input_tex, linear_samp, uv - step * 3.0).rgb * W0;
    color += textureSample(input_tex, linear_samp, uv + step * 3.0).rgb * W0;

    return vec4<f32>(color, 1.0);
}
