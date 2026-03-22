// ============================================================
// Winehouse Engine — SSAO Box Blur (Phase 4)
// 4x4 box blur to smooth SSAO noise.
// ============================================================

@group(0) @binding(0) var ssao_tex:     texture_2d<f32>;
@group(0) @binding(1) var point_samp:   sampler;

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
fn fs_main(@builtin(position) frag_coord: vec4<f32>) -> @location(0) f32 {
    let texel = vec2<i32>(frag_coord.xy);
    var result = 0.0;
    for (var x = -2; x <= 2; x++) {
        for (var y = -2; y <= 2; y++) {
            let offset = vec2<i32>(x, y);
            result += textureLoad(ssao_tex, texel + offset, 0).r;
        }
    }
    return result / 25.0;
}
