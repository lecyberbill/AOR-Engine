// [WFGY] Zone: SAFE | λ: 0.3 | Fallbacks: 0/None | Action: Dynamic Rigid Body Physics simulation with metric SI units and impulse resolution
#![allow(dead_code)]
use glam::{Quat, Vec3};
use crate::physics::collider::{Collider, ColliderShape};

/// Corps rigide dynamique soumis aux lois physiques métriques SI (Newton & Euler)
#[derive(Clone, Debug)]
pub struct RigidBody {
    pub position: Vec3,
    pub rotation: Quat,
    pub linear_velocity: Vec3,
    pub angular_velocity: Vec3,
    pub mass: f32,             // en kg (0.0 = objet statique infini / ancré)
    pub restitution: f32,      // Coefficient de restitution (rebond 0.0..1.0)
    pub friction: f32,         // Coefficient de friction dynamique
    pub is_static: bool,       // Objet immobile (sol, mur, plateforme)
    pub collider: Collider,
    pub use_gravity: bool,
    pub linear_damping: f32,   // Amortissement de l'air
    pub angular_damping: f32,  // Amortissement de rotation
}

impl Default for RigidBody {
    fn default() -> Self {
        Self {
            position: Vec3::ZERO,
            rotation: Quat::IDENTITY,
            linear_velocity: Vec3::ZERO,
            angular_velocity: Vec3::ZERO,
            mass: 1.0,
            restitution: 0.4,
            friction: 0.5,
            is_static: false,
            collider: Collider::new_box(Vec3::ZERO, Vec3::new(0.5, 0.5, 0.5)),
            use_gravity: true,
            linear_damping: 0.05,
            angular_damping: 0.1,
        }
    }
}

impl RigidBody {
    pub fn new_dynamic(position: Vec3, mass: f32, shape: ColliderShape) -> Self {
        Self {
            position,
            rotation: Quat::IDENTITY,
            linear_velocity: Vec3::ZERO,
            angular_velocity: Vec3::ZERO,
            mass: mass.max(0.001),
            restitution: 0.45,
            friction: 0.6,
            is_static: false,
            collider: Collider { position, shape },
            use_gravity: true,
            linear_damping: 0.02,
            angular_damping: 0.05,
        }
    }

    pub fn new_static(position: Vec3, shape: ColliderShape) -> Self {
        Self {
            position,
            rotation: Quat::IDENTITY,
            linear_velocity: Vec3::ZERO,
            angular_velocity: Vec3::ZERO,
            mass: 0.0,
            restitution: 0.3,
            friction: 0.7,
            is_static: true,
            collider: Collider { position, shape },
            use_gravity: false,
            linear_damping: 0.0,
            angular_damping: 0.0,
        }
    }

    pub fn apply_impulse(&mut self, impulse: Vec3) {
        if !self.is_static && self.mass > 0.0 {
            self.linear_velocity += impulse / self.mass;
        }
    }

    pub fn apply_torque_impulse(&mut self, torque_impulse: Vec3) {
        if !self.is_static && self.mass > 0.0 {
            // Approximation d'inertie sphérique I = (2/5) * m * r^2
            let inertia = 0.4 * self.mass;
            self.angular_velocity += torque_impulse / inertia;
        }
    }

    /// Intégration temporelle semi-implicite d'Euler (stabilité physique garantie)
    pub fn integrate(&mut self, dt: f32, gravity: Vec3) {
        if self.is_static {
            return;
        }

        // 1. Application des forces de pesanteur et amortissement
        if self.use_gravity {
            self.linear_velocity += gravity * dt;
        }

        self.linear_velocity *= (1.0 - self.linear_damping * dt).clamp(0.0, 1.0);
        self.angular_velocity *= (1.0 - self.angular_damping * dt).clamp(0.0, 1.0);

        // 2. Intégration de la position
        self.position += self.linear_velocity * dt;

        // 3. Intégration de la rotation
        let ang_speed = self.angular_velocity.length();
        if ang_speed > 1e-5 {
            let axis = self.angular_velocity / ang_speed;
            let delta_rot = Quat::from_axis_angle(axis, ang_speed * dt);
            self.rotation = (delta_rot * self.rotation).normalize();
        }

        // 4. Synchronisation du colliseur
        self.collider.position = self.position;

        // 5. Collision avec le sol métrique Y = 0 (Plan par défaut)
        let radius = match self.collider.shape {
            ColliderShape::Sphere { radius } => radius,
            ColliderShape::Box { half_extents } => half_extents.y,
            ColliderShape::Capsule { radius, .. } => radius,
            ColliderShape::Plane { .. } => 0.0,
        };

        if self.position.y - radius < 0.0 {
            self.position.y = radius;
            self.collider.position.y = radius;
            if self.linear_velocity.y < 0.0 {
                self.linear_velocity.y = -self.linear_velocity.y * self.restitution;
                // Friction au sol
                let friction_factor = (1.0 - self.friction * dt * 10.0).clamp(0.0, 1.0);
                self.linear_velocity.x *= friction_factor;
                self.linear_velocity.z *= friction_factor;
                self.angular_velocity *= friction_factor;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rigid_body_gravity_integration() {
        let mut body = RigidBody::new_dynamic(
            Vec3::new(0.0, 10.0, 0.0),
            2.0,
            ColliderShape::Sphere { radius: 0.5 },
        );
        let gravity = Vec3::new(0.0, -9.81, 0.0);
        body.integrate(0.1, gravity);
        assert!(body.position.y < 10.0);
        assert!(body.linear_velocity.y < 0.0);
    }

    #[test]
    fn test_rigid_body_impulse() {
        let mut body = RigidBody::new_dynamic(
            Vec3::ZERO,
            2.0,
            ColliderShape::Sphere { radius: 0.5 },
        );
        body.apply_impulse(Vec3::new(10.0, 0.0, 0.0));
        assert_eq!(body.linear_velocity.x, 5.0); // 10.0 / 2.0 = 5.0 m/s
    }
}
