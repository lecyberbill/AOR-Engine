// [WFGY] Zone: SAFE | λ: 0.25 | Fallbacks: 0/None | Action: EngineApp High-Level Runtime & Event Loop Encapsulation
use glam::Vec3;
use crate::gpu::{GpuContext, RenderTargetTexture};
use crate::particles::ParticleSystem;
use crate::renderer::{ForwardRenderer, RenderMode};
use crate::runtime::builder::WorldBuilder;
use crate::scene::{Camera, Scene};

#[derive(Clone, Debug)]
pub struct EngineConfig {
    pub title: String,
    pub width: u32,
    pub height: u32,
    pub render_mode: RenderMode,
    pub clear_color: [f64; 4],
    pub enable_taa: bool,
    pub enable_ssr: bool,
    pub enable_volumetric_fog: bool,
}

impl Default for EngineConfig {
    fn default() -> Self {
        Self {
            title: "AOR 3D Engine".to_string(),
            width: 1280,
            height: 720,
            render_mode: RenderMode::VideoGame,
            clear_color: [0.02, 0.015, 0.04, 1.0],
            enable_taa: true,
            enable_ssr: true,
            enable_volumetric_fog: true,
        }
    }
}

#[derive(Default, Clone, Debug)]
pub struct InputState {
    pub keys_down: std::collections::HashSet<String>,
    pub mouse_pos: [f32; 2],
    pub mouse_delta: [f32; 2],
    pub mouse_buttons: [bool; 3],
}

pub struct EngineApp {
    pub config: EngineConfig,
    pub context: GpuContext,
    pub scene: Scene,
    pub camera: Camera,
    pub renderer: ForwardRenderer,
    pub particles: ParticleSystem,
    pub render_target: RenderTargetTexture,
    pub input: InputState,
    pub time: f32,
    pub target_format: wgpu::TextureFormat,
}

impl EngineApp {
    pub fn new(config: EngineConfig) -> Result<Self, String> {
        let context = GpuContext::new_standalone().map_err(|e| format!("GPU Context Error: {e}"))?;
        let target_format = wgpu::TextureFormat::Rgba8UnormSrgb;

        let scene = Scene::new(&context.device, &context.queue);
        let camera = Camera::new(Vec3::new(0.0, 4.0, 0.0), 16.0, 0.55, 0.28);
        let renderer = ForwardRenderer::new(&context, target_format, &scene.model_bind_group_layout);
        let particles = ParticleSystem::new(&context, target_format, 2000);
        let render_target = RenderTargetTexture::new(
            &context.device,
            config.width,
            config.height,
            target_format,
            Some("EngineApp Main RenderTarget"),
        );

        Ok(Self {
            config,
            context,
            scene,
            camera,
            renderer,
            particles,
            render_target,
            input: InputState::default(),
            time: 0.0,
            target_format,
        })
    }

    /// Accès au WorldBuilder fluide pour instancier la scène
    pub fn world(&mut self) -> WorldBuilder<'_> {
        WorldBuilder::new(&mut self.scene, &self.context.device, &self.context.queue)
    }

    /// Redimensionne la résolution interne du moteur
    pub fn resize(&mut self, width: u32, height: u32) {
        if self.config.width == width && self.config.height == height {
            return;
        }
        self.config.width = width.max(1);
        self.config.height = height.max(1);
        self.render_target = RenderTargetTexture::new(
            &self.context.device,
            self.config.width,
            self.config.height,
            self.target_format,
            Some("EngineApp Main RenderTarget Resized"),
        );
    }

    /// Exécute une frame de mise à jour et de rendu complet
    pub fn render_frame(&mut self, dt: f32) {
        self.time += dt;

        // Mise à jour des particules
        self.particles.update(dt);
        let aspect = self.config.width as f32 / self.config.height as f32;
        self.particles.prepare_gpu(&self.context.queue, &self.camera, aspect, self.time);

        // Passe de Rendu Principale
        let clear = wgpu::Color {
            r: self.config.clear_color[0],
            g: self.config.clear_color[1],
            b: self.config.clear_color[2],
            a: self.config.clear_color[3],
        };

        self.renderer.render(
            &self.context,
            &self.render_target.color_view,
            &self.render_target.depth_texture.view,
            &self.scene,
            &self.camera,
            aspect,
            clear,
            self.time,
            self.config.width as f32,
            self.config.height as f32,
            self.config.render_mode,
            Some(&self.particles),
        );

        // Passe SSR si activée
        if self.config.enable_ssr && self.config.render_mode == RenderMode::VideoGame {
            self.renderer.resolve_ssr(
                &self.context,
                &self.render_target.color_view,
                &self.render_target.depth_texture.view,
                self.config.width,
                self.config.height,
                self.target_format,
            );
        }

        // Passe TAA si activée
        if self.config.enable_taa && self.config.render_mode == RenderMode::VideoGame {
            self.renderer.resolve_taa(
                &self.context,
                &self.render_target.color_view,
                &self.render_target.depth_texture.view,
                &self.camera,
                aspect,
                self.config.width,
                self.config.height,
                self.target_format,
            );
        }
    }
}
