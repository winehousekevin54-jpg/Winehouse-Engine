// ============================================================
// Winehouse Engine — Shadow Depth Pass (Phase 4)
// Depth-only render from the directional light's perspective.
// ============================================================

struct ShadowUniforms {
    light_view_proj: mat4x4<f32>,
}

struct ObjectUniforms {
    model:     mat4x4<f32>,
    albedo:    vec4<f32>,
    metallic:  f32,
    roughness: f32,
    _pad:      vec2<f32>,
}

@group(0) @binding(0) var<uniform> shadow_uni: ShadowUniforms;
@group(1) @binding(0) var<uniform> object:     ObjectUniforms;

@vertex
fn vs_main(
    @location(0) position: vec3<f32>,
    @location(1) _normal:  vec3<f32>,
) -> @builtin(position) vec4<f32> {
    return shadow_uni.light_view_proj * object.model * vec4<f32>(position, 1.0);
}
