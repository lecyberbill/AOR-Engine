// [WFGY] Zone: SAFE | λ: 0.3 | Fallbacks: 0/None | Action: Standalone Blade Runner Cyber City Game Executable
#![allow(dead_code)]
use std::sync::Arc;
use glam::{Quat, Vec3};
use winit::application::ApplicationHandler;
use winit::event::{ElementState, KeyEvent, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::keyboard::{KeyCode, PhysicalKey};
use winit::window::{Window, WindowAttributes, WindowId};

use ui_gpu::GpuRenderer;
use ui_layout::{AvailableSpace, Size};
use ui_widgets::{FontFamily, InteractionState, ListItemBadge, MediaFit, Theme, WidgetId, WidgetTree};

use rust_moteur_3d::gpu::{GpuContext, RenderTargetTexture};
use rust_moteur_3d::scene::{Camera, Scene};
use rust_moteur_3d::renderer::{ForwardRenderer, PostProcessPipeline, RenderMode};
use rust_moteur_3d::particles::ParticleSystem;
use rust_moteur_3d::audio::{SoundEffect, SpatialAudioEngine};
use rust_moteur_3d::game::{generate_cyber_megalopolis, generate_cyber_spinner_mesh, CyberCitySystem, CyberSpinner};

fn main() {
    let event_loop = EventLoop::new().expect("Failed to create event loop");
    event_loop.set_control_flow(ControlFlow::Poll);

    let mut app = BladeRunnerGameApp::new();
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

struct KeyState {
    pub w: bool,
    pub s: bool,
    pub a: bool,
    pub d: bool,
    pub space: bool,
    pub ctrl: bool,
    pub shift: bool,
    pub q: bool,
    pub e: bool,
}

impl Default for KeyState {
    fn default() -> Self {
        Self {
            w: false,
            s: false,
            a: false,
            d: false,
            space: false,
            ctrl: false,
            shift: false,
            q: false,
            e: false,
        }
    }
}

struct BladeRunnerGameApp {
    window: Option<Arc<Window>>,
    renderer: Option<GpuRenderer>,
    resources: ui_gpu::ResourceTable,
    theme: Theme,

    gpu_context: Option<GpuContext>,
    render_target: Option<RenderTargetTexture>,
    postprocess_target: Option<RenderTargetTexture>,
    viewport_gpu_texture: Option<Arc<ui_gpu::GpuTexture>>,
    forward_renderer: Option<ForwardRenderer>,
    postprocess: Option<PostProcessPipeline>,
    particles: Option<ParticleSystem>,
    audio: SpatialAudioEngine,

    camera: Camera,
    scene: Option<Scene>,
    spinner: CyberSpinner,
    city_system: CyberCitySystem,
    spinner_node_idx: usize,

    keys: KeyState,
    last_frame_time: std::time::Instant,
    fps: f32,
    total_time: f32,
    race_notification: Option<(String, std::time::Instant)>,
}

impl BladeRunnerGameApp {
    fn new() -> Self {
        let camera = Camera::new(
            Vec3::new(0.0, 16.0, -10.0),
            7.5,
            std::f32::consts::PI,
            0.15,
        );

        Self {
            window: None,
            renderer: None,
            resources: ui_gpu::ResourceTable::new(),
            theme: Theme::cyber_glass(),

            gpu_context: None,
            render_target: None,
            postprocess_target: None,
            viewport_gpu_texture: None,
            forward_renderer: None,
            postprocess: None,
            particles: None,
            audio: SpatialAudioEngine::new(),

            camera,
            scene: None,
            spinner: CyberSpinner::new(Vec3::new(0.0, 18.0, -25.0)),
            city_system: CyberCitySystem::default(),
            spinner_node_idx: 0,

            keys: KeyState::default(),
            last_frame_time: std::time::Instant::now(),
            fps: 60.0,
            total_time: 0.0,
            race_notification: None,
        }
    }

    fn init_gpu(&mut self, window: Arc<Window>) {
        let size = window.inner_size();
        let mut renderer = GpuRenderer::new(window.clone());
        renderer.resize(size);

        let device = renderer.device_arc();
        let queue = renderer.queue_arc();
        let surface_format = renderer.surface_format();

        let context = GpuContext::from_shared(
            device,
            queue,
            surface_format,
            wgpu::AdapterInfo {
                name: "Primary GPU".to_string(),
                vendor: 0,
                device: 0,
                device_type: wgpu::DeviceType::DiscreteGpu,
                driver: "Vulkan/Metal/DX12".to_string(),
                driver_info: "".to_string(),
                backend: wgpu::Backend::Vulkan,
            },
        );

        let rt_width = size.width.max(1);
        let rt_height = size.height.max(1);
        let rt_format = wgpu::TextureFormat::Rgba8UnormSrgb;

        let render_target = RenderTargetTexture::new(&context.device, rt_width, rt_height, rt_format, Some("Game Render Target"));
        let postprocess_target = RenderTargetTexture::new(&context.device, rt_width, rt_height, rt_format, Some("Game PostProcess Target"));

        let mut scene = Scene::new(&context.device, &context.queue);
        generate_cyber_megalopolis(&mut scene, &context.device);

        // Ajout du maillage du Spinner
        let (v_spin, i_spin) = generate_cyber_spinner_mesh();
        let s_idx = scene.nodes.len();
        scene.add_custom_mesh(&context.device, "Cyber_Spinner_Vehicle", v_spin, i_spin);
        if let Some(node) = scene.nodes.get_mut(s_idx) {
            node.material = rust_moteur_3d::scene::MaterialUniform {
                roughness: 0.2,
                metallic: 0.85,
                uv_tiling: [1.0, 1.0],
                uv_offset: [0.0, 0.0],
                ior: 1.5,
                transmission: 0.0,
                emission_color: [0.1, 2.5, 3.8, 1.0], // Réacteurs néon cyan
                use_normal_map: 0,
                clearcoat: 1.0,
                clearcoat_roughness: 0.03,
                subsurface: 0.1,
            };
        }
        self.spinner_node_idx = s_idx;

        let forward_renderer = ForwardRenderer::new(&context, rt_format, &scene.model_bind_group_layout);
        let postprocess = PostProcessPipeline::new(&context.device, rt_format);
        let particles = ParticleSystem::new(&context, rt_format, 2000);

        let initial_pixels = vec![15u8; (rt_width * rt_height * 4) as usize];
        let viewport_gpu_texture = renderer.create_texture_rgba(rt_width, rt_height, &initial_pixels);
        self.resources.insert("viewport3d", viewport_gpu_texture.clone());

        self.gpu_context = Some(context);
        self.render_target = Some(render_target);
        self.postprocess_target = Some(postprocess_target);
        self.viewport_gpu_texture = Some(viewport_gpu_texture);
        self.scene = Some(scene);
        self.forward_renderer = Some(forward_renderer);
        self.postprocess = Some(postprocess);
        self.particles = Some(particles);
        self.renderer = Some(renderer);
        self.window = Some(window);
    }

    fn update_game_simulation(&mut self, dt: f32) {
        self.total_time += dt;

        let mut throttle = 0.0;
        let mut steer_yaw = 0.0;
        let pitch_input = 0.0;
        let mut strafe = 0.0;
        let mut vertical = 0.0;

        if self.keys.w { throttle += 1.0; }
        if self.keys.s { throttle -= 0.6; }
        if self.keys.a || self.keys.q { steer_yaw += 1.0; }
        if self.keys.d { steer_yaw -= 1.0; }
        if self.keys.space { vertical += 1.0; }
        if self.keys.ctrl { vertical -= 1.0; }
        if self.keys.e { strafe += 1.0; }

        let boost_requested = self.keys.shift;

        // 1. Physique de vol du Spinner
        self.spinner.update(dt, throttle, steer_yaw, pitch_input, strafe, vertical, boost_requested);

        // 2. Synchronisation du maillage
        if let (Some(scene), Some(gpu)) = (&mut self.scene, &self.gpu_context) {
            if let Some(node) = scene.nodes.get_mut(self.spinner_node_idx) {
                node.transform.translation = self.spinner.position;
                node.transform.rotation = self.spinner.rotation;
                node.update_gpu(&gpu.queue, false);
            }
        }

        // 3. Caméra de poursuite fluide cinématique (Smooth Chase Camera)
        let rot_quat = Quat::from_euler(glam::EulerRot::YXZ, self.spinner.rotation.y, self.spinner.rotation.x, self.spinner.rotation.z);
        let cam_dist = if self.spinner.is_boosting { 9.5 } else { 7.5 };
        self.camera.distance += (cam_dist - self.camera.distance) * (5.0 * dt).min(1.0);
        self.camera.target = self.spinner.position + Vec3::new(0.0, 0.9, 0.0);
        self.camera.yaw = self.spinner.rotation.y + std::f32::consts::PI;
        self.camera.pitch = (self.spinner.rotation.x * 0.4 + 0.15).clamp(-0.6, 0.6);

        // 4. Particules de traînée de plasma des réacteurs
        if let Some(particles) = &mut self.particles {
            let (left_ex, right_ex) = self.spinner.exhaust_positions();
            let back_dir = -(rot_quat * Vec3::Z).normalize_or_zero();
            particles.spawn_sparks(left_ex, back_dir, if self.spinner.is_boosting { 8 } else { 2 });
            particles.spawn_sparks(right_ex, back_dir, if self.spinner.is_boosting { 8 } else { 2 });
            particles.update(dt, Vec3::new(0.0, -1.0, 0.0));
        }

        // 5. Checkpoints de course
        if let Some(passed_idx) = self.city_system.update(dt, self.spinner.position) {
            self.audio.play_spatial_sound(SoundEffect::Laser, self.spinner.position, 1.2);
            self.race_notification = Some((
                format!("⚡ PORTE #{}/{} FRANCHIE ! (+500 PTS)", passed_idx + 1, self.city_system.checkpoints.len()),
                std::time::Instant::now(),
            ));
        }

        // 6. Audio Spatialisé (Mise à jour auditeur et sons de réacteur)
        let cam_eye = self.camera.position();
        let cam_fwd = self.camera.forward();
        let cam_right = cam_fwd.cross(Vec3::Y).normalize_or_zero();
        self.audio.update_listener(cam_eye, cam_fwd, cam_right);
        self.audio.update();
    }

    fn render_game_viewport(&mut self) {
        let Some(gpu) = &self.gpu_context else { return; };
        let Some(scene) = &mut self.scene else { return; };
        let Some(render_target) = &self.render_target else { return; };
        let Some(post_target) = &self.postprocess_target else { return; };
        let Some(viewport_tex) = &self.viewport_gpu_texture else { return; };

        // Préparer les particules sur le GPU
        if let Some(particles) = &mut self.particles {
            particles.prepare_gpu(&gpu.queue, &self.camera, render_target.aspect_ratio(), self.total_time);
        }

        let mut encoder = gpu.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("blade runner encoder"),
        });

        if let Some(renderer) = &mut self.forward_renderer {
            // Éclairage dynamique Blade Runner : lumière cyan/magenta au niveau du Spinner
            renderer.light_pos = self.spinner.position + Vec3::new(0.0, 5.0, 2.0);

            renderer.render(
                gpu,
                &render_target.color_view,
                &render_target.depth_texture.view,
                scene,
                &self.camera,
                render_target.aspect_ratio(),
                wgpu::Color { r: 0.01, g: 0.015, b: 0.03, a: 1.0 }, // Ciel nuit noire profonde
                self.total_time,
                render_target.width as f32,
                render_target.height as f32,
                RenderMode::VideoGame,
                self.particles.as_ref(),
            );
        }

        // Passe Post-Processing HDR & Bloom Cinématographique
        if let Some(postprocess) = &self.postprocess {
            let bind_group = postprocess.create_bind_group(&gpu.device, &render_target.color_view);
            postprocess.render(&mut encoder, &bind_group, &post_target.color_view);
        }

        // Copier la texture rendue vers la texture de l'UI
        encoder.copy_texture_to_texture(
            wgpu::ImageCopyTexture {
                texture: &post_target.texture,
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

    fn redraw(&mut self) {
        let now = std::time::Instant::now();
        let dt = now.duration_since(self.last_frame_time).as_secs_f32().min(0.1);
        self.last_frame_time = now;
        if dt > 0.0 {
            self.fps = self.fps * 0.9 + (1.0 / dt) * 0.1;
        }

        // 1. Simulation de jeu
        self.update_game_simulation(dt);

        // 2. Rendu 3D HDR
        self.render_game_viewport();

        // 3. Rendu du HUD Holographique Minimaliste Cyberpunk
        let (width_f, height_f) = if let Some(r) = &self.renderer {
            let (w, h) = r.window_size();
            (w as f32, h as f32)
        } else {
            return;
        };

        let mut tree = WidgetTree::new();
        let root = self.build_game_hud(&mut tree, width_f, height_f);

        let Some(renderer) = self.renderer.as_mut() else { return; };

        let available = Size {
            width: AvailableSpace::Definite(width_f),
            height: AvailableSpace::Definite(height_f),
        };
        if tree.compute(root, available).is_err() {
            return;
        }

        let measure = renderer.text_measure();
        let interaction = InteractionState {
            hovered: None,
            pressed: None,
            measure: Some(&measure),
        };

        if let Ok(frame) = tree.build_frame(root, &self.theme, interaction) {
            let family = font_family(&self.theme.typography.family);
            let text_runs: Vec<_> = frame
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

            let layer = ui_gpu::RenderLayer {
                instances: &frame.instances,
                texts: &text_runs,
            };

            let media: Vec<ui_gpu::MediaInstance> = frame
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

            let _ = renderer.render_layers(
                wgpu::Color { r: 0.01, g: 0.015, b: 0.03, a: 1.0 },
                &[layer],
                &media,
                &self.resources,
            );
        }

        if let Some(window) = &self.window {
            window.request_redraw();
        }
    }

    fn build_game_hud(&self, tree: &mut WidgetTree, width: f32, height: f32) -> ui_layout::NodeId {
        use ui_layout::{length, auto, AlignItems, Display, FlexDirection, Position, Rect, Style};

        let mut root_children = Vec::new();

        // Viewport 3D plein écran d'arrière-plan
        let viewport_style = Style {
            position: Position::Absolute,
            inset: Rect { top: length(0.0), left: length(0.0), right: auto(), bottom: auto() },
            size: Size { width: length(width), height: length(height) },
            ..Default::default()
        };
        let viewport_node = tree.image(WidgetId::new("game_viewport"), "viewport3d", MediaFit::Fill, viewport_style).unwrap();
        root_children.push(viewport_node);

        // 1. HUD Haut : Titre du Jeu, Portes franchies & Chronomètre
        let title_badge = tree.badge("🚀 BLADE RUNNER 2099", ListItemBadge::Success, Style { size: Size { width: length(190.0), height: length(26.0) }, ..Default::default() }).unwrap();
        let cp_badge = tree.badge(
            format!("🏁 PORTES: {} / {}", self.city_system.current_checkpoint_idx, self.city_system.checkpoints.len()),
            ListItemBadge::Warning,
            Style { size: Size { width: length(140.0), height: length(26.0) }, ..Default::default() },
        ).unwrap();
        let score_badge = tree.badge(
            format!("⭐ SCORE: {}", self.city_system.score),
            ListItemBadge::None,
            Style { size: Size { width: length(120.0), height: length(26.0) }, ..Default::default() },
        ).unwrap();
        let time_badge = tree.badge(
            format!("⏱ {:.1}s", self.city_system.race_time),
            ListItemBadge::None,
            Style { size: Size { width: length(90.0), height: length(26.0) }, ..Default::default() },
        ).unwrap();
        let fps_badge = tree.badge(
            format!("{:.0} FPS", self.fps),
            ListItemBadge::None,
            Style { size: Size { width: length(80.0), height: length(26.0) }, ..Default::default() },
        ).unwrap();

        let top_hud = tree.container(
            &[title_badge, cp_badge, score_badge, time_badge, fps_badge],
            Style {
                position: Position::Absolute,
                inset: Rect { top: length(16.0), left: length(20.0), right: auto(), bottom: auto() },
                display: Display::Flex,
                flex_direction: FlexDirection::Row,
                gap: Size { width: length(12.0), height: length(0.0) },
                align_items: Some(AlignItems::Center),
                ..Default::default()
            },
        ).unwrap();
        root_children.push(top_hud);

        // 2. Notification de Checkpoint au centre de l'écran (si active)
        if let Some((msg, time)) = &self.race_notification {
            if time.elapsed().as_secs_f32() < 2.5 {
                let notif_badge = tree.badge(msg, ListItemBadge::Success, Style { size: Size { width: length(360.0), height: length(32.0) }, ..Default::default() }).unwrap();
                let notif_box = tree.container(
                    &[notif_badge],
                    Style {
                        position: Position::Absolute,
                        inset: Rect { top: length(70.0), left: length((width - 360.0) * 0.5), right: auto(), bottom: auto() },
                        ..Default::default()
                    },
                ).unwrap();
                root_children.push(notif_box);
            }
        }

        // 3. HUD Bas : Télémétrie de Vol (Vitesse, Jauge de Boost, Commandes)
        let speed_str = format!("⚡ {:.0} KM/H", self.spinner.speed_kmh());
        let speed_badge = tree.badge(
            speed_str,
            if self.spinner.is_boosting { ListItemBadge::Warning } else { ListItemBadge::Success },
            Style { size: Size { width: length(120.0), height: length(28.0) }, ..Default::default() },
        ).unwrap();

        let boost_str = format!("🔥 TURBO BOOST: {:.0}%", self.spinner.boost_fuel);
        let boost_badge = tree.badge(
            boost_str,
            if self.spinner.boost_fuel > 20.0 { ListItemBadge::Success } else { ListItemBadge::Warning },
            Style { size: Size { width: length(170.0), height: length(28.0) }, ..Default::default() },
        ).unwrap();

        let alt_str = format!("▲ ALT: {:.1}m", self.spinner.position.y);
        let alt_badge = tree.badge(
            alt_str,
            ListItemBadge::None,
            Style { size: Size { width: length(110.0), height: length(28.0) }, ..Default::default() },
        ).unwrap();

        let controls_lbl = tree.label_muted(
            "ZQSD: Vol | A/D: Roulis | Espace/Ctrl: Altitude | Shift: Boost Turbo",
            Style { size: Size { width: length(450.0), height: length(28.0) }, ..Default::default() },
        ).unwrap();

        let bottom_hud = tree.container(
            &[speed_badge, boost_badge, alt_badge, controls_lbl],
            Style {
                position: Position::Absolute,
                inset: Rect { top: auto(), left: length(20.0), right: length(20.0), bottom: length(16.0) },
                display: Display::Flex,
                flex_direction: FlexDirection::Row,
                gap: Size { width: length(14.0), height: length(0.0) },
                align_items: Some(AlignItems::Center),
                ..Default::default()
            },
        ).unwrap();
        root_children.push(bottom_hud);

        tree.container(
            &root_children,
            Style {
                size: Size { width: length(width), height: length(height) },
                ..Default::default()
            },
        ).unwrap()
    }
}

impl ApplicationHandler for BladeRunnerGameApp {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_none() {
            let win_attr = WindowAttributes::default()
                .with_title("Blade Runner 2099 — Cyber Spinner Standalone")
                .with_inner_size(winit::dpi::LogicalSize::new(1400.0, 900.0));
            let window = Arc::new(event_loop.create_window(win_attr).expect("Failed to create window"));
            self.init_gpu(window);
        }
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => {
                event_loop.exit();
            }
            WindowEvent::Resized(new_size) => {
                if let (Some(renderer), Some(gpu)) = (&mut self.renderer, &self.gpu_context) {
                    renderer.resize(new_size);
                    let rt_w = new_size.width.max(1);
                    let rt_h = new_size.height.max(1);
                    let rt_format = wgpu::TextureFormat::Rgba8UnormSrgb;
                    let target = RenderTargetTexture::new(&gpu.device, rt_w, rt_h, rt_format, Some("PostProcess Target"));
                    self.render_target = Some(RenderTargetTexture::new(&gpu.device, rt_w, rt_h, rt_format, Some("Render Target")));
                    let initial_pixels = vec![15u8; (rt_w * rt_h * 4) as usize];
                    let viewport_gpu_texture = renderer.create_texture_rgba(rt_w, rt_h, &initial_pixels);
                    self.resources.insert("viewport3d", viewport_gpu_texture.clone());
                    self.viewport_gpu_texture = Some(viewport_gpu_texture);
                    self.postprocess_target = Some(target);
                }
            }
            WindowEvent::KeyboardInput { event: KeyEvent { physical_key: PhysicalKey::Code(key), state, .. }, .. } => {
                let is_down = state == ElementState::Pressed;
                match key {
                    KeyCode::KeyW | KeyCode::KeyZ | KeyCode::ArrowUp => self.keys.w = is_down,
                    KeyCode::KeyS | KeyCode::ArrowDown => self.keys.s = is_down,
                    KeyCode::KeyA | KeyCode::ArrowLeft => self.keys.a = is_down,
                    KeyCode::KeyD | KeyCode::ArrowRight => self.keys.d = is_down,
                    KeyCode::KeyQ => self.keys.q = is_down,
                    KeyCode::KeyE => self.keys.e = is_down,
                    KeyCode::Space => self.keys.space = is_down,
                    KeyCode::ControlLeft | KeyCode::ControlRight | KeyCode::KeyC => self.keys.ctrl = is_down,
                    KeyCode::ShiftLeft | KeyCode::ShiftRight => self.keys.shift = is_down,
                    KeyCode::Escape => event_loop.exit(),
                    _ => {}
                }
            }
            WindowEvent::RedrawRequested => {
                self.redraw();
            }
            _ => {}
        }
    }
}
