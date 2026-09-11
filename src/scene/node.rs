// [WFGY] Zone: SAFE | λ: 0.3 | Fallbacks: 0/None | Action: SceneNode with Full PBR Material, Normal Mapping, and Metric Dimension Controls
#![allow(dead_code)]
use crate::gpu::{GpuTexture, UniformBuffer};
use crate::scene::mesh::{generate_cube_data, generate_prism_data, generate_pyramid_data, GpuMesh, PrimitiveType, Vertex};
use glam::{Mat4, Quat, Vec3};
use std::sync::Arc;

#[repr(C)]
#[derive(Clone, Copy, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct ModelUniform {
    pub model: [[f32; 4]; 4],
    pub color: [f32; 4],
    pub selected: u32,
    pub _pad0: u32,
    pub _pad1: u32,
    pub _pad2: u32,
}

impl Default for ModelUniform {
    fn default() -> Self {
        ModelUniform {
            model: Mat4::IDENTITY.to_cols_array_2d(),
            color: [0.0, 0.0, 0.0, 0.0],
            selected: 0,
            _pad0: 0,
            _pad1: 0,
            _pad2: 0,
        }
    }
}

fn default_ior() -> f32 {
    1.5
}

#[repr(C, align(16))]
#[derive(Clone, Copy, Debug, bytemuck::Pod, bytemuck::Zeroable, serde::Serialize, serde::Deserialize)]
pub struct MaterialUniform {
    pub roughness: f32,           // offset 0 (4 octets)
    pub metallic: f32,            // offset 4 (4 octets)
    pub uv_tiling: [f32; 2],      // offset 8 (8 octets)
    pub uv_offset: [f32; 2],      // offset 16 (8 octets)
    #[serde(default = "default_ior")]
    pub ior: f32,                 // offset 24 (4 octets - indice de réfraction, ex: 1.5 verre)
    #[serde(default)]
    pub transmission: f32,        // offset 28 (4 octets - transparence / verre 0.0..1.0)
    pub emission_color: [f32; 4], // offset 32 (16 octets)
    pub use_normal_map: u32,      // offset 48 (4 octets)
    #[serde(default)]
    pub clearcoat: f32,           // offset 52 (4 octets - couche de vernis brillant 0.0..1.0)
    #[serde(default)]
    pub clearcoat_roughness: f32, // offset 56 (4 octets - rugosité du vernis 0.0..1.0)
    #[serde(default)]
    pub subsurface: f32,          // offset 60 (4 octets - diffusion sous-surfacique SSS 0.0..1.0)
} // Total = 64 octets

impl Default for MaterialUniform {
    fn default() -> Self {
        MaterialUniform {
            roughness: 0.5,
            metallic: 0.0,
            uv_tiling: [1.0, 1.0],
            uv_offset: [0.0, 0.0],
            ior: 1.5,
            transmission: 0.0,
            emission_color: [0.0, 0.0, 0.0, 0.0],
            use_normal_map: 0,
            clearcoat: 0.0,
            clearcoat_roughness: 0.05,
            subsurface: 0.0,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Transform {
    pub translation: Vec3,
    pub rotation: Vec3, // Angles d'Euler (radians)
    pub scale: Vec3,
}

impl Default for Transform {
    fn default() -> Self {
        Transform {
            translation: Vec3::ZERO,
            rotation: Vec3::ZERO,
            scale: Vec3::ONE,
        }
    }
}

impl Transform {
    pub fn matrix(&self) -> Mat4 {
        let rot = Quat::from_euler(glam::EulerRot::YXZ, self.rotation.y, self.rotation.x, self.rotation.z);
        Mat4::from_scale_rotation_translation(self.scale, rot, self.translation)
    }
}

pub struct SceneNode {
    pub id: usize,
    pub name: String,
    pub primitive_type: PrimitiveType,
    pub transform: Transform,
    pub vertices: Vec<Vertex>,
    pub indices: Vec<u32>,
    pub gpu_mesh: GpuMesh,
    pub model_uniform: UniformBuffer<ModelUniform>,
    pub material: MaterialUniform,
    pub material_uniform: UniformBuffer<MaterialUniform>,
    pub bind_group: wgpu::BindGroup,
    pub selected_vertex_idx: usize,
    pub color: [f32; 4],
    pub texture: Arc<GpuTexture>,
    pub texture_name: Option<String>,
    pub normal_texture: Arc<GpuTexture>,
    pub normal_texture_name: Option<String>,
    pub instances: Vec<crate::scene::instance::Instance>,
    pub gpu_instances: Option<crate::scene::instance::GpuInstanceBuffer>,
    pub rigged_model: Option<crate::animation::RiggedModel>,
    // Hiérarchie de scène (Scene Graph)
    pub parent_idx: Option<usize>,
    pub children_indices: Vec<usize>,
    // Déformateurs Non Destructifs (Modifier Stack)
    pub base_vertices: Vec<Vertex>,
    pub modifiers: Vec<crate::animation::MeshModifier>,
}

impl SceneNode {


    pub fn new(
        device: &wgpu::Device,
        model_bind_group_layout: &wgpu::BindGroupLayout,
        texture: Arc<GpuTexture>,
        normal_texture: Arc<GpuTexture>,
        id: usize,
        prim: PrimitiveType,
        translation: Vec3,
    ) -> Self {
        let (vertices, indices) = match &prim {
            PrimitiveType::Cube => generate_cube_data(),
            PrimitiveType::Pyramid => generate_pyramid_data(),
            PrimitiveType::Prism => generate_prism_data(),
            PrimitiveType::Sphere => crate::scene::loader::generate_sphere_data(1.0, 16, 16),
            PrimitiveType::Cylinder => crate::scene::loader::generate_cylinder_data(0.8, 1.8, 16),
            PrimitiveType::Torus => crate::scene::loader::generate_torus_data(1.0, 0.35, 16, 12),
            PrimitiveType::Tree => crate::scene::loader::generate_lowpoly_tree_data(),
            PrimitiveType::Column => crate::scene::loader::generate_column_data(),
            PrimitiveType::Plane => crate::scene::loader::generate_plane_data(10.0, 10.0),
            PrimitiveType::Custom(_) => (Vec::new(), Vec::new()),
        };

        let gpu_mesh = GpuMesh::new(device, Some(&format!("Node #{}", id)), &vertices, &indices);
        let mut transform = Transform::default();
        transform.translation = translation;

        let initial_model = ModelUniform {
            model: transform.matrix().to_cols_array_2d(),
            color: [0.0, 0.0, 0.0, 0.0],
            selected: 0,
            _pad0: 0,
            _pad1: 0,
            _pad2: 0,
        };

        let model_uniform = UniformBuffer::new(
            device,
            Some(&format!("Model Uniform #{}", id)),
            &initial_model,
        );

        let material = MaterialUniform::default();
        let material_uniform = UniformBuffer::new(
            device,
            Some(&format!("Material Uniform #{}", id)),
            &material,
        );

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some(&format!("Model & Material BindGroup #{}", id)),
            layout: model_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: model_uniform.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: material_uniform.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::TextureView(&texture.view),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: wgpu::BindingResource::TextureView(&normal_texture.view),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: wgpu::BindingResource::Sampler(&texture.sampler),
                },
            ],
        });

        let name = match &prim {
            PrimitiveType::Cube => format!("Cube #{}", id),
            PrimitiveType::Pyramid => format!("Pyramide #{}", id),
            PrimitiveType::Prism => format!("Prisme #{}", id),
            PrimitiveType::Sphere => format!("Sphère #{}", id),
            PrimitiveType::Cylinder => format!("Cylindre #{}", id),
            PrimitiveType::Torus => format!("Tore #{}", id),
            PrimitiveType::Tree => format!("Arbre #{}", id),
            PrimitiveType::Column => format!("Colonne #{}", id),
            PrimitiveType::Plane => format!("Plan #{}", id),
            PrimitiveType::Custom(custom_name) => format!("{} #{}", custom_name, id),
        };

        SceneNode {
            id,
            name,
            primitive_type: prim,
            transform,
            vertices: vertices.clone(),
            indices,
            gpu_mesh,
            model_uniform,
            material,
            material_uniform,
            bind_group,
            selected_vertex_idx: 0,
            color: [0.0, 0.0, 0.0, 0.0],
            texture,
            texture_name: None,
            normal_texture,
            normal_texture_name: None,
            instances: Vec::new(),
            gpu_instances: None,
            rigged_model: None,
            parent_idx: None,
            children_indices: Vec::new(),
            base_vertices: vertices,
            modifiers: Vec::new(),
        }
    }



    /// Attache un modèle articulé/riggé avec squelette et animations à ce nœud
    pub fn attach_rigged_model(&mut self, rigged: crate::animation::RiggedModel) {
        self.vertices = rigged.compute_skinned_vertices();
        self.indices = rigged.indices.clone();
        self.rigged_model = Some(rigged);
    }


    /// Crée un nœud à partir d'un maillage personnalisé chargé (ex: glTF, OBJ) avec textures et matériaux complets
    pub fn from_custom_mesh_full(
        device: &wgpu::Device,
        model_bind_group_layout: &wgpu::BindGroupLayout,
        texture: Arc<GpuTexture>,
        texture_name: Option<String>,
        normal_texture: Arc<GpuTexture>,
        normal_texture_name: Option<String>,
        material: Option<MaterialUniform>,
        color: [f32; 4],
        id: usize,
        name: &str,
        vertices: Vec<Vertex>,
        indices: Vec<u32>,
        translation: Vec3,
    ) -> Self {
        let gpu_mesh = GpuMesh::new(device, Some(&format!("Node #{} ({})", id, name)), &vertices, &indices);
        let mut transform = Transform::default();
        transform.translation = translation;

        let initial_model = ModelUniform {
            model: transform.matrix().to_cols_array_2d(),
            color,
            selected: 0,
            _pad0: 0,
            _pad1: 0,
            _pad2: 0,
        };

        let model_uniform = UniformBuffer::new(
            device,
            Some(&format!("Model Uniform #{}", id)),
            &initial_model,
        );

        let mat = material.unwrap_or_default();
        let material_uniform = UniformBuffer::new(
            device,
            Some(&format!("Material Uniform #{}", id)),
            &mat,
        );

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some(&format!("Model & Material BindGroup #{}", id)),
            layout: model_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: model_uniform.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: material_uniform.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::TextureView(&texture.view),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: wgpu::BindingResource::TextureView(&normal_texture.view),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: wgpu::BindingResource::Sampler(&texture.sampler),
                },
            ],
        });

        SceneNode {
            id,
            name: format!("{} #{}", name, id),
            primitive_type: PrimitiveType::Custom(name.to_string()),
            transform,
            vertices: vertices.clone(),
            indices,
            gpu_mesh,
            model_uniform,
            material: mat,
            material_uniform,
            bind_group,
            selected_vertex_idx: 0,
            color,
            texture,
            texture_name,
            normal_texture,
            normal_texture_name,
            instances: Vec::new(),
            gpu_instances: None,
            rigged_model: None,
            parent_idx: None,
            children_indices: Vec::new(),
            base_vertices: vertices,
            modifiers: Vec::new(),
        }
    }


    /// Évalue et applique la pile de modificateurs non destructifs sur la géométrie
    pub fn evaluate_modifiers(&mut self, time: f32) -> bool {
        if self.modifiers.is_empty() || !self.modifiers.iter().any(|m| m.is_enabled()) {
            return false;
        }

        self.vertices = crate::animation::evaluate_modifier_stack(&self.base_vertices, &self.modifiers, time);
        true
    }

    /// Fige définitivement la déformation actuelle en écrasant la géométrie de base (Apply Modifier)
    pub fn apply_modifiers_permanently(&mut self) {
        self.base_vertices = self.vertices.clone();
        self.modifiers.clear();
    }



    /// Crée un nœud à partir d'un maillage personnalisé chargé (rétrocompatibilité)
    pub fn from_custom_mesh(
        device: &wgpu::Device,
        model_bind_group_layout: &wgpu::BindGroupLayout,
        texture: Arc<GpuTexture>,
        normal_texture: Arc<GpuTexture>,
        id: usize,
        name: &str,
        vertices: Vec<Vertex>,
        indices: Vec<u32>,
        translation: Vec3,
    ) -> Self {
        Self::from_custom_mesh_full(
            device,
            model_bind_group_layout,
            texture,
            None,
            normal_texture,
            None,
            None,
            [0.0, 0.0, 0.0, 0.0],
            id,
            name,
            vertices,
            indices,
            translation,
        )
    }

    /// Calcule la boîte englobante locale (AABB) à partir des sommets actuels
    pub fn local_aabb(&self) -> crate::scene::raycast::Aabb {
        let points: Vec<Vec3> = self.vertices.iter().map(|v| Vec3::from_array(v.position)).collect();
        crate::scene::raycast::Aabb::from_points(&points)
    }

    /// Calcule les dimensions réelles de l'objet dans l'espace 3D (Largeur X, Hauteur Y, Profondeur Z en mètres)
    pub fn dimensions(&self) -> Vec3 {
        let aabb = self.local_aabb();
        let local_size = aabb.size();
        Vec3::new(
            (local_size.x * self.transform.scale.x.abs()).max(0.001),
            (local_size.y * self.transform.scale.y.abs()).max(0.001),
            (local_size.z * self.transform.scale.z.abs()).max(0.001),
        )
    }

    /// Modifie directement les dimensions physiques de l'objet (recalcule automatiquement le facteur d'échelle)
    pub fn set_dimensions(&mut self, target_dims: Vec3) {
        let aabb = self.local_aabb();
        let local_size = aabb.size();

        let sx = if local_size.x > 0.001 { (target_dims.x / local_size.x).max(0.001) } else { 1.0 };
        let sy = if local_size.y > 0.001 { (target_dims.y / local_size.y).max(0.001) } else { 1.0 };
        let sz = if local_size.z > 0.001 { (target_dims.z / local_size.z).max(0.001) } else { 1.0 };

        self.transform.scale = Vec3::new(sx, sy, sz);
    }

    /// Change la texture diffuse et régénère le BindGroup GPU
    pub fn set_texture(
        &mut self,
        device: &wgpu::Device,
        model_bind_group_layout: &wgpu::BindGroupLayout,
        texture: Arc<GpuTexture>,
        name: Option<String>,
    ) {
        self.texture = texture;
        self.texture_name = name;
        self.recreate_bind_group(device, model_bind_group_layout);
    }

    /// Change la texture de normales et active le normal mapping
    pub fn set_normal_texture(
        &mut self,
        device: &wgpu::Device,
        model_bind_group_layout: &wgpu::BindGroupLayout,
        normal_texture: Arc<GpuTexture>,
        name: Option<String>,
    ) {
        self.normal_texture = normal_texture;
        self.normal_texture_name = name.clone();
        self.material.use_normal_map = if name.is_some() { 1 } else { 0 };
        self.recreate_bind_group(device, model_bind_group_layout);
    }

    /// Régénère le bind group GPU avec les textures et buffers actuels
    pub fn recreate_bind_group(
        &mut self,
        device: &wgpu::Device,
        model_bind_group_layout: &wgpu::BindGroupLayout,
    ) {
        self.bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some(&format!("Model & Material BindGroup #{}", self.id)),
            layout: model_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: self.model_uniform.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: self.material_uniform.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::TextureView(&self.texture.view),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: wgpu::BindingResource::TextureView(&self.normal_texture.view),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: wgpu::BindingResource::Sampler(&self.texture.sampler),
                },
            ],
        });
    }

    /// Clone un nœud de scène pour la duplication (avec nouveaux buffers GPU)
    pub fn clone_with_new_id(
        &self,
        device: &wgpu::Device,
        model_bind_group_layout: &wgpu::BindGroupLayout,
        new_id: usize,
        new_translation: Vec3,
    ) -> Self {
        let mut new_node = Self::new(
            device,
            model_bind_group_layout,
            self.texture.clone(),
            self.normal_texture.clone(),
            new_id,
            self.primitive_type.clone(),
            new_translation,
        );
        new_node.name = format!("{} (Copie)", self.name);
        new_node.transform.rotation = self.transform.rotation;
        new_node.transform.scale = self.transform.scale;
        new_node.vertices = self.vertices.clone();
        new_node.indices = self.indices.clone();
        new_node.color = self.color;
        new_node.material = self.material;
        new_node.texture_name = self.texture_name.clone();
        new_node.normal_texture_name = self.normal_texture_name.clone();
        new_node.instances = self.instances.clone();
        new_node
    }

    /// Configure une liste d'instances GPU pour ce maillage (accélération matérielle 1 seul Draw Call)
    pub fn set_instances(&mut self, device: &wgpu::Device, instances: Vec<crate::scene::instance::Instance>) {
        let raw: Vec<crate::scene::instance::InstanceRaw> = instances.iter().map(|i| i.to_raw()).collect();
        self.gpu_instances = Some(crate::scene::instance::GpuInstanceBuffer::new(
            device,
            Some(&format!("Node #{} Instance Buffer", self.id)),
            &raw,
        ));
        self.instances = instances;
    }

    /// Met à jour les transformations d'instances sur le GPU
    #[allow(dead_code)]
    pub fn update_instances_gpu(&mut self, queue: &wgpu::Queue) {
        if let Some(ref mut gpu_inst) = self.gpu_instances {
            let raw: Vec<crate::scene::instance::InstanceRaw> = self.instances.iter().map(|i| i.to_raw()).collect();
            gpu_inst.update(queue, &raw);
        }
    }

    pub fn update_model_uniform(&self, queue: &wgpu::Queue) {
        let uniform = ModelUniform {
            model: self.transform.matrix().to_cols_array_2d(),
            color: self.color,
            selected: 0,
            _pad0: 0,
            _pad1: 0,
            _pad2: 0,
        };
        self.model_uniform.update(queue, &uniform);
    }

    pub fn update_material_uniform(&self, queue: &wgpu::Queue) {
        self.material_uniform.update(queue, &self.material);
    }

    pub fn update_gpu(&self, queue: &wgpu::Queue, is_selected: bool) {
        let uniform = ModelUniform {
            model: self.transform.matrix().to_cols_array_2d(),
            color: self.color,
            selected: if is_selected { 1 } else { 0 },
            _pad0: 0,
            _pad1: 0,
            _pad2: 0,
        };
        self.model_uniform.update(queue, &uniform);
        self.material_uniform.update(queue, &self.material);
    }

    pub fn update_vertices_gpu(&self, queue: &wgpu::Queue) {
        self.gpu_mesh.update_vertices(queue, &self.vertices);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_transform_matrix() {
        let mut t = Transform::default();
        t.translation = Vec3::new(1.0, 2.0, 3.0);
        t.scale = Vec3::new(2.0, 2.0, 2.0);
        let mat = t.matrix();
        assert_ne!(mat, Mat4::IDENTITY);
        assert_eq!(mat.w_axis, glam::Vec4::new(1.0, 2.0, 3.0, 1.0));
    }

    #[test]
    fn test_node_dimensions_and_scaling() {
        let mut transform = Transform::default();
        transform.scale = Vec3::new(2.0, 3.0, 4.0);
        let (vertices, _) = generate_cube_data();
        let points: Vec<Vec3> = vertices.iter().map(|v| Vec3::from_array(v.position)).collect();
        let aabb = crate::scene::raycast::Aabb::from_points(&points);
        let local_size = aabb.size();
        
        assert!((local_size.x - 2.0).abs() < 0.05);
        assert!((local_size.y - 2.0).abs() < 0.05);
        assert!((local_size.z - 2.0).abs() < 0.05);

        let world_size = local_size * transform.scale;
        assert!((world_size.x - 4.0).abs() < 0.1);
        assert!((world_size.y - 6.0).abs() < 0.1);
        assert!((world_size.z - 8.0).abs() < 0.1);
    }

    #[test]
    fn test_material_uniform_size_and_alignment() {
        assert_eq!(std::mem::size_of::<MaterialUniform>(), 64);
        assert_eq!(std::mem::align_of::<MaterialUniform>(), 16);
    }
}
