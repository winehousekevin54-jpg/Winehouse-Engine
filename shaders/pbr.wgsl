// ============================================================
// Winehouse Engine — Forward PBR Shader (Phase 1)
// Cook-Torrance BRDF, one directional light + ambient
// ============================================================

struct SceneUniforms {
    view_proj:            mat4x4<f32>,
    unjittered_view_proj: mat4x4<f32>,
    prev_view_proj:       mat4x4<f32>,
    camera_pos:           vec3<f32>,
    _pad0:                f32,
    light_dir:            vec3<f32>,   // normalized, pointing FROM light
    _pad1:                f32,
    light_color:          vec3<f32>,
    _pad2:                f32,
    ambient_color:        vec3<f32>,
    _pad3:                f32,
}

struct ObjectUniforms {
    model:      mat4x4<f32>,
    prev_model: mat4x4<f32>,
    albedo:     vec4<f32>,
    metallic:   f32,
    roughness:  f32,
    _pad:       vec2<f32>,
}

@group(0) @binding(0) var<uniform> scene: SceneUniforms;
@group(1) @binding(0) var<uniform> object: ObjectUniforms;

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) normal:   vec3<f32>,
}

struct VertexOutput {
    @builtin(position) clip_pos:    vec4<f32>,
    @location(0)       world_pos:   vec3<f32>,
    @location(1)       world_normal: vec3<f32>,
}

@vertex
fn vs_main(in: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    let world_pos4 = object.model * vec4<f32>(in.position, 1.0);
    out.clip_pos   = scene.view_proj * world_pos4;
    out.world_pos  = world_pos4.xyz;

    // Normal matrix (works correctly for uniform scale)
    let nm = mat3x3<f32>(
        object.model[0].xyz,
        object.model[1].xyz,
        object.model[2].xyz,
    );
    out.world_normal = normalize(nm * in.normal);
    return out;
}

// ---- PBR helpers ----

const PI: f32 = 3.14159265358979;

fn distribution_ggx(N: vec3<f32>, H: vec3<f32>, roughness: f32) -> f32 {
    let a  = roughness * roughness;
    let a2 = a * a;
    let d  = max(dot(N, H), 0.0);
    let d2 = d * d;
    let t  = d2 * (a2 - 1.0) + 1.0;
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
    return geometry_schlick_ggx(NdotV, roughness) * geometry_schlick_ggx(NdotL, roughness);
}

fn fresnel_schlick(cos_theta: f32, F0: vec3<f32>) -> vec3<f32> {
    return F0 + (1.0 - F0) * pow(clamp(1.0 - cos_theta, 0.0, 1.0), 5.0);
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let N = normalize(in.world_normal);
    let V = normalize(scene.camera_pos - in.world_pos);
    let L = normalize(-scene.light_dir);
    let H = normalize(V + L);

    let albedo    = object.albedo.rgb;
    let metallic  = object.metallic;
    let roughness = clamp(object.roughness, 0.05, 1.0);

    // Fresnel at normal incidence
    var F0 = vec3<f32>(0.04);
    F0 = mix(F0, albedo, metallic);

    // Cook-Torrance BRDF
    let NDF = distribution_ggx(N, H, roughness);
    let G   = geometry_smith(N, V, L, roughness);
    let F   = fresnel_schlick(max(dot(H, V), 0.0), F0);

    let kD      = (vec3<f32>(1.0) - F) * (1.0 - metallic);
    let NdotL   = max(dot(N, L), 0.0);
    let denom   = 4.0 * max(dot(N, V), 0.0) * NdotL + 0.0001;
    let specular = (NDF * G * F) / denom;

    let Lo = (kD * albedo / PI + specular) * scene.light_color * NdotL;

    let ambient = scene.ambient_color * albedo;
    var color   = ambient + Lo;

    // Reinhard tone mapping + gamma correction
    color = color / (color + vec3<f32>(1.0));
    color = pow(color, vec3<f32>(1.0 / 2.2));

    return vec4<f32>(color, object.albedo.a);
}
