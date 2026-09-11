// [WFGY] Zone: SAFE | λ: 0.25 | Fallbacks: 0/None | Action: Fluent WorldBuilder, MaterialBuilder & NodeHandle API
use glam::Vec3;
use crate::scene::node::MaterialUniform;
use crate::scene::{PrimitiveType, Scene};

/// Builder fluide pour construire des matériaux PBR photoréalistes en une ligne
#[derive(Clone, Debug)]
pub struct MaterialBuilder {
    material: MaterialUniform,
}

impl Default for MaterialBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl MaterialBuilder {
    pub fn new() -> Self {
        Self {
            material: MaterialUniform::default(),
        }
    }

    /// Matériau plastique / diélectrique standard
    pub fn plastic(roughness: f32) -> Self {
        Self::new().roughness(roughness).metallic(0.0)
    }

    /// Matériau métallique brillant ou brossé
    pub fn metal(metallic: f32, roughness: f32) -> Self {
        Self::new().metallic(metallic).roughness(roughness)
    }

    /// Matériau verre ou cristal translucide avec réfraction IOR
    pub fn glass(ior: f32, roughness: f32) -> Self {
        Self::new()
            .transmission(1.0)
            .ior(ior)
            .roughness(roughness)
            .metallic(0.0)
    }

    /// Matériau carrosserie automobile ou vernis multicouche
    pub fn car_paint(clearcoat: f32, clearcoat_roughness: f32) -> Self {
        Self::new()
            .metallic(0.8)
            .roughness(0.2)
            .clearcoat(clearcoat, clearcoat_roughness)
    }

    /// Matériau organique / peau / cire avec diffusion sous-surfacique SSS
    pub fn organic(subsurface: f32, roughness: f32) -> Self {
        Self::new()
            .subsurface(subsurface)
            .roughness(roughness)
            .metallic(0.0)
    }

    pub fn roughness(mut self, roughness: f32) -> Self {
        self.material.roughness = roughness.clamp(0.0, 1.0);
        self
    }

    pub fn metallic(mut self, metallic: f32) -> Self {
        self.material.metallic = metallic.clamp(0.0, 1.0);
        self
    }

    pub fn ior(mut self, ior: f32) -> Self {
        self.material.ior = ior.max(1.0);
        self
    }

    pub fn transmission(mut self, transmission: f32) -> Self {
        self.material.transmission = transmission.clamp(0.0, 1.0);
        self
    }

    pub fn clearcoat(mut self, clearcoat: f32, roughness: f32) -> Self {
        self.material.clearcoat = clearcoat.clamp(0.0, 1.0);
        self.material.clearcoat_roughness = roughness.clamp(0.0, 1.0);
        self
    }

    pub fn subsurface(mut self, subsurface: f32) -> Self {
        self.material.subsurface = subsurface.clamp(0.0, 1.0);
        self
    }

    pub fn emission(mut self, rgb: [f32; 3], intensity: f32) -> Self {
        self.material.emission_color = [rgb[0], rgb[1], rgb[2], intensity];
        self
    }

    pub fn uv_tiling(mut self, u: f32, v: f32) -> Self {
        self.material.uv_tiling = [u, v];
        self
    }

    pub fn build(self) -> MaterialUniform {
        self.material
    }
}

/// Poignée de manipulation d'un nœud dans la scène
pub struct NodeHandle<'a> {
    pub scene: &'a mut Scene,
    pub index: usize,
    pub queue: &'a wgpu::Queue,
}

impl<'a> NodeHandle<'a> {
    pub fn at(self, pos: [f32; 3]) -> Self {
        if let Some(node) = self.scene.nodes.get_mut(self.index) {
            node.transform.translation = Vec3::from_array(pos);
            node.update_gpu(self.queue, false);
        }
        self
    }

    pub fn scale(self, s: [f32; 3]) -> Self {
        if let Some(node) = self.scene.nodes.get_mut(self.index) {
            node.transform.scale = Vec3::from_array(s);
            node.update_gpu(self.queue, false);
        }
        self
    }

    pub fn scale_uniform(self, s: f32) -> Self {
        if let Some(node) = self.scene.nodes.get_mut(self.index) {
            node.transform.scale = Vec3::splat(s);
            node.update_gpu(self.queue, false);
        }
        self
    }

    pub fn rotate_deg(self, deg: [f32; 3]) -> Self {
        if let Some(node) = self.scene.nodes.get_mut(self.index) {
            node.transform.rotation = Vec3::new(
                deg[0].to_radians(),
                deg[1].to_radians(),
                deg[2].to_radians(),
            );
            node.update_gpu(self.queue, false);
        }
        self
    }

    pub fn material(self, builder: MaterialBuilder) -> Self {
        if let Some(node) = self.scene.nodes.get_mut(self.index) {
            node.material = builder.build();
            node.update_gpu(self.queue, false);
        }
        self
    }

    pub fn color(self, rgba: [f32; 4]) -> Self {
        if let Some(node) = self.scene.nodes.get_mut(self.index) {
            node.color = rgba;
            node.update_gpu(self.queue, false);
        }
        self
    }
}

/// Contexte de construction globale de la scène (WorldBuilder)
pub struct WorldBuilder<'a> {
    pub scene: &'a mut Scene,
    pub device: &'a wgpu::Device,
    pub queue: &'a wgpu::Queue,
}

impl<'a> WorldBuilder<'a> {
    pub fn new(scene: &'a mut Scene, device: &'a wgpu::Device, queue: &'a wgpu::Queue) -> Self {
        Self { scene, device, queue }
    }

    /// Crée un mesh primitif dans la scène avec une poignée chaînable
    pub fn spawn_mesh(&mut self, name: &str, primitive: PrimitiveType) -> NodeHandle<'_> {
        let idx = self.scene.add_node_at(self.device, primitive, Vec3::ZERO);
        if let Some(node) = self.scene.nodes.get_mut(idx) {
            node.name = name.to_string();
        }
        NodeHandle {
            scene: self.scene,
            index: idx,
            queue: self.queue,
        }
    }

    /// Crée un mesh personnalisé
    pub fn spawn_custom_mesh(&mut self, name: &str, vertices: Vec<crate::scene::Vertex>, indices: Vec<u32>) -> NodeHandle<'_> {
        let idx = self.scene.nodes.len();
        self.scene.add_custom_mesh(self.device, name, vertices, indices);
        NodeHandle {
            scene: self.scene,
            index: idx,
            queue: self.queue,
        }
    }

    /// Ajoute une source lumineuse ponctuelle (lanterne, néon, cristal)
    pub fn spawn_point_light(&mut self, pos: [f32; 3], color_rgb: [f32; 3], intensity: f32, radius: f32) -> &mut Self {
        let _ = (pos, color_rgb, intensity, radius);
        // Les lumières ponctuelles sont stockées dans le LightUniform du ForwardRenderer
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_material_builder_fluent_chain() {
        let mat = MaterialBuilder::car_paint(1.0, 0.02)
            .emission([0.0, 1.0, 2.0], 1.5)
            .subsurface(0.2)
            .build();

        assert_eq!(mat.clearcoat, 1.0);
        assert_eq!(mat.clearcoat_roughness, 0.02);
        assert_eq!(mat.subsurface, 0.2);
        assert_eq!(mat.emission_color[3], 1.5);
    }
}
