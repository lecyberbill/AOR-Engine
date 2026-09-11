// [WFGY] Zone: SAFE | λ: 0.2 | Fallbacks: 0/None | Action: Clean editing tools decoupled from egui using winit KeyCode
#![allow(unused_imports, dead_code)]
pub mod gizmo;
pub use gizmo::{Gizmo, GizmoAxis};

use std::collections::HashSet;
use winit::keyboard::KeyCode;
use crate::scene::Scene;

/// État d'entrée pour la manipulation 3D et les outils
#[derive(Default, Clone, Debug)]
pub struct EngineInput {
    pub keys_down: HashSet<KeyCode>,
    pub keys_pressed: HashSet<KeyCode>,
    pub alt_down: bool,
    pub shift_down: bool,
    pub ctrl_down: bool,
    pub mouse_pos: glam::Vec2,
    pub mouse_down: bool,
    pub mouse_pressed: bool,
    pub mouse_released: bool,
    pub right_mouse_down: bool,
    pub mouse_delta: (f32, f32),
    pub scroll_delta: f32,
}

impl EngineInput {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn is_key_down(&self, key: KeyCode) -> bool {
        self.keys_down.contains(&key)
    }

    pub fn is_key_pressed(&self, key: KeyCode) -> bool {
        self.keys_pressed.contains(&key)
    }

    pub fn clear_transient(&mut self) {
        self.keys_pressed.clear();
        self.mouse_pressed = false;
        self.mouse_released = false;
        self.mouse_delta = (0.0, 0.0);
        self.scroll_delta = 0.0;
    }
}

pub trait Tool {
    fn update(&mut self, scene: &mut Scene, input: &EngineInput);
}

pub struct VertexEditTool {
    pub edit_speed: f32,
}

impl Tool for VertexEditTool {
    fn update(&mut self, scene: &mut Scene, input: &EngineInput) {
        if !scene.is_edit_mode || scene.nodes.is_empty() {
            return;
        }

        let node_idx = scene.selected_node_idx;
        if node_idx >= scene.nodes.len() {
            return;
        }

        let node = &mut scene.nodes[node_idx];

        // Changement de sommet (sélection successive)
        if input.is_key_pressed(KeyCode::Space) {
            if !node.vertices.is_empty() {
                node.selected_vertex_idx = (node.selected_vertex_idx + 1) % node.vertices.len();
            }
        }

        if node.selected_vertex_idx >= node.vertices.len() {
            return;
        }

        // Édition du sommet sélectionné
        if input.is_key_down(KeyCode::KeyJ) { // X - gauche
            node.vertices[node.selected_vertex_idx].position[0] -= self.edit_speed;
        }
        if input.is_key_down(KeyCode::KeyL) { // X - droite
            node.vertices[node.selected_vertex_idx].position[0] += self.edit_speed;
        }
        if input.is_key_down(KeyCode::KeyI) { // Y - haut
            node.vertices[node.selected_vertex_idx].position[1] += self.edit_speed;
        }
        if input.is_key_down(KeyCode::KeyK) { // Y - bas
            node.vertices[node.selected_vertex_idx].position[1] -= self.edit_speed;
        }
        if input.is_key_down(KeyCode::KeyU) { // Z - proche
            node.vertices[node.selected_vertex_idx].position[2] -= self.edit_speed;
        }
        if input.is_key_down(KeyCode::KeyO) { // Z - loin
            node.vertices[node.selected_vertex_idx].position[2] += self.edit_speed;
        }
    }
}

pub struct GlobalTransformTool {
    pub translation_speed: f32,
    pub scale_speed: f32,
}

impl Tool for GlobalTransformTool {
    fn update(&mut self, scene: &mut Scene, input: &EngineInput) {
        if scene.nodes.is_empty() {
            return;
        }

        // Nécessite Alt pour éviter les conflits avec la saisie
        if !input.alt_down {
            return;
        }

        let node_idx = scene.selected_node_idx;
        if node_idx >= scene.nodes.len() {
            return;
        }

        let node = &mut scene.nodes[node_idx];

        // Translation globale (Alt + Flèches / Alt + Touches)
        if input.is_key_down(KeyCode::KeyA) || input.is_key_down(KeyCode::ArrowLeft) {
            node.transform.translation.x -= self.translation_speed;
        }
        if input.is_key_down(KeyCode::KeyD) || input.is_key_down(KeyCode::ArrowRight) {
            node.transform.translation.x += self.translation_speed;
        }
        if input.is_key_down(KeyCode::KeyW) || input.is_key_down(KeyCode::ArrowUp) {
            node.transform.translation.y += self.translation_speed;
        }
        if input.is_key_down(KeyCode::KeyS) || input.is_key_down(KeyCode::ArrowDown) {
            node.transform.translation.y -= self.translation_speed;
        }
        if input.is_key_down(KeyCode::KeyQ) || input.is_key_down(KeyCode::PageUp) {
            node.transform.translation.z -= self.translation_speed;
        }
        if input.is_key_down(KeyCode::KeyE) || input.is_key_down(KeyCode::PageDown) {
            node.transform.translation.z += self.translation_speed;
        }

        // Échelle globale (Scale)
        if input.is_key_down(KeyCode::Equal) || input.is_key_down(KeyCode::NumpadAdd) {
            node.transform.scale += glam::Vec3::splat(self.scale_speed);
        }
        if input.is_key_down(KeyCode::Minus) || input.is_key_down(KeyCode::NumpadSubtract) {
            node.transform.scale = (node.transform.scale - glam::Vec3::splat(self.scale_speed)).max(glam::Vec3::splat(0.1));
        }
    }
}
