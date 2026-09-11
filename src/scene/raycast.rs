// [WFGY] Zone: SAFE | λ: 0.2 | Fallbacks: 0/None | Action: 3D Raycasting and AABB/OBB picking system
#![allow(dead_code)]
use glam::{Mat4, Vec2, Vec3, Vec4};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Ray {
    pub origin: Vec3,
    pub direction: Vec3,
}

impl Ray {
    pub fn new(origin: Vec3, direction: Vec3) -> Self {
        Self {
            origin,
            direction: direction.normalize(),
        }
    }

    /// Point sur le rayon à la distance t
    pub fn at(&self, t: f32) -> Vec3 {
        self.origin + self.direction * t
    }

    /// Intersection avec un plan défini par son origine et sa normale
    pub fn intersect_plane(&self, plane_origin: Vec3, plane_normal: Vec3) -> Option<f32> {
        let denom = plane_normal.dot(self.direction);
        if denom.abs() > 1e-6 {
            let t = (plane_origin - self.origin).dot(plane_normal) / denom;
            if t >= 0.0 {
                return Some(t);
            }
        }
        None
    }

    /// Intersection avec une AABB locale transformée par une matrice Model
    pub fn intersect_obb(&self, aabb: &Aabb, model_matrix: Mat4) -> Option<f32> {
        // Inverser la matrice modèle pour ramener le rayon dans l'espace local de l'AABB
        let inv_model = model_matrix.inverse();
        
        let local_origin = inv_model.transform_point3(self.origin);
        let local_dir = inv_model.transform_vector3(self.direction);
        let dir_len = local_dir.length();
        if dir_len < 1e-6 {
            return None;
        }
        let local_dir = local_dir / dir_len;

        let local_ray = Ray::new(local_origin, local_dir);
        let local_t = local_ray.intersect_aabb(aabb)?;

        // Reconvertir la distance t dans l'espace monde
        Some(local_t / dir_len)
    }

    /// Intersection classique Ray-AABB (Slab Method)
    pub fn intersect_aabb(&self, aabb: &Aabb) -> Option<f32> {
        let mut tmin = (aabb.min.x - self.origin.x) / self.direction.x;
        let mut tmax = (aabb.max.x - self.origin.x) / self.direction.x;

        if tmin > tmax {
            std::mem::swap(&mut tmin, &mut tmax);
        }

        let mut tymin = (aabb.min.y - self.origin.y) / self.direction.y;
        let mut tymax = (aabb.max.y - self.origin.y) / self.direction.y;

        if tymin > tymax {
            std::mem::swap(&mut tymin, &mut tymax);
        }

        if (tmin > tymax) || (tymin > tmax) {
            return None;
        }

        if tymin > tmin {
            tmin = tymin;
        }

        if tymax < tmax {
            tmax = tymax;
        }

        let mut tzmin = (aabb.min.z - self.origin.z) / self.direction.z;
        let mut tzmax = (aabb.max.z - self.origin.z) / self.direction.z;

        if tzmin > tzmax {
            std::mem::swap(&mut tzmin, &mut tzmax);
        }

        if (tmin > tzmax) || (tzmin > tmax) {
            return None;
        }

        if tzmin > tmin {
            tmin = tzmin;
        }

        if tzmax < tmax {
            tmax = tzmax;
        }

        if tmax < 0.0 {
            return None;
        }

        Some(if tmin >= 0.0 { tmin } else { tmax })
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Aabb {
    pub min: Vec3,
    pub max: Vec3,
}

impl Aabb {
    pub fn new(min: Vec3, max: Vec3) -> Self {
        Self { min, max }
    }

    pub fn from_points(points: &[Vec3]) -> Self {
        if points.is_empty() {
            return Self::new(Vec3::splat(-0.5), Vec3::splat(0.5));
        }

        let mut min = points[0];
        let mut max = points[0];

        for &p in points.iter().skip(1) {
            min = min.min(p);
            max = max.max(p);
        }

        // Ajouter une légère marge minimale pour éviter une boîte d'épaisseur nulle
        let size = max - min;
        if size.x < 0.05 { min.x -= 0.025; max.x += 0.025; }
        if size.y < 0.05 { min.y -= 0.025; max.y += 0.025; }
        if size.z < 0.05 { min.z -= 0.025; max.z += 0.025; }

        Self { min, max }
    }

    pub fn center(&self) -> Vec3 {
        (self.min + self.max) * 0.5
    }

    pub fn size(&self) -> Vec3 {
        (self.max - self.min).abs()
    }

    pub fn corners(&self) -> [Vec3; 8] {
        [
            Vec3::new(self.min.x, self.min.y, self.min.z),
            Vec3::new(self.max.x, self.min.y, self.min.z),
            Vec3::new(self.min.x, self.max.y, self.min.z),
            Vec3::new(self.max.x, self.max.y, self.min.z),
            Vec3::new(self.min.x, self.min.y, self.max.z),
            Vec3::new(self.max.x, self.min.y, self.max.z),
            Vec3::new(self.min.x, self.max.y, self.max.z),
            Vec3::new(self.max.x, self.max.y, self.max.z),
        ]
    }

    /// Transforme l'AABB par une matrice 4x4 et recalcule l'englobant aligné
    pub fn transformed(&self, mat: Mat4) -> Self {
        let corners = self.corners();
        let transformed_corners: Vec<Vec3> = corners.iter().map(|&c| mat.transform_point3(c)).collect();
        Self::from_points(&transformed_corners)
    }
}


/// Convertit une coordonnée écran 2D en rayon 3D Monde
/// viewport_rect: [x, y, width, height]
pub fn screen_to_ray(pos: Vec2, viewport_rect: [f32; 4], inv_view_proj: Mat4) -> Ray {
    let [vx, vy, vw, vh] = viewport_rect;
    let vw = if vw <= 0.0 { 1.0 } else { vw };
    let vh = if vh <= 0.0 { 1.0 } else { vh };

    // Normaliser les coordonnées écran dans [-1.0, 1.0] (NDC)
    // Note: Y descend en coordonnées écran (0 en haut), en NDC Y monte (+1 en haut)
    let ndc_x = ((pos.x - vx) / vw) * 2.0 - 1.0;
    let ndc_y = -(((pos.y - vy) / vh) * 2.0 - 1.0);

    // Unproject Near Plane (z = 0.0 en WebGPU / DX12)
    let near_clip = Vec4::new(ndc_x, ndc_y, 0.0, 1.0);
    let near_world = inv_view_proj * near_clip;
    let near_point = near_world.truncate() / near_world.w;

    // Unproject Far Plane (z = 1.0)
    let far_clip = Vec4::new(ndc_x, ndc_y, 1.0, 1.0);
    let far_world = inv_view_proj * far_clip;
    let far_point = far_world.truncate() / far_world.w;

    let dir = (far_point - near_point).normalize();
    Ray::new(near_point, dir)
}

/// Projette un point 3D monde en coordonnées écran 2D
/// viewport_rect: [x, y, width, height]
pub fn world_to_screen(world_pos: Vec3, view_proj: Mat4, viewport_rect: [f32; 4]) -> Option<Vec2> {
    let clip = view_proj * Vec4::new(world_pos.x, world_pos.y, world_pos.z, 1.0);
    if clip.w <= 0.0001 {
        return None; // Derrière la caméra
    }

    let ndc_x = clip.x / clip.w;
    let ndc_y = clip.y / clip.w;

    let [vx, vy, vw, vh] = viewport_rect;
    let screen_x = vx + (ndc_x + 1.0) * 0.5 * vw;
    let screen_y = vy + (1.0 - ndc_y) * 0.5 * vh;

    Some(Vec2::new(screen_x, screen_y))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ray_aabb_intersection() {
        let aabb = Aabb::new(Vec3::splat(-1.0), Vec3::splat(1.0));
        let ray_hit = Ray::new(Vec3::new(0.0, 0.0, 5.0), Vec3::new(0.0, 0.0, -1.0));
        assert!(ray_hit.intersect_aabb(&aabb).is_some());

        let ray_miss = Ray::new(Vec3::new(0.0, 5.0, 5.0), Vec3::new(0.0, 0.0, -1.0));
        assert!(ray_miss.intersect_aabb(&aabb).is_none());
    }
}
