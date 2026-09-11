// [WFGY] Zone: SAFE | λ: 0.25 | Fallbacks: 0/None | Action: Filesystem watcher (notify) bridged to asset hot-reload events
#![allow(dead_code)]
use notify::{EventKind, RecursiveMode, Watcher};
use std::path::PathBuf;
use std::sync::mpsc::{channel, Receiver, Sender};

/// Nature d'un changement détecté sur disque
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ChangeKind {
    Created,
    Modified,
    Removed,
}

/// Changement d'asset détecté par le watcher
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AssetChange {
    pub path: PathBuf,
    pub kind: ChangeKind,
}

/// Surveillant de fichiers basé sur `notify` : alimente le hot-reload du studio
pub struct FileWatcher {
    _watcher: notify::RecommendedWatcher,
    rx: Receiver<AssetChange>,
    watched: Vec<PathBuf>,
}

impl FileWatcher {
    /// Crée un watcher inactif (aucun dossier surveillé)
    pub fn new() -> notify::Result<Self> {
        let (tx, rx): (Sender<AssetChange>, Receiver<AssetChange>) = channel();

        let event_handler = move |res: notify::Result<notify::Event>| {
            if let Ok(event) = res {
                let kind = match event.kind {
                    EventKind::Create(_) => Some(ChangeKind::Created),
                    EventKind::Modify(_) => Some(ChangeKind::Modified),
                    EventKind::Remove(_) => Some(ChangeKind::Removed),
                    _ => None,
                };
                if let Some(kind) = kind {
                    for path in event.paths {
                        let _ = tx.send(AssetChange {
                            path: path.clone(),
                            kind: kind.clone(),
                        });
                    }
                }
            }
        };

        let watcher = notify::recommended_watcher(event_handler)?;
        Ok(FileWatcher {
            _watcher: watcher,
            rx,
            watched: Vec::new(),
        })
    }

    /// Ajoute un dossier à surveiller récursivement
    pub fn watch(&mut self, path: impl Into<PathBuf>) -> notify::Result<()> {
        let path = path.into();
        self._watcher.watch(&path, RecursiveMode::Recursive)?;
        self.watched.push(path);
        Ok(())
    }

    /// Retire un dossier surveillé
    pub fn unwatch(&mut self, path: &std::path::Path) -> notify::Result<()> {
        self._watcher.unwatch(path)?;
        self.watched.retain(|p| p != path);
        Ok(())
    }

    pub fn watched_paths(&self) -> &[PathBuf] {
        &self.watched
    }

    /// Récupère tous les changements en attente sans bloquer
    pub fn poll(&self) -> Vec<AssetChange> {
        let mut changes = Vec::new();
        while let Ok(change) = self.rx.try_recv() {
            changes.push(change);
        }
        changes
    }

    /// Filtre les changements pour ne garder que les fichiers d'assets pertinents (ignore .meta)
    pub fn poll_assets(&self) -> Vec<AssetChange> {
        self.poll()
            .into_iter()
            .filter(|c| c.path.extension().and_then(|e| e.to_str()) != Some("meta"))
            .filter(|c| {
                c.path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .map(|n| !n.starts_with('.'))
                    .unwrap_or(true)
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn test_watcher_detects_file_creation() {
        let root = std::env::temp_dir().join(format!("aor_watch_{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&root).unwrap();

        let mut watcher = match FileWatcher::new() {
            Ok(w) => w,
            Err(_) => {
                std::fs::remove_dir_all(&root).ok();
                return;
            }
        };
        watcher.watch(root.clone()).unwrap();

        std::fs::write(root.join("new_asset.rhai"), b"log(1)").unwrap();
        std::thread::sleep(Duration::from_millis(400));

        let changes = watcher.poll_assets();
        let detected = changes.iter().any(|c| c.path.ends_with("new_asset.rhai"));
        assert!(detected, "le watcher aurait dû détecter la création");

        std::fs::remove_dir_all(&root).ok();
    }
}
