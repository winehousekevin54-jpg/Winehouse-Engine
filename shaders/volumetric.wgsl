// ============================================================
// Winehouse Engine — Volumetric Lighting (God Rays)
// Screen-space ray marching with CSM shadow sampling.
// Runs at half resolution; composited additively before TAA.
// ============================================================

struct VolumetricUniforms {
    scattering:   vec3<f32>,
    density:      f32,
    absorption:   vec3<f32>,
    g_factor:     f32,  // Henyey-Greenstein anisotropy
    max_distance: f32,
    steps:        f32,
    _pad:         vec2<f32>,
}

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

@group(0) @binding(0) var<uniform> scene:       SceneUniforms;
@group(0) @binding(1) var<uniform> lighting:    LightingUniforms;
@group(0) @binding(2) var          t_depth:     texture_depth_2d;
@group(0) @binding(3) var          t_shadow:    texture_depth_2d_array;
@group(0) @binding(4) var          s_shadow:    sampler_comparison;
@group(0) @binding(5) var          t_noise:     texture_2d<f32>;
@group(0) @binding(6) var          s_repeat:    sampler;
@group(0) @binding(7) var<uniform> vol:         VolumetricUniforms;

@vertex
fn vs_main(@builtin(vertex_index) vi: u32) -> @builtin(position) vec4<f32> {
    let uv = vec2<f32>(f32((vi << 1u) & 2u), f32(vi & 2u));
    return vec4<f32>(uv * 2.0 - 1.0, 0.0, 1.0);
}

// Reconstruct world position from depth + UV
fn world_from_depth(uv: vec2<f32>, depth: f32) -> vec3<f32> {
    let ndc = vec4<f32>(uv * 2.0 - 1.0, depth, 1.0);
    let world = lighting.inv_view_proj * ndc;
    return world.xyz / world.w;
}

// Linearize depth (WebGPU: z ∈ [0,1])
fn linearize_depth(d: f32) -> f32 {
    return lighting.near * lighting.far / (lighting.far - d * (lighting.far - lighting.near));
}

// Select cascade and return shadow visibility (0=shadow, 1=lit)
fn shadow_visibility(world_pos: vec3<f32>) -> f32 {
    let view_depth = linearize_depth(
        (lighting.view_proj * vec4<f32>(world_pos, 1.0)).z
    );
    var cascade = 3u;
    for (var i = 0u; i < 4u; i = i + 1u) {
        if view_depth < lighting.cascade_splits[i] {
            cascade = i;
            break;
        }
    }
    let light_clip = lighting.cascade_vp[cascade] * vec4<f32>(world_pos, 1.0);
    let light_ndc  = light_clip.xyz / light_clip.w;
    let shadow_uv  = light_ndc.xy * 0.5 + 0.5;
    let shadow_z   = light_ndc.z;
    return textureSampleCompareLevel(
        t_shadow, s_shadow,
        shadow_uv, i32(cascade), shadow_z
    );
}

// Henyey-Greenstein phase function
fn henyey_greenstein(cos_theta: f32, g: f32) -> f32 {
    let g2 = g * g;
    let denom = 1.0 + g2 - 2.0 * g * cos_theta;
    return (1.0 - g2) / (4.0 * 3.14159265 * pow(denom, 1.5));
}

@fragment
fn fs_main(@builtin(position) frag_coord: vec4<f32>) -> @location(0) vec4<f32> {
    // Renders at half-resolution; t_depth is full-resolution.
    // Map half-res frag_coord to full-res for depth lookup.
    let depth_dims = vec2<i32>(textureDimensions(t_depth));
    let depth_pixel = vec2<i32>(frag_coord.xy) * 2;
    let clamped_pixel = clamp(depth_pixel, vec2<i32>(0), depth_dims - vec2<i32>(1));
    // textureLoad — no sampler needed, safe with depth textures
    let depth = textureLoad(t_depth, clamped_pixel, 0);
    // Reconstruct UV at full resolution for world-position
    let uv = (vec2<f32>(clamped_pixel) + 0.5) / vec2<f32>(depth_dims);
    let world_pos = world_from_depth(uv, depth);

    let ray_origin = scene.camera_pos;
    let ray_dir    = normalize(world_pos - ray_origin);
    let ray_length = min(distance(world_pos, ray_origin), vol.max_distance);

    // Dither ray start with noise to reduce banding
    let noise_uv = frag_coord.xy / 4.0;
    let noise_val = textureSampleLevel(t_noise, s_repeat, noise_uv, 0.0).r;

    let step_count = u32(vol.steps);
    let step_len   = ray_length / f32(step_count);

    // Light direction (negated — scene.light_dir points toward the surface)
    let L = normalize(-scene.light_dir);
    let cos_theta = dot(ray_dir, L);
    let phase = henyey_greenstein(cos_theta, vol.g_factor);

    var accumulated = vec3<f32>(0.0);
    let transmittance_step = exp(-vol.density * step_len);

    for (var i = 0u; i < step_count; i = i + 1u) {
        let t = (f32(i) + noise_val) * step_len;
        let sample_pos = ray_origin + ray_dir * t;

        let vis = shadow_visibility(sample_pos);
        let in_scatter = vol.scattering * phase * vis * scene.light_color;
        accumulated += in_scatter * step_len;
    }

    // Apply density attenuation
    accumulated *= vol.density;

    return vec4<f32>(accumulated, 1.0);
}
