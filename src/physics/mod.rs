// [WFGY] Zone: SAFE | λ: 0.25 | Fallbacks: 0 | Action: Metric SI Physics World & Simulation Engine with Rigid Bodies and Collisions
#![allow(dead_code, unused_imports)]
pub mod collider;
pub mod controller;
pub mod rigid_body;

pub use collider::{Collider, ColliderShape};
pub use controller::{CharacterController, PlayerMovementState};
pub use rigid_body::RigidBody;

use glam::Vec3;
use crate::scene::SceneNode;

/// Moteur physique temps réel à pas fixe (Fixed Timestep 60Hz)
#[derive(Clone, Debug)]
pub struct PhysicsEngine {
    pub player: CharacterController,
    pub bodies: Vec<RigidBody>,
    pub is_active: bool,
    pub accumulator: f32,
    pub fixed_dt: f32, // 1/60s = 0.016667s
    pub gravity: Vec3, // (0.0, -9.81, 0.0) m/s²
}

impl Default for PhysicsEngine {
    fn default() -> Self {
        Self {
            player: CharacterController::default(),
            bodies: Vec::new(),
            is_active: false,
            accumulator: 0.0,
            fixed_dt: 1.0 / 60.0,
            gravity: Vec3::new(0.0, -9.81, 0.0),
        }
    }
}

impl PhysicsEngine {
    pub fn new() -> Self {
        Self::default()
    }

    /// Ajoute un corps rigide dynamique ou statique à la simulation
    pub fn add_body(&mut self, body: RigidBody) -> usize {
        let idx = self.bodies.len();
        self.bodies.push(body);
        idx
    }

    /// Extrait les boîtes englobantes mondiales (AABB) de tous les objets de la scène
    pub fn extract_obstacles_from_nodes(nodes: &[SceneNode]) -> Vec<(Vec3, Vec3)> {
        let mut obstacles = Vec::with_capacity(nodes.len());

        for node in nodes {
            let local_aabb = node.local_aabb();
            let model_matrix = node.transform.matrix();

            let corners = local_aabb.corners();
            let mut world_min = Vec3::splat(f32::INFINITY);
            let mut world_max = Vec3::splat(f32::NEG_INFINITY);

            for c in corners {
                let world_pt = (model_matrix * glam::Vec4::new(c.x, c.y, c.z, 1.0)).truncate();
                world_min = world_min.min(world_pt);
                world_max = world_max.max(world_pt);
            }

            let center = (world_min + world_max) * 0.5;
            let half_extents = (world_max - world_min) * 0.5;

            // Ignorer les très petites boîtes quasi-nulles
            if half_extents.x > 0.05 && half_extents.y > 0.05 && half_extents.z > 0.05 {
                obstacles.push((center, half_extents));
            }
        }

        obstacles
    }

    /// Exécute les étapes de physique à pas fixe pour une fluidité et une stabilité maximales
    pub fn step(
        &mut self,
        dt: f32,
        move_input: Vec3,
        jump_requested: bool,
        sprint_requested: bool,
        crouch_requested: bool,
        nodes: &[SceneNode],
    ) {
        if !self.is_active {
            return;
        }

        let obstacles = Self::extract_obstacles_from_nodes(nodes);

        // Clamp dt pour éviter les spirales de simulation lors des lags
        let frame_time = dt.min(0.1);
        self.accumulator += frame_time;

        while self.accumulator >= self.fixed_dt {
            let dt_step = self.fixed_dt;

            // 1. Mise à jour du joueur (kinematic character controller)
            self.player.update(
                dt_step,
                move_input,
                jump_requested,
                sprint_requested,
                crouch_requested,
                &obstacles,
            );

            // 2. Intégration des corps rigides dynamiques
            let grav = self.gravity;
            for body in &mut self.bodies {
                body.integrate(dt_step, grav);
            }

            // 3. Résolution des collisions entre corps rigides
            let num_bodies = self.bodies.len();
            for i in 0..num_bodies {
                for j in (i + 1)..num_bodies {
                    if self.bodies[i].is_static && self.bodies[j].is_static {
                        continue;
                    }

                    // Collision Sphère - Sphère
                    let shape_i = self.bodies[i].collider.shape.clone();
                    let shape_j = self.bodies[j].collider.shape.clone();
                    let pos_i = self.bodies[i].position;
                    let pos_j = self.bodies[j].position;

                    let push = match (&shape_i, &shape_j) {
                        (ColliderShape::Sphere { radius: r_i }, ColliderShape::Sphere { radius: r_j }) => {
                            Collider::collide_spheres(pos_i, *r_i, pos_j, *r_j)
                        }
                        (ColliderShape::Sphere { radius: r_i }, ColliderShape::Box { half_extents: e_j }) => {
                            Collider::collide_sphere_box(pos_i, *r_i, pos_j, *e_j)
                        }
                        (ColliderShape::Box { half_extents: e_i }, ColliderShape::Sphere { radius: r_j }) => {
                            Collider::collide_sphere_box(pos_j, *r_j, pos_i, *e_i).map(|v| -v)
                        }
                        (ColliderShape::Box { half_extents: e_i }, ColliderShape::Box { half_extents: e_j }) => {
                            Collider::collide_boxes(pos_i, *e_i, pos_j, *e_j)
                        }
                        _ => None,
                    };

                    if let Some(push_vec) = push {
                        let total_mass = self.bodies[i].mass + self.bodies[j].mass;
                        if total_mass > 0.0 {
                            let ratio_i = if self.bodies[i].is_static { 0.0 } else { self.bodies[j].mass / total_mass };
                            let ratio_j = if self.bodies[j].is_static { 0.0 } else { self.bodies[i].mass / total_mass };

                            self.bodies[i].position += push_vec * ratio_i;
                            self.bodies[j].position -= push_vec * ratio_j;

                            let normal = push_vec.normalize_or_zero();
                            let rel_vel = self.bodies[i].linear_velocity - self.bodies[j].linear_velocity;
                            let vel_along_norm = rel_vel.dot(normal);

                            if vel_along_norm < 0.0 {
                                let restitution = self.bodies[i].restitution.min(self.bodies[j].restitution);
                                let impulse_mag = -(1.0 + restitution) * vel_along_norm / (1.0 / self.bodies[i].mass.max(0.001) + 1.0 / self.bodies[j].mass.max(0.001));
                                let impulse = normal * impulse_mag;

                                self.bodies[i].apply_impulse(impulse);
                                self.bodies[j].apply_impulse(-impulse);
                            }
                        }
                    }
                }
            }

            self.accumulator -= self.fixed_dt;
        }
    }

    /// Déclenche une onde de choc d'explosion physique projetant tous les corps rigides
    /// F = force * max(0, 1 - d / radius), avec impulsion angulaire pour faire culbuter les objets
    pub fn explode_at(&mut self, epicenter: Vec3, radius: f32, force: f32) -> Vec<(usize, Vec3)> {
        let mut affected = Vec::new();
        let r_sq = radius * radius;

        for (idx, body) in self.bodies.iter_mut().enumerate() {
            if body.is_static {
                continue;
            }

            let to_body = body.position - epicenter;
            let dist_sq = to_body.length_squared();

            if dist_sq < r_sq {
                let dist = dist_sq.sqrt().max(0.1);
                let dir = (to_body / dist).normalize();
                
                // Atténuation physique quadratique / linéaire
                let factor = (1.0 - dist / radius).max(0.0).powf(1.5);
                let impulse_mag = force * factor;
                
                // Impulsion ascendante légèrement biaisée pour faire voler les objets
                let blast_dir = (dir + Vec3::new(0.0, 0.45, 0.0)).normalize();
                let impulse = blast_dir * impulse_mag;

                body.apply_impulse(impulse);

                // Couple de rotation décentré pour un culbutage réaliste
                let torque_axis = Vec3::new(
                    dir.z * 1.5 + 0.2,
                    dir.y * 0.5,
                    -dir.x * 1.5 + 0.1,
                ).normalize_or_zero();
                body.apply_torque_impulse(torque_axis * impulse_mag * 0.8);

                affected.push((idx, body.position));
            }
        }

        // Si le joueur est dans le rayon de l'explosion, lui appliquer également une propulsion
        let to_player = self.player.position - epicenter;
        let player_dist_sq = to_player.length_squared();
        if player_dist_sq < r_sq {
            let dist = player_dist_sq.sqrt().max(0.1);
            let dir = (to_player / dist).normalize();
            let factor = (1.0 - dist / radius).max(0.0);
            let player_impulse = (dir + Vec3::new(0.0, 0.6, 0.0)).normalize() * (force * factor * 0.4);
            self.player.velocity += player_impulse;
        }

        affected
    }

    /// Synchronise la liste des nœuds de la scène avec les corps rigides physiques
    pub fn sync_from_scene_nodes(&mut self, nodes: &[SceneNode]) {
        while self.bodies.len() < nodes.len() {
            let idx = self.bodies.len();
            let node = &nodes[idx];
            let aabb = node.local_aabb();
            let half_extents = aabb.size() * node.transform.scale * 0.5;

            // Par défaut, les nœuds au-dessus du sol sont dynamiques, le sol/l'eau est statique
            let is_ground = node.name.to_lowercase().contains("sol") 
                || node.name.to_lowercase().contains("water") 
                || node.name.to_lowercase().contains("terrain")
                || node.material.transmission > 0.5;

            let body = if is_ground {
                RigidBody::new_static(
                    node.transform.translation,
                    ColliderShape::Box { half_extents: half_extents.max(Vec3::splat(0.2)) },
                )
            } else {
                let mut b = RigidBody::new_dynamic(
                    node.transform.translation,
                    (half_extents.x * half_extents.y * half_extents.z * 8.0).clamp(0.5, 50.0),
                    ColliderShape::Box { half_extents: half_extents.max(Vec3::splat(0.2)) },
                );
                b.rotation = glam::Quat::from_euler(
                    glam::EulerRot::YXZ,
                    node.transform.rotation.y,
                    node.transform.rotation.x,
                    node.transform.rotation.z,
                );
                b
            };
            self.bodies.push(body);
        }

        // Si des nœuds ont été supprimés
        if self.bodies.len() > nodes.len() {
            self.bodies.truncate(nodes.len());
        }
    }

    /// Met à jour les transforms des SceneNodes à partir des corps physiques simulés
    pub fn sync_to_scene_nodes(&self, nodes: &mut [SceneNode]) {
        for (i, node) in nodes.iter_mut().enumerate() {
            if i < self.bodies.len() {
                let body = &self.bodies[i];
                if !body.is_static {
                    node.transform.translation = body.position;
                    let (y, x, z) = body.rotation.to_euler(glam::EulerRot::YXZ);
                    node.transform.rotation = Vec3::new(x, y, z);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_character_controller_jump_physics() {
        let mut controller = CharacterController::new(Vec3::new(0.0, 0.0, 0.0));
        assert!(controller.is_grounded);

        // Saut
        controller.update(0.016, Vec3::ZERO, true, false, false, &[]);
        assert!(!controller.is_grounded);
        assert!(controller.velocity.y > 4.0); // v = sqrt(2 * 9.81 * 1.2) ≈ 4.85 m/s
    }

    #[test]
    fn test_capsule_ground_collision() {
        let mut controller = CharacterController::new(Vec3::new(0.0, 5.0, 0.0));
        assert!(!controller.is_grounded);
        // Chute libre pendant 1.6 seconde (100 pas de 0.016s)
        for _ in 0..100 {
            controller.update(0.016, Vec3::ZERO, false, false, false, &[]);
        }
        assert_eq!(controller.position.y, 0.0);
        assert!(controller.is_grounded);
    }

    #[test]
    fn test_rigid_body_physics_step() {
        let mut engine = PhysicsEngine::new();
        engine.is_active = true;
        let b1 = RigidBody::new_dynamic(Vec3::new(0.0, 5.0, 0.0), 1.0, ColliderShape::Sphere { radius: 0.5 });
        engine.add_body(b1);

        engine.step(0.1, Vec3::ZERO, false, false, false, &[]);
        assert!(engine.bodies[0].position.y < 5.0);
    }
}
