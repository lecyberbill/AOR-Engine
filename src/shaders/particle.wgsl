// [WFGY] Zone: SAFE | λ: 0.25 | Fallbacks: 0/None | Action: Instanced Quad Billboard Vertex and Fragment Shader for Particles

struct CameraUniform {
    view_proj: mat4x4<f32>,
    view: mat4x4<f32>,
    proj: mat4x4<f32>,
    camera_pos: vec4<f32>,
};

@group(0) @binding(0)
var<uniform> camera: CameraUniform;

struct InstanceInput {
    @location(0) position_scale: vec4<f32>, // xyz = center, w = scale
    @location(1) color: vec4<f32>,          // rgba
    @location(2) uv_data: vec4<f32>,        // x = rotation, y = kind (0=fire, 1=spark, 2=smoke, 3=debris)
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) color: vec4<f32>,
    @location(2) kind: f32,
};

@vertex
fn vs_main(
    @builtin(vertex_index) vertex_index: u32,
    instance: InstanceInput,
) -> VertexOutput {
    var out: VertexOutput;

    // Coordonnées locales du Quad [-0.5..0.5]
    // Indices: 0, 1, 2, 2, 3, 0 (2 triangles)
    var quad_pos = array<vec2<f32>, 6>(
        vec2<f32>(-0.5, -0.5),
        vec2<f32>( 0.5, -0.5),
        vec2<f32>( 0.5,  0.5),
        vec2<f32>( 0.5,  0.5),
        vec2<f32>(-0.5,  0.5),
        vec2<f32>(-0.5, -0.5),
    );

    let base_pos = quad_pos[vertex_index];
    out.uv = base_pos + vec2<f32>(0.5, 0.5); // [0.0..1.0]

    // Rotation du quad billboard
    let rot = instance.uv_data.x;
    let cos_r = cos(rot);
    let sin_r = sin(rot);
    let rotated_pos = vec2<f32>(
        base_pos.x * cos_r - base_pos.y * sin_r,
        base_pos.x * sin_r + base_pos.y * cos_r,
    );

    // Billboarding pur face à la caméra:
    // Extraire les vecteurs Right et Up depuis la matrice View inverse
    let cam_right = vec3<f32>(camera.view[0][0], camera.view[1][0], camera.view[2][0]);
    let cam_up = vec3<f32>(camera.view[0][1], camera.view[1][1], camera.view[2][1]);

    let scale = instance.position_scale.w;
    let world_pos = instance.position_scale.xyz 
        + cam_right * (rotated_pos.x * scale) 
        + cam_up * (rotated_pos.y * scale);

    out.clip_position = camera.view_proj * vec4<f32>(world_pos, 1.0);
    out.color = instance.color;
    out.kind = instance.uv_data.y;

    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    // Calcul de la distance au centre pour un masque circulaire lissé
    let centered_uv = in.uv * 2.0 - 1.0;
    let dist_sq = dot(centered_uv, centered_uv);

    if (dist_sq > 1.0) {
        discard;
    }

    // Glow doux et atténuation radiale
    var alpha_mask = 1.0 - smoothstep(0.0, 1.0, dist_sq);

    // Dégradé au centre plus intense
    let center_glow = exp(-dist_sq * 3.5);

    var final_color = in.color.rgb * (1.0 + center_glow * 1.5);
    var final_alpha = in.color.a * alpha_mask;

    // Type 1: Spark (étincelle vive plus concentrée)
    if (in.kind > 0.5 && in.kind < 1.5) {
        let spark_glow = exp(-dist_sq * 6.0);
        final_color = in.color.rgb * (1.2 + spark_glow * 3.0);
        final_alpha = in.color.a * exp(-dist_sq * 2.5);
    }
    // Type 2: Smoke (fumée plus douce et diffuse)
    else if (in.kind > 1.5 && in.kind < 2.5) {
        final_alpha = in.color.a * pow(alpha_mask, 1.5) * 0.7;
    }

    return vec4<f32>(final_color, final_alpha);
}
