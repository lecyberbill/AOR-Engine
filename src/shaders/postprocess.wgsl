// [WFGY] Zone: SAFE | λ: 0.25 | Fallbacks: 0 | Action: HDR Post-Processing Shader (Bloom extraction, Dual Blur, ACES Tonemapping, Vignette)

struct PostProcessUniform {
    bloom_threshold: f32,
    bloom_intensity: f32,
    exposure: f32,
    vignette_strength: f32,
};

@group(0) @binding(0)
var<uniform> params: PostProcessUniform;

@group(0) @binding(1)
var t_scene_hdr: texture_2d<f32>;

@group(0) @binding(2)
var s_sampler: sampler;

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

// Quad plein écran généré procéduralement
@vertex
fn vs_fullscreen(@builtin(vertex_index) in_vertex_index: u32) -> VertexOutput {
    var out: VertexOutput;
    let x = f32(i32(in_vertex_index & 1u) * 4 - 1);
    let y = f32(i32(in_vertex_index & 2u) * 2 - 1);
    out.position = vec4<f32>(x, y, 0.0, 1.0);
    out.uv = vec2<f32>((x + 1.0) * 0.5, (1.0 - y) * 0.5);
    return out;
}

// Opérateur ACES Film Tonemapping
fn aces_tonemap(x: vec3<f32>) -> vec3<f32> {
    let a = 2.51;
    let b = 0.03;
    let c = 2.43;
    let d = 0.59;
    let e = 0.14;
    return clamp((x * (a * x + b)) / (x * (c * x + d) + e), vec3<f32>(0.0), vec3<f32>(1.0));
}

@fragment
fn fs_postprocess(in: VertexOutput) -> @location(0) vec4<f32> {
    let base_color = textureSample(t_scene_hdr, s_sampler, in.uv).rgb;

    // Échantillonnage multi-tap pour le Bloom (Dual-Kawase 9 taps)
    let tex_dim = vec2<f32>(textureDimensions(t_scene_hdr));
    let texel = vec2<f32>(1.0) / tex_dim;
    let blur_radius = 2.5;

    // Extraction des zones brillantes et flou Dual-Kawase déroulé
    var bloom_acc = vec3<f32>(0.0);
    let bright_center = max(vec3<f32>(0.0), base_color - vec3<f32>(params.bloom_threshold));
    bloom_acc += bright_center * 0.20;

    let w = 0.80 / 8.0;
    let s0 = textureSample(t_scene_hdr, s_sampler, in.uv + vec2<f32>(-1.0, -1.0) * texel * blur_radius).rgb;
    let s1 = textureSample(t_scene_hdr, s_sampler, in.uv + vec2<f32>( 1.0, -1.0) * texel * blur_radius).rgb;
    let s2 = textureSample(t_scene_hdr, s_sampler, in.uv + vec2<f32>(-1.0,  1.0) * texel * blur_radius).rgb;
    let s3 = textureSample(t_scene_hdr, s_sampler, in.uv + vec2<f32>( 1.0,  1.0) * texel * blur_radius).rgb;
    let s4 = textureSample(t_scene_hdr, s_sampler, in.uv + vec2<f32>(-2.0,  0.0) * texel * blur_radius).rgb;
    let s5 = textureSample(t_scene_hdr, s_sampler, in.uv + vec2<f32>( 2.0,  0.0) * texel * blur_radius).rgb;
    let s6 = textureSample(t_scene_hdr, s_sampler, in.uv + vec2<f32>( 0.0, -2.0) * texel * blur_radius).rgb;
    let s7 = textureSample(t_scene_hdr, s_sampler, in.uv + vec2<f32>( 0.0,  2.0) * texel * blur_radius).rgb;

    bloom_acc += max(vec3<f32>(0.0), s0 - vec3<f32>(params.bloom_threshold)) * w;
    bloom_acc += max(vec3<f32>(0.0), s1 - vec3<f32>(params.bloom_threshold)) * w;
    bloom_acc += max(vec3<f32>(0.0), s2 - vec3<f32>(params.bloom_threshold)) * w;
    bloom_acc += max(vec3<f32>(0.0), s3 - vec3<f32>(params.bloom_threshold)) * w;
    bloom_acc += max(vec3<f32>(0.0), s4 - vec3<f32>(params.bloom_threshold)) * w;
    bloom_acc += max(vec3<f32>(0.0), s5 - vec3<f32>(params.bloom_threshold)) * w;
    bloom_acc += max(vec3<f32>(0.0), s6 - vec3<f32>(params.bloom_threshold)) * w;
    bloom_acc += max(vec3<f32>(0.0), s7 - vec3<f32>(params.bloom_threshold)) * w;

    // Combinaison HDR : Scène + Lueur Bloom
    let hdr_combined = (base_color + bloom_acc * params.bloom_intensity) * params.exposure;

    // Tonemapping ACES
    var mapped = aces_tonemap(hdr_combined);

    // Vignettage cinématographique
    let uv_dist = distance(in.uv, vec2<f32>(0.5, 0.5));
    let vignette = smoothstep(0.8, 0.2, uv_dist * params.vignette_strength);
    mapped *= vignette;

    return vec4<f32>(mapped, 1.0);
}
