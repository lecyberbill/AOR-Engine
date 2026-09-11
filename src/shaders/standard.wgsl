// [WFGY] Zone: SAFE | λ: 0.3 | Fallbacks: 0/None | Action: Advanced PBR Shader with Directional PCF Shadow Mapping, Water Reflection & ACES

struct CameraUniform {
    view_proj: mat4x4<f32>,
    view: mat4x4<f32>,
    proj: mat4x4<f32>,
    camera_pos: vec4<f32>, // xyz = cam pos, w = time
};

struct PointLightGpu {
    position: vec4<f32>, // xyz = pos, w = radius
    color: vec4<f32>,    // rgb = color, w = intensity
};

struct LightUniform {
    position: vec4<f32>,              // Sun pos, w = sun intensity
    color: vec4<f32>,                 // Sun color
    ambient: vec4<f32>,               // Ambient night color
    fog_color: vec4<f32>,             // Fog color rgb, w = fog density
    cascade_matrices: array<mat4x4<f32>, 4>, // 4 Matrices de projection d'ombres (CSM)
    cascade_splits: vec4<f32>,        // Distances de découpage en espace vue
    point_light_count: u32,
    viewport_width: f32,
    viewport_height: f32,
    render_flags: u32,
    point_lights: array<PointLightGpu, 16>,
};

struct ModelUniform {
    model: mat4x4<f32>,
    color: vec4<f32>,
    selected: u32,
    _pad0: u32,
    _pad1: u32,
    _pad2: u32,
};

struct MaterialUniform {
    roughness: f32,
    metallic: f32,
    uv_tiling: vec2<f32>,
    uv_offset: vec2<f32>,
    ior: f32,
    transmission: f32,
    emission_color: vec4<f32>,
    use_normal_map: u32,
    clearcoat: f32,
    clearcoat_roughness: f32,
    subsurface: f32,
};

@group(0) @binding(0)
var<uniform> camera: CameraUniform;

@group(0) @binding(1)
var<uniform> light: LightUniform;

@group(0) @binding(2)
var t_reflection: texture_2d<f32>;

@group(0) @binding(3)
var s_reflection: sampler;

@group(0) @binding(4)
var t_shadow: texture_depth_2d_array;

@group(0) @binding(5)
var s_shadow: sampler_comparison;

@group(0) @binding(6)
var t_ssao: texture_2d<f32>;

@group(0) @binding(7)
var s_ssao: sampler;

@group(0) @binding(8)
var t_skybox: texture_cube<f32>;

@group(0) @binding(9)
var s_skybox: sampler;

@group(1) @binding(0)
var<uniform> model: ModelUniform;

@group(1) @binding(1)
var<uniform> material: MaterialUniform;

@group(1) @binding(2)
var t_diffuse: texture_2d<f32>;

@group(1) @binding(3)
var t_normal: texture_2d<f32>;

@group(1) @binding(4)
var s_sampler: sampler;

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) tangent: vec4<f32>,
    @location(3) uv: vec2<f32>,
    @location(4) color: vec4<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) world_pos: vec3<f32>,
    @location(1) world_normal: vec3<f32>,
    @location(2) world_tangent: vec4<f32>,
    @location(3) uv: vec2<f32>,
    @location(4) color: vec4<f32>,
    @location(5) view_depth: f32,
};

@vertex
fn vs_main(in: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    let world_pos = model.model * vec4<f32>(in.position, 1.0);
    out.world_pos = world_pos.xyz;
    
    let normal_matrix = mat3x3<f32>(
        model.model[0].xyz,
        model.model[1].xyz,
        model.model[2].xyz,
    );
    
    let geom_normal = normalize(normal_matrix * in.normal);
    let geom_tangent = normalize(normal_matrix * in.tangent.xyz);
    
    out.world_normal = geom_normal;
    out.world_tangent = vec4<f32>(geom_tangent, in.tangent.w);
    let clip_pos = camera.view_proj * world_pos;
    out.clip_position = clip_pos;
    
    let view_pos = camera.view * world_pos;
    out.view_depth = -view_pos.z; // Profondeur positive le long de l'axe de vue
    
    out.uv = in.uv * material.uv_tiling + material.uv_offset;
    
    if (model.color.a > 0.0) {
        out.color = model.color;
    } else {
        out.color = in.color;
    }
    
    return out;
}

struct InstanceInput {
    @location(5) model_matrix_0: vec4<f32>,
    @location(6) model_matrix_1: vec4<f32>,
    @location(7) model_matrix_2: vec4<f32>,
    @location(8) model_matrix_3: vec4<f32>,
    @location(9) color_tint: vec4<f32>,
};

@vertex
fn vs_instanced(in: VertexInput, inst: InstanceInput) -> VertexOutput {
    var out: VertexOutput;
    let inst_model = mat4x4<f32>(
        inst.model_matrix_0,
        inst.model_matrix_1,
        inst.model_matrix_2,
        inst.model_matrix_3,
    );
    let world_model = model.model * inst_model;
    let world_pos = world_model * vec4<f32>(in.position, 1.0);
    out.world_pos = world_pos.xyz;
    
    let normal_matrix = mat3x3<f32>(
        world_model[0].xyz,
        world_model[1].xyz,
        world_model[2].xyz,
    );
    
    let geom_normal = normalize(normal_matrix * in.normal);
    let geom_tangent = normalize(normal_matrix * in.tangent.xyz);
    
    out.world_normal = geom_normal;
    out.world_tangent = vec4<f32>(geom_tangent, in.tangent.w);
    let clip_pos = camera.view_proj * world_pos;
    out.clip_position = clip_pos;
    
    let view_pos = camera.view * world_pos;
    out.view_depth = -view_pos.z;
    
    out.uv = in.uv * material.uv_tiling + material.uv_offset;
    out.color = in.color * inst.color_tint;
    if (model.color.a > 0.0) {
        out.color = model.color * inst.color_tint;
    }
    return out;
}

// Échantillonne une cascade spécifique avec filtrage PCF 3x3
fn sample_single_cascade(cascade_idx: i32, world_pos: vec3<f32>, NdotL: f32) -> f32 {
    let shadow_pos = light.cascade_matrices[cascade_idx] * vec4<f32>(world_pos, 1.0);
    let proj_coords = shadow_pos.xyz / shadow_pos.w;
    
    let shadow_uv = vec2<f32>(
        proj_coords.x * 0.5 + 0.5,
        -proj_coords.y * 0.5 + 0.5,
    );
    
    let current_depth = proj_coords.z;
    
    if (shadow_uv.x < 0.0 || shadow_uv.x > 1.0 || shadow_uv.y < 0.0 || shadow_uv.y > 1.0 || current_depth > 1.0 || current_depth < 0.0) {
        return 0.0;
    }
    
    // Biais géométrique adaptatif normalisé selon l'échelle de la cascade
    let cascade_bias_scale = 1.0 + f32(cascade_idx) * 1.5;
    let bias = max(0.0025 * (1.0 - NdotL) * cascade_bias_scale, 0.0006 * cascade_bias_scale);
    let compare_depth = current_depth - bias;
    
    let texel_size = 1.0 / 2048.0;
    var shadow_acc = 0.0;
    
    for (var x = -1; x <= 1; x = x + 1) {
        for (var y = -1; y <= 1; y = y + 1) {
            let offset = vec2<f32>(f32(x), f32(y)) * texel_size;
            shadow_acc = shadow_acc + textureSampleCompare(
                t_shadow,
                s_shadow,
                shadow_uv + offset,
                cascade_idx,
                compare_depth,
            );
        }
    }
    
    let shadow_intensity = shadow_acc / 9.0;
    return 1.0 - shadow_intensity; // 1.0 = dans l'ombre, 0.0 = éclairé
}

// Sélection de la cascade idoine avec transition douce (Cascade Blending)
fn calculate_csm_shadow(world_pos: vec3<f32>, view_depth: f32, NdotL: f32) -> f32 {
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
        return 0.0; // Au-delà de la portée maximale des ombres
    }
    
    let shadow_val = sample_single_cascade(cascade_idx, world_pos, NdotL);
    
    // Transition douce entre cascades pour éviter les artefacts de découpage brut
    let split_dist = select(
        light.cascade_splits.w,
        select(
            light.cascade_splits.z,
            select(light.cascade_splits.x, light.cascade_splits.y, cascade_idx == 1),
            cascade_idx == 2
        ),
        cascade_idx == 0 || cascade_idx == 1 || cascade_idx == 2
    );
    
    let blend_band = split_dist * 0.15;
    if (view_depth > split_dist - blend_band && cascade_idx < 3) {
        let blend_factor = (view_depth - (split_dist - blend_band)) / blend_band;
        let next_shadow = sample_single_cascade(cascade_idx + 1, world_pos, NdotL);
        return mix(shadow_val, next_shadow, clamp(blend_factor, 0.0, 1.0));
    }
    
    return shadow_val;
}

// Ondes de Gerstner fluides pour la perturbation de l'eau
fn compute_water_normal(world_pos: vec3<f32>, time: f32) -> vec3<f32> {
    let p = world_pos.xz * 1.3;
    let t = time * 1.5;

    let d1 = vec2<f32>(0.707, 0.707);
    let w1 = dot(p, d1) * 1.8 + t * 1.3;
    let s1 = cos(w1) * 0.08 * d1;

    let d2 = vec2<f32>(-0.6, 0.8);
    let w2 = dot(p, d2) * 2.6 - t * 1.5;
    let s2 = cos(w2) * 0.05 * d2;

    let d3 = vec2<f32>(0.92, -0.38);
    let w3 = dot(p, d3) * 5.4 + t * 2.2;
    let s3 = cos(w3) * 0.03 * d3;

    let d4 = vec2<f32>(-0.45, -0.89);
    let w4 = dot(p, d4) * 11.5 - t * 2.9;
    let s4 = cos(w4) * 0.015 * d4;

    let slope = s1 + s2 + s3 + s4;
    return normalize(vec3<f32>(-slope.x, 1.0, -slope.y));
}

// Tonemapping cinématographique ACES Film
fn aces_tonemap(x: vec3<f32>) -> vec3<f32> {
    let a = 2.51;
    let b = 0.03;
    let c = 2.43;
    let d = 0.59;
    let e = 0.14;
    return clamp((x * (a * x + b)) / (x * (c * x + d) + e), vec3<f32>(0.0), vec3<f32>(1.0));
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let time = camera.camera_pos.w;

    // 1. Reconstruction du repère Tangent (TBN)
    let N_geom = normalize(in.world_normal);
    let T = normalize(in.world_tangent.xyz - dot(in.world_tangent.xyz, N_geom) * N_geom);
    let B = normalize(cross(N_geom, T) * in.world_tangent.w);
    let TBN = mat3x3<f32>(T, B, N_geom);

    // 2. Calcul de la normale finale
    var N = N_geom;
    if (material.use_normal_map == 1u) {
        let normal_sample = textureSample(t_normal, s_sampler, in.uv).xyz * 2.0 - vec3<f32>(1.0);
        N = normalize(TBN * normal_sample);
    }

    var water_norm = N;
    if (material.transmission > 0.05) {
        water_norm = compute_water_normal(in.world_pos, time);
        N = normalize(mix(N, water_norm, material.transmission));
    }

    // 3. Échantillonnage de la texture diffuse
    let tex_color = textureSample(t_diffuse, s_sampler, in.uv);
    let albedo = in.color.rgb * tex_color.rgb;
    let alpha = in.color.a * tex_color.a;

    // 4. Vecteurs d'éclairage
    let V = normalize(camera.camera_pos.xyz - in.world_pos);
    let L_sun = normalize(light.position.xyz - in.world_pos);
    let H_sun = normalize(L_sun + V);

    let NdotL = max(dot(N, L_sun), 0.0);
    let NdotH = max(dot(N, H_sun), 0.0);
    let NdotV = max(dot(N, V), 0.0);

    // 5. Calcul de l'ombre portée multi-cascades (Cascaded Shadow Mapping)
    let shadow = calculate_csm_shadow(in.world_pos, in.view_depth, NdotL);
    let light_visibility = 1.0 - shadow;

    // 6. Modèle PBR (Roughness & Metallic)
    let roughness = clamp(material.roughness, 0.02, 1.0);
    let metallic = clamp(material.metallic, 0.0, 1.0);

    let F0 = mix(vec3<f32>(0.04), albedo, metallic);
    let F = F0 + (1.0 - F0) * pow(1.0 - max(dot(H_sun, V), 0.0), 5.0);

    // Composante Spéculaire Solaire (modulée par l'ombre)
    let shininess = mix(320.0, 4.0, roughness);
    let spec_factor = pow(NdotH, shininess) * ((shininess + 2.0) / 8.0);
    let sun_specular = F * spec_factor * light.color.rgb * NdotL * light.position.w * light_visibility;
    let kD = (vec3<f32>(1.0) - F) * (1.0 - metallic);
    let sun_diffuse_std = kD * albedo * light.color.rgb * NdotL * light.position.w * light_visibility;

    // Modèle de Diffusion Sous-Surfacique (Subsurface Scattering SSS - Wrap Lighting)
    let sss_wrap = clamp(material.subsurface, 0.0, 1.0);
    let sss_ndotl = clamp((dot(N, L_sun) + sss_wrap) / (1.0 + sss_wrap), 0.0, 1.0);
    let sss_sun_diffuse = kD * albedo * light.color.rgb * sss_ndotl * light.position.w * light_visibility;
    let sun_diffuse = mix(sun_diffuse_std, sss_sun_diffuse, sss_wrap);

    // 7. Éclairage Multi-Lumières Ponctuelles (Lanternes, Feux, Cristaux)
    var point_diffuse = vec3<f32>(0.0);
    var point_specular = vec3<f32>(0.0);

    let num_pt = min(light.point_light_count, 16u);
    for (var i = 0u; i < num_pt; i = i + 1u) {
        let pt = light.point_lights[i];
        let to_light = pt.position.xyz - in.world_pos;
        let dist = length(to_light);
        let radius = max(pt.position.w, 0.1);

        if (dist < radius) {
            let L_pt = normalize(to_light);
            let H_pt = normalize(L_pt + V);
            let NdotL_pt = max(dot(N, L_pt), 0.0);
            let NdotH_pt = max(dot(N, H_pt), 0.0);

            let att_dist = 1.0 / (1.0 + 0.12 * dist + 0.05 * dist * dist);
            let window = clamp(1.0 - pow(dist / radius, 4.0), 0.0, 1.0);
            let attenuation = att_dist * (window * window) * pt.color.w;

            let light_energy = pt.color.rgb * attenuation;
            let pt_ndotl = mix(NdotL_pt, clamp((dot(N, L_pt) + sss_wrap) / (1.0 + sss_wrap), 0.0, 1.0), sss_wrap);
            point_diffuse = point_diffuse + kD * albedo * light_energy * pt_ndotl;
            point_specular = point_specular + F * pow(NdotH_pt, shininess) * ((shininess + 2.0) / 8.0) * light_energy * NdotL_pt;
        }
    }

    // Composante Ambiante avec modulation SSAO et Éclairage Basé Image (IBL)
    let screen_uv = in.clip_position.xy / vec2<f32>(light.viewport_width, light.viewport_height);
    let ssao_val = textureSample(t_ssao, s_ssao, clamp(screen_uv, vec2<f32>(0.001), vec2<f32>(0.999))).r;
    let ssao_factor = mix(1.0, ssao_val, 0.85); // Poids d'occlusion 85%

    // Échantillonnage IBL Spéculaire (Radiance pré-filtrée par niveau de rugosité)
    let R = reflect(-V, N);
    let max_lod = 8.0;
    let env_lod = roughness * max_lod;
    let env_specular = textureSampleLevel(t_skybox, s_skybox, R, env_lod).rgb;

    // Échantillonnage IBL Diffuse (Irradiance hémisphérique dans la direction de la normale)
    let env_diffuse = textureSampleLevel(t_skybox, s_skybox, N, max_lod - 1.0).rgb;

    // Approximation de Fresnel-Schlick pour l'environnement IBL
    let F_env = F0 + (max(vec3<f32>(1.0 - roughness), F0) - F0) * pow(1.0 - NdotV, 5.0);
    let kD_env = (vec3<f32>(1.0) - F_env) * (1.0 - metallic);

    let ibl_diffuse = kD_env * env_diffuse * albedo;
    let ibl_specular = F_env * env_specular;

    // 7b. Couche Spéculaire de Vernis Multicouche (Clearcoat Lobe)
    var clearcoat_energy = vec3<f32>(0.0);
    if (material.clearcoat > 0.001) {
        let cc_roughness = clamp(material.clearcoat_roughness, 0.01, 1.0);
        let cc_shininess = mix(600.0, 16.0, cc_roughness);
        let F_cc = 0.04 + 0.96 * pow(1.0 - max(dot(H_sun, V), 0.0), 5.0);
        let cc_sun = F_cc * pow(NdotH, cc_shininess) * ((cc_shininess + 2.0) / 8.0) * light.color.rgb * NdotL * light.position.w * light_visibility;

        let cc_env_lod = cc_roughness * max_lod;
        let cc_env_specular = textureSampleLevel(t_skybox, s_skybox, R, cc_env_lod).rgb;
        let F_cc_env = 0.04 + 0.96 * pow(1.0 - NdotV, 5.0);
        let cc_ibl = F_cc_env * cc_env_specular * ssao_factor;

        clearcoat_energy = (cc_sun + cc_ibl) * material.clearcoat;
    }

    let ambient_ibl = (ibl_diffuse + ibl_specular + light.ambient.rgb * albedo * 0.25) * ssao_factor;
    let emission = material.emission_color.rgb * material.emission_color.a * 1.8;

    var final_color = ambient_ibl + sun_diffuse + sun_specular + point_diffuse + point_specular + clearcoat_energy + emission;

    // 8. Rendu Réel de l'Eau avec Réflexions Planaires GPU (Mode Jeu Vidéo)
    if (material.transmission > 0.05 && light.render_flags == 1u) {
        let screen_uv = in.clip_position.xy / vec2<f32>(light.viewport_width, light.viewport_height);
        let dist_perturb = water_norm.xz * vec2<f32>(0.035, -0.035);
        let refl_uv = clamp(screen_uv + dist_perturb, vec2<f32>(0.002, 0.002), vec2<f32>(0.998, 0.998));

        let planar_reflection = textureSample(t_reflection, s_reflection, refl_uv).rgb;

        let deep_water = vec3<f32>(0.006, 0.038, 0.068);
        let shallow_water = vec3<f32>(0.02, 0.12, 0.18);
        let water_base = mix(deep_water, shallow_water, NdotL * 0.4 + 0.6);

        let F_water = 0.04 + 0.96 * pow(1.0 - NdotV, 5.0);
        let water_spec = pow(NdotH, 400.0) * light.color.rgb * 4.5 * (F_water * 0.7 + 0.3) * light_visibility;

        let water_surface = mix(water_base * light.ambient.rgb * 2.0, planar_reflection, F_water) + point_specular * 2.8 + water_spec;
        final_color = mix(final_color, water_surface, material.transmission);
    }

    // 9. Brouillard Atmosphérique Nocturne (Cavern Fog)
    if (light.render_flags == 1u) {
        let dist_to_cam = length(camera.camera_pos.xyz - in.world_pos);
        let fog_density = light.fog_color.w;
        let fog_factor = clamp(1.0 - exp(-dist_to_cam * fog_density), 0.0, 1.0);
        final_color = mix(final_color, light.fog_color.rgb, fog_factor);
    }

    // 10. Effet de sélection
    if (model.selected == 1u) {
        final_color = final_color * 1.18 + vec3<f32>(0.08, 0.15, 0.22);
    }

    return vec4<f32>(final_color, alpha);
}
