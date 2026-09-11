use glam::Vec3;
use crate::scene::mesh::Vertex;

#[derive(Clone, Debug)]
pub struct WeaponState {
    pub name: String,
    pub ammo: u32,
    pub max_ammo: u32,
    pub damage_per_pellet: f32,
    pub pellet_count: u32,
    pub spread: f32,
    pub recoil_time: f32,
    pub recoil_duration: f32,
    pub muzzle_flash_time: f32,
    pub fire_cooldown: f32,
    pub current_cooldown: f32,
}

impl Default for WeaponState {
    fn default() -> Self {
        Self {
            name: "Combat Shotgun 12-Gauge".to_string(),
            ammo: 32,
            max_ammo: 64,
            damage_per_pellet: 18.0,
            pellet_count: 8,
            spread: 0.045,
            recoil_time: 0.0,
            recoil_duration: 0.35,
            muzzle_flash_time: 0.0,
            fire_cooldown: 0.75,
            current_cooldown: 0.0,
        }
    }
}

impl WeaponState {
    pub fn new_shotgun() -> Self {
        Self::default()
    }

    pub fn update(&mut self, dt: f32) {
        if self.current_cooldown > 0.0 {
            self.current_cooldown = (self.current_cooldown - dt).max(0.0);
        }
        if self.recoil_time > 0.0 {
            self.recoil_time = (self.recoil_time - dt).max(0.0);
        }
        if self.muzzle_flash_time > 0.0 {
            self.muzzle_flash_time = (self.muzzle_flash_time - dt).max(0.0);
        }
    }

    pub fn can_fire(&self) -> bool {
        self.current_cooldown <= 0.0 && self.ammo > 0
    }

    pub fn fire(&mut self) -> bool {
        if !self.can_fire() {
            return false;
        }
        self.ammo -= 1;
        self.current_cooldown = self.fire_cooldown;
        self.recoil_time = self.recoil_duration;
        self.muzzle_flash_time = 0.08;
        true
    }

    /// Calcule la transformation locale du fusil à pompe relative à la caméra (avec recul)
    pub fn compute_view_offset(&self) -> Vec3 {
        // Position de base : centré au milieu de l'écran comme dans le DOOM original (1993)
        let base_offset = Vec3::new(0.0, -0.18, 0.45);

        if self.recoil_time > 0.0 {
            let t = self.recoil_time / self.recoil_duration;
            let kick_back = t.sin() * 0.08;
            let kick_up = t.sin() * 0.05;
            base_offset + Vec3::new(0.0, kick_up, -kick_back)
        } else {
            base_offset
        }
    }
}

/// Génère le maillage 3D procédural du fusil à pompe classique (Canon double, pompe, carcasse, poignée)
pub fn generate_shotgun_mesh() -> (Vec<Vertex>, Vec<u32>) {
    let mut vertices = Vec::new();
    let mut indices = Vec::new();

    let mut add_box = |min: Vec3, max: Vec3, color: [f32; 4], _rough: f32| {
        let base_idx = vertices.len() as u32;

        let corners = [
            // Face avant (+Z)
            (Vec3::new(min.x, min.y, max.z), Vec3::Z, [0.0, 1.0]),
            (Vec3::new(max.x, min.y, max.z), Vec3::Z, [1.0, 1.0]),
            (Vec3::new(max.x, max.y, max.z), Vec3::Z, [1.0, 0.0]),
            (Vec3::new(min.x, max.y, max.z), Vec3::Z, [0.0, 0.0]),
            // Face arrière (-Z)
            (Vec3::new(max.x, min.y, min.z), -Vec3::Z, [0.0, 1.0]),
            (Vec3::new(min.x, min.y, min.z), -Vec3::Z, [1.0, 1.0]),
            (Vec3::new(min.x, max.y, min.z), -Vec3::Z, [1.0, 0.0]),
            (Vec3::new(max.x, max.y, min.z), -Vec3::Z, [0.0, 0.0]),
            // Face gauche (-X)
            (Vec3::new(min.x, min.y, min.z), -Vec3::X, [0.0, 1.0]),
            (Vec3::new(min.x, min.y, max.z), -Vec3::X, [1.0, 1.0]),
            (Vec3::new(min.x, max.y, max.z), -Vec3::X, [1.0, 0.0]),
            (Vec3::new(min.x, max.y, min.z), -Vec3::X, [0.0, 0.0]),
            // Face droite (+X)
            (Vec3::new(max.x, min.y, max.z), Vec3::X, [0.0, 1.0]),
            (Vec3::new(max.x, min.y, min.z), Vec3::X, [1.0, 1.0]),
            (Vec3::new(max.x, max.y, min.z), Vec3::X, [1.0, 0.0]),
            (Vec3::new(max.x, max.y, max.z), Vec3::X, [0.0, 0.0]),
            // Face haut (+Y)
            (Vec3::new(min.x, max.y, max.z), Vec3::Y, [0.0, 1.0]),
            (Vec3::new(max.x, max.y, max.z), Vec3::Y, [1.0, 1.0]),
            (Vec3::new(max.x, max.y, min.z), Vec3::Y, [1.0, 0.0]),
            (Vec3::new(min.x, max.y, min.z), Vec3::Y, [0.0, 0.0]),
            // Face bas (-Y)
            (Vec3::new(min.x, min.y, min.z), -Vec3::Y, [0.0, 1.0]),
            (Vec3::new(max.x, min.y, min.z), -Vec3::Y, [1.0, 1.0]),
            (Vec3::new(max.x, min.y, max.z), -Vec3::Y, [1.0, 0.0]),
            (Vec3::new(min.x, min.y, max.z), -Vec3::Y, [0.0, 0.0]),
        ];

        for (pos, norm, uv) in corners {
            vertices.push(Vertex {
                position: pos.to_array(),
                normal: norm.to_array(),
                tangent: [1.0, 0.0, 0.0, 1.0],
                uv,
                color,
            });
        }

        for f in 0..6 {
            let offset = base_idx + f * 4;
            indices.extend_from_slice(&[
                offset, offset + 1, offset + 2,
                offset, offset + 2, offset + 3,
            ]);
        }
    };

    // 1. Carcasse principale (métal argenté usiné)
    add_box(
        Vec3::new(-0.040, -0.050, -0.22),
        Vec3::new(0.040, 0.050, 0.14),
        [0.48, 0.50, 0.54, 1.0],
        0.20,
    );

    // 2. Canons jumelés de tir (acier trempé bleuté)
    add_box(
        Vec3::new(-0.024, 0.015, 0.14),
        Vec3::new(0.024, 0.048, 0.76),
        [0.32, 0.35, 0.40, 1.0],
        0.15,
    );

    // 3. Tube magasin inférieur (acier brossé)
    add_box(
        Vec3::new(-0.022, -0.030, 0.14),
        Vec3::new(0.022, 0.005, 0.68),
        [0.40, 0.42, 0.45, 1.0],
        0.20,
    );

    // 4. Pompe mobile (bois de noyer verni chaud classique DOOM)
    add_box(
        Vec3::new(-0.036, -0.040, 0.26),
        Vec3::new(0.036, 0.015, 0.50),
        [0.55, 0.30, 0.15, 1.0],
        0.35,
    );

    // 5. Crosse et poignée ergonomique (bois sombre)
    add_box(
        Vec3::new(-0.032, -0.14, -0.38),
        Vec3::new(0.032, -0.02, -0.20),
        [0.45, 0.24, 0.12, 1.0],
        0.40,
    );

    // 6. Cran de mire avant (rouge vif phosphorescent)
    add_box(
        Vec3::new(-0.006, 0.048, 0.72),
        Vec3::new(0.006, 0.065, 0.75),
        [1.0, 0.15, 0.05, 1.0],
        0.05,
    );

    crate::scene::mesh::compute_tangents(&mut vertices, &indices);
    (vertices, indices)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_shotgun_mesh_generation() {
        let (verts, inds) = generate_shotgun_mesh();
        assert!(!verts.is_empty());
        assert!(!inds.is_empty());
        assert_eq!(inds.len() % 3, 0);
    }

    #[test]
    fn test_weapon_firing_and_cooldown() {
        let mut w = WeaponState::new_shotgun();
        assert!(w.can_fire());
        assert!(w.fire());
        assert_eq!(w.ammo, 31);
        assert!(!w.can_fire());

        w.update(0.8);
        assert!(w.can_fire());
    }
}
