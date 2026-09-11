// [WFGY] Zone: SAFE | λ: 0.3 | Fallbacks: 0/None | Action: Volumetric Fog & God Rays Raymarching Shader with Mie Phase Scattering
struct VolumetricUniform {
    inv_view_proj: mat4x4<f32>,
    light_dir: vec4<f32>,      // xyz = dir to light, w = density
    light_color: vec4<f32>,    // rgb = sun color, w = intensity
    fog_params: vec4<f32>,     // x = mie g, y = max distance, z = step count, w = jitter strength
};

struct CameraUniform {
    view_proj: mat4x4<f32>,
    view: mat4x4<f32>,
    proj: mat4x4<f32>,
    camera_pos: vec4<f32>,
};

struct PointLightGpu {
    position: vec4<f32>,
    color: vec4<f32>,
};

struct LightUniform {
    position: vec4<f32>,
    color: vec4<f32>,
    ambient: vec4<f32>,
    fog_color: vec4<f32>,
    cascade_matrices: array<mat4x4<f32>, 4>,
    cascade_splits: vec4<f32>,
    point_light_count: u32,
    viewport_width: f32,
    viewport_height: f32,
    render_flags: u32,
    point_lights: array<PointLightGpu, 16>,
};

@group(0) @binding(0)
var<uniform> vol: VolumetricUniform;

@group(0) @binding(1)
var<uniform> camera: CameraUniform;

@group(0) @binding(2)
var<uniform> light: LightUniform;

@group(0) @binding(3)
var t_depth: texture_depth_2d;

@group(0) @binding(4)
var s_depth: sampler;

@group(0) @binding(5)
var t_shadow: texture_depth_2d_array;

@group(0) @binding(6)
var s_shadow: sampler_comparison;

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

// Fonction de phase de Mie (Henyey-Greenstein) pour la diffusion avant de la lumière (God Rays)
fn henyey_greenstein(cos_theta: f32, g: f32) -> f32 {
    let g2 = g * g;
    let denom = 1.0 + g2 - 2.0 * g * cos_theta;
    return (1.0 - g2) / (4.0 * 3.14159265 * pow(max(denom, 0.001), 1.5));
}

// Générateur pseudo-aléatoire pour le dithering/jittering sub-pas (élimine le banding)
fn interleaved_gradient_noise(screen_pos: vec2<f32>) -> f32 {
    let magic = vec3<f32>(0.06711056, 0.00583715, 52.9829189);
    return fract(magic.z * fract(dot(screen_pos, magic.xy)));
}

// Échantillonnage de l'ombre volumétrique pour un point dans le monde
fn sample_shadow_visibility(world_pos: vec3<f32>, view_depth: f32) -> f32 {
    var cascade_idx = 3;
    if (view_depth < light.cascade_splits.x) {
        cascade_idx = 0;
    } else if (view_depth < light.cascade_splits.y) {
        cascade_idx = 1;
    } else if (view_depth < light.cascade_splits.z) {
        cascade_idx = 2;
    } else if (view_depth < light.cascade_splits.w) {
        cascade_idx = 3;
    } else {
        return 1.0; // Hors cascade
    }

    let shadow_pos = light.cascade_matrices[cascade_idx] * vec4<f32>(world_pos, 1.0);
    let proj_coords = shadow_pos.xyz / shadow_pos.w;
    let shadow_uv = vec2<f32>(
        proj_coords.x * 0.5 + 0.5,
        -proj_coords.y * 0.5 + 0.5,
    );
    let current_depth = proj_coords.z;

    if (shadow_uv.x < 0.0 || shadow_uv.x > 1.0 || shadow_uv.y < 0.0 || shadow_uv.y > 1.0 || current_depth > 1.0 || current_depth < 0.0) {
        return 1.0;
    }

    let shadow_acc = textureSampleCompare(
        t_shadow,
        s_shadow,
        shadow_uv,
        cascade_idx,
        current_depth - 0.001,
    );

    return shadow_acc;
}

@fragment
fn fs_volumetric(in: VertexOutput) -> @location(0) vec4<f32> {
    let depth_sample = textureSample(t_depth, s_depth, in.uv);
    
    // Reconstruction de la position monde de la géométrie de fond
    let clip_space_pos = vec4<f32>(
        in.uv.x * 2.0 - 1.0,
        (1.0 - in.uv.y) * 2.0 - 1.0,
        depth_sample,
        1.0,
    );
    let world_pos_h = vol.inv_view_proj * clip_space_pos;
    let geo_world_pos = world_pos_h.xyz / world_pos_h.w;

    let cam_pos = camera.camera_pos.xyz;
    let ray_dir_full = geo_world_pos - cam_pos;
    let geo_dist = length(ray_dir_full);
    let ray_dir = normalize(ray_dir_full);

    let max_dist = min(geo_dist, vol.fog_params.y);
    if (max_dist <= 0.1) {
        return vec4<f32>(0.0);
    }

    let step_count = u32(vol.fog_params.z);
    let step_size = max_dist / f32(step_count);

    // Dithering pour adoucir les tranches
    let jitter = interleaved_gradient_noise(in.position.xy);
    var current_dist = step_size * jitter;

    let L_sun = normalize(vol.light_dir.xyz);
    let cos_theta = dot(ray_dir, L_sun);
    let phase = henyey_greenstein(cos_theta, vol.fog_params.x);

    let density = vol.light_dir.w;
    let absorption = 0.04;
    let scattering = density;
    let extinction = absorption + scattering;

    var transmittance = 1.0;
    var in_scattering = vec3<f32>(0.0);

    for (var i = 0u; i < step_count; i = i + 1u) {
        if (current_dist >= max_dist || transmittance < 0.01) {
            break;
        }

        let sample_pos = cam_pos + ray_dir * current_dist;
        let view_depth = current_dist;

        // Échantillonnage de visibilité de l'ombre
        let shadow_vis = sample_shadow_visibility(sample_pos, view_depth);

        if (shadow_vis > 0.0) {
            let step_transmittance = exp(-extinction * step_size);
            let light_energy = vol.light_color.rgb * vol.light_color.w * shadow_vis;
            let step_in_scatter = light_energy * (scattering * phase) * ((1.0 - step_transmittance) / extinction);

            in_scattering += step_in_scatter * transmittance;
            transmittance *= step_transmittance;
        } else {
            transmittance *= exp(-extinction * step_size);
        }

        current_dist += step_size;
    }

    return vec4<f32>(in_scattering, 1.0 - transmittance);
}
