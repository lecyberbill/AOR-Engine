// [WFGY] Zone: SAFE | λ: 0.2 | Fallbacks: 0/None | Action: Depth-Aware Bilateral Blur for SSAO Smoothing
// Lisse le bruit de l'occlusion ambiante tout en préservant les arêtes vives de la géométrie

struct BlurUniform {
    texel_size: vec2<f32>,
    _pad0: f32,
    _pad1: f32,
};

@group(0) @binding(0)
var<uniform> blur_params: BlurUniform;

@group(0) @binding(1)
var t_ssao_raw: texture_2d<f32>;

@group(0) @binding(2)
var s_ssao: sampler;

@group(0) @binding(3)
var t_depth: texture_depth_2d;

@group(0) @binding(4)
var s_depth: sampler;

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@vertex
fn vs_fullscreen(@builtin(vertex_index) in_vertex_index: u32) -> VertexOutput {
    var out: VertexOutput;
    let x = f32(i32(in_vertex_index & 1u) * 4 - 1);
    let y = f32(i32(in_vertex_index & 2u) * 2 - 1);
    out.position = vec4<f32>(x, y, 0.0, 1.0);
    out.uv = vec2<f32>((x + 1.0) * 0.5, (1.0 - y) * 0.5);
    return out;
}

@fragment
fn fs_blur(in: VertexOutput) -> @location(0) f32 {
    let center_depth = textureSample(t_depth, s_depth, in.uv);
    var result = 0.0;
    var total_weight = 0.0;

    // Kernel bilatéral 4x4 centré
    for (var x = -2; x <= 2; x = x + 1) {
        for (var y = -2; y <= 2; y = y + 1) {
            let offset = vec2<f32>(f32(x), f32(y)) * blur_params.texel_size;
            let sample_uv = in.uv + offset;
            
            let sample_depth = textureSample(t_depth, s_depth, sample_uv);
            let sample_ssao = textureSample(t_ssao_raw, s_ssao, sample_uv).r;

            // Poids spatial gaussien
            let spatial_dist_sq = f32(x * x + y * y);
            let spatial_w = exp(-spatial_dist_sq / 8.0);

            // Poids de discontinuité de profondeur bilatérale
            let depth_diff = abs(center_depth - sample_depth);
            let depth_w = exp(-depth_diff * 400.0);

            let weight = spatial_w * depth_w;
            result = result + sample_ssao * weight;
            total_weight = total_weight + weight;
        }
    }

    if (total_weight > 0.0001) {
        return result / total_weight;
    }
    return textureSample(t_ssao_raw, s_ssao, in.uv).r;
}
