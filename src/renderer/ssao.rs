// [WFGY] Zone: SAFE | λ: 0.3 | Fallbacks: 0/None | Action: Screen-Space Ambient Occlusion (SSAO) Pipeline with Bilateral Blur
#![allow(dead_code)]

use crate::gpu::UniformBuffer;
use glam::Mat4;

#[repr(C)]
#[derive(Clone, Copy, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct SsaoUniform {
    pub proj: [[f32; 4]; 4],
    pub inv_proj: [[f32; 4]; 4],
    pub kernel: [[f32; 4]; 32],
    pub radius: f32,
    pub bias: f32,
    pub screen_width: f32,
    pub screen_height: f32,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct BlurUniform {
    pub texel_size: [f32; 2],
    pub _pad0: f32,
    pub _pad1: f32,
}

pub struct SsaoPipeline {
    pub ssao_pipeline: wgpu::RenderPipeline,
    pub blur_pipeline: wgpu::RenderPipeline,
    pub ssao_bind_group_layout: wgpu::BindGroupLayout,
    pub blur_bind_group_layout: wgpu::BindGroupLayout,
    pub ssao_uniform_buffer: UniformBuffer<SsaoUniform>,
    pub blur_uniform_buffer: UniformBuffer<BlurUniform>,
    pub noise_texture: wgpu::Texture,
    pub noise_view: wgpu::TextureView,
    pub noise_sampler: wgpu::Sampler,
    pub linear_sampler: wgpu::Sampler,
    pub raw_target: wgpu::Texture,
    pub raw_view: wgpu::TextureView,
    pub blur_target: wgpu::Texture,
    pub blur_view: wgpu::TextureView,
    pub width: u32,
    pub height: u32,
    pub kernel_samples: [[f32; 4]; 32],
}

impl SsaoPipeline {
    pub fn new(device: &wgpu::Device, width: u32, height: u32) -> Self {
        let half_w = (width / 2).max(1);
        let half_h = (height / 2).max(1);

        // 1. Génération procédurale du kernel hémisphérique (32 échantillons avec distribution cubique)
        let mut kernel = [[0.0f32; 4]; 32];
        for (i, item) in kernel.iter_mut().enumerate() {
            // Distribution hémisphérique Z > 0
            let x = ((i as f32 * 17.13).sin() * 2.0 - 1.0).clamp(-1.0, 1.0);
            let y = ((i as f32 * 29.41).cos() * 2.0 - 1.0).clamp(-1.0, 1.0);
            let z = (i as f32 / 32.0).max(0.1); // Hémisphère supérieur

            let mut sample = glam::Vec3::new(x, y, z).normalize();
            // Facteur d'échelle exponentiel pour concentrer les échantillons près de la surface
            let mut scale = i as f32 / 32.0;
            scale = 0.1 + scale * scale * (1.0 - 0.1);
            sample *= scale;

            *item = [sample.x, sample.y, sample.z, 0.0];
        }

        // 2. Texture de bruit 4x4 (vecteurs aléatoires dans le plan tangent XY)
        let mut noise_data = [0u8; 4 * 4 * 4];
        for i in 0..16 {
            let angle = (i as f32 * 3.14159265 * 0.375).sin();
            let nx = (angle * 127.5 + 127.5) as u8;
            let ny = (((angle + 1.57).cos()) * 127.5 + 127.5) as u8;
            noise_data[i * 4 + 0] = nx;
            noise_data[i * 4 + 1] = ny;
            noise_data[i * 4 + 2] = 128; // z = 0
            noise_data[i * 4 + 3] = 255;
        }

        let noise_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("SSAO 4x4 Noise Texture"),
            size: wgpu::Extent3d {
                width: 4,
                height: 4,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });

        // Note: L'écriture de la texture sera effectuée par le device / queue initial ou par le créateur
        let noise_view = noise_texture.create_view(&wgpu::TextureViewDescriptor::default());

        let noise_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("SSAO Noise Repeat Sampler"),
            address_mode_u: wgpu::AddressMode::Repeat,
            address_mode_v: wgpu::AddressMode::Repeat,
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });

        let linear_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("SSAO Linear Clamp Sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });

        // 3. Cibles de rendu R8Unorm (Raw SSAO et Bilateral Blurred SSAO)
        let (raw_target, raw_view) = Self::create_target(device, half_w, half_h, "SSAO Raw Target");
        let (blur_target, blur_view) = Self::create_target(device, width, height, "SSAO Blurred Target");

        // 4. Uniform Buffers
        let ssao_uniform_data = SsaoUniform {
            proj: Mat4::IDENTITY.to_cols_array_2d(),
            inv_proj: Mat4::IDENTITY.to_cols_array_2d(),
            kernel,
            radius: 0.65,
            bias: 0.025,
            screen_width: half_w as f32,
            screen_height: half_h as f32,
        };
        let ssao_uniform_buffer = UniformBuffer::new(device, Some("SSAO Params Uniform"), &ssao_uniform_data);

        let blur_uniform_data = BlurUniform {
            texel_size: [1.0 / width.max(1) as f32, 1.0 / height.max(1) as f32],
            _pad0: 0.0,
            _pad1: 0.0,
        };
        let blur_uniform_buffer = UniformBuffer::new(device, Some("SSAO Blur Uniform"), &blur_uniform_data);

        // 5. Layouts des Bind Groups
        let ssao_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("SSAO Pass Bind Group Layout"),
            entries: &[
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
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Depth,
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::NonFiltering),
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 3,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 4,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });

        let blur_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("SSAO Blur Bind Group Layout"),
            entries: &[
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
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
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
                wgpu::BindGroupLayoutEntry {
                    binding: 4,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::NonFiltering),
                    count: None,
                },
            ],
        });

        // 6. Pipelines Graphiques
        let ssao_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("SSAO Shader Module"),
            source: wgpu::ShaderSource::Wgsl(include_str!("../shaders/ssao.wgsl").into()),
        });

        let ssao_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("SSAO Pipeline Layout"),
            bind_group_layouts: &[&ssao_bind_group_layout],
            push_constant_ranges: &[],
        });

        let ssao_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("SSAO Compute Render Pipeline"),
            layout: Some(&ssao_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &ssao_shader,
                entry_point: "vs_fullscreen",
                buffers: &[],
            },
            fragment: Some(wgpu::FragmentState {
                module: &ssao_shader,
                entry_point: "fs_ssao",
                targets: &[Some(wgpu::ColorTargetState {
                    format: wgpu::TextureFormat::R8Unorm,
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

        let blur_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("SSAO Blur Shader Module"),
            source: wgpu::ShaderSource::Wgsl(include_str!("../shaders/ssao_blur.wgsl").into()),
        });

        let blur_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("SSAO Blur Pipeline Layout"),
            bind_group_layouts: &[&blur_bind_group_layout],
            push_constant_ranges: &[],
        });

        let blur_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("SSAO Bilateral Blur Pipeline"),
            layout: Some(&blur_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &blur_shader,
                entry_point: "vs_fullscreen",
                buffers: &[],
            },
            fragment: Some(wgpu::FragmentState {
                module: &blur_shader,
                entry_point: "fs_blur",
                targets: &[Some(wgpu::ColorTargetState {
                    format: wgpu::TextureFormat::R8Unorm,
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
            ssao_pipeline,
            blur_pipeline,
            ssao_bind_group_layout,
            blur_bind_group_layout,
            ssao_uniform_buffer,
            blur_uniform_buffer,
            noise_texture,
            noise_view,
            noise_sampler,
            linear_sampler,
            raw_target,
            raw_view,
            blur_target,
            blur_view,
            width,
            height,
            kernel_samples: kernel,
        }
    }

    fn create_target(device: &wgpu::Device, w: u32, h: u32, label: &str) -> (wgpu::Texture, wgpu::TextureView) {
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some(label),
            size: wgpu::Extent3d {
                width: w.max(1),
                height: h.max(1),
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::R8Unorm,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        (texture, view)
    }

    pub fn resize(&mut self, device: &wgpu::Device, width: u32, height: u32) {
        if self.width == width && self.height == height {
            return;
        }
        self.width = width;
        self.height = height;
        let half_w = (width / 2).max(1);
        let half_h = (height / 2).max(1);

        let (raw_t, raw_v) = Self::create_target(device, half_w, half_h, "SSAO Raw Target");
        let (blur_t, blur_v) = Self::create_target(device, width, height, "SSAO Blurred Target");
        self.raw_target = raw_t;
        self.raw_view = raw_v;
        self.blur_target = blur_t;
        self.blur_view = blur_v;
    }

    pub fn render(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        queue: &wgpu::Queue,
        device: &wgpu::Device,
        depth_view: &wgpu::TextureView,
        depth_sampler: &wgpu::Sampler,
        proj: Mat4,
    ) {
        let half_w = (self.width / 2).max(1);
        let half_h = (self.height / 2).max(1);

        // Mettre à jour les uniformes
        let ssao_data = SsaoUniform {
            proj: proj.to_cols_array_2d(),
            inv_proj: proj.inverse().to_cols_array_2d(),
            kernel: self.kernel_samples,
            radius: 0.65,
            bias: 0.025,
            screen_width: half_w as f32,
            screen_height: half_h as f32,
        };
        self.ssao_uniform_buffer.update(queue, &ssao_data);

        let blur_data = BlurUniform {
            texel_size: [1.0 / half_w as f32, 1.0 / half_h as f32],
            _pad0: 0.0,
            _pad1: 0.0,
        };
        self.blur_uniform_buffer.update(queue, &blur_data);

        // Passe 1 : Calcul du SSAO Brut (Half-res)
        let ssao_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("SSAO Compute Bind Group"),
            layout: &self.ssao_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: self.ssao_uniform_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(depth_view),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::Sampler(depth_sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: wgpu::BindingResource::TextureView(&self.noise_view),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: wgpu::BindingResource::Sampler(&self.noise_sampler),
                },
            ],
        });

        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("SSAO Raw Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &self.raw_view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::WHITE),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            pass.set_pipeline(&self.ssao_pipeline);
            pass.set_bind_group(0, &ssao_bind_group, &[]);
            pass.draw(0..3, 0..1);
        }

        // Passe 2 : Flou Bilatéral Adaptatif (Full-res)
        let blur_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("SSAO Bilateral Blur Bind Group"),
            layout: &self.blur_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: self.blur_uniform_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&self.raw_view),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::Sampler(&self.linear_sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: wgpu::BindingResource::TextureView(depth_view),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: wgpu::BindingResource::Sampler(depth_sampler),
                },
            ],
        });

        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("SSAO Blur Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &self.blur_view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::WHITE),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            pass.set_pipeline(&self.blur_pipeline);
            pass.set_bind_group(0, &blur_bind_group, &[]);
            pass.draw(0..3, 0..1);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ssao_uniform_size_and_alignment() {
        assert_eq!(std::mem::size_of::<SsaoUniform>(), 64 + 64 + (32 * 16) + 4 + 4 + 4 + 4);
    }

    #[test]
    fn test_blur_uniform_size_and_alignment() {
        assert_eq!(std::mem::size_of::<BlurUniform>(), 16);
    }

    #[test]
    fn test_ssao_kernel_generation_bounds() {
        let mut kernel = [[0.0f32; 4]; 32];
        for (i, item) in kernel.iter_mut().enumerate() {
            let x = ((i as f32 * 17.13).sin() * 2.0 - 1.0).clamp(-1.0, 1.0);
            let y = ((i as f32 * 29.41).cos() * 2.0 - 1.0).clamp(-1.0, 1.0);
            let z = (i as f32 / 32.0).max(0.1);

            let mut sample = glam::Vec3::new(x, y, z).normalize();
            let mut scale = i as f32 / 32.0;
            scale = 0.1 + scale * scale * (1.0 - 0.1);
            sample *= scale;

            *item = [sample.x, sample.y, sample.z, 0.0];
        }

        for (i, s) in kernel.iter().enumerate() {
            let len = (s[0] * s[0] + s[1] * s[1] + s[2] * s[2]).sqrt();
            assert!(len > 0.0, "Sample {} must not be zero", i);
            assert!(len <= 1.0001, "Sample {} must be inside unit hemisphere", i);
            assert!(s[2] > 0.0, "Sample {} Z must be in upper hemisphere", i);
        }
    }
}
