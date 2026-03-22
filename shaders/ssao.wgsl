// ============================================================
// Winehouse Engine — SSAO Pass (Phase 4)
// Screen-Space Ambient Occlusion using hemisphere sampling.
// Outputs: R8Unorm occlusion factor (1.0 = fully lit, 0.0 = fully occluded)
// ============================================================

struct LightingUniforms {
    inv_view_proj:   mat4x4<f32>,
    light_view_proj: mat4x4<f32>,
    view_proj:       mat4x4<f32>,
    viewport:        vec2<f32>,
    near:            f32,
    far:             f32,
}

// binding(1) = lighting_buffer in lighting_uniforms_bg
// (binding(0) = scene_buffer which is SceneUniforms, not needed here)
@group(0) @binding(1) var<uniform> lighting:      LightingUniforms;
@group(1) @binding(0) var gbuffer_normal:          texture_2d<f32>;
@group(1) @binding(1) var gbuffer_depth:           texture_depth_2d;
@group(1) @binding(2) var noise_tex:               texture_2d<f32>;
@group(1) @binding(3) var repeat_sampler:          sampler;

const KERNEL_SIZE: i32 = 16;
const RADIUS:      f32 = 0.8;
const BIAS:        f32 = 0.015;

// Hardcoded hemisphere kernel (z > 0 = towards normal).
// Samples are accelerated toward origin with quadratic falloff.
fn kernel_sample(i: i32) -> vec3<f32> {
    switch i {
        case  0: { return normalize(vec3( 0.5381,  0.1856,  0.4319)) * 0.1; }
        case  1: { return normalize(vec3( 0.1379,  0.2486,  0.4430)) * 0.19; }
        case  2: { return normalize(vec3( 0.3371,  0.5679,  0.0057)) * 0.29; }
        case  3: { return normalize(vec3( 0.0689,  0.8236,  0.3216)) * 0.39; }
        case  4: { return normalize(vec3(-0.0023,  0.2386,  0.9621)) * 0.40; }
        case  5: { return normalize(vec3(-0.4321,  0.1236,  0.5432)) * 0.45; }
        case  6: { return normalize(vec3( 0.2341,  0.6321,  0.3241)) * 0.50; }
        case  7: { return normalize(vec3( 0.0123,  0.4321,  0.7312)) * 0.55; }
        case  8: { return normalize(vec3(-0.1234,  0.7321,  0.2134)) * 0.60; }
        case  9: { return normalize(vec3( 0.6213,  0.1234,  0.5432)) * 0.65; }
        case 10: { return normalize(vec3(-0.3421,  0.5432,  0.4312)) * 0.70; }
        case 11: { return normalize(vec3( 0.1234,  0.3421,  0.8123)) * 0.75; }
        case 12: { return normalize(vec3(-0.5432,  0.2134,  0.3214)) * 0.80; }
        case 13: { return normalize(vec3( 0.3214,  0.6543,  0.1234)) * 0.85; }
        case 14: { return normalize(vec3(-0.1023,  0.8213,  0.0543)) * 0.90; }
        case 15: { return normalize(vec3( 0.0543,  0.9012,  0.1234)) * 1.00; }
        default: { return vec3(0.0, 0.0, 1.0); }
    }
}

fn reconstruct_world(uv: vec2<f32>, depth: f32) -> vec3<f32> {
    let ndc     = vec4<f32>(uv.x * 2.0 - 1.0, (1.0 - uv.y) * 2.0 - 1.0, depth, 1.0);
    let world_h = lighting.inv_view_proj * ndc;
    return world_h.xyz / world_h.w;
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
    let uv    = frag_coord.xy / lighting.viewport;
    let texel = vec2<i32>(frag_coord.xy);
    let depth = textureLoad(gbuffer_depth, texel, 0);

    // textureSample MUST be called before any non-uniform branch (WGSL rule).
    // Call it here unconditionally, then do the sky check below.
    let noise_scale = lighting.viewport / 4.0;
    let random_vec  = textureSample(noise_tex, repeat_sampler, uv * noise_scale).xyz * 2.0 - 1.0;

    // Sky: no occlusion
    if (depth >= 1.0) { return vec4<f32>(1.0, 0.0, 0.0, 1.0); }

    let world_pos = reconstruct_world(uv, depth);
    let normal    = normalize(textureLoad(gbuffer_normal, texel, 0).rgb * 2.0 - 1.0);

    // Build TBN to orient kernel along surface normal
    let tangent   = normalize(random_vec - normal * dot(random_vec, normal));
    let bitangent = cross(normal, tangent);
    let TBN       = mat3x3<f32>(tangent, bitangent, normal);

    var occlusion = 0.0;
    for (var i = 0; i < KERNEL_SIZE; i++) {
        let sample_vec = TBN * kernel_sample(i);
        let sample_pos = world_pos + sample_vec * RADIUS;

        // Project sample into clip space
        let sample_clip = lighting.view_proj * vec4<f32>(sample_pos, 1.0);
        let sample_ndc  = sample_clip.xyz / sample_clip.w;
        let sample_uv   = vec2<f32>(sample_ndc.x * 0.5 + 0.5, 1.0 - (sample_ndc.y * 0.5 + 0.5));

        // Skip samples outside screen
        if (sample_uv.x < 0.0 || sample_uv.x > 1.0 ||
            sample_uv.y < 0.0 || sample_uv.y > 1.0) { continue; }

        let scene_texel = vec2<i32>(sample_uv * lighting.viewport);
        let scene_depth = textureLoad(gbuffer_depth, scene_texel, 0);
        let scene_pos   = reconstruct_world(sample_uv, scene_depth);

        // Range check: ignore occluders further than RADIUS
        let range_check = smoothstep(0.0, 1.0, RADIUS / distance(world_pos, scene_pos));
        // Occluded if scene geometry is closer to camera (smaller NDC depth)
        if (scene_depth < sample_ndc.z - BIAS) {
            occlusion += range_check;
        }
    }

    return vec4<f32>(1.0 - (occlusion / f32(KERNEL_SIZE)), 0.0, 0.0, 1.0);
}
