// [WFGY] Zone: SAFE | λ: 0.3 | Fallbacks: 0/None | Action: Scene module and Scene hierarchy manager with PBR Material System and Normal Mapping
#![allow(unused_imports, dead_code)]
pub mod camera;
pub mod mesh;
pub mod node;
pub mod raycast;
pub mod loader;
pub mod persistence;
pub mod bundle;
pub mod bvh;
pub mod instance;
pub mod forest;
pub mod cove;
pub mod culling;

pub use camera::{Camera, CameraMode, CameraUniform};
pub use mesh::{compute_tangents, generate_grid_plane, GpuMesh, PrimitiveType, Vertex};
pub use node::{MaterialUniform, ModelUniform, SceneNode, Transform};
pub use instance::{GpuInstanceBuffer, Instance, InstanceRaw};
pub use raycast::{screen_to_ray, world_to_screen, Aabb, Ray};
pub use culling::{Frustum, Plane};
pub use loader::{generate_column_data, generate_cylinder_data, generate_lowpoly_tree_data, generate_sphere_data, generate_torus_data, load_obj_file, load_obj_from_str};
pub use persistence::{WorldData, NodeData, save_world_to_file, load_world_from_file, serialize_world, deserialize_world};
pub use bundle::{AorHeader, AorSceneManifest, BufferView, BufferViewKind, LoadedAorBundle, TransformRaw, load_aor_bundle, save_aor_bundle, update_node_transform_in_place};
pub use bvh::{BvhBuilder, GpuBvhNode, GpuMaterial, GpuTriangle};
pub use forest::{DynamicEntityType, ForestDynamicEntity, ForestEcosystem, generate_living_forest};
pub use cove::{CoveDynamicEntity, CoveEcosystem, CoveEntityType, generate_night_cove};


use crate::gpu::GpuTexture;
use glam::Vec3;
use std::sync::Arc;

pub struct Scene {
    pub nodes: Vec<SceneNode>,
    pub selected_node_idx: usize,
    pub is_edit_mode: bool,
    pub next_node_id: usize,
    pub model_bind_group_layout: wgpu::BindGroupLayout,
    // Textures Diffuses Standards & HD
    pub white_texture: Arc<GpuTexture>,
    pub checker_texture: Arc<GpuTexture>,
    pub grid_texture: Arc<GpuTexture>,
    pub brick_texture: Arc<GpuTexture>,
    pub wood_texture: Arc<GpuTexture>,
    pub marble_hd_texture: Arc<GpuTexture>,
    pub carbon_hd_texture: Arc<GpuTexture>,
    // Normal Maps (Relief)
    pub flat_normal_texture: Arc<GpuTexture>,
    pub brick_normal_texture: Arc<GpuTexture>,
    pub wood_normal_texture: Arc<GpuTexture>,
}

impl Scene {
    pub fn new(device: &wgpu::Device, queue: &wgpu::Queue) -> Self {
        let model_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Model & Material Bind Group Layout"),
            entries: &[
                // binding 0: ModelUniform
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                // binding 1: MaterialUniform
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                // binding 2: Diffuse Texture
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                // binding 3: Normal Map Texture
                wgpu::BindGroupLayoutEntry {
                    binding: 3,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                // binding 4: Sampler
                wgpu::BindGroupLayoutEntry {
                    binding: 4,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });

        // Génération des textures procédurales intégrées avec chaînes de Mipmaps
        let white_texture = Arc::new(GpuTexture::create_white_texture(device, queue));
        let checker_texture = Arc::new(GpuTexture::create_checkered_texture(device, queue, 256, 8));
        let grid_texture = Arc::new(GpuTexture::create_grid_texture(device, queue, 256));
        let brick_texture = Arc::new(GpuTexture::create_brick_texture(device, queue, 256));
        let wood_texture = Arc::new(GpuTexture::create_wood_texture(device, queue, 256));
        let marble_hd_texture = Arc::new(GpuTexture::create_marble_hd_texture(device, queue, 1024));
        let carbon_hd_texture = Arc::new(GpuTexture::create_carbon_grid_hd_texture(device, queue, 1024));

        // Normal Maps
        let flat_normal_texture = Arc::new(GpuTexture::create_flat_normal_map(device, queue));
        let brick_normal_texture = Arc::new(GpuTexture::create_brick_normal_map(device, queue, 256));
        let wood_normal_texture = Arc::new(GpuTexture::create_wood_normal_map(device, queue, 256));

        let mut scene = Scene {
            nodes: Vec::new(),
            selected_node_idx: 0,
            is_edit_mode: false,
            next_node_id: 0,
            model_bind_group_layout,
            white_texture: white_texture.clone(),
            checker_texture,
            grid_texture,
            brick_texture,
            wood_texture,
            marble_hd_texture,
            carbon_hd_texture,
            flat_normal_texture: flat_normal_texture.clone(),
            brick_normal_texture,
            wood_normal_texture,
        };

        // Ajouter un cube par défaut
        scene.add_node(device, PrimitiveType::Cube);
        scene
    }

    pub fn add_node(&mut self, device: &wgpu::Device, prim: PrimitiveType) {
        let offset = if self.nodes.is_empty() {
            Vec3::ZERO
        } else {
            let count = self.nodes.len() as f32;
            Vec3::new(count * 1.5, 0.0, count * 0.5)
        };
        self.add_node_at(device, prim, offset);
    }

    /// Ajoute une primitive à une position 3D spécifique
    pub fn add_node_at(&mut self, device: &wgpu::Device, prim: PrimitiveType, pos: Vec3) -> usize {
        let node = SceneNode::new(
            device,
            &self.model_bind_group_layout,
            self.white_texture.clone(),
            self.flat_normal_texture.clone(),
            self.next_node_id,
            prim,
            pos,
        );
        self.nodes.push(node);
        let idx = self.nodes.len() - 1;
        self.selected_node_idx = idx;
        self.next_node_id += 1;
        idx
    }

    /// Génère une pile ou tour de cubes destructibles pour tester les collisions et explosions
    pub fn spawn_cube_tower(&mut self, device: &wgpu::Device, base_pos: Vec3, width: usize, height: usize) {
        let spacing = 1.1;
        for h in 0..height {
            let current_w = width.saturating_sub(h).max(1);
            let offset_x = -(current_w as f32 - 1.0) * 0.5 * spacing;
            let offset_z = -(current_w as f32 - 1.0) * 0.5 * spacing;

            for x in 0..current_w {
                for z in 0..current_w {
                    let pos = base_pos + Vec3::new(
                        offset_x + x as f32 * spacing,
                        0.55 + h as f32 * spacing,
                        offset_z + z as f32 * spacing,
                    );
                    self.add_node_at(device, PrimitiveType::Cube, pos);
                }
            }
        }
    }

    /// Ajoute un maillage 3D personnalisé à la scène
    #[allow(dead_code)]
    pub fn add_custom_mesh(&mut self, device: &wgpu::Device, name: &str, vertices: Vec<Vertex>, indices: Vec<u32>) {
        let offset = if self.nodes.is_empty() {
            Vec3::ZERO
        } else {
            let count = self.nodes.len() as f32;
            Vec3::new(count * 1.5, 0.0, count * 0.5)
        };

        let node = SceneNode::from_custom_mesh(
            device,
            &self.model_bind_group_layout,
            self.white_texture.clone(),
            self.flat_normal_texture.clone(),
            self.next_node_id,
            name,
            vertices,
            indices,
            offset,
        );
        self.nodes.push(node);
        self.selected_node_idx = self.nodes.len() - 1;
        self.next_node_id += 1;
    }

    /// Fait apparaître un personnage articulé humanoïde avec squelette et animations
    pub fn spawn_rigged_humanoid(&mut self, device: &wgpu::Device, position: Vec3) -> usize {
        let rigged = crate::animation::RiggedModel::create_humanoid();
        let skinned_vertices = rigged.compute_skinned_vertices();
        let indices = rigged.indices.clone();

        let mut node = SceneNode::from_custom_mesh(
            device,
            &self.model_bind_group_layout,
            self.white_texture.clone(),
            self.flat_normal_texture.clone(),
            self.next_node_id,
            "Mannequin Cyber Riggé",
            skinned_vertices,
            indices,
            position,
        );
        node.rigged_model = Some(rigged);
        node.material.roughness = 0.3;
        node.material.metallic = 0.2;

        let id = self.nodes.len();
        self.nodes.push(node);
        self.selected_node_idx = id;
        self.next_node_id += 1;
        id
    }

    /// Charge un fichier 3D (glTF, GLB, OBJ) depuis le disque avec ses textures PBR et l'ajoute à la scène

    pub fn load_model_from_path(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        path: &std::path::Path,
    ) -> Result<(), String> {
        let loaded = loader::load_3d_model_file_full(path)?;

        let (albedo_gpu, albedo_name) = if let Some((w, h, rgba, name)) = loaded.albedo_texture {
            let gpu_tex = GpuTexture::from_rgba8(device, queue, &rgba, w, h, Some(&name));
            (std::sync::Arc::new(gpu_tex), Some(name))
        } else {
            (self.white_texture.clone(), None)
        };

        let (normal_gpu, normal_name) = if let Some((w, h, rgba, name)) = loaded.normal_texture {
            let gpu_tex = GpuTexture::from_rgba8(device, queue, &rgba, w, h, Some(&name));
            (std::sync::Arc::new(gpu_tex), Some(name))
        } else {
            (self.flat_normal_texture.clone(), None)
        };

        let mut material = MaterialUniform::default();
        material.roughness = loaded.roughness;
        material.metallic = loaded.metallic;
        material.transmission = loaded.transmission;
        material.ior = loaded.ior;
        material.emission_color = loaded.emission_color;
        material.use_normal_map = if normal_name.is_some() { 1 } else { 0 };

        let offset = if self.nodes.is_empty() {
            Vec3::ZERO
        } else {
            let count = self.nodes.len() as f32;
            Vec3::new(count * 1.5, 0.0, count * 0.5)
        };

        let node = SceneNode::from_custom_mesh_full(
            device,
            &self.model_bind_group_layout,
            albedo_gpu,
            albedo_name,
            normal_gpu,
            normal_name,
            Some(material),
            loaded.base_color,
            self.next_node_id,
            &loaded.name,
            loaded.vertices,
            loaded.indices,
            offset,
        );

        self.nodes.push(node);
        self.selected_node_idx = self.nodes.len() - 1;
        self.next_node_id += 1;
        Ok(())
    }

    /// Charge un fichier OBJ depuis le disque et l'ajoute à la scène (rétrocompatibilité)
    #[allow(dead_code)]
    pub fn load_obj_from_path(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, path: &std::path::Path) -> Result<(), String> {
        self.load_model_from_path(device, queue, path)
    }

    /// Duplique le nœud actuellement sélectionné
    pub fn duplicate_selected_node(&mut self, device: &wgpu::Device) {
        if self.nodes.is_empty() || self.selected_node_idx >= self.nodes.len() {
            return;
        }

        let selected = &self.nodes[self.selected_node_idx];
        let new_translation = selected.transform.translation + Vec3::new(1.0, 0.0, 0.0);
        let new_node = selected.clone_with_new_id(
            device,
            &self.model_bind_group_layout,
            self.next_node_id,
            new_translation,
        );

        self.nodes.push(new_node);
        self.selected_node_idx = self.nodes.len() - 1;
        self.next_node_id += 1;
    }

    /// Trouve l'objet le plus proche intersecté par un rayon 3D
    pub fn pick_node(&self, ray: &Ray) -> Option<usize> {
        let mut closest_hit: Option<(usize, f32)> = None;

        for (idx, node) in self.nodes.iter().enumerate() {
            let local_aabb = node.local_aabb();
            let model_matrix = node.transform.matrix();

            if let Some(t) = ray.intersect_obb(&local_aabb, model_matrix) {
                if t > 0.0 {
                    match closest_hit {
                        Some((_, closest_t)) if t < closest_t => {
                            closest_hit = Some((idx, t));
                        }
                        None => {
                            closest_hit = Some((idx, t));
                        }
                        _ => {}
                    }
                }
            }
        }

        closest_hit.map(|(idx, _)| idx)
    }

    /// Calcule récursivement la matrice monde globale d'un nœud en prenant en compte toute la chaîne parentale
    pub fn compute_world_matrix(&self, node_idx: usize) -> glam::Mat4 {
        if node_idx >= self.nodes.len() {
            return glam::Mat4::IDENTITY;
        }

        let node = &self.nodes[node_idx];
        let local_mat = node.transform.matrix();

        if let Some(parent_idx) = node.parent_idx {
            if parent_idx < self.nodes.len() && parent_idx != node_idx {
                return self.compute_world_matrix(parent_idx) * local_mat;
            }
        }

        local_mat
    }

    /// Attache un nœud enfant à un nœud parent
    pub fn parent_node(&mut self, child_idx: usize, parent_idx: usize) {
        if child_idx == parent_idx || child_idx >= self.nodes.len() || parent_idx >= self.nodes.len() {
            return;
        }

        // Détacher du précédent parent si existant
        self.unparent_node(child_idx);

        self.nodes[child_idx].parent_idx = Some(parent_idx);
        if !self.nodes[parent_idx].children_indices.contains(&child_idx) {
            self.nodes[parent_idx].children_indices.push(child_idx);
        }
    }

    /// Détache un nœud enfant de son parent
    pub fn unparent_node(&mut self, child_idx: usize) {
        if child_idx >= self.nodes.len() {
            return;
        }

        if let Some(old_parent) = self.nodes[child_idx].parent_idx {
            if old_parent < self.nodes.len() {
                self.nodes[old_parent].children_indices.retain(|&idx| idx != child_idx);
            }
            self.nodes[child_idx].parent_idx = None;
        }
    }

    /// Met à jour les matrices mondes GPU de tous les nœuds de la scène avec propagation parentale
    pub fn update_hierarchy_gpu(&self, queue: &wgpu::Queue) {
        for (idx, node) in self.nodes.iter().enumerate() {
            let world_mat = self.compute_world_matrix(idx);
            let uniform = ModelUniform {
                model: world_mat.to_cols_array_2d(),
                color: node.color,
                selected: if idx == self.selected_node_idx { 1 } else { 0 },
                _pad0: 0,
                _pad1: 0,
                _pad2: 0,
            };
            node.model_uniform.update(queue, &uniform);
            node.material_uniform.update(queue, &node.material);
        }
    }

    pub fn remove_selected_node(&mut self) {
        if !self.nodes.is_empty() && self.selected_node_idx < self.nodes.len() {
            let removed_idx = self.selected_node_idx;
            self.unparent_node(removed_idx);

            // Détacher tous les enfants de ce nœud
            let children_to_detach: Vec<usize> = self.nodes[removed_idx].children_indices.clone();
            for child in children_to_detach {
                if child < self.nodes.len() {
                    self.nodes[child].parent_idx = None;
                }
            }

            self.nodes.remove(removed_idx);


            // Corriger les index dans les parent_idx et children_indices après décalage du vecteur
            for node in &mut self.nodes {
                if let Some(p) = node.parent_idx {
                    if p > removed_idx {
                        node.parent_idx = Some(p - 1);
                    }
                }
                node.children_indices = node
                    .children_indices
                    .iter()
                    .filter_map(|&c| {
                        if c == removed_idx {
                            None
                        } else if c > removed_idx {
                            Some(c - 1)
                        } else {
                            Some(c)
                        }
                    })
                    .collect();
            }

            if self.selected_node_idx >= self.nodes.len() && !self.nodes.is_empty() {
                self.selected_node_idx = self.nodes.len() - 1;
            } else if self.nodes.is_empty() {
                self.selected_node_idx = 0;
            }
        }
    }


    pub fn select_next_node(&mut self) {
        if !self.nodes.is_empty() {
            self.selected_node_idx = (self.selected_node_idx + 1) % self.nodes.len();
        }
    }

    pub fn clear(&mut self) {
        self.nodes.clear();
        self.selected_node_idx = 0;
        self.next_node_id = 0;
    }

    /// Charge une scène de démonstration Studio / Cycles complète avec sol, verre, miroirs, or, et lumières émissives
    pub fn load_studio_showcase(&mut self, device: &wgpu::Device) {
        self.clear();

        // 1. Sol studio réflectif gris neutre
        let mut floor = SceneNode::new(
            device,
            &self.model_bind_group_layout,
            self.white_texture.clone(),
            self.flat_normal_texture.clone(),
            self.next_node_id,
            PrimitiveType::Plane,
            Vec3::new(0.0, 0.0, 0.0),
        );
        floor.name = "Sol Studio".to_string();
        floor.transform.scale = Vec3::new(3.0, 1.0, 3.0);
        floor.material.roughness = 0.25;
        floor.material.metallic = 0.0;
        floor.color = [0.45, 0.47, 0.52, 1.0];
        self.nodes.push(floor);
        self.next_node_id += 1;

        // 2. Sphère en Verre / Cristal (Transmission & Réfraction IOR 1.52)
        let mut glass_sphere = SceneNode::new(
            device,
            &self.model_bind_group_layout,
            self.white_texture.clone(),
            self.flat_normal_texture.clone(),
            self.next_node_id,
            PrimitiveType::Sphere,
            Vec3::new(-2.2, 1.0, 0.0),
        );
        glass_sphere.name = "Sphère Verre Cristal".to_string();
        glass_sphere.material.transmission = 1.0;
        glass_sphere.material.ior = 1.52;
        glass_sphere.material.roughness = 0.02;
        glass_sphere.material.metallic = 0.0;
        glass_sphere.color = [0.95, 0.98, 1.0, 1.0];
        self.nodes.push(glass_sphere);
        self.next_node_id += 1;

        // 3. Sphère Miroir Chrome (100% Spéculaire)
        let mut mirror_sphere = SceneNode::new(
            device,
            &self.model_bind_group_layout,
            self.white_texture.clone(),
            self.flat_normal_texture.clone(),
            self.next_node_id,
            PrimitiveType::Sphere,
            Vec3::new(0.0, 1.0, 0.0),
        );
        mirror_sphere.name = "Sphère Chrome Miroir".to_string();
        mirror_sphere.material.metallic = 1.0;
        mirror_sphere.material.roughness = 0.03;
        mirror_sphere.color = [0.95, 0.95, 0.95, 1.0];
        self.nodes.push(mirror_sphere);
        self.next_node_id += 1;

        // 4. Sphère Or Pur
        let mut gold_sphere = SceneNode::new(
            device,
            &self.model_bind_group_layout,
            self.white_texture.clone(),
            self.flat_normal_texture.clone(),
            self.next_node_id,
            PrimitiveType::Sphere,
            Vec3::new(2.2, 1.0, 0.0),
        );
        gold_sphere.name = "Sphère Or Pur".to_string();
        gold_sphere.material.metallic = 0.95;
        gold_sphere.material.roughness = 0.15;
        gold_sphere.color = [1.0, 0.78, 0.28, 1.0];
        self.nodes.push(gold_sphere);
        self.next_node_id += 1;

        // 5. Lumière Émissive Néon Rose (Panneau gauche)
        let mut neon_pink = SceneNode::new(
            device,
            &self.model_bind_group_layout,
            self.white_texture.clone(),
            self.flat_normal_texture.clone(),
            self.next_node_id,
            PrimitiveType::Cube,
            Vec3::new(-3.5, 2.5, -2.0),
        );
        neon_pink.name = "Lumière Néon Rose".to_string();
        neon_pink.transform.scale = Vec3::new(0.3, 2.0, 1.5);
        neon_pink.material.emission_color = [1.0, 0.15, 0.6, 2.2];
        neon_pink.color = [1.0, 0.2, 0.6, 1.0];
        self.nodes.push(neon_pink);
        self.next_node_id += 1;

        // 6. Lumière Émissive Néon Cyan (Panneau droit)
        let mut neon_cyan = SceneNode::new(
            device,
            &self.model_bind_group_layout,
            self.white_texture.clone(),
            self.flat_normal_texture.clone(),
            self.next_node_id,
            PrimitiveType::Cube,
            Vec3::new(3.5, 2.5, -2.0),
        );
        neon_cyan.name = "Lumière Néon Cyan".to_string();
        neon_cyan.transform.scale = Vec3::new(0.3, 2.0, 1.5);
        neon_cyan.material.emission_color = [0.1, 0.8, 1.0, 2.2];
        neon_cyan.color = [0.1, 0.8, 1.0, 1.0];
        self.nodes.push(neon_cyan);
        self.next_node_id += 1;

        // 7. Tore Émeraude Jade
        let mut jade_torus = SceneNode::new(
            device,
            &self.model_bind_group_layout,
            self.white_texture.clone(),
            self.flat_normal_texture.clone(),
            self.next_node_id,
            PrimitiveType::Torus,
            Vec3::new(0.0, 0.4, 2.2),
        );
        jade_torus.name = "Tore Jade Émeraude".to_string();
        jade_torus.transform.scale = Vec3::new(0.9, 0.9, 0.9);
        jade_torus.material.roughness = 0.1;
        jade_torus.color = [0.15, 0.85, 0.55, 1.0];
        self.nodes.push(jade_torus);
        self.next_node_id += 1;

        self.selected_node_idx = 1; // Sélectionne la sphère en verre
    }

    /// Génère un banc de test de charge (Stress Test) avec une grille massive d'objets aux matériaux physiques variés et multiples lumières
    pub fn load_stress_test_grid(&mut self, device: &wgpu::Device, count_per_side: usize) {
        self.clear();

        let grid_size = count_per_side.max(3);
        let spacing = 2.4;
        let half_span = ((grid_size - 1) as f32) * spacing * 0.5;

        // 1. Sol étendu réflectif
        let mut floor = SceneNode::new(
            device,
            &self.model_bind_group_layout,
            self.white_texture.clone(),
            self.flat_normal_texture.clone(),
            self.next_node_id,
            PrimitiveType::Plane,
            Vec3::new(0.0, 0.0, 0.0),
        );
        floor.name = "Sol Arena Stress Test".to_string();
        floor.transform.scale = Vec3::new(half_span * 0.3 + 3.0, 1.0, half_span * 0.3 + 3.0);
        floor.material.roughness = 0.22;
        floor.material.metallic = 0.05;
        floor.color = [0.42, 0.45, 0.50, 1.0];
        self.nodes.push(floor);
        self.next_node_id += 1;

        // 2. Grille d'objets variés
        for row in 0..grid_size {
            for col in 0..grid_size {
                let x = -half_span + (col as f32) * spacing;
                let z = -half_span + (row as f32) * spacing;
                let pat = (row * grid_size + col) % 8;

                let (prim, name, color, rough, metal, trans, ior) = match pat {
                    0 => (PrimitiveType::Sphere, "Sphère Verre", [0.95, 0.98, 1.0, 1.0], 0.02, 0.0, 1.0, 1.52),
                    1 => (PrimitiveType::Sphere, "Sphère Chrome", [0.95, 0.95, 0.95, 1.0], 0.02, 1.0, 0.0, 1.5),
                    2 => (PrimitiveType::Sphere, "Sphère Or", [1.0, 0.78, 0.28, 1.0], 0.12, 0.95, 0.0, 1.5),
                    3 => (PrimitiveType::Torus, "Tore Jade", [0.15, 0.85, 0.55, 1.0], 0.08, 0.0, 0.0, 1.5),
                    4 => (PrimitiveType::Cube, "Cube Cuivre", [0.95, 0.55, 0.35, 1.0], 0.2, 0.9, 0.0, 1.5),
                    5 => (PrimitiveType::Cylinder, "Cylindre Saphir", [0.2, 0.4, 0.95, 1.0], 0.05, 0.0, 0.8, 1.77),
                    6 => (PrimitiveType::Tree, "Arbre", [0.2, 0.8, 0.3, 1.0], 0.7, 0.0, 0.0, 1.5),
                    _ => (PrimitiveType::Column, "Colonne Marbre", [0.9, 0.88, 0.85, 1.0], 0.35, 0.0, 0.0, 1.5),
                };

                let mut node = SceneNode::new(
                    device,
                    &self.model_bind_group_layout,
                    self.white_texture.clone(),
                    self.flat_normal_texture.clone(),
                    self.next_node_id,
                    prim,
                    Vec3::new(x, 1.0, z),
                );
                node.name = format!("{} #{}_{}", name, row, col);
                node.color = color;
                node.material.roughness = rough;
                node.material.metallic = metal;
                node.material.transmission = trans;
                node.material.ior = ior;
                self.nodes.push(node);
                self.next_node_id += 1;
            }
        }

        // 3. Multiples Sources de Lumière Émissives Colorées en Périphérie
        let lights = [
            ("Lumière Rouge Rubis", Vec3::new(-half_span - 3.0, 3.5, -half_span - 3.0), [1.0, 0.1, 0.1, 3.0]),
            ("Lumière Bleu Saphir", Vec3::new( half_span + 3.0, 3.5, -half_span - 3.0), [0.1, 0.3, 1.0, 3.0]),
            ("Lumière Vert Émeraude", Vec3::new(-half_span - 3.0, 3.5,  half_span + 3.0), [0.1, 1.0, 0.3, 3.0]),
            ("Lumière Or Ambre", Vec3::new( half_span + 3.0, 3.5,  half_span + 3.0), [1.0, 0.75, 0.1, 3.0]),
            ("Lumière Néon Rose", Vec3::new(0.0, 4.0, -half_span - 4.0), [1.0, 0.15, 0.8, 3.5]),
            ("Lumière Néon Cyan", Vec3::new(0.0, 4.0,  half_span + 4.0), [0.1, 0.9, 1.0, 3.5]),
            ("Lumière Violet Mystique", Vec3::new(-half_span - 4.0, 4.0, 0.0), [0.75, 0.1, 1.0, 3.5]),
            ("Lumière Zénithale Pure", Vec3::new(0.0, 7.0, 0.0), [1.0, 0.98, 0.95, 4.0]),
        ];

        for (name, pos, emissive) in lights {
            let mut light_node = SceneNode::new(
                device,
                &self.model_bind_group_layout,
                self.white_texture.clone(),
                self.flat_normal_texture.clone(),
                self.next_node_id,
                PrimitiveType::Sphere,
                pos,
            );
            light_node.name = name.to_string();
            light_node.transform.scale = Vec3::new(0.8, 0.8, 0.8);
            light_node.material.emission_color = emissive;
            light_node.color = [emissive[0], emissive[1], emissive[2], 1.0];
            self.nodes.push(light_node);
            self.next_node_id += 1;
        }

        self.selected_node_idx = 1;
    }

    /// Charge un écosystème complet de Forêt Vivante avec arbres, rochers, éléments mystiques et lucioles animées
    pub fn load_living_forest(&mut self, device: &wgpu::Device) -> ForestEcosystem {
        self.nodes.clear();
        self.next_node_id = 1;
        let (nodes, ecosystem) = generate_living_forest(
            device,
            &self.model_bind_group_layout,
            self.white_texture.clone(),
            self.flat_normal_texture.clone(),
            self.wood_texture.clone(),
            self.next_node_id,
        );
        let count = nodes.len();
        self.nodes = nodes;
        self.next_node_id += count;
        self.selected_node_idx = 1; // Sélectionner le Monolithe ou premier arbre
        ecosystem
    }

    /// Charge la scène photo-réaliste "Crique Nocturne & Grotte aux Lanternes" avec eau dynamique et pontons en bois
    pub fn load_night_cove(&mut self, device: &wgpu::Device) -> CoveEcosystem {
        self.nodes.clear();
        self.next_node_id = 1;
        let (nodes, ecosystem) = generate_night_cove(
            device,
            &self.model_bind_group_layout,
            self.white_texture.clone(),
            self.flat_normal_texture.clone(),
            self.wood_texture.clone(),
            self.wood_normal_texture.clone(),
            self.next_node_id,
        );
        let count = nodes.len();
        self.nodes = nodes;
        self.next_node_id += count;
        self.selected_node_idx = 1; // Sélectionner le ponton central
        ecosystem
    }

    /// Sauvegarde la scène entière vers un fichier .world
    pub fn save_world(&self, path: &std::path::Path, light_pos: Vec3) -> Result<(), String> {
        persistence::save_world_to_file(path, &self.nodes, light_pos)
    }

    /// Sauvegarde la scène entière vers un paquet industriel haute performance (.aor / .aorbundle)
    pub fn save_aor(&self, path: &std::path::Path, light_pos: Vec3) -> Result<(), String> {
        bundle::save_aor_bundle(path, &self.nodes, light_pos)
    }

    /// Charge une scène entière depuis un paquet industriel (.aor) avec validation de sécurité 6 niveaux
    pub fn load_aor(&mut self, device: &wgpu::Device, path: &std::path::Path) -> Result<Vec3, String> {
        let loaded = bundle::load_aor_bundle(
            device,
            &self.model_bind_group_layout,
            self.white_texture.clone(),
            self.flat_normal_texture.clone(),
            path,
            true,
        )?;

        let mut nodes = loaded.nodes;
        // Re-lier automatiquement les textures présélectionnées ou nommées
        for node in &mut nodes {
            if let Some(ref tname) = node.texture_name {
                let tex = match tname.as_str() {
                    "Damier UV" | "checkered" => self.checker_texture.clone(),
                    "Grille Métrique" | "grid" => self.grid_texture.clone(),
                    "Briques Rouges" | "brick" => self.brick_texture.clone(),
                    "Bois / Parquet" | "wood" => self.wood_texture.clone(),
                    "Marbre Carrare HD" | "marble" => self.marble_hd_texture.clone(),
                    "Fibre Carbone HD" | "carbon" => self.carbon_hd_texture.clone(),
                    _ => self.white_texture.clone(),
                };
                node.texture = tex;
            }

            if let Some(ref nname) = node.normal_texture_name {
                let ntex = match nname.as_str() {
                    "Briques Relief" | "brick_normal" => self.brick_normal_texture.clone(),
                    "Bois Relief" | "wood_normal" => self.wood_normal_texture.clone(),
                    _ => self.flat_normal_texture.clone(),
                };
                node.normal_texture = ntex;
            }

            node.recreate_bind_group(device, &self.model_bind_group_layout);
        }

        let max_id = nodes.iter().map(|n| n.id).max().unwrap_or(0);
        self.nodes = nodes;
        self.next_node_id = max_id + 1;
        self.selected_node_idx = if self.nodes.is_empty() { 0 } else { 0 };
        Ok(loaded.light_position)
    }

    /// Charge une scène entière depuis un fichier .world en remplaçant la scène actuelle
    pub fn load_world(&mut self, device: &wgpu::Device, path: &std::path::Path) -> Result<Vec3, String> {
        let (mut nodes, light_pos) = persistence::load_world_from_file(
            device,
            &self.model_bind_group_layout,
            self.white_texture.clone(),
            self.flat_normal_texture.clone(),
            path,
        )?;

        // Re-lier automatiquement les textures présélectionnées ou nommées
        for node in &mut nodes {
            if let Some(ref tname) = node.texture_name {
                let tex = match tname.as_str() {
                    "Damier UV" | "checkered" => self.checker_texture.clone(),
                    "Grille Métrique" | "grid" => self.grid_texture.clone(),
                    "Briques Rouges" | "brick" => self.brick_texture.clone(),
                    "Bois / Parquet" | "wood" => self.wood_texture.clone(),
                    "Marbre Carrare HD" | "marble" => self.marble_hd_texture.clone(),
                    "Fibre Carbone HD" | "carbon" => self.carbon_hd_texture.clone(),
                    _ => self.white_texture.clone(),
                };
                node.texture = tex;
            }

            if let Some(ref nname) = node.normal_texture_name {
                let ntex = match nname.as_str() {
                    "Briques Relief" | "brick_normal" => self.brick_normal_texture.clone(),
                    "Bois Relief" | "wood_normal" => self.wood_normal_texture.clone(),
                    _ => self.flat_normal_texture.clone(),
                };
                node.normal_texture = ntex;
            }

            node.recreate_bind_group(device, &self.model_bind_group_layout);
        }

        let max_id = nodes.iter().map(|n| n.id).max().unwrap_or(0);
        self.nodes = nodes;
        self.next_node_id = max_id + 1;
        self.selected_node_idx = if self.nodes.is_empty() { 0 } else { 0 };
        Ok(light_pos)
    }

    /// Calcule la mémoire VRAM totale consommée par l'ensemble des textures actives de la scène (avec Mipmaps)
    pub fn total_texture_vram_bytes(&self) -> usize {
        let mut total = self.white_texture.vram_bytes
            + self.checker_texture.vram_bytes
            + self.grid_texture.vram_bytes
            + self.brick_texture.vram_bytes
            + self.wood_texture.vram_bytes
            + self.marble_hd_texture.vram_bytes
            + self.carbon_hd_texture.vram_bytes
            + self.flat_normal_texture.vram_bytes
            + self.brick_normal_texture.vram_bytes
            + self.wood_normal_texture.vram_bytes;

        for node in &self.nodes {
            total += node.texture.vram_bytes;
            total += node.normal_texture.vram_bytes;
        }
        total
    }

    pub fn update_gpu(&self, queue: &wgpu::Queue) {
        for (idx, node) in self.nodes.iter().enumerate() {
            let is_selected = idx == self.selected_node_idx;
            node.update_gpu(queue, is_selected);
        }
    }
}
