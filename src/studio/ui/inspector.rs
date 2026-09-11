// [WFGY] Zone: SAFE | λ: 0.25 | Fallbacks: 0/None | Action: Component Inspector panel (transform, PBR material, node graph slot)
#![allow(dead_code)]
use super::style::{column, leaf, row};
use crate::scene::node::Transform;
use crate::scene::Scene;
use crate::studio::nodes::graph::GraphNode;
use crate::studio::nodes::kinds::NodeKind;
use ui_layout::NodeId;
use ui_widgets::{ColorSpace, WidgetId, WidgetTree};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InspectorSection {
    Transform,
    Material,
    Physics,
    Script,
    NodeGraph,
}

#[derive(Debug, Clone)]
pub struct InspectorState {
    pub section: InspectorSection,
    pub active_color_space: ColorSpace,
    pub current_color: [f32; 4],
}

impl Default for InspectorState {
    fn default() -> Self {
        InspectorState {
            section: InspectorSection::Transform,
            active_color_space: ColorSpace::Rgb,
            current_color: [0.1, 0.85, 1.0, 1.0],
        }
    }
}

/// Valeurs des trois axes (translation, rotation, échelle) prêtes pour l'affichage
pub fn axis_values(t: &Transform) -> [[f64; 3]; 3] {
    [
        [t.translation.x as f64, t.translation.y as f64, t.translation.z as f64],
        [t.rotation.x as f64, t.rotation.y as f64, t.rotation.z as f64],
        [t.scale.x as f64, t.scale.y as f64, t.scale.z as f64],
    ]
}

/// Résumé textuel d'un matériau PBR
pub fn material_summary(roughness: f32, metallic: f32, transmission: f32) -> String {
    format!(
        "R {:.2} · M {:.2} · T {:.2}",
        roughness, metallic, transmission
    )
}

fn axis_row(
    tree: &mut WidgetTree,
    prefix: &str,
    vals: [f64; 3],
    step: f64,
    label: &str,
) -> NodeId {
    let mut children = Vec::new();
    children.push(tree.label(label, leaf(24.0, 22.0)).expect("axis label"));
    for (i, axis) in ["X", "Y", "Z"].iter().enumerate() {
        let id = format!("{}_{}", prefix, axis);
        let input = tree
            .number_input(
                WidgetId::new(id),
                vals[i],
                -100000.0,
                100000.0,
                step,
                2,
                false,
                leaf(78.0, 22.0),
            )
            .expect("number input");
        children.push(input);
    }
    tree.container(&children, row(6.0)).expect("axis row")
}

/// Plage [min, max] d'un paramètre de nœud pour l'UI
pub fn param_range(key: &str) -> (f32, f32) {
    match key {
        "scale" => (0.5, 32.0),
        "power" => (0.1, 16.0),
        "frequency" => (0.1, 32.0),
        "speed" => (0.0, 8.0),
        "amplitude" => (0.0, 1.0),
        _ => (0.0, 1.0),
    }
}

/// Construit le panneau d'inspection pour le nœud sélectionné de la scène
pub fn build(
    tree: &mut WidgetTree,
    scene: Option<&Scene>,
    state: &InspectorState,
    graph_node: Option<&GraphNode>,
    width: f32,
    height: f32,
) -> NodeId {
    let tabs = ["Transform", "Matériau", "Physique", "Script", "Node Graph"];
    let active = match state.section {
        InspectorSection::Transform => 0,
        InspectorSection::Material => 1,
        InspectorSection::Physics => 2,
        InspectorSection::Script => 3,
        InspectorSection::NodeGraph => 4,
    };
    let inner_w = (width - 24.0).max(100.0);
    let tab_w = ((inner_w - 8.0) / tabs.len() as f32).max(40.0);
    let tabbar = tree
        .tabbar(
            WidgetId::new("inspector_tabs"),
            &tabs,
            active,
            leaf(tab_w, 24.0),
            leaf(inner_w, 26.0),
        )
        .expect("inspector tabs");

    let mut body_children: Vec<NodeId> = Vec::new();

    if let Some(scene) = scene {
        if let Some(node) = scene.nodes.get(scene.selected_node_idx) {
            body_children.push(
                tree.label(node.name.clone(), leaf(inner_w, 20.0))
                    .expect("node name"),
            );
            match state.section {
                InspectorSection::Transform => {
                    let axes = axis_values(&node.transform);
                    body_children.push(axis_row(tree, "insp_pos", axes[0], 0.1, "Pos"));
                    body_children.push(axis_row(tree, "insp_rot", axes[1], 0.05, "Rot"));
                    body_children.push(axis_row(tree, "insp_scl", axes[2], 0.05, "Sca"));
                }
                InspectorSection::Material => {
                    body_children.push(
                        tree.label(
                            material_summary(
                                node.material.roughness,
                                node.material.metallic,
                                node.material.transmission,
                            ),
                            leaf(inner_w, 18.0),
                        )
                        .expect("material summary"),
                    );
                    body_children.push(
                        tree.label("Rugosité", leaf(inner_w, 16.0)).expect("lbl rough"),
                    );
                    body_children.push(
                        tree.slider(
                            WidgetId::new("insp_roughness"),
                            0.0,
                            1.0,
                            node.material.roughness,
                            leaf(inner_w, 18.0),
                        )
                        .expect("rough slider"),
                    );
                    body_children.push(
                        tree.label("Métallique", leaf(inner_w, 16.0)).expect("lbl metal"),
                    );
                    body_children.push(
                        tree.slider(
                            WidgetId::new("insp_metallic"),
                            0.0,
                            1.0,
                            node.material.metallic,
                            leaf(inner_w, 18.0),
                        )
                        .expect("metal slider"),
                    );
                    let swatch = tree
                        .color_picker(
                            WidgetId::new("insp_color"),
                            state.current_color,
                            state.active_color_space,
                            leaf(inner_w, 180.0),
                        )
                        .expect("color picker");
                    body_children.push(swatch);
                }
                InspectorSection::NodeGraph => match graph_node {
                    Some(node) => {
                        body_children.push(
                            tree.label(node.kind.label(), leaf(inner_w, 18.0))
                                .expect("graph node title"),
                        );
                        let is_color = matches!(node.kind, NodeKind::Rgb | NodeKind::Rgba);
                        if is_color {
                            let col = [
                                node.param_f32("r", 1.0),
                                node.param_f32("g", 1.0),
                                node.param_f32("b", 1.0),
                                node.param_f32("a", 1.0),
                            ];
                            body_children.push(
                                tree.color_picker(
                                    WidgetId::new("gnp_color"),
                                    col,
                                    ColorSpace::Rgb,
                                    leaf(inner_w, 170.0),
                                )
                                .expect("graph color picker"),
                            );
                        }
                        let mut keys: Vec<&String> = node.params.keys().collect();
                        keys.sort();
                        for key in keys {
                            if is_color && matches!(key.as_str(), "r" | "g" | "b") {
                                continue;
                            }
                            let value = node.param_f32(key, 0.0);
                            let (min, max) = param_range(key);
                            body_children.push(
                                tree.label(key.clone(), leaf(inner_w, 15.0))
                                    .expect("param label"),
                            );
                            body_children.push(
                                tree.slider(
                                    WidgetId::new(format!("gnp_{}", key)),
                                    min,
                                    max,
                                    value,
                                    leaf(inner_w, 18.0),
                                )
                                .expect("param slider"),
                            );
                        }
                        body_children.push(
                            tree.label(
                                "U/N/C/M/F : ajouter · Suppr : retirer · glisser un pin à un autre",
                                leaf(inner_w, 14.0),
                            )
                            .expect("graph hint"),
                        );
                    }
                    None => {
                        body_children.push(
                            tree.label(
                                "Ajoutez ou sélectionnez un nœud dans le canvas nodal.",
                                leaf(inner_w, 18.0),
                            )
                            .expect("ng hint"),
                        );
                    }
                },
                _ => {
                    body_children.push(
                        tree.label("Section en cours d'implémentation", leaf(inner_w, 18.0))
                            .expect("placeholder"),
                    );
                }
            }
        } else {
            body_children.push(
                tree.label("Aucun objet sélectionné", leaf(inner_w, 18.0))
                    .expect("empty"),
            );
        }
    } else {
        body_children.push(
            tree.label("Scène non initialisée", leaf(inner_w, 18.0))
                .expect("no scene"),
        );
    }

    let _content_h = (height - 38.0).max(100.0);
    let body = tree
        .container(&body_children, column(6.0))
        .expect("inspector body");

    let inner = tree
        .container(&[tabbar, body], column(8.0))
        .expect("inspector inner");

    tree.palette(
        WidgetId::new("studio_inspector_panel"),
        "INSPECTEUR",
        false,
        Some(inner),
        leaf(width, height),
    )
    .expect("inspector panel")
}

#[cfg(test)]
mod tests {
    use super::*;
    use glam::Vec3;

    #[test]
    fn test_axis_values() {
        let t = Transform {
            translation: Vec3::new(1.0, 2.0, 3.0),
            rotation: Vec3::new(0.0, 1.5, 0.0),
            scale: Vec3::new(2.0, 2.0, 2.0),
        };
        let a = axis_values(&t);
        assert_eq!(a[0], [1.0, 2.0, 3.0]);
        assert_eq!(a[2], [2.0, 2.0, 2.0]);
    }

    #[test]
    fn test_material_summary_format() {
        let s = material_summary(0.25, 0.8, 0.0);
        assert!(s.contains("R 0.25"));
        assert!(s.contains("M 0.80"));
    }
}
