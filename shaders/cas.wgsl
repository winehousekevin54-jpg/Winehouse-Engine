// ============================================================
// Winehouse Engine — CAS (Contrast Adaptive Sharpening)
// AMD FidelityFX CAS adapted for WGSL.
// Replaces FXAA — TAA handles anti-aliasing, CAS restores sharpness.
// ============================================================

struct CasUniforms {
    sharpness: f32,
    _pad0:     f32,
    _pad1:     f32,
    _pad2:     f32,
};

@group(0) @binding(0) var ldr_tex:     texture_2d<f32>;
@group(0) @binding(1) var linear_samp: sampler;
@group(0) @binding(2) var<uniform> cas: CasUniforms;

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
fn fs_main(@builtin(position) fc: vec4<f32>) -> @location(0) vec4<f32> {
    let coord = vec2<i32>(fc.xy);
    let dims  = vec2<i32>(textureDimensions(ldr_tex));

    // 5-tap cross: center + N/S/E/W
    let c = textureLoad(ldr_tex, coord, 0).rgb;
    let n = textureLoad(ldr_tex, clamp(coord + vec2<i32>( 0,  1), vec2<i32>(0), dims - 1), 0).rgb;
    let s = textureLoad(ldr_tex, clamp(coord + vec2<i32>( 0, -1), vec2<i32>(0), dims - 1), 0).rgb;
    let e = textureLoad(ldr_tex, clamp(coord + vec2<i32>( 1,  0), vec2<i32>(0), dims - 1), 0).rgb;
    let w = textureLoad(ldr_tex, clamp(coord + vec2<i32>(-1,  0), vec2<i32>(0), dims - 1), 0).rgb;

    // Min/max of the cross neighbourhood
    let mn = min(c, min(min(n, s), min(e, w)));
    let mx = max(c, max(max(n, s), max(e, w)));

    // AMD CAS peak sharpening amount (adaptive, edge-aware)
    // peak = sqrt(min(1/mn, 1/(1-mx))) — measures how much headroom we have
    // Higher contrast edges get less sharpening to avoid ringing
    let rcp_mx = 1.0 / max(mx, vec3<f32>(1.0 / 65536.0));
    let amp    = sqrt(min(mn * rcp_mx, (1.0 - mx) * rcp_mx));

    // Scale by user sharpness (0 = off, 1 = max)
    // CAS maps sharpness [0,1] to filter weight [-0.125 * 8, 0] adjusted
    let peak = amp * mix(-0.125, -0.2, cas.sharpness);

    // Apply sharpening: weighted sum of neighbours with negative weights
    let w_sum = 1.0 + 4.0 * peak;
    let result = (c + (n + s + e + w) * peak) / w_sum;

    return vec4<f32>(max(result, vec3<f32>(0.0)), 1.0);
}
