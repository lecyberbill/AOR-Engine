// [WFGY] Zone: SAFE | λ: 0.3 | Fallbacks: 0/None | Action: Volumetric Fog, God Rays Raymarching & Mie Phase Function
#![allow(dead_code)]

use glam::Mat4;
use crate::gpu::UniformBuffer;

#[repr(C)]
#[derive(Clone, Copy, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct VolumetricUniform {
    pub inv_view_proj: [[f32; 4]; 4],
    pub light_dir: [f32; 4],      // xyz = dir to light, w = density
    pub light_color: [f32; 4],    // rgb = sun color, w = intensity
    pub fog_params: [f32; 4],     // x = mie g (anisotropy), y = max distance, z = step count, w = jitter strength
}

impl Default for VolumetricUniform {
    fn default() -> Self {
        Self {
            inv_view_proj: Mat4::IDENTITY.to_cols_array_2d(),
            light_dir: [0.4, 0.8, -0.45, 0.015], // densite brume standard
            light_color: [1.0, 0.95, 0.85, 1.2],
            fog_params: [0.65, 80.0, 32.0, 1.0], // Mie forward-scattering g=0.65, 80m max
        }
    }
}

pub struct VolumetricFogPipeline {
    pub pipeline: wgpu::RenderPipeline,
    pub bind_group_layout: wgpu::BindGroupLayout,
    pub uniform_buffer: UniformBuffer<VolumetricUniform>,
    pub linear_sampler: wgpu::Sampler,
    pub depth_sampler: wgpu::Sampler,
    pub target_texture: wgpu::Texture,
    pub target_view: wgpu::TextureView,
    pub width: u32,
    pub height: u32,
}

impl VolumetricFogPipeline {
    pub fn new(device: &wgpu::Device, width: u32, height: u32, target_format: wgpu::TextureFormat) -> Self {
        let half_w = (width / 2).max(1);
        let half_h = (height / 2).max(1);

        let uniform_data = VolumetricUniform::default();
        let uniform_buffer = UniformBuffer::new(device, Some("Volumetric Fog Uniforms"), &uniform_data);

        let linear_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("Volumetric Linear Sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });

        let depth_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("Volumetric Depth Sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });

        let target_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Volumetric Fog RT (Half-Res)"),
            size: wgpu::Extent3d {
                width: half_w,
                height: half_h,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: target_format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let target_view = target_texture.create_view(&wgpu::TextureViewDescriptor::default());

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Volumetric Fog Bind Group Layout"),
            entries: &[
                // binding 0: Uniforms
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
                // binding 1: Global Camera Uniform
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                // binding 2: Light Uniform (CSM Matrices)
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                // binding 3: Camera Depth Map
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
                // binding 4: Depth Sampler
                wgpu::BindGroupLayoutEntry {
                    binding: 4,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::NonFiltering),
                    count: None,
                },
                // binding 5: Shadow Depth Array (CSM)
                wgpu::BindGroupLayoutEntry {
                    binding: 5,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Depth,
                        view_dimension: wgpu::TextureViewDimension::D2Array,
                        multisampled: false,
                    },
                    count: None,
                },
                // binding 6: Shadow Comparison Sampler
                wgpu::BindGroupLayoutEntry {
                    binding: 6,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Comparison),
                    count: None,
                },
            ],
        });

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Volumetric Fog Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("../shaders/volumetric_fog.wgsl").into()),
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Volumetric Fog Pipeline Layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Volumetric Fog Render Pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "vs_fullscreen",
                buffers: &[],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: "fs_volumetric",
                targets: &[Some(wgpu::ColorTargetState {
                    format: target_format,
                    blend: Some(wgpu::BlendState {
                        color: wgpu::BlendComponent {
                            src_factor: wgpu::BlendFactor::One,
                            dst_factor: wgpu::BlendFactor::One,
                            operation: wgpu::BlendOperation::Add,
                        },
                        alpha: wgpu::BlendComponent::OVER,
                    }),
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
            target_texture,
            target_view,
            width: half_w,
            height: half_h,
        }
    }

    pub fn resize(&mut self, device: &wgpu::Device, width: u32, height: u32, target_format: wgpu::TextureFormat) {
        let half_w = (width / 2).max(1);
        let half_h = (height / 2).max(1);

        if self.width == half_w && self.height == half_h {
            return;
        }

        self.width = half_w;
        self.height = half_h;
        self.target_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Volumetric Fog RT (Resized Half-Res)"),
            size: wgpu::Extent3d {
                width: half_w,
                height: half_h,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: target_format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        self.target_view = self.target_texture.create_view(&wgpu::TextureViewDescriptor::default());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_volumetric_uniform_size_and_alignment() {
        assert_eq!(std::mem::size_of::<VolumetricUniform>() % 16, 0);
        assert_eq!(std::mem::size_of::<VolumetricUniform>(), 112); // 64 + 16 + 16 + 16 = 112
    }

    #[test]
    fn test_volumetric_fog_params_defaults() {
        let u = VolumetricUniform::default();
        assert!(u.fog_params[0] > 0.0 && u.fog_params[0] < 1.0); // Mie g factor
        assert!(u.fog_params[1] > 10.0); // Max raymarching distance
        assert!(u.fog_params[2] >= 16.0); // Sample steps
    }
}
