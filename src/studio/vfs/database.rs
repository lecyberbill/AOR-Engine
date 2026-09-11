// [WFGY] Zone: SAFE | λ: 0.25 | Fallbacks: 0/None | Action: Asset database with UUID indexing, .meta scan and cache integrity
#![allow(dead_code)]
use super::asset::{
    extension_of, file_stem, hash_file, meta_path_for, AssetId, AssetMetadata, AssetType,
};
use super::manifest::ProjectLayout;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Erreur de la base d'assets
#[derive(Debug)]
pub enum AssetError {
    Io(String),
    Parse(String),
    NotFound(String),
}

impl std::fmt::Display for AssetError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AssetError::Io(e) => write!(f, "IO: {}", e),
            AssetError::Parse(e) => write!(f, "Parse: {}", e),
            AssetError::NotFound(e) => write!(f, "NotFound: {}", e),
        }
    }
}

impl std::error::Error for AssetError {}

/// Base de données d'assets : associe UUID, métadonnées et chemins sur disque
pub struct AssetDatabase {
    pub layout: ProjectLayout,
    by_id: HashMap<AssetId, AssetMetadata>,
    by_path: HashMap<PathBuf, AssetId>,
    /// Fichiers dont le hash a changé depuis le dernier scan (hot-reload)
    dirty: Vec<AssetId>,
}

impl AssetDatabase {
    pub fn new(layout: ProjectLayout) -> Self {
        AssetDatabase {
            layout,
            by_id: HashMap::new(),
            by_path: HashMap::new(),
            dirty: Vec::new(),
        }
    }

    pub fn len(&self) -> usize {
        self.by_id.len()
    }

    pub fn is_empty(&self) -> bool {
        self.by_id.is_empty()
    }

    pub fn get(&self, id: AssetId) -> Option<&AssetMetadata> {
        self.by_id.get(&id)
    }

    pub fn id_for_path(&self, path: &Path) -> Option<AssetId> {
        self.by_path.get(path).copied()
    }

    pub fn path_for_id(&self, id: AssetId) -> Option<&PathBuf> {
        self.by_id.get(&id).map(|m| &m.source_path)
    }

    pub fn iter(&self) -> impl Iterator<Item = &AssetMetadata> {
        self.by_id.values()
    }

    /// Assets correspondant à un type donné
    pub fn assets_of_type(&self, ty: &AssetType) -> Vec<&AssetMetadata> {
        self.by_id.values().filter(|m| &m.asset_type == ty).collect()
    }

    /// Assets modifiés depuis le dernier `take_dirty`
    pub fn take_dirty(&mut self) -> Vec<AssetId> {
        std::mem::take(&mut self.dirty)
    }

    /// Enregistre ou met à jour une métadonnée en mémoire et sur disque
    pub fn insert(&mut self, meta: AssetMetadata) -> Result<(), AssetError> {
        let id = meta.id;
        let path = meta.source_path.clone();
        self.by_id.insert(id, meta.clone());
        self.by_path.insert(path.clone(), id);
        self.write_meta(&meta)
            .map_err(|e| AssetError::Io(e.to_string()))
    }

    /// Lit le fichier `.meta` associé, ou le crée s'il n'existe pas
    pub fn read_or_create_meta(&mut self, source: &Path) -> Result<AssetMetadata, AssetError> {
        let meta_path = meta_path_for(source);

        if meta_path.exists() {
            let json = std::fs::read_to_string(&meta_path).map_err(|e| AssetError::Io(e.to_string()))?;
            let mut meta: AssetMetadata =
                serde_json::from_str(&json).map_err(|e| AssetError::Parse(e.to_string()))?;
            // Resynchronise le chemin et le hash avec l'état disque réel
            meta.source_path = source.to_path_buf();
            meta.hash = hash_file(source).unwrap_or(0);
            self.by_id.insert(meta.id, meta.clone());
            self.by_path.insert(source.to_path_buf(), meta.id);
            Ok(meta)
        } else {
            let ty = AssetType::from_extension(&extension_of(source));
            let mut meta = AssetMetadata::new(source.to_path_buf(), ty);
            meta.hash = hash_file(source).unwrap_or(0);
            self.insert(meta.clone())?;
            Ok(meta)
        }
    }

    fn write_meta(&self, meta: &AssetMetadata) -> std::io::Result<()> {
        let meta_path = meta_path_for(&meta.source_path);
        let json = serde_json::to_string_pretty(meta).unwrap_or_default();
        if let Some(parent) = meta_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(meta_path, json)
    }

    /// Parcourt récursivement le dossier `Assets` et synchronise la base
    pub fn scan(&mut self) -> Result<usize, AssetError> {
        let assets_root = self.layout.assets_dir();
        if !assets_root.exists() {
            return Ok(0);
        }

        let mut scanned = 0usize;
        let mut stack = vec![assets_root];
        while let Some(dir) = stack.pop() {
            let entries = match std::fs::read_dir(&dir) {
                Ok(e) => e,
                Err(_) => continue,
            };
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    if path.file_name().and_then(|s| s.to_str()) == Some("Library") {
                        continue;
                    }
                    stack.push(path);
                } else {
                    let ext = extension_of(&path);
                    if ext == "meta" {
                        continue;
                    }
                    let before = self.by_path.get(&path).copied();
                    let old_hash = before.and_then(|id| self.by_id.get(&id)).map(|m| m.hash);
                    let meta = self.read_or_create_meta(&path)?;
                    if let Some(old) = old_hash {
                        if old != meta.hash {
                            self.dirty.push(meta.id);
                        }
                    }
                    scanned += 1;
                }
            }
        }
        Ok(scanned)
    }

    /// Supprime un asset de la base et du disque (source + meta)
    pub fn remove(&mut self, id: AssetId, delete_files: bool) -> Result<(), AssetError> {
        let meta = self
            .by_id
            .remove(&id)
            .ok_or_else(|| AssetError::NotFound(id.to_string()))?;
        self.by_path.remove(&meta.source_path);
        if delete_files {
            let _ = std::fs::remove_file(&meta.source_path);
            let _ = std::fs::remove_file(meta_path_for(&meta.source_path));
        }
        Ok(())
    }

    /// Crée un nouvel asset vide textuel et l'enregistre
    pub fn create_asset(
        &mut self,
        relative_path: &str,
        contents: &[u8],
    ) -> Result<AssetId, AssetError> {
        let full = self.layout.assets_dir().join(relative_path);
        if let Some(parent) = full.parent() {
            std::fs::create_dir_all(parent).map_err(|e| AssetError::Io(e.to_string()))?;
        }
        std::fs::write(&full, contents).map_err(|e| AssetError::Io(e.to_string()))?;
        let ty = AssetType::from_extension(&extension_of(&full));
        let mut meta = AssetMetadata::new(full, ty);
        meta.hash = super::asset::hash_bytes(contents);
        let id = meta.id;
        self.insert(meta)?;
        Ok(id)
    }

    /// Nom lisible (stem) d'un asset
    pub fn display_name(&self, id: AssetId) -> String {
        self.by_id
            .get(&id)
            .map(|m| file_stem(&m.source_path))
            .unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "aor_vfs_test_{}_{}",
            tag,
            uuid::Uuid::new_v4()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn test_scan_creates_meta_and_indexes() {
        let root = temp_dir("scan");
        let layout = ProjectLayout::new(&root);
        layout.create_dirs().unwrap();
        std::fs::write(layout.assets_dir().join("neon.png"), b"fake-png").unwrap();
        std::fs::write(layout.assets_dir().join("script.rhai"), b"log(\"hi\")").unwrap();

        let mut db = AssetDatabase::new(layout.clone());
        let n = db.scan().unwrap();
        assert_eq!(n, 2);
        assert_eq!(db.len(), 2);
        assert!(meta_path_for(&layout.assets_dir().join("neon.png")).exists());

        // Un second scan ne doit pas dupliquer les entrées
        db.scan().unwrap();
        assert_eq!(db.len(), 2);

        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn test_dirty_detection_on_change() {
        let root = temp_dir("dirty");
        let layout = ProjectLayout::new(&root);
        layout.create_dirs().unwrap();
        let file = layout.assets_dir().join("mat.aormat");
        std::fs::write(&file, b"v1").unwrap();

        let mut db = AssetDatabase::new(layout);
        db.scan().unwrap();
        assert!(db.take_dirty().is_empty());

        std::fs::write(&file, b"v2-changed").unwrap();
        db.scan().unwrap();
        assert_eq!(db.take_dirty().len(), 1);

        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn test_create_and_remove_asset() {
        let root = temp_dir("create");
        let layout = ProjectLayout::new(&root);
        layout.create_dirs().unwrap();
        let mut db = AssetDatabase::new(layout.clone());

        let id = db.create_asset("Scenes/Main.aorscene", b"{}").unwrap();
        assert_eq!(db.assets_of_type(&AssetType::Scene).len(), 1);
        assert!(db.path_for_id(id).is_some());

        db.remove(id, true).unwrap();
        assert!(db.is_empty());
        assert!(!db.layout.assets_dir().join("Scenes/Main.aorscene").exists());

        std::fs::remove_dir_all(&root).ok();
    }
}
