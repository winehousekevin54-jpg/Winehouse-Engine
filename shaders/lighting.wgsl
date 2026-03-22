// ============================================================
// Winehouse Engine — Deferred Lighting Pass (Phase 4)
// Cook-Torrance PBR + Cascaded Shadow Maps (4 cascades) + SSAO.
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
    cascade_vp:      array<mat4x4<f32>, 4>,
    cascade_splits:  vec4<f32>,
}

@group(0) @binding(0) var<uniform> scene:         SceneUniforms;
@group(0) @binding(1) var<uniform> lighting:      LightingUniforms;
@group(1) @binding(0) var gbuffer_albedo:         texture_2d<f32>;
@group(1) @binding(1) var gbuffer_normal:         texture_2d<f32>;
@group(1) @binding(2) var gbuffer_depth:          texture_depth_2d;
@group(1) @binding(3) var shadow_map:             texture_depth_2d_array;
@group(1) @binding(4) var shadow_sampler:         sampler_comparison;
@group(1) @binding(5) var ssao_tex:               texture_2d<f32>;
@group(1) @binding(6) var linear_sampler:         sampler;

const PI: f32 = 3.14159265358979;
const SHADOW_BIAS: f32 = 0.005;
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

// ── PCSS (Percentage-Closer Soft Shadows) with Cascaded Shadow Maps ───────────
// 1. Blocker search (textureLoad, 16 Poisson samples) → average blocker depth
// 2. Penumbra estimation from blocker-to-receiver distance ratio
// 3. Variable-radius PCF (textureSampleCompare, 16 Poisson samples) → soft shadow
// All textureSampleCompare calls remain in uniform control flow.

const LIGHT_SIZE: f32 = 0.02;   // Apparent light radius in shadow UV space
const PCSS_SAMPLES: i32 = 16;

// 16-sample Poisson disk (unit-disk distribution)
const POISSON_DISK: array<vec2<f32>, 16> = array<vec2<f32>, 16>(
    vec2<f32>(-0.9465, -0.1416),
    vec2<f32>(-0.5753,  0.5960),
    vec2<f32>(-0.2039, -0.4018),
    vec2<f32>( 0.1494,  0.8537),
    vec2<f32>(-0.6863, -0.6395),
    vec2<f32>( 0.4177,  0.2781),
    vec2<f32>( 0.0729, -0.8806),
    vec2<f32>( 0.6862,  0.6523),
    vec2<f32>(-0.3625,  0.1653),
    vec2<f32>( 0.8438, -0.0347),
    vec2<f32>(-0.1247, -0.0057),
    vec2<f32>( 0.4125, -0.4525),
    vec2<f32>(-0.7802,  0.2893),
    vec2<f32>( 0.2601, -0.2078),
    vec2<f32>(-0.4398, -0.8698),
    vec2<f32>( 0.6967, -0.6345),
);

fn linearize_depth(d: f32) -> f32 {
    return lighting.near * lighting.far / (lighting.far - d * (lighting.far - lighting.near));
}

fn cascade_shadow(world_pos: vec3<f32>, depth: f32) -> f32 {
    let view_z = linearize_depth(depth);

    // Select cascade — cascade_splits.xyzw = split distances for cascades 0–3
    var idx: i32 = 3;
    if (view_z < lighting.cascade_splits.x) { idx = 0; }
    else if (view_z < lighting.cascade_splits.y) { idx = 1; }
    else if (view_z < lighting.cascade_splits.z) { idx = 2; }

    let light_clip = lighting.cascade_vp[idx] * vec4<f32>(world_pos, 1.0);
    let proj       = light_clip.xyz / light_clip.w;

    // Map from NDC [-1,1] to UV [0,1]; flip Y
    let uv = vec2<f32>(proj.x * 0.5 + 0.5, 1.0 - (proj.y * 0.5 + 0.5));

    let receiver_depth = proj.z;
    let texel          = 1.0 / SHADOW_SIZE;

    // ── Step 1: Blocker search (textureLoad — no control-flow restrictions) ──
    var blocker_sum   = 0.0;
    var blocker_count = 0;
    for (var i = 0; i < PCSS_SAMPLES; i++) {
        let sample_uv = uv + POISSON_DISK[i] * LIGHT_SIZE;
        let tc = vec2<i32>(sample_uv * SHADOW_SIZE);
        let blocker_d = textureLoad(shadow_map, tc, idx, 0);
        if (blocker_d < receiver_depth) {
            blocker_sum += blocker_d;
            blocker_count++;
        }
    }

    // ── Step 2: Penumbra estimation ─────────────────────────────────────────
    // If no blockers, avg_blocker = receiver_depth → penumbra = 0 → min radius
    let avg_blocker   = select(receiver_depth, blocker_sum / f32(max(blocker_count, 1)), blocker_count > 0);
    let penumbra      = max(receiver_depth - avg_blocker, 0.0) / max(avg_blocker, 0.0001) * LIGHT_SIZE;
    let filter_radius = clamp(penumbra, texel, LIGHT_SIZE * 4.0);

    // ── Step 3: Variable-radius PCF (uniform control flow) ──────────────────
    let d_ref  = receiver_depth - SHADOW_BIAS;
    var shadow = 0.0;
    for (var i = 0; i < PCSS_SAMPLES; i++) {
        let offset = POISSON_DISK[i] * filter_radius;
        shadow += textureSampleCompare(shadow_map, shadow_sampler, uv + offset, idx, d_ref);
    }

    let in_bounds = uv.x >= 0.0 && uv.x <= 1.0 && uv.y >= 0.0 && uv.y <= 1.0;
    // No blockers → fully lit; otherwise use PCF result
    let result = select(1.0, shadow / f32(PCSS_SAMPLES), blocker_count > 0);
    return select(1.0, result, in_bounds);
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
    let shadow = cascade_shadow(world_pos, depth);

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

    // ── Hemisphere ambient (simulates open-sky IBL without an env texture) ──────
    // Sky colour (blue-white) for upward-facing normals; warm ground for downward.
    // Reflection direction is used for metallic specular approximation.
    let sky_col    = vec3<f32>(0.55, 0.65, 0.85);   // daylight sky
    let ground_col = vec3<f32>(0.06, 0.05, 0.04);   // dark earth
    let t_diff     = clamp(N.y * 0.5 + 0.5, 0.0, 1.0);
    let hemi_diff  = mix(ground_col, sky_col, t_diff);

    // Fake env specular: sample hemisphere in reflection direction
    let R          = reflect(-V, N);
    let t_spec     = clamp(R.y * 0.5 + 0.5, 0.0, 1.0);
    let hemi_spec  = mix(ground_col, sky_col, t_spec);

    // Split-sum ambient: dielectric uses diffuse hemi, metallic reflects hemi
    let kD_amb     = (1.0 - metallic);
    let amb_diff   = hemi_diff * albedo * kD_amb;
    let amb_spec   = hemi_spec * F0 * (1.0 - roughness * roughness);
    let ambient    = (amb_diff + amb_spec) * ssao;

    return vec4<f32>(ambient + Lo, 1.0);
}
