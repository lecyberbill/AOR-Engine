// [WFGY] Zone: SAFE | λ: 0.35 | Fallbacks: 0/None | Action: Cascaded Shadow Maps (CSM) Module with 4 Splits and Sub-texel Stabilization
#![allow(dead_code)]

use glam::{Mat4, Vec3};

pub const NUM_CASCADES: usize = 4;
pub const SHADOW_MAP_SIZE: u32 = 2048;

#[repr(C)]
#[derive(Clone, Copy, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct CsmUniform {
    pub cascade_matrices: [[[f32; 4]; 4]; NUM_CASCADES],
    pub cascade_split_distances: [f32; 4], // Distances linéaires en espace vue
    pub shadow_map_size: f32,
    pub bias_multiplier: f32,
    pub _pad0: f32,
    pub _pad1: f32,
}

impl Default for CsmUniform {
    fn default() -> Self {
        CsmUniform {
            cascade_matrices: [Mat4::IDENTITY.to_cols_array_2d(); NUM_CASCADES],
            cascade_split_distances: [10.0, 28.0, 70.0, 160.0],
            shadow_map_size: SHADOW_MAP_SIZE as f32,
            bias_multiplier: 1.0,
            _pad0: 0.0,
            _pad1: 0.0,
        }
    }
}

/// Structure de gestion des 4 cascades d'ombres directionnelles
pub struct CsmManager {
    pub cascade_splits: [f32; NUM_CASCADES],
    pub light_dir: Vec3,
}

impl CsmManager {
    pub fn new() -> Self {
        Self {
            // Découpage pratique pour un champ de vision standard (10m, 28m, 70m, 160m)
            cascade_splits: [10.0, 28.0, 70.0, 160.0],
            light_dir: Vec3::new(0.4, 0.8, -0.45).normalize(),
        }
    }

    /// Calcule les 4 matrices de projection orthographique stabilisées sub-texel pour chaque tranche de frustum
    pub fn compute_cascade_matrices(
        &self,
        camera: &crate::scene::Camera,
        aspect_ratio: f32,
        light_dir: Vec3,
    ) -> ([Mat4; NUM_CASCADES], [f32; 4]) {
        let light_direction = light_dir.normalize();
        let near_clip = camera.z_near;
        let splits = [
            near_clip,
            self.cascade_splits[0],
            self.cascade_splits[1],
            self.cascade_splits[2],
            self.cascade_splits[3],
        ];

        let mut matrices = [Mat4::IDENTITY; NUM_CASCADES];
        let mut split_dists = [0.0f32; 4];

        for i in 0..NUM_CASCADES {
            let cascade_near = splits[i];
            let cascade_far = splits[i + 1];
            split_dists[i] = cascade_far;

            let cascade_matrix = self.compute_single_cascade_matrix(
                camera,
                aspect_ratio,
                cascade_near,
                cascade_far,
                light_direction,
            );
            matrices[i] = cascade_matrix;
        }

        (matrices, split_dists)
    }

    /// Calcule la boîte englobante orientée lumière avec arrondi aux texels pour éliminer le shimmer
    fn compute_single_cascade_matrix(
        &self,
        camera: &crate::scene::Camera,
        aspect_ratio: f32,
        near: f32,
        far: f32,
        light_dir: Vec3,
    ) -> Mat4 {
        // 1. Calcul des 8 coins du sous-frustum en espace vue puis monde
        let fov_y = camera.fov_y;
        let tan_half_fov = (fov_y * 0.5).tan();

        let near_h = near * tan_half_fov;
        let near_w = near_h * aspect_ratio;
        let far_h = far * tan_half_fov;
        let far_w = far_h * aspect_ratio;

        let corners_view = [
            Vec3::new(-near_w, near_h, -near),
            Vec3::new(near_w, near_h, -near),
            Vec3::new(-near_w, -near_h, -near),
            Vec3::new(near_w, -near_h, -near),
            Vec3::new(-far_w, far_h, -far),
            Vec3::new(far_w, far_h, -far),
            Vec3::new(-far_w, -far_h, -far),
            Vec3::new(far_w, -far_h, -far),
        ];

        let (_, cam_view, _) = camera.build_view_proj_matrix(aspect_ratio);
        let inv_cam_view = cam_view.inverse();

        // 2. Transformer les coins en coordonnées monde et calculer le centre de la sphère englobante
        let mut frustum_center = Vec3::ZERO;
        let mut corners_world = [Vec3::ZERO; 8];
        for (i, corner) in corners_view.iter().enumerate() {
            let world_pt = inv_cam_view.transform_point3(*corner);
            corners_world[i] = world_pt;
            frustum_center += world_pt;
        }
        frustum_center /= 8.0;

        // Calcul du rayon de la sphère circonscrite pour garantir l'invariance en rotation
        let mut radius = 0.0f32;
        for pt in &corners_world {
            let dist = (*pt - frustum_center).length();
            radius = radius.max(dist);
        }
        // Marge de sécurité 10%
        radius = (radius * 1.1).ceil();

        // 3. Construction de la vue lumière
        let light_up = if light_dir.dot(Vec3::Y).abs() > 0.99 {
            Vec3::Z
        } else {
            Vec3::Y
        };

        // Positionner la caméra d'ombre en amont du centre du sous-frustum
        let light_cam_pos = frustum_center + light_dir * (radius * 2.0);
        let light_view = Mat4::look_at_rh(light_cam_pos, frustum_center, light_up);

        // 4. Bounding box dans l'espace vue lumière
        let mut min_x = f32::MAX;
        let mut max_x = f32::MIN;
        let mut min_y = f32::MAX;
        let mut max_y = f32::MIN;
        let mut min_z = f32::MAX;
        let mut max_z = f32::MIN;

        for pt in &corners_world {
            let light_space_pt = light_view.transform_point3(*pt);
            min_x = min_x.min(light_space_pt.x);
            max_x = max_x.max(light_space_pt.x);
            min_y = min_y.min(light_space_pt.y);
            max_y = max_y.max(light_space_pt.y);
            min_z = min_z.min(light_space_pt.z);
            max_z = max_z.max(light_space_pt.z);
        }

        // Étendre Z pour inclure les occluseurs potentiels derrière la caméra
        let z_mult = 3.0;
        let near_plane = if min_z < 0.0 { min_z * z_mult } else { min_z / z_mult };
        let far_plane = if max_z > 0.0 { max_z * z_mult } else { max_z / z_mult };

        // 5. Stabilisation Sub-texel (Snap to Texel Grid)
        let world_units_per_texel_x = (max_x - min_x) / (SHADOW_MAP_SIZE as f32);
        let world_units_per_texel_y = (max_y - min_y) / (SHADOW_MAP_SIZE as f32);

        min_x = (min_x / world_units_per_texel_x).floor() * world_units_per_texel_x;
        max_x = (max_x / world_units_per_texel_x).floor() * world_units_per_texel_x;
        min_y = (min_y / world_units_per_texel_y).floor() * world_units_per_texel_y;
        max_y = (max_y / world_units_per_texel_y).floor() * world_units_per_texel_y;

        let light_proj = Mat4::orthographic_rh(min_x, max_x, min_y, max_y, -far_plane.abs() - (radius * 2.0), far_plane.abs() + (radius * 2.0) - near_plane.min(0.0));

        light_proj * light_view
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scene::Camera;

    #[test]
    fn test_csm_uniform_size_and_alignment() {
        assert_eq!(std::mem::size_of::<CsmUniform>() % 16, 0);
        assert_eq!(std::mem::size_of::<CsmUniform>(), 288); // 4 * 64 + 16 + 16 = 288
    }

    #[test]
    fn test_csm_cascade_splits_ordering() {
        let manager = CsmManager::new();
        assert!(manager.cascade_splits[0] < manager.cascade_splits[1]);
        assert!(manager.cascade_splits[1] < manager.cascade_splits[2]);
        assert!(manager.cascade_splits[2] < manager.cascade_splits[3]);
    }

    #[test]
    fn test_csm_matrices_computation() {
        let manager = CsmManager::new();
        let camera = Camera::new(Vec3::ZERO, 10.0, 0.5, 0.3);
        let (matrices, splits) = manager.compute_cascade_matrices(&camera, 16.0 / 9.0, manager.light_dir);

        assert_eq!(matrices.len(), NUM_CASCADES);
        assert_eq!(splits.len(), NUM_CASCADES);
        for mat in &matrices {
            assert!(!mat.is_nan());
        }
    }
}
