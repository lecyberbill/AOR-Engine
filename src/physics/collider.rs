// [WFGY] Zone: SAFE | λ: 0.2 | Fallbacks: 0 | Action: Geometric and Metric SI Physics Colliders (Capsule, AABB, Sphere, Plane, Ray-Triangle)
#![allow(dead_code)]
use glam::Vec3;

/// Forme géométrique de collision d'un corps physique
#[derive(Clone, Debug, PartialEq)]
pub enum ColliderShape {
    /// Capsule verticale orientée sur l'axe Y (rayon, hauteur totale)
    Capsule { radius: f32, height: f32 },
    /// Boîte englobante alignée sur les axes (demi-étendues)
    Box { half_extents: Vec3 },
    /// Sphère (rayon)
    Sphere { radius: f32 },
    /// Plan horizontal infini à une hauteur Y donnée
    Plane { y: f32 },
}

/// Collider physique positionné dans le monde métrique SI
#[derive(Clone, Debug)]
pub struct Collider {
    pub position: Vec3,
    pub shape: ColliderShape,
}

impl Collider {
    pub fn new_capsule(position: Vec3, radius: f32, height: f32) -> Self {
        Self {
            position,
            shape: ColliderShape::Capsule { radius, height },
        }
    }

    pub fn new_box(position: Vec3, half_extents: Vec3) -> Self {
        Self {
            position,
            shape: ColliderShape::Box { half_extents },
        }
    }

    pub fn new_sphere(position: Vec3, radius: f32) -> Self {
        Self {
            position,
            shape: ColliderShape::Sphere { radius },
        }
    }

    pub fn new_plane(y: f32) -> Self {
        Self {
            position: Vec3::new(0.0, y, 0.0),
            shape: ColliderShape::Plane { y },
        }
    }

    /// Calcule le point le plus proche sur une boîte AABB
    pub fn closest_point_on_box(box_center: Vec3, half_extents: Vec3, point: Vec3) -> Vec3 {
        let min = box_center - half_extents;
        let max = box_center + half_extents;
        point.clamp(min, max)
    }

    /// Résout la collision entre 2 sphères (retourne le vecteur de pénétration/poussée)
    pub fn collide_spheres(pos_a: Vec3, r_a: f32, pos_b: Vec3, r_b: f32) -> Option<Vec3> {
        let delta = pos_a - pos_b;
        let dist_sq = delta.length_squared();
        let total_radius = r_a + r_b;
        if dist_sq < total_radius * total_radius && dist_sq > 1e-6 {
            let dist = dist_sq.sqrt();
            let normal = delta / dist;
            let penetration = total_radius - dist;
            Some(normal * penetration)
        } else {
            None
        }
    }

    /// Résout la collision entre une sphère et une boîte AABB
    pub fn collide_sphere_box(sphere_pos: Vec3, sphere_radius: f32, box_pos: Vec3, box_half_extents: Vec3) -> Option<Vec3> {
        let closest = Self::closest_point_on_box(box_pos, box_half_extents, sphere_pos);
        let delta = sphere_pos - closest;
        let dist_sq = delta.length_squared();
        if dist_sq < sphere_radius * sphere_radius {
            let dist = dist_sq.sqrt();
            if dist > 1e-5 {
                let normal = delta / dist;
                let penetration = sphere_radius - dist;
                Some(normal * penetration)
            } else {
                Some(Vec3::new(0.0, sphere_radius, 0.0))
            }
        } else {
            None
        }
    }

    /// Résout l'interpénétration entre deux boîtes AABB
    pub fn collide_boxes(pos_a: Vec3, ext_a: Vec3, pos_b: Vec3, ext_b: Vec3) -> Option<Vec3> {
        let delta = pos_a - pos_b;
        let overlap_x = (ext_a.x + ext_b.x) - delta.x.abs();
        let overlap_y = (ext_a.y + ext_b.y) - delta.y.abs();
        let overlap_z = (ext_a.z + ext_b.z) - delta.z.abs();

        if overlap_x > 0.0 && overlap_y > 0.0 && overlap_z > 0.0 {
            if overlap_x < overlap_y && overlap_x < overlap_z {
                Some(Vec3::new(delta.x.signum() * overlap_x, 0.0, 0.0))
            } else if overlap_y < overlap_z {
                Some(Vec3::new(0.0, delta.y.signum() * overlap_y, 0.0))
            } else {
                Some(Vec3::new(0.0, 0.0, delta.z.signum() * overlap_z))
            }
        } else {
            None
        }
    }

    /// Résout l'interpénétration entre une capsule verticale (joueur) et une boîte AABB (obstacle)
    pub fn collide_capsule_box(
        capsule_pos: Vec3,
        capsule_radius: f32,
        capsule_height: f32,
        box_pos: Vec3,
        box_half_extents: Vec3,
    ) -> Option<Vec3> {
        let seg_bottom = capsule_pos + Vec3::new(0.0, capsule_radius, 0.0);
        let seg_top = capsule_pos + Vec3::new(0.0, capsule_height - capsule_radius, 0.0);

        let clamped_y = box_pos.y.clamp(seg_bottom.y, seg_top.y);
        let cap_point = Vec3::new(capsule_pos.x, clamped_y, capsule_pos.z);

        let box_closest = Self::closest_point_on_box(box_pos, box_half_extents, cap_point);
        let delta = cap_point - box_closest;
        let dist_sq = delta.length_squared();

        if dist_sq < capsule_radius * capsule_radius {
            let dist = dist_sq.sqrt();
            if dist > 1e-5 {
                let normal = delta / dist;
                let penetration = capsule_radius - dist;
                Some(normal * penetration)
            } else {
                Some(Vec3::new(0.0, capsule_radius, 0.0))
            }
        } else {
            None
        }
    }

    /// Test d'intersection Rayon-Triangle de Möller-Trumbore (pour Mesh Collider exact)
    pub fn ray_intersect_triangle(
        ray_origin: Vec3,
        ray_dir: Vec3,
        v0: Vec3,
        v1: Vec3,
        v2: Vec3,
    ) -> Option<(f32, Vec3)> {
        let edge1 = v1 - v0;
        let edge2 = v2 - v0;
        let h = ray_dir.cross(edge2);
        let a = edge1.dot(h);

        if a.abs() < 1e-7 {
            return None; // Le rayon est parallèle au triangle
        }

        let f = 1.0 / a;
        let s = ray_origin - v0;
        let u = f * s.dot(h);

        if !(0.0..=1.0).contains(&u) {
            return None;
        }

        let q = s.cross(edge1);
        let v = f * ray_dir.dot(q);

        if v < 0.0 || u + v > 1.0 {
            return None;
        }

        let t = f * edge2.dot(q);
        if t > 1e-6 {
            let hit_pos = ray_origin + ray_dir * t;
            Some((t, hit_pos))
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sphere_sphere_collision() {
        let p1 = Vec3::new(0.0, 0.0, 0.0);
        let p2 = Vec3::new(1.5, 0.0, 0.0);
        let push = Collider::collide_spheres(p1, 1.0, p2, 1.0);
        assert!(push.is_some());
        let p = push.unwrap();
        assert_eq!(p.x, -0.5);
    }

    #[test]
    fn test_ray_triangle_intersection() {
        let origin = Vec3::new(0.0, 2.0, 0.0);
        let dir = Vec3::new(0.0, -1.0, 0.0);
        let v0 = Vec3::new(-1.0, 0.0, -1.0);
        let v1 = Vec3::new(1.0, 0.0, -1.0);
        let v2 = Vec3::new(0.0, 0.0, 1.0);
        let hit = Collider::ray_intersect_triangle(origin, dir, v0, v1, v2);
        assert!(hit.is_some());
        let (t, hit_pos) = hit.unwrap();
        assert_eq!(t, 2.0);
        assert_eq!(hit_pos, Vec3::new(0.0, 0.0, 0.0));
    }
}
