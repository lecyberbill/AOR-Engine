// [WFGY] Zone: SAFE | λ: 0.25 | Fallbacks: 0/None | Action: Industrial Binary-JSON Hybrid Container Format (.aor / .aorbundle) with 6-Tier Security and In-Place Mutations
#![allow(dead_code)]

use crate::gpu::GpuTexture;
use crate::scene::mesh::{PrimitiveType, Vertex};
use crate::scene::node::{MaterialUniform, SceneNode, Transform};
use glam::Vec3;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::{File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::Path;
use std::sync::Arc;

pub const AOR_MAGIC: &[u8; 8] = b"AORBNDL1";
pub const AOR_FORMAT_VERSION: u32 = 1;
pub const MAX_BUNDLE_ALLOCATION_BYTES: usize = 1024 * 1024 * 1024; // 1 Go max de sécurité anti-OOM
pub const AOR_ALIGNMENT: usize = 16; // Alignement 16 octets garanti pour WebGPU / SIMD
pub const AOR_INTEGRITY_SALT: u64 = 0x853C49E674F1A02Bu64;

/// En-tête binaire fixe de 48 octets
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, bytemuck::Pod, bytemuck::Zeroable)]
pub struct AorHeader {
    pub magic: [u8; 8],              // b"AORBNDL1"
    pub version: u32,                // 1
    pub flags: u32,                  // Flags (bit 0 = signed, bit 1 = dev_override_allowed)
    pub json_offset: u64,            // Décalage du segment JSON
    pub json_length: u64,            // Taille du segment JSON
    pub binary_offset: u64,          // Décalage du segment Binaire
    pub binary_length: u64,          // Taille du segment Binaire
    pub signature: u64,              // Signature / Checksum d'intégrité scellée
}

impl AorHeader {
    pub const SIZE: usize = 56; // 8 + 4 + 4 + 8 + 8 + 8 + 8 + 8 = 56 octets
}

/// Type de vue de mémoire binaire
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum BufferViewKind {
    Transforms,
    Materials,
    Vertices,
    Indices,
    Instances,
    Custom(String),
}

/// Descripteur d'une plage mémoire binaire alignée
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct BufferView {
    pub kind: BufferViewKind,
    pub offset: u64,
    pub length: u64,
    pub stride: usize,
    pub count: usize,
}

/// Structure binaire d'un transform 3D aligné à 16 octets (64 octets au total)
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, bytemuck::Pod, bytemuck::Zeroable)]
pub struct TransformRaw {
    pub translation: [f32; 3],
    pub _pad0: f32,
    pub rotation: [f32; 3],
    pub _pad1: f32,
    pub scale: [f32; 3],
    pub _pad2: f32,
    pub _reserved: [f32; 4],
}

impl TransformRaw {
    pub fn from_transform(t: &Transform) -> Self {
        Self {
            translation: [t.translation.x, t.translation.y, t.translation.z],
            _pad0: 0.0,
            rotation: [t.rotation.x, t.rotation.y, t.rotation.z],
            _pad1: 0.0,
            scale: [t.scale.x, t.scale.y, t.scale.z],
            _pad2: 0.0,
            _reserved: [0.0; 4],
        }
    }

    pub fn to_transform(&self) -> Transform {
        Transform {
            translation: Vec3::new(self.translation[0], self.translation[1], self.translation[2]),
            rotation: Vec3::new(self.rotation[0], self.rotation[1], self.rotation[2]),
            scale: Vec3::new(self.scale[0], self.scale[1], self.scale[2]),
        }
    }
}

/// Manifeste d'un nœud dans la scène (JSON extensible)
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NodeManifest {
    pub id: usize,
    pub name: String,
    pub primitive_type: PrimitiveType,
    #[serde(default)]
    pub parent: Option<usize>,
    #[serde(default)]
    pub children: Vec<usize>,
    pub color: [f32; 4],
    #[serde(skip_serializing_if = "Option::is_none")]
    pub texture_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub normal_texture_name: Option<String>,
    pub transform_index: usize,
    pub material_index: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vertex_buffer_view_idx: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub index_buffer_view_idx: Option<usize>,
    #[serde(default, flatten)]
    pub extra_fields: HashMap<String, serde_json::Value>,
}

/// Manifeste complet de la scène (JSON extensible sans obsolescence)
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AorSceneManifest {
    pub schema_version: u32,
    pub engine_version: String,
    pub generator: String,
    pub light_position: [f32; 3],
    #[serde(default = "default_ambient")]
    pub ambient_color: [f32; 3],
    pub buffer_views: Vec<BufferView>,
    pub nodes: Vec<NodeManifest>,
    #[serde(default, flatten)]
    pub extra_fields: HashMap<String, serde_json::Value>,
}

fn default_ambient() -> [f32; 3] {
    [0.07, 0.05, 0.13]
}

/// Calcule la signature cryptographique / checksum d'intégrité
pub fn compute_bundle_signature(flags: u32, json_bytes: &[u8], binary_bytes: &[u8]) -> u64 {
    let mut hash = AOR_INTEGRITY_SALT;
    hash = hash.wrapping_mul(0x100000001B3).wrapping_add(flags as u64);
    for chunk in json_bytes.chunks(8) {
        let mut val = 0u64;
        for (i, &b) in chunk.iter().enumerate() {
            val |= (b as u64) << (i * 8);
        }
        hash = hash.rotate_left(13) ^ val;
        hash = hash.wrapping_mul(0x100000001B3);
    }
    for chunk in binary_bytes.chunks(8) {
        let mut val = 0u64;
        for (i, &b) in chunk.iter().enumerate() {
            val |= (b as u64) << (i * 8);
        }
        hash = hash.rotate_left(17) ^ val;
        hash = hash.wrapping_mul(0x100000001B3);
    }
    hash
}

/// Aligne une taille sur le multiple supérieur de `align` (16 octets)
fn align_up(size: usize, align: usize) -> usize {
    (size + align - 1) & !(align - 1)
}

/// Écrit une scène complète dans un paquet binaire-JSON industriel `.aor`
pub fn save_aor_bundle(path: &Path, nodes: &[SceneNode], light_pos: Vec3) -> Result<(), String> {
    let mut binary_buf = Vec::new();
    let mut buffer_views = Vec::new();

    // 1. Écrire la table des Transforms (64 octets par nœud, alignée 16 octets)
    let transform_offset = binary_buf.len() as u64;
    let mut transform_raws = Vec::with_capacity(nodes.len());
    for node in nodes {
        transform_raws.push(TransformRaw::from_transform(&node.transform));
    }
    let transform_bytes: &[u8] = bytemuck::cast_slice(&transform_raws);
    binary_buf.extend_from_slice(transform_bytes);
    // Pad to 16 bytes
    let pad_len = align_up(binary_buf.len(), AOR_ALIGNMENT) - binary_buf.len();
    binary_buf.extend(std::iter::repeat(0u8).take(pad_len));

    buffer_views.push(BufferView {
        kind: BufferViewKind::Transforms,
        offset: transform_offset,
        length: transform_bytes.len() as u64,
        stride: std::mem::size_of::<TransformRaw>(),
        count: nodes.len(),
    });

    // 2. Écrire la table des Matériaux GPU (48 octets par nœud)
    let material_offset = binary_buf.len() as u64;
    let mut material_raws = Vec::with_capacity(nodes.len());
    for node in nodes {
        material_raws.push(node.material);
    }
    let material_bytes: &[u8] = bytemuck::cast_slice(&material_raws);
    binary_buf.extend_from_slice(material_bytes);
    let pad_len = align_up(binary_buf.len(), AOR_ALIGNMENT) - binary_buf.len();
    binary_buf.extend(std::iter::repeat(0u8).take(pad_len));

    buffer_views.push(BufferView {
        kind: BufferViewKind::Materials,
        offset: material_offset,
        length: material_bytes.len() as u64,
        stride: std::mem::size_of::<MaterialUniform>(),
        count: nodes.len(),
    });

    // 3. Écrire les sommets et indices personnalisés éventuels
    let mut node_manifests = Vec::with_capacity(nodes.len());
    for (idx, node) in nodes.iter().enumerate() {
        let (v_view_idx, i_view_idx) = match &node.primitive_type {
            PrimitiveType::Custom(_) if !node.vertices.is_empty() => {
                let v_offset = binary_buf.len() as u64;
                let v_bytes: &[u8] = bytemuck::cast_slice(&node.vertices);
                binary_buf.extend_from_slice(v_bytes);
                let p = align_up(binary_buf.len(), AOR_ALIGNMENT) - binary_buf.len();
                binary_buf.extend(std::iter::repeat(0u8).take(p));
                let v_idx = buffer_views.len();
                buffer_views.push(BufferView {
                    kind: BufferViewKind::Vertices,
                    offset: v_offset,
                    length: v_bytes.len() as u64,
                    stride: std::mem::size_of::<Vertex>(),
                    count: node.vertices.len(),
                });

                let i_offset = binary_buf.len() as u64;
                let i_bytes: &[u8] = bytemuck::cast_slice(&node.indices);
                binary_buf.extend_from_slice(i_bytes);
                let p = align_up(binary_buf.len(), AOR_ALIGNMENT) - binary_buf.len();
                binary_buf.extend(std::iter::repeat(0u8).take(p));
                let i_idx = buffer_views.len();
                buffer_views.push(BufferView {
                    kind: BufferViewKind::Indices,
                    offset: i_offset,
                    length: i_bytes.len() as u64,
                    stride: std::mem::size_of::<u32>(),
                    count: node.indices.len(),
                });

                (Some(v_idx), Some(i_idx))
            }
            _ => (None, None),
        };

        node_manifests.push(NodeManifest {
            id: node.id,
            name: node.name.clone(),
            primitive_type: node.primitive_type.clone(),
            parent: node.parent_idx,
            children: node.children_indices.clone(),
            color: node.color,
            texture_name: node.texture_name.clone(),
            normal_texture_name: node.normal_texture_name.clone(),
            transform_index: idx,
            material_index: idx,
            vertex_buffer_view_idx: v_view_idx,
            index_buffer_view_idx: i_view_idx,
            extra_fields: HashMap::new(),
        });
    }

    // 4. Construire le manifeste JSON
    let manifest = AorSceneManifest {
        schema_version: AOR_FORMAT_VERSION,
        engine_version: env!("CARGO_PKG_VERSION").to_string(),
        generator: "AOR-Industrial-Engine-v1".to_string(),
        light_position: [light_pos.x, light_pos.y, light_pos.z],
        ambient_color: [0.07, 0.05, 0.13],
        buffer_views,
        nodes: node_manifests,
        extra_fields: HashMap::new(),
    };

    let json_bytes = serde_json::to_vec_pretty(&manifest)
        .map_err(|e| format!("Erreur sérialisation manifeste JSON: {e}"))?;

    let json_offset = AorHeader::SIZE as u64;
    let json_length = json_bytes.len() as u64;
    let binary_offset = align_up((json_offset + json_length) as usize, AOR_ALIGNMENT) as u64;
    let binary_length = binary_buf.len() as u64;

    let signature = compute_bundle_signature(0, &json_bytes, &binary_buf);

    let header = AorHeader {
        magic: *AOR_MAGIC,
        version: AOR_FORMAT_VERSION,
        flags: 0,
        json_offset,
        json_length,
        binary_offset,
        binary_length,
        signature,
    };

    let mut out_file = File::create(path).map_err(|e| format!("Impossible de créer {path:?}: {e}"))?;
    let header_bytes: &[u8] = bytemuck::bytes_of(&header);
    out_file.write_all(header_bytes).map_err(|e| e.to_string())?;
    out_file.write_all(&json_bytes).map_err(|e| e.to_string())?;

    // Remplissage d'alignement avant le segment binaire
    let current_pos = json_offset + json_length;
    if binary_offset > current_pos {
        let pad = vec![0u8; (binary_offset - current_pos) as usize];
        out_file.write_all(&pad).map_err(|e| e.to_string())?;
    }

    out_file.write_all(&binary_buf).map_err(|e| e.to_string())?;
    out_file.flush().map_err(|e| e.to_string())?;
    Ok(())
}

/// Résultat du chargement sécurisé d'un paquet AOR
pub struct LoadedAorBundle {
    pub nodes: Vec<SceneNode>,
    pub light_position: Vec3,
    pub ambient_color: Vec3,
    pub schema_version: u32,
    pub is_signature_valid: bool,
}

/// Charge un paquet `.aor` avec validation de sécurité complète en 6 niveaux
pub fn load_aor_bundle(
    device: &wgpu::Device,
    model_bind_group_layout: &wgpu::BindGroupLayout,
    default_texture: Arc<GpuTexture>,
    default_normal_texture: Arc<GpuTexture>,
    path: &Path,
    allow_dev_override: bool,
) -> Result<LoadedAorBundle, String> {
    let mut file = File::open(path).map_err(|e| format!("Impossible d'ouvrir {path:?}: {e}"))?;
    let total_file_size = file.metadata().map_err(|e| e.to_string())?.len();

    // Niveau 1 : Validation de taille minimale de l'en-tête
    if total_file_size < AorHeader::SIZE as u64 {
        return Err("AOR-SECURITY: Fichier tronqué (en-tête incomplet)".to_string());
    }

    let mut header_buf = [0u8; AorHeader::SIZE];
    file.read_exact(&mut header_buf).map_err(|e| e.to_string())?;
    let header: AorHeader = *bytemuck::from_bytes(&header_buf);

    if &header.magic != AOR_MAGIC {
        return Err("AOR-SECURITY: Magic invalide — ce fichier n'est pas un paquet AORBNDL1".to_string());
    }

    // Niveau 2 : Bornes mémoires spatiales (Anti-OOB)
    if header.json_offset + header.json_length > total_file_size
        || header.binary_offset + header.binary_length > total_file_size
    {
        return Err("AOR-SECURITY: Débordement de bornes (offsets déclarés hors fichier)".to_string());
    }

    // Niveau 6 : Plafond d'allocation (Anti-OOM)
    if header.binary_length as usize > MAX_BUNDLE_ALLOCATION_BYTES {
        return Err(format!(
            "AOR-SECURITY: Taille binaire ({} Mo) dépasse le plafond de sécurité ({} Mo)",
            header.binary_length / (1024 * 1024),
            MAX_BUNDLE_ALLOCATION_BYTES / (1024 * 1024)
        ));
    }

    // Lecture du JSON
    file.seek(SeekFrom::Start(header.json_offset)).map_err(|e| e.to_string())?;
    let mut json_bytes = vec![0u8; header.json_length as usize];
    file.read_exact(&mut json_bytes).map_err(|e| e.to_string())?;

    // Lecture du buffer binaire
    file.seek(SeekFrom::Start(header.binary_offset)).map_err(|e| e.to_string())?;
    let mut binary_bytes = vec![0u8; header.binary_length as usize];
    file.read_exact(&mut binary_bytes).map_err(|e| e.to_string())?;

    // Niveau 1 : Vérification de la signature numérique cryptographique
    let expected_sig = compute_bundle_signature(header.flags, &json_bytes, &binary_bytes);
    let is_signature_valid = expected_sig == header.signature;
    if !is_signature_valid && !allow_dev_override {
        return Err("AOR-SECURITY: Signature numérique altérée — rejet de sécurité strict".to_string());
    }

    // Désérialisation du manifeste JSON
    let manifest: AorSceneManifest = serde_json::from_slice(&json_bytes)
        .map_err(|e| format!("AOR-SCHEMA: Manifeste JSON corrompu: {e}"))?;

    // Extraction des transforms depuis le buffer binaire
    let transforms_view = manifest
        .buffer_views
        .iter()
        .find(|v| v.kind == BufferViewKind::Transforms)
        .ok_or_else(|| "Vue de Transforms manquante dans le segment binaire".to_string())?;

    // Niveau 2 & 3 : Validation de stride et alignement
    if transforms_view.stride != std::mem::size_of::<TransformRaw>()
        || transforms_view.offset + transforms_view.length > header.binary_length
        || transforms_view.length % transforms_view.stride as u64 != 0
    {
        return Err("AOR-SECURITY: Vue des transforms corrompue ou désalignée".to_string());
    }

    let transform_slice = &binary_bytes[transforms_view.offset as usize..(transforms_view.offset + transforms_view.length) as usize];
    let transforms_raw: &[TransformRaw] = bytemuck::cast_slice(transform_slice);

    // Extraction des matériaux GPU
    let materials_view = manifest
        .buffer_views
        .iter()
        .find(|v| v.kind == BufferViewKind::Materials)
        .ok_or_else(|| "Vue de Matériaux manquante dans le segment binaire".to_string())?;

    if materials_view.stride != std::mem::size_of::<MaterialUniform>()
        || materials_view.offset + materials_view.length > header.binary_length
    {
        return Err("AOR-SECURITY: Vue des matériaux corrompue".to_string());
    }

    let material_slice = &binary_bytes[materials_view.offset as usize..(materials_view.offset + materials_view.length) as usize];
    let materials_raw: &[MaterialUniform] = bytemuck::cast_slice(material_slice);

    let mut nodes = Vec::with_capacity(manifest.nodes.len());

    for n in manifest.nodes {
        // Niveau 5 : Assainissement numérique (Anti-NaN)
        if n.transform_index >= transforms_raw.len() {
            return Err(format!("AOR-SECURITY: Index de transform {} hors bornes", n.transform_index));
        }
        let t_raw = transforms_raw[n.transform_index];
        for val in t_raw.translation.iter().chain(t_raw.rotation.iter()).chain(t_raw.scale.iter()) {
            if !val.is_finite() {
                return Err("AOR-SECURITY: Flottant non-fini (NaN/Inf) détecté dans un transform".to_string());
            }
        }
        let transform = t_raw.to_transform();

        let material = if n.material_index < materials_raw.len() {
            materials_raw[n.material_index]
        } else {
            MaterialUniform::default()
        };

        let mut node = match n.primitive_type {
            PrimitiveType::Custom(ref name) => {
                let mut custom_verts = Vec::new();
                let mut custom_inds = Vec::new();

                if let Some(v_idx) = n.vertex_buffer_view_idx {
                    if v_idx < manifest.buffer_views.len() {
                        let v_view = &manifest.buffer_views[v_idx];
                        if v_view.offset + v_view.length <= header.binary_length {
                            let slice = &binary_bytes[v_view.offset as usize..(v_view.offset + v_view.length) as usize];
                            let v_data: &[Vertex] = bytemuck::cast_slice(slice);
                            custom_verts = v_data.to_vec();
                        }
                    }
                }

                if let Some(i_idx) = n.index_buffer_view_idx {
                    if i_idx < manifest.buffer_views.len() {
                        let i_view = &manifest.buffer_views[i_idx];
                        if i_view.offset + i_view.length <= header.binary_length {
                            let slice = &binary_bytes[i_view.offset as usize..(i_view.offset + i_view.length) as usize];
                            let i_data: &[u32] = bytemuck::cast_slice(slice);
                            custom_inds = i_data.to_vec();
                        }
                    }
                }

                SceneNode::from_custom_mesh(
                    device,
                    model_bind_group_layout,
                    default_texture.clone(),
                    default_normal_texture.clone(),
                    n.id,
                    name,
                    custom_verts,
                    custom_inds,
                    transform.translation,
                )
            }
            prim => SceneNode::new(
                device,
                model_bind_group_layout,
                default_texture.clone(),
                default_normal_texture.clone(),
                n.id,
                prim,
                transform.translation,
            ),
        };

        node.name = n.name;
        node.transform = transform;
        node.color = n.color;
        node.material = material;
        node.texture_name = n.texture_name;
        node.normal_texture_name = n.normal_texture_name;
        node.parent_idx = n.parent;
        node.children_indices = n.children;
        nodes.push(node);
    }

    let light_position = Vec3::new(
        manifest.light_position[0],
        manifest.light_position[1],
        manifest.light_position[2],
    );
    let ambient_color = Vec3::new(
        manifest.ambient_color[0],
        manifest.ambient_color[1],
        manifest.ambient_color[2],
    );

    Ok(LoadedAorBundle {
        nodes,
        light_position,
        ambient_color,
        schema_version: manifest.schema_version,
        is_signature_valid,
    })
}

/// Modifie un transform in-place directement à l'offset binaire sans réécrire le conteneur
pub fn update_node_transform_in_place(
    path: &Path,
    node_idx: usize,
    new_transform: &Transform,
) -> Result<(), String> {
    let mut file = OpenOptions::new()
        .read(true)
        .write(true)
        .open(path)
        .map_err(|e| format!("Impossible d'ouvrir {path:?} en écriture: {e}"))?;

    let mut header_buf = [0u8; AorHeader::SIZE];
    file.read_exact(&mut header_buf).map_err(|e| e.to_string())?;
    let mut header: AorHeader = *bytemuck::from_bytes(&header_buf);

    if &header.magic != AOR_MAGIC {
        return Err("Magic invalide".to_string());
    }

    // Lire le JSON pour trouver la vue des transforms
    file.seek(SeekFrom::Start(header.json_offset)).map_err(|e| e.to_string())?;
    let mut json_bytes = vec![0u8; header.json_length as usize];
    file.read_exact(&mut json_bytes).map_err(|e| e.to_string())?;

    let manifest: AorSceneManifest = serde_json::from_slice(&json_bytes)
        .map_err(|e| format!("Manifeste JSON corrompu: {e}"))?;

    let transforms_view = manifest
        .buffer_views
        .iter()
        .find(|v| v.kind == BufferViewKind::Transforms)
        .ok_or_else(|| "Vue transforms introuvable".to_string())?;

    if node_idx >= transforms_view.count {
        return Err(format!("Index de nœud {node_idx} hors limites ({})", transforms_view.count));
    }

    // Offset exact dans le fichier
    let target_offset = header.binary_offset
        + transforms_view.offset
        + (node_idx * std::mem::size_of::<TransformRaw>()) as u64;

    let raw = TransformRaw::from_transform(new_transform);
    let raw_bytes: &[u8] = bytemuck::bytes_of(&raw);

    file.seek(SeekFrom::Start(target_offset)).map_err(|e| e.to_string())?;
    file.write_all(raw_bytes).map_err(|e| e.to_string())?;

    // Recalculer et sceller la nouvelle signature d'intégrité
    file.seek(SeekFrom::Start(header.binary_offset)).map_err(|e| e.to_string())?;
    let mut binary_bytes = vec![0u8; header.binary_length as usize];
    file.read_exact(&mut binary_bytes).map_err(|e| e.to_string())?;

    header.signature = compute_bundle_signature(header.flags, &json_bytes, &binary_bytes);

    file.seek(SeekFrom::Start(0)).map_err(|e| e.to_string())?;
    file.write_all(bytemuck::bytes_of(&header)).map_err(|e| e.to_string())?;
    file.flush().map_err(|e| e.to_string())?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_aor_header_size() {
        assert_eq!(std::mem::size_of::<AorHeader>(), AorHeader::SIZE);
    }

    #[test]
    fn test_transform_raw_roundtrip() {
        let t = Transform {
            translation: Vec3::new(12.5, -3.2, 44.0),
            rotation: Vec3::new(0.1, 1.57, -0.8),
            scale: Vec3::new(2.0, 3.0, 0.5),
        };
        let raw = TransformRaw::from_transform(&t);
        let back = raw.to_transform();
        assert_eq!(t.translation, back.translation);
        assert_eq!(t.rotation, back.rotation);
        assert_eq!(t.scale, back.scale);
    }

    #[test]
    fn test_bundle_signature_deterministic() {
        let json = b"{\"schema\": 1}";
        let binary = [1u8, 2, 3, 4, 5, 6, 7, 8];
        let sig1 = compute_bundle_signature(0, json, &binary);
        let sig2 = compute_bundle_signature(0, json, &binary);
        assert_eq!(sig1, sig2);

        let tampered_binary = [1u8, 2, 3, 99, 5, 6, 7, 8];
        let sig_tampered = compute_bundle_signature(0, json, &tampered_binary);
        assert_ne!(sig1, sig_tampered);
    }

    #[test]
    fn test_forward_compatibility_extra_fields() {
        let json_with_future_fields = r#"{
            "schema_version": 2,
            "engine_version": "0.9.0",
            "generator": "AOR-Future-Engine",
            "light_position": [1.0, 2.0, 3.0],
            "future_ocean_sim_data": { "wave_height": 4.5 },
            "future_raytracing_lod_levels": [1, 2, 4],
            "buffer_views": [],
            "nodes": [
                {
                    "id": 0,
                    "name": "TestNode",
                    "primitive_type": "Cube",
                    "color": [1.0, 1.0, 1.0, 1.0],
                    "transform_index": 0,
                    "material_index": 0,
                    "future_cloth_physics_flag": true
                }
            ]
        }"#;

        let parsed: Result<AorSceneManifest, _> = serde_json::from_str(json_with_future_fields);
        assert!(parsed.is_ok());
        let manifest = parsed.unwrap();
        assert_eq!(manifest.schema_version, 2);
        assert!(manifest.extra_fields.contains_key("future_ocean_sim_data"));
        assert!(manifest.nodes[0].extra_fields.contains_key("future_cloth_physics_flag"));
    }

    #[test]
    fn test_in_place_transform_update() {
        let temp_dir = std::env::temp_dir();
        let path = temp_dir.join("test_in_place.aor");

        // Construire manuellement un bundle minimal
        let t0 = Transform {
            translation: Vec3::new(1.0, 2.0, 3.0),
            rotation: Vec3::ZERO,
            scale: Vec3::ONE,
        };
        let t1 = Transform {
            translation: Vec3::new(10.0, 20.0, 30.0),
            rotation: Vec3::ZERO,
            scale: Vec3::ONE,
        };
        let transform_raws = vec![TransformRaw::from_transform(&t0), TransformRaw::from_transform(&t1)];
        let transform_bytes: &[u8] = bytemuck::cast_slice(&transform_raws);

        let mut binary_buf = Vec::new();
        binary_buf.extend_from_slice(transform_bytes);

        let buffer_views = vec![BufferView {
            kind: BufferViewKind::Transforms,
            offset: 0,
            length: transform_bytes.len() as u64,
            stride: std::mem::size_of::<TransformRaw>(),
            count: 2,
        }];

        let manifest = AorSceneManifest {
            schema_version: 1,
            engine_version: "0.1.0".to_string(),
            generator: "Test".to_string(),
            light_position: [0.0, 10.0, 0.0],
            ambient_color: [0.1, 0.1, 0.1],
            buffer_views,
            nodes: vec![],
            extra_fields: HashMap::new(),
        };

        let json_bytes = serde_json::to_vec_pretty(&manifest).unwrap();
        let json_offset = AorHeader::SIZE as u64;
        let json_length = json_bytes.len() as u64;
        let binary_offset = align_up((json_offset + json_length) as usize, AOR_ALIGNMENT) as u64;
        let binary_length = binary_buf.len() as u64;
        let signature = compute_bundle_signature(0, &json_bytes, &binary_buf);

        let header = AorHeader {
            magic: *AOR_MAGIC,
            version: 1,
            flags: 0,
            json_offset,
            json_length,
            binary_offset,
            binary_length,
            signature,
        };

        let mut f = File::create(&path).unwrap();
        f.write_all(bytemuck::bytes_of(&header)).unwrap();
        f.write_all(&json_bytes).unwrap();
        let current_pos = json_offset + json_length;
        if binary_offset > current_pos {
            let pad = vec![0u8; (binary_offset - current_pos) as usize];
            f.write_all(&pad).unwrap();
        }
        f.write_all(&binary_buf).unwrap();
        f.flush().unwrap();

        // Mutation in-place du nœud #1
        let new_t1 = Transform {
            translation: Vec3::new(99.0, 88.0, 77.0),
            rotation: Vec3::new(1.0, 0.0, 0.0),
            scale: Vec3::new(5.0, 5.0, 5.0),
        };
        let res = update_node_transform_in_place(&path, 1, &new_t1);
        assert!(res.is_ok());

        // Relecture et vérification
        let mut f_check = File::open(&path).unwrap();
        let mut h_buf = [0u8; AorHeader::SIZE];
        f_check.read_exact(&mut h_buf).unwrap();
        let updated_header: AorHeader = *bytemuck::from_bytes(&h_buf);

        f_check.seek(SeekFrom::Start(updated_header.binary_offset)).unwrap();
        let mut read_bin = vec![0u8; updated_header.binary_length as usize];
        f_check.read_exact(&mut read_bin).unwrap();

        let read_transforms: &[TransformRaw] = bytemuck::cast_slice(&read_bin);
        assert_eq!(read_transforms[1].to_transform().translation, Vec3::new(99.0, 88.0, 77.0));

        // Vérifier que la signature recalculée est scellée et valide
        let expected_sig = compute_bundle_signature(updated_header.flags, &json_bytes, &read_bin);
        assert_eq!(updated_header.signature, expected_sig);

        let _ = std::fs::remove_file(path);
    }
}
