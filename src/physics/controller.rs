// [WFGY] Zone: SAFE | λ: 0.25 | Fallbacks: 0 | Action: Metric SI Kinematic Character Controller (FPS/TPS, ballistic jump, ground detection)
#![allow(dead_code)]
use glam::Vec3;
use crate::physics::collider::Collider;

/// État du joueur
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PlayerMovementState {
    Idle,
    Walking,
    Sprinting,
    Crouching,
    InAir,
}

/// Contrôleur Joueur en unités métriques réelles SI ($1.0 = 1.0\text{ m}$)
#[derive(Clone, Debug)]
pub struct CharacterController {
    pub position: Vec3,
    pub velocity: Vec3,
    pub yaw: f32,   // Rotation horizontale (radians)
    pub pitch: f32, // Inclinaison verticale du regard (radians)

    // Dimensions métriques de la capsule
    pub standing_height: f32, // 1.80 m (taille adulte standard)
    pub crouch_height: f32,   // 1.10 m (hauteur accroupi)
    pub current_height: f32,
    pub radius: f32,          // 0.35 m (rayon du corps)
    pub eye_offset: f32,      // 0.10 m sous le sommet de la capsule

    // Constantes physiques métriques SI
    pub gravity: f32,         // 9.81 m/s²
    pub walk_speed: f32,      // 4.5 m/s (16.2 km/h)
    pub sprint_speed: f32,    // 8.0 m/s (28.8 km/h)
    pub crouch_speed: f32,    // 2.0 m/s (7.2 km/h)
    pub jump_height: f32,     // 1.20 m (hauteur de saut net)
    pub ground_friction: f32, // Facteur de décélération au sol
    pub air_control: f32,     // Facteur de contrôle dans les airs

    // États
    pub is_grounded: bool,
    pub is_crouching: bool,
    pub is_sprinting: bool,
    pub state: PlayerMovementState,
}

impl Default for CharacterController {
    fn default() -> Self {
        Self {
            position: Vec3::new(0.0, 0.0, 4.0),
            velocity: Vec3::ZERO,
            yaw: 0.0,
            pitch: 0.0,
            standing_height: 1.80,
            crouch_height: 1.10,
            current_height: 1.80,
            radius: 0.35,
            eye_offset: 0.10,
            gravity: 9.81,
            walk_speed: 4.5,
            sprint_speed: 8.0,
            crouch_speed: 2.0,
            jump_height: 1.20,
            ground_friction: 12.0,
            air_control: 0.35,
            is_grounded: true,
            is_crouching: false,
            is_sprinting: false,
            state: PlayerMovementState::Idle,
        }
    }
}

impl CharacterController {
    pub fn new(spawn_pos: Vec3) -> Self {
        let mut controller = Self::default();
        controller.position = spawn_pos;
        controller.is_grounded = spawn_pos.y <= 0.05;
        controller
    }

    /// Position exacte des yeux de la caméra (en mètres)
    pub fn eye_position(&self) -> Vec3 {
        self.position + Vec3::new(0.0, self.current_height - self.eye_offset, 0.0)
    }

    /// Vecteur directionnel avant horizontal (Forward XZ)
    pub fn forward_vector(&self) -> Vec3 {
        Vec3::new(self.yaw.sin(), 0.0, -self.yaw.cos()).normalize_or_zero()
    }

    /// Vecteur directionnel droite horizontal (Right XZ)
    pub fn right_vector(&self) -> Vec3 {
        Vec3::new(self.yaw.cos(), 0.0, self.yaw.sin()).normalize_or_zero()
    }

    /// Vecteur de regard 3D complet (Look vector avec pitch et yaw)
    pub fn look_direction(&self) -> Vec3 {
        let cos_p = self.pitch.cos();
        Vec3::new(
            cos_p * self.yaw.sin(),
            self.pitch.sin(),
            -cos_p * self.yaw.cos(),
        ).normalize_or_zero()
    }

    /// Applique la rotation du regard à la souris
    pub fn rotate_look(&mut self, delta_yaw: f32, delta_pitch: f32) {
        self.yaw += delta_yaw;
        // Clamping vertical à 89 degrés pour éviter le retournement de caméra
        let max_pitch = 89.0_f32.to_radians();
        self.pitch = (self.pitch - delta_pitch).clamp(-max_pitch, max_pitch);
    }

    /// Met à jour la physique et les déplacements du joueur
    pub fn update(
        &mut self,
        dt: f32,
        move_input: Vec3, // x: gauche/droite, z: avant/arrière
        jump_requested: bool,
        sprint_requested: bool,
        crouch_requested: bool,
        obstacles: &[(Vec3, Vec3)], // Liste des (centre, demi-étendues) des obstacles AABB de la scène
    ) {
        if dt <= 0.0 {
            return;
        }

        // 1. Gestion de l'accroupissement
        self.is_crouching = crouch_requested;
        let target_height = if self.is_crouching {
            self.crouch_height
        } else {
            self.standing_height
        };
        self.current_height += (target_height - self.current_height) * (dt * 15.0).min(1.0);

        // 2. Détermination de la vitesse cible
        self.is_sprinting = sprint_requested && !self.is_crouching && move_input.z > 0.0;
        let max_speed = if self.is_crouching {
            self.crouch_speed
        } else if self.is_sprinting {
            self.sprint_speed
        } else {
            self.walk_speed
        };

        // 3. Calcul de la direction souhaitée dans le repère mondial
        let forward = self.forward_vector();
        let right = self.right_vector();
        let wish_dir = (forward * move_input.z + right * move_input.x).normalize_or_zero();

        // 4. Intégration des forces horizontales (accélération et friction)
        let accel = if self.is_grounded { 45.0 } else { 45.0 * self.air_control };
        let target_vel_h = wish_dir * max_speed;

        let cur_vel_h = Vec3::new(self.velocity.x, 0.0, self.velocity.z);
        let vel_diff = target_vel_h - cur_vel_h;

        if vel_diff.length_squared() > 1e-5 {
            let max_accel = accel * dt;
            let step = vel_diff.clamp_length_max(max_accel);
            self.velocity.x += step.x;
            self.velocity.z += step.z;
        }

        // Friction au sol en l'absence de commande
        if self.is_grounded && wish_dir.length_squared() < 1e-4 {
            let friction_drop = (self.ground_friction * dt).min(1.0);
            self.velocity.x *= 1.0 - friction_drop;
            self.velocity.z *= 1.0 - friction_drop;
        }

        // 5. Saut physique calibré : v = sqrt(2 * g * h)
        if self.is_grounded && jump_requested {
            self.velocity.y = (2.0 * self.gravity * self.jump_height).sqrt();
            self.is_grounded = false;
        }

        // 6. Gravité métrique
        if !self.is_grounded {
            self.velocity.y -= self.gravity * dt;
            // Vitesse terminale de chute (53 m/s = 190 km/h)
            self.velocity.y = self.velocity.y.max(-53.0);
        }

        // 7. Déplacement prévisionnel
        self.position += self.velocity * dt;

        // 8. Résolution des collisions avec le sol (Y = 0)
        let ground_level = 0.0;
        if self.position.y <= ground_level {
            self.position.y = ground_level;
            if self.velocity.y < 0.0 {
                self.velocity.y = 0.0;
            }
            self.is_grounded = true;
        } else {
            // Tolérance de détection au sol (0.05 m)
            if self.position.y - ground_level < 0.05 && self.velocity.y <= 0.0 {
                self.position.y = ground_level;
                self.velocity.y = 0.0;
                self.is_grounded = true;
            } else {
                self.is_grounded = false;
            }
        }

        // 9. Résolution des collisions avec les obstacles AABB de la scène
        for &(box_center, box_half_extents) in obstacles {
            if let Some(push_vector) = Collider::collide_capsule_box(
                self.position,
                self.radius,
                self.current_height,
                box_center,
                box_half_extents,
            ) {
                self.position += push_vector;

                // Annuler la vitesse dans la direction normale de contact
                let norm = push_vector.normalize_or_zero();
                let vel_dot = self.velocity.dot(norm);
                if vel_dot < 0.0 {
                    self.velocity -= norm * vel_dot;
                }

                // Si contact par le dessus d'un obstacle
                if norm.y > 0.7 {
                    self.is_grounded = true;
                }
            }
        }

        // 10. Mise à jour de l'état
        let speed_h = (self.velocity.x * self.velocity.x + self.velocity.z * self.velocity.z).sqrt();
        self.state = if !self.is_grounded {
            PlayerMovementState::InAir
        } else if self.is_crouching {
            PlayerMovementState::Crouching
        } else if speed_h > 5.0 {
            PlayerMovementState::Sprinting
        } else if speed_h > 0.2 {
            PlayerMovementState::Walking
        } else {
            PlayerMovementState::Idle
        };
    }
}
