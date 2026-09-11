// [WFGY] Zone: SAFE | λ: 0.3 | Fallbacks: 0/None | Action: Temporal Anti-Aliasing (TAA) Shader with 3x3 Neighborhood Color Clamping & Motion Vector Reprojection

struct TaaUniform {
    current_inv_view_proj: mat4x4<f32>,
    prev_view_proj: mat4x4<f32>,
    jitter_offset: vec2<f32>,
    screen_size: vec2<f32>,
    feedback_factor: f32,
    _pad0: f32,
    _pad1: f32,
    _pad2: f32,
};

@group(0) @binding(0)
var<uniform> taa: TaaUniform;

@group(0) @binding(1)
var t_current_color: texture_2d<f32>;

@group(0) @binding(2)
var t_history_color: texture_2d<f32>;

@group(0) @binding(3)
var t_depth: texture_depth_2d;

@group(0) @binding(4)
var s_linear: sampler;

@group(0) @binding(5)
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

// Convertit RGB vers YCoCg pour un clamping couleur plus stable et sans dérive de teinte
fn rgb_to_ycocg(c: vec3<f32>) -> vec3<f32> {
    let y = dot(c, vec3<f32>(0.25, 0.50, 0.25));
    let co = dot(c, vec3<f32>(0.50, 0.00, -0.50));
    let cg = dot(c, vec3<f32>(-0.25, 0.50, -0.25));
    return vec3<f32>(y, co, cg);
}

fn ycocg_to_rgb(c: vec3<f32>) -> vec3<f32> {
    let y = c.x;
    let co = c.y;
    let cg = c.z;
    let r = y + co - cg;
    let g = y + cg;
    let b = y - co - cg;
    return max(vec3<f32>(0.0), vec3<f32>(r, g, b));
}

@fragment
fn fs_taa(in: VertexOutput) -> @location(0) vec4<f32> {
    let texel_size = 1.0 / taa.screen_size;
    let depth = textureSample(t_depth, s_depth, in.uv);

    // 1. Reconstruction de la position monde actuelle depuis le depth buffer
    let current_clip = vec4<f32>(
        in.uv.x * 2.0 - 1.0,
        (1.0 - in.uv.y) * 2.0 - 1.0,
        depth,
        1.0,
    );
    let current_world_h = taa.current_inv_view_proj * current_clip;
    let current_world = current_world_h.xyz / current_world_h.w;

    // 2. Reprojection dans l'écran de la trame précédente pour calculer le vecteur de mouvement
    let prev_clip = taa.prev_view_proj * vec4<f32>(current_world, 1.0);
    let prev_ndc = prev_clip.xyz / prev_clip.w;
    let prev_uv = vec2<f32>(
        prev_ndc.x * 0.5 + 0.5,
        -prev_ndc.y * 0.5 + 0.5,
    );

    // Échantillonnage de la couleur courante (dé-jitterée)
    let current_sample_uv = in.uv - taa.jitter_offset;
    let current_color = textureSample(t_current_color, s_linear, clamp(current_sample_uv, vec2<f32>(0.001), vec2<f32>(0.999))).rgb;

    // 3. Calcul de la boîte englobante du voisinage 3x3 (Neighborhood Clamping en espace YCoCg)
    var min_color = rgb_to_ycocg(current_color);
    var max_color = min_color;

    for (var x = -1; x <= 1; x = x + 1) {
        for (var y = -1; y <= 1; y = y + 1) {
            let sample_uv = clamp(current_sample_uv + vec2<f32>(f32(x), f32(y)) * texel_size, vec2<f32>(0.001), vec2<f32>(0.999));
            let neighbor_rgb = textureSample(t_current_color, s_linear, sample_uv).rgb;
            let neighbor_ycocg = rgb_to_ycocg(neighbor_rgb);
            min_color = min(min_color, neighbor_ycocg);
            max_color = max(max_color, neighbor_ycocg);
        }
    }

    // 4. Échantillonnage de l'historique et détection de validité (hors écran = réinitialisation)
    var is_history_valid = true;
    if (prev_uv.x < 0.0 || prev_uv.x > 1.0 || prev_uv.y < 0.0 || prev_uv.y > 1.0) {
        is_history_valid = false;
    }

    if (!is_history_valid || taa.feedback_factor <= 0.01) {
        return vec4<f32>(current_color, 1.0);
    }

    let history_raw = textureSample(t_history_color, s_linear, clamp(prev_uv, vec2<f32>(0.001), vec2<f32>(0.999))).rgb;
    let history_ycocg = rgb_to_ycocg(history_raw);

    // Clamping de l'historique dans la boîte englobante pour éliminer le ghosting
    let clamped_history_ycocg = clamp(history_ycocg, min_color, max_color);
    let history_color = ycocg_to_rgb(clamped_history_ycocg);

    // 5. Mélange temporel avec pondération exponentielle (Exponential Moving Average)
    let final_color = mix(current_color, history_color, taa.feedback_factor);

    return vec4<f32>(final_color, 1.0);
}
