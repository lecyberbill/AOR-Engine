// [WFGY] Zone: SAFE | λ: 0.4 | Fallbacks: 0/None | Action: GPU Forward Renderer with Planar Water Reflection, Point Lights & HDR Ambient
use crate::gpu::{GpuContext, RenderPipelineBuilder, RenderTargetTexture, UniformBuffer};
use crate::scene::{Camera, CameraUniform, GpuMesh, Scene, Vertex, generate_grid_plane};
use glam::Vec3;

pub mod csm;
pub mod pathtracer;
pub mod postprocess;
pub mod ssao;
pub mod ssr;
pub mod taa;
pub mod volumetric;

pub use csm::{CsmManager, CsmUniform, NUM_CASCADES, SHADOW_MAP_SIZE};
pub use pathtracer::{PathTracerConfig, PathTracerPipeline};
pub use postprocess::PostProcessPipeline;
pub use ssao::SsaoPipeline;
pub use ssr::{SsrPipeline, SsrUniform};
pub use taa::{TaaPipeline, TaaUniform, halton_sequence};
pub use volumetric::{VolumetricFogPipeline, VolumetricUniform};


#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RenderMode {
    /// 1. Rendu Standard (Rasterisation temps réel légère, édition rapide)
    Rasterize,
    /// 2. Rendu Jeu Vidéo (Reflets miroir temps réel, vagues dynamiques, multi-lumières, brouillard, ACES)
    VideoGame,
    /// 3. Rendu Studio (Path Tracer physique GPU, illumination globale, accumulation)
    PathTrace,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct PointLightGpu {
    pub position: [f32; 4], // xyz = pos, w = radius
    pub color: [f32; 4],    // rgb = color, w = intensity
}

impl Default for PointLightGpu {
    fn default() -> Self {
        PointLightGpu {
            position: [0.0, 0.0, 0.0, 0.0],
            color: [0.0, 0.0, 0.0, 0.0],
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct LightUniform {
    pub position: [f32; 4],              // Sun position, w = sun intensity
    pub color: [f32; 4],                 // Sun color
    pub ambient: [f32; 4],               // Ambient night glow
    pub fog_color: [f32; 4],             // Fog color rgb, w = fog density
    pub cascade_matrices: [[[f32; 4]; 4]; NUM_CASCADES], // 4 Cascades d'ombres directionnelles
    pub cascade_splits: [f32; 4],        // Distances de transition en espace vue
    pub point_light_count: u32,
    pub viewport_width: f32,
    pub viewport_height: f32,
    pub render_flags: u32,               // 0 = Standard Preview, 1 = Video Game Next-Gen
    pub point_lights: [PointLightGpu; 16],
}

impl Default for LightUniform {
    fn default() -> Self {
        LightUniform {
            position: [6.0, 8.0, -8.0, 1.2],
            color: [1.0, 0.95, 0.85, 1.0],
            ambient: [0.08, 0.06, 0.14, 1.0],
            fog_color: [0.02, 0.015, 0.04, 0.022],
            cascade_matrices: [glam::Mat4::IDENTITY.to_cols_array_2d(); NUM_CASCADES],
            cascade_splits: [10.0, 28.0, 70.0, 160.0],
            point_light_count: 0,
            viewport_width: 960.0,
            viewport_height: 640.0,
            render_flags: 1,
            point_lights: [PointLightGpu::default(); 16],
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct ShadowUniform {
    pub light_space_matrix: [[f32; 4]; 4],
}

#[allow(dead_code)]
pub struct ForwardRenderer {
    pub mesh_pipeline: wgpu::RenderPipeline,
    pub instanced_pipeline: wgpu::RenderPipeline,
    pub depth_prepass_pipeline: wgpu::RenderPipeline,
    pub instanced_depth_prepass_pipeline: wgpu::RenderPipeline,
    pub grid_pipeline: wgpu::RenderPipeline,
    pub shadow_pipeline: wgpu::RenderPipeline,
    pub instanced_shadow_pipeline: wgpu::RenderPipeline,
    pub shadow_bind_group_layout: wgpu::BindGroupLayout,
    pub shadow_bind_groups: [wgpu::BindGroup; NUM_CASCADES],
    pub shadow_uniforms: [UniformBuffer<ShadowUniform>; NUM_CASCADES],
    pub shadow_target: crate::gpu::DepthTexture,
    pub shadow_layer_views: Vec<wgpu::TextureView>,
    pub shadow_sampler: wgpu::Sampler,
    pub csm_manager: CsmManager,
    pub camera_bind_group_layout: wgpu::BindGroupLayout,
    pub camera_bind_group: wgpu::BindGroup,
    pub global_bind_group_layout: wgpu::BindGroupLayout,
    pub global_bind_group: wgpu::BindGroup,
    pub reflection_bind_group: wgpu::BindGroup,
    pub camera_uniform: UniformBuffer<CameraUniform>,
    pub reflection_camera_uniform: UniformBuffer<CameraUniform>,
    pub light_uniform: UniformBuffer<LightUniform>,
    pub reflection_target: RenderTargetTexture,
    pub grid_model_uniform: UniformBuffer<crate::scene::node::ModelUniform>,
    pub grid_bind_group: wgpu::BindGroup,
    pub grid_mesh: GpuMesh,
    pub light_pos: Vec3,
    pub ssao: SsaoPipeline,
    pub ssr: SsrPipeline,
    pub skybox: std::sync::Arc<crate::gpu::GpuCubemap>,
    pub skybox_pipeline: wgpu::RenderPipeline,
    pub skybox_bind_group: wgpu::BindGroup,
    pub volumetric_fog: VolumetricFogPipeline,
    pub taa: TaaPipeline,
    /// Pipeline matériau compilé à chaud depuis le graphe nodal (si présent, remplace mesh_pipeline)
    pub material_pipeline: Option<wgpu::RenderPipeline>,
}

impl ForwardRenderer {
    pub fn new(context: &GpuContext, target_format: wgpu::TextureFormat, model_bind_group_layout: &wgpu::BindGroupLayout) -> Self {
        let device = &context.device;

        // 1. Uniforms globaux (Caméra Principale + Caméra Miroir + Lumières)
        let camera_uniform = UniformBuffer::new(
            device,
            Some("Global Camera Uniform"),
            &CameraUniform::default(),
        );

        let reflection_camera_uniform = UniformBuffer::new(
            device,
            Some("Planar Reflection Camera Uniform"),
            &CameraUniform::default(),
        );

        let initial_light = LightUniform::default();
        let light_uniform = UniformBuffer::new(
            device,
            Some("Global Light Uniform"),
            &initial_light,
        );

        // Texture cible pour la réflexion planaire dans l'eau (Miroir GPU temps réel)
        let reflection_target = RenderTargetTexture::new(
            device,
            1024,
            1024,
            target_format,
            Some("Water Planar Reflection RT"),
        );

        let reflection_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("Planar Reflection Sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });

        // 1b. Texture & Sampler d'Ombre Portée Multi-Cascades CSM (4 Couches 2048x2048)
        let (shadow_target, shadow_layer_views) = crate::gpu::DepthTexture::create_depth_texture_array(
            device,
            SHADOW_MAP_SIZE,
            SHADOW_MAP_SIZE,
            NUM_CASCADES as u32,
            "Directional CSM Depth Texture Array",
        );

        let shadow_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("Shadow Comparison Sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Nearest,
            compare: Some(wgpu::CompareFunction::LessEqual),
            ..Default::default()
        });

        let shadow_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Shadow Bind Group Layout (Light Space Matrix)"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });

        let shadow_uniforms: [UniformBuffer<ShadowUniform>; NUM_CASCADES] = std::array::from_fn(|i| {
            UniformBuffer::new(
                device,
                Some(&format!("Shadow Cascade {i} Uniform")),
                &ShadowUniform {
                    light_space_matrix: glam::Mat4::IDENTITY.to_cols_array_2d(),
                },
            )
        });

        let shadow_bind_groups: [wgpu::BindGroup; NUM_CASCADES] = std::array::from_fn(|i| {
            device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some(&format!("Shadow Cascade {i} Bind Group")),
                layout: &shadow_bind_group_layout,
                entries: &[wgpu::BindGroupEntry {
                    binding: 0,
                    resource: shadow_uniforms[i].as_entire_binding(),
                }],
            })
        });

        let csm_manager = CsmManager::new();

        // Pipeline Graphique Shadow Pass (Écriture de profondeur uniquement)
        let shadow_shader_source = include_str!("../shaders/shadow.wgsl");
        let shadow_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Shadow Shader Module"),
            source: wgpu::ShaderSource::Wgsl(shadow_shader_source.into()),
        });

        let shadow_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Shadow Pipeline Layout"),
            bind_group_layouts: &[&shadow_bind_group_layout, model_bind_group_layout],
            push_constant_ranges: &[],
        });

        let shadow_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Mesh Shadow Render Pipeline"),
            layout: Some(&shadow_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shadow_shader,
                entry_point: "vs_main",
                buffers: &[Vertex::desc()],
            },
            fragment: None, // Depth only pass!
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: Some(wgpu::Face::Back),
                polygon_mode: wgpu::PolygonMode::Fill,
                unclipped_depth: false,
                conservative: false,
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: wgpu::TextureFormat::Depth32Float,
                depth_write_enabled: true,
                depth_compare: wgpu::CompareFunction::LessEqual,
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState {
                    constant: 2, // Slope scale & constant depth bias
                    slope_scale: 2.0,
                    clamp: 0.0,
                },
            }),
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
        });

        let instanced_shadow_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Instanced Mesh Shadow Render Pipeline"),
            layout: Some(&shadow_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shadow_shader,
                entry_point: "vs_instanced",
                buffers: &[Vertex::desc(), crate::scene::instance::InstanceRaw::desc()],
            },
            fragment: None,
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: Some(wgpu::Face::Back),
                polygon_mode: wgpu::PolygonMode::Fill,
                unclipped_depth: false,
                conservative: false,
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: wgpu::TextureFormat::Depth32Float,
                depth_write_enabled: true,
                depth_compare: wgpu::CompareFunction::LessEqual,
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState {
                    constant: 2,
                    slope_scale: 2.0,
                    clamp: 0.0,
                },
            }),
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
        });

        // Pipeline Graphique Depth Pre-Pass (Remplissage rapide du buffer de profondeur de la caméra pour SSAO & Early-Z)
        let depth_prepass_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Depth Prepass Shader Module"),
            source: wgpu::ShaderSource::Wgsl(include_str!("../shaders/depth_prepass.wgsl").into()),
        });

        let camera_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Camera Only Bind Group Layout"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });

        let camera_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Camera Only Bind Group"),
            layout: &camera_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: camera_uniform.as_entire_binding(),
            }],
        });

        let depth_prepass_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Depth Prepass Pipeline Layout"),
            bind_group_layouts: &[&camera_bind_group_layout, model_bind_group_layout],
            push_constant_ranges: &[],
        });

        let depth_prepass_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Depth Prepass Render Pipeline"),
            layout: Some(&depth_prepass_layout),
            vertex: wgpu::VertexState {
                module: &depth_prepass_shader,
                entry_point: "vs_main",
                buffers: &[Vertex::desc()],
            },
            fragment: None,
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: Some(wgpu::Face::Back),
                ..Default::default()
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: wgpu::TextureFormat::Depth32Float,
                depth_write_enabled: true,
                depth_compare: wgpu::CompareFunction::LessEqual,
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            }),
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
        });

        let instanced_depth_prepass_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Instanced Depth Prepass Render Pipeline"),
            layout: Some(&depth_prepass_layout),
            vertex: wgpu::VertexState {
                module: &depth_prepass_shader,
                entry_point: "vs_instanced",
                buffers: &[Vertex::desc(), crate::scene::instance::InstanceRaw::desc()],
            },
            fragment: None,
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: Some(wgpu::Face::Back),
                ..Default::default()
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: wgpu::TextureFormat::Depth32Float,
                depth_write_enabled: true,
                depth_compare: wgpu::CompareFunction::LessEqual,
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            }),
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
        });

        // Initialisation du Pipeline SSAO (Screen-Space Ambient Occlusion)
        let ssao = SsaoPipeline::new(device, 960, 640);

        // Initialisation de la Cubemap d'Environnement Atmosphérique HDR Procédurale (IBL)
        let skybox = crate::gpu::GpuCubemap::create_procedural_skybox(device, &context.queue, 512);

        let global_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Global Bind Group Layout (Camera, Light, Reflection, Shadow, SSAO, Skybox IBL)"),
            entries: &[
                // binding 0: Camera Uniform
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                // binding 1: Light Uniform (Multi-Lights & Fog & Light Space Matrix)
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                // binding 2: Planar Reflection Texture
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
                // binding 3: Planar Reflection Sampler
                wgpu::BindGroupLayoutEntry {
                    binding: 3,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
                // binding 4: Directional Shadow Map Texture Array (CSM Depth)
                wgpu::BindGroupLayoutEntry {
                    binding: 4,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Depth,
                        view_dimension: wgpu::TextureViewDimension::D2Array,
                        multisampled: false,
                    },
                    count: None,
                },
                // binding 5: Shadow Map Comparison Sampler (PCF)
                wgpu::BindGroupLayoutEntry {
                    binding: 5,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Comparison),
                    count: None,
                },
                // binding 6: SSAO Blurred Texture
                wgpu::BindGroupLayoutEntry {
                    binding: 6,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                // binding 7: SSAO Linear Sampler
                wgpu::BindGroupLayoutEntry {
                    binding: 7,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
                // binding 8: Skybox Cubemap Texture (IBL)
                wgpu::BindGroupLayoutEntry {
                    binding: 8,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::Cube,
                        multisampled: false,
                    },
                    count: None,
                },
                // binding 9: Skybox Cubemap Sampler (IBL)
                wgpu::BindGroupLayoutEntry {
                    binding: 9,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });

        let global_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Global Bind Group (Main Pass)"),
            layout: &global_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: camera_uniform.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: light_uniform.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::TextureView(&reflection_target.color_view),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: wgpu::BindingResource::Sampler(&reflection_sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: wgpu::BindingResource::TextureView(&shadow_target.view),
                },
                wgpu::BindGroupEntry {
                    binding: 5,
                    resource: wgpu::BindingResource::Sampler(&shadow_sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 6,
                    resource: wgpu::BindingResource::TextureView(&ssao.blur_view),
                },
                wgpu::BindGroupEntry {
                    binding: 7,
                    resource: wgpu::BindingResource::Sampler(&ssao.linear_sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 8,
                    resource: wgpu::BindingResource::TextureView(&skybox.view),
                },
                wgpu::BindGroupEntry {
                    binding: 9,
                    resource: wgpu::BindingResource::Sampler(&skybox.sampler),
                },
            ],
        });

        let dummy_reflection_tex = crate::gpu::GpuTexture::create_white_texture(device, &context.queue);

        let reflection_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Reflection Bind Group (Mirror Pass)"),
            layout: &global_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: reflection_camera_uniform.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: light_uniform.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::TextureView(&dummy_reflection_tex.view),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: wgpu::BindingResource::Sampler(&reflection_sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: wgpu::BindingResource::TextureView(&shadow_target.view),
                },
                wgpu::BindGroupEntry {
                    binding: 5,
                    resource: wgpu::BindingResource::Sampler(&shadow_sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 6,
                    resource: wgpu::BindingResource::TextureView(&ssao.blur_view),
                },
                wgpu::BindGroupEntry {
                    binding: 7,
                    resource: wgpu::BindingResource::Sampler(&ssao.linear_sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 8,
                    resource: wgpu::BindingResource::TextureView(&skybox.view),
                },
                wgpu::BindGroupEntry {
                    binding: 9,
                    resource: wgpu::BindingResource::Sampler(&skybox.sampler),
                },
            ],
        });

        // Pipeline Graphique pour la Skybox Cubemap
        let skybox_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Skybox Shader Module"),
            source: wgpu::ShaderSource::Wgsl(include_str!("../shaders/skybox.wgsl").into()),
        });

        let skybox_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Skybox Bind Group Layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX,
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
                        view_dimension: wgpu::TextureViewDimension::Cube,
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
            ],
        });

        let skybox_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Skybox Bind Group"),
            layout: &skybox_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: camera_uniform.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&skybox.view),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::Sampler(&skybox.sampler),
                },
            ],
        });

        let skybox_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Skybox Pipeline Layout"),
            bind_group_layouts: &[&skybox_bind_group_layout],
            push_constant_ranges: &[],
        });

        let skybox_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Skybox Render Pipeline"),
            layout: Some(&skybox_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &skybox_shader,
                entry_point: "vs_skybox",
                buffers: &[],
            },
            fragment: Some(wgpu::FragmentState {
                module: &skybox_shader,
                entry_point: "fs_skybox",
                targets: &[Some(wgpu::ColorTargetState {
                    format: target_format,
                    blend: None,
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                cull_mode: None,
                ..Default::default()
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: wgpu::TextureFormat::Depth32Float,
                depth_write_enabled: false,
                depth_compare: wgpu::CompareFunction::LessEqual,
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            }),
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
        });

        // 2. Pipeline Graphique pour les Maillages 3D (Shaders WGSL)
        let shader_source = include_str!("../shaders/standard.wgsl");
        let (mesh_pipeline, _) = RenderPipelineBuilder::new(device, shader_source, target_format)
            .with_label("Mesh Forward Pipeline")
            .with_bind_group_layout(&global_bind_group_layout)
            .with_bind_group_layout(model_bind_group_layout)
            .with_vertex_layout(Vertex::desc())
            .with_topology(wgpu::PrimitiveTopology::TriangleList)
            .build();

        // 2b. Pipeline Graphique pour les Instances Multiples (GPU Hardware Instancing)
        let (instanced_pipeline, _) = RenderPipelineBuilder::new(device, shader_source, target_format)
            .with_label("Instanced Mesh Forward Pipeline")
            .with_bind_group_layout(&global_bind_group_layout)
            .with_bind_group_layout(model_bind_group_layout)
            .with_vertex_layout(Vertex::desc())
            .with_vertex_layout(crate::scene::instance::InstanceRaw::desc())
            .with_vertex_entry_point("vs_instanced")
            .with_topology(wgpu::PrimitiveTopology::TriangleList)
            .build();

        // 3. Pipeline Graphique pour la Grille au sol (Lines)
        let (grid_pipeline, _) = RenderPipelineBuilder::new(device, shader_source, target_format)
            .with_label("Grid Line Pipeline")
            .with_bind_group_layout(&global_bind_group_layout)
            .with_bind_group_layout(model_bind_group_layout)
            .with_vertex_layout(Vertex::desc())
            .with_topology(wgpu::PrimitiveTopology::LineList)
            .build();

        // 4. Uniform et BindGroup dédiés à la Grille
        let grid_model_uniform = UniformBuffer::new(
            device,
            Some("Grid Identity Model Uniform"),
            &crate::scene::node::ModelUniform::default(),
        );

        let grid_material_uniform = UniformBuffer::new(
            device,
            Some("Grid Material Uniform"),
            &crate::scene::node::MaterialUniform::default(),
        );

        let grid_texture = crate::gpu::GpuTexture::create_white_texture(device, &context.queue);
        let grid_normal_texture = crate::gpu::GpuTexture::create_flat_normal_map(device, &context.queue);

        let grid_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Grid Model & Material Bind Group"),
            layout: model_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: grid_model_uniform.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: grid_material_uniform.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::TextureView(&grid_texture.view),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: wgpu::BindingResource::TextureView(&grid_normal_texture.view),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: wgpu::BindingResource::Sampler(&grid_texture.sampler),
                },
            ],
        });

        // 5. Mesh de la grille au sol
        let (grid_verts, grid_inds) = generate_grid_plane(20.0, 20);
        let grid_mesh = GpuMesh::new(device, Some("Ground Grid"), &grid_verts, &grid_inds);

        let volumetric_fog = VolumetricFogPipeline::new(device, 960, 640, target_format);
        let ssr = SsrPipeline::new(device, 960, 640, target_format);
        let taa = TaaPipeline::new(device, 960, 640, target_format);

        ForwardRenderer {
            mesh_pipeline,
            instanced_pipeline,
            depth_prepass_pipeline,
            instanced_depth_prepass_pipeline,
            grid_pipeline,
            shadow_pipeline,
            instanced_shadow_pipeline,
            shadow_bind_group_layout,
            shadow_bind_groups,
            shadow_uniforms,
            shadow_target,
            shadow_layer_views,
            shadow_sampler,
            csm_manager,
            camera_bind_group_layout,
            camera_bind_group,
            global_bind_group_layout,
            global_bind_group,
            reflection_bind_group,
            camera_uniform,
            reflection_camera_uniform,
            light_uniform,
            reflection_target,
            grid_model_uniform,
            grid_bind_group,
            grid_mesh,
            light_pos: Vec3::new(6.0, 8.0, -8.0),
            ssao,
            ssr,
            skybox,
            skybox_pipeline,
            skybox_bind_group,
            volumetric_fog,
            taa,
            material_pipeline: None,
        }
    }

    /// Compile à chaud un shader WGSL (issu du graphe nodal) en pipeline WGPU complet,
    /// compatible avec les bind groups et le layout de vertex du moteur.
    pub fn build_material_pipeline(
        &mut self,
        device: &wgpu::Device,
        shader_wgsl: &str,
        model_bind_group_layout: &wgpu::BindGroupLayout,
        target_format: wgpu::TextureFormat,
    ) -> Result<(), String> {
        let (pipeline, _layout) = RenderPipelineBuilder::new(device, shader_wgsl, target_format)
            .with_label("Material Graph Hot Pipeline")
            .with_bind_group_layout(&self.global_bind_group_layout)
            .with_bind_group_layout(model_bind_group_layout)
            .with_vertex_layout(Vertex::desc())
            .with_topology(wgpu::PrimitiveTopology::TriangleList)
            .build();
        self.material_pipeline = Some(pipeline);
        Ok(())
    }

    /// Retire le pipeline matériau compilé (retour au PBR standard)
    pub fn clear_material_pipeline(&mut self) {
        self.material_pipeline = None;
    }

    pub fn has_material_pipeline(&self) -> bool {
        self.material_pipeline.is_some()
    }

    pub fn render(
        &mut self,
        context: &GpuContext,
        target_view: &wgpu::TextureView,
        depth_view: &wgpu::TextureView,
        scene: &Scene,
        camera: &Camera,
        aspect_ratio: f32,
        clear_color: wgpu::Color,
        time: f32,
        viewport_w: f32,
        viewport_h: f32,
        mode: RenderMode,
        particle_system: Option<&crate::particles::ParticleSystem>,
    ) {
        let is_video_game_mode = mode == RenderMode::VideoGame;

        // 1. Collecter les lumières ponctuelles de la scène (Lanternes, Cristaux)
        let mut point_lights = [PointLightGpu::default(); 16];
        let mut pt_idx = 0;
        if is_video_game_mode {
            for node in &scene.nodes {
                if node.material.emission_color[3] > 0.0 && pt_idx < 16 {
                    let pos = node.transform.translation;
                    let em = node.material.emission_color;
                    point_lights[pt_idx] = PointLightGpu {
                        position: [pos.x, pos.y, pos.z, 14.0], // Rayon d'action 14 mètres
                        color: [em[0], em[1], em[2], (em[3] * 0.9).max(1.2)],
                    };
                    pt_idx += 1;
                }
            }
        }

        // 2. Mettre à jour les Uniforms de caméra et lumières
        let cam_data = camera.to_uniform_with_time(aspect_ratio, time);
        self.camera_uniform.update(&context.queue, &cam_data);

        // Calcul des 4 matrices de projection d'ombres stabilisées sub-texel (CSM)
        let sun_pos = self.light_pos;
        let sun_dir = sun_pos.normalize();
        let (cascade_matrices, cascade_splits) = self.csm_manager.compute_cascade_matrices(
            camera,
            aspect_ratio,
            sun_dir,
        );

        let mut cascade_matrices_raw = [glam::Mat4::IDENTITY.to_cols_array_2d(); NUM_CASCADES];
        for i in 0..NUM_CASCADES {
            cascade_matrices_raw[i] = cascade_matrices[i].to_cols_array_2d();
            let shadow_data = ShadowUniform {
                light_space_matrix: cascade_matrices_raw[i],
            };
            self.shadow_uniforms[i].update(&context.queue, &shadow_data);
        }

        if is_video_game_mode {
            // Caméra Miroir pour la Réflexion Planaire Réelle dans l'Eau (Y = 0)
            let (_main_view_proj, main_view, proj) = camera.build_view_proj_matrix(aspect_ratio);
            let cam_pos = camera.position();
            let refl_scale = glam::Mat4::from_scale(Vec3::new(1.0, -1.0, 1.0));
            let mirror_view = main_view * refl_scale;
            let mirror_view_proj = proj * mirror_view;
            let mirror_pos = Vec3::new(cam_pos.x, -cam_pos.y, cam_pos.z);
            let mirror_cam_uniform = CameraUniform {
                view_proj: mirror_view_proj.to_cols_array_2d(),
                view: mirror_view.to_cols_array_2d(),
                proj: proj.to_cols_array_2d(),
                camera_pos: [mirror_pos.x, mirror_pos.y, mirror_pos.z, time],
            };
            self.reflection_camera_uniform.update(&context.queue, &mirror_cam_uniform);
        }

        let light_data = LightUniform {
            position: [self.light_pos.x, self.light_pos.y, self.light_pos.z, 1.2],
            color: [1.0, 0.95, 0.88, 1.0],
            ambient: if is_video_game_mode { [0.07, 0.05, 0.13, 1.0] } else { [0.25, 0.25, 0.28, 1.0] },
            fog_color: [0.02, 0.015, 0.04, 0.022],
            cascade_matrices: cascade_matrices_raw,
            cascade_splits,
            point_light_count: pt_idx as u32,
            viewport_width: viewport_w.max(1.0),
            viewport_height: viewport_h.max(1.0),
            render_flags: if is_video_game_mode { 1 } else { 0 },
            point_lights,
        };
        self.light_uniform.update(&context.queue, &light_data);

        // 3. Synchroniser les objets de la scène avec le GPU
        scene.update_gpu(&context.queue);

        let mut encoder = context.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Forward 3D Renderer Encoder"),
        });

        // =========================================================================
        // PASSE 0 : Rendu des 4 Cascades d'Ombres Directionnelles (CSM Multi-Layer Depth Passes)
        // =========================================================================
        for cascade_idx in 0..NUM_CASCADES {
            let mut shadow_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some(&format!("Directional Shadow Cascade {cascade_idx} Pass")),
                color_attachments: &[],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.shadow_layer_views[cascade_idx],
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            shadow_pass.set_pipeline(&self.shadow_pipeline);
            shadow_pass.set_bind_group(0, &self.shadow_bind_groups[cascade_idx], &[]);

            for node in &scene.nodes {
                // Le plan d'eau ne projette pas d'ombre
                if node.material.transmission > 0.5 {
                    continue;
                }

                shadow_pass.set_bind_group(1, &node.bind_group, &[]);
                shadow_pass.set_vertex_buffer(0, node.gpu_mesh.vertex_buffer.raw.slice(..));
                shadow_pass.set_index_buffer(node.gpu_mesh.index_buffer.raw.slice(..), wgpu::IndexFormat::Uint32);

                if let Some(ref inst_buf) = node.gpu_instances {
                    if inst_buf.count > 0 {
                        shadow_pass.set_pipeline(&self.instanced_shadow_pipeline);
                        shadow_pass.set_vertex_buffer(1, inst_buf.buffer.raw.slice(..));
                        shadow_pass.draw_indexed(0..node.gpu_mesh.index_count, 0, 0..inst_buf.count);
                        shadow_pass.set_pipeline(&self.shadow_pipeline);
                        continue;
                    }
                }

                shadow_pass.draw_indexed(0..node.gpu_mesh.index_count, 0, 0..1);
            }
        }

        // =========================================================================
        // PASSE 1 : Rendu de la Réflexion Planaire dans la Texture Miroir GPU (Mode Jeu Vidéo)
        // =========================================================================
        if is_video_game_mode {
            let mut refl_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Planar Water Reflection Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &self.reflection_target.color_view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color { r: 0.03, g: 0.02, b: 0.05, a: 1.0 }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.reflection_target.depth_texture.view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            refl_pass.set_pipeline(&self.mesh_pipeline);
            refl_pass.set_bind_group(0, &self.reflection_bind_group, &[]);

            for node in &scene.nodes {
                // On exclut le plan d'eau lui-même de sa propre réflexion pour éviter l'artefact miroir infini
                if node.material.transmission > 0.5 {
                    continue;
                }

                refl_pass.set_bind_group(1, &node.bind_group, &[]);
                refl_pass.set_vertex_buffer(0, node.gpu_mesh.vertex_buffer.raw.slice(..));
                refl_pass.set_index_buffer(node.gpu_mesh.index_buffer.raw.slice(..), wgpu::IndexFormat::Uint32);

                if let Some(ref inst_buf) = node.gpu_instances {
                    if inst_buf.count > 0 {
                        refl_pass.set_pipeline(&self.instanced_pipeline);
                        refl_pass.set_vertex_buffer(1, inst_buf.buffer.raw.slice(..));
                        refl_pass.draw_indexed(0..node.gpu_mesh.index_count, 0, 0..inst_buf.count);
                        refl_pass.set_pipeline(&self.mesh_pipeline);
                        continue;
                    }
                }

                refl_pass.draw_indexed(0..node.gpu_mesh.index_count, 0, 0..1);
            }
        }

        // =========================================================================
        // PASSE 2 : Depth Pre-Pass de la Caméra & Calcul SSAO
        // =========================================================================
        self.ssao.resize(&context.device, viewport_w.round() as u32, viewport_h.round() as u32);
        let (_view_proj_mat, _, proj_mat) = camera.build_view_proj_matrix(aspect_ratio);
        let depth_sampler = context.device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("Depth NonFiltering Sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });

        {
            let mut prepass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Camera Depth Prepass"),
                color_attachments: &[],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: depth_view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            prepass.set_pipeline(&self.depth_prepass_pipeline);
            prepass.set_bind_group(0, &self.camera_bind_group, &[]);

            for node in &scene.nodes {
                // Pas de profondeur pour les objets très transparents
                if node.material.transmission > 0.8 {
                    continue;
                }

                prepass.set_bind_group(1, &node.bind_group, &[]);
                prepass.set_vertex_buffer(0, node.gpu_mesh.vertex_buffer.raw.slice(..));
                prepass.set_index_buffer(node.gpu_mesh.index_buffer.raw.slice(..), wgpu::IndexFormat::Uint32);

                if let Some(ref inst_buf) = node.gpu_instances {
                    if inst_buf.count > 0 {
                        prepass.set_pipeline(&self.instanced_depth_prepass_pipeline);
                        prepass.set_vertex_buffer(1, inst_buf.buffer.raw.slice(..));
                        prepass.draw_indexed(0..node.gpu_mesh.index_count, 0, 0..inst_buf.count);
                        prepass.set_pipeline(&self.depth_prepass_pipeline);
                        continue;
                    }
                }

                prepass.draw_indexed(0..node.gpu_mesh.index_count, 0, 0..1);
            }
        }

        // Exécution de la passe SSAO & Flou Bilatéral
        self.ssao.render(
            &mut encoder,
            &context.queue,
            &context.device,
            depth_view,
            &depth_sampler,
            proj_mat,
        );

        // Création du bind group volumétrique avant la passe principale si en mode jeu vidéo
        let vol_bind_group = if is_video_game_mode {
            let (view_proj_mat, _, _) = camera.build_view_proj_matrix(aspect_ratio);
            let inv_vp = view_proj_mat.inverse();
            let vol_uniform_data = VolumetricUniform {
                inv_view_proj: inv_vp.to_cols_array_2d(),
                light_dir: [self.light_pos.x, self.light_pos.y, self.light_pos.z, 0.012],
                light_color: [1.0, 0.95, 0.85, 1.3],
                fog_params: [0.65, 90.0, 32.0, 1.0],
            };
            self.volumetric_fog.uniform_buffer.update(&context.queue, &vol_uniform_data);

            Some(context.device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("Volumetric Fog Forward Bind Group"),
                layout: &self.volumetric_fog.bind_group_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: self.volumetric_fog.uniform_buffer.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: self.camera_uniform.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: self.light_uniform.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 3,
                        resource: wgpu::BindingResource::TextureView(depth_view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 4,
                        resource: wgpu::BindingResource::Sampler(&self.volumetric_fog.depth_sampler),
                    },
                    wgpu::BindGroupEntry {
                        binding: 5,
                        resource: wgpu::BindingResource::TextureView(&self.shadow_target.view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 6,
                        resource: wgpu::BindingResource::Sampler(&self.shadow_sampler),
                    },
                ],
            }))
        } else {
            None
        };

        // =========================================================================
        // PASSE 3 : Rendu Principal de la Scène avec Rendu d'Eau Réfléchissante, SSAO & God Rays
        // =========================================================================
        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Main Forward 3D Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: target_view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(clear_color),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: depth_view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Load, // Conserver le depth buffer rempli par le prepass
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            // A. Rendu des Objets 3D (Statiques et Instanciés) avec Frustum Culling
            // Utilise le pipeline matériau compilé à chaud depuis le graphe nodal s'il existe
            let main_pipeline: &wgpu::RenderPipeline =
                self.material_pipeline.as_ref().unwrap_or(&self.mesh_pipeline);
            render_pass.set_pipeline(main_pipeline);
            render_pass.set_bind_group(0, &self.global_bind_group, &[]);

            // Extraction du Frustum de la caméra pour le culling CPU ultra-rapide
            let (view_proj_mat, _, _) = camera.build_view_proj_matrix(aspect_ratio);
            let frustum = crate::scene::culling::Frustum::from_view_projection_matrix(view_proj_mat);

            for (node_idx, node) in scene.nodes.iter().enumerate() {
                // Test Frustum Culling si l'objet n'a pas d'instances massives
                if node.gpu_instances.is_none() {
                    let world_mat = scene.compute_world_matrix(node_idx);
                    let world_aabb = node.local_aabb().transformed(world_mat);
                    if !frustum.intersects_aabb(&world_aabb) {
                        continue; // Objet hors champ de vision : économie immédiate de Draw Call GPU !
                    }
                }

                render_pass.set_bind_group(1, &node.bind_group, &[]);
                render_pass.set_vertex_buffer(0, node.gpu_mesh.vertex_buffer.raw.slice(..));
                render_pass.set_index_buffer(node.gpu_mesh.index_buffer.raw.slice(..), wgpu::IndexFormat::Uint32);

                if let Some(ref inst_buf) = node.gpu_instances {
                    if inst_buf.count > 0 {
                        render_pass.set_pipeline(&self.instanced_pipeline);
                        render_pass.set_vertex_buffer(1, inst_buf.buffer.raw.slice(..));
                        render_pass.draw_indexed(0..node.gpu_mesh.index_count, 0, 0..inst_buf.count);
                        render_pass.set_pipeline(main_pipeline);
                        continue;
                    }
                }


                render_pass.draw_indexed(0..node.gpu_mesh.index_count, 0, 0..1);
            }

            // B. Rendu de la Skybox Cubemap (Arrière-plan IBL à l'infini avec depth_compare LessEqual)
            render_pass.set_pipeline(&self.skybox_pipeline);
            render_pass.set_bind_group(0, &self.skybox_bind_group, &[]);
            render_pass.draw(0..3, 0..1);

            // C. Rendu de la Grille au Sol (si pas sur l'eau)
            render_pass.set_pipeline(&self.grid_pipeline);
            render_pass.set_bind_group(0, &self.global_bind_group, &[]);
            render_pass.set_bind_group(1, &self.grid_bind_group, &[]);
            render_pass.set_vertex_buffer(0, self.grid_mesh.vertex_buffer.raw.slice(..));
            render_pass.set_index_buffer(self.grid_mesh.index_buffer.raw.slice(..), wgpu::IndexFormat::Uint32);
            render_pass.draw_indexed(0..self.grid_mesh.index_count, 0, 0..1);

            // D. Rendu du Système de Particules GPU (Explosions, Étincelles, Flammes)
            if let Some(particles) = particle_system {
                particles.render(&mut render_pass);
            }
        }

        // =========================================================================
        // PASSE 4 : Rendu Volumétrique Additif (God Rays & Brume Atmosphérique)
        // =========================================================================
        if let Some(ref bg) = vol_bind_group {
            let mut vol_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Volumetric Fog & God Rays Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: target_view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load, // Mélange additif sur la scène déjà rendue
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            vol_pass.set_pipeline(&self.volumetric_fog.pipeline);
            vol_pass.set_bind_group(0, bg, &[]);
            vol_pass.draw(0..3, 0..1);
        }

        context.queue.submit(std::iter::once(encoder.finish()));
    }

    /// Résout l'Anti-Aliasing Temporel (TAA) en rééchantillonnant l'historique et éliminant le scintillement sub-pixel
    pub fn resolve_taa(
        &mut self,
        context: &GpuContext,
        input_color_view: &wgpu::TextureView,
        depth_view: &wgpu::TextureView,
        camera: &Camera,
        aspect_ratio: f32,
        viewport_w: u32,
        viewport_h: u32,
        target_format: wgpu::TextureFormat,
    ) {
        self.taa.resize(&context.device, viewport_w, viewport_h, target_format);
        let (view_proj_mat, _, _) = camera.build_view_proj_matrix(aspect_ratio);
        let (_, uv_jitter) = self.taa.compute_jitter();

        let mut encoder = context.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("TAA Resolve Command Encoder"),
        });

        self.taa.render(
            &mut encoder,
            &context.device,
            &context.queue,
            input_color_view,
            depth_view,
            view_proj_mat,
            uv_jitter,
        );

        context.queue.submit(std::iter::once(encoder.finish()));
    }

    /// Résout la passe Screen-Space Reflections (SSR) par raymarching GPU additif
    pub fn resolve_ssr(
        &mut self,
        context: &GpuContext,
        color_view: &wgpu::TextureView,
        depth_view: &wgpu::TextureView,
        viewport_w: u32,
        viewport_h: u32,
        target_format: wgpu::TextureFormat,
    ) {
        self.ssr.resize(&context.device, viewport_w, viewport_h, target_format);
        let bind_group = self.ssr.create_bind_group(
            &context.device,
            &self.camera_uniform,
            color_view,
            depth_view,
        );

        let mut encoder = context.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("SSR Resolve Command Encoder"),
        });

        self.ssr.render(&mut encoder, &bind_group);

        context.queue.submit(std::iter::once(encoder.finish()));
    }
}
