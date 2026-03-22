// ============================================================
// Winehouse Engine — Deferred Lighting Pass (Phase 4)
// Cook-Torrance PBR + PCF directional shadow + SSAO.
// Outputs HDR Rgba16Float.
// ============================================================

struct SceneUniforms {
    view_proj:            mat4x4<f32>,
    unjittered_view_proj: mat4x4<f32>,
    prev_view_proj:       mat4x4<f32>,
    camera_pos:           vec3<f32>,
    _pad0:                f32,
    light_dir:            vec3<f32>,
    _pad1:                f32,
    light_color:          vec3<f32>,
    _pad2:                f32,
    ambient_color:        vec3<f32>,
    _pad3:                f32,
}

struct LightingUniforms {
    inv_view_proj:   mat4x4<f32>,
    light_view_proj: mat4x4<f32>,
    view_proj:       mat4x4<f32>,
    viewport:        vec2<f32>,
    near:            f32,
    far:             f32,
}

@group(0) @binding(0) var<uniform> scene:         SceneUniforms;
@group(0) @binding(1) var<uniform> lighting:      LightingUniforms;
@group(1) @binding(0) var gbuffer_albedo:         texture_2d<f32>;
@group(1) @binding(1) var gbuffer_normal:         texture_2d<f32>;
@group(1) @binding(2) var gbuffer_depth:          texture_depth_2d;
@group(1) @binding(3) var shadow_map:             texture_depth_2d;
@group(1) @binding(4) var shadow_sampler:         sampler_comparison;
@group(1) @binding(5) var ssao_tex:               texture_2d<f32>;
@group(1) @binding(6) var linear_sampler:         sampler;

const PI: f32 = 3.14159265358979;
const SHADOW_BIAS: f32 = 0.003;
const SHADOW_SIZE: f32 = 2048.0;

// ── PBR helpers ───────────────────────────────────────────────────────────────

fn distribution_ggx(N: vec3<f32>, H: vec3<f32>, roughness: f32) -> f32 {
    let a  = roughness * roughness;
    let a2 = a * a;
    let d  = max(dot(N, H), 0.0);
    let t  = d * d * (a2 - 1.0) + 1.0;
    return a2 / (PI * t * t);
}

fn geometry_schlick_ggx(NdotV: f32, roughness: f32) -> f32 {
    let r = roughness + 1.0;
    let k = (r * r) / 8.0;
    return NdotV / (NdotV * (1.0 - k) + k);
}

fn geometry_smith(N: vec3<f32>, V: vec3<f32>, L: vec3<f32>, roughness: f32) -> f32 {
    let NdotV = max(dot(N, V), 0.0);
    let NdotL = max(dot(N, L), 0.0);
    return geometry_schlick_ggx(NdotV, roughness)
         * geometry_schlick_ggx(NdotL, roughness);
}

fn fresnel_schlick(cos_theta: f32, F0: vec3<f32>) -> vec3<f32> {
    return F0 + (1.0 - F0) * pow(clamp(1.0 - cos_theta, 0.0, 1.0), 5.0);
}

// ── PCF Shadow (3×3 kernel) ───────────────────────────────────────────────────
// NOTE: No early-return inside this function — textureSampleCompare requires
// uniform control flow. Use select() for bounds check instead.

fn pcf_shadow(world_pos: vec3<f32>) -> f32 {
    let light_clip = lighting.light_view_proj * vec4<f32>(world_pos, 1.0);
    let proj       = light_clip.xyz / light_clip.w;

    // Map from NDC [-1,1] to UV [0,1]; flip Y
    let uv = vec2<f32>(proj.x * 0.5 + 0.5, 1.0 - (proj.y * 0.5 + 0.5));

    let depth    = proj.z - SHADOW_BIAS;
    let texel    = 1.0 / SHADOW_SIZE;
    var shadow   = 0.0;
    for (var x = -1; x <= 1; x++) {
        for (var y = -1; y <= 1; y++) {
            let offset = vec2<f32>(f32(x), f32(y)) * texel;
            shadow += textureSampleCompare(shadow_map, shadow_sampler, uv + offset, depth);
        }
    }
    // select() instead of early-return: avoids non-uniform control flow
    let in_bounds = uv.x >= 0.0 && uv.x <= 1.0 && uv.y >= 0.0 && uv.y <= 1.0;
    return select(1.0, shadow / 9.0, in_bounds);
}

// ── Fullscreen triangle vertex ─────────────────────────────────────────────────

@vertex
fn vs_main(@builtin(vertex_index) idx: u32) -> @builtin(position) vec4<f32> {
    var pos = array<vec2<f32>, 3>(
        vec2<f32>(-1.0, -1.0),
        vec2<f32>( 3.0, -1.0),
        vec2<f32>(-1.0,  3.0),
    );
    return vec4<f32>(pos[idx], 0.0, 1.0);
}

// ── Deferred lighting fragment ─────────────────────────────────────────────────

@fragment
fn fs_main(@builtin(position) frag_coord: vec4<f32>) -> @location(0) vec4<f32> {
    let uv    = frag_coord.xy / lighting.viewport;
    let texel = vec2<i32>(frag_coord.xy);

    let depth = textureLoad(gbuffer_depth, texel, 0);

    // Reconstruct world position before sky check (needed for pcf_shadow below)
    let ndc       = vec4<f32>(uv.x * 2.0 - 1.0, (1.0 - uv.y) * 2.0 - 1.0, depth, 1.0);
    let world_h   = lighting.inv_view_proj * ndc;
    let world_pos = world_h.xyz / world_h.w;

    // textureSample and textureSampleCompare MUST be called before any non-uniform
    // branch (WGSL uniform control flow requirement).
    let ssao   = textureSample(ssao_tex, linear_sampler, uv).r;
    let shadow = pcf_shadow(world_pos);

    // Sky / background — return after texture calls to satisfy uniform control flow
    if (depth >= 1.0) {
        return vec4<f32>(0.07, 0.07, 0.10, 1.0);
    }

    // ── Sample G-Buffer ─────────────────────────────────────────────────────────
    let albedo_rough  = textureLoad(gbuffer_albedo, texel, 0);
    let normal_metal  = textureLoad(gbuffer_normal, texel, 0);

    let albedo    = albedo_rough.rgb;
    let roughness = albedo_rough.a;
    let N         = normalize(normal_metal.rgb * 2.0 - 1.0);
    let metallic  = normal_metal.a;

    // ── PBR Shading ─────────────────────────────────────────────────────────────
    let V  = normalize(scene.camera_pos - world_pos);
    let L  = normalize(-scene.light_dir);
    let H  = normalize(V + L);

    var F0 = vec3<f32>(0.04);
    F0     = mix(F0, albedo, metallic);

    let NDF      = distribution_ggx(N, H, roughness);
    let G        = geometry_smith(N, V, L, roughness);
    let F        = fresnel_schlick(max(dot(H, V), 0.0), F0);

    let NdotL    = max(dot(N, L), 0.0);
    let kD       = (vec3<f32>(1.0) - F) * (1.0 - metallic);
    let denom    = 4.0 * max(dot(N, V), 0.0) * NdotL + 0.0001;
    let specular = (NDF * G * F) / denom;

    let Lo      = (kD * albedo / PI + specular) * scene.light_color * NdotL * shadow;
    let ambient = scene.ambient_color * albedo * ssao;

    return vec4<f32>(ambient + Lo, 1.0);
}
