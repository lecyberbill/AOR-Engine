// [WFGY] Zone: SAFE | λ: 0.3 | Fallbacks: 0/None | Action: WGSL 3D infinite ground grid shader

struct CameraUniform {
    view_proj: mat4x4<f32>,
    view: mat4x4<f32>,
    proj: mat4x4<f32>,
    camera_pos: vec4<f32>,
};

@group(0) @binding(0)
var<uniform> camera: CameraUniform;

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) color: vec4<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) near_point: vec3<f32>,
    @location(1) far_point: vec3<f32>,
};

// Quadrilatère couvrant le plan NDC (-1..1)
var<private> POS: array<vec3<f32>, 6> = array<vec3<f32>, 6>(
    vec3<f32>(-1.0, -1.0, 0.0),
    vec3<f32>( 1.0, -1.0, 0.0),
    vec3<f32>( 1.0,  1.0, 0.0),
    vec3<f32>(-1.0, -1.0, 0.0),
    vec3<f32>( 1.0,  1.0, 0.0),
    vec3<f32>(-1.0,  1.0, 0.0)
);

fn unproject_point(x: f32, y: f32, z: f32, inv_view_proj: mat4x4<f32>) -> vec3<f32> {
    let unproj = inv_view_proj * vec4<f32>(x, y, z, 1.0);
    return unproj.xyz / unproj.w;
}

@vertex
fn vs_grid(@builtin(vertex_index) vertex_index: u32) -> VertexOutput {
    var out: VertexOutput;
    let p = POS[vertex_index];
    out.clip_position = vec4<f32>(p.xy, 0.0, 1.0);
    return out;
}
