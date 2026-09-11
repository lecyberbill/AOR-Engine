// [WFGY] Zone: RISK | λ: 0.3 | Fallbacks: 0/None | Action: Standalone game packager (.aorpak archive, dependency collection & binary export)
#![allow(dead_code)]
use crate::studio::vfs::{hash_bytes, meta_path_for, AssetDatabase, AssetId};
use std::collections::{HashMap, HashSet, VecDeque};
use std::path::{Path, PathBuf};

pub const PAK_MAGIC: &[u8; 8] = b"AORPAK01";
pub const PAK_VERSION: u32 = 1;

/// Statistiques d'un build de paquet
#[derive(Debug, Clone, PartialEq)]
pub struct PakStats {
    pub file_count: usize,
    pub total_bytes: usize,
    pub output_path: PathBuf,
}

/// Métadonnées d'une entrée d'archive
#[derive(Debug, Clone, PartialEq)]
pub struct PakEntryInfo {
    pub path: String,
    pub size: usize,
    pub hash: u64,
}

/// Écrit une archive `.aorpak` à partir d'une liste (chemin logique, données)
pub fn write_pak(path: &Path, entries: &[(String, Vec<u8>)]) -> std::io::Result<Vec<PakEntryInfo>> {
    let mut buf = Vec::new();
    buf.extend_from_slice(PAK_MAGIC);
    buf.extend_from_slice(&PAK_VERSION.to_le_bytes());
    buf.extend_from_slice(&(entries.len() as u32).to_le_bytes());

    let mut infos = Vec::with_capacity(entries.len());
    for (logical_path, data) in entries {
        let pb = logical_path.as_bytes();
        let hash = hash_bytes(data);
        buf.extend_from_slice(&(pb.len() as u32).to_le_bytes());
        buf.extend_from_slice(pb);
        buf.extend_from_slice(&(data.len() as u64).to_le_bytes());
        buf.extend_from_slice(&hash.to_le_bytes());
        buf.extend_from_slice(data);
        infos.push(PakEntryInfo {
            path: logical_path.clone(),
            size: data.len(),
            hash,
        });
    }

    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(path, buf)?;
    Ok(infos)
}

/// Lit une archive `.aorpak` et retourne ses entrées indexées par chemin logique
pub fn read_pak(path: &Path) -> Result<HashMap<String, Vec<u8>>, String> {
    let bytes = std::fs::read(path).map_err(|e| e.to_string())?;
    let mut cursor = 0usize;

    fn take<'a>(bytes: &'a [u8], cursor: &mut usize, n: usize) -> Result<&'a [u8], String> {
        if *cursor + n > bytes.len() {
            return Err("archive .aorpak tronquée".to_string());
        }
        let slice = &bytes[*cursor..*cursor + n];
        *cursor += n;
        Ok(slice)
    }

    let magic = take(&bytes, &mut cursor, 8)?;
    if magic != PAK_MAGIC {
        return Err("signature .aorpak invalide".to_string());
    }
    let version = u32::from_le_bytes(take(&bytes, &mut cursor, 4)?.try_into().unwrap());
    if version != PAK_VERSION {
        return Err(format!("version .aorpak non supportée: {}", version));
    }
    let count = u32::from_le_bytes(take(&bytes, &mut cursor, 4)?.try_into().unwrap()) as usize;

    let mut map = HashMap::with_capacity(count);
    for _ in 0..count {
        let path_len =
            u32::from_le_bytes(take(&bytes, &mut cursor, 4)?.try_into().unwrap()) as usize;
        let logical = String::from_utf8(take(&bytes, &mut cursor, path_len)?.to_vec())
            .map_err(|e| e.to_string())?;
        let data_len =
            u64::from_le_bytes(take(&bytes, &mut cursor, 8)?.try_into().unwrap()) as usize;
        let _stored_hash = u64::from_le_bytes(take(&bytes, &mut cursor, 8)?.try_into().unwrap());
        let data = take(&bytes, &mut cursor, data_len)?.to_vec();
        map.insert(logical, data);
    }
    Ok(map)
}

fn asset_reference_tokens(meta: &crate::studio::vfs::AssetMetadata) -> Vec<String> {
    let mut tokens = vec![meta.id.to_string()];
    let rel = meta
        .source_path
        .file_name()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_default();
    if !rel.is_empty() {
        tokens.push(rel);
    }
    if let Some(stem) = meta.source_path.file_stem().and_then(|s| s.to_str()) {
        if !stem.is_empty() {
            tokens.push(stem.to_string());
        }
    }
    tokens
}

/// Parcourt les dépendances d'une scène de démarrage (références par UUID ou nom de fichier)
pub fn collect_dependencies(db: &AssetDatabase, start_scene: &Path) -> Result<Vec<AssetId>, String> {
    let mut result: Vec<AssetId> = Vec::new();
    let mut seen: HashSet<AssetId> = HashSet::new();
    let mut queue: VecDeque<PathBuf> = VecDeque::new();

    let start_id = db
        .id_for_path(start_scene)
        .ok_or_else(|| format!("scène de démarrage absente de la base: {:?}", start_scene))?;
    queue.push_back(db.path_for_id(start_id).cloned().unwrap());

    while let Some(path) = queue.pop_front() {
        let Some(id) = db.id_for_path(&path) else {
            continue;
        };
        if !seen.insert(id) {
            continue;
        }
        result.push(id);

        // Lecture tolérante (texte ou binaire) pour chercher les références
        let Ok(bytes) = std::fs::read(&path) else {
            continue;
        };
        let haystack = String::from_utf8_lossy(&bytes);

        for meta in db.iter() {
            if seen.contains(&meta.id) {
                continue;
            }
            let referenced = asset_reference_tokens(meta)
                .iter()
                .any(|tok| !tok.is_empty() && haystack.contains(tok));
            if referenced {
                queue.push_back(meta.source_path.clone());
            }
        }
    }

    Ok(result)
}

/// Compile une archive `.aorpak` contenant les assets demandés (source + .meta)
pub fn build_pak(
    db: &AssetDatabase,
    asset_ids: &[AssetId],
    out_path: &Path,
) -> Result<PakStats, String> {
    let root = &db.layout.assets_dir();
    let mut entries: Vec<(String, Vec<u8>)> = Vec::new();

    for id in asset_ids {
        let meta = db.get(*id).ok_or_else(|| format!("asset non trouvé: {}", id))?;
        let logical = meta
            .source_path
            .strip_prefix(root)
            .unwrap_or(&meta.source_path)
            .to_string_lossy()
            .replace('\\', "/");

        let data = std::fs::read(&meta.source_path).map_err(|e| e.to_string())?;
        entries.push((format!("assets/{}", logical), data));

        let meta_path = meta_path_for(&meta.source_path);
        if meta_path.exists() {
            if let Ok(meta_bytes) = std::fs::read(&meta_path) {
                entries.push((format!("assets/{}.meta", logical), meta_bytes));
            }
        }
    }

    let total_bytes = entries.iter().map(|(_, d)| d.len()).sum();
    let infos = write_pak(out_path, &entries).map_err(|e| e.to_string())?;

    Ok(PakStats {
        file_count: infos.len(),
        total_bytes,
        output_path: out_path.to_path_buf(),
    })
}

/// Pipeline complet : collecte des dépendances puis packaging
pub fn build_standalone(
    db: &AssetDatabase,
    start_scene: &Path,
    out_path: &Path,
) -> Result<PakStats, String> {
    let deps = collect_dependencies(db, start_scene)?;
    build_pak(db, &deps, out_path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::studio::vfs::ProjectLayout;

    fn temp_dir(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("aor_pak_{}_{}", tag, uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn test_pak_write_read_roundtrip() {
        let dir = temp_dir("roundtrip");
        let pak = dir.join("game.aorpak");
        let entries = vec![
            ("assets/scene.aorscene".to_string(), b"scene-data".to_vec()),
            ("assets/neon.png".to_string(), b"png-bytes".to_vec()),
        ];
        let infos = write_pak(&pak, &entries).unwrap();
        assert_eq!(infos.len(), 2);

        let map = read_pak(&pak).unwrap();
        assert_eq!(map.len(), 2);
        assert_eq!(map["assets/neon.png"], b"png-bytes");
        assert_eq!(map["assets/scene.aorscene"], b"scene-data");

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn test_pak_rejects_bad_magic() {
        let dir = temp_dir("bad");
        let pak = dir.join("bad.aorpak");
        std::fs::write(&pak, b"NOTAPAK!!!!!").unwrap();
        assert!(read_pak(&pak).is_err());
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn test_dependency_collection_and_build() {
        let dir = temp_dir("deps");
        let layout = ProjectLayout::new(&dir);
        layout.create_dirs().unwrap();
        let mut db = AssetDatabase::new(layout.clone());

        let tex_id = db.create_asset("Textures/Neon.png", b"PNGDATA").unwrap();
        let scene_id = db
            .create_asset(
                "Scenes/Main.aorscene",
                format!("{{\"texture\":\"{}\"}}", tex_id).as_bytes(),
            )
            .unwrap();
        let scene_path = db.path_for_id(scene_id).cloned().unwrap();

        let deps = collect_dependencies(&db, &scene_path).unwrap();
        assert!(deps.contains(&scene_id));
        assert!(deps.contains(&tex_id));

        let pak = dir.join("build/game.aorpak");
        let stats = build_pak(&db, &deps, &pak).unwrap();
        assert!(stats.file_count >= 2);
        assert!(pak.exists());

        let map = read_pak(&pak).unwrap();
        assert!(map.keys().any(|k| k.contains("Main.aorscene")));
        assert!(map.keys().any(|k| k.contains("Neon.png")));

        std::fs::remove_dir_all(&dir).ok();
    }
}
