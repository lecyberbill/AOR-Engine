// [WFGY] Zone: SAFE | λ: 0.2 | Fallbacks: 0/None | Action: Procedural Skybox Fullscreen Shader with Inverse Camera Matrix
// Affiche l'arrière-plan de la Skybox Cubemap à l'infini avec un test de profondeur LessEqual

struct CameraUniform {
    view_proj: mat4x4<f32>,
    view: mat4x4<f32>,
    proj: mat4x4<f32>,
    camera_pos: vec4<f32>,
};

@group(0) @binding(0)
var<uniform> camera: CameraUniform;

@group(0) @binding(1)
var t_skybox: texture_cube<f32>;

@group(0) @binding(2)
var s_skybox: sampler;

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) view_dir: vec3<f32>,
};

@vertex
fn vs_skybox(@builtin(vertex_index) in_vertex_index: u32) -> VertexOutput {
    var out: VertexOutput;
    let x = f32(i32(in_vertex_index & 1u) * 4 - 1);
    let y = f32(i32(in_vertex_index & 2u) * 2 - 1);
    
    // Position du quad projeté au plan lointain (z = 1.0 en WebGPU)
    out.position = vec4<f32>(x, y, 1.0, 1.0);

    // Calcul de la direction du rayon de vue dans le monde via la matrice de vue inverse (sans translation)
    let clip_pos = vec4<f32>(x, y, 1.0, 1.0);
    let view_inv = transpose(mat3x3<f32>(
        camera.view[0].xyz,
        camera.view[1].xyz,
        camera.view[2].xyz,
    ));

    // Projection inverse
    let inv_proj_pos = vec3<f32>(
        x / camera.proj[0][0],
        y / camera.proj[1][1],
        -1.0
    );

    out.view_dir = normalize(view_inv * inv_proj_pos);
    return out;
}

@fragment
fn fs_skybox(in: VertexOutput) -> @location(0) vec4<f32> {
    let env_color = textureSample(t_skybox, s_skybox, in.view_dir).rgb;
    return vec4<f32>(env_color, 1.0);
}
