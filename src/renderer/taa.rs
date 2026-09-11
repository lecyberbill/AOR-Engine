// [WFGY] Zone: SAFE | λ: 0.35 | Fallbacks: 0/None | Action: Temporal Anti-Aliasing (TAA) Pipeline with Halton Jittering & Neighborhood Color Clamping
#![allow(dead_code)]

use glam::{Mat4, Vec2};
use crate::gpu::UniformBuffer;

#[repr(C)]
#[derive(Clone, Copy, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct TaaUniform {
    pub current_inv_view_proj: [[f32; 4]; 4],
    pub prev_view_proj: [[f32; 4]; 4],
    pub jitter_offset: [f32; 2],
    pub screen_size: [f32; 2],
    pub feedback_factor: f32, // Typiquement 0.90..0.95 pour un TAA doux
    pub _pad0: f32,
    pub _pad1: f32,
    pub _pad2: f32,
}

impl Default for TaaUniform {
    fn default() -> Self {
        Self {
            current_inv_view_proj: Mat4::IDENTITY.to_cols_array_2d(),
            prev_view_proj: Mat4::IDENTITY.to_cols_array_2d(),
            jitter_offset: [0.0, 0.0],
            screen_size: [960.0, 640.0],
            feedback_factor: 0.92,
            _pad0: 0.0,
            _pad1: 0.0,
            _pad2: 0.0,
        }
    }
}

/// Séquence quasi-aléatoire de Halton (base 2, base 3) pour le jittering sub-pixel
pub fn halton_sequence(index: usize, base: usize) -> f32 {
    let mut f = 1.0;
    let mut r = 0.0;
    let mut i = index;
    while i > 0 {
        f /= base as f32;
        r += f * (i % base) as f32;
        i /= base;
    }
    r
}

pub struct TaaPipeline {
    pub pipeline: wgpu::RenderPipeline,
    pub bind_group_layout: wgpu::BindGroupLayout,
    pub uniform_buffer: UniformBuffer<TaaUniform>,
    pub linear_sampler: wgpu::Sampler,
    pub depth_sampler: wgpu::Sampler,
    pub history_texture: wgpu::Texture,
    pub history_view: wgpu::TextureView,
    pub output_texture: wgpu::Texture,
    pub output_view: wgpu::TextureView,
    pub width: u32,
    pub height: u32,
    pub frame_index: usize,
    pub prev_view_proj: Mat4,
}

impl TaaPipeline {
    pub fn new(device: &wgpu::Device, width: u32, height: u32, format: wgpu::TextureFormat) -> Self {
        let uniform_data = TaaUniform::default();
        let uniform_buffer = UniformBuffer::new(device, Some("TAA Uniform Buffer"), &uniform_data);

        let linear_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("TAA Linear Sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });

        let depth_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("TAA Point Depth Sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });

        let history_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("TAA History Texture"),
            size: wgpu::Extent3d {
                width: width.max(1),
                height: height.max(1),
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        let history_view = history_texture.create_view(&wgpu::TextureViewDescriptor::default());

        let output_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("TAA Output Texture"),
            size: wgpu::Extent3d {
                width: width.max(1),
                height: height.max(1),
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        let output_view = output_texture.create_view(&wgpu::TextureViewDescriptor::default());

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("TAA Bind Group Layout"),
            entries: &[
                // binding 0: Uniform Buffer
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                // binding 1: Current Frame Color Texture
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                // binding 2: History Color Texture
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                // binding 3: Camera Depth Texture
                wgpu::BindGroupLayoutEntry {
                    binding: 3,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Depth,
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                // binding 4: Linear Sampler
                wgpu::BindGroupLayoutEntry {
                    binding: 4,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
                // binding 5: Depth Sampler
                wgpu::BindGroupLayoutEntry {
                    binding: 5,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::NonFiltering),
                    count: None,
                },
            ],
        });

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("TAA WGSL Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("../shaders/taa.wgsl").into()),
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("TAA Pipeline Layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("TAA Render Pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "vs_fullscreen",
                buffers: &[],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: "fs_taa",
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: None,
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                cull_mode: None,
                ..Default::default()
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
        });

        Self {
            pipeline,
            bind_group_layout,
            uniform_buffer,
            linear_sampler,
            depth_sampler,
            history_texture,
            history_view,
            output_texture,
            output_view,
            width,
            height,
            frame_index: 0,
            prev_view_proj: Mat4::IDENTITY,
        }
    }

    /// Calcule le décalage de jittering sub-pixel pour la trame actuelle (en espace NDC [-1, 1])
    pub fn compute_jitter(&mut self) -> (Vec2, [f32; 2]) {
        let sample_idx = (self.frame_index % 8) + 1;
        let h_x = halton_sequence(sample_idx, 2) - 0.5;
        let h_y = halton_sequence(sample_idx, 3) - 0.5;

        let pixel_w = 2.0 / self.width.max(1) as f32;
        let pixel_h = 2.0 / self.height.max(1) as f32;

        let ndc_jitter = Vec2::new(h_x * pixel_w, h_y * pixel_h);
        let uv_jitter = [h_x / self.width.max(1) as f32, h_y / self.height.max(1) as f32];

        (ndc_jitter, uv_jitter)
    }

    pub fn resize(&mut self, device: &wgpu::Device, width: u32, height: u32, format: wgpu::TextureFormat) {
        if self.width == width && self.height == height {
            return;
        }

        self.width = width.max(1);
        self.height = height.max(1);

        self.history_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("TAA Resized History Texture"),
            size: wgpu::Extent3d {
                width: self.width,
                height: self.height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        self.history_view = self.history_texture.create_view(&wgpu::TextureViewDescriptor::default());

        self.output_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("TAA Resized Output Texture"),
            size: wgpu::Extent3d {
                width: self.width,
                height: self.height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        self.output_view = self.output_texture.create_view(&wgpu::TextureViewDescriptor::default());
    }

    /// Exécute la passe de résolution TAA et copie le résultat dans la texture d'historique pour la trame suivante
    pub fn render(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        current_color_view: &wgpu::TextureView,
        depth_view: &wgpu::TextureView,
        current_view_proj: Mat4,
        uv_jitter: [f32; 2],
    ) {
        let inv_vp = current_view_proj.inverse();
        let uniform_data = TaaUniform {
            current_inv_view_proj: inv_vp.to_cols_array_2d(),
            prev_view_proj: self.prev_view_proj.to_cols_array_2d(),
            jitter_offset: uv_jitter,
            screen_size: [self.width as f32, self.height as f32],
            feedback_factor: if self.frame_index == 0 { 0.0 } else { 0.92 },
            _pad0: 0.0,
            _pad1: 0.0,
            _pad2: 0.0,
        };
        self.uniform_buffer.update(queue, &uniform_data);

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("TAA Frame Bind Group"),
            layout: &self.bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: self.uniform_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(current_color_view),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::TextureView(&self.history_view),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: wgpu::BindingResource::TextureView(depth_view),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: wgpu::BindingResource::Sampler(&self.linear_sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 5,
                    resource: wgpu::BindingResource::Sampler(&self.depth_sampler),
                },
            ],
        });

        {
            let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("TAA Resolve Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &self.output_view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            rpass.set_pipeline(&self.pipeline);
            rpass.set_bind_group(0, &bind_group, &[]);
            rpass.draw(0..3, 0..1);
        }

        // Copie du résultat résolu vers la texture d'historique pour la trame N+1
        encoder.copy_texture_to_texture(
            wgpu::ImageCopyTexture {
                texture: &self.output_texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::ImageCopyTexture {
                texture: &self.history_texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::Extent3d {
                width: self.width,
                height: self.height,
                depth_or_array_layers: 1,
            },
        );

        self.prev_view_proj = current_view_proj;
        self.frame_index = self.frame_index.wrapping_add(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_taa_uniform_size_and_alignment() {
        assert_eq!(std::mem::size_of::<TaaUniform>() % 16, 0);
        assert_eq!(std::mem::size_of::<TaaUniform>(), 160); // 64 + 64 + 8 + 8 + 4 + 12 = 160
    }

    #[test]
    fn test_halton_sequence_bounds() {
        for i in 1..=16 {
            let h2 = halton_sequence(i, 2);
            let h3 = halton_sequence(i, 3);
            assert!(h2 >= 0.0 && h2 <= 1.0);
            assert!(h3 >= 0.0 && h3 <= 1.0);
        }
    }
}
