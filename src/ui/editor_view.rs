// [WFGY] Zone: SAFE | λ: 0.25 | Fallbacks: 0/None | Action: Advanced Ergonomic Cyber-Glass Editor UI Layout for rust_moteur_3d
use ui_layout::{
    auto, length, AlignItems, Display, FlexDirection,
    NodeId, Position, Rect, Size, Style,
};
use ui_widgets::{
    ListItemBadge, MediaFit, WidgetId, WidgetTree,
};

use crate::scene::Scene;
use crate::ui::editor_state::{ActiveTool, EditorState, InspectorTab};

pub fn leaf(width: f32, height: f32) -> Style {
    Style {
        size: Size {
            width: length(width),
            height: length(height),
        },
        ..Default::default()
    }
}

pub fn row(gap: f32) -> Style {
    Style {
        display: Display::Flex,
        flex_direction: FlexDirection::Row,
        gap: Size {
            width: length(gap),
            height: length(0.0),
        },
        align_items: Some(AlignItems::Center),
        ..Default::default()
    }
}

pub fn column(gap: f32) -> Style {
    Style {
        display: Display::Flex,
        flex_direction: FlexDirection::Column,
        gap: Size {
            width: length(0.0),
            height: length(gap),
        },
        ..Default::default()
    }
}

fn popover_style(x: f32, y: f32, w: f32, h: f32) -> Style {
    Style {
        position: Position::Absolute,
        inset: Rect {
            top: length(y),
            left: length(x),
            right: auto(),
            bottom: auto(),
        },
        size: Size {
            width: length(w),
            height: length(h),
        },
        flex_direction: FlexDirection::Column,
        gap: Size {
            width: length(0.0),
            height: length(2.0),
        },
        ..Default::default()
    }
}

pub struct EngineMetrics {
    pub fps: f32,
    pub triangle_count: usize,
    pub node_count: usize,
    pub camera_mode: &'static str,
    pub render_mode: &'static str,
    pub status_text: String,
}

/// Construit la vue de base (Layer 0) : Viewport 3D, Gizmo Vectoriel, Menubar, Palettes flottantes et HUD
pub fn build_editor_base(
    tree: &mut WidgetTree,
    state: &EditorState,
    metrics: &EngineMetrics,
    scene: Option<&Scene>,
    gizmo: Option<&crate::tools::Gizmo>,
    view_proj: Option<glam::Mat4>,
    width: f32,
    height: f32,
) -> NodeId {
    let mut root_children = Vec::new();

    // 1. Fond Viewport 3D (Texture 3D GPU enregistrée dans ResourceTable "viewport3d")
    let viewport_style = Style {
        position: Position::Absolute,
        inset: Rect {
            left: length(0.0),
            top: length(0.0),
            right: length(0.0),
            bottom: length(0.0),
        },
        size: Size {
            width: length(width),
            height: length(height),
        },
        ..Default::default()
    };
    let viewport_node = tree
        .image(
            WidgetId::new("viewport_3d_canvas"),
            "viewport3d",
            MediaFit::Fill,
            viewport_style,
        )
        .unwrap();
    root_children.push(viewport_node);

    // 1b. Calque Gizmo 3D Vectoriel Cyber-Glass (CustomPaint Canvas)
    if let (Some(scene), Some(gizmo), Some(vp)) = (scene, gizmo, view_proj) {
        if !scene.nodes.is_empty() && scene.selected_node_idx < scene.nodes.len() {
            let selected_node = &scene.nodes[scene.selected_node_idx];
            let cmds = gizmo.generate_paint_commands(
                selected_node.transform.translation,
                vp,
                [0.0, 0.0, width, height],
            );
            if !cmds.is_empty() {
                let gizmo_style = Style {
                    position: Position::Absolute,
                    inset: Rect {
                        left: length(0.0),
                        top: length(0.0),
                        right: length(0.0),
                        bottom: length(0.0),
                    },
                    size: Size {
                        width: length(width),
                        height: length(height),
                    },
                    ..Default::default()
                };
                if let Ok(gizmo_node) = tree.custom_paint("gizmo_canvas", cmds, gizmo_style) {
                    root_children.push(gizmo_node);
                }
            }
        }
    }

    // 2. Barre de Navigation / Menubar Supérieure (Ajustée aux dimensions exactes des boutons)
    let menu_items = [
        "📂 FICHIER",
        "✨ PRIMITIVES",
        "⚡ RENDU",
        "🕹 MODE JEU",
        "🌐 ÉCOSYSTÈME",
        "❓ AIDE",
    ];
    let menubar_item_w = 132.0;
    let menubar_item_h = 28.0;
    let menubar_node = tree
        .menubar(
            WidgetId::new("top_menubar"),
            &menu_items,
            state.active_menu,
            leaf(menubar_item_w, menubar_item_h),
            Style {
                position: Position::Absolute,
                inset: Rect {
                    left: length(16.0),
                    top: length(10.0),
                    right: auto(),
                    bottom: auto(),
                },
                size: Size {
                    width: length(((menu_items.len() as f32 * (menubar_item_w + 4.0)) + 4.0).min(width - 32.0)),
                    height: length(32.0),
                },
                align_items: Some(AlignItems::Center),
                ..Default::default()
            },
        )
        .unwrap();
    root_children.push(menubar_node);

    // 3. Palette d'Outils Flottante (Gauche - Déplaçable, Pliable, Fermable)
    if state.show_tools_palette {
        let tool_w = state.tools_size.0 - 28.0;
        let tool_select = tree
            .button(
                "tool_select",
                "↖ Sélection",
                state.active_tool == ActiveTool::Select,
                leaf(tool_w, 28.0),
            )
            .unwrap();
        let tool_trans = tree
            .button(
                "tool_translate",
                "✥ Déplacement",
                state.active_tool == ActiveTool::Translate,
                leaf(tool_w, 28.0),
            )
            .unwrap();
        let tool_rot = tree
            .button(
                "tool_rotate",
                "↻ Rotation",
                state.active_tool == ActiveTool::Rotate,
                leaf(tool_w, 28.0),
            )
            .unwrap();
        let tool_scale = tree
            .button(
                "tool_scale",
                "⤢ Échelle (+/-)",
                state.active_tool == ActiveTool::Scale,
                leaf(tool_w, 28.0),
            )
            .unwrap();
        let tool_vert = tree
            .button(
                "tool_vertex",
                "☩ Sommets (Edit)",
                state.active_tool == ActiveTool::VertexEdit,
                leaf(tool_w, 28.0),
            )
            .unwrap();

        let tools_col = tree
            .container(&[tool_select, tool_trans, tool_rot, tool_scale, tool_vert], column(6.0))
            .unwrap();

        let tools_palette = tree
            .palette(
                "tools_palette",
                "OUTILS 3D",
                state.tools_folded,
                Some(tools_col),
                Style {
                    position: Position::Absolute,
                    inset: Rect {
                        left: length(state.tools_pos.0),
                        top: length(state.tools_pos.1),
                        right: auto(),
                        bottom: auto(),
                    },
                    size: Size {
                        width: length(state.tools_size.0),
                        height: if state.tools_folded { auto() } else { length(state.tools_size.1) },
                    },
                    ..Default::default()
                },
            )
            .unwrap();
        root_children.push(tools_palette);
    }

    // 4. Palette Déformations Procédurales (Gauche - Bas)
    if state.show_deformers_palette {
        let def_w = state.deformers_size.0 - 28.0;
        let wind_btn = tree
            .button(
                "toggle_wind",
                if state.wind_enabled { "🍃 Vent: ACTIF" } else { "🍃 Vent: INACTIF" },
                true,
                leaf(def_w, 28.0),
            )
            .unwrap();
        let wind_lbl = tree.label_muted("Force du Vent", leaf(def_w, 16.0)).unwrap();
        let wind_slider = tree
            .slider("wind_strength_slider", 0.0, 1.0, state.wind_strength, leaf(def_w, 20.0))
            .unwrap();

        let wave_btn = tree
            .button(
                "toggle_waves",
                if state.wave_enabled { "🌊 Vagues: ACTIF" } else { "🌊 Vagues: INACTIF" },
                true,
                leaf(def_w, 28.0),
            )
            .unwrap();
        let wave_lbl = tree.label_muted("Amplitude Vagues", leaf(def_w, 16.0)).unwrap();
        let wave_slider = tree
            .slider("wave_amp_slider", 0.0, 0.5, state.wave_amplitude, leaf(def_w, 20.0))
            .unwrap();

        let deformers_col = tree
            .container(&[wind_btn, wind_lbl, wind_slider, wave_btn, wave_lbl, wave_slider], column(6.0))
            .unwrap();

        let deformers_palette = tree
            .palette(
                "deformers_palette",
                "DÉFORMATION",
                state.deformers_folded,
                Some(deformers_col),
                Style {
                    position: Position::Absolute,
                    inset: Rect {
                        left: length(state.deformers_pos.0),
                        top: length(state.deformers_pos.1),
                        right: auto(),
                        bottom: auto(),
                    },
                    size: Size {
                        width: length(state.deformers_size.0),
                        height: if state.deformers_folded { auto() } else { length(state.deformers_size.1) },
                    },
                    ..Default::default()
                },
            )
            .unwrap();
        root_children.push(deformers_palette);
    }

    // 5. Dock / Inspecteur Latéral Droit (Cyber-Glass Dock Pro)
    if state.show_inspector_palette {
        let insp_w = state.inspector_size.0 - 28.0;
        let tabs = ["Transform", "Matériau", "Physique", "Anim", "Modif", "Scène", "Scripts"];
        let tab_idx = match state.inspector_tab {
            InspectorTab::Transform => 0,
            InspectorTab::Material => 1,
            InspectorTab::Physics => 2,
            InspectorTab::Animation => 3,
            InspectorTab::Modifiers => 4,
            InspectorTab::Hierarchy => 5,
            InspectorTab::Scripting => 6,
            _ => 0,
        };
        let tabbar = tree
            .segmented_control(
                "inspector_tabs",
                &tabs,
                tab_idx,
                leaf(insp_w / 6.0 - 3.0, 26.0),
                leaf(insp_w, 32.0),
            )
            .unwrap();

        let mut inspector_content = Vec::new();
        inspector_content.push(tabbar);

        match state.inspector_tab {
            InspectorTab::Transform => {
                let prim_title = tree.label("✨ Ajout Primitives Rapides", leaf(insp_w, 20.0)).unwrap();
                let prim_cube = tree.button("add_cube", "+ Cube", true, leaf(insp_w * 0.47, 28.0)).unwrap();
                let prim_pyr = tree.button("add_pyramid", "+ Pyramide", true, leaf(insp_w * 0.47, 28.0)).unwrap();
                let prim_row = tree.container(&[prim_cube, prim_pyr], row(10.0)).unwrap();

                let hier_title = tree.label("🔗 Hiérarchie Parent / Enfant", leaf(insp_w, 20.0)).unwrap();
                let parent_btn = tree.button("btn_parent_selected", "📎 Définir comme Enfant du Nœud #0", true, leaf(insp_w, 28.0)).unwrap();
                let unparent_btn = tree.button("btn_unparent_selected", "🔓 Détacher du Parent (Unparent)", true, leaf(insp_w, 28.0)).unwrap();

                let actions_title = tree.label("⚙ Actions Objet Sélectionné", leaf(insp_w, 20.0)).unwrap();
                let dup_btn = tree.button("duplicate_node", "📋 Dupliquer (Ctrl+D)", true, leaf(insp_w, 28.0)).unwrap();
                let del_btn = tree.button("delete_node", "🗑 Supprimer (Suppr)", true, leaf(insp_w, 28.0)).unwrap();

                let hint_lbl = tree.label_muted("Manipulez les objets avec la souris ou Alt + Flèches", leaf(insp_w, 18.0)).unwrap();

                inspector_content.push(prim_title);
                inspector_content.push(prim_row);
                inspector_content.push(hier_title);
                inspector_content.push(parent_btn);
                inspector_content.push(unparent_btn);
                inspector_content.push(actions_title);
                inspector_content.push(dup_btn);
                inspector_content.push(del_btn);
                inspector_content.push(hint_lbl);
            }
            InspectorTab::Material => {
                let cp_title = tree.label("🎨 Couleur Albedo & Espace Colorimétrique", leaf(insp_w, 20.0)).unwrap();
                let color_picker = tree
                    .color_picker(
                        "material_color_picker",
                        state.current_color,
                        state.active_space,
                        leaf(insp_w, 220.0),
                    )
                    .unwrap();
                let rough_lbl = tree.label("Rugosité (Roughness)", leaf(insp_w, 18.0)).unwrap();
                let rough_slider = tree
                    .slider("roughness_slider", 0.0, 1.0, state.roughness, leaf(insp_w, 20.0))
                    .unwrap();
                let metal_lbl = tree.label("Métallique (Metallic)", leaf(insp_w, 18.0)).unwrap();
                let metal_slider = tree
                    .slider("metallic_slider", 0.0, 1.0, state.metallic, leaf(insp_w, 20.0))
                    .unwrap();

                inspector_content.push(cp_title);
                inspector_content.push(color_picker);
                inspector_content.push(rough_lbl);
                inspector_content.push(rough_slider);
                inspector_content.push(metal_lbl);
                inspector_content.push(metal_slider);
            }
            InspectorTab::Physics => {
                let phys_btn = tree
                    .button(
                        "toggle_physics",
                        if state.physics_active { "🕹 Contrôleur Joueur: ACTIF" } else { "🕹 Contrôleur Joueur: INACTIF" },
                        true,
                        leaf(insp_w, 32.0),
                    )
                    .unwrap();
                let grav_lbl = tree.label("Gravité Mondiale (m/s²)", leaf(insp_w, 18.0)).unwrap();
                let grav_slider = tree
                    .slider("gravity_slider", 0.0, 25.0, state.gravity, leaf(insp_w, 20.0))
                    .unwrap();
                let tower_btn = tree
                    .button("btn_spawn_tower", "🏗 Spawner Tour de Cubes (Touche 4)", true, leaf(insp_w, 28.0))
                    .unwrap();
                let explode_btn = tree
                    .button("btn_trigger_explosion", "💥 Déclencher Explosion (Touche E)", true, leaf(insp_w, 32.0))
                    .unwrap();
                let reset_phys = tree
                    .button("reset_player_pos", "↺ Réinitialiser Position Joueur", true, leaf(insp_w, 28.0))
                    .unwrap();

                let controls_box = tree.label_muted("E / Clic Molette: Explosion | 4: Tour | G: Jeu", leaf(insp_w, 36.0)).unwrap();

                inspector_content.push(phys_btn);
                inspector_content.push(grav_lbl);
                inspector_content.push(grav_slider);
                inspector_content.push(tower_btn);
                inspector_content.push(explode_btn);
                inspector_content.push(reset_phys);
                inspector_content.push(controls_box);
            }
            InspectorTab::Animation => {
                let spawn_rigged = tree
                    .button("btn_spawn_humanoid", "🕺 Invoquer Mannequin Riggé (Touche 5)", true, leaf(insp_w, 32.0))
                    .unwrap();

                let anim_clips = ["Marche (Walk)", "Respiration (Idle)", "Salut (Wave)"];
                let clip_selector = tree
                    .segmented_control(
                        "anim_clip_selector",
                        &anim_clips,
                        state.anim_clip_idx,
                        leaf(insp_w / 3.0 - 4.0, 24.0),
                        leaf(insp_w, 30.0),
                    )
                    .unwrap();

                let play_pause_label = if state.anim_playing { "⏸ Mettre en Pause" } else { "▶ Lire l'Animation" };
                let play_btn = tree.button("btn_toggle_anim_play", play_pause_label, true, leaf(insp_w, 28.0)).unwrap();

                let speed_lbl = tree.label(&format!("Vitesse de Lecture ({:.1}x)", state.anim_speed), leaf(insp_w, 18.0)).unwrap();
                let speed_slider = tree
                    .slider("anim_speed_slider", 0.1, 3.0, state.anim_speed, leaf(insp_w, 20.0))
                    .unwrap();

                let jelly_lbl = tree.label("✨ Déformation Gélatineuse (Squash & Stretch)", leaf(insp_w, 18.0)).unwrap();
                let jelly_toggle = tree
                    .button(
                        "btn_toggle_jelly",
                        if state.squash_enabled { "🟢 Squash & Stretch: ACTIF" } else { "⚪ Squash & Stretch: INACTIF" },
                        true,
                        leaf(insp_w, 28.0),
                    )
                    .unwrap();
                let jelly_slider = tree
                    .slider("squash_intensity_slider", 0.05, 0.6, state.squash_intensity, leaf(insp_w, 20.0))
                    .unwrap();

                inspector_content.push(spawn_rigged);
                inspector_content.push(clip_selector);
                inspector_content.push(play_btn);
                inspector_content.push(speed_lbl);
                inspector_content.push(speed_slider);
                inspector_content.push(jelly_lbl);
                inspector_content.push(jelly_toggle);
                inspector_content.push(jelly_slider);
            }
            InspectorTab::Modifiers => {
                let mod_title = tree.label("✨ Pile de Déformateurs Non Destructifs", leaf(insp_w, 20.0)).unwrap();

                let add_twist = tree.button("btn_add_mod_twist", "+ Twist (Torsion)", true, leaf(insp_w * 0.48, 28.0)).unwrap();
                let add_bend = tree.button("btn_add_mod_bend", "+ Bend (Courbure)", true, leaf(insp_w * 0.48, 28.0)).unwrap();
                let row_1 = tree.container(&[add_twist, add_bend], row(8.0)).unwrap();

                let add_taper = tree.button("btn_add_mod_taper", "+ Taper (Évaser)", true, leaf(insp_w * 0.48, 28.0)).unwrap();
                let add_noise = tree.button("btn_add_mod_noise", "+ Noise (Relief)", true, leaf(insp_w * 0.48, 28.0)).unwrap();
                let row_2 = tree.container(&[add_taper, add_noise], row(8.0)).unwrap();

                let clear_mods = tree.button("btn_clear_mods", "🗑 Réinitialiser Modificateurs", true, leaf(insp_w * 0.48, 28.0)).unwrap();
                let apply_mods = tree.button("btn_apply_mods", "💾 Figer Déformation (Apply)", true, leaf(insp_w * 0.48, 28.0)).unwrap();
                let row_3 = tree.container(&[clear_mods, apply_mods], row(8.0)).unwrap();

                let active_title = tree.label("⚙ Modificateurs Actifs sur l'Objet :", leaf(insp_w, 18.0)).unwrap();

                inspector_content.push(mod_title);
                inspector_content.push(row_1);
                inspector_content.push(row_2);
                inspector_content.push(row_3);
                inspector_content.push(active_title);

                if let Some(sc) = scene {
                    if sc.selected_node_idx < sc.nodes.len() {
                        let node = &sc.nodes[sc.selected_node_idx];
                        if node.modifiers.is_empty() {
                            let empty_lbl = tree.label_muted("Aucun modificateur actif. Ajoutez-en un ci-dessus !", leaf(insp_w, 24.0)).unwrap();
                            inspector_content.push(empty_lbl);
                        } else {
                            for (m_idx, m) in node.modifiers.iter().enumerate() {
                                let m_label = tree.label(&format!("{}. {}", m_idx + 1, m.name()), leaf(insp_w, 20.0)).unwrap();
                                inspector_content.push(m_label);
                            }
                        }
                    }
                }
            }
            InspectorTab::Hierarchy => {
                let node_id_strings: Vec<String> = if let Some(sc) = scene {
                    (0..sc.nodes.len()).map(|idx| format!("node_{}", idx)).collect()
                } else {
                    Vec::new()
                };

                let node_labels: Vec<String> = if let Some(sc) = scene {
                    sc.nodes
                        .iter()
                        .map(|node| {
                            if let Some(p) = node.parent_idx {
                                format!("↳ {} (Enfant de #{})", node.name, p)
                            } else {
                                node.name.clone()
                            }
                        })
                        .collect()
                } else {
                    Vec::new()
                };

                let mut tree_items = Vec::new();
                tree_items.push(("root_scene", "🌐 Scène 3D", 0, true, state.expanded_nodes.contains("root_scene"), false));
                if state.expanded_nodes.contains("root_scene") {
                    if let Some(sc) = scene {
                        for (idx, node) in sc.nodes.iter().enumerate() {
                            let is_selected = idx == sc.selected_node_idx;
                            let depth = if node.parent_idx.is_some() { 2 } else { 1 };
                            tree_items.push((node_id_strings[idx].as_str(), node_labels[idx].as_str(), depth, false, false, is_selected));
                        }
                    }
                }

                let tree_view = tree
                    .tree_view(
                        "scene_hierarchy_tree",
                        &tree_items,
                        leaf(insp_w - 8.0, 30.0),
                        leaf(insp_w, 260.0),
                    )
                    .unwrap();
                inspector_content.push(tree_view);
            }
            InspectorTab::Scripting => {
                let script_title = tree.label("📜 Terminal & Moteur de Scripts (DSL / Rhai)", leaf(insp_w, 20.0)).unwrap();

                let dsl_example1 = tree.button("btn_script_spawn_cubes", "⚡ DSL: Spawner Ligne de Cubes", true, leaf(insp_w, 28.0)).unwrap();
                let dsl_example2 = tree.button("btn_script_orbit_torus", "🔄 DSL: Auto-Rotation & Torsion", true, leaf(insp_w, 28.0)).unwrap();
                let rhai_example = tree.button("btn_script_rhai_firework", "🎆 Rhai: Scénario Feu d'Artifice 3D", true, leaf(insp_w, 32.0)).unwrap();
                let clear_scripts = tree.button("btn_script_reset", "🗑 Réinitialiser Scripts Actifs", true, leaf(insp_w, 28.0)).unwrap();

                let help_lbl = tree.label_muted("Commandes DSL: spawn, pos, scale, rotate_auto, impulse, explode, sound", leaf(insp_w, 36.0)).unwrap();

                inspector_content.push(script_title);
                inspector_content.push(dsl_example1);
                inspector_content.push(dsl_example2);
                inspector_content.push(rhai_example);
                inspector_content.push(clear_scripts);
                inspector_content.push(help_lbl);
            }
            _ => {}
        }

        let inspector_inner = tree.container(&inspector_content, column(10.0)).unwrap();
        let inspector_palette = tree
            .palette(
                "inspector_palette",
                "INSPECTEUR",
                state.inspector_folded,
                Some(inspector_inner),
                Style {
                    position: Position::Absolute,
                    inset: Rect {
                        left: length(state.inspector_pos.0.min(width - state.inspector_size.0 - 10.0)),
                        top: length(state.inspector_pos.1),
                        right: auto(),
                        bottom: auto(),
                    },
                    size: Size {
                        width: length(state.inspector_size.0),
                        height: if state.inspector_folded { auto() } else { length(state.inspector_size.1) },
                    },
                    ..Default::default()
                },
            )
            .unwrap();
        root_children.push(inspector_palette);
    }

    // 6. Barre d'Informations et Métriques Inférieure (Cyber HUD Boxed)
    let metrics_bar = if state.blade_runner_mode {
        let speed_str = format!("⚡ {:.0} km/h", state.spinner_speed_kmh);
        let boost_str = format!("🔥 Boost: {:.0}%", state.spinner_boost_fuel);
        let cp_str = format!("🏁 Portes: {}/{}", state.race_checkpoints_passed, state.race_total_checkpoints);
        let score_str = format!("⭐ Score: {}", state.race_score);
        let time_str = format!("⏱ {:.1}s", state.race_time);
        let controls_str = "ZQSD: Vol | Espace/Ctrl: Altitude | Shift: Boost".to_string();

        let speed_badge = tree.badge(speed_str, if state.spinner_speed_kmh > 150.0 { ListItemBadge::Warning } else { ListItemBadge::Success }, leaf(110.0, 24.0)).unwrap();
        let boost_badge = tree.badge(boost_str, if state.spinner_boost_fuel > 20.0 { ListItemBadge::Success } else { ListItemBadge::Warning }, leaf(120.0, 24.0)).unwrap();
        let cp_badge = tree.badge(cp_str, ListItemBadge::None, leaf(110.0, 24.0)).unwrap();
        let score_badge = tree.badge(score_str, ListItemBadge::None, leaf(110.0, 24.0)).unwrap();
        let time_badge = tree.badge(time_str, ListItemBadge::None, leaf(80.0, 24.0)).unwrap();
        let controls_lbl = tree.label_muted(&controls_str, leaf(320.0, 24.0)).unwrap();

        tree.container(
            &[speed_badge, boost_badge, cp_badge, score_badge, time_badge, controls_lbl],
            row(10.0),
        ).unwrap()
    } else {
        let fps_str = format!("{:.0} FPS", metrics.fps);
        let tri_str = format!("{} Triangles", metrics.triangle_count);
        let node_str = format!("{} Objets", metrics.node_count);
        let mode_str = format!("Rendu: {}", metrics.render_mode);
        let cam_str = format!("Caméra: {}", metrics.camera_mode);

        let fps_card = tree.badge(fps_str, ListItemBadge::Success, leaf(90.0, 24.0)).unwrap();
        let tri_card = tree.badge(tri_str, ListItemBadge::Warning, leaf(110.0, 24.0)).unwrap();
        let node_card = tree.badge(node_str, ListItemBadge::None, leaf(100.0, 24.0)).unwrap();
        let mode_card = tree.badge(mode_str, ListItemBadge::None, leaf(140.0, 24.0)).unwrap();
        let cam_card = tree.badge(cam_str, ListItemBadge::None, leaf(140.0, 24.0)).unwrap();
        let status_lbl = tree.label_muted(&metrics.status_text, leaf(280.0, 24.0)).unwrap();

        tree.container(
            &[fps_card, tri_card, node_card, mode_card, cam_card, status_lbl],
            row(10.0),
        ).unwrap()
    };

    let bottom_panel_style = Style {
        position: Position::Absolute,
        inset: Rect {
            left: length(16.0),
            top: auto(),
            right: length(16.0),
            bottom: length(10.0),
        },
        size: Size {
            width: length(width - 32.0),
            height: length(36.0),
        },
        ..Default::default()
    };

    let bottom_hud = tree.container(&[metrics_bar], bottom_panel_style).unwrap();
    root_children.push(bottom_hud);

    tree.container(
        &root_children,
        Style {
            size: Size {
                width: length(width),
                height: length(height),
            },
            ..Default::default()
        },
    )
    .unwrap()
}

/// Construit la couche supérieure (Layer 1) : Menus déroulants, Modals, Toasts avec flou d'arrière-plan
pub fn build_editor_overlays(
    tree: &mut WidgetTree,
    state: &EditorState,
    width: f32,
    height: f32,
) -> Option<NodeId> {
    let mut overlay_nodes = Vec::new();

    // 1. Menu Déroulant / Submenu Popover
    if let Some(active_idx) = state.active_menu {
        let (menu_x, items): (f32, Vec<(&str, &str, Option<&str>, bool)>) = match active_idx {
            0 => (
                16.0,
                vec![
                    ("file_new", "Nouvelle Scène", Some("Ctrl+N"), true),
                    ("file_open", "Ouvrir Fichier 3D (.world, .gltf, .obj)", Some("Ctrl+O"), true),
                    ("file_save", "Sauvegarder Scène", Some("Ctrl+S"), true),
                    ("file_save_as", "Enregistrer sous...", Some("Ctrl+Shift+S"), true),
                    ("file_quit", "Quitter", Some("Alt+F4"), true),
                ],
            ),
            1 => (
                156.0,
                vec![
                    ("add_cube", "Ajouter Cube", Some("1"), true),
                    ("add_pyramid", "Ajouter Pyramide", Some("2"), true),
                    ("add_prism", "Ajouter Prisme", Some("3"), true),
                ],
            ),
            2 => (
                296.0,
                vec![
                    ("render_forward", "Mode Forward PBR (Temps Réel)", None, true),
                    ("render_pathtrace", "Mode PathTracer GPU (Raytracing)", None, true),
                    ("render_raster", "Mode Rasterisation Basique", None, true),
                ],
            ),
            3 => (
                436.0,
                vec![
                    ("game_toggle", "Basculer Mode FPS / Éditeur", Some("G"), true),
                    ("game_cam_1st", "Caméra 1ère Personne", Some("V"), true),
                    ("game_cam_3rd", "Caméra 3ème Personne", None, true),
                    ("game_cam_orb", "Caméra Orbitale", None, true),
                ],
            ),
            4 => (
                576.0,
                vec![
                    ("eco_bladerunner", "🚀 Blade Runner 2099 (Mini-Jeu)", Some("B"), true),
                    ("eco_studio", "Studio Showcase PBR (Démo)", None, true),
                    ("eco_forest", "Générer Forêt Vivante", None, true),
                    ("eco_cove", "Générer Crique Nocturne & Mer", None, true),
                    ("eco_stress", "Grille de Test (25 Objets)", None, true),
                ],
            ),
            _ => (
                716.0,
                vec![
                    ("help_shortcuts", "Guide des Raccourcis", Some("F1"), true),
                    ("help_about", "À propos du Moteur AORUI", None, true),
                ],
            ),
        };

        let item_h = 30.0;
        let popover_w = 280.0;
        let popover_h = items.len() as f32 * (item_h + 3.0) + 12.0;

        let popover = tree
            .menu_popover(
                format!("top_menubar:{}", active_idx),
                &items,
                leaf(popover_w - 12.0, item_h),
                popover_style(menu_x.min(width - popover_w - 10.0), 42.0, popover_w, popover_h),
            )
            .unwrap();
        overlay_nodes.push(popover);
    }

    // 2. Toast de Notification Flottant (si actif)
    if let Some((title, msg, kind, start_time)) = &state.toast {
        if start_time.elapsed().as_secs_f32() < 6.0 {
            let toast_node = tree
                .toast(
                    WidgetId::new("engine_toast"),
                    title,
                    msg,
                    *kind,
                    Style {
                        position: Position::Absolute,
                        inset: Rect {
                            left: auto(),
                            top: auto(),
                            right: length(24.0),
                            bottom: length(56.0),
                        },
                        size: Size {
                            width: length(340.0),
                            height: length(64.0),
                        },
                        ..Default::default()
                    },
                )
                .unwrap();
            overlay_nodes.push(toast_node);
        }
    }

    // 3. Modal Raccourcis Clavier
    if state.show_shortcuts_modal {
        let help_items = [
            tree.label("🎮 Commandes du Mode Jeu (Touche G)", leaf(400.0, 22.0)).unwrap(),
            tree.label_muted("• ZQSD / WASD : Déplacer le personnage", leaf(400.0, 18.0)).unwrap(),
            tree.label_muted("• Espace : Sauter | Shift : Courir | C / Ctrl : Accroupi", leaf(400.0, 18.0)).unwrap(),
            tree.label_muted("• V : Basculer Caméra 1ère / 3ème personne", leaf(400.0, 18.0)).unwrap(),
            tree.label("🛠 Outils d'Édition 3D & Physique", leaf(400.0, 22.0)).unwrap(),
            tree.label_muted("• 1 / 2 / 3 : Ajouter Cube / Pyramide / Prisme", leaf(400.0, 18.0)).unwrap(),
            tree.label_muted("• 4 : Spawner une Tour de Cubes destructibles", leaf(400.0, 18.0)).unwrap(),
            tree.label_muted("• E / Clic Molette : Déclencher une Explosion & Particules", leaf(400.0, 18.0)).unwrap(),
            tree.label_muted("• N : Sélectionner l'objet suivant | Tab : Mode Sommets", leaf(400.0, 18.0)).unwrap(),
            tree.label_muted("• Ctrl+D : Dupliquer | Suppr / Backspace : Supprimer", leaf(400.0, 18.0)).unwrap(),
            tree.label_muted("• Clic Droit + Souris : Rotation de la caméra", leaf(400.0, 18.0)).unwrap(),
            tree.button("close_shortcuts_modal", "Fermer le Guide", true, leaf(400.0, 32.0)).unwrap(),
        ];
        let modal_inner = tree.container(&help_items, column(6.0)).unwrap();
        let modal = tree
            .modal_dialog(
                "shortcuts_modal",
                "GUIDE DES COMMANDES & RACCOURCIS",
                &[modal_inner],
                Style {
                    size: Size {
                        width: length(460.0),
                        height: length(400.0),
                    },
                    ..Default::default()
                },
            )
            .unwrap();
        overlay_nodes.push(modal);
    }

    if overlay_nodes.is_empty() {
        None
    } else {
        Some(tree.container(&overlay_nodes, leaf(width, height)).unwrap())
    }
}
