// [WFGY] Zone: SAFE | λ: 0.25 | Fallbacks: 0/None | Action: 3D Model Loader (OBJ with Tangents) and Procedural Asset Generator with Tangent Space
#![allow(dead_code)]
use std::path::Path;
use glam::Vec3;
use crate::scene::mesh::{compute_tangents, Vertex};

/// Charge un modèle 3D au format Wavefront OBJ depuis un fichier
pub fn load_obj_file(path: &Path) -> Result<(String, Vec<Vertex>, Vec<u32>), String> {
    let load_options = tobj::LoadOptions {
        triangulate: true,
        single_index: true,
        ..Default::default()
    };

    let (models, _materials) = tobj::load_obj(path, &load_options)
        .map_err(|e| format!("Erreur de chargement OBJ {:?} : {}", path, e))?;

    if models.is_empty() {
        return Err("Le fichier OBJ ne contient aucun maillage valide".to_string());
    }

    let model_name = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("Modèle 3D")
        .to_string();

    let (vertices, indices) = merge_tobj_models(&models);
    Ok((model_name, vertices, indices))
}

pub struct LoadedModel3D {
    pub name: String,
    pub vertices: Vec<Vertex>,
    pub indices: Vec<u32>,
    pub albedo_texture: Option<(u32, u32, Vec<u8>, String)>,
    pub normal_texture: Option<(u32, u32, Vec<u8>, String)>,
    pub roughness: f32,
    pub metallic: f32,
    pub base_color: [f32; 4],
    pub transmission: f32,
    pub ior: f32,
    pub emission_color: [f32; 4],
}

/// Convertit les données d'image glTF en buffer RGBA8 standard
fn gltf_image_to_rgba8(data: &gltf::image::Data) -> (u32, u32, Vec<u8>) {
    let width = data.width;
    let height = data.height;
    let total_pixels = (width * height) as usize;

    let rgba = match data.format {
        gltf::image::Format::R8G8B8A8 => data.pixels.clone(),
        gltf::image::Format::R8G8B8 => {
            let mut out = Vec::with_capacity(total_pixels * 4);
            for chunk in data.pixels.chunks_exact(3) {
                out.push(chunk[0]);
                out.push(chunk[1]);
                out.push(chunk[2]);
                out.push(255);
            }
            out
        }
        gltf::image::Format::R8 => {
            let mut out = Vec::with_capacity(total_pixels * 4);
            for &r in &data.pixels {
                out.push(r);
                out.push(r);
                out.push(r);
                out.push(255);
            }
            out
        }
        gltf::image::Format::R8G8 => {
            let mut out = Vec::with_capacity(total_pixels * 4);
            for chunk in data.pixels.chunks_exact(2) {
                out.push(chunk[0]);
                out.push(chunk[1]);
                out.push(0);
                out.push(255);
            }
            out
        }
        _ => {
            let mut out = Vec::with_capacity(total_pixels * 4);
            for chunk in data.pixels.chunks_exact(4) {
                out.push(chunk[0]);
                out.push(chunk[1]);
                out.push(chunk[2]);
                out.push(chunk[3]);
            }
            if out.is_empty() {
                out.resize(total_pixels * 4, 255);
            }
            out
        }
    };
    (width, height, rgba)
}

/// Cherche une texture sur disque (dossier courant ou sous-dossier textures/)
fn find_texture_in_dir(dir: &Path, patterns: &[&str]) -> Option<std::path::PathBuf> {
    let check_entry = |path: &Path| -> bool {
        if path.is_file() {
            if let Some(fname) = path.file_name().and_then(|n| n.to_str()).map(|s| s.to_lowercase()) {
                if (fname.ends_with(".png") || fname.ends_with(".jpg") || fname.ends_with(".jpeg") || fname.ends_with(".webp"))
                    && patterns.iter().any(|pat| fname.contains(pat)) {
                    return true;
                }
            }
        }
        false
    };

    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let p = entry.path();
            if check_entry(&p) {
                return Some(p);
            }
        }
    }

    let textures_dir = dir.join("textures");
    if textures_dir.is_dir() {
        if let Ok(entries) = std::fs::read_dir(&textures_dir) {
            for entry in entries.flatten() {
                let p = entry.path();
                if check_entry(&p) {
                    return Some(p);
                }
            }
        }
    }

    None
}

/// Charge un modèle 3D glTF / GLB avec géométrie et extraction complète des textures PBR
pub fn load_gltf_file_full(path: &Path) -> Result<LoadedModel3D, String> {
    let (document, buffers, images) = gltf::import(path)
        .map_err(|e| format!("Erreur de chargement glTF/GLB {:?} : {}", path, e))?;

    let model_name = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("Modèle glTF")
        .to_string();

    let mut raw_positions: Vec<Vec3> = Vec::new();
    let mut raw_normals: Vec<Vec3> = Vec::new();
    let mut raw_uvs: Vec<[f32; 2]> = Vec::new();
    let mut indices: Vec<u32> = Vec::new();

    let mut albedo_texture: Option<(u32, u32, Vec<u8>, String)> = None;
    let mut normal_texture: Option<(u32, u32, Vec<u8>, String)> = None;
    let mut roughness = 0.5f32;
    let mut metallic = 0.0f32;
    let mut base_color = [1.0f32, 1.0, 1.0, 1.0];
    let mut transmission = 0.0f32;
    let mut ior = 1.5f32;
    let mut emission_color = [0.0f32, 0.0, 0.0, 0.0];

    for mesh in document.meshes() {
        for primitive in mesh.primitives() {
            let reader = primitive.reader(|buffer| Some(&buffers[buffer.index()]));

            let base_vertex = raw_positions.len() as u32;

            if let Some(pos_iter) = reader.read_positions() {
                for pos in pos_iter {
                    raw_positions.push(Vec3::new(pos[0], pos[1], pos[2]));
                }
            }

            if let Some(norm_iter) = reader.read_normals() {
                for norm in norm_iter {
                    raw_normals.push(Vec3::new(norm[0], norm[1], norm[2]));
                }
            }

            if let Some(uv_iter) = reader.read_tex_coords(0) {
                for uv in uv_iter.into_f32() {
                    raw_uvs.push([uv[0], uv[1]]);
                }
            }

            if let Some(indices_iter) = reader.read_indices() {
                for idx in indices_iter.into_u32() {
                    indices.push(base_vertex + idx);
                }
            }

            // Extraction des paramètres PBR du matériau
            let material = primitive.material();
            let pbr = material.pbr_metallic_roughness();
            base_color = pbr.base_color_factor();
            roughness = pbr.roughness_factor();
            metallic = pbr.metallic_factor();

            if let Some(trans) = material.transmission() {
                transmission = trans.transmission_factor();
            }
            if let Some(ior_val) = material.ior() {
                ior = ior_val;
            }
            let emissive_factor = material.emissive_factor();
            if emissive_factor[0] > 0.0 || emissive_factor[1] > 0.0 || emissive_factor[2] > 0.0 {
                let emissive_strength = material.emissive_strength().unwrap_or(1.0);
                emission_color = [
                    emissive_factor[0] * emissive_strength,
                    emissive_factor[1] * emissive_strength,
                    emissive_factor[2] * emissive_strength,
                    emissive_strength,
                ];
            }

            // 1. Texture Albedo / Diffuse
            if albedo_texture.is_none() {
                if let Some(tex_info) = pbr.base_color_texture() {
                    let img_idx = tex_info.texture().source().index();
                    if img_idx < images.len() {
                        let img_data = &images[img_idx];
                        let (w, h, rgba) = gltf_image_to_rgba8(img_data);
                        albedo_texture = Some((w, h, rgba, format!("{}_diffuse.png", model_name)));
                    }
                }
            }

            // 2. Texture Normale
            if normal_texture.is_none() {
                if let Some(norm_info) = material.normal_texture() {
                    let img_idx = norm_info.texture().source().index();
                    if img_idx < images.len() {
                        let img_data = &images[img_idx];
                        let (w, h, rgba) = gltf_image_to_rgba8(img_data);
                        normal_texture = Some((w, h, rgba, format!("{}_normal.png", model_name)));
                    }
                }
            }
        }
    }

    if raw_positions.is_empty() {
        return Err("Le fichier glTF/GLB ne contient aucune géométrie valide".to_string());
    }

    // Recherche de textures sur disque si non embarquées dans le conteneur
    if let Some(parent) = path.parent() {
        if albedo_texture.is_none() {
            if let Some(tex_path) = find_texture_in_dir(parent, &["diff", "albedo", "basecolor", "base_color", "col", "color"]) {
                if let Ok(img) = image::open(&tex_path) {
                    let rgba = img.to_rgba8();
                    let (w, h) = (rgba.width(), rgba.height());
                    let fname = tex_path.file_name().and_then(|n| n.to_str()).unwrap_or("diffuse.png").to_string();
                    albedo_texture = Some((w, h, rgba.into_raw(), fname));
                }
            }
        }

        if normal_texture.is_none() {
            if let Some(tex_path) = find_texture_in_dir(parent, &["nor_gl", "normal", "nor", "nrm"]) {
                if let Ok(img) = image::open(&tex_path) {
                    let rgba = img.to_rgba8();
                    let (w, h) = (rgba.width(), rgba.height());
                    let fname = tex_path.file_name().and_then(|n| n.to_str()).unwrap_or("normal.png").to_string();
                    normal_texture = Some((w, h, rgba.into_raw(), fname));
                }
            }
        }
    }

    // Normalisation des buffers
    while raw_normals.len() < raw_positions.len() {
        raw_normals.push(Vec3::Y);
    }
    while raw_uvs.len() < raw_positions.len() {
        raw_uvs.push([0.0, 0.0]);
    }

    if indices.is_empty() {
        indices = (0..raw_positions.len() as u32).collect();
    }

    // Calcul automatique des normales si manquantes
    let needs_normals = raw_normals.iter().all(|n| n.length_squared() < 1e-4);
    if needs_normals {
        raw_normals = compute_smooth_normals(&raw_positions, &indices);
    }

    // Normalisation géométrique
    let (normalized_positions, _scale) = normalize_mesh_geometry(&raw_positions);

    // Construction du buffer initial de sommets avec couleur de base
    let mut vertices: Vec<Vertex> = normalized_positions
        .into_iter()
        .zip(raw_normals)
        .zip(raw_uvs)
        .map(|((p, n), uv)| Vertex {
            position: [p.x, p.y, p.z],
            normal: [n.x, n.y, n.z],
            tangent: [1.0, 0.0, 0.0, 1.0],
            uv,
            color: base_color,
        })
        .collect();

    // Calcul universel des tangentes pour le normal mapping
    compute_tangents(&mut vertices, &indices);

    Ok(LoadedModel3D {
        name: model_name,
        vertices,
        indices,
        albedo_texture,
        normal_texture,
        roughness,
        metallic,
        base_color,
        transmission,
        ior,
        emission_color,
    })
}

/// Charge un modèle 3D au format standard glTF / GLB (.gltf, .glb)
pub fn load_gltf_file(path: &Path) -> Result<(String, Vec<Vertex>, Vec<u32>), String> {
    let full = load_gltf_file_full(path)?;
    Ok((full.name, full.vertices, full.indices))
}

/// Charge un modèle 3D au format Wavefront OBJ avec recherche de textures associées
pub fn load_obj_file_full(path: &Path) -> Result<LoadedModel3D, String> {
    let (name, vertices, indices) = load_obj_file(path)?;

    let mut albedo_texture = None;
    let mut normal_texture = None;

    if let Some(parent) = path.parent() {
        if let Some(tex_path) = find_texture_in_dir(parent, &["diff", "albedo", "basecolor", "base_color", "col", "color"]) {
            if let Ok(img) = image::open(&tex_path) {
                let rgba = img.to_rgba8();
                let (w, h) = (rgba.width(), rgba.height());
                let fname = tex_path.file_name().and_then(|n| n.to_str()).unwrap_or("diffuse.png").to_string();
                albedo_texture = Some((w, h, rgba.into_raw(), fname));
            }
        }
        if let Some(tex_path) = find_texture_in_dir(parent, &["nor_gl", "normal", "nor", "nrm"]) {
            if let Ok(img) = image::open(&tex_path) {
                let rgba = img.to_rgba8();
                let (w, h) = (rgba.width(), rgba.height());
                let fname = tex_path.file_name().and_then(|n| n.to_str()).unwrap_or("normal.png").to_string();
                normal_texture = Some((w, h, rgba.into_raw(), fname));
            }
        }
    }

    Ok(LoadedModel3D {
        name,
        vertices,
        indices,
        albedo_texture,
        normal_texture,
        roughness: 0.5,
        metallic: 0.0,
        base_color: [1.0, 1.0, 1.0, 1.0],
        transmission: 0.0,
        ior: 1.5,
        emission_color: [0.0, 0.0, 0.0, 0.0],
    })
}

/// Charge n'importe quel modèle 3D supporté (.obj, .gltf, .glb) avec ses textures
pub fn load_3d_model_file_full(path: &Path) -> Result<LoadedModel3D, String> {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .map(|s| s.to_lowercase())
        .unwrap_or_default();

    match ext.as_str() {
        "gltf" | "glb" => load_gltf_file_full(path),
        "obj" => load_obj_file_full(path),
        _ => Err(format!("Format non supporté '.{}' (Formats gérés : .gltf, .glb, .obj)", ext)),
    }
}

/// Charge n'importe quel modèle 3D supporté (.obj, .gltf, .glb) selon son extension
pub fn load_3d_model_file(path: &Path) -> Result<(String, Vec<Vertex>, Vec<u32>), String> {
    let full = load_3d_model_file_full(path)?;
    Ok((full.name, full.vertices, full.indices))
}

/// Charge un modèle 3D au format OBJ depuis une chaîne de caractères en mémoire
pub fn load_obj_from_str(obj_data: &str, model_name: &str) -> Result<(String, Vec<Vertex>, Vec<u32>), String> {
    let mut reader = std::io::Cursor::new(obj_data);
    let load_options = tobj::LoadOptions {
        triangulate: true,
        single_index: true,
        ..Default::default()
    };

    let (models, _materials) = tobj::load_obj_buf(&mut reader, &load_options, |_| {
        Err(tobj::LoadError::GenericFailure)
    })
    .map_err(|e| format!("Erreur parsing OBJ : {}", e))?;

    if models.is_empty() {
        return Err("Données OBJ invalides ou vides".to_string());
    }

    let (vertices, indices) = merge_tobj_models(&models);
    Ok((model_name.to_string(), vertices, indices))
}

/// Fusionne et normalise l'ensemble des sous-maillages d'un fichier OBJ en calculant normales et tangentes
fn merge_tobj_models(models: &[tobj::Model]) -> (Vec<Vertex>, Vec<u32>) {
    let mut raw_positions: Vec<Vec3> = Vec::new();
    let mut raw_normals: Vec<Vec3> = Vec::new();
    let mut raw_uvs: Vec<[f32; 2]> = Vec::new();
    let mut indices: Vec<u32> = Vec::new();

    let mut index_offset = 0u32;

    for model in models {
        let mesh = &model.mesh;
        let pos_count = mesh.positions.len() / 3;
        let has_normals = !mesh.normals.is_empty() && mesh.normals.len() == mesh.positions.len();
        let has_uvs = !mesh.texcoords.is_empty() && mesh.texcoords.len() >= pos_count * 2;

        for i in 0..pos_count {
            let px = mesh.positions[i * 3];
            let py = mesh.positions[i * 3 + 1];
            let pz = mesh.positions[i * 3 + 2];
            raw_positions.push(Vec3::new(px, py, pz));

            if has_normals {
                let nx = mesh.normals[i * 3];
                let ny = mesh.normals[i * 3 + 1];
                let nz = mesh.normals[i * 3 + 2];
                raw_normals.push(Vec3::new(nx, ny, nz).normalize_or_zero());
            } else {
                raw_normals.push(Vec3::ZERO);
            }

            if has_uvs {
                let u = mesh.texcoords[i * 2];
                let v = 1.0 - mesh.texcoords[i * 2 + 1];
                raw_uvs.push([u, v]);
            } else {
                let u = (px * 0.5 + 0.5).fract();
                let v = (pz * 0.5 + 0.5).fract();
                raw_uvs.push([u, v]);
            }
        }

        for &idx in &mesh.indices {
            indices.push(idx + index_offset);
        }

        index_offset += pos_count as u32;
    }

    // Calcul automatique des normales douces si manquantes
    let needs_normals = raw_normals.iter().all(|n| n.length_squared() < 1e-4);
    if needs_normals {
        raw_normals = compute_smooth_normals(&raw_positions, &indices);
    }

    // Normalisation géométrique
    let (normalized_positions, _scale) = normalize_mesh_geometry(&raw_positions);

    // Construction du buffer initial de sommets
    let default_color = [0.85, 0.88, 0.95, 1.0];
    let mut vertices: Vec<Vertex> = normalized_positions
        .into_iter()
        .zip(raw_normals)
        .zip(raw_uvs)
        .map(|((p, n), uv)| Vertex {
            position: [p.x, p.y, p.z],
            normal: [n.x, n.y, n.z],
            tangent: [1.0, 0.0, 0.0, 1.0],
            uv,
            color: default_color,
        })
        .collect();

    // Calcul universel des tangentes
    compute_tangents(&mut vertices, &indices);

    (vertices, indices)
}

/// Calcule les normales douces moyennes pour chaque sommet
fn compute_smooth_normals(positions: &[Vec3], indices: &[u32]) -> Vec<Vec3> {
    let mut normals = vec![Vec3::ZERO; positions.len()];

    for chunk in indices.chunks(3) {
        if chunk.len() == 3 {
            let i0 = chunk[0] as usize;
            let i1 = chunk[1] as usize;
            let i2 = chunk[2] as usize;

            if i0 < positions.len() && i1 < positions.len() && i2 < positions.len() {
                let v0 = positions[i0];
                let v1 = positions[i1];
                let v2 = positions[i2];

                let edge1 = v1 - v0;
                let edge2 = v2 - v0;
                let face_normal = edge1.cross(edge2);

                normals[i0] += face_normal;
                normals[i1] += face_normal;
                normals[i2] += face_normal;
            }
        }
    }

    for n in &mut normals {
        *n = n.normalize_or_zero();
        if n.length_squared() < 1e-4 {
            *n = Vec3::Y;
        }
    }

    normals
}

/// Recentrage et normalisation géométrique d'un maillage
fn normalize_mesh_geometry(positions: &[Vec3]) -> (Vec<Vec3>, f32) {
    if positions.is_empty() {
        return (Vec::new(), 1.0);
    }

    let mut min = positions[0];
    let mut max = positions[0];

    for &p in positions.iter().skip(1) {
        min = min.min(p);
        max = max.max(p);
    }

    let center = (min + max) * 0.5;
    let size = max - min;
    let max_dim = size.x.max(size.y).max(size.z).max(0.0001);
    let scale_factor = 2.0 / max_dim;

    let normalized = positions
        .iter()
        .map(|&p| (p - center) * scale_factor)
        .collect();

    (normalized, scale_factor)
}

// --- GÉNÉRATEURS DE FORMES ET D'ASSETS AVANCÉS ---

/// Génère un maillage de Sphère UV avec tangentes
pub fn generate_sphere_data(radius: f32, rings: usize, sectors: usize) -> (Vec<Vertex>, Vec<u32>) {
    let mut vertices = Vec::new();
    let mut indices = Vec::new();
    let color = [0.4, 0.7, 1.0, 1.0];

    for r in 0..=rings {
        let v = (r as f32) / (rings as f32);
        let phi = std::f32::consts::PI * v;
        for s in 0..=sectors {
            let u = (s as f32) / (sectors as f32);
            let theta = 2.0 * std::f32::consts::PI * u;

            let x = radius * phi.sin() * theta.cos();
            let y = radius * phi.cos();
            let z = radius * phi.sin() * theta.sin();

            let normal = Vec3::new(x, y, z).normalize_or_zero();

            vertices.push(Vertex {
                position: [x, y, z],
                normal: [normal.x, normal.y, normal.z],
                tangent: [1.0, 0.0, 0.0, 1.0],
                uv: [u, v],
                color,
            });
        }
    }

    for r in 0..rings {
        for s in 0..sectors {
            let first = (r * (sectors + 1) + s) as u32;
            let second = first + (sectors + 1) as u32;

            indices.push(first);
            indices.push(second);
            indices.push(first + 1);

            indices.push(second);
            indices.push(second + 1);
            indices.push(first + 1);
        }
    }

    compute_tangents(&mut vertices, &indices);
    (vertices, indices)
}

/// Génère un maillage de Cylindre avec tangentes
pub fn generate_cylinder_data(radius: f32, height: f32, segments: usize) -> (Vec<Vertex>, Vec<u32>) {
    let mut vertices = Vec::new();
    let mut indices = Vec::new();
    let half_h = height * 0.5;
    let color = [0.95, 0.6, 0.3, 1.0];

    for i in 0..=segments {
        let u = (i as f32) / (segments as f32);
        let theta = 2.0 * std::f32::consts::PI * u;
        let x = radius * theta.cos();
        let z = radius * theta.sin();
        let normal = Vec3::new(x, 0.0, z).normalize_or_zero();

        // Sommet haut
        vertices.push(Vertex {
            position: [x, half_h, z],
            normal: [normal.x, 0.0, normal.z],
            tangent: [1.0, 0.0, 0.0, 1.0],
            uv: [u, 0.0],
            color,
        });
        // Sommet bas
        vertices.push(Vertex {
            position: [x, -half_h, z],
            normal: [normal.x, 0.0, normal.z],
            tangent: [1.0, 0.0, 0.0, 1.0],
            uv: [u, 1.0],
            color,
        });
    }

    for i in 0..segments {
        let i0 = (i * 2) as u32;
        let i1 = i0 + 1;
        let i2 = i0 + 2;
        let i3 = i0 + 3;

        indices.push(i0);
        indices.push(i1);
        indices.push(i2);

        indices.push(i2);
        indices.push(i1);
        indices.push(i3);
    }

    compute_tangents(&mut vertices, &indices);
    (vertices, indices)
}

/// Génère un maillage de Tore (Donut) avec tangentes
pub fn generate_torus_data(r_major: f32, r_minor: f32, radial_segs: usize, tubular_segs: usize) -> (Vec<Vertex>, Vec<u32>) {
    let mut vertices = Vec::new();
    let mut indices = Vec::new();
    let color = [0.9, 0.35, 0.6, 1.0];

    for i in 0..=radial_segs {
        let u = (i as f32) / (radial_segs as f32);
        let theta = 2.0 * std::f32::consts::PI * u;
        for j in 0..=tubular_segs {
            let v = (j as f32) / (tubular_segs as f32);
            let phi = 2.0 * std::f32::consts::PI * v;

            let x = (r_major + r_minor * phi.cos()) * theta.cos();
            let y = r_minor * phi.sin();
            let z = (r_major + r_minor * phi.cos()) * theta.sin();

            let center = Vec3::new(r_major * theta.cos(), 0.0, r_major * theta.sin());
            let normal = (Vec3::new(x, y, z) - center).normalize_or_zero();

            vertices.push(Vertex {
                position: [x, y, z],
                normal: [normal.x, normal.y, normal.z],
                tangent: [1.0, 0.0, 0.0, 1.0],
                uv: [u, v],
                color,
            });
        }
    }

    for i in 0..radial_segs {
        for j in 0..tubular_segs {
            let a = (i * (tubular_segs + 1) + j) as u32;
            let b = ((i + 1) * (tubular_segs + 1) + j) as u32;
            let c = b + 1;
            let d = a + 1;

            indices.push(a);
            indices.push(b);
            indices.push(d);

            indices.push(b);
            indices.push(c);
            indices.push(d);
        }
    }

    compute_tangents(&mut vertices, &indices);
    (vertices, indices)
}

/// Génère un Arbre Low-Poly stylisé avec tangentes
pub fn generate_lowpoly_tree_data() -> (Vec<Vertex>, Vec<u32>) {
    let mut vertices = Vec::new();
    let mut indices = Vec::new();

    let trunk_color = [0.45, 0.28, 0.15, 1.0];
    let leaf_color_dark = [0.15, 0.65, 0.3, 1.0];
    let leaf_color_light = [0.25, 0.8, 0.35, 1.0];

    // 1. Tronc (Cylindre 6 faces)
    let trunk_radius = 0.25;
    let trunk_height = 1.0;
    let segs = 6;

    for i in 0..=segs {
        let u = (i as f32) / (segs as f32);
        let theta = 2.0 * std::f32::consts::PI * u;
        let x = trunk_radius * theta.cos();
        let z = trunk_radius * theta.sin();
        let normal = Vec3::new(x, 0.0, z).normalize_or_zero();

        vertices.push(Vertex { position: [x, trunk_height, z], normal: normal.to_array(), tangent: [1.0, 0.0, 0.0, 1.0], uv: [u, 0.0], color: trunk_color });
        vertices.push(Vertex { position: [x, -0.2, z], normal: normal.to_array(), tangent: [1.0, 0.0, 0.0, 1.0], uv: [u, 1.0], color: trunk_color });
    }

    for i in 0..segs {
        let i0 = (i * 2) as u32;
        let i1 = i0 + 1;
        let i2 = i0 + 2;
        let i3 = i0 + 3;

        indices.push(i0); indices.push(i1); indices.push(i2);
        indices.push(i2); indices.push(i1); indices.push(i3);
    }

    // 2. Trois étages de cônes/pyramides pour le feuillage
    let cone_layers = [
        (0.6, 1.5, 1.2, leaf_color_dark),
        (1.1, 2.1, 1.0, leaf_color_light),
        (1.6, 2.7, 0.75, leaf_color_dark),
    ];

    for (base_y, tip_y, base_r, col) in cone_layers {
        let base_idx = vertices.len() as u32;
        let tip_idx = base_idx;

        // Sommet de la pointe du cône
        vertices.push(Vertex { position: [0.0, tip_y, 0.0], normal: [0.0, 1.0, 0.0], tangent: [1.0, 0.0, 0.0, 1.0], uv: [0.5, 0.0], color: col });

        let num_pts = 7;
        for i in 0..num_pts {
            let u = (i as f32) / (num_pts as f32);
            let theta = 2.0 * std::f32::consts::PI * u;
            let x = base_r * theta.cos();
            let z = base_r * theta.sin();
            let n = Vec3::new(x, 0.5, z).normalize();
            vertices.push(Vertex { position: [x, base_y, z], normal: n.to_array(), tangent: [1.0, 0.0, 0.0, 1.0], uv: [u, 1.0], color: col });
        }

        for i in 0..num_pts {
            let current = base_idx + 1 + i as u32;
            let next = base_idx + 1 + ((i + 1) % num_pts) as u32;

            indices.push(tip_idx);
            indices.push(current);
            indices.push(next);
        }
    }

    compute_tangents(&mut vertices, &indices);
    (vertices, indices)
}

/// Génère une Colonne / Pilier Antique avec tangentes
pub fn generate_column_data() -> (Vec<Vertex>, Vec<u32>) {
    let mut vertices = Vec::new();
    let mut indices = Vec::new();
    let stone_color = [0.88, 0.86, 0.82, 1.0];

    let segs = 12;
    let base_r = 0.5;
    let shaft_r = 0.35;
    let height = 2.2;

    let y_levels = [-1.0, -0.8, -0.7, 0.7, 0.8, 1.0];
    let radii = [base_r, base_r, shaft_r, shaft_r, base_r, base_r];

    for (layer, (&y, &r)) in y_levels.iter().zip(radii.iter()).enumerate() {
        let layer_start = vertices.len() as u32;
        let v = (layer as f32) / (y_levels.len() as f32 - 1.0);

        for i in 0..=segs {
            let u = (i as f32) / (segs as f32);
            let theta = 2.0 * std::f32::consts::PI * u;
            let x = r * theta.cos();
            let z = r * theta.sin();
            let normal = Vec3::new(x, 0.0, z).normalize_or_zero();

            vertices.push(Vertex {
                position: [x, y * (height * 0.5), z],
                normal: normal.to_array(),
                tangent: [1.0, 0.0, 0.0, 1.0],
                uv: [u, v],
                color: stone_color,
            });
        }

        if layer > 0 {
            let prev_layer_start = layer_start - (segs as u32 + 1);
            for i in 0..segs {
                let p0 = prev_layer_start + i as u32;
                let p1 = p0 + 1;
                let c0 = layer_start + i as u32;
                let c1 = c0 + 1;

                indices.push(p0); indices.push(p1); indices.push(c0);
                indices.push(c0); indices.push(p1); indices.push(c1);
            }
        }
    }

    compute_tangents(&mut vertices, &indices);
    (vertices, indices)
}

/// Génère un plan horizontal (sol) avec normales vers le haut et tangentes
pub fn generate_plane_data(width: f32, depth: f32) -> (Vec<Vertex>, Vec<u32>) {
    let hw = width * 0.5;
    let hd = depth * 0.5;
    let normal = [0.0, 1.0, 0.0];
    let tangent = [1.0, 0.0, 0.0, 1.0];
    let color = [0.85, 0.85, 0.88, 1.0];

    let mut vertices = vec![
        Vertex { position: [-hw, 0.0, -hd], normal, tangent, uv: [0.0, 0.0], color },
        Vertex { position: [ hw, 0.0, -hd], normal, tangent, uv: [5.0, 0.0], color },
        Vertex { position: [ hw, 0.0,  hd], normal, tangent, uv: [5.0, 5.0], color },
        Vertex { position: [-hw, 0.0,  hd], normal, tangent, uv: [0.0, 5.0], color },
    ];
    let indices = vec![0, 1, 2, 0, 2, 3];
    compute_tangents(&mut vertices, &indices);
    (vertices, indices)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_procedural_generators() {
        let (v_sphere, i_sphere) = generate_sphere_data(1.0, 8, 8);
        assert!(!v_sphere.is_empty());
        assert!(!i_sphere.is_empty());
        assert_eq!(v_sphere[0].tangent.len(), 4);

        let (v_tree, i_tree) = generate_lowpoly_tree_data();
        assert!(!v_tree.is_empty());
        assert!(!i_tree.is_empty());

        let (v_col, i_col) = generate_column_data();
        assert!(!v_col.is_empty());
        assert!(!i_col.is_empty());
    }

    #[test]
    fn test_load_obj_from_str() {
        let obj_str = r#"
            v 0.0 0.0 0.0
            v 1.0 0.0 0.0
            v 0.0 1.0 0.0
            vt 0.0 0.0
            vt 1.0 0.0
            vt 0.0 1.0
            f 1/1 2/2 3/3
        "#;
        let res = load_obj_from_str(obj_str, "TestTriangle");
        assert!(res.is_ok());
        let (name, v, i) = res.unwrap();
        assert_eq!(name, "TestTriangle");
        assert_eq!(v.len(), 3);
        assert_eq!(i.len(), 3);
        assert_eq!(v[0].tangent.len(), 4);
    }
}
