// [WFGY] Zone: SAFE | λ: 0.25 | Fallbacks: 0/None | Action: High-level Studio session facade (project, VFS, graph, triggers, PIE, packaging)
#![allow(dead_code)]
use super::level::TriggerSystem;
use super::nodes::MaterialNodeGraph;
use super::packager::{build_standalone, PakStats};
use super::pie::PlayInEditor;
use super::vfs::{
    AssetDatabase, HotReloadManager, ProjectLayout, ProjectManifest, ReloadBatch,
    PROJECT_EXTENSION,
};
use std::path::{Path, PathBuf};

/// Session de travail complète du studio : assemble les cinq sous-systèmes
pub struct StudioSession {
    pub manifest: ProjectManifest,
    pub layout: ProjectLayout,
    pub manager: HotReloadManager,
    pub graph: MaterialNodeGraph,
    pub triggers: TriggerSystem,
    pub pie: PlayInEditor,
}

impl StudioSession {
    /// Ouvre (ou initialise) un projet situé à la racine `root`
    pub fn open(root: impl Into<PathBuf>) -> Result<Self, String> {
        let layout = ProjectLayout::new(root);
        layout.create_dirs().map_err(|e| e.to_string())?;

        let manifest = match find_manifest(&layout.root) {
            Some(path) => ProjectManifest::load(&path)?,
            None => {
                let manifest = ProjectManifest::default();
                manifest.save(&layout.root)?;
                manifest
            }
        };

        let mut database = AssetDatabase::new(layout.clone());
        let _ = database.scan();

        let mut manager = HotReloadManager::new(database);
        let _ = manager.start();

        let graph = MaterialNodeGraph::new("Nouveau Matériau PBR");

        Ok(StudioSession {
            manifest,
            layout,
            manager,
            graph,
            triggers: TriggerSystem::new(),
            pie: PlayInEditor::new(),
        })
    }

    pub fn database(&self) -> &AssetDatabase {
        &self.manager.database
    }

    pub fn database_mut(&mut self) -> &mut AssetDatabase {
        &mut self.manager.database
    }

    /// Relance un scan complet du dossier d'assets
    pub fn rescan(&mut self) -> Result<usize, String> {
        let count = self.manager.database.scan().map_err(|e| e.to_string())?;
        Ok(count)
    }

    /// Traite les changements disque en attente (hot-reload)
    pub fn hot_reload_tick(&mut self) -> ReloadBatch {
        self.manager.tick()
    }

    /// Nombre d'assets indexés
    pub fn asset_count(&self) -> usize {
        self.manager.database.len()
    }

    /// Empaquette le jeu autonome depuis la scène de démarrage configurée
    pub fn package_start_scene(&self, out_path: &Path) -> Result<PakStats, String> {
        let start = self
            .manifest
            .start_scene
            .clone()
            .ok_or_else(|| "aucune scène de démarrage définie dans le manifeste".to_string())?;
        let start_full = if start.is_absolute() {
            start
        } else {
            self.layout.root.join(&start)
        };
        build_standalone(&self.manager.database, &start_full, out_path)
    }

    /// Empaquette un ensemble explicite d'assets
    pub fn package_assets(
        &self,
        asset_ids: &[super::vfs::AssetId],
        out_path: &Path,
    ) -> Result<PakStats, String> {
        super::packager::build_pak(&self.manager.database, asset_ids, out_path)
    }
}

/// Recherche le premier fichier `.aorproj` dans un dossier
fn find_manifest(root: &Path) -> Option<PathBuf> {
    let entries = std::fs::read_dir(root).ok()?;
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) == Some(PROJECT_EXTENSION) {
            return Some(path);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_open_creates_project_and_assets() {
        let root =
            std::env::temp_dir().join(format!("aor_session_{}", uuid::Uuid::new_v4()));
        let mut session = StudioSession::open(&root).expect("ouverture du projet");
        assert!(session.layout.assets_dir().exists());
        assert!(session.manifest.save(&root).is_ok());

        // Ajout d'un asset puis réindexation
        {
            let db = session.database_mut();
            db.create_asset("Textures/Neon.png", b"png").unwrap();
        }
        let n = session.rescan().unwrap();
        assert!(n >= 1);
        assert!(session.asset_count() >= 1);

        // Réouverture : le manifeste doit être retrouvé
        let reopened = StudioSession::open(&root).unwrap();
        assert!(!reopened.manifest.name.is_empty());

        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn test_package_without_start_scene_errors() {
        let root = std::env::temp_dir().join(format!("aor_session2_{}", uuid::Uuid::new_v4()));
        let session = StudioSession::open(&root).unwrap();
        let out = root.join("build/game.aorpak");
        let err = session.package_start_scene(&out).unwrap_err();
        assert!(err.contains("scène de démarrage"));
        std::fs::remove_dir_all(&root).ok();
    }
}
