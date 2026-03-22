// ============================================================
// Winehouse Engine — FXAA Pass (Phase 4)
// Fast Approximate Anti-Aliasing (Nvidia FXAA 3.11 simplified).
// All textureSample calls before any branch — uniform control flow.
// ============================================================

@group(0) @binding(0) var ldr_tex:     texture_2d<f32>;
@group(0) @binding(1) var linear_samp: sampler;

fn luma(rgb: vec3<f32>) -> f32 {
    return dot(rgb, vec3<f32>(0.299, 0.587, 0.114));
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
fn fs_main(@builtin(position) fc: vec4<f32>) -> @location(0) vec4<f32> {
    let dims = vec2<f32>(textureDimensions(ldr_tex));
    let uv   = fc.xy / dims;
    let t    = 1.0 / dims;

    // ── 9-tap neighborhood — ALL samples before any branch (uniform flow) ──────
    let c  = textureSample(ldr_tex, linear_samp, uv                        ).rgb;
    let n  = textureSample(ldr_tex, linear_samp, uv + vec2<f32>( 0.0,  t.y)).rgb;
    let s  = textureSample(ldr_tex, linear_samp, uv + vec2<f32>( 0.0, -t.y)).rgb;
    let e  = textureSample(ldr_tex, linear_samp, uv + vec2<f32>( t.x,  0.0)).rgb;
    let w  = textureSample(ldr_tex, linear_samp, uv + vec2<f32>(-t.x,  0.0)).rgb;
    let ne = textureSample(ldr_tex, linear_samp, uv + vec2<f32>( t.x,  t.y)).rgb;
    let nw = textureSample(ldr_tex, linear_samp, uv + vec2<f32>(-t.x,  t.y)).rgb;
    let se = textureSample(ldr_tex, linear_samp, uv + vec2<f32>( t.x, -t.y)).rgb;
    let sw = textureSample(ldr_tex, linear_samp, uv + vec2<f32>(-t.x, -t.y)).rgb;

    // ── Luminance ──────────────────────────────────────────────────────────────
    let lc  = luma(c);  let ln  = luma(n);  let ls  = luma(s);
    let le  = luma(e);  let lw  = luma(w);
    let lne = luma(ne); let lnw = luma(nw); let lse = luma(se); let lsw = luma(sw);

    let lmin = min(lc, min(min(ln, ls), min(le, lw)));
    let lmax = max(lc, max(max(ln, ls), max(le, lw)));
    let contrast = lmax - lmin;

    // ── Sub-pixel blend (smooths single-pixel features) ────────────────────────
    let avg_neigh = (ln + ls + le + lw) * 2.0 + (lne + lnw + lse + lsw);
    let avg       = avg_neigh * (1.0 / 12.0);
    let sub_blend = clamp(abs(avg - lc) / max(contrast, 0.0001), 0.0, 1.0);
    let sub_blend2 = sub_blend * sub_blend * 0.75;

    // ── Edge direction via Sobel ───────────────────────────────────────────────
    let eh = abs(lnw - lne) + abs(lw - le) * 2.0 + abs(lsw - lse);
    let ev = abs(lnw - lsw) + abs(ln - ls) * 2.0 + abs(lne - lse);
    let horiz = eh >= ev;

    // Blend offset perpendicular to the detected edge
    let offset = select(vec2<f32>(t.x, 0.0), vec2<f32>(0.0, t.y), horiz) * sub_blend2;

    // One additional sample along blend direction — still uniform control flow
    let blended = textureSample(ldr_tex, linear_samp, uv + offset).rgb;

    // ── Apply FXAA only where contrast exceeds threshold ──────────────────────
    // select() avoids non-uniform branching
    let thresh  = max(0.0312, lmax * 0.063);
    let do_fxaa = contrast >= thresh;
    return vec4<f32>(select(c, blended, do_fxaa), 1.0);
}
