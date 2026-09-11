// [WFGY] Zone: SAFE | λ: 0.25 | Fallbacks: 0 | Action: Procedural Mesh Deformation (Wind swaying, Waves, Bend Deformers)
#![allow(dead_code)]
use glam::Vec3;
use crate::scene::Vertex;

/// Déformateur de maillage procédural temps réel
pub struct MeshDeformer;

impl MeshDeformer {
    /// Applique une déformation de vent oscillant (idéal pour la végétation, arbres, drapeaux)
    pub fn apply_wind(
        vertices: &mut [Vertex],
        base_positions: &[[f32; 3]],
        time: f32,
        wind_strength: f32,
        wind_direction: Vec3,
    ) {
        if vertices.len() != base_positions.len() {
            return;
        }

        let dir = wind_direction.normalize_or_zero();

        for (i, v) in vertices.iter_mut().enumerate() {
            let base_pos = Vec3::from_array(base_positions[i]);
            // Plus le sommet est haut (Y élevé), plus il oscille avec le vent
            let height_factor = (base_pos.y.max(0.0) * 0.5).powf(1.5);
            let wave = (time * 3.0 + base_pos.x * 0.5 + base_pos.z * 0.5).sin();
            let wave_gust = (time * 1.2 + base_pos.z * 0.2).cos() * 0.5;

            let displacement = dir * (wave + wave_gust) * wind_strength * height_factor;
            let new_pos = base_pos + displacement;

            v.position = new_pos.to_array();
        }
    }

    /// Applique une déformation d'ondes sinusoidales (idéal pour l'eau, les membranes, les voiles)
    pub fn apply_waves(
        vertices: &mut [Vertex],
        base_positions: &[[f32; 3]],
        time: f32,
        wave_amplitude: f32,
        wave_frequency: f32,
    ) {
        if vertices.len() != base_positions.len() {
            return;
        }

        for (i, v) in vertices.iter_mut().enumerate() {
            let base_pos = Vec3::from_array(base_positions[i]);
            let dist = (base_pos.x * base_pos.x + base_pos.z * base_pos.z).sqrt();
            let wave_y = (dist * wave_frequency - time * 4.0).sin() * wave_amplitude;

            v.position[1] = base_pos.y + wave_y;
        }
    }

    /// Applique une déformation Squash & Stretch gélatineuse (effet Cartoon / Soft-Body)
    pub fn apply_squash_and_stretch(
        vertices: &mut [Vertex],
        base_positions: &[[f32; 3]],
        time: f32,
        intensity: f32,
    ) {
        if vertices.len() != base_positions.len() {
            return;
        }

        let bounce = (time * 6.0).sin() * intensity;
        let factor_y = 1.0 + bounce;
        // Préservation du volume : si Y s'étire, X et Z s'écrasent, et inversement
        let factor_xz = 1.0 / (factor_y.max(0.1)).sqrt();

        for (i, v) in vertices.iter_mut().enumerate() {
            let base = Vec3::from_array(base_positions[i]);
            v.position[0] = base.x * factor_xz;
            v.position[1] = base.y * factor_y;
            v.position[2] = base.z * factor_xz;
        }
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wind_deformation() {
        let mut vertices = vec![
            Vertex {
                position: [0.0, 4.0, 0.0],
                normal: [0.0, 1.0, 0.0],
                tangent: [1.0, 0.0, 0.0, 1.0],
                uv: [0.0, 0.0],
                color: [1.0, 1.0, 1.0, 1.0],
            }
        ];
        let base_positions = vec![[0.0, 4.0, 0.0]];
        let dir = Vec3::new(1.0, 0.0, 0.0);

        MeshDeformer::apply_wind(&mut vertices, &base_positions, 0.5, 0.2, dir);
        assert_ne!(vertices[0].position[0], 0.0);
    }
}
