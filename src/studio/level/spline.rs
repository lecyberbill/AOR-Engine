// [WFGY] Zone: SAFE | λ: 0.25 | Fallbacks: 0/None | Action: 3D Bezier/Catmull-Rom splines & procedural tunnel/road mesh generation
#![allow(dead_code)]
use crate::scene::mesh::Vertex;
use glam::Vec3;

/// Courbe 3D à points de contrôle (Catmull-Rom, ouverte ou fermée)
#[derive(Debug, Clone, PartialEq)]
pub struct Spline {
    pub control_points: Vec<Vec3>,
    pub closed: bool,
}

impl Spline {
    pub fn new(control_points: Vec<Vec3>, closed: bool) -> Self {
        Spline {
            control_points,
            closed,
        }
    }

    pub fn is_valid(&self) -> bool {
        self.control_points.len() >= 2
    }

    fn point_at(&self, idx: isize) -> Vec3 {
        let n = self.control_points.len() as isize;
        if n == 0 {
            return Vec3::ZERO;
        }
        if self.closed {
            let i = ((idx % n) + n) % n;
            self.control_points[i as usize]
        } else {
            let i = idx.clamp(0, n - 1);
            self.control_points[i as usize]
        }
    }

    /// Évalue la position sur la spline pour t dans [0, 1]
    pub fn sample(&self, t: f32) -> Vec3 {
        if !self.is_valid() {
            return self.control_points.first().copied().unwrap_or(Vec3::ZERO);
        }
        let t = t.clamp(0.0, 1.0);
        let segments = if self.closed {
            self.control_points.len()
        } else {
            self.control_points.len() - 1
        } as f32;
        let scaled = t * segments;
        let i = scaled.floor() as isize;
        let local = scaled - i as f32;

        let p0 = self.point_at(i - 1);
        let p1 = self.point_at(i);
        let p2 = self.point_at(i + 1);
        let p3 = self.point_at(i + 2);

        catmull_rom(p0, p1, p2, p3, local)
    }

    /// Dérivée numérique (tangente normalisée) à t
    pub fn tangent(&self, t: f32) -> Vec3 {
        let eps = 1e-3;
        let a = self.sample((t - eps).max(0.0));
        let b = self.sample((t + eps).min(1.0));
        (b - a).normalize_or_zero()
    }

    /// Échantillons réguliers le long de la courbe
    pub fn sample_uniform(&self, count: usize) -> Vec<Vec3> {
        let count = count.max(2);
        (0..count)
            .map(|i| self.sample(i as f32 / (count - 1) as f32))
            .collect()
    }

    /// Longueur approchée par intégration numérique
    pub fn approximate_length(&self, samples: usize) -> f32 {
        let pts = self.sample_uniform(samples.max(2));
        pts.windows(2).map(|w| w[0].distance(w[1])).sum()
    }
}

/// Interpolation Catmull-Rom d'un segment
pub fn catmull_rom(p0: Vec3, p1: Vec3, p2: Vec3, p3: Vec3, t: f32) -> Vec3 {
    let t2 = t * t;
    let t3 = t2 * t;
    0.5 * ((2.0 * p1)
        + (-p0 + p2) * t
        + (2.0 * p0 - 5.0 * p1 + 4.0 * p2 - p3) * t2
        + (-p0 + 3.0 * p1 - 3.0 * p2 + p3) * t3)
}

/// Cadre de Frenet parallèle transporté le long des tangentes
fn parallel_frames(tangents: &[Vec3]) -> Vec<(Vec3, Vec3)> {
    let mut frames = Vec::with_capacity(tangents.len());
    if tangents.is_empty() {
        return frames;
    }
    let mut normal = if tangents[0].x.abs() < 0.9 {
        Vec3::X
    } else {
        Vec3::Y
    };
    normal = (normal - tangents[0] * normal.dot(tangents[0])).normalize_or_zero();
    for tan in tangents {
        normal = (normal - *tan * normal.dot(*tan)).normalize_or_zero();
        if normal.length_squared() < 1e-6 {
            normal = if tan.x.abs() < 0.9 { Vec3::X } else { Vec3::Y };
            normal = (normal - *tan * normal.dot(*tan)).normalize_or_zero();
        }
        let binormal = tan.cross(normal).normalize_or_zero();
        frames.push((normal, binormal));
    }
    frames
}

/// Génère un maillage de tunnel tubulaire le long d'une spline
pub fn generate_tube_mesh(
    spline: &Spline,
    radius: f32,
    segments: usize,
    sides: usize,
) -> (Vec<Vertex>, Vec<u32>) {
    let segments = segments.max(2);
    let sides = sides.max(3);
    let pts = spline.sample_uniform(segments + 1);
    let tangents: Vec<Vec3> = (0..=segments)
        .map(|i| spline.tangent(i as f32 / segments as f32))
        .collect();
    let frames = parallel_frames(&tangents);

    let mut vertices = Vec::with_capacity((segments + 1) * sides);
    for (i, p) in pts.iter().enumerate() {
        let (normal, binormal) = frames[i];
        for s in 0..sides {
            let angle = (s as f32 / sides as f32) * std::f32::consts::TAU;
            let ring = normal * angle.cos() + binormal * angle.sin();
            let pos = *p + ring * radius;
            let outward = ring;
            let uv = [s as f32 / sides as f32, i as f32 / segments as f32];
            vertices.push(Vertex {
                position: pos.to_array(),
                normal: outward.to_array(),
                tangent: [tangents[i].x, tangents[i].y, tangents[i].z, 1.0],
                uv,
                color: [1.0, 1.0, 1.0, 1.0],
            });
        }
    }

    let mut indices = Vec::with_capacity(segments * sides * 6);
    for i in 0..segments {
        for s in 0..sides {
            let a = (i * sides + s) as u32;
            let b = (i * sides + (s + 1) % sides) as u32;
            let c = ((i + 1) * sides + s) as u32;
            let d = ((i + 1) * sides + (s + 1) % sides) as u32;
            indices.extend_from_slice(&[a, c, b, b, c, d]);
        }
    }

    (vertices, indices)
}

/// Génère un ruban plat (route/piste) le long d'une spline
pub fn generate_ribbon_mesh(spline: &Spline, width: f32, segments: usize) -> (Vec<Vertex>, Vec<u32>) {
    let segments = segments.max(2);
    let pts = spline.sample_uniform(segments + 1);
    let half = width * 0.5;
    let up = Vec3::Y;

    let mut vertices = Vec::with_capacity((segments + 1) * 2);
    for (i, p) in pts.iter().enumerate() {
        let tan = spline.tangent(i as f32 / segments as f32);
        let right = tan.cross(up).normalize_or_zero();
        for (side, sign) in [(0usize, -1.0f32), (1usize, 1.0f32)] {
            let pos = *p + right * half * sign;
            vertices.push(Vertex {
                position: pos.to_array(),
                normal: up.to_array(),
                tangent: [tan.x, tan.y, tan.z, 1.0],
                uv: [side as f32, i as f32 / segments as f32],
                color: [1.0, 1.0, 1.0, 1.0],
            });
        }
    }

    let mut indices = Vec::with_capacity(segments * 6);
    for i in 0..segments {
        let a = (i * 2) as u32;
        let b = (i * 2 + 1) as u32;
        let c = ((i + 1) * 2) as u32;
        let d = ((i + 1) * 2 + 1) as u32;
        indices.extend_from_slice(&[a, c, b, b, c, d]);
    }

    (vertices, indices)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_spline() -> Spline {
        Spline::new(
            vec![
                Vec3::new(0.0, 0.0, 0.0),
                Vec3::new(0.0, 0.0, 10.0),
                Vec3::new(10.0, 0.0, 10.0),
                Vec3::new(10.0, 0.0, 0.0),
            ],
            false,
        )
    }

    #[test]
    fn test_sample_endpoints() {
        let s = test_spline();
        assert!((s.sample(0.0) - Vec3::ZERO).length() < 1e-3);
        assert!((s.sample(1.0) - Vec3::new(10.0, 0.0, 0.0)).length() < 1e-3);
    }

    #[test]
    fn test_length_greater_than_chord() {
        let s = test_spline();
        let chord = s.control_points.first().unwrap().distance(*s.control_points.last().unwrap());
        assert!(s.approximate_length(64) > chord);
    }

    #[test]
    fn test_tube_mesh_counts() {
        let s = test_spline();
        let (verts, idx) = generate_tube_mesh(&s, 1.5, 20, 8);
        assert_eq!(verts.len(), 21 * 8);
        assert_eq!(idx.len(), 20 * 8 * 6);
    }

    #[test]
    fn test_ribbon_mesh_counts() {
        let s = test_spline();
        let (verts, idx) = generate_ribbon_mesh(&s, 4.0, 16);
        assert_eq!(verts.len(), 17 * 2);
        assert_eq!(idx.len(), 16 * 6);
    }

    #[test]
    fn test_closed_spline_wraps() {
        let s = Spline::new(
            vec![
                Vec3::new(0.0, 0.0, 0.0),
                Vec3::new(5.0, 0.0, 0.0),
                Vec3::new(5.0, 0.0, 5.0),
                Vec3::new(0.0, 0.0, 5.0),
            ],
            true,
        );
        assert!((s.sample(0.0) - s.sample(1.0)).length() < 1e-3);
    }
}
