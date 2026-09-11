// [WFGY] Zone: SAFE | λ: 0.3 | Fallbacks: 0/None | Action: Advanced 3D Engine Workstation with AORUI Cyber-Glass Interface & Complete Submenus
#![allow(dead_code)]
mod gpu;
mod scene;
mod renderer;
mod tools;
mod ui;
mod physics;
mod animation;
mod particles;
mod audio;
mod ecs;
mod scripting;
mod game;
mod studio;

use std::sync::Arc;
use glam::{Vec2, Vec3};
use winit::application::ApplicationHandler;
use winit::event::{ElementState, MouseButton, MouseScrollDelta, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::keyboard::{KeyCode, PhysicalKey};
use winit::window::{Window, WindowAttributes, WindowId};

use ui_core::UiEvent;
use ui_gpu::GpuRenderer;
use ui_layout::{AvailableSpace, Size};
use ui_widgets::{
    FontFamily, InteractionKey, InteractionState, Theme, ToastKind, WidgetTree,
};

use gpu::{GpuContext, RenderTargetTexture};
use scene::{Camera, CameraMode, CoveEcosystem, ForestEcosystem, PrimitiveType, Scene};
use renderer::{ForwardRenderer, PathTracerConfig, PathTracerPipeline, PostProcessPipeline, RenderMode};
use tools::{EngineInput, Gizmo, GlobalTransformTool, Tool, VertexEditTool};
use physics::PhysicsEngine;
use animation::MeshDeformer;
use particles::ParticleSystem;
use audio::{SoundEffect, SpatialAudioEngine};
use ui::editor_state::{ActiveTool, EditorState, InspectorTab};
use ui::editor_view::{build_editor_base, build_editor_overlays, EngineMetrics};
use studio::NodeKind;
use studio::ui::{InspectorSection, LogLevel, StudioCenterTab, StudioShellState};

fn main() {
    let event_loop = EventLoop::new().expect("Failed to create event loop");
    event_loop.set_control_flow(ControlFlow::Poll);

    let mut app = EngineApplication::new();
    let _ = event_loop.run_app(&mut app);
}

fn font_family(family: &FontFamily) -> glyphon::Family<'_> {
    match family {
        FontFamily::SansSerif => glyphon::Family::SansSerif,
        FontFamily::Monospace => glyphon::Family::Monospace,
        FontFamily::Serif => glyphon::Family::Serif,
        FontFamily::Named(name) => glyphon::Family::Name(name.as_str()),
    }
}

/// Construit l'arbre UI du Game Studio (docks, canvas nodal, navigateur, console).
fn build_studio_layout(
    tree: &mut WidgetTree,
    session: Option<&studio::StudioSession>,
    shell: &StudioShellState,
    scene: Option<&Scene>,
    width: f32,
    height: f32,
) -> studio::ui::StudioShellLayout {
    let db = session.map(|s| s.database());
    let graph = session.map(|s| &s.graph);
    studio::ui::build_studio_shell(tree, scene, db, graph, shell, width, height)
}

/// Thème sombre du studio : surfaces opaques et halo discret
fn dark_theme() -> Theme {
    let mut t = Theme::cyber_glass();
    t.glass_bg = [0.055, 0.065, 0.085, 0.96];
    t.text_color = [0.90, 0.93, 0.97, 1.0];
    t.text_muted = [0.52, 0.58, 0.68, 1.0];
    t.accent = [0.35, 0.75, 1.0, 1.0];
    t.accent_secondary = [0.55, 0.40, 0.95, 1.0];
    t.glow_intensity = 0.05;
    t.glow_intensity_hover = 0.22;
    t.corner_radius = 8.0;
    t.border_width = 1.0;
    t
}

struct EngineApplication {
    window: Option<Arc<Window>>,
    renderer: Option<GpuRenderer>,
    resources: ui_gpu::ResourceTable,
    theme: Theme,
    
    // Éléments GPU et Moteur 3D
    gpu_context: Option<GpuContext>,
    render_target: Option<RenderTargetTexture>,
    postprocess_target: Option<RenderTargetTexture>,
    viewport_gpu_texture: Option<Arc<ui_gpu::GpuTexture>>,
    forward_renderer: Option<ForwardRenderer>,
    pathtracer: Option<PathTracerPipeline>,
    postprocess: Option<PostProcessPipeline>,
    pathtrace_config: PathTracerConfig,
    render_mode: RenderMode,
    particles: Option<ParticleSystem>,

    // Audio Spatialisé 3D
    audio: SpatialAudioEngine,

    // Moteur de Scripting (DSL + Rhai)
    scripting: scripting::ScriptingEngine,

    // Scène et simulation
    camera: Camera,
    scene: Option<Scene>,
    physics: PhysicsEngine,
    gizmo: Gizmo,
    tools: Vec<Box<dyn Tool>>,
    editor_state: EditorState,
    input: EngineInput,
    forest_eco: ForestEcosystem,
    cove_eco: CoveEcosystem,

    // Mini-Jeu Blade Runner
    spinner: game::CyberSpinner,
    cyber_city: game::CyberCitySystem,
    spinner_node_idx: Option<usize>,

    
    // Métriques et temps
    last_frame_time: std::time::Instant,
    fps: f32,
    total_time: f32,
    wind_time: f32,
    base_vertices: std::collections::HashMap<usize, Vec<[f32; 3]>>,
    current_world_path: Option<std::path::PathBuf>,
    status_msg: String,

    // États d'interaction UI
    cursor_pos: (f32, f32),
    pressed: Option<InteractionKey>,
    camera_dragging: bool,
    last_mouse_pos: (f32, f32),

    // Drag & Resize interactifs des palettes
    palette_drag: Option<(String, (f32, f32))>,
    palette_resize: Option<(String, (f32, f32), (f32, f32))>,
    slider_drag: Option<String>,
    color_picker_drag: Option<String>,

    // === AOR Game Studio (Phases 1-5) ===
    studio: Option<studio::StudioSession>,
    studio_shell: StudioShellState,
    studio_mode: bool,
    studio_canvas_dragging: bool,
    studio_canvas_bounds: Option<[f32; 4]>,
    studio_viewport_bounds: Option<[f32; 4]>,
    dark_mode: bool,
}

impl EngineApplication {
    fn new() -> Self {
        let camera = Camera::new(
            Vec3::new(0.0, 0.0, 0.0),
            6.0,
            0.0,
            0.2,
        );

        let tools: Vec<Box<dyn Tool>> = vec![
            Box::new(VertexEditTool { edit_speed: 0.05 }),
            Box::new(GlobalTransformTool { translation_speed: 0.05, scale_speed: 0.02 }),
        ];

        Self {
            window: None,
            renderer: None,
            resources: ui_gpu::ResourceTable::new(),
            theme: dark_theme(),
            gpu_context: None,
            render_target: None,
            postprocess_target: None,
            viewport_gpu_texture: None,
            forward_renderer: None,
            pathtracer: None,
            postprocess: None,
            pathtrace_config: PathTracerConfig::default(),
            render_mode: RenderMode::VideoGame,
            particles: None,
            audio: SpatialAudioEngine::new(),
            scripting: scripting::ScriptingEngine::new(),
            camera,
            scene: None,
            physics: PhysicsEngine::new(),

            gizmo: Gizmo::new(),
            tools,
            editor_state: EditorState::new(),
            input: EngineInput::new(),
            forest_eco: ForestEcosystem::new(),
            cove_eco: CoveEcosystem::new(),
            spinner: game::CyberSpinner::default(),
            cyber_city: game::CyberCitySystem::default(),
            spinner_node_idx: None,
            last_frame_time: std::time::Instant::now(),
            fps: 60.0,
            total_time: 0.0,
            wind_time: 0.0,
            base_vertices: std::collections::HashMap::new(),
            current_world_path: None,
            status_msg: "Poste 3D Prêt".to_string(),
            cursor_pos: (0.0, 0.0),
            pressed: None,
            camera_dragging: false,
            last_mouse_pos: (0.0, 0.0),
            palette_drag: None,
            palette_resize: None,
            slider_drag: None,
            color_picker_drag: None,
            studio: None,
            studio_shell: StudioShellState::default(),
            studio_mode: true,
            studio_canvas_dragging: false,
            studio_canvas_bounds: None,
            studio_viewport_bounds: None,
            dark_mode: true,
        }
    }

    fn init_graphics(&mut self, window: Arc<Window>) {
        let renderer = GpuRenderer::new(window.clone());
        let device = renderer.device_arc();
        let queue = renderer.queue_arc();
        let surface_format = renderer.surface_format();

        let context = GpuContext::from_shared(
            device,
            queue,
            surface_format,
            wgpu::AdapterInfo {
                name: "AORUI Primary GPU Adapter".to_string(),
                vendor: 0,
                device: 0,
                device_type: wgpu::DeviceType::DiscreteGpu,
                driver: "Vulkan/Metal/DX12".to_string(),
                driver_info: "".to_string(),
                backend: wgpu::Backend::Vulkan,
            },
        );

        let size = window.inner_size();
        let rt_width = size.width.max(1);
        let rt_height = size.height.max(1);
        let rt_format = wgpu::TextureFormat::Rgba8UnormSrgb;

        let render_target = RenderTargetTexture::new(
            &context.device,
            rt_width,
            rt_height,
            rt_format,
            Some("AORUI 3D Viewport Target"),
        );

        let postprocess_target = RenderTargetTexture::new(
            &context.device,
            rt_width,
            rt_height,
            rt_format,
            Some("AORUI PostProcess Target"),
        );

        let scene = Scene::new(&context.device, &context.queue);
        let forward_renderer = ForwardRenderer::new(&context, rt_format, &scene.model_bind_group_layout);
        let postprocess = PostProcessPipeline::new(&context.device, rt_format);

        let mut pathtracer = PathTracerPipeline::new(&context.device, &context.queue, rt_format);
        pathtracer.resize(&context.device, rt_width, rt_height);
        pathtracer.update_scene(&context.device, &scene.nodes);

        let particles = ParticleSystem::new(&context, rt_format, 2000);

        // Texture RGBA GPU enregistrée dans la table de ressources AORUI
        let initial_pixels = vec![20u8; (rt_width * rt_height * 4) as usize];
        let viewport_gpu_texture = renderer.create_texture_rgba(rt_width, rt_height, &initial_pixels);
        self.resources.insert("viewport3d", viewport_gpu_texture.clone());

        // Positionner l'inspecteur à droite en fonction de la largeur de fenêtre
        self.editor_state.inspector_pos.0 = (size.width as f32 - self.editor_state.inspector_size.0 - 20.0).max(200.0);

        self.gpu_context = Some(context);
        self.render_target = Some(render_target);
        self.postprocess_target = Some(postprocess_target);
        self.viewport_gpu_texture = Some(viewport_gpu_texture);
        self.scene = Some(scene);
        self.forward_renderer = Some(forward_renderer);
        self.postprocess = Some(postprocess);
        self.pathtracer = Some(pathtracer);
        self.particles = Some(particles);
        self.renderer = Some(renderer);
        self.window = Some(window);

        self.init_studio();
    }

    /// Initialise la session Game Studio : projet sur disque, base d'assets et graphe de démonstration.
    fn init_studio(&mut self) {
        let root = std::env::current_dir()
            .unwrap_or_default()
            .join("AORProject");

        let mut session = match studio::StudioSession::open(&root) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("[Studio] impossible d'ouvrir le projet: {e}");
                return;
            }
        };

        // Graphe de matériau PBR de démonstration (Noise -> Mix -> Albedo, Fresnel -> Emissive)
        {
            let g = &mut session.graph;
            // Positionner le nœud de sortie à droite
            if let Some(out_node) = g.node_mut(g.output_node_id) {
                out_node.position = [820.0, 80.0];
            }
            let uv = g.add_node(NodeKind::Uv, [40.0, 60.0]);
            let noise = g.add_node(NodeKind::PerlinNoise, [280.0, 60.0]);
            let color = g.add_node(NodeKind::Rgb, [280.0, 240.0]);
            let mix = g.add_node(NodeKind::Mix, [540.0, 100.0]);
            let fres = g.add_node(NodeKind::Fresnel, [540.0, 320.0]);
            let out = g.output_node_id;
            let _ = g.connect(uv, "UV", noise, "UV");
            let _ = g.connect(noise, "Out", mix, "T");
            let _ = g.connect(color, "Color", mix, "A");
            let _ = g.connect(mix, "Out", out, "Albedo");
            let _ = g.connect(fres, "Out", out, "Emissive");

            // Réglages néon par défaut
            if let Some(n) = g.node_mut(color) {
                n.set_param_f32("r", 0.0);
                n.set_param_f32("g", 0.85);
                n.set_param_f32("b", 1.0);
            }
            if let Some(n) = g.node_mut(noise) {
                n.set_param_f32("scale", 5.0);
            }
            if let Some(n) = g.node_mut(fres) {
                n.set_param_f32("power", 3.0);
                n.set_param_f32("bias", 0.25);
            }
        }

        let compile_msg = match studio::compile_and_validate(&session.graph) {
            Ok(shader) => format!(
                "Matériau compilé & validé ({} nœuds, {} helpers, {} textures).",
                shader.node_count,
                shader.helpers.len(),
                shader.texture_bindings
            ),
            Err(e) => format!("Échec compilation matériau: {e}"),
        };

        let assets = session.asset_count();
        self.studio = Some(session);
        self.studio_shell
            .console
            .push(format!("Projet AOR ouvert ({} assets indexés).", assets), LogLevel::Info);
        self.studio_shell.console.push(compile_msg, LogLevel::Success);
        self.studio_shell.console.push(
            "F10 : basculer Studio ↔ Atelier 3D".to_string(),
            LogLevel::Info,
        );
    }

    fn update_physics_and_simulation(&mut self, dt: f32) {
        self.total_time += dt;

        // Synchronisation des nœuds de la scène avec le moteur de physique
        if let Some(scene) = &mut self.scene {
            self.physics.sync_from_scene_nodes(&scene.nodes);
        }

        // Mise à jour de la physique
        if self.physics.is_active {
            let mut move_input = Vec3::ZERO;
            if self.input.is_key_down(KeyCode::KeyW) || self.input.is_key_down(KeyCode::KeyZ) || self.input.is_key_down(KeyCode::ArrowUp) {
                move_input.z += 1.0;
            }
            if self.input.is_key_down(KeyCode::KeyS) || self.input.is_key_down(KeyCode::ArrowDown) {
                move_input.z -= 1.0;
            }
            if self.input.is_key_down(KeyCode::KeyA) || self.input.is_key_down(KeyCode::KeyQ) || self.input.is_key_down(KeyCode::ArrowLeft) {
                move_input.x -= 1.0;
            }
            if self.input.is_key_down(KeyCode::KeyD) || self.input.is_key_down(KeyCode::ArrowRight) {
                move_input.x += 1.0;
            }

            let jump = self.input.is_key_pressed(KeyCode::Space);
            let sprint = self.input.shift_down;
            let crouch = self.input.ctrl_down || self.input.is_key_down(KeyCode::KeyC);

            if let Some(scene) = &self.scene {
                self.physics.step(
                    dt,
                    move_input,
                    jump,
                    sprint,
                    crouch,
                    &scene.nodes,
                );
            }

            // Répercuter les mouvements physiques sur les objets de la scène
            if let (Some(scene), Some(gpu)) = (&mut self.scene, &self.gpu_context) {
                self.physics.sync_to_scene_nodes(&mut scene.nodes);
                for (idx, node) in scene.nodes.iter_mut().enumerate() {
                    node.update_gpu(&gpu.queue, idx == scene.selected_node_idx);
                }
            }

            // Synchronisation de la caméra avec le joueur
            let player_pos = self.physics.player.position;
            match self.camera.mode {
                CameraMode::FirstPerson => {
                    self.camera.target = player_pos + Vec3::new(0.0, 1.65, 0.0);
                    self.camera.yaw = self.physics.player.yaw;
                    self.camera.pitch = self.physics.player.pitch;
                }
                CameraMode::ThirdPerson => {
                    self.camera.target = player_pos + Vec3::new(0.0, 1.2, 0.0);
                    self.camera.yaw = self.physics.player.yaw;
                    self.camera.pitch = self.physics.player.pitch;
                }
                _ => {}
            }

            if jump {
                self.audio.play_spatial_sound(SoundEffect::Jump, player_pos, 0.6);
            }
        }

        // ==========================================
        // Mini-Jeu : Blade Runner Cyber Spinner 2099
        // ==========================================
        if self.editor_state.blade_runner_mode {
            let mut throttle = 0.0;
            let mut steer_yaw = 0.0;
            let mut pitch_input = 0.0;
            let mut strafe = 0.0;
            let mut vertical = 0.0;

            if self.input.is_key_down(KeyCode::KeyW) || self.input.is_key_down(KeyCode::KeyZ) || self.input.is_key_down(KeyCode::ArrowUp) {
                throttle += 1.0;
            }
            if self.input.is_key_down(KeyCode::KeyS) || self.input.is_key_down(KeyCode::ArrowDown) {
                throttle -= 0.6;
            }
            if self.input.is_key_down(KeyCode::KeyA) || self.input.is_key_down(KeyCode::KeyQ) || self.input.is_key_down(KeyCode::ArrowLeft) {
                steer_yaw += 1.0;
            }
            if self.input.is_key_down(KeyCode::KeyD) || self.input.is_key_down(KeyCode::ArrowRight) {
                steer_yaw -= 1.0;
            }
            if self.input.is_key_down(KeyCode::Space) {
                vertical += 1.0;
            }
            if self.input.ctrl_down || self.input.is_key_down(KeyCode::KeyC) {
                vertical -= 1.0;
            }
            if self.input.is_key_down(KeyCode::KeyE) {
                strafe += 1.0;
            }
            if self.input.is_key_down(KeyCode::KeyR) {
                pitch_input += 1.0;
            }
            if self.input.is_key_down(KeyCode::KeyF) {
                pitch_input -= 1.0;
            }

            let boost_requested = self.input.shift_down;

            // 1. Mise à jour de la physique de vol du Spinner
            self.spinner.update(dt, throttle, steer_yaw, pitch_input, strafe, vertical, boost_requested);

            // 2. Synchronisation de la position du maillage 3D du Spinner
            if let (Some(scene), Some(gpu)) = (&mut self.scene, &self.gpu_context) {
                if let Some(s_idx) = self.spinner_node_idx {
                    if let Some(node) = scene.nodes.get_mut(s_idx) {
                        node.transform.translation = self.spinner.position;
                        node.transform.rotation = self.spinner.rotation;
                        node.update_gpu(&gpu.queue, false);
                    }
                }
            }

            // 3. Suivi dynamique de la caméra de poursuite 3ème personne (Chase Camera)
            let rot_quat = glam::Quat::from_euler(glam::EulerRot::YXZ, self.spinner.rotation.y, self.spinner.rotation.x, self.spinner.rotation.z);
            let _cam_offset = rot_quat * Vec3::new(0.0, 2.2, -6.5);
            self.camera.target = self.spinner.position + Vec3::new(0.0, 0.8, 0.0);
            self.camera.yaw = self.spinner.rotation.y + std::f32::consts::PI;
            self.camera.pitch = (self.spinner.rotation.x * 0.5 + 0.15).clamp(-0.8, 0.8);

            // 4. Émission de plasma et particules de réacteurs
            if let Some(particles) = &mut self.particles {
                let (left_ex, right_ex) = self.spinner.exhaust_positions();
                let back_dir = -(rot_quat * Vec3::Z).normalize_or_zero();
                particles.spawn_sparks(left_ex, back_dir, if self.spinner.is_boosting { 6 } else { 2 });
                particles.spawn_sparks(right_ex, back_dir, if self.spinner.is_boosting { 6 } else { 2 });
            }

            // 5. Détection de franchissement des checkpoints de course
            if let Some(passed_idx) = self.cyber_city.update(dt, self.spinner.position) {
                self.audio.play_spatial_sound(SoundEffect::Laser, self.spinner.position, 1.2);
                self.editor_state.set_toast(
                    "🏁 Checkpoint Franchi !",
                    format!("Anneau #{}/{} validé ! +500 pts", passed_idx + 1, self.cyber_city.checkpoints.len()),
                    ToastKind::Success,
                );
            }

            // 6. Mise à jour de l'état UI
            self.editor_state.spinner_speed_kmh = self.spinner.speed_kmh();
            self.editor_state.spinner_boost_fuel = self.spinner.boost_fuel;
            self.editor_state.race_time = self.cyber_city.race_time;
            self.editor_state.race_checkpoints_passed = self.cyber_city.current_checkpoint_idx;
            self.editor_state.race_score = self.cyber_city.score;
        }

        // Mise à jour de l'auditeur audio spatial (Caméra 3D)
        let cam_eye = self.camera.position();
        let cam_fwd = self.camera.forward();
        let cam_right = cam_fwd.cross(Vec3::Y).normalize_or_zero();
        self.audio.update_listener(cam_eye, cam_fwd, cam_right);
        self.audio.update();



        // Mise à jour du système de particules (feu, étincelles, fumée)
        if let Some(particles) = &mut self.particles {
            particles.update(dt, self.physics.gravity);
        }

        // Déformations procédurales de maillages & Squelettes Riggés
        if let (Some(scene), Some(gpu)) = (&mut self.scene, &self.gpu_context) {
            let anim_playing = self.editor_state.anim_playing;
            let anim_speed = self.editor_state.anim_speed;
            let anim_clip_idx = self.editor_state.anim_clip_idx;

            for (idx, node) in scene.nodes.iter_mut().enumerate() {
                let mut vertices_modified = false;

                // 1. Animation Squelettique (Rigged Model + Linear Blend Skinning)
                if let Some(rigged) = &mut node.rigged_model {
                    rigged.player.is_playing = anim_playing;
                    rigged.player.speed = anim_speed;
                    if anim_clip_idx < rigged.player.clips.len() {
                        rigged.player.current_clip_idx = anim_clip_idx;
                    }

                    rigged.player.update(dt, &mut rigged.skeleton);
                    node.vertices = rigged.compute_skinned_vertices();
                    vertices_modified = true;
                }

                // 2. Déformation de Vent
                if self.editor_state.wind_enabled && node.rigged_model.is_none() {
                    self.wind_time += dt;
                    let wind_dir = Vec3::new(1.0, 0.0, 0.5).normalize();
                    let base = self.base_vertices.entry(idx).or_insert_with(|| {
                        node.vertices.iter().map(|v| v.position).collect()
                    });

                    MeshDeformer::apply_wind(
                        &mut node.vertices,
                        base,
                        self.wind_time,
                        self.editor_state.wind_strength,
                        wind_dir,
                    );
                    vertices_modified = true;
                }

                // 3. Déformation Squash & Stretch Gélatineuse
                if self.editor_state.squash_enabled && node.rigged_model.is_none() {
                    let base = self.base_vertices.entry(idx).or_insert_with(|| {
                        node.vertices.iter().map(|v| v.position).collect()
                    });

                    MeshDeformer::apply_squash_and_stretch(
                        &mut node.vertices,
                        base,
                        self.total_time,
                        self.editor_state.squash_intensity,
                    );
                    vertices_modified = true;
                }

                // 4. Pile de Déformateurs Non Destructifs (Modifier Stack: Twist, Bend, Taper, Noise...)
                if node.evaluate_modifiers(self.total_time) {
                    vertices_modified = true;
                }

                if vertices_modified {
                    node.update_vertices_gpu(&gpu.queue);
                }
            }

            // Mise à jour de la hiérarchie parentale et propagation des matrices mondes
            scene.update_hierarchy_gpu(&gpu.queue);

            // Exécution des scripts périodiques (DSL & Rhai)
            let mut script_ctx = scripting::ScriptContext {
                scene,
                physics: &mut self.physics,
                particles: self.particles.as_mut(),
                audio: &mut self.audio,
                device: &gpu.device,
                queue: &gpu.queue,
                delta_time: dt,
                current_time: self.total_time,
            };
            self.scripting.update(&mut script_ctx);
        }
    }



    /// Déclenche une explosion physique spectaculaire avec onde de choc et particules
    pub fn trigger_explosion(&mut self, epicenter: Vec3, force: f32) {
        // 1. Onde de choc physique sur les corps rigides
        let affected = self.physics.explode_at(epicenter, 14.0, force);
        if let (Some(scene), Some(gpu)) = (&mut self.scene, &self.gpu_context) {
            self.physics.sync_to_scene_nodes(&mut scene.nodes);
            for (idx, node) in scene.nodes.iter_mut().enumerate() {
                node.update_gpu(&gpu.queue, idx == scene.selected_node_idx);
            }
        }

        // 2. Émission de particules d'explosion (boule de feu, shrapnels, fumée)
        if let Some(particles) = &mut self.particles {
            particles.spawn_explosion(epicenter, 1.8);
        }

        // 3. Déclenchement d'un événement sonore 3D spatialisé
        self.audio.play_spatial_sound(SoundEffect::Explosion, epicenter, 1.0);

        // 4. Flash lumineux temporaire (lumière ponctuelle vive)
        if let Some(renderer) = &mut self.forward_renderer {
            renderer.light_pos = epicenter + Vec3::new(0.0, 2.0, 0.0);
        }

        self.editor_state.set_toast(
            "💥 Explosion !",
            format!("Détonation ({} objets propulsés)", affected.len()),
            ToastKind::Warning,
        );
    }


    fn render_3d_viewport(&mut self) {
        let Some(gpu) = &self.gpu_context else { return; };

        // Déterminer la taille cible et le ratio d'aspect selon le viewport effectif
        let (target_w, target_h, aspect) = if self.studio_mode {
            if let Some(vp_bounds) = self.studio_viewport_bounds {
                let w = (vp_bounds[2].round() as u32).max(64);
                let h = (vp_bounds[3].round() as u32).max(64);
                (w, h, (w as f32 / h as f32).max(0.1))
            } else if let Some(rt) = &self.render_target {
                (rt.width, rt.height, rt.aspect_ratio())
            } else {
                return;
            }
        } else if let Some(rt) = &self.render_target {
            (rt.width, rt.height, rt.aspect_ratio())
        } else {
            return;
        };

        // Redimensionner les cibles GPU si la taille du viewport 3D a changé
        let need_resize = self.render_target.as_ref().map(|rt| rt.width != target_w || rt.height != target_h).unwrap_or(true);
        if need_resize {
            let new_rt = RenderTargetTexture::new(
                &gpu.device,
                target_w,
                target_h,
                wgpu::TextureFormat::Rgba8UnormSrgb,
                Some("AORUI 3D Viewport Target"),
            );
            self.render_target = Some(new_rt);
            if let Some(post_target) = &mut self.postprocess_target {
                *post_target = RenderTargetTexture::new(
                    &gpu.device,
                    target_w,
                    target_h,
                    wgpu::TextureFormat::Rgba8UnormSrgb,
                    Some("AORUI PostProcess Target"),
                );
            }
            if let Some(renderer) = &self.renderer {
                let initial_pixels = vec![20u8; (target_w * target_h * 4) as usize];
                let new_tex = renderer.create_texture_rgba(target_w, target_h, &initial_pixels);
                self.resources.insert("viewport3d", new_tex.clone());
                self.viewport_gpu_texture = Some(new_tex);
            }
            if let Some(pathtracer) = &mut self.pathtracer {
                pathtracer.resize(&gpu.device, target_w, target_h);
            }
        }

        let Some(scene) = &mut self.scene else { return; };
        let Some(render_target) = &self.render_target else { return; };
        let Some(viewport_tex) = &self.viewport_gpu_texture else { return; };

        // Préparer les particules sur le GPU
        if let Some(particles) = &mut self.particles {
            particles.prepare_gpu(&gpu.queue, &self.camera, aspect, self.total_time);
        }

        let mut encoder = gpu.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("viewport render encoder"),
        });

        match self.render_mode {
            RenderMode::VideoGame | RenderMode::Rasterize => {
                if let Some(renderer) = &mut self.forward_renderer {
                    renderer.render(
                        gpu,
                        &render_target.color_view,
                        &render_target.depth_texture.view,
                        scene,
                        &self.camera,
                        aspect,
                        wgpu::Color { r: 0.05, g: 0.07, b: 0.12, a: 1.0 },
                        self.total_time,
                        target_w as f32,
                        target_h as f32,
                        self.render_mode,
                        self.particles.as_ref(),
                    );
                }

                // Passe de Post-Processing HDR & Bloom
                if let (Some(postprocess), Some(post_target)) = (&self.postprocess, &self.postprocess_target) {
                    let bind_group = postprocess.create_bind_group(&gpu.device, &render_target.color_view);
                    postprocess.render(&mut encoder, &bind_group, &post_target.color_view);
                }
            }
            RenderMode::PathTrace => {
                if let Some(pathtracer) = &mut self.pathtracer {
                    pathtracer.render(
                        &gpu.device,
                        &gpu.queue,
                        &mut encoder,
                        &render_target.color_view,
                        &self.camera,
                        &self.pathtrace_config,
                    );
                }
            }
        }

        // Texture source finale à envoyer vers AORUI (postprocess_target si actif, sinon render_target)
        let source_texture = if self.render_mode != RenderMode::PathTrace && self.postprocess_target.is_some() {
            &self.postprocess_target.as_ref().unwrap().texture
        } else {
            &render_target.texture
        };

        // Synchroniser la texture du viewport pour AORUI
        encoder.copy_texture_to_texture(
            wgpu::ImageCopyTexture {
                texture: source_texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::ImageCopyTexture {
                texture: &viewport_tex.texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::Extent3d {
                width: render_target.width,
                height: render_target.height,
                depth_or_array_layers: 1,
            },
        );


        gpu.queue.submit(std::iter::once(encoder.finish()));
    }

    fn handle_ui_event(&mut self, event: UiEvent) {
        match event {
            UiEvent::ButtonClicked { widget_id } => {
                match widget_id.as_str() {
                    "tool_select" => self.editor_state.active_tool = ActiveTool::Select,
                    "tool_translate" => self.editor_state.active_tool = ActiveTool::Translate,
                    "tool_rotate" => self.editor_state.active_tool = ActiveTool::Rotate,
                    "tool_scale" => self.editor_state.active_tool = ActiveTool::Scale,
                    "tool_vertex" => self.editor_state.active_tool = ActiveTool::VertexEdit,
                    "toggle_wind" => self.editor_state.wind_enabled = !self.editor_state.wind_enabled,
                    "toggle_waves" => self.editor_state.wave_enabled = !self.editor_state.wave_enabled,
                    "toggle_physics" => {
                        self.physics.is_active = !self.physics.is_active;
                        self.editor_state.physics_active = self.physics.is_active;
                    }
                    "reset_player_pos" => {
                        self.physics.player.position = Vec3::new(0.0, 2.0, 0.0);
                        self.physics.player.velocity = Vec3::ZERO;
                    }
                    "btn_trigger_explosion" => {
                        let pos = if let Some(scene) = &self.scene {
                            if scene.selected_node_idx < scene.nodes.len() {
                                scene.nodes[scene.selected_node_idx].transform.translation
                            } else {
                                Vec3::new(0.0, 0.5, 0.0)
                            }
                        } else {
                            Vec3::new(0.0, 0.5, 0.0)
                        };
                        self.trigger_explosion(pos, 45.0);
                    }
                    "btn_spawn_tower" => {
                        if let (Some(scene), Some(gpu)) = (&mut self.scene, &self.gpu_context) {
                            scene.spawn_cube_tower(&gpu.device, Vec3::new(0.0, 0.0, 0.0), 3, 3);
                            self.physics.sync_from_scene_nodes(&scene.nodes);
                            self.editor_state.set_toast("Tour Destructible", "Pile de cubes générée.", ToastKind::Success);
                        }
                    }
                    "btn_spawn_humanoid" => {
                        if let (Some(scene), Some(gpu)) = (&mut self.scene, &self.gpu_context) {
                            let pos = Vec3::new(0.0, 0.0, 0.0);
                            let id = scene.spawn_rigged_humanoid(&gpu.device, pos);
                            self.editor_state.set_toast(
                                "Mannequin Riggé Invoqué",
                                format!("Modèle articulé #{} ajouté avec squelette et animations !", id),
                                ToastKind::Success,
                            );
                        }
                    }
                    "btn_toggle_anim_play" => {
                        self.editor_state.anim_playing = !self.editor_state.anim_playing;
                    }
                    "btn_parent_selected" => {
                        if let Some(scene) = &mut self.scene {
                            if scene.nodes.len() > 1 && scene.selected_node_idx != 0 {
                                let child_idx = scene.selected_node_idx;
                                scene.parent_node(child_idx, 0);
                                self.editor_state.set_toast(
                                    "Hiérarchie",
                                    format!("Nœud #{} est maintenant enfant du Nœud #0", child_idx),
                                    ToastKind::Success,
                                );
                            }
                        }
                    }
                    "btn_unparent_selected" => {
                        if let Some(scene) = &mut self.scene {
                            let child_idx = scene.selected_node_idx;
                            scene.unparent_node(child_idx);
                            self.editor_state.set_toast("Hiérarchie", "Nœud détaché du parent.", ToastKind::Info);
                        }
                    }
                    "btn_add_mod_twist" => {
                        if let Some(scene) = &mut self.scene {
                            if scene.selected_node_idx < scene.nodes.len() {
                                let node = &mut scene.nodes[scene.selected_node_idx];
                                node.modifiers.push(crate::animation::MeshModifier::Twist {
                                    enabled: true,
                                    angle: std::f32::consts::PI * 0.75,
                                });
                                self.editor_state.set_toast("Modificateur Ajouté", "Torsion (Twist) ajoutée à la pile.", ToastKind::Success);
                            }
                        }
                    }
                    "btn_add_mod_bend" => {
                        if let Some(scene) = &mut self.scene {
                            if scene.selected_node_idx < scene.nodes.len() {
                                let node = &mut scene.nodes[scene.selected_node_idx];
                                node.modifiers.push(crate::animation::MeshModifier::Bend {
                                    enabled: true,
                                    angle: std::f32::consts::PI * 0.5,
                                });
                                self.editor_state.set_toast("Modificateur Ajouté", "Courbure (Bend) ajoutée à la pile.", ToastKind::Success);
                            }
                        }
                    }
                    "btn_add_mod_taper" => {
                        if let Some(scene) = &mut self.scene {
                            if scene.selected_node_idx < scene.nodes.len() {
                                let node = &mut scene.nodes[scene.selected_node_idx];
                                node.modifiers.push(crate::animation::MeshModifier::Taper {
                                    enabled: true,
                                    factor: -0.6,
                                });
                                self.editor_state.set_toast("Modificateur Ajouté", "Évasement (Taper) ajouté à la pile.", ToastKind::Success);
                            }
                        }
                    }
                    "btn_add_mod_noise" => {
                        if let Some(scene) = &mut self.scene {
                            if scene.selected_node_idx < scene.nodes.len() {
                                let node = &mut scene.nodes[scene.selected_node_idx];
                                node.modifiers.push(crate::animation::MeshModifier::Noise {
                                    enabled: true,
                                    strength: 0.25,
                                    frequency: 3.0,
                                });
                                self.editor_state.set_toast("Modificateur Ajouté", "Relief (Noise) ajouté à la pile.", ToastKind::Success);
                            }
                        }
                    }
                    "btn_script_spawn_cubes" => {
                        if let (Some(scene), Some(gpu)) = (&mut self.scene, &self.gpu_context) {
                            let script = r#"
                                spawn cube -4.0 1.0 0.0
                                spawn sphere -2.0 1.0 0.0
                                spawn cylinder 0.0 1.0 0.0
                                spawn sphere 2.0 1.0 0.0
                                spawn cube 4.0 1.0 0.0
                                sound jump 0.0 1.0 0.0
                            "#;
                            let mut script_ctx = scripting::ScriptContext {
                                scene,
                                physics: &mut self.physics,
                                particles: self.particles.as_mut(),
                                audio: &mut self.audio,
                                device: &gpu.device,
                                queue: &gpu.queue,
                                delta_time: 0.016,
                                current_time: self.total_time,
                            };
                            let _ = self.scripting.execute_dsl(script, &mut script_ctx);
                            self.editor_state.set_toast("DSL Exécuté", "Ligne de 5 objets générée avec son 3D !", ToastKind::Success);
                        }
                    }
                    "btn_script_orbit_torus" => {
                        if let (Some(scene), Some(gpu)) = (&mut self.scene, &self.gpu_context) {
                            let script = r#"
                                spawn cube 0.0 3.0 0.0
                                scale 0 1.5 1.5 1.5
                                rotate_auto 0 y 2.5
                                sound laser 0.0 3.0 0.0
                            "#;
                            let mut script_ctx = scripting::ScriptContext {
                                scene,
                                physics: &mut self.physics,
                                particles: self.particles.as_mut(),
                                audio: &mut self.audio,
                                device: &gpu.device,
                                queue: &gpu.queue,
                                delta_time: 0.016,
                                current_time: self.total_time,
                            };
                            let _ = self.scripting.execute_dsl(script, &mut script_ctx);
                            self.editor_state.set_toast("DSL Exécuté", "Auto-rotation et mise à l'échelle enregistrées !", ToastKind::Success);
                        }
                    }
                    "btn_script_rhai_firework" => {
                        if let (Some(scene), Some(gpu)) = (&mut self.scene, &self.gpu_context) {
                            let script = r#"
                                spawn_primitive("FeuArtifice_Central", "sphere", 0.0, 4.0, 0.0);
                                trigger_explosion(0.0, 4.0, 0.0, 10.0, 35.0);
                                spawn_particles("sparks", 0.0, 4.0, 0.0, 50);
                                play_sound("explosion", 0.0, 4.0, 0.0);
                                log("Feu d'artifice déclenché via Rhai !");
                            "#;
                            let mut script_ctx = scripting::ScriptContext {
                                scene,
                                physics: &mut self.physics,
                                particles: self.particles.as_mut(),
                                audio: &mut self.audio,
                                device: &gpu.device,
                                queue: &gpu.queue,
                                delta_time: 0.016,
                                current_time: self.total_time,
                            };
                            let _ = self.scripting.execute_rhai(script, &mut script_ctx);
                            self.editor_state.set_toast("Rhai Exécuté", "Scénario Feu d'Artifice 3D déclenché !", ToastKind::Success);
                        }
                    }
                    "btn_script_reset" => {
                        self.scripting = scripting::ScriptingEngine::new();
                        self.editor_state.set_toast("Scripts Reset", "Tous les scripts et rotations actives réinitialisés.", ToastKind::Info);
                    }
                    "btn_clear_mods" => {
                        if let (Some(scene), Some(gpu)) = (&mut self.scene, &self.gpu_context) {
                            if scene.selected_node_idx < scene.nodes.len() {
                                let node = &mut scene.nodes[scene.selected_node_idx];
                                node.modifiers.clear();
                                node.vertices = node.base_vertices.clone();
                                node.update_vertices_gpu(&gpu.queue);
                                self.editor_state.set_toast("Modificateurs", "Pile réinitialisée à la géométrie de base.", ToastKind::Info);
                            }
                        }
                    }
                    "btn_apply_mods" => {
                        if let Some(scene) = &mut self.scene {
                            if scene.selected_node_idx < scene.nodes.len() {
                                let node = &mut scene.nodes[scene.selected_node_idx];
                                node.apply_modifiers_permanently();
                                self.editor_state.set_toast("Modificateurs Figés", "Déformation enregistrée dans la géométrie de base.", ToastKind::Success);
                            }
                        }
                    }
                    "add_cube" | "prim_cube" => {


                        if let (Some(scene), Some(gpu)) = (&mut self.scene, &self.gpu_context) {
                            scene.add_node(&gpu.device, PrimitiveType::Cube);
                            self.base_vertices.clear();
                            if let Some(pt) = &mut self.pathtracer {
                                pt.update_scene(&gpu.device, &scene.nodes);
                                pt.reset_accumulation();
                            }
                            self.editor_state.set_toast("Objet Ajouté", "Cube primitif ajouté à la scène.", ToastKind::Success);
                        }
                    }
                    "add_pyramid" | "prim_pyramid" => {
                        if let (Some(scene), Some(gpu)) = (&mut self.scene, &self.gpu_context) {
                            scene.add_node(&gpu.device, PrimitiveType::Pyramid);
                            self.base_vertices.clear();
                            if let Some(pt) = &mut self.pathtracer {
                                pt.update_scene(&gpu.device, &scene.nodes);
                                pt.reset_accumulation();
                            }
                            self.editor_state.set_toast("Objet Ajouté", "Pyramide ajoutée à la scène.", ToastKind::Success);
                        }
                    }
                    "add_prism" | "prim_prism" => {
                        if let (Some(scene), Some(gpu)) = (&mut self.scene, &self.gpu_context) {
                            scene.add_node(&gpu.device, PrimitiveType::Prism);
                            self.base_vertices.clear();
                            if let Some(pt) = &mut self.pathtracer {
                                pt.update_scene(&gpu.device, &scene.nodes);
                                pt.reset_accumulation();
                            }
                            self.editor_state.set_toast("Objet Ajouté", "Prisme ajouté à la scène.", ToastKind::Success);
                        }
                    }
                    "duplicate_node" => {
                        if let (Some(scene), Some(gpu)) = (&mut self.scene, &self.gpu_context) {
                            scene.duplicate_selected_node(&gpu.device);
                            self.base_vertices.clear();
                            if let Some(pt) = &mut self.pathtracer {
                                pt.update_scene(&gpu.device, &scene.nodes);
                                pt.reset_accumulation();
                            }
                            self.editor_state.set_toast("Duplication", "Objet sélectionné dupliqué.", ToastKind::Info);
                        }
                    }
                    "delete_node" => {
                        if let (Some(scene), Some(gpu)) = (&mut self.scene, &self.gpu_context) {
                            scene.remove_selected_node();
                            self.base_vertices.clear();
                            if let Some(pt) = &mut self.pathtracer {
                                pt.update_scene(&gpu.device, &scene.nodes);
                                pt.reset_accumulation();
                            }
                            self.editor_state.set_toast("Suppression", "Objet retiré de la scène.", ToastKind::Warning);
                        }
                    }
                    "close_shortcuts_modal" => {
                        self.editor_state.show_shortcuts_modal = false;
                    }
                    "studio_play" => self.studio_play(),
                    "studio_pause" => self.studio_pause(),
                    "studio_stop" => self.studio_stop(),
                    "studio_apply_material" => self.studio_apply_material(),
                    "studio_reset_material" => self.studio_reset_material(),
                    "studio_toggle_theme" => self.toggle_theme(),
                    "toggle_studio_mode" => self.toggle_studio_mode(),
                    _ => {
                        if let Some(idx) = widget_id
                            .strip_prefix("gn_add_")
                            .and_then(|s| s.parse::<usize>().ok())
                        {
                            if let Some((_, kind)) = studio::ui::NODE_LIBRARY.get(idx) {
                                self.studio_add_node(*kind);
                            }
                        }
                    }
                }
            }
            UiEvent::MenuItemClicked { menu_id: _, item_id } => {
                self.editor_state.active_menu = None; // Fermer le menu après clic
                self.studio_shell.active_menu = None;
                match item_id.as_str() {
                    "file_new" => {
                        if let (Some(scene), Some(gpu)) = (&mut self.scene, &self.gpu_context) {
                            scene.clear();
                            scene.add_node(&gpu.device, PrimitiveType::Cube);
                            self.current_world_path = None;
                            self.base_vertices.clear();
                            if let Some(pt) = &mut self.pathtracer {
                                pt.update_scene(&gpu.device, &scene.nodes);
                                pt.reset_accumulation();
                            }
                            self.editor_state.set_toast("Nouvelle Scène", "Scène 3D réinitialisée.", ToastKind::Success);
                        }
                    }
                    "file_open" => self.open_file_dialog(),
                    "file_save" => self.save_world_quick(),
                    "file_save_as" => self.save_world_as_dialog(),
                    "file_quit" => {
                        std::process::exit(0);
                    }
                    "add_cube" | "prim_cube" => {
                        if let (Some(scene), Some(gpu)) = (&mut self.scene, &self.gpu_context) {
                            scene.add_node(&gpu.device, PrimitiveType::Cube);
                            self.base_vertices.clear();
                            if let Some(pt) = &mut self.pathtracer {
                                pt.update_scene(&gpu.device, &scene.nodes);
                                pt.reset_accumulation();
                            }
                            self.editor_state.set_toast("Ajout Primitive", "Cube ajouté avec succès.", ToastKind::Success);
                        }
                    }
                    "add_pyramid" | "prim_pyramid" => {
                        if let (Some(scene), Some(gpu)) = (&mut self.scene, &self.gpu_context) {
                            scene.add_node(&gpu.device, PrimitiveType::Pyramid);
                            self.base_vertices.clear();
                            if let Some(pt) = &mut self.pathtracer {
                                pt.update_scene(&gpu.device, &scene.nodes);
                                pt.reset_accumulation();
                            }
                            self.editor_state.set_toast("Ajout Primitive", "Pyramide ajoutée.", ToastKind::Success);
                        }
                    }
                    "add_prism" | "prim_prism" => {
                        if let (Some(scene), Some(gpu)) = (&mut self.scene, &self.gpu_context) {
                            scene.add_node(&gpu.device, PrimitiveType::Prism);
                            self.base_vertices.clear();
                            if let Some(pt) = &mut self.pathtracer {
                                pt.update_scene(&gpu.device, &scene.nodes);
                                pt.reset_accumulation();
                            }
                            self.editor_state.set_toast("Ajout Primitive", "Prisme ajouté.", ToastKind::Success);
                        }
                    }
                    "render_forward" => {
                        self.render_mode = RenderMode::VideoGame;
                        self.editor_state.set_toast("Mode Rendu", "Forward PBR Activé.", ToastKind::Info);
                    }
                    "render_pathtrace" => {
                        self.render_mode = RenderMode::PathTrace;
                        if let (Some(scene), Some(gpu), Some(pt)) = (&self.scene, &self.gpu_context, &mut self.pathtracer) {
                            pt.update_scene(&gpu.device, &scene.nodes);
                            pt.reset_accumulation();
                        }
                        self.editor_state.set_toast("Mode Rendu", "PathTracer GPU Activé.", ToastKind::Info);
                    }
                    "render_raster" => {
                        self.render_mode = RenderMode::Rasterize;
                        self.editor_state.set_toast("Mode Rendu", "Rasterisation Légère.", ToastKind::Info);
                    }
                    "game_toggle" => {
                        self.physics.is_active = !self.physics.is_active;
                        self.editor_state.physics_active = self.physics.is_active;
                        self.editor_state.set_toast("Mode Jeu", if self.physics.is_active { "Contrôles FPS activés" } else { "Mode Éditeur 3D" }, ToastKind::Info);
                    }
                    "game_cam_1st" => self.camera.mode = CameraMode::FirstPerson,
                    "game_cam_3rd" => self.camera.mode = CameraMode::ThirdPerson,
                    "game_cam_orb" => self.camera.mode = CameraMode::Orbital,
                    "eco_studio" => {
                        if let (Some(scene), Some(gpu)) = (&mut self.scene, &self.gpu_context) {
                            scene.clear();
                            scene.load_studio_showcase(&gpu.device);
                            self.base_vertices.clear();
                            if let Some(pt) = &mut self.pathtracer {
                                pt.update_scene(&gpu.device, &scene.nodes);
                                pt.reset_accumulation();
                            }
                            self.editor_state.set_toast("Studio PBR", "Scène Studio Showcase chargée !", ToastKind::Success);
                        }
                    }
                    "eco_forest" => {
                        if let (Some(scene), Some(gpu)) = (&mut self.scene, &self.gpu_context) {
                            scene.clear();
                            scene.load_living_forest(&gpu.device);
                            self.base_vertices.clear();
                            if let Some(pt) = &mut self.pathtracer {
                                pt.update_scene(&gpu.device, &scene.nodes);
                                pt.reset_accumulation();
                            }
                            self.editor_state.set_toast("Forêt Vivante", "Écosystème Forêt Vivante généré !", ToastKind::Success);
                        }
                    }
                    "eco_cove" => {
                        if let (Some(scene), Some(gpu)) = (&mut self.scene, &self.gpu_context) {
                            scene.clear();
                            scene.load_night_cove(&gpu.device);
                            self.base_vertices.clear();
                            if let Some(pt) = &mut self.pathtracer {
                                pt.update_scene(&gpu.device, &scene.nodes);
                                pt.reset_accumulation();
                            }
                            self.editor_state.set_toast("Crique Nocturne", "Écosystème Crique Nocturne généré !", ToastKind::Success);
                        }
                    }
                    "eco_bladerunner" => {
                        if let (Some(scene), Some(gpu)) = (&mut self.scene, &self.gpu_context) {
                            scene.clear();
                            game::generate_cyber_megalopolis(scene, &gpu.device);
                            self.base_vertices.clear();

                            // Spawner le véhicule Cyber Spinner au centre de la mégapole
                            let (v_spinner, i_spinner) = game::generate_cyber_spinner_mesh();
                            let s_idx = scene.nodes.len();
                            scene.add_custom_mesh(&gpu.device, "Cyber_Spinner_Vehicle", v_spinner, i_spinner);
                            if let Some(node) = scene.nodes.get_mut(s_idx) {
                                node.material = scene::MaterialUniform {
                                    roughness: 0.2,
                                    metallic: 0.85,
                                    uv_tiling: [1.0, 1.0],
                                    uv_offset: [0.0, 0.0],
                                    ior: 1.5,
                                    transmission: 0.0,
                                    emission_color: [0.1, 2.5, 3.8, 1.0], // Réacteurs et phares néon cyan
                                    use_normal_map: 0,
                                    clearcoat: 1.0,
                                    clearcoat_roughness: 0.03,
                                    subsurface: 0.1,
                                };
                            }

                            self.spinner = game::CyberSpinner::new(Vec3::new(0.0, 16.0, -10.0));
                            self.cyber_city = game::CyberCitySystem::default();
                            self.spinner_node_idx = Some(s_idx);

                            // Activer le mode jeu Blade Runner
                            self.editor_state.blade_runner_mode = true;
                            self.physics.is_active = false;
                            self.render_mode = RenderMode::VideoGame;
                            self.camera.mode = CameraMode::ThirdPerson;

                            self.editor_state.set_toast(
                                "🚀 Blade Runner 2099",
                                "Spinner prêt ! ZQSD: Vol | Espace/Ctrl: Altitude | Shift: Boost",
                                ToastKind::Success,
                            );
                        }
                    }
                    "eco_stress" => {
                        if let (Some(scene), Some(gpu)) = (&mut self.scene, &self.gpu_context) {
                            scene.clear();
                            scene.load_stress_test_grid(&gpu.device, 5);
                            self.base_vertices.clear();
                            if let Some(pt) = &mut self.pathtracer {
                                pt.update_scene(&gpu.device, &scene.nodes);
                                pt.reset_accumulation();
                            }
                            self.editor_state.set_toast("Stress Test", "Grille de 25 objets instanciés.", ToastKind::Info);
                        }
                    }
                    "help_shortcuts" => {
                        self.editor_state.show_shortcuts_modal = true;
                    }
                    "help_about" => {
                        self.editor_state.set_toast("À propos", "AORUI 3D Engine — Moteur Cyber-Glass GPU v0.1.0", ToastKind::Info);
                    }
                    "asset_rescan" => {
                        if let Some(session) = self.studio.as_mut() {
                            match session.rescan() {
                                Ok(n) => {
                                    let _ = n;
                                    self.studio_shell.console.push(
                                        format!("Scan terminé: {} assets indexés.", session.asset_count()),
                                        LogLevel::Success,
                                    );
                                }
                                Err(e) => self.studio_shell.console.push(e, LogLevel::Error),
                            }
                        }
                    }
                    "build_standalone" => {
                        let out = std::env::current_dir().unwrap_or_default().join("AORProject/build/Game.aorpak");
                        let result = self.studio.as_ref().map(|s| s.package_start_scene(&out));
                        match result {
                            Some(Ok(stats)) => self.studio_shell.console.push(
                                format!("Build .aorpak: {} fichiers, {} octets → {:?}", stats.file_count, stats.total_bytes, stats.output_path),
                                LogLevel::Success,
                            ),
                               Some(Err(e)) => self.studio_shell.console.push(format!("Build échoué: {e}"), LogLevel::Error),
                               None => self.studio_shell.console.push("Aucune session studio active.", LogLevel::Warning),
                           }
                       }
                       "studio_apply_material" => self.studio_apply_material(),
                       "studio_reset_material" => self.studio_reset_material(),
                       "studio_delete_graph_node" => self.studio_delete_node(),
                       "st_export_aor" => self.export_aor_dialog(),
                       "st_import_aor" => self.import_aor_dialog(),
                       "st_new_scene" => {
                           if let (Some(scene), Some(gpu)) = (&mut self.scene, &self.gpu_context) {
                               scene.clear();
                               scene.add_node(&gpu.device, PrimitiveType::Cube);
                               self.base_vertices.clear();
                               if let Some(pt) = &mut self.pathtracer {
                                   pt.update_scene(&gpu.device, &scene.nodes);
                                   pt.reset_accumulation();
                               }
                               self.studio_shell.console.push("Nouvelle scène créée.".to_string(), LogLevel::Success);
                           }
                       }
                       _ => {
                           if let Some(idx) = item_id
                               .strip_prefix("gn_add_")
                               .and_then(|s| s.parse::<usize>().ok())
                           {
                               if let Some((_, kind)) = studio::ui::NODE_LIBRARY.get(idx) {
                                   self.studio_add_node(*kind);
                               }
                           }
                       }
                   }
               }
            UiEvent::SegmentSelected { widget_id, selected_index } => {
                if widget_id.as_str() == "inspector_tabs" {
                    self.editor_state.inspector_tab = match selected_index {
                        0 => InspectorTab::Transform,
                        1 => InspectorTab::Material,
                        2 => InspectorTab::Physics,
                        3 => InspectorTab::Animation,
                        4 => InspectorTab::Modifiers,
                        5 => InspectorTab::Hierarchy,
                        6 => InspectorTab::Scripting,
                        _ => InspectorTab::Transform,
                    };
                } else if widget_id.as_str() == "anim_clip_selector" {

                    self.editor_state.anim_clip_idx = selected_index;
                }
            }
            UiEvent::MenuToggled { menu_id, open } => {
                if open {
                    let idx = menu_id.split(':').nth(1).and_then(|s| s.parse::<usize>().ok());
                    self.editor_state.active_menu = idx;
                    self.studio_shell.active_menu = idx;
                } else {
                    self.editor_state.active_menu = None;
                    self.studio_shell.active_menu = None;
                }
            }
            UiEvent::SliderChanged { widget_id, value } => {
                match widget_id.as_str() {
                    "anim_speed_slider" => self.editor_state.anim_speed = value,
                    "squash_intensity_slider" => self.editor_state.squash_intensity = value,
                    "wind_strength_slider" => self.editor_state.wind_strength = value,
                    "wave_amp_slider" => self.editor_state.wave_amplitude = value,
                    "roughness_slider" => {

                        self.editor_state.roughness = value;
                        if let (Some(scene), Some(gpu)) = (&mut self.scene, &self.gpu_context) {
                            if scene.selected_node_idx < scene.nodes.len() {
                                scene.nodes[scene.selected_node_idx].material.roughness = value;
                                scene.nodes[scene.selected_node_idx].update_gpu(&gpu.queue, true);
                            }
                        }
                    }
                    "metallic_slider" => {
                        self.editor_state.metallic = value;
                        if let (Some(scene), Some(gpu)) = (&mut self.scene, &self.gpu_context) {
                            if scene.selected_node_idx < scene.nodes.len() {
                                scene.nodes[scene.selected_node_idx].material.metallic = value;
                                scene.nodes[scene.selected_node_idx].update_gpu(&gpu.queue, true);
                            }
                        }
                    }
                    "gravity_slider" => {
                        self.editor_state.gravity = value;
                        self.physics.gravity = Vec3::new(0.0, -value, 0.0);
                    }
                    "insp_roughness" => {
                        if let (Some(scene), Some(gpu)) = (&mut self.scene, &self.gpu_context) {
                            if scene.selected_node_idx < scene.nodes.len() {
                                scene.nodes[scene.selected_node_idx].material.roughness = value;
                                scene.nodes[scene.selected_node_idx].update_gpu(&gpu.queue, true);
                            }
                        }
                    }
                    "insp_metallic" => {
                        if let (Some(scene), Some(gpu)) = (&mut self.scene, &self.gpu_context) {
                            if scene.selected_node_idx < scene.nodes.len() {
                                scene.nodes[scene.selected_node_idx].material.metallic = value;
                                scene.nodes[scene.selected_node_idx].update_gpu(&gpu.queue, true);
                            }
                        }
                    }
                    _ => {
                        if let Some(key) = widget_id.strip_prefix("gnp_") {
                            self.studio_set_graph_param(key, value);
                        }
                    }
                }
            }
            UiEvent::ColorChanged { widget_id, color } => {
                if widget_id.as_str() == "insp_color" {
                    self.studio_shell.inspector.current_color = color;
                }
                if widget_id.as_str() == "gnp_color" {
                    self.studio_set_graph_color(color);
                }
                if widget_id.as_str() == "material_color_picker" || widget_id.as_str() == "insp_color" {
                    self.editor_state.current_color = color;
                    if let (Some(scene), Some(gpu)) = (&mut self.scene, &self.gpu_context) {
                        if scene.selected_node_idx < scene.nodes.len() {
                            for v in &mut scene.nodes[scene.selected_node_idx].vertices {
                                v.color = color;
                            }
                            scene.nodes[scene.selected_node_idx].update_gpu(&gpu.queue, true);
                        }
                    }
                }
            }
            UiEvent::PaletteFoldToggled { palette_id, folded } => {
                match palette_id.as_str() {
                    "tools_palette" => self.editor_state.tools_folded = folded,
                    "deformers_palette" => self.editor_state.deformers_folded = folded,
                    "inspector_palette" => self.editor_state.inspector_folded = folded,
                    _ => {}
                }
            }
            UiEvent::PaletteClosed { palette_id } => {
                match palette_id.as_str() {
                    "tools_palette" => self.editor_state.show_tools_palette = false,
                    "deformers_palette" => self.editor_state.show_deformers_palette = false,
                    "inspector_palette" => self.editor_state.show_inspector_palette = false,
                    _ => {}
                }
            }
            UiEvent::TreeNodeToggled { tree_id, node_id, expanded } => {
                match tree_id.as_str() {
                    "scene_hierarchy" => {
                        if let Ok(idx) = node_id.parse::<usize>() {
                            if expanded {
                                self.studio_shell.hierarchy.expanded.insert(idx);
                            } else {
                                self.studio_shell.hierarchy.expanded.remove(&idx);
                            }
                        }
                    }
                    "asset_tree" => {
                        if expanded {
                            self.studio_shell.asset_browser.expanded.insert(node_id);
                        } else {
                            self.studio_shell.asset_browser.expanded.remove(&node_id);
                        }
                    }
                    _ => {
                        if expanded {
                            self.editor_state.expanded_nodes.insert(node_id);
                        } else {
                            self.editor_state.expanded_nodes.remove(&node_id);
                        }
                    }
                }
            }
            UiEvent::TreeNodeSelected { tree_id, node_id } => {
                match tree_id.as_str() {
                    "scene_hierarchy" => {
                        if let Ok(idx) = node_id.parse::<usize>() {
                            if let Some(scene) = &mut self.scene {
                                if idx < scene.nodes.len() {
                                    scene.selected_node_idx = idx;
                                }
                            }
                        }
                    }
                    "asset_tree" => {
                        if let Some(rest) = node_id.strip_prefix("file:") {
                            if let Ok(u) = uuid::Uuid::parse_str(rest) {
                                self.studio_shell.asset_browser.selected = Some(studio::AssetId(u));
                            }
                        }
                    }
                    _ => {
                        self.editor_state.selected_tree_id = Some(node_id.clone());
                        if let Some(idx_str) = node_id.strip_prefix("node_") {
                            if let Ok(idx) = idx_str.parse::<usize>() {
                                if let Some(scene) = &mut self.scene {
                                    scene.selected_node_idx = idx;
                                }
                            }
                        }
                    }
                }
            }
            UiEvent::TabSelected { widget_id, tab_index } => {
                if widget_id.as_str() == "studio_center_tabs" {
                    self.studio_shell.active_tab = match tab_index {
                        0 => StudioCenterTab::Scene3D,
                        1 => StudioCenterTab::Split3DShader,
                        2 => StudioCenterTab::ShaderEditor,
                        3 => StudioCenterTab::PrefabView,
                        _ => StudioCenterTab::RhaiEditor,
                    };
                    if self.studio_shell.active_tab == StudioCenterTab::ShaderEditor || self.studio_shell.active_tab == StudioCenterTab::Split3DShader {
                        self.studio_shell.inspector.section = InspectorSection::NodeGraph;
                    }
                } else if widget_id.as_str() == "inspector_tabs" {
                    self.studio_shell.inspector.section = match tab_index {
                        0 => InspectorSection::Transform,
                        1 => InspectorSection::Material,
                        2 => InspectorSection::Physics,
                        3 => InspectorSection::Script,
                        _ => InspectorSection::NodeGraph,
                    };
                }
            }
            UiEvent::CustomPaintPointerDown { widget_id, local_pos, .. } => {
                if widget_id.as_str() == "studio_node_canvas" && self.studio_shell.active_tab == StudioCenterTab::ShaderEditor {
                    if let Some(session) = self.studio.as_mut() {
                        let transform = self.studio_shell.canvas;
                        // 1. Tester les pins
                        if let Some(hit) = studio::ui::node_canvas::hit_pin(
                            &session.graph,
                            &transform,
                            local_pos,
                            12.0 * transform.zoom,
                        ) {
                            if hit.is_output {
                                self.studio_shell.pending_link = Some((hit.node, hit.pin));
                                self.studio_shell.console.push(
                                    "Liaison: sortie choisie — cliquez maintenant une entrée.".to_string(),
                                    LogLevel::Info,
                                );
                            } else if let Some((from_node, from_pin)) = self.studio_shell.pending_link.take() {
                                self.connect_graph_pins(from_node, from_pin, hit.node, hit.pin);
                            }
                            return;
                        }
                        // 2. Tester les nœuds
                        if let Some(node_id) = studio::ui::hit_node(&session.graph, &transform, local_pos) {
                            self.studio_shell.selected_graph_node = Some(node_id);
                            let gpos = transform.screen_to_graph(local_pos);
                            let offset = session
                                .graph
                                .node(node_id)
                                .map(|n| [gpos[0] - n.position[0], gpos[1] - n.position[1]])
                                .unwrap_or([0.0, 0.0]);
                            self.studio_shell.dragging_node = Some((node_id, offset));
                            let label = session
                                .graph
                                .node(node_id)
                                .map(|n| n.kind.label())
                                .unwrap_or("?");
                            self.studio_shell.console.push(
                                format!("Nœud sélectionné: {}", label),
                                LogLevel::Info,
                            );
                            return;
                        }
                        // 3. Clic dans le vide : pan
                        self.studio_canvas_dragging = true;
                    }
                }
            }
            UiEvent::NumberChanged { widget_id, value } => {
                self.handle_studio_number(&widget_id, value as f32);
            }
            _ => {}
        }
    }

    // =====================================================================
    // Game Studio : transport PIE, interactions et bascule d'atelier
    // =====================================================================

    fn toggle_studio_mode(&mut self) {
        self.studio_mode = !self.studio_mode;
        let msg = if self.studio_mode {
            "Studio AOR activé (F10 pour revenir)."
        } else {
            "Atelier 3D classique activé."
        };
        self.editor_state.set_toast("Bascule", msg, ToastKind::Info);
        self.studio_shell.console.push(msg.to_string(), LogLevel::Info);
    }

    /// Bascule entre thème sombre et cyber-glass
    fn toggle_theme(&mut self) {
        self.dark_mode = !self.dark_mode;
        self.theme = if self.dark_mode {
            dark_theme()
        } else {
            Theme::cyber_glass()
        };
        self.studio_shell.console.push(
            format!("Thème: {}", if self.dark_mode { "sombre" } else { "cyber-glass" }),
            LogLevel::Info,
        );
    }

    /// Bouton Play : capture l'état courant de la scène et démarre le PIE
    fn studio_play(&mut self) {
        let light = self
            .forward_renderer
            .as_ref()
            .map(|r| r.light_pos)
            .unwrap_or(Vec3::new(3.0, 5.0, 4.0));
        let result = match (self.scene.as_ref(), self.studio.as_mut()) {
            (Some(scene), Some(session)) => Some(session.pie.play(scene, light)),
            _ => None,
        };
        match result {
            Some(Ok(())) => {
                self.studio_shell
                    .console
                    .push("▶ Play : simulation démarrée (snapshot ECS capturé).".to_string(), LogLevel::Success);
                self.editor_state
                    .set_toast("Play", "Simulation démarrée.", ToastKind::Success);
            }
            Some(Err(e)) => self
                .studio_shell
                .console
                .push(format!("Play impossible: {e}"), LogLevel::Error),
            None => self
                .studio_shell
                .console
                .push("Aucune scène à simuler.".to_string(), LogLevel::Warning),
        }
    }

    fn studio_pause(&mut self) {
        if let Some(session) = self.studio.as_mut() {
            session.pie.toggle_pause();
            let state = format!("{:?}", session.pie.state);
            self.studio_shell
                .console
                .push(format!("⏸ Transport : {}", state), LogLevel::Info);
        }
    }

    /// Bouton Stop : arrête le PIE et restaure l'état ECS d'origine
    fn studio_stop(&mut self) {
        let world = self.studio.as_mut().and_then(|s| s.pie.stop());
        let Some(world) = world else {
            self.studio_shell
                .console
                .push("Stop : aucune simulation active.".to_string(), LogLevel::Warning);
            return;
        };
        let json = match serde_json::to_string(&world) {
            Ok(j) => j,
            Err(e) => {
                self.studio_shell.console.push(format!("Stop: sérialisation échouée: {e}"), LogLevel::Error);
                return;
            }
        };
        if let (Some(scene), Some(gpu)) = (&mut self.scene, &self.gpu_context) {
            match crate::scene::persistence::deserialize_world(
                &gpu.device,
                &scene.model_bind_group_layout,
                scene.white_texture.clone(),
                scene.flat_normal_texture.clone(),
                &json,
            ) {
                Ok((nodes, _light)) => {
                    let max_id = nodes.iter().map(|n| n.id).max().unwrap_or(0);
                    scene.nodes = nodes;
                    scene.next_node_id = max_id + 1;
                    scene.selected_node_idx = 0;
                    self.base_vertices.clear();
                    if let Some(pt) = &mut self.pathtracer {
                        pt.update_scene(&gpu.device, &scene.nodes);
                        pt.reset_accumulation();
                    }
                    self.studio_shell
                        .console
                        .push("⏹ Stop : état ECS restauré.".to_string(), LogLevel::Success);
                    self.editor_state
                        .set_toast("Stop", "État d'origine restauré.", ToastKind::Info);
                }
                Err(e) => self
                    .studio_shell
                    .console
                    .push(format!("Stop: restauration échouée: {e}"), LogLevel::Error),
            }
        }
    }

    /// Compile le graphe nodal en shader moteur et l'injecte comme pipeline WGPU actif
    fn studio_apply_material(&mut self) {
        let compiled = match self.studio.as_ref() {
            Some(session) => match studio::compile_engine_shader(&session.graph) {
                Ok(c) => c,
                Err(e) => {
                    self.studio_shell
                        .console
                        .push(format!("Compilation graphe échouée: {e}"), LogLevel::Error);
                    return;
                }
            },
            None => {
                self.studio_shell
                    .console
                    .push("Aucune session studio active.".to_string(), LogLevel::Warning);
                return;
            }
        };

        if let Err(e) = studio::validate_wgsl(&compiled.wgsl) {
            self.studio_shell
                .console
                .push(format!("WGSL invalide: {e}"), LogLevel::Error);
            return;
        }

        let result = match (&self.gpu_context, &self.scene, &mut self.forward_renderer) {
            (Some(gpu), Some(scene), Some(renderer)) => renderer.build_material_pipeline(
                &gpu.device,
                &compiled.wgsl,
                &scene.model_bind_group_layout,
                wgpu::TextureFormat::Rgba8UnormSrgb,
            ),
            _ => Err("Contexte GPU ou scène indisponible".to_string()),
        };

        match result {
            Ok(()) => {
                self.editor_state.set_toast(
                    "Matériau Compilé",
                    format!(
                        "Pipeline WGPU nodal appliqué ({} nœuds, {} textures).",
                        compiled.node_count, compiled.texture_bindings
                    ),
                    ToastKind::Success,
                );
                self.studio_shell.console.push(
                    format!(
                        "Pipeline WGPU compilé à chaud: {} nœuds, {} bindings texture.",
                        compiled.node_count, compiled.texture_bindings
                    ),
                    LogLevel::Success,
                );
            }
            Err(e) => self
                .studio_shell
                .console
                .push(format!("Création pipeline échouée: {e}"), LogLevel::Error),
        }
    }

    /// Retire le pipeline nodal et revient au shader PBR standard
    fn studio_reset_material(&mut self) {
        if let Some(renderer) = &mut self.forward_renderer {
            renderer.clear_material_pipeline();
        }
        self.studio_shell.console.push(
            "Pipeline nodal retiré — PBR standard restauré.".to_string(),
            LogLevel::Info,
        );
        self.editor_state.set_toast(
            "Matériau Standard",
            "Retour au shader PBR standard.",
            ToastKind::Info,
        );
    }

    /// Traite l'édition d'un champ numérique de l'inspecteur studio
    fn handle_studio_number(&mut self, widget_id: &str, value: f32) {
        let Some(rest) = widget_id.strip_prefix("insp_") else {
            return;
        };
        let mut parts = rest.split('_');
        let kind = parts.next().unwrap_or("");
        let axis = parts.next().unwrap_or("X");
        let axis_idx = match axis {
            "Y" => 1,
            "Z" => 2,
            _ => 0,
        };
        if let Some(scene) = &mut self.scene {
            if scene.selected_node_idx < scene.nodes.len() {
                let node = &mut scene.nodes[scene.selected_node_idx];
                match kind {
                    "pos" => {
                        let mut v = node.transform.translation;
                        v[axis_idx] = value;
                        node.transform.translation = v;
                    }
                    "rot" => {
                        let mut v = node.transform.rotation;
                        v[axis_idx] = value;
                        node.transform.rotation = v;
                    }
                    "scl" => {
                        let mut v = node.transform.scale;
                        v[axis_idx] = value.max(0.001);
                        node.transform.scale = v;
                    }
                    _ => return,
                }
                if let Some(gpu) = &self.gpu_context {
                    node.update_gpu(&gpu.queue, true);
                }
                if let Some(pt) = &mut self.pathtracer {
                    pt.reset_accumulation();
                }
            }
        }
    }

    /// Gestion des clics dans le studio (overlays, shell, canvas nodal)
    fn handle_studio_mouse_down(&mut self) {
        let (width, height) = self
            .renderer
            .as_ref()
            .map(|r| r.window_size())
            .unwrap_or((1, 1));
        let width_f = width as f32;
        let height_f = height as f32;
        let available = Size {
            width: AvailableSpace::Definite(width_f),
            height: AvailableSpace::Definite(height_f),
        };

        // 1. Overlays (menus, toasts, modals) — menus propres au studio
        {
            let mut overlay_tree = WidgetTree::new();
            if let Some(root) = studio::ui::build_studio_overlays(
                &mut overlay_tree,
                &self.studio_shell,
                width_f,
                height_f,
            ) {
                let _ = overlay_tree.compute(root, available);
                if let Ok(Some(event)) = overlay_tree.dispatch_click(root, self.cursor_pos) {
                    self.handle_ui_event(event);
                    return;
                }
            }
        }

        // Si un menu déroulant était ouvert et qu'on a cliqué hors overlay, on le referme
        let had_active_menu = self.studio_shell.active_menu.is_some();
        self.studio_shell.active_menu = None;

        // 2. Shell studio (boutons transport, onglets, arbres, inspecteur)
        let (event, canvas_bounds, viewport_bounds) = {
            let mut tree = WidgetTree::new();
            let layout = build_studio_layout(
                &mut tree,
                self.studio.as_ref(),
                &self.studio_shell,
                self.scene.as_ref(),
                width_f,
                height_f,
            );
            if tree.compute(layout.root, available).is_err() {
                return;
            }
            let mut c_bounds = None;
            let mut v_bounds = None;
            if let Ok(resolved) = tree.resolved_bounds(layout.root) {
                if let Some(canvas) = layout.node_canvas {
                    c_bounds = resolved.get(&canvas).copied();
                }
                if let Some(vp) = layout.viewport {
                    v_bounds = resolved.get(&vp).copied();
                }
            }
            let ev = tree.dispatch_click(layout.root, self.cursor_pos).unwrap_or(None);
            (ev, c_bounds, v_bounds)
        };

        if let Some(event) = event {
            self.handle_ui_event(event);
            return;
        }

        // Si on venait de fermer un menu par un clic extérieur, ne pas démarrer d'autre action
        if had_active_menu {
            return;
        }

        // 3. Interaction avec le Viewport 3D (Sélection d'objet par raycast, Gizmo, ou Orbit Caméra)
        if let Some(vp_bounds) = viewport_bounds.or(self.studio_viewport_bounds) {
            let inside_vp = self.cursor_pos.0 >= vp_bounds[0]
                && self.cursor_pos.0 <= vp_bounds[0] + vp_bounds[2]
                && self.cursor_pos.1 >= vp_bounds[1]
                && self.cursor_pos.1 <= vp_bounds[1] + vp_bounds[3];
            if inside_vp {
                let aspect = (vp_bounds[2] / vp_bounds[3]).max(0.1);
                let (view_proj, _, _) = self.camera.build_view_proj_matrix(aspect);
                let ray = crate::scene::raycast::screen_to_ray(
                    Vec2::new(self.cursor_pos.0, self.cursor_pos.1),
                    vp_bounds,
                    view_proj.inverse(),
                );

                // A. Tester le Gizmo
                let mut gizmo_hit = false;
                if let Some(scene) = &mut self.scene {
                    if scene.selected_node_idx < scene.nodes.len() {
                        let node = &mut scene.nodes[scene.selected_node_idx];
                        if self.gizmo.update(
                            node,
                            &ray,
                            view_proj,
                            vp_bounds,
                            Some(Vec2::new(self.cursor_pos.0, self.cursor_pos.1)),
                            true,
                        ) {
                            gizmo_hit = true;
                        }
                    }
                }

                if !gizmo_hit {
                    if let Some(scene) = &mut self.scene {
                        if let Some(hit_idx) = scene.pick_node(&ray) {
                            scene.selected_node_idx = hit_idx;
                            self.editor_state.selected_tree_id = Some(format!("node_{}", hit_idx));
                        } else {
                            self.camera_dragging = true;
                        }
                    } else {
                        self.camera_dragging = true;
                    }
                }
                return;
            }
        }

        // 4. Interaction directe du canvas nodal (pins, sélection de nœud / pan)
        let mut pending_connect: Option<(uuid::Uuid, uuid::Uuid, uuid::Uuid, uuid::Uuid)> = None;
        if self.studio_shell.active_tab == StudioCenterTab::ShaderEditor || self.studio_shell.active_tab == StudioCenterTab::Split3DShader {
            if let (Some(bounds), Some(session)) = (canvas_bounds, self.studio.as_ref()) {
                let inside = self.cursor_pos.0 >= bounds[0]
                    && self.cursor_pos.0 <= bounds[0] + bounds[2]
                    && self.cursor_pos.1 >= bounds[1]
                    && self.cursor_pos.1 <= bounds[1] + bounds[3];
                if inside {
                    let local = [self.cursor_pos.0 - bounds[0], self.cursor_pos.1 - bounds[1]];
                    let transform = self.studio_shell.canvas;

                    // 4a. Pin sous le curseur : démarre/termine une liaison
                    if let Some(hit) = studio::ui::node_canvas::hit_pin(
                        &session.graph,
                        &transform,
                        local,
                        10.0 * transform.zoom,
                    ) {
                        if hit.is_output {
                            self.studio_shell.pending_link = Some((hit.node, hit.pin));
                            self.studio_shell.console.push(
                                "Liaison: sortie choisie — cliquez maintenant une entrée."
                                    .to_string(),
                                LogLevel::Info,
                            );
                        } else if let Some((from_node, from_pin)) =
                            self.studio_shell.pending_link.take()
                        {
                            pending_connect = Some((from_node, from_pin, hit.node, hit.pin));
                        }
                        // Ne pas démarrer de pan dans tous les cas de pin
                        if pending_connect.is_none() {
                            return;
                        }
                    } else if let Some(node_id) =
                        studio::ui::hit_node(&session.graph, &transform, local)
                    {
                        self.studio_shell.selected_graph_node = Some(node_id);
                        let gpos = transform.screen_to_graph(local);
                        let offset = session
                            .graph
                            .node(node_id)
                            .map(|n| [gpos[0] - n.position[0], gpos[1] - n.position[1]])
                            .unwrap_or([0.0, 0.0]);
                        self.studio_shell.dragging_node = Some((node_id, offset));
                        let label = session
                            .graph
                            .node(node_id)
                            .map(|n| n.kind.label())
                            .unwrap_or("?");
                        self.studio_shell.console.push(
                            format!("Nœud sélectionné: {}", label),
                            LogLevel::Info,
                        );
                        return;
                    }
                }
            }
        }

        if let Some((fn_node, fn_pin, tn_node, tn_pin)) = pending_connect {
            self.connect_graph_pins(fn_node, fn_pin, tn_node, tn_pin);
            return;
        }

        // 5. Clic dans le vide : démarre le pan du canvas
        self.studio_canvas_dragging = true;
    }

    /// Crée une liaison entre deux pins du graphe nodal et recalcule le pipeline
    fn connect_graph_pins(
        &mut self,
        from_node: uuid::Uuid,
        from_pin: uuid::Uuid,
        to_node: uuid::Uuid,
        to_pin: uuid::Uuid,
    ) {
        let result = self
            .studio
            .as_mut()
            .map(|s| s.graph.connect_by_pin_ids(from_node, from_pin, to_node, to_pin));
        match result {
            Some(Ok(_)) => {
                self.studio_shell
                    .console
                    .push("Liaison créée — recompilation du matériau.".to_string(), LogLevel::Success);
                self.studio_apply_material();
            }
            Some(Err(e)) => self
                .studio_shell
                .console
                .push(format!("Liaison refusée: {e}"), LogLevel::Error),
            None => {}
        }
        self.studio_shell.pending_link = None;
    }

    /// Met à jour un paramètre flottant du nœud nodal sélectionné et recompile
    fn studio_set_graph_param(&mut self, key: &str, value: f32) {
        let Some(id) = self.studio_shell.selected_graph_node else {
            return;
        };
        if let Some(session) = self.studio.as_mut() {
            if let Some(node) = session.graph.node_mut(id) {
                node.set_param_f32(key, value);
            }
        }
        self.studio_apply_material();
    }

    /// Met à jour la couleur (r,g,b,a) du nœud nodal sélectionné et recompile
    fn studio_set_graph_color(&mut self, color: [f32; 4]) {
        let Some(id) = self.studio_shell.selected_graph_node else {
            return;
        };
        if let Some(session) = self.studio.as_mut() {
            if let Some(node) = session.graph.node_mut(id) {
                node.set_param_f32("r", color[0]);
                node.set_param_f32("g", color[1]);
                node.set_param_f32("b", color[2]);
                node.set_param_f32("a", color[3]);
            }
        }
        self.studio_apply_material();
    }

    /// Ajoute un nœud au graphe à la position du curseur canvas
    fn studio_add_node(&mut self, kind: NodeKind) {
        let tf = self.studio_shell.canvas;
        let pos = tf.screen_to_graph(self.studio_shell.canvas_cursor);
        if let Some(session) = self.studio.as_mut() {
            let id = session.graph.add_node(kind, pos);
            self.studio_shell.selected_graph_node = Some(id);
            self.studio_shell
                .console
                .push(format!("Nœud ajouté: {}", kind.label()), LogLevel::Success);
            self.studio_apply_material();
        }
    }

    /// Supprime le nœud nodal sélectionné
    fn studio_delete_node(&mut self) {
        let Some(id) = self.studio_shell.selected_graph_node else {
            return;
        };
        if let Some(session) = self.studio.as_mut() {
            if session.graph.remove_node(id) {
                self.studio_shell.selected_graph_node = None;
                self.studio_shell
                    .console
                    .push("Nœud supprimé.".to_string(), LogLevel::Info);
                self.studio_apply_material();
            } else {
                self.studio_shell.console.push(
                    "Le nœud de sortie PBR ne peut pas être supprimé.".to_string(),
                    LogLevel::Warning,
                );
            }
        }
    }

    /// Raccourcis clavier en mode Studio. Retourne true si la touche est consommée.
    fn handle_studio_key(&mut self, code: winit::keyboard::KeyCode) -> bool {
        use winit::keyboard::KeyCode;
        let shader_tab = self.studio_shell.active_tab == StudioCenterTab::ShaderEditor;
        match code {
            KeyCode::F9 => {
                self.toggle_theme();
                true
            }
            KeyCode::Delete | KeyCode::Backspace if shader_tab => {
                self.studio_delete_node();
                true
            }
            KeyCode::KeyU if shader_tab => {
                self.studio_add_node(NodeKind::Uv);
                true
            }
            KeyCode::KeyN if shader_tab => {
                self.studio_add_node(NodeKind::PerlinNoise);
                true
            }
            KeyCode::KeyC if shader_tab => {
                self.studio_add_node(NodeKind::Rgb);
                true
            }
            KeyCode::KeyM if shader_tab => {
                self.studio_add_node(NodeKind::Mix);
                true
            }
            KeyCode::KeyF if shader_tab => {
                self.studio_add_node(NodeKind::Fresnel);
                true
            }
            _ => false,
        }
    }

    fn open_file_dialog(&mut self) {
        let file = rfd::FileDialog::new()
            .set_title("Ouvrir un modèle ou monde 3D")
            .add_filter("Tous formats (.world, .gltf, .glb, .obj)", &["world", "gltf", "glb", "obj", "json"])
            .pick_file();

        if let (Some(path), Some(scene), Some(gpu)) = (file, &mut self.scene, &self.gpu_context) {
            let ext = path.extension().and_then(|e| e.to_str()).map(|s| s.to_lowercase()).unwrap_or_default();
            if ext == "gltf" || ext == "glb" || ext == "obj" {
                match scene.load_model_from_path(&gpu.device, &gpu.queue, &path) {
                    Ok(()) => {
                        let fname = path.file_name().and_then(|n| n.to_str()).unwrap_or("modèle");
                        self.base_vertices.clear();
                        if let Some(pt) = &mut self.pathtracer {
                            pt.update_scene(&gpu.device, &scene.nodes);
                            pt.reset_accumulation();
                        }
                        self.editor_state.selected_tree_id = None;
                        if !scene.nodes.is_empty() {
                            scene.selected_node_idx = scene.nodes.len() - 1;
                        }
                        self.editor_state.set_toast("Modèle Importé", format!("'{}' chargé avec succès !", fname), ToastKind::Success);
                    }
                    Err(e) => {
                        self.editor_state.set_toast("Erreur Import", format!("{}", e), ToastKind::Error);
                    }
                }
            } else {
                match scene.load_world(&gpu.device, &path) {
                    Ok(light_pos) => {
                        if let Some(renderer) = &mut self.forward_renderer {
                            renderer.light_pos = light_pos;
                        }
                        self.current_world_path = Some(path.clone());
                        self.base_vertices.clear();
                        if let Some(pt) = &mut self.pathtracer {
                            pt.update_scene(&gpu.device, &scene.nodes);
                            pt.reset_accumulation();
                        }
                        self.editor_state.selected_tree_id = None;
                        if !scene.nodes.is_empty() {
                            scene.selected_node_idx = 0;
                        }
                        self.editor_state.set_toast("Scène Chargée", format!("{} objets restaurés.", scene.nodes.len()), ToastKind::Success);
                    }
                    Err(e) => {
                        self.editor_state.set_toast("Erreur Chargement", format!("{}", e), ToastKind::Error);
                    }
                }
            }
        }
    }

    fn save_world_quick(&mut self) {
        if let (Some(path), Some(scene), Some(renderer)) = (&self.current_world_path, &self.scene, &self.forward_renderer) {
            match scene.save_world(path, renderer.light_pos) {
                Ok(_) => self.editor_state.set_toast("Sauvegarde", "Monde 3D enregistré.", ToastKind::Success),
                Err(e) => self.editor_state.set_toast("Erreur Sauvegarde", format!("{}", e), ToastKind::Error),
            }
        } else {
            self.save_world_as_dialog();
        }
    }

    fn save_world_as_dialog(&mut self) {
        let file = rfd::FileDialog::new()
            .set_title("Enregistrer la scène 3D")
            .add_filter("Monde 3D (.world)", &["world"])
            .save_file();

        if let (Some(path), Some(scene), Some(renderer)) = (file, &self.scene, &self.forward_renderer) {
            match scene.save_world(&path, renderer.light_pos) {
                Ok(_) => {
                    self.current_world_path = Some(path);
                    self.editor_state.set_toast("Sauvegarde", "Scène enregistrée sous ce fichier.", ToastKind::Success);
                }
                Err(e) => self.editor_state.set_toast("Erreur Sauvegarde", format!("{}", e), ToastKind::Error),
            }
        }
    }

    fn export_aor_dialog(&mut self) {
        let file = rfd::FileDialog::new()
            .set_title("Exporter en Paquet Industriel AOR")
            .add_filter("Paquet AOR (.aor)", &["aor", "aorbundle"])
            .save_file();

        if let (Some(path), Some(scene), Some(renderer)) = (file, &self.scene, &self.forward_renderer) {
            match scene.save_aor(&path, renderer.light_pos) {
                Ok(_) => {
                    let fname = path.file_name().and_then(|n| n.to_str()).unwrap_or("paquet.aor");
                    self.editor_state.set_toast("Export Paquet AOR", format!("'{}' exporté avec succès (intégrité scellée) !", fname), ToastKind::Success);
                    self.studio_shell.console.push(format!("Export Paquet AOR réussi: {:?} ({} nœuds)", path, scene.nodes.len()), LogLevel::Success);
                }
                Err(e) => {
                    self.editor_state.set_toast("Erreur Export AOR", format!("{}", e), ToastKind::Error);
                    self.studio_shell.console.push(format!("Erreur Export AOR: {}", e), LogLevel::Error);
                }
            }
        }
    }

    fn import_aor_dialog(&mut self) {
        let file = rfd::FileDialog::new()
            .set_title("Ouvrir un Paquet Industriel AOR")
            .add_filter("Paquet AOR (.aor)", &["aor", "aorbundle"])
            .pick_file();

        if let (Some(path), Some(scene), Some(gpu)) = (file, &mut self.scene, &self.gpu_context) {
            match scene.load_aor(&gpu.device, &path) {
                Ok(light_pos) => {
                    if let Some(renderer) = &mut self.forward_renderer {
                        renderer.light_pos = light_pos;
                    }
                    self.current_world_path = Some(path.clone());
                    self.base_vertices.clear();
                    if let Some(pt) = &mut self.pathtracer {
                        pt.update_scene(&gpu.device, &scene.nodes);
                        pt.reset_accumulation();
                    }
                    self.editor_state.selected_tree_id = None;
                    if !scene.nodes.is_empty() {
                        scene.selected_node_idx = 0;
                    }
                    let fname = path.file_name().and_then(|n| n.to_str()).unwrap_or("paquet.aor");
                    self.editor_state.set_toast("Paquet AOR Chargé", format!("'{}' chargé ({} nœuds, sécurité validée)", fname, scene.nodes.len()), ToastKind::Success);
                    self.studio_shell.console.push(format!("Chargement AOR réussi: {} objets restaurés avec intégrité validée.", scene.nodes.len()), LogLevel::Success);
                }
                Err(e) => {
                    self.editor_state.set_toast("Erreur Paquet AOR", format!("{}", e), ToastKind::Error);
                    self.studio_shell.console.push(format!("Erreur Paquet AOR: {}", e), LogLevel::Error);
                }
            }
        }
    }

    fn redraw(&mut self) {
        let now = std::time::Instant::now();
        let dt = now.duration_since(self.last_frame_time).as_secs_f32().min(0.1);
        self.last_frame_time = now;
        if dt > 0.0 {
            self.fps = self.fps * 0.9 + (1.0 / dt) * 0.1;
        }

        // 1. Mise à jour de la physique et des déformations
        self.update_physics_and_simulation(dt);

        // 2. Rendu de la scène 3D dans le render target
        self.render_3d_viewport();

        // 3. Construction et rendu de l'interface Cyber-Glass AORUI Multi-Couches
        let Some(renderer) = self.renderer.as_mut() else { return; };
        let (width, height) = renderer.window_size();
        let width_f = width as f32;
        let height_f = height as f32;
        let available = Size {
            width: AvailableSpace::Definite(width_f),
            height: AvailableSpace::Definite(height_f),
        };

        let tri_count = self.scene.as_ref().map(|s| {
            s.nodes.iter().map(|n| n.vertices.len() / 3).sum()
        }).unwrap_or(0);

        let node_count = self.scene.as_ref().map(|s| s.nodes.len()).unwrap_or(0);

        let metrics = EngineMetrics {
            fps: self.fps,
            triangle_count: tri_count,
            node_count,
            camera_mode: match self.camera.mode {
                CameraMode::Orbital => "Orbital",
                CameraMode::FirstPerson => "1ère Personne",
                CameraMode::ThirdPerson => "3ème Personne",
            },
            render_mode: match self.render_mode {
                RenderMode::VideoGame => "Forward PBR",
                RenderMode::PathTrace => "PathTracer GPU",
                RenderMode::Rasterize => "Rasterize Basique",
            },
            status_text: self.status_msg.clone(),
        };

        // --- Layer 0: Base UI (Viewport + Gizmo Vectoriel + Palettes + HUD) ---
        let aspect = (width_f / height_f).max(0.1);
        let (view_proj, _, _) = self.camera.build_view_proj_matrix(aspect);

        let mut base_tree = WidgetTree::new();
        let mut studio_canvas_bounds: Option<[f32; 4]> = None;
        let mut studio_viewport_bounds: Option<[f32; 4]> = None;
        let base_root = if self.studio_mode {
            let layout = build_studio_layout(
                &mut base_tree,
                self.studio.as_ref(),
                &self.studio_shell,
                self.scene.as_ref(),
                width_f,
                height_f,
            );
            if base_tree.compute(layout.root, available).is_err() {
                return;
            }
            if let Ok(bounds) = base_tree.resolved_bounds(layout.root) {
                if let Some(canvas) = layout.node_canvas {
                    studio_canvas_bounds = bounds.get(&canvas).copied();
                }
                if let Some(vp) = layout.viewport {
                    studio_viewport_bounds = bounds.get(&vp).copied();
                }
            }
            layout.root
        } else {
            let root = build_editor_base(
                &mut base_tree,
                &self.editor_state,
                &metrics,
                self.scene.as_ref(),
                Some(&self.gizmo),
                Some(view_proj),
                width_f,
                height_f,
            );
            if base_tree.compute(root, available).is_err() {
                return;
            }
            root
        };
        self.studio_canvas_bounds = studio_canvas_bounds;
        self.studio_viewport_bounds = studio_viewport_bounds;

        let measure = renderer.text_measure();
        let hovered = base_tree.interaction_key_at(base_root, self.cursor_pos).unwrap_or(None);
        let interaction = InteractionState {
            hovered: hovered.as_ref(),
            pressed: self.pressed.as_ref(),
            measure: Some(&measure),
        };

        let base_frame = match base_tree.build_frame(base_root, &self.theme, interaction) {
            Ok(f) => f,
            Err(e) => {
                eprintln!("[AORUI Engine] base build_frame error: {e}");
                return;
            }
        };

        let family = font_family(&self.theme.typography.family);

        let base_text_runs: Vec<_> = base_frame
            .texts
            .iter()
            .map(|spec| {
                let align = match spec.align {
                    ui_widgets::TextAlign::Left => glyphon::cosmic_text::Align::Left,
                    ui_widgets::TextAlign::Center => glyphon::cosmic_text::Align::Center,
                    ui_widgets::TextAlign::Right => glyphon::cosmic_text::Align::Right,
                };
                let weight = match spec.weight {
                    ui_widgets::FontWeight::Normal => glyphon::Weight::NORMAL,
                    ui_widgets::FontWeight::Bold => glyphon::Weight::BOLD,
                };
                renderer.make_text_run(
                    &spec.text,
                    spec.bounds,
                    spec.font_size,
                    spec.color,
                    align,
                    family,
                    weight,
                    spec.clip,
                )
            })
            .collect();

        let base_layer = ui_gpu::RenderLayer {
            instances: &base_frame.instances,
            texts: &base_text_runs,
        };

        let media: Vec<ui_gpu::MediaInstance> = base_frame
            .media
            .iter()
            .map(|spec| {
                let fit_mode = match spec.fit {
                    ui_widgets::MediaFit::Fill => 0,
                    ui_widgets::MediaFit::Contain => 1,
                    ui_widgets::MediaFit::Cover => 2,
                };
                ui_gpu::MediaInstance {
                    kind: spec.kind.0.to_string(),
                    bounds: spec.bounds,
                    resource_id: spec.resource_id.clone(),
                    fit_mode,
                    radius: spec.radius,
                    clip_bounds: spec.clip,
                }
            })
            .collect();

        // --- Layer 1: Overlays (Menus Déroulants, Toasts, Modals) avec flou Dual-Kawase ---
        let mut overlay_tree = WidgetTree::new();
        let overlay_root_opt = if self.studio_mode {
            studio::ui::build_studio_overlays(
                &mut overlay_tree,
                &self.studio_shell,
                width_f,
                height_f,
            )
        } else {
            build_editor_overlays(&mut overlay_tree, &self.editor_state, width_f, height_f)
        };
        if let Some(overlay_root) = overlay_root_opt {
            let _ = overlay_tree.compute(overlay_root, available);
            let overlay_hovered = overlay_tree.interaction_key_at(overlay_root, self.cursor_pos).unwrap_or(None);
            let overlay_interaction = InteractionState {
                hovered: overlay_hovered.as_ref(),
                pressed: self.pressed.as_ref(),
                measure: Some(&measure),
            };

            if let Ok(overlay_frame) = overlay_tree.build_frame(overlay_root, &self.theme, overlay_interaction) {
                let overlay_text_runs: Vec<_> = overlay_frame
                    .texts
                    .iter()
                    .map(|spec| {
                        let align = match spec.align {
                            ui_widgets::TextAlign::Left => glyphon::cosmic_text::Align::Left,
                            ui_widgets::TextAlign::Center => glyphon::cosmic_text::Align::Center,
                            ui_widgets::TextAlign::Right => glyphon::cosmic_text::Align::Right,
                        };
                        let weight = match spec.weight {
                            ui_widgets::FontWeight::Normal => glyphon::Weight::NORMAL,
                            ui_widgets::FontWeight::Bold => glyphon::Weight::BOLD,
                        };
                        renderer.make_text_run(
                            &spec.text,
                            spec.bounds,
                            spec.font_size,
                            spec.color,
                            align,
                            family,
                            weight,
                            spec.clip,
                        )
                    })
                    .collect();

                let overlay_layer = ui_gpu::RenderLayer {
                    instances: &overlay_frame.instances,
                    texts: &overlay_text_runs,
                };

                let _ = renderer.render_layers(
                    wgpu::Color { r: 0.05, g: 0.07, b: 0.12, a: 1.0 },
                    &[base_layer, overlay_layer],
                    &media,
                    &self.resources,
                );
                self.input.clear_transient();
                return;
            }
        }

        let _ = renderer.render_layers(
            wgpu::Color { r: 0.05, g: 0.07, b: 0.12, a: 1.0 },
            &[base_layer],
            &media,
            &self.resources,
        );

        // Réinitialiser les entrées éphémères de la frame
        self.input.clear_transient();
    }
}

impl ApplicationHandler for EngineApplication {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_none() {
            let window_attrs = WindowAttributes::default()
                .with_title("AOR Game Studio — Atelier 3D Cyber-Glass (F10 : atelier classique)")
                .with_inner_size(winit::dpi::LogicalSize::new(1380.0, 840.0));

            let window = Arc::new(
                event_loop
                    .create_window(window_attrs)
                    .expect("Failed to create winit window"),
            );
            self.init_graphics(window);
        }
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => {
                event_loop.exit();
            }
            WindowEvent::Resized(physical_size) => {
                if let Some(renderer) = self.renderer.as_mut() {
                    renderer.resize(physical_size);
                }
                if let (Some(gpu), Some(rt)) = (&self.gpu_context, &mut self.render_target) {
                    *rt = RenderTargetTexture::new(
                        &gpu.device,
                        physical_size.width.max(1),
                        physical_size.height.max(1),
                        wgpu::TextureFormat::Rgba8UnormSrgb,
                        Some("Resized Viewport Target"),
                    );
                    if let Some(post_target) = &mut self.postprocess_target {
                        *post_target = RenderTargetTexture::new(
                            &gpu.device,
                            physical_size.width.max(1),
                            physical_size.height.max(1),
                            wgpu::TextureFormat::Rgba8UnormSrgb,
                            Some("Resized PostProcess Target"),
                        );
                    }
                    if let Some(renderer) = &self.renderer {
                        let initial_pixels = vec![20u8; (rt.width * rt.height * 4) as usize];
                        let new_tex = renderer.create_texture_rgba(rt.width, rt.height, &initial_pixels);
                        self.resources.insert("viewport3d", new_tex.clone());
                        self.viewport_gpu_texture = Some(new_tex);
                    }
                }

                if let Some(window) = &self.window {
                    window.request_redraw();
                }
            }
            WindowEvent::CursorMoved { position, .. } => {
                let (x, y) = (position.x as f32, position.y as f32);
                let dx = x - self.last_mouse_pos.0;
                let dy = y - self.last_mouse_pos.1;
                self.last_mouse_pos = (x, y);
                self.cursor_pos = (x, y);
                self.input.mouse_pos = Vec2::new(x, y);
                self.input.mouse_delta = (dx, dy);

                // Mode Studio : pan du canvas nodal, interaction caméra ou gizmo
                if self.studio_mode {
                    // 1. Déplacement de la caméra 3D
                    if self.camera_dragging {
                        self.camera.yaw += dx * 0.005;
                        self.camera.pitch = (self.camera.pitch + dy * 0.005).clamp(-1.4, 1.4);
                        if let Some(pt) = &mut self.pathtracer {
                            pt.reset_accumulation();
                        }
                        return;
                    }

                    // 2. Interaction avec le Gizmo 3D
                    if self.gizmo.is_interacting() {
                        if let (Some(scene), Some(bounds)) = (&mut self.scene, self.studio_viewport_bounds) {
                            let aspect = (bounds[2] / bounds[3]).max(0.1);
                            let (view_proj, _, _) = self.camera.build_view_proj_matrix(aspect);
                            let ray = crate::scene::raycast::screen_to_ray(
                                Vec2::new(self.cursor_pos.0, self.cursor_pos.1),
                                bounds,
                                view_proj.inverse(),
                            );
                            if scene.selected_node_idx < scene.nodes.len() {
                                let node = &mut scene.nodes[scene.selected_node_idx];
                                self.gizmo.update(
                                    node,
                                    &ray,
                                    view_proj,
                                    bounds,
                                    Some(Vec2::new(self.cursor_pos.0, self.cursor_pos.1)),
                                    true,
                                );
                                if let Some(gpu) = &self.gpu_context {
                                    node.update_gpu(&gpu.queue, true);
                                }
                                if let Some(pt) = &mut self.pathtracer {
                                    pt.reset_accumulation();
                                }
                            }
                        }
                        return;
                    }

                    // 3. Glissement de nœud du graphe shader
                    if let (Some(bounds), Some((node_id, offset))) =
                        (self.studio_canvas_bounds, self.studio_shell.dragging_node)
                    {
                        if self.studio_shell.active_tab == StudioCenterTab::ShaderEditor || self.studio_shell.active_tab == StudioCenterTab::Split3DShader {
                            let tf = self.studio_shell.canvas;
                            let g = tf.screen_to_graph([x - bounds[0], y - bounds[1]]);
                            if let Some(session) = self.studio.as_mut() {
                                if let Some(node) = session.graph.node_mut(node_id) {
                                    node.position = [g[0] - offset[0], g[1] - offset[1]];
                                }
                            }
                        }
                    }
                    if self.studio_canvas_dragging {
                        self.studio_shell.canvas.pan_by([dx, dy]);
                    }
                    if let Some(bounds) = self.studio_canvas_bounds {
                        self.studio_shell.canvas_cursor = [x - bounds[0], y - bounds[1]];
                    }
                    return;
                }

                // 1. Traitement du drag de palette
                if let Some((palette_id, (anchor_x, anchor_y))) = &self.palette_drag {
                    let new_pos = (x - anchor_x, y - anchor_y);
                    match palette_id.as_str() {
                        "tools_palette" => self.editor_state.tools_pos = new_pos,
                        "deformers_palette" => self.editor_state.deformers_pos = new_pos,
                        "inspector_palette" => self.editor_state.inspector_pos = new_pos,
                        _ => {}
                    }
                }
                // 2. Traitement du redimensionnement de palette
                else if let Some((palette_id, (init_w, init_h), (init_x, init_y))) = &self.palette_resize {
                    let new_w = (init_w + (x - init_x)).max(140.0);
                    let new_h = (init_h + (y - init_y)).max(100.0);
                    match palette_id.as_str() {
                        "tools_palette" => self.editor_state.tools_size = (new_w, new_h),
                        "deformers_palette" => self.editor_state.deformers_size = (new_w, new_h),
                        "inspector_palette" => self.editor_state.inspector_size = (new_w, new_h),
                        _ => {}
                    }
                }
                // 3. Traitement du slider continu
                else if let Some(slider_id) = &self.slider_drag {
                    if let Some(renderer) = &self.renderer {
                        let (width, height) = renderer.window_size();
                        let mut test_tree = WidgetTree::new();
                        let metrics = EngineMetrics {
                            fps: self.fps, triangle_count: 0, node_count: 0,
                            camera_mode: "Orbital", render_mode: "Forward PBR", status_text: "".to_string(),
                        };
                        let aspect = (width as f32 / height as f32).max(0.1);
                        let (view_proj, _, _) = self.camera.build_view_proj_matrix(aspect);
                        let root = build_editor_base(&mut test_tree, &self.editor_state, &metrics, self.scene.as_ref(), Some(&self.gizmo), Some(view_proj), width as f32, height as f32);
                        let available = Size { width: AvailableSpace::Definite(width as f32), height: AvailableSpace::Definite(height as f32) };
                        let _ = test_tree.compute(root, available);

                        if let Ok(Some((_, val))) = test_tree.slider_value_at(root, self.cursor_pos) {
                            self.handle_ui_event(UiEvent::SliderChanged { widget_id: slider_id.clone(), value: val });
                        }
                    }
                }
                // 4. Manipulation interactive du Gizmo 3D (Déplacement sur axes X, Y, Z, Centre)
                else if self.gizmo.is_interacting() {
                    if let (Some(scene), Some(renderer)) = (&mut self.scene, &self.renderer) {
                        let (width, height) = renderer.window_size();
                        let width_f = width as f32;
                        let height_f = height as f32;
                        let aspect = (width_f / height_f).max(0.1);
                        let (view_proj, _, _) = self.camera.build_view_proj_matrix(aspect);
                        let ray = crate::scene::raycast::screen_to_ray(
                            Vec2::new(self.cursor_pos.0, self.cursor_pos.1),
                            [0.0, 0.0, width_f, height_f],
                            view_proj.inverse(),
                        );

                        if scene.selected_node_idx < scene.nodes.len() {
                            let node = &mut scene.nodes[scene.selected_node_idx];
                            self.gizmo.update(
                                node,
                                &ray,
                                view_proj,
                                [0.0, 0.0, width_f, height_f],
                                Some(Vec2::new(self.cursor_pos.0, self.cursor_pos.1)),
                                true,
                            );
                            if let Some(gpu) = &self.gpu_context {
                                node.update_gpu(&gpu.queue, true);
                            }
                            if let Some(pt) = &mut self.pathtracer {
                                pt.reset_accumulation();
                            }
                        }
                    }
                }
                // 5. Contrôle de la caméra en mode glisser (si pas d'interaction Gizmo)
                else if self.camera_dragging && !self.physics.is_active {
                    self.camera.yaw += dx * 0.005;
                    self.camera.pitch = (self.camera.pitch + dy * 0.005).clamp(-1.4, 1.4);
                    if let Some(pt) = &mut self.pathtracer {
                        pt.reset_accumulation();
                    }
                } else if self.physics.is_active {
                    self.physics.player.yaw += dx * 0.003;
                    self.physics.player.pitch = (self.physics.player.pitch + dy * 0.003).clamp(-1.4, 1.4);
                    if let Some(pt) = &mut self.pathtracer {
                        pt.reset_accumulation();
                    }
                } else {
                    // Hover test passif du Gizmo quand la souris bouge sans clic
                    if let (Some(scene), Some(renderer)) = (&mut self.scene, &self.renderer) {
                        let (width, height) = renderer.window_size();
                        let width_f = width as f32;
                        let height_f = height as f32;
                        let aspect = (width_f / height_f).max(0.1);
                        let (view_proj, _, _) = self.camera.build_view_proj_matrix(aspect);
                        let ray = crate::scene::raycast::screen_to_ray(
                            Vec2::new(self.cursor_pos.0, self.cursor_pos.1),
                            [0.0, 0.0, width_f, height_f],
                            view_proj.inverse(),
                        );
                        if scene.selected_node_idx < scene.nodes.len() {
                            let node = &mut scene.nodes[scene.selected_node_idx];
                            self.gizmo.update(
                                node,
                                &ray,
                                view_proj,
                                [0.0, 0.0, width_f, height_f],
                                Some(Vec2::new(self.cursor_pos.0, self.cursor_pos.1)),
                                false,
                            );
                        }
                    }
                }
            }
            WindowEvent::MouseInput { state, button, .. } => {
                match button {
                    MouseButton::Left => {
                        let is_down = state == ElementState::Pressed;
                        self.input.mouse_down = is_down;
                        if is_down {
                            self.input.mouse_pressed = true;
                            if self.studio_mode {
                                self.handle_studio_mouse_down();
                                return;
                            }
                            if let Some(renderer) = &self.renderer {
                                let (width, height) = renderer.window_size();
                                let width_f = width as f32;
                                let height_f = height as f32;
                                let available = Size { width: AvailableSpace::Definite(width_f), height: AvailableSpace::Definite(height_f) };

                                // A. Vérifier d'abord les clics sur les Overlays (Menus Déroulants / Modals)
                                let mut overlay_tree = WidgetTree::new();
                                if let Some(overlay_root) = build_editor_overlays(&mut overlay_tree, &self.editor_state, width_f, height_f) {
                                    let _ = overlay_tree.compute(overlay_root, available);
                                    if let Some(event) = overlay_tree.dispatch_click(overlay_root, self.cursor_pos).unwrap_or(None) {
                                        self.handle_ui_event(event);
                                        return;
                                    }
                                }

                                // B. Vérifier les clics sur la Base UI (Boutons, Palettes)
                                let mut test_tree = WidgetTree::new();
                                let metrics = EngineMetrics {
                                    fps: self.fps, triangle_count: 0, node_count: 0,
                                    camera_mode: "Orbital", render_mode: "Forward PBR", status_text: "".to_string(),
                                };
                                let aspect = (width_f / height_f).max(0.1);
                                let (view_proj, _, _) = self.camera.build_view_proj_matrix(aspect);
                                let root = build_editor_base(&mut test_tree, &self.editor_state, &metrics, self.scene.as_ref(), Some(&self.gizmo), Some(view_proj), width_f, height_f);
                                let _ = test_tree.compute(root, available);

                                // Détection du drag de header de palette
                                if let Ok(Some((pal_id, anchor))) = test_tree.palette_drag_anchor_at(root, self.cursor_pos) {
                                    self.palette_drag = Some((pal_id, anchor));
                                }
                                // Détection du redimensionnement de palette
                                else if let Ok(Some((pal_id, (init_w, init_h)))) = test_tree.palette_resize_at(root, self.cursor_pos) {
                                    self.palette_resize = Some((pal_id, (init_w, init_h), self.cursor_pos));
                                }
                                // Détection du début de drag sur un slider
                                else if let Ok(Some((s_id, val))) = test_tree.slider_value_at(root, self.cursor_pos) {
                                    self.slider_drag = Some(s_id.clone());
                                    self.handle_ui_event(UiEvent::SliderChanged { widget_id: s_id, value: val });
                                }
                                // Détection du color picker
                                else if let Ok(Some((cp_id, col))) = test_tree.color_picker_hue_at(root, self.cursor_pos) {
                                    self.color_picker_drag = Some(cp_id.clone());
                                    self.handle_ui_event(UiEvent::ColorChanged { widget_id: cp_id, color: col });
                                }
                                // Dispatch normal des boutons et menus
                                else if let Some(event) = test_tree.dispatch_click(root, self.cursor_pos).unwrap_or(None) {
                                    self.handle_ui_event(event);
                                } else {
                                    // Clic dans le viewport 3D : Fermer les menus déroulants
                                    self.editor_state.active_menu = None;

                                    let ray = crate::scene::raycast::screen_to_ray(
                                        glam::Vec2::new(self.cursor_pos.0, self.cursor_pos.1),
                                        [0.0, 0.0, width_f, height_f],
                                        view_proj.inverse(),
                                    );

                                    // C. Tester d'abord si le clic saisit un axe du Gizmo 3D
                                    let mut gizmo_hit = false;
                                    if let Some(scene) = &mut self.scene {
                                        if scene.selected_node_idx < scene.nodes.len() {
                                            let node = &mut scene.nodes[scene.selected_node_idx];
                                            if self.gizmo.update(
                                                node,
                                                &ray,
                                                view_proj,
                                                [0.0, 0.0, width_f, height_f],
                                                Some(Vec2::new(self.cursor_pos.0, self.cursor_pos.1)),
                                                true,
                                            ) {
                                                gizmo_hit = true;
                                            }
                                        }
                                    }

                                    if !gizmo_hit {
                                        // D. Sélection 3D par Lancer de Rayon (Raycast picking sur les nœuds)
                                        let mut picked_new = false;
                                        if let Some(scene) = &mut self.scene {
                                            if let Some(hit_idx) = scene.pick_node(&ray) {
                                                scene.selected_node_idx = hit_idx;
                                                self.editor_state.selected_tree_id = Some(format!("node_{}", hit_idx));
                                                if hit_idx < scene.nodes.len() {
                                                    let node = &scene.nodes[hit_idx];
                                                    self.editor_state.roughness = node.material.roughness;
                                                    self.editor_state.metallic = node.material.metallic;
                                                    if let Some(first_v) = node.vertices.first() {
                                                        self.editor_state.current_color = first_v.color;
                                                    }
                                                    self.editor_state.set_toast(
                                                        "Objet Sélectionné",
                                                        format!("'{}' (Nœud #{})", node.name, hit_idx),
                                                        ToastKind::Info,
                                                    );
                                                }
                                                picked_new = true;
                                            }
                                        }

                                        if !picked_new {
                                            // Activer la rotation caméra si on a cliqué dans le vide
                                            self.camera_dragging = true;
                                        }
                                    }
                                }
                            }
                        } else {
                            // Relâchement du clic gauche
                            self.input.mouse_released = true;
                            self.camera_dragging = false;
                            self.palette_drag = None;
                            self.palette_resize = None;
                            self.slider_drag = None;
                            self.color_picker_drag = None;
                            self.studio_canvas_dragging = false;
                            self.studio_shell.dragging_node = None;

                            // Terminer le drag du Gizmo
                            if let (Some(scene), Some(renderer)) = (&mut self.scene, &self.renderer) {
                                let (width, height) = renderer.window_size();
                                let aspect = (width as f32 / height as f32).max(0.1);
                                let (view_proj, _, _) = self.camera.build_view_proj_matrix(aspect);
                                let ray = crate::scene::raycast::screen_to_ray(
                                    Vec2::new(self.cursor_pos.0, self.cursor_pos.1),
                                    [0.0, 0.0, width as f32, height as f32],
                                    view_proj.inverse(),
                                );
                                if scene.selected_node_idx < scene.nodes.len() {
                                    let node = &mut scene.nodes[scene.selected_node_idx];
                                    self.gizmo.update(
                                        node,
                                        &ray,
                                        view_proj,
                                        [0.0, 0.0, width as f32, height as f32],
                                        Some(Vec2::new(self.cursor_pos.0, self.cursor_pos.1)),
                                        false,
                                    );
                                }
                            }
                        }
                    }
                    MouseButton::Right => {
                        let is_down = state == ElementState::Pressed;
                        self.input.right_mouse_down = is_down;
                        if self.studio_mode {
                            if let Some(bounds) = self.studio_viewport_bounds {
                                let inside = self.cursor_pos.0 >= bounds[0]
                                    && self.cursor_pos.0 <= bounds[0] + bounds[2]
                                    && self.cursor_pos.1 >= bounds[1]
                                    && self.cursor_pos.1 <= bounds[1] + bounds[3];
                                if inside {
                                    self.camera_dragging = is_down;
                                    return;
                                }
                            }
                        }
                        self.camera_dragging = is_down;
                    }
                    MouseButton::Middle => {
                        let is_down = state == ElementState::Pressed;
                        if self.studio_mode {
                            if let Some(bounds) = self.studio_viewport_bounds {
                                let inside = self.cursor_pos.0 >= bounds[0]
                                    && self.cursor_pos.0 <= bounds[0] + bounds[2]
                                    && self.cursor_pos.1 >= bounds[1]
                                    && self.cursor_pos.1 <= bounds[1] + bounds[3];
                                if inside {
                                    self.camera_dragging = is_down;
                                    return;
                                }
                            }
                            self.studio_canvas_dragging = is_down;
                            return;
                        }
                        if is_down {
                            if let Some(renderer) = &self.renderer {
                                let (width, height) = renderer.window_size();
                                let aspect = (width as f32 / height as f32).max(0.1);
                                let (view_proj, _, _) = self.camera.build_view_proj_matrix(aspect);
                                let ray = crate::scene::raycast::screen_to_ray(
                                    Vec2::new(self.cursor_pos.0, self.cursor_pos.1),
                                    [0.0, 0.0, width as f32, height as f32],
                                    view_proj.inverse(),
                                );
                                // Intersection avec le plan de sol (y = 0)
                                let hit_pos = ray.intersect_plane(Vec3::ZERO, Vec3::Y)
                                    .map(|t| ray.at(t))
                                    .unwrap_or_else(|| ray.origin + ray.direction * 8.0);
                                self.trigger_explosion(hit_pos, 42.0);
                            }
                        }
                    }
                    _ => {}
                }
            }
            WindowEvent::MouseWheel { delta, .. } => {
                let scroll_y = match delta {
                    MouseScrollDelta::LineDelta(_, y) => y * 20.0,
                    MouseScrollDelta::PixelDelta(pos) => pos.y as f32,
                };
                self.input.scroll_delta = scroll_y;

                // Mode Studio : zoom du canvas nodal ou zoom de la caméra 3D
                if self.studio_mode {
                    // Zoom sur la vue 3D
                    if let Some(bounds) = self.studio_viewport_bounds {
                        let inside = self.cursor_pos.0 >= bounds[0]
                            && self.cursor_pos.0 <= bounds[0] + bounds[2]
                            && self.cursor_pos.1 >= bounds[1]
                            && self.cursor_pos.1 <= bounds[1] + bounds[3];
                        if inside {
                            self.camera.distance = (self.camera.distance - scroll_y * 0.1).clamp(1.5, 50.0);
                            if let Some(pt) = &mut self.pathtracer {
                                pt.reset_accumulation();
                            }
                            return;
                        }
                    }

                    // Zoom sur le canvas nodal
                    if self.studio_shell.active_tab == StudioCenterTab::ShaderEditor || self.studio_shell.active_tab == StudioCenterTab::Split3DShader {
                        if let Some(bounds) = self.studio_canvas_bounds {
                            let inside = self.cursor_pos.0 >= bounds[0]
                                && self.cursor_pos.0 <= bounds[0] + bounds[2]
                                && self.cursor_pos.1 >= bounds[1]
                                && self.cursor_pos.1 <= bounds[1] + bounds[3];
                            if inside {
                                let local =
                                    [self.cursor_pos.0 - bounds[0], self.cursor_pos.1 - bounds[1]];
                                let factor = (1.0 + (scroll_y * 0.002)).clamp(0.5, 2.0);
                                self.studio_shell.canvas.zoom_at(local, factor);
                            }
                        }
                    }
                    return;
                }

                if !self.physics.is_active {
                    self.camera.distance = (self.camera.distance - scroll_y * 0.1).clamp(1.5, 50.0);
                    if let Some(pt) = &mut self.pathtracer {
                        pt.reset_accumulation();
                    }
                }
            }
            WindowEvent::KeyboardInput { event, .. } => {
                if let PhysicalKey::Code(code) = event.physical_key {
                    if event.state == ElementState::Pressed {
                        self.input.keys_down.insert(code);
                        self.input.keys_pressed.insert(code);

                        if self.studio_mode && self.handle_studio_key(code) {
                            return;
                        }

                        match code {
                            KeyCode::KeyG => {
                                self.physics.is_active = !self.physics.is_active;
                                self.editor_state.physics_active = self.physics.is_active;
                                self.editor_state.set_toast(
                                    "Mode Jeu",
                                    if self.physics.is_active { "Contrôles FPS activés (ZQSD/WASD, Espace)" } else { "Mode Éditeur 3D restauré" },
                                    ToastKind::Info,
                                );
                            }
                            KeyCode::Digit1 => {
                                if let (Some(scene), Some(gpu)) = (&mut self.scene, &self.gpu_context) {
                                    scene.add_node(&gpu.device, PrimitiveType::Cube);
                                    self.editor_state.set_toast("Ajout Cube", "Cube primitif ajouté.", ToastKind::Success);
                                }
                            }
                            KeyCode::Digit2 => {
                                if let (Some(scene), Some(gpu)) = (&mut self.scene, &self.gpu_context) {
                                    scene.add_node(&gpu.device, PrimitiveType::Pyramid);
                                    self.editor_state.set_toast("Ajout Pyramide", "Pyramide ajoutée.", ToastKind::Success);
                                }
                            }
                            KeyCode::Digit3 => {
                                if let (Some(scene), Some(gpu)) = (&mut self.scene, &self.gpu_context) {
                                    scene.add_node(&gpu.device, PrimitiveType::Prism);
                                    self.editor_state.set_toast("Ajout Prisme", "Prisme ajouté.", ToastKind::Success);
                                }
                            }
                            KeyCode::Digit4 => {
                                if let (Some(scene), Some(gpu)) = (&mut self.scene, &self.gpu_context) {
                                    let base_pos = Vec3::new(0.0, 0.0, 0.0);
                                    scene.spawn_cube_tower(&gpu.device, base_pos, 3, 3);
                                    self.physics.sync_from_scene_nodes(&scene.nodes);
                                    self.editor_state.set_toast("Tour de Cubes", "Pile physique destructible générée !", ToastKind::Success);
                                }
                            }
                            KeyCode::Digit5 => {
                                if let (Some(scene), Some(gpu)) = (&mut self.scene, &self.gpu_context) {
                                    let pos = Vec3::new(0.0, 0.0, 0.0);
                                    let id = scene.spawn_rigged_humanoid(&gpu.device, pos);
                                    self.editor_state.set_toast(
                                        "Mannequin Riggé Animé",
                                        format!("Personnage articulé #{} invoqué avec squelette & animations !", id),
                                        ToastKind::Success,
                                    );
                                }
                            }
                            KeyCode::KeyE => {

                                // Déclencher une explosion à la position du rayon de visée ou de l'objet sélectionné
                                let pos = if let Some(scene) = &self.scene {
                                    if scene.selected_node_idx < scene.nodes.len() {
                                        scene.nodes[scene.selected_node_idx].transform.translation
                                    } else {
                                        Vec3::new(0.0, 0.5, 0.0)
                                    }
                                } else {
                                    Vec3::new(0.0, 0.5, 0.0)
                                };
                                self.trigger_explosion(pos, 38.0);
                            }
                            KeyCode::F1 => {
                                self.editor_state.show_shortcuts_modal = !self.editor_state.show_shortcuts_modal;
                            }
                            KeyCode::F10 => {
                                self.toggle_studio_mode();
                            }
                            KeyCode::F9 => {
                                self.toggle_theme();
                            }
                            KeyCode::Escape => {
                                self.editor_state.active_menu = None;
                                self.studio_shell.active_menu = None;
                                self.editor_state.show_shortcuts_modal = false;
                            }
                            KeyCode::KeyN => {
                                if let Some(scene) = &mut self.scene {
                                    scene.select_next_node();
                                }
                            }
                            KeyCode::Tab => {
                                if let Some(scene) = &mut self.scene {
                                    scene.is_edit_mode = !scene.is_edit_mode;
                                }
                            }
                            KeyCode::Delete | KeyCode::Backspace => {
                                if let Some(scene) = &mut self.scene {
                                    scene.remove_selected_node();
                                    self.editor_state.set_toast("Suppression", "Objet supprimé.", ToastKind::Warning);
                                }
                            }
                            _ => {}
                        }
                    } else {
                        self.input.keys_down.remove(&code);
                    }
                }
            }
            WindowEvent::RedrawRequested => {
                self.redraw();
                if let Some(window) = &self.window {
                    window.request_redraw();
                }
            }
            _ => {}
        }
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        if let Some(window) = &self.window {
            window.request_redraw();
        }
    }
}
