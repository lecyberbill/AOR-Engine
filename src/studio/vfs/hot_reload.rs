// [WFGY] Zone: SAFE | λ: 0.2 | Fallbacks: 0/None | Action: Hot-reload manager coupling watcher and asset database
#![allow(dead_code)]
use super::asset::{hash_file, meta_path_for};
use super::database::AssetDatabase;
use super::watcher::{AssetChange, ChangeKind, FileWatcher};
use super::asset::AssetId;

/// Résultat d'un cycle de hot-reload
#[derive(Debug, Default, Clone)]
pub struct ReloadBatch {
    pub created: Vec<AssetId>,
    pub modified: Vec<AssetId>,
    pub removed: Vec<AssetId>,
}

impl ReloadBatch {
    pub fn is_empty(&self) -> bool {
        self.created.is_empty() && self.modified.is_empty() && self.removed.is_empty()
    }

    pub fn total(&self) -> usize {
        self.created.len() + self.modified.len() + self.removed.len()
    }
}

/// Orchestre la détection disque -> mise à jour de la base d'assets
pub struct HotReloadManager {
    pub database: AssetDatabase,
    pub watcher: Option<FileWatcher>,
}

impl HotReloadManager {
    pub fn new(database: AssetDatabase) -> Self {
        HotReloadManager {
            database,
            watcher: None,
        }
    }

    /// Démarre la surveillance du dossier Assets du projet
    pub fn start(&mut self) -> notify::Result<()> {
        let mut watcher = FileWatcher::new()?;
        watcher.watch(self.database.layout.assets_dir())?;
        self.watcher = Some(watcher);
        Ok(())
    }

    pub fn stop(&mut self) {
        self.watcher = None;
    }

    /// Traite un lot de changements et met à jour la base. Retourne les assets réellement affectés.
    pub fn process(&mut self, changes: &[AssetChange]) -> ReloadBatch {
        let mut batch = ReloadBatch::default();

        for change in changes {
            if change.path.extension().and_then(|e| e.to_str()) == Some("meta") {
                continue;
            }

            match change.kind {
                ChangeKind::Removed => {
                    if let Some(id) = self.database.id_for_path(&change.path) {
                        let _ = self.database.remove(id, false);
                        batch.removed.push(id);
                    }
                }
                ChangeKind::Created => {
                    if let Ok(meta) = self.database.read_or_create_meta(&change.path) {
                        batch.created.push(meta.id);
                    }
                }
                ChangeKind::Modified => {
                    if let Some(id) = self.database.id_for_path(&change.path) {
                        let new_hash = hash_file(&change.path).unwrap_or(0);
                        let changed = self
                            .database
                            .get(id)
                            .map(|m| m.hash != new_hash)
                            .unwrap_or(true);
                        if changed {
                            let mut meta = self.database.get(id).cloned().unwrap();
                            meta.hash = new_hash;
                            let _ = std::fs::remove_file(meta_path_for(&change.path));
                            let _ = self.database.insert(meta);
                            batch.modified.push(id);
                        }
                    } else if let Ok(meta) = self.database.read_or_create_meta(&change.path) {
                        batch.created.push(meta.id);
                    }
                }
            }
        }

        batch
    }

    /// Cycle complet : récupère les changements depuis le watcher et les applique
    pub fn tick(&mut self) -> ReloadBatch {
        let changes = match &self.watcher {
            Some(w) => w.poll_assets(),
            None => Vec::new(),
        };
        self.process(&changes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::studio::vfs::manifest::ProjectLayout;

    #[test]
    fn test_process_modification_marks_dirty() {
        let root = std::env::temp_dir().join(format!("aor_hot_{}", uuid::Uuid::new_v4()));
        let layout = ProjectLayout::new(&root);
        layout.create_dirs().unwrap();
        let file = layout.assets_dir().join("shader.wgsl");
        std::fs::write(&file, b"@fragment fn fs() {}").unwrap();

        let mut db = AssetDatabase::new(layout);
        db.scan().unwrap();

        let mut mgr = HotReloadManager::new(db);
        std::fs::write(&file, b"@fragment fn fs() -> vec4f { return vec4f(1.0); }").unwrap();

        let batch = mgr.process(&[AssetChange {
            path: file.clone(),
            kind: ChangeKind::Modified,
        }]);
        assert_eq!(batch.modified.len(), 1);

        std::fs::remove_dir_all(&root).ok();
    }
}
