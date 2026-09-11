// [WFGY] Zone: SAFE | λ: 0.2 | Fallbacks: 0 | Action: GPU BVH Builder with Multi-Material and Vertex Color Support

use glam::{Vec3, Vec4};
use crate::scene::node::SceneNode;

#[repr(C, align(16))]
#[derive(Clone, Copy, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct GpuTriangle {
    pub v0: [f32; 3],
    pub material_id: u32,
    pub v1: [f32; 3],
    pub _pad0: u32,
    pub v2: [f32; 3],
    pub _pad1: u32,
    pub n0: [f32; 3],
    pub _pad2: u32,
    pub n1: [f32; 3],
    pub _pad3: u32,
    pub n2: [f32; 3],
    pub _pad4: u32,
    pub uv0: [f32; 2],
    pub uv1: [f32; 2],
    pub uv2: [f32; 2],
    pub _pad5: [f32; 2],
}

#[repr(C, align(16))]
#[derive(Clone, Copy, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct GpuMaterial {
    pub albedo: [f32; 4],
    pub emission: [f32; 4],
    pub roughness: f32,
    pub metallic: f32,
    pub ior: f32,
    pub transmission: f32,
    pub use_texture: u32,
    pub _pad0: u32,
    pub _pad1: u32,
    pub _pad2: u32,
}

#[repr(C, align(16))]
#[derive(Clone, Copy, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct GpuBvhNode {
    pub aabb_min: [f32; 3],
    pub left_or_first: i32,
    pub aabb_max: [f32; 3],
    pub count: i32, // <= 0: Feuille (-count triangles), > 0: Nœud interne (count = right child)
}

#[derive(Clone, Debug)]
struct TriangleInfo {
    gpu_triangle: GpuTriangle,
    center: Vec3,
    min: Vec3,
    max: Vec3,
}

#[derive(Clone, Copy, Debug)]
pub struct Aabb {
    pub min: Vec3,
    pub max: Vec3,
}

impl Aabb {
    pub fn empty() -> Self {
        Self {
            min: Vec3::splat(f32::INFINITY),
            max: Vec3::splat(f32::NEG_INFINITY),
        }
    }

    pub fn grow(&mut self, point: Vec3) {
        self.min = self.min.min(point);
        self.max = self.max.max(point);
    }

    #[allow(dead_code)]
    pub fn grow_aabb(&mut self, other: &Aabb) {
        self.min = self.min.min(other.min);
        self.max = self.max.max(other.max);
    }
}

pub struct BvhBuilder;

impl BvhBuilder {
    /// Construit le BVH complet à partir d'une liste de noeuds de la scène
    pub fn build_from_scene_nodes(
        nodes: &[SceneNode],
    ) -> (Vec<GpuTriangle>, Vec<GpuBvhNode>, Vec<GpuMaterial>) {
        let mut triangle_infos = Vec::new();
        let mut gpu_materials = Vec::new();

        for node in nodes {
            let model_matrix = node.transform.matrix();
            let normal_matrix = model_matrix.inverse().transpose();

            let vertices = &node.vertices;
            let indices = &node.indices;

            for chunk in indices.chunks_exact(3) {
                let v0_raw = vertices[chunk[0] as usize];
                let v1_raw = vertices[chunk[1] as usize];
                let v2_raw = vertices[chunk[2] as usize];

                // Détermination de l'Albedo : Couleur de nœud prioritaire, sinon couleur du sommet
                let albedo = if node.color[3] > 0.01 {
                    node.color
                } else if v0_raw.color[3] > 0.01 {
                    v0_raw.color
                } else {
                    [0.85, 0.85, 0.85, 1.0]
                };

                let use_texture = if node.texture_name.is_some() { 1u32 } else { 0u32 };

                // Chercher ou créer le matériau correspondant
                let mat_idx = match gpu_materials.iter().position(|m: &GpuMaterial| {
                    m.albedo == albedo
                        && m.emission == node.material.emission_color
                        && (m.roughness - node.material.roughness).abs() < 1e-4
                        && (m.metallic - node.material.metallic).abs() < 1e-4
                        && (m.ior - node.material.ior).abs() < 1e-4
                        && (m.transmission - node.material.transmission).abs() < 1e-4
                        && m.use_texture == use_texture
                }) {
                    Some(idx) => idx as u32,
                    None => {
                        let new_idx = gpu_materials.len() as u32;
                        gpu_materials.push(GpuMaterial {
                            albedo,
                            emission: node.material.emission_color,
                            roughness: node.material.roughness,
                            metallic: node.material.metallic,
                            ior: node.material.ior,
                            transmission: node.material.transmission,
                            use_texture,
                            _pad0: 0,
                            _pad1: 0,
                            _pad2: 0,
                        });
                        new_idx
                    }
                };

                let p0 = (model_matrix * Vec4::new(v0_raw.position[0], v0_raw.position[1], v0_raw.position[2], 1.0)).truncate();
                let p1 = (model_matrix * Vec4::new(v1_raw.position[0], v1_raw.position[1], v1_raw.position[2], 1.0)).truncate();
                let p2 = (model_matrix * Vec4::new(v2_raw.position[0], v2_raw.position[1], v2_raw.position[2], 1.0)).truncate();

                let n0 = (normal_matrix * Vec4::new(v0_raw.normal[0], v0_raw.normal[1], v0_raw.normal[2], 0.0)).truncate().normalize_or_zero();
                let n1 = (normal_matrix * Vec4::new(v1_raw.normal[0], v1_raw.normal[1], v1_raw.normal[2], 0.0)).truncate().normalize_or_zero();
                let n2 = (normal_matrix * Vec4::new(v2_raw.normal[0], v2_raw.normal[1], v2_raw.normal[2], 0.0)).truncate().normalize_or_zero();

                let center = (p0 + p1 + p2) / 3.0;
                let min = p0.min(p1).min(p2);
                let max = p0.max(p1).max(p2);

                let gpu_tri = GpuTriangle {
                    v0: [p0.x, p0.y, p0.z],
                    material_id: mat_idx,
                    v1: [p1.x, p1.y, p1.z],
                    _pad0: 0,
                    v2: [p2.x, p2.y, p2.z],
                    _pad1: 0,
                    n0: [n0.x, n0.y, n0.z],
                    _pad2: 0,
                    n1: [n1.x, n1.y, n1.z],
                    _pad3: 0,
                    n2: [n2.x, n2.y, n2.z],
                    _pad4: 0,
                    uv0: v0_raw.uv,
                    uv1: v1_raw.uv,
                    uv2: v2_raw.uv,
                    _pad5: [0.0, 0.0],
                };

                triangle_infos.push(TriangleInfo {
                    gpu_triangle: gpu_tri,
                    center,
                    min,
                    max,
                });
            }
        }

        if triangle_infos.is_empty() {
            let empty_node = GpuBvhNode {
                aabb_min: [-1.0, -1.0, -1.0],
                left_or_first: 0,
                aabb_max: [1.0, 1.0, 1.0],
                count: 0,
            };
            return (Vec::new(), vec![empty_node], gpu_materials);
        }

        let total_count = triangle_infos.len();
        let mut gpu_triangles = Vec::with_capacity(total_count);
        let mut bvh_nodes = Vec::new();

        Self::subdivide(
            &mut triangle_infos,
            0,
            total_count,
            &mut gpu_triangles,
            &mut bvh_nodes,
        );

        (gpu_triangles, bvh_nodes, gpu_materials)
    }

    /// Subdivision récursive de l'arbre BVH
    fn subdivide(
        triangles: &mut [TriangleInfo],
        start: usize,
        count: usize,
        out_triangles: &mut Vec<GpuTriangle>,
        out_nodes: &mut Vec<GpuBvhNode>,
    ) -> usize {
        let node_index = out_nodes.len();
        out_nodes.push(GpuBvhNode {
            aabb_min: [0.0; 3],
            left_or_first: 0,
            aabb_max: [0.0; 3],
            count: 0,
        });

        // Calcul de l'AABB globale du nœud
        let mut node_aabb = Aabb::empty();
        for tri in &triangles[start..start + count] {
            node_aabb.grow(tri.min);
            node_aabb.grow(tri.max);
        }

        // Seuil feuille : <= 4 triangles
        if count <= 4 {
            let first_triangle = out_triangles.len() as i32;
            for tri in &triangles[start..start + count] {
                out_triangles.push(tri.gpu_triangle);
            }

            out_nodes[node_index] = GpuBvhNode {
                aabb_min: [node_aabb.min.x, node_aabb.min.y, node_aabb.min.z],
                left_or_first: first_triangle,
                aabb_max: [node_aabb.max.x, node_aabb.max.y, node_aabb.max.z],
                count: -(count as i32),
            };
            return node_index;
        }

        // Trouver l'axe avec la plus grande étendue
        let extent = node_aabb.max - node_aabb.min;
        let mut axis = 0;
        if extent.y > extent.x && extent.y > extent.z {
            axis = 1;
        } else if extent.z > extent.x && extent.z > extent.y {
            axis = 2;
        }

        // Partitionnement linéaire O(N) des triangles selon leur centre sur l'axe dominant
        let mid = count / 2;
        let slice = &mut triangles[start..start + count];
        slice.select_nth_unstable_by(mid, |a, b| {
            let val_a = match axis {
                0 => a.center.x,
                1 => a.center.y,
                _ => a.center.z,
            };
            let val_b = match axis {
                0 => b.center.x,
                1 => b.center.y,
                _ => b.center.z,
            };
            val_a.partial_cmp(&val_b).unwrap_or(std::cmp::Ordering::Equal)
        });

        let left_child = Self::subdivide(triangles, start, mid, out_triangles, out_nodes);
        let right_child = Self::subdivide(triangles, start + mid, count - mid, out_triangles, out_nodes);

        out_nodes[node_index] = GpuBvhNode {
            aabb_min: [node_aabb.min.x, node_aabb.min.y, node_aabb.min.z],
            left_or_first: left_child as i32,
            aabb_max: [node_aabb.max.x, node_aabb.max.y, node_aabb.max.z],
            count: right_child as i32,
        };

        node_index
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bvh_gpu_structures_layout() {
        assert_eq!(std::mem::size_of::<GpuTriangle>(), 128);
        assert_eq!(std::mem::align_of::<GpuTriangle>(), 16);

        assert_eq!(std::mem::size_of::<GpuBvhNode>(), 32);
        assert_eq!(std::mem::align_of::<GpuBvhNode>(), 16);

        assert_eq!(std::mem::size_of::<GpuMaterial>(), 64);
        assert_eq!(std::mem::align_of::<GpuMaterial>(), 16);
    }

    #[test]
    fn test_empty_bvh_builder() {
        let (triangles, nodes, materials) = BvhBuilder::build_from_scene_nodes(&[]);
        assert_eq!(triangles.len(), 0);
        assert_eq!(nodes.len(), 1);
        assert_eq!(materials.len(), 0);
    }
}
