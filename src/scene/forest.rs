// [WFGY] Zone: SAFE | λ: 0.3 | Fallbacks: 0/None | Action: Living 3D Forest Procedural Generator and Dynamic Entity Animation Engine
#![allow(dead_code)]
use crate::gpu::GpuTexture;
use crate::scene::mesh::PrimitiveType;
use crate::scene::node::SceneNode;
use glam::Vec3;
use std::collections::HashMap;
use std::sync::Arc;

/// Type d'entité dynamique dans la forêt vivante
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub enum DynamicEntityType {
    /// Luciole / Esprit volant avec trajectoire orbitale 3D et pulsation lumineuse
    Firefly {
        orbit_radius_x: f32,
        orbit_radius_z: f32,
        height_amplitude: f32,
        freq_y: f32,
        pulse_speed: f32,
    },
    /// Monolithe ou cristal magique en lévitation avec rotation et flottement doux
    FloatingMonolith {
        float_amplitude: f32,
        float_freq: f32,
        rot_speed_y: f32,
        rot_speed_z: f32,
    },
    /// Arbre soumis au balancement du vent
    SwayingTree {
        sway_factor: f32,
        bend_axis: Vec3,
    },
    /// Cristal au sol avec pulsation émissive
    PulsingFlora {
        pulse_min: f32,
        pulse_max: f32,
        pulse_freq: f32,
    },
}

/// Description d'une entité dynamique rattachée à un nœud de scène
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct ForestDynamicEntity {
    pub node_id: usize,
    pub entity_type: DynamicEntityType,
    pub base_translation: Vec3,
    pub base_rotation: Vec3,
    pub base_emission: [f32; 4],
    pub base_color: [f32; 4],
    pub phase: f32,
    pub speed: f32,
}

/// Gestionnaire d'animation de l'écosystème de la forêt
#[derive(Clone, Debug)]
pub struct ForestEcosystem {
    pub entities: Vec<ForestDynamicEntity>,
    pub wind_strength: f32,
    pub wind_speed: f32,
    pub time: f32,
    pub enabled: bool,
}

impl Default for ForestEcosystem {
    fn default() -> Self {
        Self {
            entities: Vec::new(),
            wind_strength: 0.35,
            wind_speed: 1.2,
            time: 0.0,
            enabled: true,
        }
    }
}

impl ForestEcosystem {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn clear(&mut self) {
        self.entities.clear();
        self.time = 0.0;
    }

    /// Met à jour toutes les entités dynamiques de la forêt en fonction du temps delta
    pub fn update(
        &mut self,
        dt: f32,
        nodes: &mut [SceneNode],
        queue: &wgpu::Queue,
    ) {
        if !self.enabled || self.entities.is_empty() || dt <= 0.0 {
            return;
        }

        self.time += dt * self.wind_speed;
        let t = self.time;
        let wind_str = self.wind_strength;

        // Indexation rapide des nœuds par ID
        let mut node_map: HashMap<usize, &mut SceneNode> = nodes.iter_mut().map(|n| (n.id, n)).collect();

        for entity in &self.entities {
            if let Some(node) = node_map.get_mut(&entity.node_id) {
                let node = &mut **node;
                match &entity.entity_type {
                    DynamicEntityType::Firefly {
                        orbit_radius_x,
                        orbit_radius_z,
                        height_amplitude,
                        freq_y,
                        pulse_speed,
                    } => {
                        let local_t = t * entity.speed + entity.phase;
                        // Trajectoire en courbe de Lissajous 3D
                        let offset_x = (local_t * 0.7).cos() * orbit_radius_x;
                        let offset_z = (local_t * 0.9).sin() * orbit_radius_z;
                        let offset_y = (local_t * freq_y).sin() * height_amplitude;

                        node.transform.translation = entity.base_translation + Vec3::new(offset_x, offset_y, offset_z);

                        // Pulsation de luminosité émissive
                        let pulse = 0.5 + 0.5 * (local_t * pulse_speed).sin();
                        let intensity = 1.5 + pulse * 2.5;
                        node.material.emission_color = [
                            entity.base_emission[0] * intensity,
                            entity.base_emission[1] * intensity,
                            entity.base_emission[2] * intensity,
                            intensity,
                        ];
                        node.update_material_uniform(queue);
                        node.update_model_uniform(queue);
                    }

                    DynamicEntityType::FloatingMonolith {
                        float_amplitude,
                        float_freq,
                        rot_speed_y,
                        rot_speed_z,
                    } => {
                        let local_t = t * entity.speed + entity.phase;
                        let bob = (local_t * float_freq).sin() * float_amplitude;
                        node.transform.translation = entity.base_translation + Vec3::new(0.0, bob, 0.0);
                        node.transform.rotation.y = entity.base_rotation.y + local_t * rot_speed_y;
                        node.transform.rotation.z = entity.base_rotation.z + (local_t * rot_speed_z).sin() * 0.08;
                        node.update_model_uniform(queue);
                    }

                    DynamicEntityType::SwayingTree {
                        sway_factor,
                        bend_axis,
                    } => {
                        let local_t = t * entity.speed + entity.phase;
                        let sway = (local_t).sin() * sway_factor * wind_str;
                        let gust = (local_t * 2.3).sin() * (sway_factor * 0.4) * wind_str;
                        let total_sway = sway + gust;

                        node.transform.rotation.x = entity.base_rotation.x + bend_axis.x * total_sway;
                        node.transform.rotation.z = entity.base_rotation.z + bend_axis.z * total_sway;
                        node.update_model_uniform(queue);
                    }

                    DynamicEntityType::PulsingFlora {
                        pulse_min,
                        pulse_max,
                        pulse_freq,
                    } => {
                        let local_t = t * entity.speed + entity.phase;
                        let s = 0.5 + 0.5 * (local_t * pulse_freq).sin();
                        let intensity = pulse_min + s * (pulse_max - pulse_min);
                        node.material.emission_color = [
                            entity.base_emission[0] * intensity,
                            entity.base_emission[1] * intensity,
                            entity.base_emission[2] * intensity,
                            intensity,
                        ];
                        node.update_material_uniform(queue);
                    }
                }
            }
        }
    }
}

/// Génère une scène complète de forêt vivante avec arbres, rochers, éléments mystiques et lucioles
pub fn generate_living_forest(
    device: &wgpu::Device,
    model_bind_group_layout: &wgpu::BindGroupLayout,
    white_texture: Arc<GpuTexture>,
    flat_normal_texture: Arc<GpuTexture>,
    wood_texture: Arc<GpuTexture>,
    start_id: usize,
) -> (Vec<SceneNode>, ForestEcosystem) {
    let mut nodes = Vec::new();
    let mut ecosystem = ForestEcosystem::new();
    let mut next_id = start_id;

    // Pseudo-RNG déterministe LCG simple pour reproductibilité
    let mut seed: u64 = 1337420;
    let mut rand_f32 = || -> f32 {
        seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        ((seed >> 32) as u32 as f32) / (u32::MAX as f32)
    };

    // 1. Sol Forêt Mousse & Sous-bois (Large Plane PBR vert profond avec rugosité)
    let ground_size = 70.0;
    let mut ground = SceneNode::new(
        device,
        model_bind_group_layout,
        white_texture.clone(),
        flat_normal_texture.clone(),
        next_id,
        PrimitiveType::Plane,
        Vec3::new(0.0, 0.0, 0.0),
    );
    ground.name = "Sol Forêt Mousse".to_string();
    ground.transform.scale = Vec3::new(ground_size * 0.1, 1.0, ground_size * 0.1);
    ground.color = [0.12, 0.28, 0.14, 1.0]; // Vert mousse profond
    ground.material.roughness = 0.85;
    ground.material.metallic = 0.0;
    ground.material.uv_tiling = [15.0, 15.0];
    nodes.push(ground);
    next_id += 1;

    // 2. Sanctuaire Central : Monolithe Ancien Flottant & Cristaux
    let mut monolith = SceneNode::new(
        device,
        model_bind_group_layout,
        wood_texture.clone(),
        flat_normal_texture.clone(),
        next_id,
        PrimitiveType::Prism,
        Vec3::new(0.0, 2.5, 0.0),
    );
    monolith.name = "Monolithe Ancien Flottant".to_string();
    monolith.transform.scale = Vec3::new(0.8, 2.4, 0.8);
    monolith.color = [0.35, 0.38, 0.42, 1.0];
    monolith.material.roughness = 0.3;
    monolith.material.metallic = 0.15;
    monolith.material.emission_color = [0.1, 0.4, 0.8, 0.8]; // Lueur runique bleue
    let monolith_id = next_id;
    nodes.push(monolith);
    next_id += 1;

    ecosystem.entities.push(ForestDynamicEntity {
        node_id: monolith_id,
        entity_type: DynamicEntityType::FloatingMonolith {
            float_amplitude: 0.35,
            float_freq: 0.8,
            rot_speed_y: 0.25,
            rot_speed_z: 0.15,
        },
        base_translation: Vec3::new(0.0, 2.5, 0.0),
        base_rotation: Vec3::ZERO,
        base_emission: [0.1, 0.4, 0.8, 0.8],
        base_color: [0.35, 0.38, 0.42, 1.0],
        phase: 0.0,
        speed: 1.0,
    });

    // 3. Cristaux et Champignons Luminescents au sol
    let crystal_colors = [
        ([0.2, 0.9, 0.6, 1.0], [0.1, 0.9, 0.5, 2.0], "Cristal Émeraude"),
        ([0.2, 0.6, 1.0, 1.0], [0.1, 0.5, 1.0, 2.2], "Cristal Saphir"),
        ([0.9, 0.3, 0.8, 1.0], [0.8, 0.2, 0.7, 2.0], "Champignon Féerique"),
        ([1.0, 0.7, 0.2, 1.0], [1.0, 0.6, 0.1, 2.5], "Cristal Ambre"),
    ];

    for (i, (col, emissive, name)) in crystal_colors.iter().enumerate() {
        let angle = (i as f32) * (std::f32::consts::PI * 2.0 / 4.0) + 0.3;
        let dist = 3.2;
        let pos = Vec3::new(angle.cos() * dist, 0.6, angle.sin() * dist);

        let mut crystal = SceneNode::new(
            device,
            model_bind_group_layout,
            white_texture.clone(),
            flat_normal_texture.clone(),
            next_id,
            if i % 2 == 0 { PrimitiveType::Pyramid } else { PrimitiveType::Cylinder },
            pos,
        );
        crystal.name = format!("{} #{}", name, i + 1);
        crystal.transform.scale = Vec3::new(0.4, 0.8 + rand_f32() * 0.4, 0.4);
        crystal.transform.rotation = Vec3::new((rand_f32() - 0.5) * 0.3, rand_f32() * 3.0, (rand_f32() - 0.5) * 0.3);
        crystal.color = *col;
        crystal.material.roughness = 0.1;
        crystal.material.metallic = 0.2;
        crystal.material.emission_color = *emissive;
        let c_id = next_id;
        nodes.push(crystal);
        next_id += 1;

        ecosystem.entities.push(ForestDynamicEntity {
            node_id: c_id,
            entity_type: DynamicEntityType::PulsingFlora {
                pulse_min: 0.8,
                pulse_max: 3.2,
                pulse_freq: 1.2 + rand_f32() * 0.8,
            },
            base_translation: pos,
            base_rotation: Vec3::ZERO,
            base_emission: *emissive,
            base_color: *col,
            phase: rand_f32() * 6.28,
            speed: 1.0,
        });
    }

    // 4. Arbres Diversifiés en Clarière et Forêt Dense (~48 arbres)
    let tree_palettes = [
        ([0.16, 0.48, 0.22, 1.0], "Sapin Émeraude"),
        ([0.22, 0.62, 0.28, 1.0], "Pin Clair"),
        ([0.10, 0.38, 0.20, 1.0], "Épicéa Sombre"),
        ([0.72, 0.58, 0.18, 1.0], "Bouleau Doré"),
        ([0.25, 0.50, 0.45, 1.0], "Arbre Boréal"),
        ([0.55, 0.25, 0.65, 1.0], "Arbre Magique"),
    ];

    let num_trees = 48;
    for i in 0..num_trees {
        let (tree_col, type_name) = tree_palettes[i % tree_palettes.len()];
        
        // Distribution en couronne pour garder la clairière centrale ouverte
        let radius = 6.0 + rand_f32() * 26.0;
        let angle = rand_f32() * std::f32::consts::PI * 2.0;
        let x = angle.cos() * radius;
        let z = angle.sin() * radius;
        let y = 0.0;

        let scale_y = 0.85 + rand_f32() * 0.9;
        let scale_xz = (0.8 + rand_f32() * 0.5) * (scale_y * 0.8);

        let rot_y = rand_f32() * std::f32::consts::PI * 2.0;
        let mut tree = SceneNode::new(
            device,
            model_bind_group_layout,
            white_texture.clone(),
            flat_normal_texture.clone(),
            next_id,
            PrimitiveType::Tree,
            Vec3::new(x, y, z),
        );
        tree.name = format!("{} #{:02}", type_name, i + 1);
        tree.transform.scale = Vec3::new(scale_xz, scale_y, scale_xz);
        tree.transform.rotation.y = rot_y;
        tree.color = tree_col;
        tree.material.roughness = 0.75;
        tree.material.metallic = 0.0;
        let tree_id = next_id;
        nodes.push(tree);
        next_id += 1;

        // Entité dynamique pour balancement du vent
        let bend_dir = Vec3::new(rand_f32() - 0.5, 0.0, rand_f32() - 0.5).normalize_or_zero();
        ecosystem.entities.push(ForestDynamicEntity {
            node_id: tree_id,
            entity_type: DynamicEntityType::SwayingTree {
                sway_factor: 0.04 + rand_f32() * 0.05,
                bend_axis: bend_dir,
            },
            base_translation: Vec3::new(x, y, z),
            base_rotation: Vec3::new(0.0, rot_y, 0.0),
            base_emission: [0.0, 0.0, 0.0, 0.0],
            base_color: tree_col,
            phase: rand_f32() * std::f32::consts::PI * 2.0,
            speed: 0.8 + rand_f32() * 0.5,
        });
    }

    // 5. Rochers Moussus et Roches Naturelles (Pyramides & Prismes taillés)
    let rock_colors = [
        [0.45, 0.44, 0.42, 1.0],
        [0.35, 0.40, 0.32, 1.0], // Teinte mousseuse
        [0.52, 0.50, 0.48, 1.0],
    ];

    for i in 0..14 {
        let radius = 4.5 + rand_f32() * 24.0;
        let angle = rand_f32() * std::f32::consts::PI * 2.0;
        let x = angle.cos() * radius;
        let z = angle.sin() * radius;

        let prim = if i % 2 == 0 { PrimitiveType::Pyramid } else { PrimitiveType::Prism };
        let mut rock = SceneNode::new(
            device,
            model_bind_group_layout,
            white_texture.clone(),
            flat_normal_texture.clone(),
            next_id,
            prim,
            Vec3::new(x, 0.4, z),
        );
        rock.name = format!("Rocher Moussu #{:02}", i + 1);
        let s_x = 0.7 + rand_f32() * 1.1;
        let s_y = 0.4 + rand_f32() * 0.6;
        let s_z = 0.7 + rand_f32() * 1.1;
        rock.transform.scale = Vec3::new(s_x, s_y, s_z);
        rock.transform.rotation = Vec3::new(rand_f32() * 0.3, rand_f32() * 3.14, rand_f32() * 0.3);
        rock.color = rock_colors[i % rock_colors.len()];
        rock.material.roughness = 0.9;
        rock.material.metallic = 0.05;
        nodes.push(rock);
        next_id += 1;
    }

    // 6. Lucioles et Esprits Volants Lumineux (Wisps) en Mouvement 3D
    let firefly_configs = [
        ("Luciole Dorée", [1.0, 0.85, 0.3, 3.5], Vec3::new(0.0, 2.0, 0.0), 4.5, 3.5, 1.2, 1.4),
        ("Esprit Saphir", [0.2, 0.7, 1.0, 4.0], Vec3::new(4.0, 2.5, -3.0), 6.0, 5.0, 1.5, 1.1),
        ("Luciole Émeraude", [0.3, 1.0, 0.5, 3.8], Vec3::new(-5.0, 1.8, 4.0), 5.5, 4.5, 1.0, 1.3),
        ("Feu Follet Améthyste", [0.85, 0.3, 1.0, 4.2], Vec3::new(2.0, 3.0, 5.0), 7.0, 6.0, 1.8, 0.9),
        ("Luciole Ruby", [1.0, 0.35, 0.3, 3.5], Vec3::new(-6.0, 2.2, -5.0), 6.5, 5.5, 1.4, 1.2),
        ("Esprit Astral Cyan", [0.1, 1.0, 0.9, 4.5], Vec3::new(7.0, 2.8, 2.0), 8.0, 7.0, 2.0, 0.8),
        ("Luciole Soleil", [1.0, 0.95, 0.5, 3.2], Vec3::new(-2.0, 1.6, -7.0), 5.0, 4.0, 1.1, 1.5),
        ("Feu Follet Zénith", [0.6, 0.9, 1.0, 4.0], Vec3::new(0.0, 4.0, 3.0), 9.0, 8.0, 2.2, 0.7),
    ];

    for (i, (name, emissive, center, rx, rz, freq, spd)) in firefly_configs.iter().enumerate() {
        let mut firefly = SceneNode::new(
            device,
            model_bind_group_layout,
            white_texture.clone(),
            flat_normal_texture.clone(),
            next_id,
            PrimitiveType::Sphere,
            *center,
        );
        firefly.name = format!("✨ {} #{:02}", name, i + 1);
        firefly.transform.scale = Vec3::splat(0.22);
        firefly.color = [emissive[0], emissive[1], emissive[2], 1.0];
        firefly.material.roughness = 0.05;
        firefly.material.emission_color = *emissive;
        let ff_id = next_id;
        nodes.push(firefly);
        next_id += 1;

        ecosystem.entities.push(ForestDynamicEntity {
            node_id: ff_id,
            entity_type: DynamicEntityType::Firefly {
                orbit_radius_x: *rx,
                orbit_radius_z: *rz,
                height_amplitude: 0.8 + rand_f32() * 0.6,
                freq_y: *freq,
                pulse_speed: 2.5 + rand_f32() * 1.5,
            },
            base_translation: *center,
            base_rotation: Vec3::ZERO,
            base_emission: *emissive,
            base_color: [emissive[0], emissive[1], emissive[2], 1.0],
            phase: rand_f32() * std::f32::consts::PI * 2.0,
            speed: *spd,
        });
    }

    (nodes, ecosystem)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_forest_ecosystem_default() {
        let eco = ForestEcosystem::new();
        assert!(eco.enabled);
        assert_eq!(eco.entities.len(), 0);
    }
}
