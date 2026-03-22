// ============================================================
// Winehouse Engine — Screen-Space Reflections (SSR)
// Linear ray march + binary refinement in screen space.
// Output: Rgba16Float — reflected color (RGB) + confidence (A)
// ============================================================

struct SsrUniforms {
    view_proj:     mat4x4<f32>,
    inv_view_proj: mat4x4<f32>,
    viewport:      vec2<f32>,
    near:          f32,
    far:           f32,
    camera_pos:    vec3<f32>,
    max_distance:  f32,
}

@group(0) @binding(0) var<uniform> ssr: SsrUniforms;
@group(0) @binding(1) var t_hdr:     texture_2d<f32>;
@group(0) @binding(2) var t_depth:   texture_depth_2d;
@group(0) @binding(3) var t_normal:  texture_2d<f32>;
@group(0) @binding(4) var t_albedo:  texture_2d<f32>;
@group(0) @binding(5) var s_linear:  sampler;
@group(0) @binding(6) var s_point:   sampler;

@vertex
fn vs_main(@builtin(vertex_index) vi: u32) -> @builtin(position) vec4<f32> {
    let uv = vec2<f32>(f32((vi << 1u) & 2u), f32(vi & 2u));
    return vec4<f32>(uv * 2.0 - 1.0, 0.0, 1.0);
}

fn world_from_depth(uv: vec2<f32>, depth: f32) -> vec3<f32> {
    let ndc = vec4<f32>(uv * 2.0 - 1.0, depth, 1.0);
    let world = ssr.inv_view_proj * ndc;
    return world.xyz / world.w;
}

// textureLoad on depth — no sampler, safe in non-uniform control flow
fn load_depth(pixel: vec2<i32>) -> f32 {
    return textureLoad(t_depth, pixel, 0);
}

const MAX_STEPS: u32 = 48u;
const BINARY_STEPS: u32 = 6u;
const THICKNESS: f32 = 0.15;

@fragment
fn fs_main(@builtin(position) frag_coord: vec4<f32>) -> @location(0) vec4<f32> {
    let uv = frag_coord.xy / ssr.viewport;
    let pixel = vec2<i32>(frag_coord.xy);
    let depth = load_depth(pixel);

    // Skip sky pixels
    if depth >= 1.0 {
        return vec4<f32>(0.0);
    }

    // Read G-Buffer (textureSampleLevel — safe in non-uniform control flow)
    let normal_sample = textureSampleLevel(t_normal, s_linear, uv, 0.0);
    let albedo_sample = textureSampleLevel(t_albedo, s_linear, uv, 0.0);
    let world_normal = normalize(normal_sample.rgb * 2.0 - 1.0);
    let metallic   = normal_sample.a;
    let roughness  = albedo_sample.a;

    // Only reflect reasonably smooth or metallic surfaces
    if roughness > 0.6 && metallic < 0.2 {
        return vec4<f32>(0.0);
    }

    // Reconstruct world position and compute reflection direction
    let world_pos = world_from_depth(uv, depth);
    let view_dir  = normalize(world_pos - ssr.camera_pos);
    let reflect_dir = reflect(view_dir, world_normal);

    // March in screen space
    let start_clip = ssr.view_proj * vec4<f32>(world_pos, 1.0);
    let start_ndc  = start_clip.xyz / start_clip.w;
    let start_screen = start_ndc.xy * 0.5 + 0.5;

    let end_world  = world_pos + reflect_dir * ssr.max_distance;
    let end_clip   = ssr.view_proj * vec4<f32>(end_world, 1.0);
    let end_ndc    = end_clip.xyz / end_clip.w;
    let end_screen = end_ndc.xy * 0.5 + 0.5;

    let ray_screen = end_screen - start_screen;
    let ray_depth  = end_ndc.z - start_ndc.z;

    let step_size = 1.0 / f32(MAX_STEPS);
    var hit_uv = vec2<f32>(0.0);
    var hit = false;
    var hit_t = 0.0;

    for (var i = 0u; i < MAX_STEPS; i = i + 1u) {
        let t = (f32(i) + 0.5) * step_size;
        let sample_uv = start_screen + ray_screen * t;
        let sample_z  = start_ndc.z + ray_depth * t;

        if sample_uv.x < 0.0 || sample_uv.x > 1.0 || sample_uv.y < 0.0 || sample_uv.y > 1.0 {
            break;
        }

        let sample_pixel = vec2<i32>(sample_uv * ssr.viewport);
        let scene_depth = load_depth(sample_pixel);
        if scene_depth >= 1.0 { continue; }

        let depth_diff = sample_z - scene_depth;
        if depth_diff > 0.0 && depth_diff < THICKNESS {
            hit_uv = sample_uv;
            hit_t = t;
            hit = true;
            break;
        }
    }

    if !hit {
        return vec4<f32>(0.0);
    }

    // Binary refinement around the hit
    var lo = max(hit_t - step_size, 0.0);
    var hi = hit_t;

    for (var b = 0u; b < BINARY_STEPS; b = b + 1u) {
        let mid = (lo + hi) * 0.5;
        let mid_uv = start_screen + ray_screen * mid;
        let mid_z  = start_ndc.z + ray_depth * mid;
        let mid_pixel = vec2<i32>(mid_uv * ssr.viewport);
        let mid_depth = load_depth(mid_pixel);
        if mid_z > mid_depth {
            hi = mid;
        } else {
            lo = mid;
        }
    }
    hit_uv = start_screen + ray_screen * ((lo + hi) * 0.5);

    // Fetch reflected color (textureSampleLevel — safe in non-uniform flow)
    let reflected = textureSampleLevel(t_hdr, s_linear, hit_uv, 0.0);

    // Confidence: fade at screen borders and based on roughness
    let border = smoothstep(0.0, 0.05, hit_uv.x) * smoothstep(1.0, 0.95, hit_uv.x)
               * smoothstep(0.0, 0.05, hit_uv.y) * smoothstep(1.0, 0.95, hit_uv.y);
    let roughness_fade = 1.0 - smoothstep(0.2, 0.6, roughness);

    // Fresnel (Schlick approximation)
    let n_dot_v = max(dot(-view_dir, world_normal), 0.0);
    let f0 = mix(vec3<f32>(0.04), albedo_sample.rgb, metallic);
    let fresnel = f0 + (1.0 - f0) * pow(1.0 - n_dot_v, 5.0);
    let fresnel_weight = max(fresnel.r, max(fresnel.g, fresnel.b));

    let confidence = border * roughness_fade * fresnel_weight;
    return vec4<f32>(reflected.rgb, confidence);
}
