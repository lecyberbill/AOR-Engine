// [WFGY] Zone: SAFE | λ: 0.25 | Fallbacks: 0 | Action: Frustum Culling and View Frustum Plane extraction
#![allow(dead_code)]
use glam::{Mat4, Vec3, Vec4};
use crate::scene::raycast::Aabb;

/// Plan 3D de l'espace défini par l'équation ax + by + cz + d = 0
#[derive(Clone, Copy, Debug)]
pub struct Plane {
    pub normal: Vec3,
    pub distance: f32,
}

impl Plane {
    pub fn new(a: f32, b: f32, c: f32, d: f32) -> Self {
        let n = Vec3::new(a, b, c);
        let len = n.length();
        if len > 1e-6 {
            Self {
                normal: n / len,
                distance: d / len,
            }
        } else {
            Self {
                normal: Vec3::Y,
                distance: 0.0,
            }
        }
    }

    /// Distance signée d'un point au plan
    pub fn distance_to_point(&self, point: Vec3) -> f32 {
        self.normal.dot(point) + self.distance
    }
}

/// Volume de vue pyramidal de la caméra (Frustum) composé de 6 plans
#[derive(Clone, Copy, Debug)]
pub struct Frustum {
    pub planes: [Plane; 6], // Left, Right, Bottom, Top, Near, Far
}

impl Frustum {
    /// Extrait les 6 plans de vue à partir de la matrice View-Projection combinée
    pub fn from_view_projection_matrix(m: Mat4) -> Self {
        let r0 = Vec4::new(m.x_axis.x, m.y_axis.x, m.z_axis.x, m.w_axis.x);
        let r1 = Vec4::new(m.x_axis.y, m.y_axis.y, m.z_axis.y, m.w_axis.y);
        let r2 = Vec4::new(m.x_axis.z, m.y_axis.z, m.z_axis.z, m.w_axis.z);
        let r3 = Vec4::new(m.x_axis.w, m.y_axis.w, m.z_axis.w, m.w_axis.w);

        // Left Plane: R3 + R0
        let left = Plane::new(r3.x + r0.x, r3.y + r0.y, r3.z + r0.z, r3.w + r0.w);
        // Right Plane: R3 - R0
        let right = Plane::new(r3.x - r0.x, r3.y - r0.y, r3.z - r0.z, r3.w - r0.w);
        // Bottom Plane: R3 + R1
        let bottom = Plane::new(r3.x + r1.x, r3.y + r1.y, r3.z + r1.z, r3.w + r1.w);
        // Top Plane: R3 - R1
        let top = Plane::new(r3.x - r1.x, r3.y - r1.y, r3.z - r1.z, r3.w - r1.w);
        // Near Plane: R2
        let near = Plane::new(r2.x, r2.y, r2.z, r2.w);
        // Far Plane: R3 - R2
        let far = Plane::new(r3.x - r2.x, r3.y - r2.y, r3.z - r2.z, r3.w - r2.w);

        Self {
            planes: [left, right, bottom, top, near, far],
        }
    }

    /// Teste si une boîte englobante alignée sur les axes (AABB) est visible dans le champ de vision
    pub fn intersects_aabb(&self, aabb: &Aabb) -> bool {
        for plane in &self.planes {
            // Trouver le point le plus favorable de l'AABB le long de la normale du plan
            let p = Vec3::new(
                if plane.normal.x >= 0.0 { aabb.max.x } else { aabb.min.x },
                if plane.normal.y >= 0.0 { aabb.max.y } else { aabb.min.y },
                if plane.normal.z >= 0.0 { aabb.max.z } else { aabb.min.z },
            );

            // Si le point le plus avancé est derrière le plan, toute la boîte est hors champ !
            if plane.distance_to_point(p) < 0.0 {
                return false;
            }
        }

        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_frustum_aabb_intersection() {
        let view = Mat4::look_at_rh(Vec3::new(0.0, 0.0, 10.0), Vec3::ZERO, Vec3::Y);
        let proj = Mat4::perspective_rh(std::f32::consts::PI / 4.0, 1.0, 0.1, 100.0);
        let frustum = Frustum::from_view_projection_matrix(proj * view);

        let inside_box = Aabb::new(Vec3::new(-1.0, -1.0, -1.0), Vec3::new(1.0, 1.0, 1.0));
        assert!(frustum.intersects_aabb(&inside_box), "Objet au centre doit être visible");

        let behind_box = Aabb::new(Vec3::new(-1.0, -1.0, 20.0), Vec3::new(1.0, 1.0, 22.0));
        assert!(!frustum.intersects_aabb(&behind_box), "Objet derrière la caméra ne doit pas être visible");
    }
}
