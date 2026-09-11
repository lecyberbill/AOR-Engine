// [WFGY] Zone: SAFE | λ: 0.25 | Fallbacks: 0/None | Action: Advanced ergonomic state for AORUI 3D Engine workstation
#![allow(dead_code)]
use std::collections::HashSet;
use ui_widgets::{ColorSpace, ToastKind};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActiveTool {
    Select,
    Translate,
    Rotate,
    Scale,
    VertexEdit,
    DeformWind,
    DeformWaves,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InspectorTab {
    Hierarchy,
    Transform,
    Material,
    Physics,
    Animation,
    Modifiers,
    Ecosystem,
    Scripting,
}


pub struct EditorState {
    pub active_tool: ActiveTool,
    pub inspector_tab: InspectorTab,
    pub active_menu: Option<usize>,
    pub active_space: ColorSpace,
    pub current_color: [f32; 4],
    
    // Palettes flottantes avec positions et tailles persistantes
    pub show_tools_palette: bool,
    pub tools_pos: (f32, f32),
    pub tools_size: (f32, f32),
    pub tools_folded: bool,

    pub show_deformers_palette: bool,
    pub deformers_pos: (f32, f32),
    pub deformers_size: (f32, f32),
    pub deformers_folded: bool,

    pub show_inspector_palette: bool,
    pub inspector_pos: (f32, f32),
    pub inspector_size: (f32, f32),
    pub inspector_folded: bool,

    // Hiérarchie de scène
    pub expanded_nodes: HashSet<String>,
    pub selected_tree_id: Option<String>,

    // Toasts et Modals
    pub toast: Option<(String, String, ToastKind, std::time::Instant)>,
    pub show_shortcuts_modal: bool,
    pub show_about_modal: bool,

    // Contrôles procéduraux & Déformations
    pub wind_enabled: bool,
    pub wind_strength: f32,
    pub wave_enabled: bool,
    pub wave_amplitude: f32,
    pub wave_frequency: f32,
    pub squash_enabled: bool,
    pub squash_intensity: f32,
    pub physics_active: bool,
    pub gravity: f32,

    // Contrôles d'animation
    pub anim_playing: bool,
    pub anim_speed: f32,
    pub anim_clip_idx: usize,

    // Matériau PBR sélectionné
    pub roughness: f32,
    pub metallic: f32,
    pub emissive: f32,

    // Mode Jeu Blade Runner Spinner
    pub blade_runner_mode: bool,
    pub spinner_speed_kmh: f32,
    pub spinner_boost_fuel: f32,
    pub race_time: f32,
    pub race_checkpoints_passed: usize,
    pub race_total_checkpoints: usize,
    pub race_score: u32,
}


impl Default for EditorState {
    fn default() -> Self {
        let mut expanded_nodes = HashSet::new();
        expanded_nodes.insert("root_scene".to_string());
        expanded_nodes.insert("primitives_group".to_string());

        Self {
            active_tool: ActiveTool::Translate,
            inspector_tab: InspectorTab::Transform,
            active_menu: None,
            active_space: ColorSpace::Rgb,
            current_color: [0.2, 0.6, 1.0, 1.0],
            
            show_tools_palette: true,
            tools_pos: (20.0, 56.0),
            tools_size: (176.0, 240.0),
            tools_folded: false,

            show_deformers_palette: true,
            deformers_pos: (20.0, 316.0),
            deformers_size: (196.0, 230.0),
            deformers_folded: false,

            show_inspector_palette: true,
            inspector_pos: (980.0, 56.0),
            inspector_size: (360.0, 580.0),
            inspector_folded: false,

            expanded_nodes,
            selected_tree_id: Some("node_0".to_string()),

            toast: Some((
                "Poste de Travail 3D Prêt".to_string(),
                "Interface Cyber-Glass organisée & menus déroulants opérationnels.".to_string(),
                ToastKind::Success,
                std::time::Instant::now(),
            )),
            show_shortcuts_modal: false,
            show_about_modal: false,

            wind_enabled: true,
            wind_strength: 0.25,
            wave_enabled: false,
            wave_amplitude: 0.15,
            wave_frequency: 0.8,
            squash_enabled: false,
            squash_intensity: 0.2,
            physics_active: false,
            gravity: 9.81,
            anim_playing: true,
            anim_speed: 1.0,
            anim_clip_idx: 0,
            roughness: 0.5,
            metallic: 0.0,
            emissive: 0.0,

            blade_runner_mode: false,
            spinner_speed_kmh: 0.0,
            spinner_boost_fuel: 100.0,
            race_time: 0.0,
            race_checkpoints_passed: 0,
            race_total_checkpoints: 7,
            race_score: 0,
        }
    }

}

impl EditorState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set_toast(&mut self, title: impl Into<String>, msg: impl Into<String>, kind: ToastKind) {
        self.toast = Some((title.into(), msg.into(), kind, std::time::Instant::now()));
    }
}
