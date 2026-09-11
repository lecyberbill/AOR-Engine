// [WFGY] Zone: SAFE | λ: 0.3 | Fallbacks: 0/None | Action: Standalone Interactive Playable 3D DOOM (E1M1 Hangar) with Window & FPS Controls
#![allow(dead_code)]
use std::sync::Arc;
use glam::Vec3;
use winit::application::ApplicationHandler;
use winit::event::{ElementState, KeyEvent, MouseButton, WindowEvent};
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
use rust_moteur_3d::runtime::{MaterialBuilder, WorldBuilder};
use rust_moteur_3d::game::doom::{
    build_hangar_level, generate_shotgun_mesh, DoomEntity, EntityEvent, EntityKind, FpsPlayer,
};

fn main() {
    println!("============================================================");
    println!("🔥 DOOM (1993) - E1M1 HANGAR REMASTERISÉ 3D PBR WGPU");
    println!("🎮 Contrôles : ZQSD / Flèches = Déplacement | Souris = Visée");
    println!("💥 Clic Gauche = Tirer au Shotgun | Shift = Courir | E = Interagir");
    println!("============================================================");

    let event_loop = EventLoop::new().expect("Failed to create event loop");
    event_loop.set_control_flow(ControlFlow::Poll);

    let mut app = DoomHangarGameApp::new();
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

#[derive(Default)]
struct KeyState {
    pub forward: bool,
    pub backward: bool,
    pub left: bool,
    pub right: bool,
    pub sprint: bool,
}

pub struct DoomHangarGameApp {
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
    player: FpsPlayer,
    weapon_node_idx: usize,
    entities: Vec<DoomEntity>,

    keys: KeyState,
    last_mouse_pos: Option<(f32, f32)>,
    mouse_delta: (f32, f32),

    last_frame_time: std::time::Instant,
    fps: f32,
    total_time: f32,
    score: u32,
    notification: Option<(String, std::time::Instant)>,
}

impl DoomHangarGameApp {
    pub fn new() -> Self {
        let spawn_pos = Vec3::new(0.0, 1.8, -12.0);
        let mut player = FpsPlayer::new(spawn_pos);
        player.yaw = 2.8;
        player.pitch = -0.05;

        let mut camera = Camera::new(spawn_pos, 0.1, player.yaw, player.pitch);
        camera.mode = rust_moteur_3d::scene::CameraMode::FirstPerson;

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
            player,
            weapon_node_idx: 0,
            entities: Vec::new(),

            keys: KeyState::default(),
            last_mouse_pos: None,
            mouse_delta: (0.0, 0.0),

            last_frame_time: std::time::Instant::now(),
            fps: 60.0,
            total_time: 0.0,
            score: 0,
            notification: Some(("BIENVENUE DANS E1M1 HANGAR !".to_string(), std::time::Instant::now())),
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
                name: "DOOM Primary GPU".to_string(),
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

        let render_target = RenderTargetTexture::new(&context.device, rt_width, rt_height, rt_format, Some("Doom RT"));
        let postprocess_target = RenderTargetTexture::new(&context.device, rt_width, rt_height, rt_format, Some("Doom PostProcess RT"));

        let mut scene = Scene::new(&context.device, &context.queue);

        // Construction du niveau E1M1
        let entities = {
            let mut world = WorldBuilder::new(&mut scene, &context.device, &context.queue);
            build_hangar_level(&mut world)
        };
        self.entities = entities;

        // Modèle 3D Shotgun en vue subjective
        let (shotgun_v, shotgun_i) = generate_shotgun_mesh();
        let weapon_idx = scene.nodes.len();
        scene.add_custom_mesh(&context.device, "Player_Combat_Shotgun", shotgun_v, shotgun_i);
        if let Some(w_node) = scene.nodes.get_mut(weapon_idx) {
            w_node.material = MaterialBuilder::car_paint(0.7, 0.03)
                .metallic(0.9)
                .roughness(0.12)
                .build();
        }
        self.weapon_node_idx = weapon_idx;

        let mut forward_renderer = ForwardRenderer::new(&context, rt_format, &scene.model_bind_group_layout);
        forward_renderer.light_pos = Vec3::new(-10.0, 16.0, -15.0);

        let postprocess = PostProcessPipeline::new(&context.device, rt_format);
        let particles = ParticleSystem::new(&context, rt_format, 2000);

        let initial_pixels = vec![10u8; (rt_width * rt_height * 4) as usize];
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

    fn fire_weapon(&mut self) {
        if !self.player.weapon.can_fire() {
            return;
        }

        if self.player.weapon.fire() {
            self.audio.play_spatial_sound(SoundEffect::Laser, self.player.position, 1.3);

            let (w_pos, w_rot) = self.player.compute_weapon_world_transform();
            let muzzle_pos = w_pos + w_rot * Vec3::new(0.0, 0.03, 0.75);

            if let Some(particles) = &mut self.particles {
                particles.spawn_sparks(muzzle_pos, self.camera.forward(), 15);
            }

            // Raycast des plombs du fusil
            let forward = self.camera.forward();
            let player_eye = self.player.position + Vec3::new(0.0, self.player.height * 0.9, 0.0);

            for entity in &mut self.entities {
                let to_ent = entity.position - player_eye;
                let proj = to_ent.dot(forward);
                if proj > 0.5 && proj < 25.0 {
                    let closest_pt = player_eye + forward * proj;
                    let dist_to_line = (entity.position - closest_pt).length();
                    if dist_to_line < entity.radius + 0.3 {
                        if let Some(event) = entity.apply_damage(45.0) {
                            match event {
                                EntityEvent::BarrelExploded { position, radius, damage } => {
                                    self.audio.play_spatial_sound(SoundEffect::Explosion, position, 2.0);
                                    if let Some(particles) = &mut self.particles {
                                        particles.spawn_explosion(position, 1.2);
                                    }
                                    self.score += 250;
                                    self.notification = Some((
                                        format!("💥 BARIL EXPLOSIF DÉTRUIT ! (+250 PTS)"),
                                        std::time::Instant::now(),
                                    ));

                                    // Dégâts de souffle au joueur s'il est trop près
                                    let dist = (self.player.position - position).length();
                                    if dist < radius {
                                        let falloff = 1.0 - (dist / radius);
                                        self.player.take_damage((damage * falloff) as i32);
                                    }
                                }
                                EntityEvent::ImpDied { .. } => {
                                    self.score += 500;
                                    self.notification = Some((
                                        "💀 DÉMON IMP ÉLIMINÉ ! (+500 PTS)".to_string(),
                                        std::time::Instant::now(),
                                    ));
                                }
                                _ => {}
                            }
                        }
                    }
                }
            }
        }
    }

    fn update_game(&mut self, dt: f32) {
        self.total_time += dt;

        let mut input_forward = 0.0;
        let mut input_strafe = 0.0;

        if self.keys.forward { input_forward += 1.0; }
        if self.keys.backward { input_forward -= 1.0; }
        if self.keys.left { input_strafe -= 1.0; }
        if self.keys.right { input_strafe += 1.0; }

        let (dx, dy) = self.mouse_delta;
        self.mouse_delta = (0.0, 0.0);

        // 1. Déplacement du joueur FPS
        self.player.update(dt, input_forward, input_strafe, self.keys.sprint, dx, dy);
        self.player.sync_camera(&mut self.camera);

        // 2. Mise à jour de l'entité Fusil à pompe 3D
        let (w_pos, w_rot) = self.player.compute_weapon_world_transform();
        if let (Some(scene), Some(gpu)) = (&mut self.scene, &self.gpu_context) {
            if let Some(w_node) = scene.nodes.get_mut(self.weapon_node_idx) {
                w_node.transform.translation = w_pos;
                let (euler_y, euler_x, euler_z) = w_rot.to_euler(glam::EulerRot::YXZ);
                w_node.transform.rotation = Vec3::new(euler_x, euler_y, euler_z);
                w_node.update_gpu(&gpu.queue, false);
            }
        }

        // 3. Mise à jour des entités interactives du niveau
        let player_pos = self.player.position;
        for ent in &mut self.entities {
            if let Some(event) = ent.update(dt, player_pos) {
                match event {
                    EntityEvent::ImpAttack { damage } => {
                        self.player.take_damage(damage);
                        self.notification = Some((
                            format!("⚠ ATTAQUE DE DÉMON ! (-{} VIE)", damage),
                            std::time::Instant::now(),
                        ));
                    }
                    _ => {}
                }
            }

            // Mise à jour de la translation de la porte si active
            if let (Some(scene), Some(gpu), Some(node_idx)) = (&mut self.scene, &self.gpu_context, ent.node_idx) {
                if let EntityKind::Door { open_amount, .. } = ent.kind {
                    if let Some(node) = scene.nodes.get_mut(node_idx) {
                        node.transform.translation.y = ent.position.y + open_amount * 3.8;
                        node.update_gpu(&gpu.queue, false);
                    }
                }
            }
        }

        // 4. Audio spatialisé
        let cam_eye = self.camera.position();
        let cam_fwd = self.camera.forward();
        let cam_right = cam_fwd.cross(Vec3::Y).normalize_or_zero();
        self.audio.update_listener(cam_eye, cam_fwd, cam_right);
        self.audio.update();

        // 5. Particules
        if let Some(particles) = &mut self.particles {
            particles.update(dt);
        }
    }

    fn render_game_viewport(&mut self) {
        let Some(gpu) = &self.gpu_context else { return; };
        let Some(scene) = &mut self.scene else { return; };
        let Some(render_target) = &self.render_target else { return; };
        let Some(post_target) = &self.postprocess_target else { return; };
        let Some(viewport_tex) = &self.viewport_gpu_texture else { return; };

        let aspect = render_target.aspect_ratio();

        if let Some(particles) = &mut self.particles {
            particles.prepare_gpu(&gpu.queue, &self.camera, aspect, self.total_time);
        }

        let mut encoder = gpu.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("doom hangar encoder"),
        });

        if let Some(renderer) = &mut self.forward_renderer {
            renderer.render(
                gpu,
                &render_target.color_view,
                &render_target.depth_texture.view,
                scene,
                &self.camera,
                aspect,
                wgpu::Color { r: 0.01, g: 0.015, b: 0.02, a: 1.0 },
                self.total_time,
                render_target.width as f32,
                render_target.height as f32,
                RenderMode::VideoGame,
                self.particles.as_ref(),
            );
        }

        if let Some(postprocess) = &self.postprocess {
            let bind_group = postprocess.create_bind_group(&gpu.device, &render_target.color_view);
            postprocess.render(&mut encoder, &bind_group, &post_target.color_view);
        }

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

        self.update_game(dt);
        self.render_game_viewport();

        let (width_f, height_f) = if let Some(r) = &self.renderer {
            let (w, h) = r.window_size();
            (w as f32, h as f32)
        } else {
            return;
        };

        let mut tree = WidgetTree::new();
        let root = self.build_doom_hud(&mut tree, width_f, height_f);

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
                wgpu::Color { r: 0.01, g: 0.015, b: 0.02, a: 1.0 },
                &[layer],
                &media,
                &self.resources,
            );
        }

        if let Some(window) = &self.window {
            window.request_redraw();
        }
    }

    fn build_doom_hud(&self, tree: &mut WidgetTree, width: f32, height: f32) -> ui_layout::NodeId {
        use ui_layout::{length, auto, AlignItems, Display, FlexDirection, Position, Rect, Style};

        let mut root_children = Vec::new();

        // 0. Arrière-plan Viewport 3D
        let viewport_style = Style {
            position: Position::Absolute,
            inset: Rect { top: length(0.0), left: length(0.0), right: auto(), bottom: auto() },
            size: Size { width: length(width), height: length(height) },
            ..Default::default()
        };
        let viewport_node = tree.image(WidgetId::new("doom_viewport"), "viewport3d", MediaFit::Fill, viewport_style).unwrap();
        root_children.push(viewport_node);

        // 1. Viseur central (Crosshair réticule épuré)
        let crosshair_size = 8.0;
        let crosshair_node = tree.badge(
            "+",
            ListItemBadge::Success,
            Style {
                position: Position::Absolute,
                inset: Rect {
                    top: length((height - crosshair_size) * 0.5),
                    left: length((width - crosshair_size) * 0.5),
                    right: auto(),
                    bottom: auto(),
                },
                size: Size { width: length(crosshair_size * 2.0), height: length(crosshair_size * 2.0) },
                ..Default::default()
            },
        ).unwrap();
        root_children.push(crosshair_node);

        // 2. Barre d'état supérieure (DOOM E1M1, Score, FPS)
        let title_badge = tree.badge("🔥 DOOM : E1M1 HANGAR", ListItemBadge::Warning, Style { size: Size { width: length(190.0), height: length(26.0) }, ..Default::default() }).unwrap();
        let score_badge = tree.badge(
            format!("⭐ SCORE: {}", self.score),
            ListItemBadge::None,
            Style { size: Size { width: length(130.0), height: length(26.0) }, ..Default::default() },
        ).unwrap();
        let fps_badge = tree.badge(
            format!("{:.0} FPS", self.fps),
            ListItemBadge::None,
            Style { size: Size { width: length(80.0), height: length(26.0) }, ..Default::default() },
        ).unwrap();

        let top_hud = tree.container(
            &[title_badge, score_badge, fps_badge],
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

        // 3. Notification d'événement (centre haut)
        if let Some((msg, time)) = &self.notification {
            if time.elapsed().as_secs_f32() < 2.5 {
                let notif_badge = tree.badge(msg, ListItemBadge::Success, Style { size: Size { width: length(360.0), height: length(32.0) }, ..Default::default() }).unwrap();
                let notif_box = tree.container(
                    &[notif_badge],
                    Style {
                        position: Position::Absolute,
                        inset: Rect { top: length(65.0), left: length((width - 360.0) * 0.5), right: auto(), bottom: auto() },
                        ..Default::default()
                    },
                ).unwrap();
                root_children.push(notif_box);
            }
        }

        // 4. Barre d'état inférieure classique DOOM (Santé, Armure, Munitions, Commandes)
        let hp_badge = tree.badge(
            format!("❤️ VIE: {}%", self.player.health),
            if self.player.health > 25 { ListItemBadge::Success } else { ListItemBadge::Warning },
            Style { size: Size { width: length(110.0), height: length(28.0) }, ..Default::default() },
        ).unwrap();

        let arm_badge = tree.badge(
            format!("🛡️ ARMURE: {}%", self.player.armor),
            ListItemBadge::None,
            Style { size: Size { width: length(130.0), height: length(28.0) }, ..Default::default() },
        ).unwrap();

        let ammo_badge = tree.badge(
            format!("💥 CARTOUCHES: {}/{}", self.player.weapon.ammo, self.player.weapon.max_ammo),
            if self.player.weapon.ammo > 5 { ListItemBadge::Success } else { ListItemBadge::Warning },
            Style { size: Size { width: length(180.0), height: length(28.0) }, ..Default::default() },
        ).unwrap();

        let help_lbl = tree.label_muted(
            "Clic Gauche / Espace: Tirer | ZQSD: Avancer | Souris: Viser | Shift: Courir",
            Style { size: Size { width: length(480.0), height: length(28.0) }, ..Default::default() },
        ).unwrap();

        let bottom_hud = tree.container(
            &[hp_badge, arm_badge, ammo_badge, help_lbl],
            Style {
                position: Position::Absolute,
                inset: Rect { top: length(height - 48.0), left: length(20.0), right: auto(), bottom: auto() },
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

impl ApplicationHandler for DoomHangarGameApp {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_none() {
            let win_attr = WindowAttributes::default()
                .with_title("DOOM (1993) - E1M1 Hangar Remastered 3D PBR [AOR-Engine]")
                .with_inner_size(winit::dpi::LogicalSize::new(1360.0, 820.0));
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
                    let initial_pixels = vec![10u8; (rt_w * rt_h * 4) as usize];
                    let viewport_gpu_texture = renderer.create_texture_rgba(rt_w, rt_h, &initial_pixels);
                    self.resources.insert("viewport3d", viewport_gpu_texture.clone());
                    self.viewport_gpu_texture = Some(viewport_gpu_texture);
                    self.postprocess_target = Some(target);
                }
            }
            WindowEvent::CursorMoved { position, .. } => {
                let (x, y) = (position.x as f32, position.y as f32);
                if let Some((last_x, last_y)) = self.last_mouse_pos {
                    self.mouse_delta = (x - last_x, y - last_y);
                }
                self.last_mouse_pos = Some((x, y));
            }
            WindowEvent::MouseInput { state: ElementState::Pressed, button, .. } => {
                match button {
                    MouseButton::Left => {
                        self.fire_weapon();
                    }
                    _ => {}
                }
            }
            WindowEvent::KeyboardInput { event: KeyEvent { physical_key: PhysicalKey::Code(key), state, .. }, .. } => {
                let is_down = state == ElementState::Pressed;
                match key {
                    KeyCode::KeyW | KeyCode::KeyZ | KeyCode::ArrowUp => self.keys.forward = is_down,
                    KeyCode::KeyS | KeyCode::ArrowDown => self.keys.backward = is_down,
                    KeyCode::KeyA | KeyCode::KeyQ | KeyCode::ArrowLeft => self.keys.left = is_down,
                    KeyCode::KeyD | KeyCode::ArrowRight => self.keys.right = is_down,
                    KeyCode::ShiftLeft | KeyCode::ShiftRight => self.keys.sprint = is_down,
                    KeyCode::Space => {
                        if is_down {
                            self.fire_weapon();
                        }
                    }
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
