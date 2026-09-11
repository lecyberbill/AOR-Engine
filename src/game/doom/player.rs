// [WFGY] Zone: SAFE | λ: 0.25 | Fallbacks: 0/None | Action: FPS Player Controller with Physics and Gun Rig
use glam::Vec3;
use crate::scene::Camera;
use super::weapon::WeaponState;

#[derive(Clone, Debug)]
pub struct FpsPlayer {
    pub position: Vec3,
    pub velocity: Vec3,
    pub yaw: f32,
    pub pitch: f32,
    pub height: f32,
    pub speed: f32,
    pub sprint_multiplier: f32,
    pub is_grounded: bool,
    pub health: i32,
    pub armor: i32,
    pub weapon: WeaponState,
    pub headbob_time: f32,
}

impl Default for FpsPlayer {
    fn default() -> Self {
        Self {
            position: Vec3::new(0.0, 1.7, 0.0),
            velocity: Vec3::ZERO,
            yaw: 0.0,
            pitch: 0.0,
            height: 1.7,
            speed: 7.5,
            sprint_multiplier: 1.6,
            is_grounded: true,
            health: 100,
            armor: 50,
            weapon: WeaponState::new_shotgun(),
            headbob_time: 0.0,
        }
    }
}

impl FpsPlayer {
    pub fn new(spawn_pos: Vec3) -> Self {
        let mut p = Self::default();
        p.position = spawn_pos;
        p
    }

    pub fn update(
        &mut self,
        dt: f32,
        input_forward: f32,
        input_strafe: f32,
        is_sprinting: bool,
        mouse_dx: f32,
        mouse_dy: f32,
    ) {
        // Rotation souris
        let sens = 0.0022;
        self.yaw += mouse_dx * sens;
        let max_pitch = 85.0_f32.to_radians();
        self.pitch = (self.pitch - mouse_dy * sens).clamp(-max_pitch, max_pitch);

        // Vecteurs de direction au sol (plan XZ)
        let forward = Vec3::new(self.yaw.sin(), 0.0, -self.yaw.cos()).normalize_or_zero();
        let right = Vec3::new(self.yaw.cos(), 0.0, self.yaw.sin()).normalize_or_zero();

        let move_dir = (forward * input_forward + right * input_strafe).normalize_or_zero();
        let current_speed = if is_sprinting {
            self.speed * self.sprint_multiplier
        } else {
            self.speed
        };

        // Amortissement et accélération
        let target_vel = move_dir * current_speed;
        let lerp_factor = (dt * 14.0).min(1.0);
        self.velocity.x += (target_vel.x - self.velocity.x) * lerp_factor;
        self.velocity.z += (target_vel.z - self.velocity.z) * lerp_factor;

        // Déplacement effectif
        self.position += self.velocity * dt;

        // Headbobbing subtil pendant la marche
        let horiz_speed = Vec3::new(self.velocity.x, 0.0, self.velocity.z).length();
        if horiz_speed > 0.5 {
            self.headbob_time += dt * (horiz_speed * 1.8);
        }

        self.weapon.update(dt);
    }

    /// Synchronise la caméra du moteur avec la tête du joueur
    pub fn sync_camera(&self, camera: &mut Camera) {
        let bob_offset = (self.headbob_time * 2.0).sin() * 0.035;
        let eye_pos = self.position + Vec3::new(0.0, bob_offset, 0.0);

        camera.mode = crate::scene::CameraMode::FirstPerson;
        camera.target = eye_pos;
        camera.yaw = self.yaw;
        camera.pitch = self.pitch;
    }

    /// Calcule la position monde et l'orientation du fusil à pompe
    pub fn compute_weapon_world_transform(&self) -> (Vec3, glam::Quat) {
        let forward = Vec3::new(
            self.pitch.cos() * self.yaw.sin(),
            self.pitch.sin(),
            -self.pitch.cos() * self.yaw.cos(),
        ).normalize_or_zero();

        // Dans un repère main-droite avec look_at_rh: right = forward.cross(Vec3::Y).normalize()
        let right = forward.cross(Vec3::Y).normalize_or_zero();
        let up = right.cross(forward).normalize_or_zero();

        let local_offset = self.weapon.compute_view_offset();
        // Le fusil est placé devant les yeux du joueur le long de `forward`
        let world_pos = self.position
            + right * local_offset.x
            + up * local_offset.y
            + forward * local_offset.z;

        let rot = glam::Quat::from_axis_angle(Vec3::Y, -self.yaw + std::f32::consts::PI) * glam::Quat::from_axis_angle(Vec3::X, self.pitch);

        (world_pos, rot)
    }

    pub fn take_damage(&mut self, amount: i32) {
        let absorbed_by_armor = (amount as f32 * 0.4).round() as i32;
        let actual_armor_damage = absorbed_by_armor.min(self.armor);
        self.armor -= actual_armor_damage;

        let remaining = amount - actual_armor_damage;
        self.health = (self.health - remaining).max(0);
    }

    pub fn is_alive(&self) -> bool {
        self.health > 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_player_movement_and_damage() {
        let mut player = FpsPlayer::new(Vec3::new(0.0, 1.7, 0.0));
        assert_eq!(player.health, 100);

        player.update(0.1, 1.0, 0.0, false, 0.0, 0.0);
        assert_ne!(player.position, Vec3::new(0.0, 1.7, 0.0));

        player.take_damage(40);
        assert!(player.is_alive());
        assert!(player.health < 100);
    }
}
