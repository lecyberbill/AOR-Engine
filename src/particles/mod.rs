// [WFGY] Zone: SAFE | λ: 0.3 | Fallbacks: 0/None | Action: High-Performance GPU Particle System with Compute Simulation & Instanced Billboard Rendering
use glam::Vec3;
use crate::gpu::{GpuContext, UniformBuffer};
use crate::scene::{Camera, CameraUniform};
use bytemuck::{Pod, Zeroable};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ParticleKind {
    Fire,
    Spark,
    Smoke,
    Debris,
}

#[derive(Clone, Debug)]
pub struct Particle {
    pub position: Vec3,
    pub velocity: Vec3,
    pub color: [f32; 4],
    pub size: f32,
    pub initial_size: f32,
    pub age: f32,
    pub max_life: f32,
    pub kind: ParticleKind,
    pub gravity_mult: f32,
    pub drag: f32,
    pub rotation: f32,
    pub rot_speed: f32,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct ParticleGpuPod {
    pub position: [f32; 4], // xyz = pos, w = scale
    pub velocity: [f32; 4], // xyz = vel, w = life
    pub color: [f32; 4],    // rgba
    pub extra: [f32; 4],    // x = rotation, y = rot_speed, z = kind, w = max_life
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct ParticleComputeParams {
    pub delta_time: f32,
    pub gravity: f32,
    pub drag: f32,
    pub bounce: f32,
    pub particle_count: u32,
    pub ground_height: f32,
    pub _pad0: u32,
    pub _pad1: u32,
}

impl Default for ParticleComputeParams {
    fn default() -> Self {
        Self {
            delta_time: 1.0 / 60.0,
            gravity: 9.81,
            drag: 0.05,
            bounce: 0.45,
            particle_count: 0,
            ground_height: 0.0,
            _pad0: 0,
            _pad1: 0,
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct ParticleInstanceGpu {
    pub position_scale: [f32; 4], // xyz = pos, w = scale
    pub color: [f32; 4],          // rgba (with HDR glow in rgb and alpha)
    pub uv_data: [f32; 4],        // x = rotation, y = kind (0=fire, 1=spark, 2=smoke, 3=debris), zw = unused
}

impl ParticleInstanceGpu {
    pub fn desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<ParticleInstanceGpu>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Instance,
            attributes: &[
                // position_scale
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: wgpu::VertexFormat::Float32x4,
                },
                // color
                wgpu::VertexAttribute {
                    offset: 16,
                    shader_location: 1,
                    format: wgpu::VertexFormat::Float32x4,
                },
                // uv_data
                wgpu::VertexAttribute {
                    offset: 32,
                    shader_location: 2,
                    format: wgpu::VertexFormat::Float32x4,
                },
            ],
        }
    }
}

pub struct ParticleSystem {
    pub particles: Vec<Particle>,
    pub max_particles: usize,
    pub instance_buffer: wgpu::Buffer,
    pub pipeline: wgpu::RenderPipeline,
    pub camera_uniform: UniformBuffer<CameraUniform>,
    pub bind_group: wgpu::BindGroup,
    // Compute pipeline components
    pub compute_pipeline: wgpu::ComputePipeline,
    pub compute_params_uniform: UniformBuffer<ParticleComputeParams>,
    pub storage_buffer_a: wgpu::Buffer,
    pub storage_buffer_b: wgpu::Buffer,
    pub compute_bind_group_a: wgpu::BindGroup,
    pub compute_bind_group_b: wgpu::BindGroup,
    pub ping_pong_state: bool,
}

impl ParticleSystem {
    pub fn new(context: &GpuContext, target_format: wgpu::TextureFormat, max_particles: usize) -> Self {
        let device = &context.device;

        let camera_uniform = UniformBuffer::new(
            device,
            Some("Particle Camera Uniform"),
            &CameraUniform::default(),
        );

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Particle Bind Group Layout"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Particle Bind Group"),
            layout: &bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: camera_uniform.as_entire_binding(),
            }],
        });

        let instance_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Particle Instance GPU Buffer"),
            size: (max_particles * std::mem::size_of::<ParticleInstanceGpu>()) as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // Compute Buffers & Pipeline setup
        let storage_buffer_size = (max_particles * std::mem::size_of::<ParticleGpuPod>()).max(64) as u64;
        let storage_buffer_a = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Particle Compute Buffer A"),
            size: storage_buffer_size,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });
        let storage_buffer_b = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Particle Compute Buffer B"),
            size: storage_buffer_size,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });

        let compute_params = ParticleComputeParams::default();
        let compute_params_uniform = UniformBuffer::new(
            device,
            Some("Particle Compute Params Uniform"),
            &compute_params,
        );

        let compute_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Particle Compute Bind Group Layout"),
            entries: &[
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
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: false },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });

        let compute_bind_group_a = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Particle Compute Bind Group A (A->B)"),
            layout: &compute_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: compute_params_uniform.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: storage_buffer_a.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: storage_buffer_b.as_entire_binding(),
                },
            ],
        });

        let compute_bind_group_b = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Particle Compute Bind Group B (B->A)"),
            layout: &compute_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: compute_params_uniform.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: storage_buffer_b.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: storage_buffer_a.as_entire_binding(),
                },
            ],
        });

        let compute_shader_source = include_str!("../shaders/particles_compute.wgsl");
        let compute_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Particle Compute Shader"),
            source: wgpu::ShaderSource::Wgsl(compute_shader_source.into()),
        });

        let compute_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Particle Compute Pipeline Layout"),
            bind_group_layouts: &[&compute_bind_group_layout],
            push_constant_ranges: &[],
        });

        let compute_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("Particle Simulation Compute Pipeline"),
            layout: Some(&compute_pipeline_layout),
            module: &compute_shader,
            entry_point: "cs_main",
        });

        // Pipeline Shader WGSL pour les particules
        let shader_source = include_str!("../shaders/particle.wgsl");
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Particle Shader Module"),
            source: wgpu::ShaderSource::Wgsl(shader_source.into()),
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Particle Pipeline Layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Particle Billboard Render Pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "vs_main",
                buffers: &[ParticleInstanceGpu::desc()],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: "fs_main",
                targets: &[Some(wgpu::ColorTargetState {
                    format: target_format,
                    blend: Some(wgpu::BlendState {
                        color: wgpu::BlendComponent {
                            src_factor: wgpu::BlendFactor::SrcAlpha,
                            dst_factor: wgpu::BlendFactor::One, // Additive blending for fiery glow
                            operation: wgpu::BlendOperation::Add,
                        },
                        alpha: wgpu::BlendComponent {
                            src_factor: wgpu::BlendFactor::One,
                            dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
                            operation: wgpu::BlendOperation::Add,
                        },
                    }),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: None, // Billboard faces camera both sides
                polygon_mode: wgpu::PolygonMode::Fill,
                unclipped_depth: false,
                conservative: false,
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: wgpu::TextureFormat::Depth32Float,
                depth_write_enabled: false, // Don't occlude particles behind transparent quads
                depth_compare: wgpu::CompareFunction::LessEqual,
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            }),
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
        });

        Self {
            particles: Vec::with_capacity(max_particles),
            max_particles,
            instance_buffer,
            pipeline,
            camera_uniform,
            bind_group,
            compute_pipeline,
            compute_params_uniform,
            storage_buffer_a,
            storage_buffer_b,
            compute_bind_group_a,
            compute_bind_group_b,
            ping_pong_state: false,
        }
    }

    /// Émet une déflagration d'explosion avec étincelles, flash, débris et fumée
    pub fn spawn_explosion(&mut self, origin: Vec3, intensity: f32) {
        let mut seed = (origin.x * 1000.0 + origin.y * 100.0 + origin.z * 10.0 + self.particles.len() as f32 * 17.0) as u32 ^ 0x9e3779b9;
        let mut rand_f32 = || -> f32 {
            seed = seed.wrapping_mul(1664525).wrapping_add(1013904223);
            (seed >> 8) as f32 / 16777216.0
        };
        let mut rand_range = |min: f32, max: f32| -> f32 {
            min + rand_f32() * (max - min)
        };

        // 1. Boule de feu centrale (Fire Core)
        let fire_count = (30.0 * intensity).min(100.0) as usize;
        for _ in 0..fire_count {
            if self.particles.len() >= self.max_particles { break; }
            let dir = Vec3::new(
                rand_range(-1.0, 1.0),
                rand_range(0.2, 1.5),
                rand_range(-1.0, 1.0),
            ).normalize_or_zero();
            let speed = rand_range(3.0, 12.0) * intensity;
            let life = rand_range(0.4, 0.9);
            let size = rand_range(0.6, 1.4) * intensity.min(2.0);

            self.particles.push(Particle {
                position: origin + dir * rand_range(0.0, 0.5),
                velocity: dir * speed,
                color: [2.5, 1.2, 0.3, 1.0], // Super incandescent HDR
                size,
                initial_size: size,
                age: 0.0,
                max_life: life,
                kind: ParticleKind::Fire,
                gravity_mult: -0.2, // Rises up
                drag: 0.08,
                rotation: rand_range(0.0, std::f32::consts::TAU),
                rot_speed: rand_range(-3.0, 3.0),
            });
        }

        // 2. Étincelles vives et débris balistiques (Sparks & Shrapnel)
        let spark_count = (60.0 * intensity).min(250.0) as usize;
        for _ in 0..spark_count {
            if self.particles.len() >= self.max_particles { break; }
            let dir = Vec3::new(
                rand_range(-1.0, 1.0),
                rand_range(-0.2, 1.8),
                rand_range(-1.0, 1.0),
            ).normalize_or_zero();
            let speed = rand_range(8.0, 28.0) * intensity;
            let life = rand_range(0.6, 1.6);
            let size = rand_range(0.12, 0.3);

            self.particles.push(Particle {
                position: origin,
                velocity: dir * speed,
                color: [3.0, 1.8, 0.8, 1.0],
                size,
                initial_size: size,
                age: 0.0,
                max_life: life,
                kind: ParticleKind::Spark,
                gravity_mult: 1.2, // Falls with gravity
                drag: 0.03,
                rotation: rand_range(0.0, std::f32::consts::TAU),
                rot_speed: rand_range(-8.0, 8.0),
            });
        }

        // 3. Volutes de Fumée d'explosion (Smoke Plume)
        let smoke_count = (20.0 * intensity).min(60.0) as usize;
        for _ in 0..smoke_count {
            if self.particles.len() >= self.max_particles { break; }
            let dir = Vec3::new(
                rand_range(-1.0, 1.0),
                rand_range(0.5, 2.0),
                rand_range(-1.0, 1.0),
            ).normalize_or_zero();
            let speed = rand_range(1.5, 5.0) * intensity;
            let life = rand_range(1.2, 2.8);
            let size = rand_range(0.8, 1.8);

            self.particles.push(Particle {
                position: origin + dir * 0.2,
                velocity: dir * speed,
                color: [0.15, 0.15, 0.18, 0.6],
                size,
                initial_size: size,
                age: 0.0,
                max_life: life,
                kind: ParticleKind::Smoke,
                gravity_mult: -0.4, // Smoke rises
                drag: 0.15,
                rotation: rand_range(0.0, std::f32::consts::TAU),
                rot_speed: rand_range(-1.0, 1.0),
            });
        }
    }

    /// Émet un flux continu de flammes et fumée pour réacteurs et feux de camp
    pub fn emit_thruster_plume(&mut self, origin: Vec3, direction: Vec3, count: usize) {
        let mut seed = (origin.x * 37.0 + origin.y * 59.0 + self.particles.len() as f32) as u32 ^ 0xa5a5a5a5;
        let mut rand_f32 = || -> f32 {
            seed = seed.wrapping_mul(1664525).wrapping_add(1013904223);
            (seed >> 8) as f32 / 16777216.0
        };

        for _ in 0..count {
            if self.particles.len() >= self.max_particles { break; }
            let spread = 0.25;
            let dir = (direction + Vec3::new(
                (rand_f32() - 0.5) * spread,
                (rand_f32() - 0.5) * spread,
                (rand_f32() - 0.5) * spread,
            )).normalize_or_zero();

            let speed = 12.0 + rand_f32() * 8.0;
            let life = 0.2 + rand_f32() * 0.3;
            let size = 0.35 + rand_f32() * 0.3;

            self.particles.push(Particle {
                position: origin + dir * 0.1,
                velocity: dir * speed,
                color: [0.2, 1.2, 3.5, 1.0], // Plasma cyan / bleu électrique
                size,
                initial_size: size,
                age: 0.0,
                max_life: life,
                kind: ParticleKind::Fire,
                gravity_mult: 0.0,
                drag: 0.05,
                rotation: rand_f32() * std::f32::consts::TAU,
                rot_speed: (rand_f32() - 0.5) * 6.0,
            });
        }
    }

    /// Émet des étincelles directionnelles (impact, tir laser, friction)
    pub fn spawn_sparks(&mut self, origin: Vec3, normal: Vec3, count: usize) {
        let mut seed = (origin.x * 31.0 + origin.y * 47.0 + origin.z * 71.0 + self.particles.len() as f32 * 13.0) as u32 ^ 0x12345678;
        let mut rand_f32 = || -> f32 {
            seed = seed.wrapping_mul(1664525).wrapping_add(1013904223);
            (seed >> 8) as f32 / 16777216.0
        };

        for _ in 0..count {
            if self.particles.len() >= self.max_particles { break; }
            let spread_dir = (normal + Vec3::new(
                (rand_f32() - 0.5) * 1.5,
                rand_f32() * 0.8,
                (rand_f32() - 0.5) * 1.5,
            )).normalize_or_zero();

            let speed = 4.0 + rand_f32() * 12.0;
            let life = 0.3 + rand_f32() * 0.7;
            let size = 0.12 + rand_f32() * 0.18;

            self.particles.push(Particle {
                position: origin,
                velocity: spread_dir * speed,
                color: [3.0, 2.0, 0.7, 1.0],
                size,
                initial_size: size,
                age: 0.0,
                max_life: life,
                kind: ParticleKind::Spark,
                gravity_mult: 1.2,
                drag: 0.04,
                rotation: rand_f32() * std::f32::consts::TAU,
                rot_speed: (rand_f32() - 0.5) * 10.0,
            });
        }
    }

    /// Exécute la simulation physique des particules (CPU et transition Compute)
    pub fn update(&mut self, dt: f32) {
        let mut i = 0;
        while i < self.particles.len() {
            let p = &mut self.particles[i];
            p.age += dt;

            if p.age >= p.max_life {
                self.particles.swap_remove(i);
                continue;
            }

            let life_ratio = p.age / p.max_life;

            // Évolution de taille et couleur selon le type
            match p.kind {
                ParticleKind::Fire => {
                    p.size = p.initial_size * (1.0 + life_ratio * 0.8);
                    p.color[3] = (1.0 - life_ratio).powf(1.5);
                }
                ParticleKind::Spark => {
                    p.size = p.initial_size * (1.0 - life_ratio * 0.7);
                    p.color[3] = (1.0 - life_ratio).powf(0.8);
                }
                ParticleKind::Smoke => {
                    p.size = p.initial_size * (1.0 + life_ratio * 2.5); // Expansion volumique
                    p.color[3] = (1.0 - life_ratio) * 0.45;
                }
                ParticleKind::Debris => {
                    p.size = p.initial_size;
                    p.color[3] = (1.0 - life_ratio).powf(0.5);
                }
            }

            // Dynamique physique
            let gravity = Vec3::new(0.0, -9.81 * p.gravity_mult, 0.0);
            p.velocity += gravity * dt;
            let drag_factor = (1.0 - p.drag * dt).max(0.0);
            p.velocity *= drag_factor;
            p.position += p.velocity * dt;

            // Rebond simple sur le sol (y = 0.0)
            if p.position.y < 0.05 && p.velocity.y < 0.0 {
                p.position.y = 0.05;
                p.velocity.y = -p.velocity.y * 0.45; // Rebond amorti
                p.velocity.x *= 0.7;
                p.velocity.z *= 0.7;
            }

            p.rotation += p.rot_speed * dt;
            i += 1;
        }
    }

    /// Lance la simulation physique GPU Compute avec double-buffering ping-pong
    pub fn dispatch_compute(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        queue: &wgpu::Queue,
        dt: f32,
    ) {
        if self.particles.is_empty() {
            return;
        }

        let count = self.particles.len().min(self.max_particles) as u32;
        let compute_params = ParticleComputeParams {
            delta_time: dt,
            gravity: 9.81,
            drag: 0.05,
            bounce: 0.45,
            particle_count: count,
            ground_height: 0.0,
            _pad0: 0,
            _pad1: 0,
        };
        self.compute_params_uniform.update(queue, &compute_params);

        let mut cpass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: Some("Particle Physics Compute Pass"),
            timestamp_writes: None,
        });

        cpass.set_pipeline(&self.compute_pipeline);
        if self.ping_pong_state {
            cpass.set_bind_group(0, &self.compute_bind_group_b, &[]);
        } else {
            cpass.set_bind_group(0, &self.compute_bind_group_a, &[]);
        }

        let workgroups = (count + 63) / 64;
        cpass.dispatch_workgroups(workgroups, 1, 1);
        drop(cpass);

        self.ping_pong_state = !self.ping_pong_state;
    }

    /// Prépare le buffer GPU pour le rendu de la frame
    pub fn prepare_gpu(&mut self, queue: &wgpu::Queue, camera: &Camera, aspect: f32, time: f32) {
        let (view_proj, view, proj) = camera.build_view_proj_matrix(aspect);
        let cam_pos = camera.position();

        let cam_uniform = CameraUniform {
            view_proj: view_proj.to_cols_array_2d(),
            view: view.to_cols_array_2d(),
            proj: proj.to_cols_array_2d(),
            camera_pos: [cam_pos.x, cam_pos.y, cam_pos.z, time],
        };
        self.camera_uniform.update(queue, &cam_uniform);

        if self.particles.is_empty() {
            return;
        }

        let instances: Vec<ParticleInstanceGpu> = self.particles
            .iter()
            .take(self.max_particles)
            .map(|p| {
                let kind_idx = match p.kind {
                    ParticleKind::Fire => 0.0,
                    ParticleKind::Spark => 1.0,
                    ParticleKind::Smoke => 2.0,
                    ParticleKind::Debris => 3.0,
                };
                ParticleInstanceGpu {
                    position_scale: [p.position.x, p.position.y, p.position.z, p.size],
                    color: p.color,
                    uv_data: [p.rotation, kind_idx, 0.0, 0.0],
                }
            })
            .collect();

        queue.write_buffer(
            &self.instance_buffer,
            0,
            bytemuck::cast_slice(&instances),
        );
    }

    /// Effectue le rendu des particules dans le render pass actif
    pub fn render<'a>(&'a self, render_pass: &mut wgpu::RenderPass<'a>) {
        if self.particles.is_empty() {
            return;
        }

        render_pass.set_pipeline(&self.pipeline);
        render_pass.set_bind_group(0, &self.bind_group, &[]);
        render_pass.set_vertex_buffer(0, self.instance_buffer.slice(0..(self.particles.len() * std::mem::size_of::<ParticleInstanceGpu>()) as u64));
        
        // Dessine 6 sommets par instance (2 triangles formant un Quad billboard généré en shader)
        render_pass.draw(0..6, 0..self.particles.len() as u32);
    }
}

#[cfg(test)]
pub mod tests {
    use super::*;

    #[test]
    fn test_particle_compute_params_size_and_alignment() {
        assert_eq!(std::mem::size_of::<ParticleComputeParams>(), 32);
        assert_eq!(std::mem::size_of::<ParticleComputeParams>() % 16, 0);
    }

    #[test]
    fn test_particle_gpu_pod_size_and_alignment() {
        assert_eq!(std::mem::size_of::<ParticleGpuPod>(), 64);
        assert_eq!(std::mem::size_of::<ParticleGpuPod>() % 16, 0);
    }

    #[test]
    fn test_particle_instance_gpu_size_and_alignment() {
        assert_eq!(std::mem::size_of::<ParticleInstanceGpu>(), 48);
        assert_eq!(std::mem::size_of::<ParticleInstanceGpu>() % 16, 0);
    }
}
