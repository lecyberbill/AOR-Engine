// [WFGY] Zone: SAFE | λ: 0.3 | Fallbacks: 0/None | Action: Vertex layout with Tangents (TBN), GpuMesh and 3D primitives generation
use crate::gpu::GpuBuffer;
use glam::Vec3;

#[repr(C)]
#[derive(Clone, Copy, Debug, bytemuck::Pod, bytemuck::Zeroable, serde::Serialize, serde::Deserialize)]
pub struct Vertex {
    pub position: [f32; 3], // 12 octets - location 0
    pub normal: [f32; 3],   // 12 octets - location 1
    pub tangent: [f32; 4],  // 16 octets - location 2 (T.xyz + sign w)
    pub uv: [f32; 2],       // 8 octets  - location 3
    pub color: [f32; 4],    // 16 octets - location 4
} // Total = 64 octets

impl Vertex {
    pub fn desc<'a>() -> wgpu::VertexBufferLayout<'a> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Vertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                // location 0: position
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: wgpu::VertexFormat::Float32x3,
                },
                // location 1: normal
                wgpu::VertexAttribute {
                    offset: 12,
                    shader_location: 1,
                    format: wgpu::VertexFormat::Float32x3,
                },
                // location 2: tangent
                wgpu::VertexAttribute {
                    offset: 24,
                    shader_location: 2,
                    format: wgpu::VertexFormat::Float32x4,
                },
                // location 3: uv
                wgpu::VertexAttribute {
                    offset: 40,
                    shader_location: 3,
                    format: wgpu::VertexFormat::Float32x2,
                },
                // location 4: color
                wgpu::VertexAttribute {
                    offset: 48,
                    shader_location: 4,
                    format: wgpu::VertexFormat::Float32x4,
                },
            ],
        }
    }
}

pub struct GpuMesh {
    pub vertex_buffer: GpuBuffer,
    pub index_buffer: GpuBuffer,
    pub index_count: u32,
}

impl GpuMesh {
    pub fn new(device: &wgpu::Device, label: Option<&str>, vertices: &[Vertex], indices: &[u32]) -> Self {
        let vertex_buffer = GpuBuffer::new_init(
            device,
            label.map(|l| format!("{} Vertex Buffer", l)).as_deref(),
            bytemuck::cast_slice(vertices),
            wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
        );
        let index_buffer = GpuBuffer::new_init(
            device,
            label.map(|l| format!("{} Index Buffer", l)).as_deref(),
            bytemuck::cast_slice(indices),
            wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
        );

        GpuMesh {
            vertex_buffer,
            index_buffer,
            index_count: indices.len() as u32,
        }
    }

    pub fn update_vertices(&self, queue: &wgpu::Queue, vertices: &[Vertex]) {
        self.vertex_buffer.write_bytes(queue, 0, bytemuck::cast_slice(vertices));
    }
}

#[allow(dead_code)]
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum PrimitiveType {
    Cube,
    Pyramid,
    Prism,
    Plane,
    Sphere,
    Cylinder,
    Torus,
    Tree,
    Column,
    Custom(String),
}

/// Calcule les tangentes et bitangentes orthogonales pour un maillage triangulé
pub fn compute_tangents(vertices: &mut [Vertex], indices: &[u32]) {
    let mut tangents = vec![Vec3::ZERO; vertices.len()];
    let mut bitangents = vec![Vec3::ZERO; vertices.len()];

    for chunk in indices.chunks(3) {
        if chunk.len() == 3 {
            let i0 = chunk[0] as usize;
            let i1 = chunk[1] as usize;
            let i2 = chunk[2] as usize;

            if i0 < vertices.len() && i1 < vertices.len() && i2 < vertices.len() {
                let v0 = &vertices[i0];
                let v1 = &vertices[i1];
                let v2 = &vertices[i2];

                let p0 = Vec3::from_array(v0.position);
                let p1 = Vec3::from_array(v1.position);
                let p2 = Vec3::from_array(v2.position);

                let uv0 = glam::Vec2::from_array(v0.uv);
                let uv1 = glam::Vec2::from_array(v1.uv);
                let uv2 = glam::Vec2::from_array(v2.uv);

                let dp1 = p1 - p0;
                let dp2 = p2 - p0;

                let duv1 = uv1 - uv0;
                let duv2 = uv2 - uv0;

                let det = duv1.x * duv2.y - duv1.y * duv2.x;
                let r = if det.abs() > 1e-6 { 1.0 / det } else { 1.0 };

                let tangent = (dp1 * duv2.y - dp2 * duv1.y) * r;
                let bitangent = (dp2 * duv1.x - dp1 * duv2.x) * r;

                tangents[i0] += tangent;
                tangents[i1] += tangent;
                tangents[i2] += tangent;

                bitangents[i0] += bitangent;
                bitangents[i1] += bitangent;
                bitangents[i2] += bitangent;
            }
        }
    }

    for (i, v) in vertices.iter_mut().enumerate() {
        let n = Vec3::from_array(v.normal).normalize_or_zero();
        let t = tangents[i];
        let b = bitangents[i];

        // Gram-Schmidt orthogonalisation
        let tangent_ortho = (t - n * n.dot(t)).normalize_or_zero();
        let final_tangent = if tangent_ortho.length_squared() > 1e-4 {
            tangent_ortho
        } else {
            // Tangente de repli si UVs dégénérées
            if n.y.abs() < 0.99 { Vec3::new(-n.z, 0.0, n.x).normalize() } else { Vec3::X }
        };

        // Calcul du signe de la bitangente pour la chiralité
        let sign = if n.cross(final_tangent).dot(b) < 0.0 { -1.0 } else { 1.0 };

        v.tangent = [final_tangent.x, final_tangent.y, final_tangent.z, sign];
    }
}

pub fn generate_cube_data() -> (Vec<Vertex>, Vec<u32>) {
    let mut vertices = Vec::new();
    let mut indices = Vec::new();

    // 6 faces définies avec normales, couleurs et coordonnées UV par quad
    let faces = [
        // Avant (+Z)
        (Vec3::new(0.0, 0.0, 1.0), [0.93, 0.33, 0.31, 1.0], [
            (Vec3::new(-1.0, -1.0,  1.0), [0.0, 1.0]),
            (Vec3::new( 1.0, -1.0,  1.0), [1.0, 1.0]),
            (Vec3::new( 1.0,  1.0,  1.0), [1.0, 0.0]),
            (Vec3::new(-1.0,  1.0,  1.0), [0.0, 0.0]),
        ]),
        // Arrière (-Z)
        (Vec3::new(0.0, 0.0, -1.0), [1.0, 0.79, 0.16, 1.0], [
            (Vec3::new( 1.0, -1.0, -1.0), [0.0, 1.0]),
            (Vec3::new(-1.0, -1.0, -1.0), [1.0, 1.0]),
            (Vec3::new(-1.0,  1.0, -1.0), [1.0, 0.0]),
            (Vec3::new( 1.0,  1.0, -1.0), [0.0, 0.0]),
        ]),
        // Gauche (-X)
        (Vec3::new(-1.0, 0.0, 0.0), [0.4, 0.73, 0.42, 1.0], [
            (Vec3::new(-1.0, -1.0, -1.0), [0.0, 1.0]),
            (Vec3::new(-1.0, -1.0,  1.0), [1.0, 1.0]),
            (Vec3::new(-1.0,  1.0,  1.0), [1.0, 0.0]),
            (Vec3::new(-1.0,  1.0, -1.0), [0.0, 0.0]),
        ]),
        // Droite (+X)
        (Vec3::new(1.0, 0.0, 0.0), [0.26, 0.65, 0.96, 1.0], [
            (Vec3::new( 1.0, -1.0,  1.0), [0.0, 1.0]),
            (Vec3::new( 1.0, -1.0, -1.0), [1.0, 1.0]),
            (Vec3::new( 1.0,  1.0, -1.0), [1.0, 0.0]),
            (Vec3::new( 1.0,  1.0,  1.0), [0.0, 0.0]),
        ]),
        // Haut (+Y)
        (Vec3::new(0.0, 1.0, 0.0), [0.67, 0.28, 0.74, 1.0], [
            (Vec3::new(-1.0,  1.0,  1.0), [0.0, 1.0]),
            (Vec3::new( 1.0,  1.0,  1.0), [1.0, 1.0]),
            (Vec3::new( 1.0,  1.0, -1.0), [1.0, 0.0]),
            (Vec3::new(-1.0,  1.0, -1.0), [0.0, 0.0]),
        ]),
        // Bas (-Y)
        (Vec3::new(0.0, -1.0, 0.0), [0.15, 0.65, 0.6, 1.0], [
            (Vec3::new(-1.0, -1.0, -1.0), [0.0, 1.0]),
            (Vec3::new( 1.0, -1.0, -1.0), [1.0, 1.0]),
            (Vec3::new( 1.0, -1.0,  1.0), [1.0, 0.0]),
            (Vec3::new(-1.0, -1.0,  1.0), [0.0, 0.0]),
        ]),
    ];

    for (normal, color, quad) in faces {
        let start_idx = vertices.len() as u32;
        for (pos, uv) in quad {
            vertices.push(Vertex {
                position: pos.to_array(),
                normal: normal.to_array(),
                tangent: [1.0, 0.0, 0.0, 1.0],
                uv,
                color,
            });
        }
        indices.extend_from_slice(&[
            start_idx, start_idx + 1, start_idx + 2,
            start_idx, start_idx + 2, start_idx + 3,
        ]);
    }

    compute_tangents(&mut vertices, &indices);
    (vertices, indices)
}

pub fn generate_pyramid_data() -> (Vec<Vertex>, Vec<u32>) {
    let mut vertices = Vec::new();
    let mut indices = Vec::new();

    let apex = Vec3::new(0.0, 1.0, 0.0);
    let base = [
        (Vec3::new(-1.0, -1.0,  1.0), [0.0, 1.0]), // 0: avant gauche
        (Vec3::new( 1.0, -1.0,  1.0), [1.0, 1.0]), // 1: avant droit
        (Vec3::new( 1.0, -1.0, -1.0), [1.0, 0.0]), // 2: arrière droit
        (Vec3::new(-1.0, -1.0, -1.0), [0.0, 0.0]), // 3: arrière gauche
    ];

    // Base carrée
    let base_color = [0.93, 0.33, 0.31, 1.0];
    let base_start = vertices.len() as u32;
    for (p, uv) in base {
        vertices.push(Vertex {
            position: p.to_array(),
            normal: [0.0, -1.0, 0.0],
            tangent: [1.0, 0.0, 0.0, 1.0],
            uv,
            color: base_color,
        });
    }
    indices.extend_from_slice(&[
        base_start, base_start + 2, base_start + 1,
        base_start, base_start + 3, base_start + 2,
    ]);

    // 4 côtés triangulaires
    let side_colors = [
        [1.0, 0.79, 0.16, 1.0],
        [0.4, 0.73, 0.42, 1.0],
        [0.26, 0.65, 0.96, 1.0],
        [0.67, 0.28, 0.74, 1.0],
    ];

    for i in 0..4 {
        let (p0, _) = base[i];
        let (p1, _) = base[(i + 1) % 4];
        let normal = (p1 - p0).cross(apex - p0).normalize();
        let color = side_colors[i];

        let start = vertices.len() as u32;
        vertices.push(Vertex { position: p0.to_array(), normal: normal.to_array(), tangent: [1.0, 0.0, 0.0, 1.0], uv: [0.0, 1.0], color });
        vertices.push(Vertex { position: p1.to_array(), normal: normal.to_array(), tangent: [1.0, 0.0, 0.0, 1.0], uv: [1.0, 1.0], color });
        vertices.push(Vertex { position: apex.to_array(), normal: normal.to_array(), tangent: [1.0, 0.0, 0.0, 1.0], uv: [0.5, 0.0], color });

        indices.extend_from_slice(&[start, start + 1, start + 2]);
    }

    compute_tangents(&mut vertices, &indices);
    (vertices, indices)
}

pub fn generate_prism_data() -> (Vec<Vertex>, Vec<u32>) {
    let mut vertices = Vec::new();
    let mut indices = Vec::new();
    let sides = 5;

    let mut bottom_pts = Vec::new();
    let mut top_pts = Vec::new();

    for i in 0..sides {
        let angle = (i as f32) * 2.0 * std::f32::consts::PI / (sides as f32);
        bottom_pts.push(Vec3::new(angle.cos(), -1.0, angle.sin()));
        top_pts.push(Vec3::new(angle.cos(), 1.0, angle.sin()));
    }

    let colors = [
        [0.93, 0.33, 0.31, 1.0],
        [1.0, 0.79, 0.16, 1.0],
        [0.4, 0.73, 0.42, 1.0],
        [0.26, 0.65, 0.96, 1.0],
        [0.67, 0.28, 0.74, 1.0],
    ];

    // Côtés (rectangles)
    for i in 0..sides {
        let next = (i + 1) % sides;
        let p0 = bottom_pts[i];
        let p1 = bottom_pts[next];
        let p2 = top_pts[next];
        let p3 = top_pts[i];
        let normal = (p1 - p0).cross(p3 - p0).normalize();
        let color = colors[i];

        let start = vertices.len() as u32;
        let u0 = (i as f32) / (sides as f32);
        let u1 = ((i + 1) as f32) / (sides as f32);
        vertices.push(Vertex { position: p0.to_array(), normal: normal.to_array(), tangent: [1.0, 0.0, 0.0, 1.0], uv: [u0, 1.0], color });
        vertices.push(Vertex { position: p1.to_array(), normal: normal.to_array(), tangent: [1.0, 0.0, 0.0, 1.0], uv: [u1, 1.0], color });
        vertices.push(Vertex { position: p2.to_array(), normal: normal.to_array(), tangent: [1.0, 0.0, 0.0, 1.0], uv: [u1, 0.0], color });
        vertices.push(Vertex { position: p3.to_array(), normal: normal.to_array(), tangent: [1.0, 0.0, 0.0, 1.0], uv: [u0, 0.0], color });

        indices.extend_from_slice(&[
            start, start + 1, start + 2,
            start, start + 2, start + 3,
        ]);
    }

    compute_tangents(&mut vertices, &indices);
    (vertices, indices)
}

pub fn generate_grid_plane(size: f32, steps: i32) -> (Vec<Vertex>, Vec<u32>) {
    let mut vertices = Vec::new();
    let mut indices = Vec::new();
    let step_size = size / (steps as f32);
    let half_size = size / 2.0;
    let grid_color = [0.2, 0.24, 0.3, 1.0];

    for i in 0..=steps {
        let offset = -half_size + (i as f32) * step_size;
        // Ligne X
        let start_x = vertices.len() as u32;
        vertices.push(Vertex { position: [-half_size, 0.0, offset], normal: [0.0, 1.0, 0.0], tangent: [1.0, 0.0, 0.0, 1.0], uv: [0.0, 0.0], color: grid_color });
        vertices.push(Vertex { position: [ half_size, 0.0, offset], normal: [0.0, 1.0, 0.0], tangent: [1.0, 0.0, 0.0, 1.0], uv: [1.0, 0.0], color: grid_color });
        indices.extend_from_slice(&[start_x, start_x + 1]);

        // Ligne Z
        let start_z = vertices.len() as u32;
        vertices.push(Vertex { position: [offset, 0.0, -half_size], normal: [0.0, 1.0, 0.0], tangent: [0.0, 0.0, 1.0, 1.0], uv: [0.0, 0.0], color: grid_color });
        vertices.push(Vertex { position: [offset, 0.0,  half_size], normal: [0.0, 1.0, 0.0], tangent: [0.0, 0.0, 1.0, 1.0], uv: [0.0, 1.0], color: grid_color });
        indices.extend_from_slice(&[start_z, start_z + 1]);
    }

    (vertices, indices)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_primitive_generators() {
        let (cube_v, cube_i) = generate_cube_data();
        assert_eq!(cube_v.len(), 24);
        assert_eq!(cube_i.len(), 36);
        assert_eq!(cube_v[0].tangent.len(), 4);

        let (pyr_v, pyr_i) = generate_pyramid_data();
        assert_eq!(pyr_v.len(), 16);
        assert_eq!(pyr_i.len(), 18);

        let (prism_v, prism_i) = generate_prism_data();
        assert_eq!(prism_v.len(), 20);
        assert_eq!(prism_i.len(), 30);

        let (grid_v, grid_i) = generate_grid_plane(10.0, 10);
        assert!(!grid_v.is_empty());
        assert!(!grid_i.is_empty());
    }
}
