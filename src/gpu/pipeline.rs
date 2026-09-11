// [WFGY] Zone: SAFE | λ: 0.3 | Fallbacks: 0/None | Action: Graphics Pipeline Builder for wgpu
use crate::gpu::DepthTexture;

pub struct RenderPipelineBuilder<'a> {
    device: &'a wgpu::Device,
    shader_source: &'a str,
    bind_group_layouts: Vec<&'a wgpu::BindGroupLayout>,
    vertex_layouts: Vec<wgpu::VertexBufferLayout<'a>>,
    target_format: wgpu::TextureFormat,
    depth_format: Option<wgpu::TextureFormat>,
    cull_mode: Option<wgpu::Face>,
    topology: wgpu::PrimitiveTopology,
    label: Option<&'a str>,
    vertex_entry_point: &'a str,
}

impl<'a> RenderPipelineBuilder<'a> {
    pub fn new(device: &'a wgpu::Device, shader_source: &'a str, target_format: wgpu::TextureFormat) -> Self {
        RenderPipelineBuilder {
            device,
            shader_source,
            bind_group_layouts: Vec::new(),
            vertex_layouts: Vec::new(),
            target_format,
            depth_format: Some(DepthTexture::DEPTH_FORMAT),
            cull_mode: None, // No backface culling by default for two-sided visibility or configurable
            topology: wgpu::PrimitiveTopology::TriangleList,
            label: Some("GPU Render Pipeline"),
            vertex_entry_point: "vs_main",
        }
    }

    pub fn with_vertex_entry_point(mut self, entry_point: &'a str) -> Self {
        self.vertex_entry_point = entry_point;
        self
    }

    pub fn with_bind_group_layout(mut self, layout: &'a wgpu::BindGroupLayout) -> Self {
        self.bind_group_layouts.push(layout);
        self
    }

    pub fn with_vertex_layout(mut self, layout: wgpu::VertexBufferLayout<'a>) -> Self {
        self.vertex_layouts.push(layout);
        self
    }

    pub fn with_cull_mode(mut self, cull: Option<wgpu::Face>) -> Self {
        self.cull_mode = cull;
        self
    }

    pub fn with_topology(mut self, topology: wgpu::PrimitiveTopology) -> Self {
        self.topology = topology;
        self
    }

    pub fn with_label(mut self, label: &'a str) -> Self {
        self.label = Some(label);
        self
    }

    pub fn build(self) -> (wgpu::RenderPipeline, wgpu::PipelineLayout) {
        let shader = self.device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: self.label,
            source: wgpu::ShaderSource::Wgsl(self.shader_source.into()),
        });

        let pipeline_layout = self.device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: self.label,
            bind_group_layouts: &self.bind_group_layouts,
            push_constant_ranges: &[],
        });

        let depth_stencil = self.depth_format.map(|format| wgpu::DepthStencilState {
            format,
            depth_write_enabled: true,
            depth_compare: wgpu::CompareFunction::Less,
            stencil: wgpu::StencilState::default(),
            bias: wgpu::DepthBiasState::default(),
        });

        let pipeline = self.device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: self.label,
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: self.vertex_entry_point,
                buffers: &self.vertex_layouts,
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: "fs_main",
                targets: &[Some(wgpu::ColorTargetState {
                    format: self.target_format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState {
                topology: self.topology,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: self.cull_mode,
                polygon_mode: wgpu::PolygonMode::Fill,
                unclipped_depth: false,
                conservative: false,
            },
            depth_stencil,
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview: None,
        });

        (pipeline, pipeline_layout)
    }
}
