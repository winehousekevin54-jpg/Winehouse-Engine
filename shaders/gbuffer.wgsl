// ============================================================
// Winehouse Engine — G-Buffer Pass (PBR Textures)
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
    model:        mat4x4<f32>,
    prev_model:   mat4x4<f32>,
    albedo:       vec4<f32>,
    metallic:     f32,
    roughness:    f32,
    /// 0 = opaque; >0 = AlphaMask cutoff — fragments below are discarded
    alpha_cutoff: f32,
    _pad:         f32,
}

@group(0) @binding(0) var<uniform> scene:  SceneUniforms;
@group(1) @binding(0) var<uniform> object: ObjectUniforms;
@group(1) @binding(1) var t_albedo:    texture_2d<f32>;
@group(1) @binding(2) var t_normal:    texture_2d<f32>;
@group(1) @binding(3) var t_mr:        texture_2d<f32>;
@group(1) @binding(4) var s_material:  sampler;

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) normal:   vec3<f32>,
    @location(2) uv:       vec2<f32>,
    @location(3) tangent:  vec4<f32>,
}

struct VertexOutput {
    @builtin(position) clip_pos:      vec4<f32>,
    @location(0)       world_normal:  vec3<f32>,
    @location(1)       curr_ndc_pos:  vec3<f32>,
    @location(2)       prev_ndc_pos:  vec3<f32>,
    @location(3)       uv:           vec2<f32>,
    @location(4)       world_tangent: vec3<f32>,
    @location(5)       tangent_w:     f32,
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
    out.world_normal  = normalize(nm * in.normal);
    out.world_tangent = normalize(nm * in.tangent.xyz);
    out.tangent_w     = in.tangent.w;
    out.uv            = in.uv;
    return out;
}

struct GBufferOut {
    @location(0) albedo_roughness: vec4<f32>, // RGB=albedo, A=roughness
    @location(1) normal_metallic:  vec4<f32>, // RGB=encoded normal, A=metallic
    @location(2) velocity:         vec4<f32>, // RG=screen-space motion in UV space
}

@fragment
fn fs_main(in: VertexOutput) -> GBufferOut {
    var out: GBufferOut;

    // ── Sample PBR textures ────────────────────────────────────────────────
    // All textureSample calls must occur before any non-uniform branch (WGSL rule)
    let albedo_sample = textureSample(t_albedo, s_material, in.uv);
    let mr_sample     = textureSample(t_mr, s_material, in.uv);
    let normal_sample = textureSample(t_normal, s_material, in.uv);

    // Alpha mask: discard fragments whose alpha is below the cutoff.
    // Required for foliage, feathers, hair, wing membranes etc.
    // (object.alpha_cutoff == 0.0 means opaque → no discard overhead)
    if (object.alpha_cutoff > 0.0 && albedo_sample.a < object.alpha_cutoff) {
        discard;
    }

    // Albedo: texture × uniform tint (base_color_factor applied via object.albedo)
    let albedo = albedo_sample.rgb * object.albedo.rgb;

    // Metallic-roughness: green=roughness, blue=metallic (glTF convention)
    let roughness = clamp(mr_sample.g * object.roughness, 0.05, 1.0);
    let metallic  = clamp(mr_sample.b * object.metallic, 0.0, 1.0);

    // ── Normal mapping via TBN matrix ──────────────────────────────────────
    let N = normalize(in.world_normal);
    let T = normalize(in.world_tangent);
    let B = cross(N, T) * in.tangent_w;
    let TBN = mat3x3<f32>(T, B, N);
    // Tangent-space normal from texture: remap [0,1] → [-1,1]
    let ts_normal = normal_sample.rgb * 2.0 - 1.0;
    let world_normal = normalize(TBN * ts_normal);

    // ── Write G-Buffer ─────────────────────────────────────────────────────
    out.albedo_roughness = vec4<f32>(albedo, roughness);
    out.normal_metallic  = vec4<f32>(world_normal * 0.5 + 0.5, metallic);

    // Velocity
    let velocity = (in.curr_ndc_pos.xy - in.prev_ndc_pos.xy) * 0.5;
    out.velocity = vec4<f32>(velocity, 0.0, 0.0);
    return out;
}
