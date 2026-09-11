// [WFGY] Zone: SAFE | λ: 0.3 | Fallbacks: 0 | Action: GPU Path Tracer Engine with BVH, Monte Carlo and Storage Buffer Progressive Accumulation

use glam::Vec3;
use wgpu::util::DeviceExt;
use crate::scene::bvh::{BvhBuilder, GpuBvhNode, GpuMaterial, GpuTriangle};
use crate::scene::camera::Camera;
use crate::scene::node::SceneNode;

#[repr(C, align(16))]
#[derive(Clone, Copy, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct PathTraceUniform {
    pub camera_pos: [f32; 4],      // xyz: pos, w: frame_count as f32
    pub inv_view_proj: [f32; 16],  // 64 octets
    pub sun_dir: [f32; 4],         // xyz: dir, w: sky_intensity
    pub sun_color: [f32; 4],       // xyz: color, w: sun_intensity
    pub resolution: [u32; 2],      // width, height
    pub bounces: u32,
    pub samples_per_pass: u32,
}

#[repr(C, align(16))]
#[derive(Clone, Copy, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct BlitUniform {
    pub resolution: [u32; 2],
    pub enable_denoise: u32,
    pub denoise_radius: u32,
    pub exposure: f32,
    pub sharpness: f32,
    pub _pad0: f32,
    pub _pad1: f32,
}

#[derive(Clone, Copy, Debug)]
pub struct PathTracerConfig {
    pub max_bounces: u32,
    pub samples_per_pass: u32,
    pub sky_intensity: f32,
    pub sun_intensity: f32,
    pub sun_direction: Vec3,
    pub sun_color: Vec3,
    pub enable_denoise: bool,
    pub denoise_radius: u32,
    pub exposure: f32,
    pub sharpness: f32,      // Netteté et accentuation des arêtes (0.0 à 1.0)
    pub max_spp: u32,        // Limite maximale d'échantillons (mise en veille GPU à 0% dès convergence)
    pub unlimited_spp: bool, // Mode continu sans limite de SPP
    pub is_paused: bool,     // Pause manuelle du calcul
}

impl Default for PathTracerConfig {
    fn default() -> Self {
        Self {
            max_bounces: 4,
            samples_per_pass: 1,
            sky_intensity: 0.35,
            sun_intensity: 0.8,
            sun_direction: Vec3::new(0.5, 0.8, 0.4).normalize(),
            sun_color: Vec3::new(1.0, 0.95, 0.85),
            enable_denoise: true,
            denoise_radius: 1, // Rayon 1 par défaut pour une netteté maximale
            exposure: 0.85,
            sharpness: 0.35,
            max_spp: 256,
            unlimited_spp: false,
            is_paused: false,
        }
    }
}

pub struct PathTracerPipeline {
    compute_pipeline: wgpu::ComputePipeline,
    compute_bind_group_layout: wgpu::BindGroupLayout,
    compute_bind_group: Option<wgpu::BindGroup>,

    blit_pipeline: wgpu::RenderPipeline,
    blit_bind_group_layout: wgpu::BindGroupLayout,
    blit_bind_group: Option<wgpu::BindGroup>,

    uniform_buffer: wgpu::Buffer,
    blit_uniform_buffer: wgpu::Buffer,
    triangles_buffer: Option<wgpu::Buffer>,
    bvh_buffer: Option<wgpu::Buffer>,
    materials_buffer: Option<wgpu::Buffer>,
    accum_buffer: Option<wgpu::Buffer>,

    width: u32,
    height: u32,
    pub frame_count: u32,
    pub total_triangles: usize,
    pub total_bvh_nodes: usize,
    pub default_texture: std::sync::Arc<crate::gpu::GpuTexture>,
    pub current_texture: Option<std::sync::Arc<crate::gpu::GpuTexture>>,
}

impl PathTracerPipeline {
    pub fn new(device: &wgpu::Device, queue: &wgpu::Queue, target_format: wgpu::TextureFormat) -> Self {
        let compute_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("PathTrace Compute Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("../shaders/pathtrace.wgsl").into()),
        });

        let blit_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("PathTrace Blit Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("../shaders/pathtrace_blit.wgsl").into()),
        });

        // Layout Compute (Standard WebGPU Storage Buffers)
        let compute_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("PathTrace Compute Bind Group Layout"),
            entries: &[
                // 0: Uniform
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                // 1: Triangles (Storage Read)
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                // 2: BVH Nodes (Storage Read)
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                // 3: Materials (Storage Read)
                wgpu::BindGroupLayoutEntry {
                    binding: 3,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                // 4: Accumulation Buffer (Storage ReadWrite)
                wgpu::BindGroupLayoutEntry {
                    binding: 4,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: false },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                // 5: Albedo Texture
                wgpu::BindGroupLayoutEntry {
                    binding: 5,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                // 6: Texture Sampler
                wgpu::BindGroupLayoutEntry {
                    binding: 6,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });

        let compute_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("PathTrace Compute Pipeline Layout"),
            bind_group_layouts: &[&compute_bind_group_layout],
            push_constant_ranges: &[],
        });

        let compute_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("PathTrace Compute Pipeline"),
            layout: Some(&compute_pipeline_layout),
            module: &compute_shader,
            entry_point: "main",
        });

        // Layout Blit
        let blit_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("PathTrace Blit Bind Group Layout"),
            entries: &[
                // 0: Resolution Uniform
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
                // 1: Accumulation Buffer (Storage Read)
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });

        let blit_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("PathTrace Blit Pipeline Layout"),
            bind_group_layouts: &[&blit_bind_group_layout],
            push_constant_ranges: &[],
        });

        let blit_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("PathTrace Blit Render Pipeline"),
            layout: Some(&blit_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &blit_shader,
                entry_point: "vs_main",
                buffers: &[],
            },
            fragment: Some(wgpu::FragmentState {
                module: &blit_shader,
                entry_point: "fs_main",
                targets: &[Some(wgpu::ColorTargetState {
                    format: target_format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: None,
                unclipped_depth: false,
                polygon_mode: wgpu::PolygonMode::Fill,
                conservative: false,
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
        });

        let uniform_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("PathTrace Uniform Buffer"),
            size: std::mem::size_of::<PathTraceUniform>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let blit_uniform_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("PathTrace Blit Uniform Buffer"),
            size: std::mem::size_of::<BlitUniform>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let default_texture = std::sync::Arc::new(crate::gpu::GpuTexture::create_white_texture(device, queue));

        Self {
            compute_pipeline,
            compute_bind_group_layout,
            compute_bind_group: None,
            blit_pipeline,
            blit_bind_group_layout,
            blit_bind_group: None,
            uniform_buffer,
            blit_uniform_buffer,
            triangles_buffer: None,
            bvh_buffer: None,
            materials_buffer: None,
            accum_buffer: None,
            width: 0,
            height: 0,
            frame_count: 1,
            total_triangles: 0,
            total_bvh_nodes: 0,
            default_texture,
            current_texture: None,
        }
    }

    pub fn resize(&mut self, device: &wgpu::Device, width: u32, height: u32) {
        if width == 0 || height == 0 || (self.width == width && self.height == height) {
            return;
        }

        self.width = width;
        self.height = height;
        self.frame_count = 1;

        let buffer_size = (width as u64) * (height as u64) * 16; // 16 octets par pixel (vec4<f32>)
        let accum_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("PathTrace Accumulation Storage Buffer"),
            size: buffer_size.max(64),
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        self.blit_bind_group = Some(device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("PathTrace Blit Bind Group"),
            layout: &self.blit_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: self.blit_uniform_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: accum_buffer.as_entire_binding(),
                },
            ],
        }));

        self.accum_buffer = Some(accum_buffer);
        self.rebuild_compute_bind_group(device);
    }

    pub fn update_scene(&mut self, device: &wgpu::Device, nodes: &[SceneNode]) {
        let (triangles, bvh_nodes, materials) = BvhBuilder::build_from_scene_nodes(nodes);

        self.total_triangles = triangles.len();
        self.total_bvh_nodes = bvh_nodes.len();

        let active_tex = nodes
            .iter()
            .find(|n| n.texture_name.is_some())
            .map(|n| n.texture.clone())
            .or_else(|| nodes.first().map(|n| n.texture.clone()));
        self.current_texture = active_tex;

        let tri_slice: &[GpuTriangle] = if triangles.is_empty() {
            &[GpuTriangle {
                v0: [0.0; 3], material_id: 0, v1: [0.0; 3], _pad0: 0, v2: [0.0; 3], _pad1: 0,
                n0: [0.0; 3], _pad2: 0, n1: [0.0; 3], _pad3: 0, n2: [0.0; 3], _pad4: 0,
                uv0: [0.0; 2], uv1: [0.0; 2], uv2: [0.0; 2], _pad5: [0.0; 2],
            }]
        } else {
            &triangles
        };

        let bvh_slice: &[GpuBvhNode] = if bvh_nodes.is_empty() {
            &[GpuBvhNode {
                aabb_min: [-1.0; 3], left_or_first: 0, aabb_max: [1.0; 3], count: 0,
            }]
        } else {
            &bvh_nodes
        };

        let mat_slice: &[GpuMaterial] = if materials.is_empty() {
            &[GpuMaterial {
                albedo: [1.0; 4], emission: [0.0; 4], roughness: 0.5, metallic: 0.0, ior: 1.5, transmission: 0.0,
                use_texture: 0, _pad0: 0, _pad1: 0, _pad2: 0,
            }]
        } else {
            &materials
        };

        self.triangles_buffer = Some(device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("PathTrace Triangles Storage Buffer"),
            contents: bytemuck::cast_slice(tri_slice),
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        }));

        self.bvh_buffer = Some(device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("PathTrace BVH Storage Buffer"),
            contents: bytemuck::cast_slice(bvh_slice),
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        }));

        self.materials_buffer = Some(device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("PathTrace Materials Storage Buffer"),
            contents: bytemuck::cast_slice(mat_slice),
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        }));

        self.frame_count = 1;
        self.rebuild_compute_bind_group(device);
    }

    fn rebuild_compute_bind_group(&mut self, device: &wgpu::Device) {
        if let (Some(accum_buf), Some(tri_buf), Some(bvh_buf), Some(mat_buf)) = (
            &self.accum_buffer,
            &self.triangles_buffer,
            &self.bvh_buffer,
            &self.materials_buffer,
        ) {
            let active_tex = self.current_texture.as_ref().unwrap_or(&self.default_texture);

            self.compute_bind_group = Some(device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("PathTrace Compute Bind Group"),
                layout: &self.compute_bind_group_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: self.uniform_buffer.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: tri_buf.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: bvh_buf.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 3,
                        resource: mat_buf.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 4,
                        resource: accum_buf.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 5,
                        resource: wgpu::BindingResource::TextureView(&active_tex.view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 6,
                        resource: wgpu::BindingResource::Sampler(&active_tex.sampler),
                    },
                ],
            }));
        }
    }

    pub fn reset_accumulation(&mut self) {
        self.frame_count = 1;
    }

    pub fn is_converged(&self, config: &PathTracerConfig) -> bool {
        !config.unlimited_spp && self.frame_count >= config.max_spp
    }

    pub fn render(
        &mut self,
        _device: &wgpu::Device,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        target_view: &wgpu::TextureView,
        camera: &Camera,
        config: &PathTracerConfig,
    ) {
        if self.width == 0 || self.height == 0 || self.compute_bind_group.is_none() {
            return;
        }

        let is_converged = self.is_converged(config);
        let should_compute = !is_converged && !config.is_paused;

        if should_compute {
            let aspect_ratio = (self.width as f32) / (self.height as f32).max(1.0);
            let (view_proj, _, _) = camera.build_view_proj_matrix(aspect_ratio);
            let inv_view_proj = view_proj.inverse();
            let cam_pos = camera.position();

            let uniform = PathTraceUniform {
                camera_pos: [cam_pos.x, cam_pos.y, cam_pos.z, self.frame_count as f32],
                inv_view_proj: inv_view_proj.to_cols_array(),
                sun_dir: [config.sun_direction.x, config.sun_direction.y, config.sun_direction.z, config.sky_intensity],
                sun_color: [config.sun_color.x, config.sun_color.y, config.sun_color.z, config.sun_intensity],
                resolution: [self.width, self.height],
                bounces: config.max_bounces,
                samples_per_pass: config.samples_per_pass,
            };

            queue.write_buffer(&self.uniform_buffer, 0, bytemuck::cast_slice(&[uniform]));
        }

        let blit_uniform = BlitUniform {
            resolution: [self.width, self.height],
            enable_denoise: if config.enable_denoise { 1 } else { 0 },
            denoise_radius: config.denoise_radius,
            exposure: config.exposure,
            sharpness: config.sharpness,
            _pad0: 0.0,
            _pad1: 0.0,
        };
        queue.write_buffer(&self.blit_uniform_buffer, 0, bytemuck::cast_slice(&[blit_uniform]));

        // 1. Passe Compute (Lancer de Rayons) - Exécutée uniquement si non convergé et non en pause
        if should_compute {
            let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("PathTrace Compute Pass"),
                timestamp_writes: None,
            });
            compute_pass.set_pipeline(&self.compute_pipeline);
            if let Some(ref bg) = self.compute_bind_group {
                compute_pass.set_bind_group(0, bg, &[]);
            }
            let workgroups_x = (self.width + 7) / 8;
            let workgroups_y = (self.height + 7) / 8;
            compute_pass.dispatch_workgroups(workgroups_x, workgroups_y, 1);

            self.frame_count += 1;
        }

        // 2. Passe Blit (Affichage écran avec ACES Filmic & Débruitage) - Toujours active pour le viewport
        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("PathTrace Blit Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: target_view,
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
            render_pass.set_pipeline(&self.blit_pipeline);
            if let Some(ref bg) = self.blit_bind_group {
                render_pass.set_bind_group(0, bg, &[]);
            }
            render_pass.draw(0..3, 0..1);
        }
    }
}
