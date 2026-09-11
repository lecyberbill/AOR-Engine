// [WFGY] Zone: SAFE | λ: 0.25 | Fallbacks: 0/None | Action: Asset Browser panel (VFS tree, thumbnails, drag metadata)
#![allow(dead_code)]
use super::style::{column, leaf};
use crate::studio::vfs::{AssetDatabase, AssetId};
use std::collections::{BTreeMap, HashSet};
use ui_layout::NodeId;
use ui_widgets::{ListItemBadge, WidgetId, WidgetTree};

/// État d'affichage de l'explorateur d'assets
#[derive(Debug, Clone)]
pub struct AssetBrowserState {
    pub selected: Option<AssetId>,
    pub expanded: HashSet<String>,
    pub thumbnail_size: f32,
    pub search_query: String,
}

impl Default for AssetBrowserState {
    fn default() -> Self {
        AssetBrowserState {
            selected: None,
            expanded: HashSet::new(),
            thumbnail_size: 64.0,
            search_query: String::new(),
        }
    }
}

/// Ligne aplatie de l'arbre d'assets
#[derive(Debug, Clone, PartialEq)]
pub struct AssetRow {
    pub key: String,
    pub label: String,
    pub depth: usize,
    pub is_dir: bool,
    pub expanded: bool,
    pub selected: bool,
    pub id: Option<AssetId>,
}

#[derive(Default)]
struct DirNode {
    dirs: BTreeMap<String, DirNode>,
    files: Vec<(String, AssetId)>,
}

impl DirNode {
    fn insert(&mut self, components: &[String], id: AssetId) {
        if components.is_empty() {
            return;
        }
        if components.len() == 1 {
            self.files.push((components[0].clone(), id));
        } else {
            let dir = self
                .dirs
                .entry(components[0].clone())
                .or_insert_with(DirNode::default);
            dir.insert(&components[1..], id);
        }
    }
}

/// Aplatit l'arbre d'assets en lignes affichables selon l'état d'expansion
pub fn browse_rows(db: &AssetDatabase, state: &AssetBrowserState) -> Vec<AssetRow> {
    let root = db.layout.assets_dir();
    let mut tree = DirNode::default();

    for meta in db.iter() {
        let rel = meta
            .source_path
            .strip_prefix(&root)
            .unwrap_or(&meta.source_path);
        let components: Vec<String> = rel
            .components()
            .map(|c| c.as_os_str().to_string_lossy().to_string())
            .collect();
        if components.is_empty() {
            continue;
        }
        tree.insert(&components, meta.id);
    }

    let mut rows = Vec::new();
    flatten(&tree, "", 0, state, &mut rows);
    rows
}

fn flatten(
    node: &DirNode,
    prefix: &str,
    depth: usize,
    state: &AssetBrowserState,
    rows: &mut Vec<AssetRow>,
) {
    for (name, child) in &node.dirs {
        let full = if prefix.is_empty() {
            name.clone()
        } else {
            format!("{}/{}", prefix, name)
        };
        let expanded = state.expanded.contains(&full);
        rows.push(AssetRow {
            key: format!("dir:{}", full),
            label: name.clone(),
            depth,
            is_dir: true,
            expanded,
            selected: false,
            id: None,
        });
        if expanded {
            flatten(child, &full, depth + 1, state, rows);
        }
    }
    for (name, id) in &node.files {
        let matches = state.search_query.is_empty()
            || name.to_lowercase().contains(&state.search_query.to_lowercase());
        if !matches {
            continue;
        }
        rows.push(AssetRow {
            key: format!("file:{}", id),
            label: name.clone(),
            depth,
            is_dir: false,
            expanded: false,
            selected: state.selected == Some(*id),
            id: Some(*id),
        });
    }
}

fn badge_for(name: &str) -> ListItemBadge {
    let lower = name.to_ascii_lowercase();
    if lower.ends_with(".png") || lower.ends_with(".jpg") || lower.ends_with(".ktx2") {
        ListItemBadge::Active("TEX".to_string())
    } else if lower.ends_with(".rhai") {
        ListItemBadge::Success
    } else if lower.ends_with(".aormat") {
        ListItemBadge::Warning
    } else {
        ListItemBadge::None
    }
}

/// Construit le widget d'explorateur d'assets
pub fn build(
    tree: &mut WidgetTree,
    db: &AssetDatabase,
    state: &AssetBrowserState,
    width: f32,
    height: f32,
) -> NodeId {
    let rows = browse_rows(db, state);

    let crumbs = [
        ("assets_root", "📂 Assets".to_string()),
        ("assets_project", db.layout.root.file_name().map(|s| s.to_string_lossy().to_string()).unwrap_or_else(|| "Projet".into())),
    ];
    let inner_w = (width - 24.0).max(100.0);
    let breadcrumb = tree
        .breadcrumb(
            WidgetId::new("asset_breadcrumb"),
            &crumbs,
            leaf(90.0, 22.0),
            leaf(inner_w, 26.0),
        )
        .expect("breadcrumb");

    let view_rows: Vec<(String, String, usize, bool, bool, bool)> = rows
        .iter()
        .map(|r| {
            (
                r.key.clone(),
                r.label.clone(),
                r.depth,
                r.is_dir,
                r.expanded,
                r.selected,
            )
        })
        .collect();

    let tree_node = tree
        .tree_view(
            WidgetId::new("asset_tree"),
            &view_rows,
            leaf(inner_w, 22.0),
            leaf(inner_w, (height - 36.0).max(40.0)),
        )
        .expect("tree_view");

    // Liste secondaire de métadonnées (type / badge)
    let badges: Vec<(String, ListItemBadge)> = rows
        .iter()
        .filter(|r| !r.is_dir)
        .map(|r| (r.label.clone(), badge_for(&r.label)))
        .collect();
    let badge_list = tree
        .rich_list(
            WidgetId::new("asset_badges"),
            &badges,
            None,
            leaf(inner_w, 20.0),
            column(2.0),
        )
        .ok();

    let mut children = vec![breadcrumb, tree_node];
    if let Some(b) = badge_list {
        children.push(b);
    }

    let body = tree.container(&children, column(4.0)).expect("asset_browser body");
    tree.palette(
        WidgetId::new("studio_asset_browser_panel"),
        "ASSETS",
        false,
        Some(body),
        leaf(width, height),
    )
    .expect("asset_browser palette")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::studio::vfs::ProjectLayout;

    fn make_db() -> (AssetDatabase, std::path::PathBuf) {
        let root = std::env::temp_dir().join(format!("aor_browser_{}", uuid::Uuid::new_v4()));
        let layout = ProjectLayout::new(&root);
        layout.create_dirs().unwrap();
        let mut db = AssetDatabase::new(layout);
        db.create_asset("Textures/Neon.png", b"png").unwrap();
        db.create_asset("Textures/Grid.png", b"png").unwrap();
        db.create_asset("Scripts/Player.rhai", b"// rhai").unwrap();
        (db, root)
    }

    #[test]
    fn test_browse_rows_hierarchy_and_expansion() {
        let (db, root) = make_db();
        let mut state = AssetBrowserState::default();

        // Dossiers repliés : seuls les dossiers racine apparaissent
        let rows = browse_rows(&db, &state);
        assert_eq!(rows.iter().filter(|r| r.is_dir).count(), 2);
        assert_eq!(rows.iter().filter(|r| !r.is_dir).count(), 0);

        // On déplie "Textures"
        state.expanded.insert("Textures".to_string());
        let rows = browse_rows(&db, &state);
        assert!(rows.iter().any(|r| !r.is_dir && r.label == "Neon.png"));

        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn test_search_filter() {
        let (db, root) = make_db();
        let mut state = AssetBrowserState::default();
        state.expanded.insert("Textures".to_string());
        state.expanded.insert("Scripts".to_string());
        state.search_query = "player".to_string();
        let rows = browse_rows(&db, &state);
        let files: Vec<_> = rows.iter().filter(|r| !r.is_dir).collect();
        assert_eq!(files.len(), 1);
        assert_eq!(files[0].label, "Player.rhai");

        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn test_build_asset_browser_widget() {
        let (db, root) = make_db();
        let mut tree = WidgetTree::new();
        let mut state = AssetBrowserState::default();
        state.expanded.insert("Textures".to_string());
        let node = build(&mut tree, &db, &state, 320.0, 400.0);
        assert!(tree.layout().children(node).is_ok());

        std::fs::remove_dir_all(&root).ok();
    }
}
