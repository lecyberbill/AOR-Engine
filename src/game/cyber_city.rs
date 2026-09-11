// [WFGY] Zone: SAFE | λ: 0.3 | Fallbacks: 0/None | Action: Procedural Cyberpunk Megalopolis & Holographic Checkpoint Circuit
#![allow(dead_code)]
use glam::Vec3;
use crate::scene::{MaterialUniform, PrimitiveType, Scene};

/// Données d'un anneau de checkpoint de course holographique
#[derive(Clone, Debug)]
pub struct CyberCheckpoint {
    pub position: Vec3,
    pub radius: f32,
    pub passed: bool,
}

/// Système de jeu Cyber City et gestionnaire de course Blade Runner
#[derive(Clone, Debug)]
pub struct CyberCitySystem {
    pub checkpoints: Vec<CyberCheckpoint>,
    pub current_checkpoint_idx: usize,
    pub race_time: f32,
    pub lap_completed: bool,
    pub score: u32,
}

impl Default for CyberCitySystem {
    fn default() -> Self {
        // Circuit d'anneaux de checkpoints slalomant entre les gratte-ciels
        let checkpoints = vec![
            CyberCheckpoint { position: Vec3::new(0.0, 18.0, 30.0), radius: 6.5, passed: false },
            CyberCheckpoint { position: Vec3::new(40.0, 28.0, 90.0), radius: 6.5, passed: false },
            CyberCheckpoint { position: Vec3::new(80.0, 45.0, 40.0), radius: 6.5, passed: false },
            CyberCheckpoint { position: Vec3::new(50.0, 35.0, -40.0), radius: 6.5, passed: false },
            CyberCheckpoint { position: Vec3::new(-40.0, 22.0, -80.0), radius: 6.5, passed: false },
            CyberCheckpoint { position: Vec3::new(-80.0, 38.0, -20.0), radius: 6.5, passed: false },
            CyberCheckpoint { position: Vec3::new(-50.0, 20.0, 20.0), radius: 6.5, passed: false },
        ];

        Self {
            checkpoints,
            current_checkpoint_idx: 0,
            race_time: 0.0,
            lap_completed: false,
            score: 0,
        }
    }
}

impl CyberCitySystem {
    pub fn new() -> Self {
        Self::default()
    }

    /// Met à jour la course et vérifie la traversée des anneaux par le Spinner
    pub fn update(&mut self, dt: f32, spinner_pos: Vec3) -> Option<usize> {
        self.race_time += dt;

        if self.current_checkpoint_idx < self.checkpoints.len() {
            let cp = &mut self.checkpoints[self.current_checkpoint_idx];
            let dist = (spinner_pos - cp.position).length();

            if dist <= cp.radius {
                cp.passed = true;
                let passed_idx = self.current_checkpoint_idx;
                self.current_checkpoint_idx += 1;
                self.score += 500;

                if self.current_checkpoint_idx >= self.checkpoints.len() {
                    self.lap_completed = true;
                }
                return Some(passed_idx);
            }
        }
        None
    }
}

/// Génère une mégapole cyberpunk procédurale monumentale dans la scène
pub fn generate_cyber_megalopolis(scene: &mut Scene, device: &wgpu::Device) {
    scene.clear();

    // 1. Sol / Asphalt détrempé hautement réfléchissant
    let ground_idx = scene.add_node_at(device, PrimitiveType::Plane, Vec3::new(0.0, 0.0, 0.0));
    if let Some(ground) = scene.nodes.get_mut(ground_idx) {
        ground.name = "Cyber_Highway_Wet_Asphalt".to_string();
        ground.transform.scale = Vec3::new(300.0, 1.0, 300.0);
        ground.material = MaterialUniform {
            roughness: 0.08,    // Miroir d'eau réfléchissant
            metallic: 0.85,
            uv_tiling: [20.0, 20.0],
            uv_offset: [0.0, 0.0],
            ior: 1.5,
            transmission: 0.0,
            emission_color: [0.01, 0.02, 0.05, 1.0],
            use_normal_map: 0,
            clearcoat: 0.0,
            clearcoat_roughness: 0.05,
            subsurface: 0.0,
        };
    }

    // 2. Grille de Mégastructures et Gratte-Ciels (Hauteurs de 40m à 160m)
    let grid_range: i32 = 4;
    let spacing = 45.0;

    let mut seed: u32 = 0x1337c0de;
    let mut rand_f32 = || -> f32 {
        seed = seed.wrapping_mul(1664525).wrapping_add(1013904223);
        (seed >> 8) as f32 / 16777216.0
    };

    for gx in -grid_range..=grid_range {
        for gz in -grid_range..=grid_range {
            // Laisser les artères principales dégagées pour le vol
            if gx.abs() <= 1 && gz.abs() <= 1 {
                continue;
            }

            let height = 40.0 + rand_f32() * 110.0;
            let width = 16.0 + rand_f32() * 12.0;
            let depth = 16.0 + rand_f32() * 12.0;

            let x = gx as f32 * spacing + (rand_f32() - 0.5) * 8.0;
            let z = gz as f32 * spacing + (rand_f32() - 0.5) * 8.0;
            let y = height * 0.5;

            let building_idx = scene.add_node_at(device, PrimitiveType::Cube, Vec3::new(x, y, z));
            if let Some(b) = scene.nodes.get_mut(building_idx) {
                b.name = format!("Skyscraper_{}_{}", gx, gz);
                b.transform.scale = Vec3::new(width, height, depth);

                let is_neon_tower = rand_f32() > 0.65;
                if is_neon_tower {
                    // Tour avec panneaux et enseignes émissives (Cyan ou Magenta)
                    let is_cyan = rand_f32() > 0.5;
                    let glow = if is_cyan {
                        [0.0, 1.8, 3.5, 1.0] // Cyan Néon intense
                    } else {
                        [3.5, 0.0, 2.2, 1.0] // Magenta Néon intense
                    };

                    b.material = MaterialUniform {
                        roughness: 0.25,
                        metallic: 0.9,
                        uv_tiling: [1.0, 1.0],
                        uv_offset: [0.0, 0.0],
                        ior: 1.6,
                        transmission: 0.0,
                        emission_color: glow,
                        use_normal_map: 0,
                        clearcoat: 0.8,
                        clearcoat_roughness: 0.05,
                        subsurface: 0.0,
                    };
                } else {
                    // Façade vitrée sombre PBR
                    b.material = MaterialUniform {
                        roughness: 0.15,
                        metallic: 0.95,
                        uv_tiling: [1.0, 1.0],
                        uv_offset: [0.0, 0.0],
                        ior: 1.55,
                        transmission: 0.0,
                        emission_color: [0.05, 0.1, 0.18, 1.0],
                        use_normal_map: 0,
                        clearcoat: 0.5,
                        clearcoat_roughness: 0.05,
                        subsurface: 0.0,
                    };
                }
            }
        }
    }

    // 3. Anneaux de Checkpoints Lumineux (Torus ou Cylindres émissifs)
    let city_sys = CyberCitySystem::default();
    for (idx, cp) in city_sys.checkpoints.iter().enumerate() {
        let cp_node_idx = scene.add_node_at(device, PrimitiveType::Cylinder, cp.position);
        if let Some(node) = scene.nodes.get_mut(cp_node_idx) {
            node.name = format!("Checkpoint_Gate_{}", idx + 1);
            node.transform.scale = Vec3::new(cp.radius * 1.8, 0.4, cp.radius * 1.8);
            node.transform.rotation = Vec3::new(std::f32::consts::FRAC_PI_2, 0.0, 0.0);
            node.material = MaterialUniform {
                roughness: 0.1,
                metallic: 0.1,
                uv_tiling: [1.0, 1.0],
                uv_offset: [0.0, 0.0],
                ior: 1.4,
                transmission: 0.5,
                emission_color: [0.2, 3.5, 1.2, 1.0], // Vert Holographique émissif
                use_normal_map: 0,
                clearcoat: 0.0,
                clearcoat_roughness: 0.05,
                subsurface: 0.0,
            };
        }
    }
}
