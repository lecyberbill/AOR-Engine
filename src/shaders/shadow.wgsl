// [WFGY] Zone: SAFE | λ: 0.2 | Fallbacks: 0/None | Action: Ultra-Fast Depth-Only Shadow Mapping Shader

struct ShadowUniform {
    light_space_matrix: mat4x4<f32>,
};

@group(0) @binding(0)
var<uniform> shadow_uniform: ShadowUniform;

struct ModelUniform {
    model: mat4x4<f32>,
    color: vec4<f32>,
    selected: u32,
    _pad0: u32,
    _pad1: u32,
    _pad2: u32,
};

@group(1) @binding(0)
var<uniform> model: ModelUniform;

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) tangent: vec4<f32>,
    @location(3) uv: vec2<f32>,
    @location(4) color: vec4<f32>,
};

@vertex
fn vs_main(in: VertexInput) -> @builtin(position) vec4<f32> {
    let world_pos = model.model * vec4<f32>(in.position, 1.0);
    return shadow_uniform.light_space_matrix * world_pos;
}

struct InstanceInput {
    @location(5) model_matrix_0: vec4<f32>,
    @location(6) model_matrix_1: vec4<f32>,
    @location(7) model_matrix_2: vec4<f32>,
    @location(8) model_matrix_3: vec4<f32>,
    @location(9) color_tint: vec4<f32>,
};

@vertex
fn vs_instanced(in: VertexInput, inst: InstanceInput) -> @builtin(position) vec4<f32> {
    let inst_model = mat4x4<f32>(
        inst.model_matrix_0,
        inst.model_matrix_1,
        inst.model_matrix_2,
        inst.model_matrix_3,
    );
    let world_pos = inst_model * vec4<f32>(in.position, 1.0);
    return shadow_uniform.light_space_matrix * world_pos;
}
