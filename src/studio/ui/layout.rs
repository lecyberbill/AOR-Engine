// [WFGY] Zone: SAFE | λ: 0.25 | Fallbacks: 0/None | Action: Cyber-Glass studio dock layout assembling all editor panels
#![allow(dead_code)]
use super::asset_browser::{self, AssetBrowserState};
use super::console::{self, ConsoleState};
use super::hierarchy::{self, HierarchyState};
use super::inspector::{self, InspectorState};
use super::node_canvas::{self, CanvasTransform};
use super::style::{absolute, column, leaf, popover_style, row};
use crate::scene::Scene;
use crate::studio::nodes::kinds::NodeKind;
use crate::studio::nodes::MaterialNodeGraph;
use crate::studio::vfs::AssetDatabase;
use ui_layout::{length, NodeId, Size, Style};
use ui_widgets::{MediaFit, WidgetId, WidgetTree};
use uuid::Uuid;

/// Onglets centraux commutables
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StudioCenterTab {
    Scene3D,
    Split3DShader,
    ShaderEditor,
    PrefabView,
    RhaiEditor,
}

impl StudioCenterTab {
    pub fn index(&self) -> usize {
        match self {
            StudioCenterTab::Scene3D => 0,
            StudioCenterTab::Split3DShader => 1,
            StudioCenterTab::ShaderEditor => 2,
            StudioCenterTab::PrefabView => 3,
            StudioCenterTab::RhaiEditor => 4,
        }
    }

    pub const LABELS: [&'static str; 5] =
        ["3D Scene", "Vue Combinée (3D + Shader)", "Shader Editor", "Prefab View", "Rhai Editor"];
}

/// État global du shell studio
#[derive(Debug, Clone)]
pub struct StudioShellState {
    pub active_tab: StudioCenterTab,
    pub active_menu: Option<usize>,
    pub hierarchy: HierarchyState,
    pub inspector: InspectorState,
    pub asset_browser: AssetBrowserState,
    pub console: ConsoleState,
    pub canvas: CanvasTransform,
    pub selected_graph_node: Option<Uuid>,
    pub pending_link: Option<(Uuid, Uuid)>,
    pub dragging_node: Option<(Uuid, [f32; 2])>,
    pub canvas_cursor: [f32; 2],
    pub playing: bool,
    pub paused: bool,
}

impl Default for StudioShellState {
    fn default() -> Self {
        StudioShellState {
            active_tab: StudioCenterTab::Scene3D,
            active_menu: None,
            hierarchy: HierarchyState::default(),
            inspector: InspectorState::default(),
            asset_browser: AssetBrowserState::default(),
            console: ConsoleState::default(),
            canvas: CanvasTransform::default(),
            selected_graph_node: None,
            pending_link: None,
            dragging_node: None,
            canvas_cursor: [0.0, 0.0],
            playing: false,
            paused: false,
        }
    }
}

fn grow(mut style: Style) -> Style {
    style.flex_grow = 1.0;
    style
}

/// Résultat du layout du shell : racine + nœud canvas nodal + viewport 3D
#[derive(Debug, Clone, Copy)]
pub struct StudioShellLayout {
    pub root: NodeId,
    pub node_canvas: Option<NodeId>,
    pub viewport: Option<NodeId>,
}

/// Construit un centre vide conservant la mise en page
fn placeholder_center(tree: &mut WidgetTree, message: &str, width: f32, _height: f32) -> NodeId {
    let label = tree
        .label(message.to_string(), leaf(width, 24.0))
        .expect("center label");
    tree.container(&[label], column(0.0))
        .expect("center placeholder")
}

/// Assemble le layout complet du studio (menubar, toolbar, docks, console)
pub fn build_studio_shell(
    tree: &mut WidgetTree,
    scene: Option<&Scene>,
    db: Option<&AssetDatabase>,
    graph: Option<&MaterialNodeGraph>,
    state: &StudioShellState,
    width: f32,
    height: f32,
) -> StudioShellLayout {
    // 1. Menubar (items de largeur fixe pour aligner les sous-menus)
    let menus = ["Fichier", "Édition", "Assets", "Objets", "Shader", "Build"];
    let mut menubar_style = row(0.0);
    menubar_style.size = Size {
        width: length(width),
        height: length(30.0),
    };
    let menubar = tree
        .menubar(
            WidgetId::new("studio_menubar"),
            &menus,
            state.active_menu,
            leaf(132.0, 26.0),
            menubar_style,
        )
        .expect("menubar");

    // 2. Toolbar (transport + tabs)
    let play = tree
        .icon_button(
            WidgetId::new("studio_play"),
            ui_widgets::IconKind::Play,
            true,
            leaf(28.0, 24.0),
        )
        .expect("play");
    let pause = tree
        .icon_button(
            WidgetId::new("studio_pause"),
            ui_widgets::IconKind::Pause,
            true,
            leaf(28.0, 24.0),
        )
        .expect("pause");
    let stop = tree
        .icon_button(
            WidgetId::new("studio_stop"),
            ui_widgets::IconKind::Close,
            true,
            leaf(28.0, 24.0),
        )
        .expect("stop");
    let apply_material = tree
        .icon_button(
            WidgetId::new("studio_apply_material"),
            ui_widgets::IconKind::Check,
            true,
            leaf(28.0, 24.0),
        )
        .expect("apply material");
    let reset_material = tree
        .icon_button(
            WidgetId::new("studio_reset_material"),
            ui_widgets::IconKind::Refresh,
            true,
            leaf(28.0, 24.0),
        )
        .expect("reset material");
    let theme_btn = tree
        .icon_button(
            WidgetId::new("studio_toggle_theme"),
            ui_widgets::IconKind::Eye,
            true,
            leaf(28.0, 24.0),
        )
        .expect("theme toggle");
    let tab_item_w = 140.0;
    let tabbar_total_w = (StudioCenterTab::LABELS.len() as f32 * tab_item_w + 10.0).min(width - 240.0);
    let tabbar = tree
        .tabbar(
            WidgetId::new("studio_center_tabs"),
            &StudioCenterTab::LABELS,
            state.active_tab.index(),
            leaf(tab_item_w, 24.0),
            leaf(tabbar_total_w, 26.0),
        )
        .expect("center tabs");
    let toolbar = tree
        .container(
            &[play, pause, stop, apply_material, reset_material, theme_btn, tabbar],
            row(8.0),
        )
        .expect("toolbar");

    // 3. Panneau de hiérarchie
    let hierarchy_panel = scene
        .map(|s| hierarchy::build(tree, s, &state.hierarchy, 250.0, height - 340.0))
        .unwrap_or_else(|| placeholder_center(tree, "Hiérarchie indisponible", 250.0, 120.0));

    // 4. Centre
    let center_width = (width - 250.0 - 360.0 - 40.0).max(240.0);
    let center_height = (height - 340.0).max(200.0);
    let mut node_canvas_id: Option<NodeId> = None;
    let mut viewport_id: Option<NodeId> = None;
    let center = match state.active_tab {
        StudioCenterTab::Scene3D => {
            let vp = tree
                .image(
                    WidgetId::new("studio_viewport"),
                    "viewport3d",
                    MediaFit::Fill,
                    leaf(center_width, center_height),
                )
                .unwrap_or_else(|_| {
                    placeholder_center(tree, "Viewport 3D", center_width, center_height)
                });
            viewport_id = Some(vp);
            vp
        }
        StudioCenterTab::Split3DShader => {
            let half_w = ((center_width - 10.0) * 0.5).max(180.0);
            let viewport_node = tree
                .image(
                    WidgetId::new("studio_viewport"),
                    "viewport3d",
                    MediaFit::Fill,
                    leaf(half_w, center_height),
                )
                .unwrap_or_else(|_| {
                    placeholder_center(tree, "Viewport 3D", half_w, center_height)
                });
            viewport_id = Some(viewport_node);
            let shader_node = match graph {
                Some(g) => {
                    let library = build_node_library(tree, 110.0, center_height);
                    let canvas_w = (half_w - 118.0).max(140.0);
                    let id = node_canvas::build(
                        tree,
                        g,
                        &state.canvas,
                        state.selected_graph_node,
                        state.pending_link,
                        state.canvas_cursor,
                        canvas_w,
                        center_height,
                    );
                    node_canvas_id = Some(id);
                    tree.container(&[library, id], row(6.0))
                        .expect("shader workspace split")
                }
                None => placeholder_center(tree, "Aucun graphe nodal", half_w, center_height),
            };
            tree.container(&[viewport_node, shader_node], row(10.0))
                .expect("split 3d shader workspace")
        }
        StudioCenterTab::ShaderEditor => match graph {
            Some(g) => {
                let library = build_node_library(tree, 150.0, center_height);
                let canvas_w = (center_width - 158.0).max(200.0);
                let id = node_canvas::build(
                    tree,
                    g,
                    &state.canvas,
                    state.selected_graph_node,
                    state.pending_link,
                    state.canvas_cursor,
                    canvas_w,
                    center_height,
                );
                node_canvas_id = Some(id);
                tree.container(&[library, id], row(8.0))
                    .expect("shader workspace")
            }
            None => placeholder_center(tree, "Aucun graphe nodal", center_width, center_height),
        },
        StudioCenterTab::PrefabView => {
            placeholder_center(tree, "Vue Prefab", center_width, center_height)
        }
        StudioCenterTab::RhaiEditor => tree
            .text_area(
                WidgetId::new("studio_rhai_editor"),
                state.console.input.clone(),
                "-- Script Rhai de l'entité",
                true,
                true,
                leaf(center_width, center_height),
            )
            .unwrap_or_else(|_| {
                placeholder_center(tree, "Éditeur Rhai", center_width, center_height)
            }),
    };

    // 5. Inspecteur (scène ou nœud de graphe sélectionné)
    let graph_node = graph.and_then(|g| state.selected_graph_node.and_then(|id| g.node(id)));
    let inspector_panel = inspector::build(
        tree,
        scene,
        &state.inspector,
        graph_node,
        360.0,
        height - 340.0,
    );

    let main_row = tree
        .container(
            &[hierarchy_panel, center, inspector_panel],
            grow(row(10.0)),
        )
        .expect("main row");

    // 6. Docks inférieurs : navigateur d'assets + console (largeur répartie équitablement)
    let bottom_panel_w = ((width - 40.0) * 0.5).max(200.0);
    let bottom_panel_h = 180.0;
    let browser = db
        .map(|d| asset_browser::build(tree, d, &state.asset_browser, bottom_panel_w, bottom_panel_h))
        .unwrap_or_else(|| placeholder_center(tree, "Assets indisponibles", bottom_panel_w, 120.0));
    let console_panel = console::build(tree, &state.console, bottom_panel_w, bottom_panel_h);
    let bottom_row = tree
        .container(&[browser, console_panel], row(10.0))
        .expect("bottom row");

    let mut root_style = column(8.0);
    root_style.size = Size {
        width: length(width),
        height: length(height),
    };
    let root = tree
        .container(&[menubar, toolbar, main_row, bottom_row], root_style)
        .expect("studio shell root");

    StudioShellLayout {
        root,
        node_canvas: node_canvas_id,
        viewport: viewport_id,
    }
}

/// Bibliothèque de nœuds disponibles dans la palette latérale
pub static NODE_LIBRARY: &[(&str, NodeKind)] = &[
    ("UV", NodeKind::Uv),
    ("Perlin", NodeKind::PerlinNoise),
    ("Voronoi", NodeKind::Voronoi),
    ("Checker", NodeKind::Checkerboard),
    ("Rgb", NodeKind::Rgb),
    ("Mix", NodeKind::Mix),
    ("Add", NodeKind::Add),
    ("Fresnel", NodeKind::Fresnel),
    ("Texture", NodeKind::SampleTexture2D),
    ("ColorRamp", NodeKind::ColorRamp),
];

/// Palette latérale de nœuds du Shader Editor
fn build_node_library(tree: &mut WidgetTree, width: f32, height: f32) -> NodeId {
    let mut children = Vec::new();
    for (i, (label, _)) in NODE_LIBRARY.iter().enumerate() {
        children.push(
            tree.button(
                WidgetId::new(format!("gn_add_{}", i)),
                *label,
                true,
                leaf(width - 24.0, 22.0),
            )
            .expect("library button"),
        );
    }
    let body = tree.container(&children, column(6.0))
        .expect("node library body");

    tree.palette(
        WidgetId::new("studio_node_library"),
        "NŒUDS",
        false,
        Some(body),
        leaf(width, height),
    )
    .expect("node library palette")
}

/// Menus déroulants propres au studio, alignés sous la menubar
pub fn build_studio_overlays(
    tree: &mut WidgetTree,
    state: &StudioShellState,
    width: f32,
    _height: f32,
) -> Option<NodeId> {
    let active = state.active_menu?;
    let items: Vec<(String, String, Option<String>, bool)> = match active {
        0 => vec![
            ("st_new_scene".into(), "Nouvelle Scène".into(), Some("Ctrl+N".into()), true),
            ("file_open".into(), "Ouvrir Fichier 3D (.world / .gltf / .obj)".into(), Some("Ctrl+O".into()), true),
            ("file_save".into(), "Sauvegarder Scène (.world)".into(), Some("Ctrl+S".into()), true),
            ("file_save_as".into(), "Enregistrer sous…".into(), None, true),
            ("st_import_aor".into(), "Ouvrir Paquet AOR (.aor)".into(), None, true),
            ("st_export_aor".into(), "Exporter Paquet AOR (.aor)".into(), None, true),
            ("file_quit".into(), "Quitter".into(), None, true),
        ],
        1 => vec![
            ("studio_delete_graph_node".into(), "Supprimer le nœud".into(), Some("Suppr".into()), true),
            ("studio_apply_material".into(), "Appliquer le matériau".into(), None, true),
            ("studio_reset_material".into(), "Réinitialiser au PBR".into(), None, true),
        ],
        2 => vec![
            ("asset_rescan".into(), "Réindexer les assets".into(), None, true),
        ],
        3 => vec![
            ("add_cube".into(), "Ajouter Cube".into(), Some("1".into()), true),
            ("add_pyramid".into(), "Ajouter Pyramide".into(), Some("2".into()), true),
            ("add_prism".into(), "Ajouter Prisme".into(), Some("3".into()), true),
            ("eco_bladerunner".into(), "Mini-Jeu Blade Runner".into(), None, true),
        ],
        4 => vec![
            ("gn_add_0".into(), "Ajouter UV".into(), None, true),
            ("gn_add_1".into(), "Ajouter Perlin".into(), None, true),
            ("gn_add_4".into(), "Ajouter Rgb".into(), None, true),
            ("gn_add_5".into(), "Ajouter Mix".into(), None, true),
            ("gn_add_7".into(), "Ajouter Fresnel".into(), None, true),
            ("studio_apply_material".into(), "Appliquer".into(), None, true),
        ],
        _ => vec![
            ("build_standalone".into(), "Build Standalone (.aorpak)".into(), None, true),
        ],
    };

    let item_h = 28.0;
    let popover_w = 260.0;
    let popover_h = items.len() as f32 * (item_h + 2.0) + 12.0;
    // La barre de menu commence à x=0 et chaque élément mesure 132.0px de large
    let menu_item_w = 132.0;
    let target_x = active as f32 * menu_item_w;
    let x = target_x.clamp(4.0, (width - popover_w - 4.0).max(4.0));
    let popover_s = popover_style(x, 32.0, popover_w, popover_h);
    let popover = tree
        .menu_popover(
            format!("studio_menu:{}", active),
            &items,
            leaf(popover_w - 12.0, item_h),
            popover_s,
        )
        .ok()?;

    tree.container(&[popover], leaf(width, _height)).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tab_labels_and_index() {
        assert_eq!(StudioCenterTab::Scene3D.index(), 0);
        assert_eq!(StudioCenterTab::Split3DShader.index(), 1);
        assert_eq!(StudioCenterTab::LABELS[1], "Vue Combinée (3D + Shader)");
        assert_eq!(StudioCenterTab::LABELS[2], "Shader Editor");
        assert_eq!(StudioCenterTab::RhaiEditor.index(), 4);
    }

    #[test]
    fn test_shell_builds_without_scene() {
        let mut tree = WidgetTree::new();
        let state = StudioShellState::default();
        let layout = build_studio_shell(&mut tree, None, None, None, &state, 1280.0, 720.0);
        assert!(tree.layout().children(layout.root).is_ok());
        assert!(layout.node_canvas.is_none());
    }
}
