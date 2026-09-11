// [WFGY] Zone: SAFE | λ: 0.25 | Fallbacks: 0/None | Action: Scene serialization and persistence (JSON .world format with Full PBR Material and Tangents)
use crate::gpu::GpuTexture;
use crate::scene::mesh::{PrimitiveType, Vertex};
use crate::scene::node::{MaterialUniform, SceneNode, Transform};
use glam::Vec3;
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::{Read, Write};
use std::path::Path;
use std::sync::Arc;

/// Données sérialisables d'un nœud de scène 3D avec propriétés PBR
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NodeData {
    pub id: usize,
    pub name: String,
    pub primitive_type: PrimitiveType,
    pub transform: Transform,
    pub color: [f32; 4],
    #[serde(skip_serializing_if = "Option::is_none")]
    pub texture_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub normal_texture_name: Option<String>,
    #[serde(default)]
    pub material: MaterialUniform,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub custom_vertices: Option<Vec<Vertex>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub custom_indices: Option<Vec<u32>>,
}

/// Structure racine du fichier `.world`
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WorldData {
    pub version: u32,
    pub generator: String,
    pub light_position: [f32; 3],
    pub nodes: Vec<NodeData>,
}

impl Default for WorldData {
    fn default() -> Self {
        WorldData {
            version: 1,
            generator: "Rust3D-WorldBuilder-v0.1".to_string(),
            light_position: [3.0, 5.0, 4.0],
            nodes: Vec::new(),
        }
    }
}

/// Convertit une liste de `SceneNode` en chaîne JSON formattée
pub fn serialize_world(nodes: &[SceneNode], light_pos: Vec3) -> Result<String, String> {
    let mut node_data_list = Vec::with_capacity(nodes.len());

    for node in nodes {
        let (custom_vertices, custom_indices) = match &node.primitive_type {
            PrimitiveType::Custom(_) => (Some(node.vertices.clone()), Some(node.indices.clone())),
            _ => (None, None),
        };

        node_data_list.push(NodeData {
            id: node.id,
            name: node.name.clone(),
            primitive_type: node.primitive_type.clone(),
            transform: node.transform,
            color: node.color,
            texture_name: node.texture_name.clone(),
            normal_texture_name: node.normal_texture_name.clone(),
            material: node.material,
            custom_vertices,
            custom_indices,
        });
    }

    let world = WorldData {
        version: 1,
        generator: "Rust3D-WorldBuilder-v0.1".to_string(),
        light_position: [light_pos.x, light_pos.y, light_pos.z],
        nodes: node_data_list,
    };

    serde_json::to_string_pretty(&world).map_err(|e| format!("Erreur lors de la sérialisation JSON: {}", e))
}

/// Sauvegarde la scène dans un fichier `.world`
pub fn save_world_to_file(path: &Path, nodes: &[SceneNode], light_pos: Vec3) -> Result<(), String> {
    let json = serialize_world(nodes, light_pos)?;
    let mut file = File::create(path).map_err(|e| format!("Impossible de créer le fichier '{:?}': {}", path, e))?;
    file.write_all(json.as_bytes())
        .map_err(|e| format!("Erreur lors de l'écriture dans le fichier: {}", e))?;
    Ok(())
}

/// Désérialise une chaîne JSON en liste de `SceneNode` GPU
pub fn deserialize_world(
    device: &wgpu::Device,
    model_bind_group_layout: &wgpu::BindGroupLayout,
    default_texture: Arc<GpuTexture>,
    default_normal_texture: Arc<GpuTexture>,
    json_str: &str,
) -> Result<(Vec<SceneNode>, Vec3), String> {
    let world: WorldData = serde_json::from_str(json_str)
        .map_err(|e| format!("Erreur lors du décodage du fichier world: {}", e))?;

    let mut nodes = Vec::with_capacity(world.nodes.len());

    for n in world.nodes {
        let mut node = match n.primitive_type {
            PrimitiveType::Custom(ref name) => {
                let vertices = n.custom_vertices.unwrap_or_default();
                let indices = n.custom_indices.unwrap_or_default();
                SceneNode::from_custom_mesh(
                    device,
                    model_bind_group_layout,
                    default_texture.clone(),
                    default_normal_texture.clone(),
                    n.id,
                    name,
                    vertices,
                    indices,
                    n.transform.translation,
                )
            }
            prim => SceneNode::new(
                device,
                model_bind_group_layout,
                default_texture.clone(),
                default_normal_texture.clone(),
                n.id,
                prim,
                n.transform.translation,
            ),
        };

        node.name = n.name;
        node.transform = n.transform;
        node.color = n.color;
        node.material = n.material;
        node.texture_name = n.texture_name;
        node.normal_texture_name = n.normal_texture_name;
        nodes.push(node);
    }

    let light_pos = Vec3::new(world.light_position[0], world.light_position[1], world.light_position[2]);
    Ok((nodes, light_pos))
}

/// Charge une scène depuis un fichier `.world`
pub fn load_world_from_file(
    device: &wgpu::Device,
    model_bind_group_layout: &wgpu::BindGroupLayout,
    default_texture: Arc<GpuTexture>,
    default_normal_texture: Arc<GpuTexture>,
    path: &Path,
) -> Result<(Vec<SceneNode>, Vec3), String> {
    let mut file = File::open(path).map_err(|e| format!("Impossible d'ouvrir le fichier '{:?}': {}", path, e))?;
    let mut json_str = String::new();
    file.read_to_string(&mut json_str)
        .map_err(|e| format!("Erreur de lecture du fichier: {}", e))?;

    deserialize_world(device, model_bind_group_layout, default_texture, default_normal_texture, &json_str)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scene::mesh::PrimitiveType;

    #[test]
    fn test_world_data_serialization() {
        let world = WorldData {
            version: 1,
            generator: "Test".into(),
            light_position: [1.0, 2.0, 3.0],
            nodes: vec![NodeData {
                id: 1,
                name: "Test Cube".into(),
                primitive_type: PrimitiveType::Cube,
                transform: Transform::default(),
                color: [1.0, 0.0, 0.0, 1.0],
                texture_name: Some("checkered".into()),
                normal_texture_name: Some("brick_normal".into()),
                material: MaterialUniform {
                    roughness: 0.2,
                    metallic: 0.8,
                    uv_tiling: [2.0, 2.0],
                    uv_offset: [0.0, 0.0],
                    ior: 1.5,
                    transmission: 0.0,
                    emission_color: [1.0, 0.5, 0.0, 1.0],
                    use_normal_map: 1,
                    _pad1: 0,
                    _pad2: 0,
                    _pad3: 0,
                },
                custom_vertices: None,
                custom_indices: None,
            }],
        };

        let json = serde_json::to_string_pretty(&world).expect("Serialization failed");
        assert!(json.contains("Test Cube"));
        assert!(json.contains("checkered"));
        assert!(json.contains("brick_normal"));

        let deserialized: WorldData = serde_json::from_str(&json).expect("Deserialization failed");
        assert_eq!(deserialized.nodes.len(), 1);
        assert_eq!(deserialized.nodes[0].name, "Test Cube");
        assert_eq!(deserialized.nodes[0].material.roughness, 0.2);
        assert_eq!(deserialized.nodes[0].material.metallic, 0.8);
        assert_eq!(deserialized.nodes[0].material.use_normal_map, 1);
    }

    #[test]
    fn test_load_sample_world_file() {
        let sample_path = std::path::Path::new("assets/monde_exemple.world");
        if sample_path.exists() {
            let mut file = File::open(sample_path).expect("Could not open sample world file");
            let mut json_str = String::new();
            file.read_to_string(&mut json_str).expect("Could not read sample world file");
            let world: WorldData = serde_json::from_str(&json_str).expect("Failed to parse sample world JSON");
            assert!(world.nodes.len() >= 5);
            assert_eq!(world.version, 1);
            assert!(world.nodes[0].material.roughness > 0.0);
        }
    }
}
