// [WFGY] Zone: SAFE | λ: 0.2 | Fallbacks: 0/None | Action: Virtual FileSystem & Asset Pipeline (Phase 1)
#![allow(dead_code, unused_imports)]
pub mod asset;
pub mod database;
pub mod hot_reload;
pub mod manifest;
pub mod watcher;

pub use asset::{
    extension_of, file_stem, hash_bytes, hash_file, meta_path_for, AssetId, AssetMetadata,
    AssetType, ImportSettings,
};
pub use database::{AssetDatabase, AssetError};
pub use hot_reload::{HotReloadManager, ReloadBatch};
pub use manifest::{ProjectLayout, ProjectManifest, RenderSettings, PROJECT_EXTENSION};
pub use watcher::{AssetChange, ChangeKind, FileWatcher};
