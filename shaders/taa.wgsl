// ============================================================
// Winehouse Engine — Temporal Anti-Aliasing (TAA) Resolve
// Per-object motion vectors, YCoCg neighbourhood clamp, Halton jitter.
// ============================================================

struct TaaUniforms {
    viewport:     vec2<f32>,
    jitter:       vec2<f32>,
    blend_factor: f32,
    _pad0:        f32,
    _pad1:        f32,
    _pad2:        f32,
};

@group(0) @binding(0) var<uniform> taa:          TaaUniforms;
@group(0) @binding(1) var          hdr_tex:      texture_2d<f32>;
@group(0) @binding(2) var          history_tex:  texture_2d<f32>;
@group(0) @binding(3) var          velocity_tex: texture_2d<f32>;
@group(0) @binding(4) var          depth_tex:    texture_depth_2d;
@group(0) @binding(5) var          linear_samp:  sampler;

// ── Colour-space helpers ──────────────────────────────────────

fn rgb_to_ycocg(c: vec3<f32>) -> vec3<f32> {
    let co = c.r - c.b;
    let t  = c.b + co * 0.5;
    let cg = c.g - t;
    let y  = t + cg * 0.5;
    return vec3(y, co, cg);
}

fn ycocg_to_rgb(c: vec3<f32>) -> vec3<f32> {
    let t = c.x - c.z * 0.5;
    let g = c.z + t;
    let b = t - c.y * 0.5;
    let r = b + c.y;
    return vec3(r, g, b);
}

// ── Fullscreen triangle vertex shader ─────────────────────────

struct VsOut {
    @builtin(position) pos: vec4<f32>,
    @location(0)       uv:  vec2<f32>,
};

@vertex
fn vs_main(@builtin(vertex_index) idx: u32) -> VsOut {
    var positions = array<vec2<f32>, 3>(
        vec2<f32>(-1.0, -1.0),
        vec2<f32>( 3.0, -1.0),
        vec2<f32>(-1.0,  3.0),
    );
    let p = positions[idx];
    var out: VsOut;
    out.pos = vec4<f32>(p, 0.0, 1.0);
    out.uv  = p * 0.5 + 0.5;
    // Flip Y: NDC bottom-left → UV top-left
    out.uv.y = 1.0 - out.uv.y;
    return out;
}

// ── TAA resolve fragment shader ───────────────────────────────

@fragment
fn fs_main(in: VsOut) -> @location(0) vec4<f32> {
    let dims = vec2<i32>(textureDimensions(hdr_tex));
    let coord = vec2<i32>(in.pos.xy);

    // Current colour (unjittered sample point)
    let current_rgb = textureLoad(hdr_tex, coord, 0).rgb;

    // Motion vector — stored as UV-space offset (signed)
    let velocity = textureLoad(velocity_tex, coord, 0).xy;

    // Reproject: find where this pixel was last frame
    let history_uv = in.uv - velocity;

    // Reject samples outside the viewport
    if history_uv.x < 0.0 || history_uv.x > 1.0 || history_uv.y < 0.0 || history_uv.y > 1.0 {
        return vec4<f32>(current_rgb, 1.0);
    }

    // Sample history with bilinear interpolation
    let history_rgb = textureSampleLevel(history_tex, linear_samp, history_uv, 0.0).rgb;

    // ── 3×3 neighbourhood clamp in YCoCg ──────────────────────
    let c  = rgb_to_ycocg(current_rgb);
    var mn = c;
    var mx = c;

    for (var dy: i32 = -1; dy <= 1; dy++) {
        for (var dx: i32 = -1; dx <= 1; dx++) {
            let sc = clamp(coord + vec2<i32>(dx, dy), vec2<i32>(0), dims - 1);
            let s  = rgb_to_ycocg(textureLoad(hdr_tex, sc, 0).rgb);
            mn = min(mn, s);
            mx = max(mx, s);
        }
    }

    let history_ycocg = clamp(rgb_to_ycocg(history_rgb), mn, mx);
    let clamped_rgb   = ycocg_to_rgb(history_ycocg);

    // Blend: higher weight to history for stability
    let result = mix(clamped_rgb, current_rgb, taa.blend_factor);
    return vec4<f32>(max(result, vec3(0.0)), 1.0);
}
