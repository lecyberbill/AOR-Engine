// [WFGY] Zone: SAFE | λ: 0.25 | Fallbacks: 0/None | Action: Scene Hierarchy panel (parent/child tree with expand & select)
#![allow(dead_code)]
use super::style::{column, leaf};
use crate::scene::Scene;
use std::collections::HashSet;
use ui_layout::NodeId;
use ui_widgets::{WidgetId, WidgetTree};

/// État du panneau de hiérarchie de scène
#[derive(Debug, Clone, Default)]
pub struct HierarchyState {
    pub expanded: HashSet<usize>,
    pub rename_target: Option<usize>,
}

/// Description minimale d'un nœud pour l'aplatissement de l'arbre
#[derive(Debug, Clone, PartialEq)]
pub struct NodeInfo {
    pub id: usize,
    pub name: String,
    pub parent: Option<usize>,
    pub children: Vec<usize>,
}

/// Aplatit une forêt de nœuds (racines puis descendants) selon l'état d'expansion
pub fn hierarchy_rows(
    infos: &[NodeInfo],
    expanded: &HashSet<usize>,
    selected: Option<usize>,
) -> Vec<(String, String, usize, bool, bool, bool)> {
    let by_id: std::collections::HashMap<usize, &NodeInfo> =
        infos.iter().map(|n| (n.id, n)).collect();

    let mut rows = Vec::new();

    fn walk(
        info: &NodeInfo,
        depth: usize,
        by_id: &std::collections::HashMap<usize, &NodeInfo>,
        expanded: &HashSet<usize>,
        selected: Option<usize>,
        rows: &mut Vec<(String, String, usize, bool, bool, bool)>,
    ) {
        let is_dir = !info.children.is_empty();
        let is_expanded = expanded.contains(&info.id);
        rows.push((
            info.id.to_string(),
            info.name.clone(),
            depth,
            is_dir,
            is_expanded,
            selected == Some(info.id),
        ));
        if is_dir && is_expanded {
            for child in &info.children {
                if let Some(c) = by_id.get(child) {
                    walk(c, depth + 1, by_id, expanded, selected, rows);
                }
            }
        }
    }

    // Racines = nœuds sans parent, dans l'ordre d'identifiant
    let mut roots: Vec<&NodeInfo> = infos.iter().filter(|n| n.parent.is_none()).collect();
    roots.sort_by_key(|n| n.id);
    for root in roots {
        walk(root, 0, &by_id, expanded, selected, &mut rows);
    }
    rows
}

/// Construit les informations de nœuds depuis une scène GPU
pub fn scene_infos(scene: &Scene) -> Vec<NodeInfo> {
    scene
        .nodes
        .iter()
        .enumerate()
        .map(|(idx, node)| NodeInfo {
            id: idx,
            name: node.name.clone(),
            parent: node.parent_idx,
            children: node.children_indices.clone(),
        })
        .collect()
}

/// Construit le widget de hiérarchie de scène
pub fn build(
    tree: &mut WidgetTree,
    scene: &Scene,
    state: &HierarchyState,
    width: f32,
    height: f32,
) -> NodeId {
    let infos = scene_infos(scene);
    let rows = hierarchy_rows(&infos, &state.expanded, Some(scene.selected_node_idx));

    let view = tree
        .tree_view(
            WidgetId::new("scene_hierarchy"),
            &rows,
            leaf(width - 24.0, 22.0),
            leaf(width - 20.0, (height - 48.0).max(40.0)),
        )
        .expect("scene tree view");

    tree.palette(
        WidgetId::new("scene_hierarchy_panel"),
        "HIÉRARCHIE SCÈNE",
        false,
        Some(view),
        leaf(width, height),
    )
    .expect("hierarchy panel")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_infos() -> Vec<NodeInfo> {
        vec![
            NodeInfo {
                id: 0,
                name: "World Root".into(),
                parent: None,
                children: vec![1, 2],
            },
            NodeInfo {
                id: 1,
                name: "CyberCity".into(),
                parent: Some(0),
                children: vec![3],
            },
            NodeInfo {
                id: 2,
                name: "Player".into(),
                parent: Some(0),
                children: vec![],
            },
            NodeInfo {
                id: 3,
                name: "Skyscraper".into(),
                parent: Some(1),
                children: vec![],
            },
        ]
    }

    #[test]
    fn test_rows_collapsed_only_shows_root() {
        let infos = sample_infos();
        let rows = hierarchy_rows(&infos, &HashSet::new(), None);
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].1, "World Root");
    }

    #[test]
    fn test_rows_expanded_walks_children() {
        let infos = sample_infos();
        let mut expanded = HashSet::new();
        expanded.insert(0);
        let rows = hierarchy_rows(&infos, &expanded, Some(3));
        let labels: Vec<_> = rows.iter().map(|r| r.1.clone()).collect();
        assert_eq!(labels, vec!["World Root", "CyberCity", "Player"]);
        // Nœud 3 non visible car CyberCity (1) replié
        assert!(!labels.contains(&"Skyscraper".to_string()));

        let mut expanded = HashSet::new();
        expanded.insert(0);
        expanded.insert(1);
        let rows = hierarchy_rows(&infos, &expanded, Some(3));
        let selected = rows.iter().find(|r| r.1 == "Skyscraper").unwrap();
        assert!(selected.5, "le nœud sélectionné doit être marqué");
        assert_eq!(selected.2, 2, "profondeur attendue 2");
    }
}
