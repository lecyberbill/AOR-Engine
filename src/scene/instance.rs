// [WFGY] Zone: SAFE | λ: 0.3 | Fallbacks: 0/None | Action: GPU Hardware Mesh Instancing data structures and buffer management
#![allow(dead_code)]
use glam::{Mat4, Quat, Vec3};
use crate::gpu::GpuBuffer;

/// Données d'une instance virtuelle côté CPU
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Instance {
    pub position: Vec3,
    pub rotation: Quat,
    pub scale: Vec3,
    pub color_tint: [f32; 4],
}

impl Default for Instance {
    fn default() -> Self {
        Self {
            position: Vec3::ZERO,
            rotation: Quat::IDENTITY,
            scale: Vec3::ONE,
            color_tint: [1.0, 1.0, 1.0, 1.0],
        }
    }
}

impl Instance {
    pub fn new(position: Vec3, rotation: Quat, scale: Vec3, color_tint: [f32; 4]) -> Self {
        Self {
            position,
            rotation,
            scale,
            color_tint,
        }
    }

    pub fn to_matrix(&self) -> Mat4 {
        Mat4::from_scale_rotation_translation(self.scale, self.rotation, self.position)
    }

    pub fn to_raw(&self) -> InstanceRaw {
        let matrix = self.to_matrix();
        InstanceRaw {
            model_matrix_0: matrix.x_axis.to_array(),
            model_matrix_1: matrix.y_axis.to_array(),
            model_matrix_2: matrix.z_axis.to_array(),
            model_matrix_3: matrix.w_axis.to_array(),
            color_tint: self.color_tint,
        }
    }
}

/// Représentation binaire exacte GPU (80 octets, alignement 16 octets)
#[repr(C)]
#[derive(Clone, Copy, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct InstanceRaw {
    pub model_matrix_0: [f32; 4], // 16 octets - location 5
    pub model_matrix_1: [f32; 4], // 16 octets - location 6
    pub model_matrix_2: [f32; 4], // 16 octets - location 7
    pub model_matrix_3: [f32; 4], // 16 octets - location 8
    pub color_tint: [f32; 4],     // 16 octets - location 9
}

impl InstanceRaw {
    pub fn desc<'a>() -> wgpu::VertexBufferLayout<'a> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<InstanceRaw>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Instance,
            attributes: &[
                // location 5: model_matrix col 0
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 5,
                    format: wgpu::VertexFormat::Float32x4,
                },
                // location 6: model_matrix col 1
                wgpu::VertexAttribute {
                    offset: 16,
                    shader_location: 6,
                    format: wgpu::VertexFormat::Float32x4,
                },
                // location 7: model_matrix col 2
                wgpu::VertexAttribute {
                    offset: 32,
                    shader_location: 7,
                    format: wgpu::VertexFormat::Float32x4,
                },
                // location 8: model_matrix col 3
                wgpu::VertexAttribute {
                    offset: 48,
                    shader_location: 8,
                    format: wgpu::VertexFormat::Float32x4,
                },
                // location 9: color_tint
                wgpu::VertexAttribute {
                    offset: 64,
                    shader_location: 9,
                    format: wgpu::VertexFormat::Float32x4,
                },
            ],
        }
    }
}

/// Buffer d'instances GPU pour le rendu accéléré en un seul draw call
pub struct GpuInstanceBuffer {
    pub buffer: GpuBuffer,
    pub count: u32,
}

impl GpuInstanceBuffer {
    pub fn new(device: &wgpu::Device, label: Option<&str>, instances: &[InstanceRaw]) -> Self {
        let count = instances.len().max(1) as u32;
        let data: Vec<InstanceRaw> = if instances.is_empty() {
            vec![InstanceRaw {
                model_matrix_0: [1.0, 0.0, 0.0, 0.0],
                model_matrix_1: [0.0, 1.0, 0.0, 0.0],
                model_matrix_2: [0.0, 0.0, 1.0, 0.0],
                model_matrix_3: [0.0, 0.0, 0.0, 1.0],
                color_tint: [1.0, 1.0, 1.0, 1.0],
            }]
        } else {
            instances.to_vec()
        };

        let buffer = GpuBuffer::new_init(
            device,
            label,
            bytemuck::cast_slice(&data),
            wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
        );

        Self { buffer, count: count }
    }

    pub fn update(&mut self, queue: &wgpu::Queue, instances: &[InstanceRaw]) {
        self.count = instances.len() as u32;
        if !instances.is_empty() {
            self.buffer.write_bytes(queue, 0, bytemuck::cast_slice(instances));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_instance_raw_size_and_alignment() {
        assert_eq!(std::mem::size_of::<InstanceRaw>(), 80);
        assert_eq!(std::mem::align_of::<InstanceRaw>(), 4);
    }

    #[test]
    fn test_instance_to_matrix() {
        let inst = Instance::new(
            Vec3::new(10.0, 20.0, 30.0),
            Quat::IDENTITY,
            Vec3::new(2.0, 2.0, 2.0),
            [1.0, 0.5, 0.2, 1.0],
        );
        let raw = inst.to_raw();
        assert_eq!(raw.model_matrix_3[0], 10.0);
        assert_eq!(raw.model_matrix_3[1], 20.0);
        assert_eq!(raw.model_matrix_3[2], 30.0);
        assert_eq!(raw.color_tint, [1.0, 0.5, 0.2, 1.0]);
    }
}
