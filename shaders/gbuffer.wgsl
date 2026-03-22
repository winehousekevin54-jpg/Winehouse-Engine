// ============================================================
// Winehouse Engine — G-Buffer Pass
// Outputs: albedo+roughness (Rgba8Unorm), normal+metallic (Rgba16Float),
//          velocity (Rgba16Float — screen-space motion vectors)
// ============================================================

struct SceneUniforms {
    view_proj:            mat4x4<f32>,   // jittered — used for rasterization
    unjittered_view_proj: mat4x4<f32>,   // current frame, no jitter — for velocity
    prev_view_proj:       mat4x4<f32>,   // previous frame, no jitter — for velocity
    camera_pos:           vec3<f32>,
    _pad0:                f32,
    light_dir:            vec3<f32>,
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

@group(0) @binding(0) var<uniform> scene:  SceneUniforms;
@group(1) @binding(0) var<uniform> object: ObjectUniforms;

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) normal:   vec3<f32>,
}

struct VertexOutput {
    @builtin(position) clip_pos:      vec4<f32>,
    @location(0)       world_normal:  vec3<f32>,
    @location(1)       curr_ndc_pos:  vec3<f32>,
    @location(2)       prev_ndc_pos:  vec3<f32>,
}

@vertex
fn vs_main(in: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    let world_pos4 = object.model * vec4<f32>(in.position, 1.0);

    // Jittered clip position for rasterization / depth
    out.clip_pos = scene.view_proj * world_pos4;

    // Unjittered clip positions for clean velocity computation
    let curr_clip = scene.unjittered_view_proj * world_pos4;
    let prev_world_pos4 = object.prev_model * vec4<f32>(in.position, 1.0);
    let prev_clip = scene.prev_view_proj * prev_world_pos4;

    // Perspective divide to NDC — pass to fragment for interpolation
    out.curr_ndc_pos = curr_clip.xyz / curr_clip.w;
    out.prev_ndc_pos = prev_clip.xyz / prev_clip.w;

    // Normal matrix (correct for uniform scale)
    let nm = mat3x3<f32>(
        object.model[0].xyz,
        object.model[1].xyz,
        object.model[2].xyz,
    );
    out.world_normal = normalize(nm * in.normal);
    return out;
}

struct GBufferOut {
    @location(0) albedo_roughness: vec4<f32>, // RGB=albedo, A=roughness
    @location(1) normal_metallic:  vec4<f32>, // RGB=octahedron-encoded normal, A=metallic
    @location(2) velocity:         vec4<f32>, // RG=screen-space motion in UV space
}

@fragment
fn fs_main(in: VertexOutput) -> GBufferOut {
    var out: GBufferOut;
    let roughness = clamp(object.roughness, 0.05, 1.0);
    // Store linear albedo and roughness
    out.albedo_roughness = vec4<f32>(object.albedo.rgb, roughness);
    // Encode normal to [0,1] for Rgba8Unorm-compatible storage
    out.normal_metallic  = vec4<f32>(normalize(in.world_normal) * 0.5 + 0.5, object.metallic);
    // Velocity: difference in NDC mapped to UV space (NDC * 0.5)
    // curr_ndc_pos and prev_ndc_pos are already perspective-divided
    let velocity = (in.curr_ndc_pos.xy - in.prev_ndc_pos.xy) * 0.5;
    out.velocity = vec4<f32>(velocity, 0.0, 0.0);
    return out;
}
