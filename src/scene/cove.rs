// [WFGY] Zone: SAFE | λ: 0.3 | Fallbacks: 0/None | Action: High-Fidelity Night Cove & Lantern Cavern Procedural Generator with Individual Planks, Point Lights & Water Reflections
#![allow(dead_code)]
use crate::gpu::GpuTexture;
use crate::scene::mesh::{compute_tangents, generate_cube_data, generate_pyramid_data, PrimitiveType, Vertex};
use crate::scene::node::{MaterialUniform, SceneNode};
use glam::Vec3;
use std::collections::HashMap;
use std::sync::Arc;

/// Type d'entité dynamique dans l'écosystème de la crique nocturne
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub enum CoveEntityType {
    /// Lanterne flottante sur l'eau (tangage, roulis et houle sinusoïdale)
    BobbingLantern {
        float_amplitude: f32,
        bob_frequency: f32,
        tilt_amplitude: f32,
    },
    /// Barque en bois amarrée qui ondule doucement au fil de l'eau
    MooredBoat {
        rocking_amplitude: f32,
        rocking_freq: f32,
    },
    /// Lanterne sur poteau en bois avec vacillement réaliste de la flamme
    FlickeringLantern {
        flicker_speed: f32,
        intensity_min: f32,
        intensity_max: f32,
    },
    /// Cristal bioluminescent cavernicole avec pulsation mystique
    GlowingCrystal {
        pulse_min: f32,
        pulse_max: f32,
        pulse_freq: f32,
    },
}

/// Description d'une entité dynamique rattachée à un nœud de la crique
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct CoveDynamicEntity {
    pub node_id: usize,
    pub entity_type: CoveEntityType,
    pub base_translation: Vec3,
    pub base_rotation: Vec3,
    pub base_emission: [f32; 4],
    pub base_color: [f32; 4],
    pub phase: f32,
    pub speed: f32,
}

/// Gestionnaire d'animation de la crique et du lagon nocturne
#[derive(Clone, Debug)]
pub struct CoveEcosystem {
    pub entities: Vec<CoveDynamicEntity>,
    pub water_speed: f32,
    pub time: f32,
    pub enabled: bool,
}

impl Default for CoveEcosystem {
    fn default() -> Self {
        Self {
            entities: Vec::new(),
            water_speed: 1.4,
            time: 0.0,
            enabled: true,
        }
    }
}

impl CoveEcosystem {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn clear(&mut self) {
        self.entities.clear();
        self.time = 0.0;
    }

    /// Met à jour l'ensemble des éléments dynamiques (lanternes, barques, cristaux)
    pub fn update(
        &mut self,
        dt: f32,
        nodes: &mut [SceneNode],
        queue: &wgpu::Queue,
    ) {
        if !self.enabled || self.entities.is_empty() || dt <= 0.0 {
            return;
        }

        self.time += dt * self.water_speed;
        let t = self.time;

        let mut node_map: HashMap<usize, &mut SceneNode> = HashMap::new();
        for node in nodes.iter_mut() {
            node_map.insert(node.id, node);
        }

        for ent in &self.entities {
            if let Some(node) = node_map.get_mut(&ent.node_id) {
                match &ent.entity_type {
                    CoveEntityType::BobbingLantern { float_amplitude, bob_frequency, tilt_amplitude } => {
                        let local_t = t * bob_frequency * ent.speed + ent.phase;
                        let dy = (local_t).sin() * float_amplitude;
                        let roll = (local_t * 0.85).cos() * tilt_amplitude;
                        let pitch = (local_t * 1.15).sin() * tilt_amplitude * 0.7;

                        node.transform.translation = ent.base_translation + Vec3::new(0.0, dy, 0.0);
                        node.transform.rotation = ent.base_rotation + Vec3::new(pitch, 0.0, roll);

                        let flicker = 0.92 + 0.16 * (local_t * 3.7).sin() * (local_t * 5.3).cos();
                        node.material.emission_color = [
                            ent.base_emission[0] * flicker,
                            ent.base_emission[1] * flicker,
                            ent.base_emission[2] * flicker,
                            ent.base_emission[3] * flicker,
                        ];
                        node.material_uniform.update(queue, &node.material);
                    }
                    CoveEntityType::MooredBoat { rocking_amplitude, rocking_freq } => {
                        let local_t = t * rocking_freq * ent.speed + ent.phase;
                        let dy = (local_t).sin() * 0.04;
                        let roll = (local_t * 0.9).cos() * rocking_amplitude;
                        let pitch = (local_t * 1.3).sin() * (rocking_amplitude * 0.4);

                        node.transform.translation = ent.base_translation + Vec3::new(0.0, dy, 0.0);
                        node.transform.rotation = ent.base_rotation + Vec3::new(pitch, 0.0, roll);
                    }
                    CoveEntityType::FlickeringLantern { flicker_speed, intensity_min, intensity_max } => {
                        let local_t = t * flicker_speed * ent.speed + ent.phase;
                        let noise = ((local_t * 4.2).sin() * 0.5 + (local_t * 9.1).cos() * 0.3 + (local_t * 17.3).sin() * 0.2) * 0.5 + 0.5;
                        let intensity = intensity_min + (intensity_max - intensity_min) * noise;

                        node.material.emission_color = [
                            ent.base_emission[0] * intensity,
                            ent.base_emission[1] * intensity,
                            ent.base_emission[2] * intensity,
                            ent.base_emission[3] * intensity,
                        ];
                        node.material_uniform.update(queue, &node.material);
                    }
                    CoveEntityType::GlowingCrystal { pulse_min, pulse_max, pulse_freq } => {
                        let local_t = t * pulse_freq * ent.speed + ent.phase;
                        let s = (local_t.sin() * 0.5 + 0.5).clamp(0.0, 1.0);
                        let intensity = pulse_min + (pulse_max - pulse_min) * (s * s);

                        node.material.emission_color = [
                            ent.base_emission[0] * intensity,
                            ent.base_emission[1] * intensity,
                            ent.base_emission[2] * intensity,
                            ent.base_emission[3] * intensity,
                        ];
                        node.material_uniform.update(queue, &node.material);
                    }
                }
            }
        }
    }
}

/// Génère un maillage de plan d'eau haute résolution subdivisé
fn generate_subdivided_plane(size: f32, subdivisions: u32) -> (Vec<Vertex>, Vec<u32>) {
    let mut vertices = Vec::new();
    let mut indices = Vec::new();

    let half = size * 0.5;
    let step = size / (subdivisions as f32);

    for z_i in 0..=subdivisions {
        let z = -half + (z_i as f32) * step;
        let v = (z_i as f32) / (subdivisions as f32);
        for x_i in 0..=subdivisions {
            let x = -half + (x_i as f32) * step;
            let u = (x_i as f32) / (subdivisions as f32);

            vertices.push(Vertex {
                position: [x, 0.0, z],
                normal: [0.0, 1.0, 0.0],
                tangent: [1.0, 0.0, 0.0, 1.0],
                uv: [u * 12.0, v * 12.0],
                color: [0.05, 0.22, 0.35, 1.0],
            });
        }
    }

    let stride = subdivisions + 1;
    for z_i in 0..subdivisions {
        for x_i in 0..subdivisions {
            let top_left = z_i * stride + x_i;
            let top_right = top_left + 1;
            let bottom_left = (z_i + 1) * stride + x_i;
            let bottom_right = bottom_left + 1;

            indices.push(top_left);
            indices.push(bottom_left);
            indices.push(top_right);

            indices.push(top_right);
            indices.push(bottom_left);
            indices.push(bottom_right);
        }
    }

    compute_tangents(&mut vertices, &indices);
    (vertices, indices)
}

/// Génère un tronçon de ponton haute fidélité constitué de vraies lattes de bois séparées et de poutres de soutien
fn generate_detailed_pier(length: f32, width: f32, plank_width: f32, gap: f32) -> (Vec<Vertex>, Vec<u32>) {
    let mut all_vertices = Vec::new();
    let mut all_indices = Vec::new();

    let num_planks = ((length) / (plank_width + gap)).floor() as usize;
    let half_w = width * 0.5;
    let plank_thickness = 0.08;

    // 1. Lattes transversales individuelles espacées
    for i in 0..num_planks {
        let z_center = -length * 0.5 + (i as f32 + 0.5) * (plank_width + gap);
        let (mut p_verts, p_inds) = generate_cube_data();
        let half_pw = plank_width * 0.5;
        let half_pt = plank_thickness * 0.5;

        for v in &mut p_verts {
            let mut pos = Vec3::from_array(v.position);
            pos.x *= half_w;
            pos.y *= half_pt;
            pos.z = pos.z * half_pw + z_center;
            v.position = pos.to_array();
            v.uv = [pos.x * 1.5, pos.z * 1.5];
            v.color = [0.55 + ((i % 3) as f32) * 0.03, 0.38 + ((i % 2) as f32) * 0.02, 0.24, 1.0];
        }

        let base_idx = all_vertices.len() as u32;
        all_vertices.extend(p_verts);
        all_indices.extend(p_inds.iter().map(|idx| idx + base_idx));
    }

    // 2. Poutres longitudinales de soutien sous les lattes (2 longerons)
    let beam_w = 0.16;
    let beam_h = 0.22;
    for &side_x in &[-half_w * 0.75, half_w * 0.75] {
        let (mut b_verts, b_inds) = generate_cube_data();
        for v in &mut b_verts {
            let mut pos = Vec3::from_array(v.position);
            pos.x = pos.x * (beam_w * 0.5) + side_x;
            pos.y = pos.y * (beam_h * 0.5) - (plank_thickness * 0.5 + beam_h * 0.5);
            pos.z *= length * 0.5;
            v.position = pos.to_array();
            v.uv = [pos.z * 1.2, pos.y * 2.0];
            v.color = [0.40, 0.26, 0.16, 1.0];
        }
        let base_idx = all_vertices.len() as u32;
        all_vertices.extend(b_verts);
        all_indices.extend(b_inds.iter().map(|idx| idx + base_idx));
    }

    compute_tangents(&mut all_vertices, &all_indices);
    (all_vertices, all_indices)
}

/// Génère une lanterne japonaise en bois ajourée avec toit en pointe
fn generate_lantern_frame() -> (Vec<Vertex>, Vec<u32>) {
    let mut all_vertices = Vec::new();
    let mut all_indices = Vec::new();

    // Toit en pyramide
    let (mut roof_verts, roof_inds) = generate_pyramid_data();
    for v in &mut roof_verts {
        let mut p = Vec3::from_array(v.position);
        p.x *= 0.32;
        p.z *= 0.32;
        p.y = p.y * 0.18 + 0.22;
        v.position = p.to_array();
        v.color = [0.28, 0.16, 0.10, 1.0];
    }
    let base_idx = all_vertices.len() as u32;
    all_vertices.extend(roof_verts);
    all_indices.extend(roof_inds.iter().map(|idx| idx + base_idx));

    // Base en cube plat
    let (mut base_verts, base_inds) = generate_cube_data();
    for v in &mut base_verts {
        let mut p = Vec3::from_array(v.position);
        p.x *= 0.28;
        p.z *= 0.28;
        p.y = p.y * 0.05 - 0.22;
        v.position = p.to_array();
        v.color = [0.32, 0.20, 0.12, 1.0];
    }
    let base_idx = all_vertices.len() as u32;
    all_vertices.extend(base_verts);
    all_indices.extend(base_inds.iter().map(|idx| idx + base_idx));

    // 4 Piliers d'angle en bois
    for &px in &[-0.22, 0.22] {
        for &pz in &[-0.22, 0.22] {
            let (mut col_verts, col_inds) = generate_cube_data();
            for v in &mut col_verts {
                let mut p = Vec3::from_array(v.position);
                p.x = p.x * 0.035 + px;
                p.z = p.z * 0.035 + pz;
                p.y *= 0.20;
                v.position = p.to_array();
                v.color = [0.35, 0.22, 0.14, 1.0];
            }
            let base_idx = all_vertices.len() as u32;
            all_vertices.extend(col_verts);
            all_indices.extend(col_inds.iter().map(|idx| idx + base_idx));
        }
    }

    compute_tangents(&mut all_vertices, &all_indices);
    (all_vertices, all_indices)
}

/// Génère un maillage de barque en bois
fn generate_boat_mesh() -> (Vec<Vertex>, Vec<u32>) {
    let (mut base_verts, base_inds) = generate_cube_data();
    for v in &mut base_verts {
        let mut p = Vec3::from_array(v.position);
        if p.y < 0.0 {
            p.x *= 0.7;
            p.z *= 0.85;
        } else {
            if p.z > 0.4 {
                p.x *= 0.3; // Proue effilée
            }
        }
        v.position = p.to_array();
        v.color = [0.46, 0.30, 0.18, 1.0];
    }
    compute_tangents(&mut base_verts, &base_inds);
    (base_verts, base_inds)
}

/// Génère la scène complète "Crique Nocturne & Grotte aux Lanternes"
pub fn generate_night_cove(
    device: &wgpu::Device,
    model_bind_group_layout: &wgpu::BindGroupLayout,
    white_texture: Arc<GpuTexture>,
    flat_normal_texture: Arc<GpuTexture>,
    wood_texture: Arc<GpuTexture>,
    wood_normal_texture: Arc<GpuTexture>,
    start_id: usize,
) -> (Vec<SceneNode>, CoveEcosystem) {
    let mut nodes = Vec::new();
    let mut entities = Vec::new();
    let mut id = start_id;

    // =========================================================================
    // 1. SURFACE DE L'EAU DU LAGON (Plan d'eau avec Réflexions Planaires GPU)
    // =========================================================================
    let (water_verts, water_inds) = generate_subdivided_plane(64.0, 32);
    let mut water_mat = MaterialUniform::default();
    water_mat.roughness = 0.02;
    water_mat.metallic = 0.05;
    water_mat.ior = 1.33;
    water_mat.transmission = 1.0; // Active le shader d'ondes & réflexion planaire GPU
    water_mat.uv_tiling = [8.0, 8.0];

    let water_node = SceneNode::from_custom_mesh_full(
        device,
        model_bind_group_layout,
        white_texture.clone(),
        Some("Eau Lagonaire".to_string()),
        flat_normal_texture.clone(),
        None,
        Some(water_mat),
        [0.02, 0.12, 0.22, 0.95],
        id,
        "🌊 Surface de l'Eau (Réflexion Planaire GPU)",
        water_verts,
        water_inds,
        Vec3::new(0.0, 0.0, 0.0),
    );
    nodes.push(water_node);
    id += 1;

    // =========================================================================
    // 2. RÉSEAU DE PONTONS EN LATTES DE BOIS DÉTAILLÉES SUR PILOTIS
    // =========================================================================
    let mut wood_mat = MaterialUniform::default();
    wood_mat.roughness = 0.76;
    wood_mat.metallic = 0.0;
    wood_mat.use_normal_map = 1;
    wood_mat.uv_tiling = [4.0, 1.0];

    // Tronçons de pontons haute fidélité (longueur, largeur, position)
    let walkways = [
        // Ponton central
        (18.0, 2.4, Vec3::new(0.0, 0.35, -4.0), Vec3::ZERO, "Ponton Principal en Lattes"),
        // Embarcadère Ouest
        (10.0, 2.2, Vec3::new(-6.0, 0.35, 2.0), Vec3::new(0.0, 1.57, 0.0), "Embarcadère Ouest"),
        // Passerelle Est vers la grotte
        (12.0, 2.2, Vec3::new(7.0, 0.35, -6.0), Vec3::new(0.0, 1.57, 0.0), "Passerelle Est"),
        // Belvédère du Lagon
        (6.0, 5.0, Vec3::new(0.0, 0.35, -14.0), Vec3::ZERO, "Belvédère du Lagon"),
        // Débarcadère Sud
        (5.0, 3.5, Vec3::new(-10.0, 0.35, 2.0), Vec3::ZERO, "Débarcadère des Barques"),
    ];

    for (len, wid, pos, rot, name) in &walkways {
        let (p_verts, p_inds) = generate_detailed_pier(*len, *wid, 0.22, 0.02);

        let mut node = SceneNode::from_custom_mesh_full(
            device,
            model_bind_group_layout,
            wood_texture.clone(),
            Some("Bois / Parquet".to_string()),
            wood_normal_texture.clone(),
            Some("Bois Relief".to_string()),
            Some(wood_mat),
            [0.55, 0.38, 0.24, 1.0],
            id,
            name,
            p_verts,
            p_inds,
            *pos,
        );
        node.transform.rotation = *rot;
        nodes.push(node);
        id += 1;
    }

    // Pilotis en bois cylindriques verticaux immergés
    let stilt_positions = [
        Vec3::new(-1.0, -0.6, -11.0), Vec3::new(1.0, -0.6, -11.0),
        Vec3::new(-1.0, -0.6, -7.0),  Vec3::new(1.0, -0.6, -7.0),
        Vec3::new(-1.0, -0.6, -3.0),  Vec3::new(1.0, -0.6, -3.0),
        Vec3::new(-1.0, -0.6, 1.0),   Vec3::new(1.0, -0.6, 1.0),
        Vec3::new(-4.0, -0.6, 2.0),   Vec3::new(-8.0, -0.6, 2.0),
        Vec3::new(4.0, -0.6, -6.0),   Vec3::new(9.0, -0.6, -6.0),
        Vec3::new(-2.2, -0.6, -14.0), Vec3::new(2.2, -0.6, -14.0),
    ];

    for (i, pos) in stilt_positions.iter().enumerate() {
        let (mut c_verts, c_inds) = crate::scene::loader::generate_cylinder_data(0.18, 2.0, 10);
        compute_tangents(&mut c_verts, &c_inds);

        let node = SceneNode::from_custom_mesh_full(
            device,
            model_bind_group_layout,
            wood_texture.clone(),
            Some("Bois / Parquet".to_string()),
            wood_normal_texture.clone(),
            Some("Bois Relief".to_string()),
            Some(wood_mat),
            [0.42, 0.28, 0.18, 1.0],
            id,
            &format!("Pilotis Bois #{}", i + 1),
            c_verts,
            c_inds,
            *pos,
        );
        nodes.push(node);
        id += 1;
    }

    // =========================================================================
    // 3. LANTERNES JAPONAISES EN BOIS & FLAMMES ÉCLAIREUSES (Point Lights)
    // =========================================================================
    let lantern_posts = [
        Vec3::new(-1.1, 0.45, -11.0),
        Vec3::new(1.1, 0.45, -7.0),
        Vec3::new(-1.1, 0.45, -3.0),
        Vec3::new(1.1, 0.45, 1.0),
        Vec3::new(-5.0, 0.45, 2.9),
        Vec3::new(-9.0, 0.45, 2.9),
        Vec3::new(5.0, 0.45, -5.1),
        Vec3::new(10.0, 0.45, -5.1),
        Vec3::new(-2.3, 0.45, -16.2),
        Vec3::new(2.3, 0.45, -16.2),
    ];

    for (i, post_pos) in lantern_posts.iter().enumerate() {
        // Poteau vertical de lanterne
        let (mut post_verts, post_inds) = crate::scene::loader::generate_cylinder_data(0.08, 1.6, 8);
        compute_tangents(&mut post_verts, &post_inds);
        let post_node = SceneNode::from_custom_mesh_full(
            device,
            model_bind_group_layout,
            wood_texture.clone(),
            Some("Bois".to_string()),
            wood_normal_texture.clone(),
            None,
            Some(wood_mat),
            [0.35, 0.22, 0.14, 1.0],
            id,
            &format!("Poteau Lanterne #{}", i + 1),
            post_verts,
            post_inds,
            *post_pos + Vec3::new(0.0, 0.8, 0.0),
        );
        nodes.push(post_node);
        id += 1;

        // Structure de la lanterne japonaise (cadre bois + toit pyramide)
        let (l_frame_verts, l_frame_inds) = generate_lantern_frame();
        let frame_node = SceneNode::from_custom_mesh_full(
            device,
            model_bind_group_layout,
            wood_texture.clone(),
            None,
            wood_normal_texture.clone(),
            None,
            Some(wood_mat),
            [0.32, 0.20, 0.12, 1.0],
            id,
            &format!("Structure Lanterne #{}", i + 1),
            l_frame_verts,
            l_frame_inds,
            *post_pos + Vec3::new(0.0, 1.65, 0.0),
        );
        nodes.push(frame_node);
        id += 1;

        // Flamme intérieure émissive incandescente (Point Light)
        let (mut flame_verts, flame_inds) = crate::scene::loader::generate_sphere_data(0.12, 8, 8);
        compute_tangents(&mut flame_verts, &flame_inds);
        let mut flame_mat = MaterialUniform::default();
        flame_mat.roughness = 0.1;
        let base_emissive = [1.0, 0.82, 0.38, 4.5];
        flame_mat.emission_color = base_emissive;

        let flame_pos = *post_pos + Vec3::new(0.0, 1.65, 0.0);
        let flame_node = SceneNode::from_custom_mesh_full(
            device,
            model_bind_group_layout,
            white_texture.clone(),
            None,
            flat_normal_texture.clone(),
            None,
            Some(flame_mat),
            [1.0, 0.90, 0.60, 1.0],
            id,
            &format!("💡 Flamme Lanterne #{}", i + 1),
            flame_verts,
            flame_inds,
            flame_pos,
        );

        entities.push(CoveDynamicEntity {
            node_id: id,
            entity_type: CoveEntityType::FlickeringLantern {
                flicker_speed: 1.8,
                intensity_min: 0.85,
                intensity_max: 1.30,
            },
            base_translation: flame_pos,
            base_rotation: Vec3::ZERO,
            base_emission: base_emissive,
            base_color: [1.0, 0.90, 0.60, 1.0],
            phase: (i as f32) * 1.35,
            speed: 1.0 + (i as f32) * 0.1,
        });

        nodes.push(flame_node);
        id += 1;
    }

    // =========================================================================
    // 4. LANTERNES FLOTTANTES SUR L'EAU (Bobbing Lotus Candles)
    // =========================================================================
    let floating_lantern_coords = [
        Vec3::new(3.2, 0.08, -3.5),
        Vec3::new(-4.5, 0.08, -6.0),
        Vec3::new(6.8, 0.08, -10.5),
        Vec3::new(-3.0, 0.08, -14.0),
        Vec3::new(4.0, 0.08, 1.5),
        Vec3::new(-8.0, 0.08, -2.5),
    ];

    for (i, float_pos) in floating_lantern_coords.iter().enumerate() {
        let (mut fl_verts, fl_inds) = generate_cube_data();
        compute_tangents(&mut fl_verts, &fl_inds);
        let mut fl_mat = MaterialUniform::default();
        let base_glow = [1.0, 0.72, 0.28, 4.2];
        fl_mat.emission_color = base_glow;
        fl_mat.roughness = 0.2;

        let mut fl_node = SceneNode::from_custom_mesh_full(
            device,
            model_bind_group_layout,
            white_texture.clone(),
            None,
            flat_normal_texture.clone(),
            None,
            Some(fl_mat),
            [1.0, 0.80, 0.40, 1.0],
            id,
            &format!("🏮 Bougie Flottante #{}", i + 1),
            fl_verts,
            fl_inds,
            *float_pos,
        );
        fl_node.transform.scale = Vec3::new(0.32, 0.25, 0.32);

        entities.push(CoveDynamicEntity {
            node_id: id,
            entity_type: CoveEntityType::BobbingLantern {
                float_amplitude: 0.06,
                bob_frequency: 1.2,
                tilt_amplitude: 0.08,
            },
            base_translation: *float_pos,
            base_rotation: Vec3::new(0.0, (i as f32) * 0.7, 0.0),
            base_emission: base_glow,
            base_color: [1.0, 0.80, 0.40, 1.0],
            phase: (i as f32) * 1.8,
            speed: 0.9 + (i as f32) * 0.15,
        });

        nodes.push(fl_node);
        id += 1;
    }

    // =========================================================================
    // 5. BARQUES EN BOIS AMARRÉES AU PONTON
    // =========================================================================
    let boats = [
        (Vec3::new(-11.5, 0.12, 0.5), Vec3::new(0.0, 0.3, 0.0), "Barque en Bois #1"),
        (Vec3::new(-9.0, 0.12, 4.5), Vec3::new(0.0, -0.6, 0.0), "Barque en Bois #2"),
    ];

    for (b_pos, b_rot, b_name) in &boats {
        let (boat_verts, boat_inds) = generate_boat_mesh();
        let mut boat_node = SceneNode::from_custom_mesh_full(
            device,
            model_bind_group_layout,
            wood_texture.clone(),
            Some("Bois / Parquet".to_string()),
            wood_normal_texture.clone(),
            Some("Bois Relief".to_string()),
            Some(wood_mat),
            [0.48, 0.32, 0.20, 1.0],
            id,
            b_name,
            boat_verts,
            boat_inds,
            *b_pos,
        );
        boat_node.transform.scale = Vec3::new(1.1, 0.5, 2.4);
        boat_node.transform.rotation = *b_rot;

        entities.push(CoveDynamicEntity {
            node_id: id,
            entity_type: CoveEntityType::MooredBoat {
                rocking_amplitude: 0.05,
                rocking_freq: 0.9,
            },
            base_translation: *b_pos,
            base_rotation: *b_rot,
            base_emission: [0.0, 0.0, 0.0, 0.0],
            base_color: [0.48, 0.32, 0.20, 1.0],
            phase: (id as f32) * 1.1,
            speed: 1.0,
        });

        nodes.push(boat_node);
        id += 1;
    }

    // =========================================================================
    // 6. FORMATIONS ROCHEUSES, FALAISES & ARCHES DE LA GROTTE
    // =========================================================================
    let mut rock_mat = MaterialUniform::default();
    rock_mat.roughness = 0.88;
    rock_mat.metallic = 0.05;

    let rock_formations = [
        // Grande falaise Nord / Voûte de fond
        (Vec3::new(0.0, 7.0, -22.0), Vec3::new(34.0, 15.0, 6.0), Vec3::ZERO, "Grande Falaise de Fond (Nord)"),
        // Paroi Ouest de la grotte
        (Vec3::new(-18.0, 6.0, -6.0), Vec3::new(6.0, 13.0, 28.0), Vec3::new(0.0, 0.1, 0.05), "Paroi Rocheuse Ouest"),
        // Paroi Est de la grotte
        (Vec3::new(18.0, 6.0, -6.0), Vec3::new(6.0, 13.0, 28.0), Vec3::new(0.0, -0.1, -0.05), "Paroi Rocheuse Est"),
        // Arche rocheuse naturelle au-dessus de l'eau
        (Vec3::new(8.0, 5.5, -12.0), Vec3::new(12.0, 3.5, 4.0), Vec3::new(0.15, 0.4, -0.1), "Arche Rocheuse Naturelle"),
        // Pilier rocheux émergé 1
        (Vec3::new(-7.0, 1.8, -10.0), Vec3::new(3.0, 4.0, 3.0), Vec3::new(0.1, 0.3, 0.1), "Pilier Basaltique Ouest"),
        // Pilier rocheux émergé 2
        (Vec3::new(9.0, 2.2, 2.0), Vec3::new(3.5, 4.5, 3.5), Vec3::new(-0.1, -0.2, 0.1), "Éperon Rocheux Est"),
    ];

    for (r_pos, r_scale, r_rot, r_name) in &rock_formations {
        let (mut r_verts, r_inds) = generate_cube_data();
        compute_tangents(&mut r_verts, &r_inds);

        let mut r_node = SceneNode::from_custom_mesh_full(
            device,
            model_bind_group_layout,
            white_texture.clone(),
            None,
            flat_normal_texture.clone(),
            None,
            Some(rock_mat),
            [0.22, 0.21, 0.26, 1.0], // Teinte ardoise / basalte sombre
            id,
            r_name,
            r_verts,
            r_inds,
            *r_pos,
        );
        r_node.transform.scale = *r_scale;
        r_node.transform.rotation = *r_rot;
        nodes.push(r_node);
        id += 1;
    }

    // =========================================================================
    // 7. CRISTAUX BIOLUMINESCENTS DE LA GROTTE (Lueur Violette & Cyan)
    // =========================================================================
    let crystals = [
        (Vec3::new(-14.5, 3.2, -14.0), [0.2, 0.85, 1.0, 3.8], "Cristal Cyan Caverne #1"),
        (Vec3::new(-14.2, 5.0, -8.0), [0.95, 0.2, 0.85, 3.5], "Cristal Magenta Caverne #2"),
        (Vec3::new(14.5, 4.0, -15.0), [0.85, 0.15, 0.95, 3.8], "Cristal Violet Caverne #3"),
        (Vec3::new(14.0, 2.8, -4.0), [0.15, 0.95, 0.8, 3.5], "Cristal Emeraude Caverne #4"),
        (Vec3::new(-5.5, 2.8, -18.0), [0.9, 0.3, 0.9, 3.2], "Cristal Voûte Nord #5"),
    ];

    for (i, (c_pos, c_glow, c_name)) in crystals.iter().enumerate() {
        let (mut cr_verts, cr_inds) = crate::scene::loader::generate_sphere_data(0.5, 12, 12);
        compute_tangents(&mut cr_verts, &cr_inds);

        let mut cr_mat = MaterialUniform::default();
        cr_mat.emission_color = *c_glow;
        cr_mat.roughness = 0.15;

        let mut cr_node = SceneNode::from_custom_mesh_full(
            device,
            model_bind_group_layout,
            white_texture.clone(),
            None,
            flat_normal_texture.clone(),
            None,
            Some(cr_mat),
            [c_glow[0], c_glow[1], c_glow[2], 1.0],
            id,
            c_name,
            cr_verts,
            cr_inds,
            *c_pos,
        );
        cr_node.transform.scale = Vec3::new(0.6, 1.2, 0.6);

        entities.push(CoveDynamicEntity {
            node_id: id,
            entity_type: CoveEntityType::GlowingCrystal {
                pulse_min: 0.6,
                pulse_max: 1.4,
                pulse_freq: 0.8,
            },
            base_translation: *c_pos,
            base_rotation: Vec3::ZERO,
            base_emission: *c_glow,
            base_color: [c_glow[0], c_glow[1], c_glow[2], 1.0],
            phase: (i as f32) * 1.5,
            speed: 1.0,
        });

        nodes.push(cr_node);
        id += 1;
    }

    let ecosystem = CoveEcosystem {
        entities,
        water_speed: 1.4,
        time: 0.0,
        enabled: true,
    };

    (nodes, ecosystem)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cove_ecosystem_default() {
        let eco = CoveEcosystem::new();
        assert!(eco.enabled);
        assert!((eco.water_speed - 1.4).abs() < 1e-4);
        assert_eq!(eco.entities.len(), 0);
    }

    #[test]
    fn test_subdivided_plane_generation() {
        let (verts, inds) = generate_subdivided_plane(20.0, 4);
        assert_eq!(verts.len(), 25);
        assert_eq!(inds.len(), 96);
    }

    #[test]
    fn test_detailed_pier_generation() {
        let (verts, inds) = generate_detailed_pier(10.0, 2.0, 0.22, 0.02);
        assert!(!verts.is_empty());
        assert!(!inds.is_empty());
    }

    #[test]
    fn test_boat_mesh_generation() {
        let (verts, inds) = generate_boat_mesh();
        assert!(!verts.is_empty());
        assert!(!inds.is_empty());
    }
}
