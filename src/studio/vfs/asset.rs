// [WFGY] Zone: SAFE | λ: 0.2 | Fallbacks: 0 | Action: Asset identity, typing, import settings and metadata
#![allow(dead_code)]
use serde::{Deserialize, Serialize};
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};

/// Identifiant unique et stable d'un asset (UUIDv4)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct AssetId(pub uuid::Uuid);

impl AssetId {
    pub fn new() -> Self {
        AssetId(uuid::Uuid::new_v4())
    }

    pub fn nil() -> Self {
        AssetId(uuid::Uuid::nil())
    }

    pub fn to_hyphenated(&self) -> String {
        self.0.hyphenated().to_string()
    }
}

impl Default for AssetId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for AssetId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0.hyphenated())
    }
}

/// Nature d'une ressource du projet
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AssetType {
    Texture2D,
    Mesh,
    MaterialNodeGraph,
    Shader,
    AudioClip,
    RhaiScript,
    Prefab,
    Scene,
    Folder,
    Unknown,
}

impl AssetType {
    /// Déduit le type d'asset depuis l'extension de fichier
    pub fn from_extension(ext: &str) -> Self {
        match ext.to_ascii_lowercase().as_str() {
            "png" | "jpg" | "jpeg" | "bmp" | "tga" | "hdr" | "ktx2" => AssetType::Texture2D,
            "gltf" | "glb" | "obj" | "fbx" | "aomesh" => AssetType::Mesh,
            "aormat" => AssetType::MaterialNodeGraph,
            "wgsl" | "vert" | "frag" => AssetType::Shader,
            "wav" | "ogg" | "mp3" | "flac" => AssetType::AudioClip,
            "rhai" => AssetType::RhaiScript,
            "aorprefab" => AssetType::Prefab,
            "aorscene" | "world" => AssetType::Scene,
            _ => AssetType::Unknown,
        }
    }
}

/// Options d'importation par type d'asset
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ImportSettings {
    Texture {
        generate_mipmaps: bool,
        srgb: bool,
        filter_mode: u8,
    },
    Mesh {
        calculate_tangents: bool,
        flip_uv: bool,
        optimize_indices: bool,
    },
    Audio {
        streaming: bool,
        spatial_3d: bool,
        volume: f32,
    },
    Generic,
}

impl ImportSettings {
    /// Options par défaut associées à un type d'asset
    pub fn default_for(asset_type: &AssetType) -> Self {
        match asset_type {
            AssetType::Texture2D => ImportSettings::Texture {
                generate_mipmaps: true,
                srgb: true,
                filter_mode: 1,
            },
            AssetType::Mesh => ImportSettings::Mesh {
                calculate_tangents: true,
                flip_uv: false,
                optimize_indices: true,
            },
            AssetType::AudioClip => ImportSettings::Audio {
                streaming: false,
                spatial_3d: true,
                volume: 1.0,
            },
            _ => ImportSettings::Generic,
        }
    }
}

/// Métadonnées persistées dans le fichier `.meta` associé à chaque asset
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AssetMetadata {
    pub id: AssetId,
    pub asset_type: AssetType,
    pub source_path: PathBuf,
    pub import_settings: ImportSettings,
    pub hash: u64,
}

impl AssetMetadata {
    pub fn new(source_path: PathBuf, asset_type: AssetType) -> Self {
        let settings = ImportSettings::default_for(&asset_type);
        AssetMetadata {
            id: AssetId::new(),
            asset_type,
            source_path,
            import_settings: settings,
            hash: 0,
        }
    }
}

/// Hash Blake3-like non cryptographique (FNV-1a 64 bits) pour l'invalidation de cache.
/// Volontairement sans dépendance externe : suffisant pour la vérification de cache.
pub fn hash_bytes(bytes: &[u8]) -> u64 {
    let mut hasher = DefaultHasher::new();
    bytes.hash(&mut hasher);
    hasher.finish()
}

/// Calcule le hash d'un fichier sur disque
pub fn hash_file(path: &Path) -> std::io::Result<u64> {
    let bytes = std::fs::read(path)?;
    Ok(hash_bytes(&bytes))
}

/// Chemin du fichier `.meta` associé à un asset source
pub fn meta_path_for(source: &Path) -> PathBuf {
    let mut meta = source.as_os_str().to_os_string();
    meta.push(".meta");
    PathBuf::from(meta)
}

/// Nom de fichier sans extension
pub fn file_stem(path: &Path) -> String {
    path.file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or_default()
        .to_string()
}

/// Extension normalisée en minuscules
pub fn extension_of(path: &Path) -> String {
    path.extension()
        .and_then(|s| s.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_asset_id_unique_and_display() {
        let a = AssetId::new();
        let b = AssetId::new();
        assert_ne!(a, b);
        assert_eq!(a.to_string().len(), 36);
    }

    #[test]
    fn test_asset_type_from_extension() {
        assert_eq!(AssetType::from_extension("PNG"), AssetType::Texture2D);
        assert_eq!(AssetType::from_extension("glb"), AssetType::Mesh);
        assert_eq!(AssetType::from_extension("rhai"), AssetType::RhaiScript);
        assert_eq!(AssetType::from_extension("aorprefab"), AssetType::Prefab);
        assert_eq!(AssetType::from_extension("xyz"), AssetType::Unknown);
    }

    #[test]
    fn test_metadata_roundtrip_json() {
        let meta = AssetMetadata::new(PathBuf::from("Assets/Neon.png"), AssetType::Texture2D);
        let json = serde_json::to_string(&meta).unwrap();
        let back: AssetMetadata = serde_json::from_str(&json).unwrap();
        assert_eq!(meta, back);
    }

    #[test]
    fn test_hash_is_deterministic() {
        let h1 = hash_bytes(b"cyber-neon");
        let h2 = hash_bytes(b"cyber-neon");
        assert_eq!(h1, h2);
        assert_ne!(h1, hash_bytes(b"cyber-neon!"));
    }
}
