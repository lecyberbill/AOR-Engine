// [WFGY] Zone: SAFE | λ: 0.2 | Fallbacks: 0 | Action: Project manifest (.aorproj) and on-disk layout
#![allow(dead_code)]
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

pub const PROJECT_EXTENSION: &str = "aorproj";
pub const ASSETS_DIR: &str = "Assets";
pub const LIBRARY_DIR: &str = "Library";
pub const CACHE_DIR: &str = "Library/Cache";

/// Paramètres de rendu globaux du projet
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RenderSettings {
    pub hdr: bool,
    pub bloom: bool,
    pub shadows: bool,
    pub msaa_samples: u32,
    pub render_scale: f32,
    pub ambient_color: [f32; 3],
}

impl Default for RenderSettings {
    fn default() -> Self {
        RenderSettings {
            hdr: true,
            bloom: true,
            shadows: true,
            msaa_samples: 4,
            render_scale: 1.0,
            ambient_color: [0.05, 0.07, 0.12],
        }
    }
}

/// Manifeste racine d'un projet de jeu AOR
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProjectManifest {
    pub name: String,
    pub version: String,
    pub engine_version: String,
    pub start_scene: Option<PathBuf>,
    pub render_settings: RenderSettings,
}

impl Default for ProjectManifest {
    fn default() -> Self {
        ProjectManifest {
            name: "Nouveau Projet Cyber".to_string(),
            version: "0.1.0".to_string(),
            engine_version: env!("CARGO_PKG_VERSION").to_string(),
            start_scene: None,
            render_settings: RenderSettings::default(),
        }
    }
}

impl ProjectManifest {
    pub fn new(name: impl Into<String>) -> Self {
        ProjectManifest {
            name: name.into(),
            ..Default::default()
        }
    }

    pub fn to_json(&self) -> Result<String, String> {
        serde_json::to_string_pretty(self).map_err(|e| e.to_string())
    }

    pub fn from_json(json: &str) -> Result<Self, String> {
        serde_json::from_str(json).map_err(|e| e.to_string())
    }

    pub fn save(&self, project_dir: &Path) -> Result<PathBuf, String> {
        let path = project_dir.join(format!("{}.{}", self.name, PROJECT_EXTENSION));
        let json = self.to_json()?;
        std::fs::write(&path, json).map_err(|e| e.to_string())?;
        Ok(path)
    }

    pub fn load(path: &Path) -> Result<Self, String> {
        let json = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
        Self::from_json(&json)
    }
}

/// Layout standard d'un projet sur disque
#[derive(Debug, Clone)]
pub struct ProjectLayout {
    pub root: PathBuf,
}

impl ProjectLayout {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        ProjectLayout { root: root.into() }
    }

    pub fn assets_dir(&self) -> PathBuf {
        self.root.join(ASSETS_DIR)
    }

    pub fn library_dir(&self) -> PathBuf {
        self.root.join(LIBRARY_DIR)
    }

    pub fn cache_dir(&self) -> PathBuf {
        self.root.join(CACHE_DIR)
    }

    /// Crée l'arborescence complète du projet sur disque
    pub fn create_dirs(&self) -> std::io::Result<()> {
        std::fs::create_dir_all(self.root.join(ASSETS_DIR))?;
        std::fs::create_dir_all(self.root.join(CACHE_DIR))?;
        Ok(())
    }

    /// Initialise un projet complet (manifest + dossiers)
    pub fn init(&self, manifest: &ProjectManifest) -> Result<PathBuf, String> {
        self.create_dirs().map_err(|e| e.to_string())?;
        manifest.save(&self.root)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_manifest_roundtrip() {
        let mut m = ProjectManifest::new("NeonTunnel");
        m.start_scene = Some(PathBuf::from("Assets/Scenes/Main.aorscene"));
        let json = m.to_json().unwrap();
        let back = ProjectManifest::from_json(&json).unwrap();
        assert_eq!(m, back);
        assert!(json.contains("NeonTunnel"));
    }

    #[test]
    fn test_layout_paths() {
        let layout = ProjectLayout::new("D:/Projects/MyGame");
        assert!(layout.assets_dir().ends_with("Assets"));
        assert!(layout.cache_dir().ends_with("Cache"));
    }
}
