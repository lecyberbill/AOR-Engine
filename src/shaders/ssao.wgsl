// [WFGY] Zone: SAFE | λ: 0.25 | Fallbacks: 0/None | Action: Screen-Space Ambient Occlusion (SSAO) Compute & Screen Shader
// Calcule l'occlusion ambiante par échantillonnage hémisphérique à partir de la profondeur

struct SsaoUniform {
    proj: mat4x4<f32>,
    inv_proj: mat4x4<f32>,
    kernel: array<vec4<f32>, 32>,
    radius: f32,
    bias: f32,
    screen_width: f32,
    screen_height: f32,
};

@group(0) @binding(0)
var<uniform> params: SsaoUniform;

@group(0) @binding(1)
var t_depth: texture_depth_2d;

@group(0) @binding(2)
var s_depth: sampler;

@group(0) @binding(3)
var t_noise: texture_2d<f32>;

@group(0) @binding(4)
var s_noise: sampler;

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

// Reconstruit la position dans l'espace vue (View-Space) depuis le Depth Buffer
fn view_pos_from_depth(uv: vec2<f32>) -> vec3<f32> {
    let depth = textureSample(t_depth, s_depth, uv);
    let clip_pos = vec4<f32>(uv.x * 2.0 - 1.0, (1.0 - uv.y) * 2.0 - 1.0, depth, 1.0);
    let view_pos_h = params.inv_proj * clip_pos;
    return view_pos_h.xyz / max(view_pos_h.w, 0.00001);
}

@fragment
fn fs_ssao(in: VertexOutput) -> @location(0) f32 {
    let frag_pos = view_pos_from_depth(in.uv);
    
    // Si nous sommes sur le fond / ciel lointain, pas d'occlusion
    if (frag_pos.z <= -990.0 || frag_pos.z >= 0.0) {
        return 1.0;
    }

    // Calcul de la normale vue par dérivées partielles spatiales de la géométrie reconstruite
    let dX = dpdx(frag_pos);
    let dY = dpdy(frag_pos);
    var normal = normalize(cross(dX, dY));
    if (normal.z < 0.0) {
        normal = -normal;
    }

    // Récupérer le vecteur de rotation aléatoire depuis la texture de bruit 4x4
    let noise_scale = vec2<f32>(params.screen_width / 4.0, params.screen_height / 4.0);
    let random_vec = textureSample(t_noise, s_noise, in.uv * noise_scale).xyz * 2.0 - vec3<f32>(1.0);

    // Construction de la matrice TBN (Tangent, Bitangent, Normal)
    let tangent = normalize(random_vec - normal * dot(random_vec, normal));
    let bitangent = cross(normal, tangent);
    let tbn = mat3x3<f32>(tangent, bitangent, normal);

    var occlusion = 0.0;
    let kernel_size = 32;

    for (var i = 0; i < kernel_size; i = i + 1) {
        // Échantillon orienté dans l'hémisphère de la normale
        let sample_pos = frag_pos + (tbn * params.kernel[i].xyz) * params.radius;

        // Projeter la position d'échantillon sur l'écran
        var offset = params.proj * vec4<f32>(sample_pos, 1.0);
        offset = vec4<f32>(offset.xyz / offset.w, offset.w);
        let sample_uv = vec2<f32>(offset.x * 0.5 + 0.5, -offset.y * 0.5 + 0.5);

        // Tester si l'UV projeté est dans l'écran
        if (sample_uv.x >= 0.0 && sample_uv.x <= 1.0 && sample_uv.y >= 0.0 && sample_uv.y <= 1.0) {
            let sample_depth_pos = view_pos_from_depth(sample_uv);
            
            // Évaluation de l'occlusion avec atténuation par distance pour éviter les artefacts d'objets lointains
            let range_check = smoothstep(0.0, 1.0, params.radius / max(abs(frag_pos.z - sample_depth_pos.z), 0.001));
            if (sample_depth_pos.z >= sample_pos.z + params.bias) {
                occlusion = occlusion + (1.0 * range_check);
            }
        }
    }

    let ao = 1.0 - (occlusion / f32(kernel_size));
    return clamp(ao, 0.0, 1.0);
}
