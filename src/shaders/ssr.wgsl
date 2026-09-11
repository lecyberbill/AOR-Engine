// [WFGY] Zone: SAFE | λ: 0.3 | Fallbacks: 0/None | Action: Screen-Space Reflections (SSR) Raymarching Shader
struct CameraUniform {
    view_proj: mat4x4<f32>,
    view: mat4x4<f32>,
    proj: mat4x4<f32>,
    camera_pos: vec4<f32>,
};

struct SsrUniform {
    max_ray_distance: f32,
    max_steps: u32,
    thickness: f32,
    roughness_cutoff: f32,
    edge_fade_start: f32,
    edge_fade_end: f32,
    intensity: f32,
    _pad: u32,
};

@group(0) @binding(0)
var<uniform> camera: CameraUniform;

@group(0) @binding(1)
var<uniform> ssr: SsrUniform;

@group(0) @binding(2)
var t_color: texture_2d<f32>;

@group(0) @binding(3)
var t_depth: texture_depth_2d;

@group(0) @binding(4)
var s_linear: sampler;

@group(0) @binding(5)
var s_point: sampler;

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@vertex
fn vs_main(@builtin(vertex_index) in_vertex_index: u32) -> VertexOutput {
    var out: VertexOutput;
    let x = f32((in_vertex_index << 1u) & 2u);
    let y = f32(in_vertex_index & 2u);
    out.position = vec4<f32>(x * 2.0 - 1.0, 1.0 - y * 2.0, 0.0, 1.0);
    out.uv = vec2<f32>(x, y);
    return out;
}

// Reconstruct world position from depth
fn world_pos_from_depth(uv: vec2<f32>, depth: f32) -> vec3<f32> {
    let clip_space = vec4<f32>(uv.x * 2.0 - 1.0, (1.0 - uv.y) * 2.0 - 1.0, depth, 1.0);
    // Invert view_proj manually or via reconstructed ray
    // We compute view_proj inverse on CPU or reconstruct in view space
    let inv_proj_0 = vec4<f32>(1.0 / camera.proj[0][0], 0.0, 0.0, 0.0);
    let inv_proj_1 = vec4<f32>(0.0, 1.0 / camera.proj[1][1], 0.0, 0.0);
    let inv_proj_2 = vec4<f32>(0.0, 0.0, 0.0, 1.0 / camera.proj[3][2]);
    let inv_proj_3 = vec4<f32>(0.0, 0.0, -1.0, camera.proj[2][2] / camera.proj[3][2]);
    
    // View space position
    let ndc_x = uv.x * 2.0 - 1.0;
    let ndc_y = (1.0 - uv.y) * 2.0 - 1.0;
    let view_x = ndc_x / camera.proj[0][0];
    let view_y = ndc_y / camera.proj[1][1];
    let view_z = camera.proj[3][2] / (depth - camera.proj[2][2]);
    let view_pos = vec3<f32>(view_x * -view_z, view_y * -view_z, view_z);

    // Transform view space to world space: camera.view inverse rotation + camera_pos
    let cam_right = vec3<f32>(camera.view[0][0], camera.view[1][0], camera.view[2][0]);
    let cam_up    = vec3<f32>(camera.view[0][1], camera.view[1][1], camera.view[2][1]);
    let cam_fwd   = vec3<f32>(-camera.view[0][2], -camera.view[1][2], -camera.view[2][2]);

    return camera.camera_pos.xyz + cam_right * view_pos.x + cam_up * view_pos.y - cam_fwd * view_pos.z;
}

// Approximate normal from depth derivative
fn normal_from_depth(uv: vec2<f32>, depth: f32, tex_size: vec2<f32>) -> vec3<f32> {
    let texel = 1.0 / tex_size;
    let d_r = textureSample(t_depth, s_point, uv + vec2<f32>(texel.x, 0.0));
    let d_u = textureSample(t_depth, s_point, uv + vec2<f32>(0.0, -texel.y));

    let p_c = world_pos_from_depth(uv, depth);
    let p_r = world_pos_from_depth(uv + vec2<f32>(texel.x, 0.0), d_r);
    let p_u = world_pos_from_depth(uv + vec2<f32>(0.0, -texel.y), d_u);

    let d_x = p_r - p_c;
    let d_y = p_u - p_c;
    return normalize(cross(d_x, d_y));
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let depth = textureSample(t_depth, s_point, in.uv);
    if (depth >= 1.0 || depth <= 0.0) {
        return vec4<f32>(0.0);
    }

    let tex_dim = vec2<f32>(textureDimensions(t_depth));
    let world_pos = world_pos_from_depth(in.uv, depth);
    let normal = normal_from_depth(in.uv, depth, tex_dim);

    let V = normalize(camera.camera_pos.xyz - world_pos);
    let R = reflect(-V, normal);

    // Only march rays pointing generally towards or across the camera view
    if (dot(normal, V) <= 0.0) {
        return vec4<f32>(0.0);
    }

    // Step raymarching in world/view space
    let step_size = ssr.max_ray_distance / f32(ssr.max_steps);
    var current_pos = world_pos + normal * 0.05; // Offset to avoid self-intersection
    var hit = false;
    var hit_uv = vec2<f32>(0.0);
    var hit_dist = 0.0;

    for (var i = 0u; i < ssr.max_steps; i = i + 1u) {
        current_pos = current_pos + R * step_size;

        // Project ray pos to clip space
        let clip_pos = camera.view_proj * vec4<f32>(current_pos, 1.0);
        if (clip_pos.w <= 0.0) {
            break;
        }

        let ndc = clip_pos.xyz / clip_pos.w;
        let sample_uv = vec2<f32>(ndc.x * 0.5 + 0.5, 1.0 - (ndc.y * 0.5 + 0.5));

        // Screen boundary check
        if (sample_uv.x < 0.0 || sample_uv.x > 1.0 || sample_uv.y < 0.0 || sample_uv.y > 1.0) {
            break;
        }

        let sampled_depth = textureSample(t_depth, s_point, sample_uv);
        if (sampled_depth < 1.0 && ndc.z >= sampled_depth) {
            // Reconstruct depth surface distance
            let surface_pos = world_pos_from_depth(sample_uv, sampled_depth);
            let dist_to_surface = length(current_pos - surface_pos);

            if (dist_to_surface < ssr.thickness) {
                hit = true;
                hit_uv = sample_uv;
                hit_dist = length(current_pos - world_pos);
                break;
            }
        }
    }

    if (!hit) {
        return vec4<f32>(0.0);
    }

    // Edge fading
    let edge_dist = min(min(hit_uv.x, 1.0 - hit_uv.x), min(hit_uv.y, 1.0 - hit_uv.y)) * 2.0;
    let edge_factor = clamp((edge_dist - (1.0 - ssr.edge_fade_end)) / (ssr.edge_fade_end - ssr.edge_fade_start), 0.0, 1.0);

    // Distance fading
    let dist_factor = clamp(1.0 - (hit_dist / ssr.max_ray_distance), 0.0, 1.0);

    // Sample reflected scene color
    let reflected_color = textureSample(t_color, s_linear, hit_uv).rgb;
    let alpha = edge_factor * dist_factor * ssr.intensity * 0.7;

    return vec4<f32>(reflected_color * alpha, alpha);
}
