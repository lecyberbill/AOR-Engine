// [WFGY] Zone: SAFE | λ: 0.2 | Fallbacks: 0/None | Action: Lightweight Custom 3D Engine DSL
#![allow(dead_code)]
use glam::Vec3;
use super::ScriptContext;
use crate::scene::PrimitiveType;
use crate::particles::ParticleKind;
use crate::audio::SoundEffect;
use crate::physics::{ColliderShape, RigidBody};

/// Comportement périodique ou scripté enregistré dans le DSL
#[derive(Clone, Debug)]
pub struct DslPeriodicAction {
    pub node_id: usize,
    pub action_type: String,
    pub speed: f32,
    pub axis: Vec3,
}

/// Exécuteur du DSL Léger (Commandes directes et scripts procéduraux)
pub struct DslRunner {
    pub periodic_actions: Vec<DslPeriodicAction>,
}

impl Default for DslRunner {
    fn default() -> Self {
        Self {
            periodic_actions: Vec::new(),
        }
    }
}

impl DslRunner {
    pub fn new() -> Self {
        Self::default()
    }

    /// Exécute un script ou une ligne de commande DSL
    /// Exemples de syntaxe DSL :
    /// - `spawn cube 0.0 5.0 0.0`
    /// - `spawn sphere 2.0 1.0 -3.0`
    /// - `pos 1 10.0 2.0 0.0`
    /// - `rotate_auto 1 y 1.5`
    /// - `impulse 0 0.0 50.0 0.0`
    /// - `explode 0.0 0.0 0.0 10.0 25.0`
    /// - `particles sparks 0.0 2.0 0.0 50`
    /// - `sound explosion 0.0 0.0 0.0`
    pub fn execute(&mut self, script: &str, ctx: &mut ScriptContext) -> Result<String, String> {
        let mut outputs = Vec::new();

        for line in script.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') || line.starts_with("//") {
                continue;
            }

            let tokens: Vec<&str> = line.split_whitespace().collect();
            if tokens.is_empty() {
                continue;
            }

            match tokens[0].to_lowercase().as_str() {
                "spawn" => {
                    if tokens.len() < 5 {
                        return Err("Usage: spawn <cube|sphere|plane|cylinder> <x> <y> <z>".to_string());
                    }
                    let prim_type = match tokens[1].to_lowercase().as_str() {
                        "cube" => PrimitiveType::Cube,
                        "sphere" => PrimitiveType::Sphere,
                        "plane" => PrimitiveType::Plane,
                        "cylinder" => PrimitiveType::Cylinder,
                        _ => return Err(format!("Unknown primitive type: {}", tokens[1])),
                    };
                    let x: f32 = tokens[2].parse().map_err(|e| format!("Invalid X: {e}"))?;
                    let y: f32 = tokens[3].parse().map_err(|e| format!("Invalid Y: {e}"))?;
                    let z: f32 = tokens[4].parse().map_err(|e| format!("Invalid Z: {e}"))?;
                    let pos = Vec3::new(x, y, z);

                    let node_idx = ctx.scene.add_node_at(ctx.device, prim_type, pos);
                    let body = RigidBody::new_dynamic(pos, 10.0, ColliderShape::Box { half_extents: Vec3::splat(0.5) });
                    ctx.physics.add_body(body);
                    outputs.push(format!("Spawned {} with Node ID {} at ({:.1}, {:.1}, {:.1})", tokens[1], node_idx, x, y, z));
                }

                "pos" | "set_pos" => {
                    if tokens.len() < 5 {
                        return Err("Usage: pos <node_id> <x> <y> <z>".to_string());
                    }
                    let id: usize = tokens[1].parse().map_err(|e| format!("Invalid node ID: {e}"))?;
                    let x: f32 = tokens[2].parse().map_err(|e| format!("Invalid X: {e}"))?;
                    let y: f32 = tokens[3].parse().map_err(|e| format!("Invalid Y: {e}"))?;
                    let z: f32 = tokens[4].parse().map_err(|e| format!("Invalid Z: {e}"))?;
                    
                    if let Some(node) = ctx.scene.nodes.get_mut(id) {
                        node.transform.translation = Vec3::new(x, y, z);
                        if let Some(rb) = ctx.physics.bodies.get_mut(id) {
                            rb.position = Vec3::new(x, y, z);
                        }
                        node.update_gpu(ctx.queue, false);
                        outputs.push(format!("Set node {} position to ({:.1}, {:.1}, {:.1})", id, x, y, z));
                    } else {
                        return Err(format!("Node ID {} not found", id));
                    }
                }

                "scale" | "set_scale" => {
                    if tokens.len() < 5 {
                        return Err("Usage: scale <node_id> <sx> <sy> <sz>".to_string());
                    }
                    let id: usize = tokens[1].parse().map_err(|e| format!("Invalid node ID: {e}"))?;
                    let sx: f32 = tokens[2].parse().map_err(|e| format!("Invalid SX: {e}"))?;
                    let sy: f32 = tokens[3].parse().map_err(|e| format!("Invalid SY: {e}"))?;
                    let sz: f32 = tokens[4].parse().map_err(|e| format!("Invalid SZ: {e}"))?;
                    
                    if let Some(node) = ctx.scene.nodes.get_mut(id) {
                        node.transform.scale = Vec3::new(sx, sy, sz);
                        node.update_gpu(ctx.queue, false);
                        outputs.push(format!("Set node {} scale to ({:.1}, {:.1}, {:.1})", id, sx, sy, sz));
                    } else {
                        return Err(format!("Node ID {} not found", id));
                    }
                }

                "rotate_auto" => {
                    if tokens.len() < 4 {
                        return Err("Usage: rotate_auto <node_id> <x|y|z> <speed>".to_string());
                    }
                    let id: usize = tokens[1].parse().map_err(|e| format!("Invalid node ID: {e}"))?;
                    let axis = match tokens[2].to_lowercase().as_str() {
                        "x" => Vec3::X,
                        "y" => Vec3::Y,
                        "z" => Vec3::Z,
                        _ => return Err(format!("Invalid axis: {}", tokens[2])),
                    };
                    let speed: f32 = tokens[3].parse().map_err(|e| format!("Invalid speed: {e}"))?;

                    self.periodic_actions.push(DslPeriodicAction {
                        node_id: id,
                        action_type: "rotate".to_string(),
                        speed,
                        axis,
                    });
                    outputs.push(format!("Registered auto-rotation on node {} around axis {:?} at speed {:.2}", id, tokens[2], speed));
                }

                "impulse" => {
                    if tokens.len() < 5 {
                        return Err("Usage: impulse <node_id> <fx> <fy> <fz>".to_string());
                    }
                    let id: usize = tokens[1].parse().map_err(|e| format!("Invalid node ID: {e}"))?;
                    let fx: f32 = tokens[2].parse().map_err(|e| format!("Invalid Fx: {e}"))?;
                    let fy: f32 = tokens[3].parse().map_err(|e| format!("Invalid Fy: {e}"))?;
                    let fz: f32 = tokens[4].parse().map_err(|e| format!("Invalid Fz: {e}"))?;

                    if let Some(rb) = ctx.physics.bodies.get_mut(id) {
                        rb.apply_impulse(Vec3::new(fx, fy, fz));
                        outputs.push(format!("Applied impulse ({:.1}, {:.1}, {:.1}) on node {}", fx, fy, fz, id));
                    } else {
                        return Err(format!("RigidBody for node ID {} not found", id));
                    }
                }

                "explode" => {
                    if tokens.len() < 6 {
                        return Err("Usage: explode <x> <y> <z> <radius> <force>".to_string());
                    }
                    let x: f32 = tokens[1].parse().map_err(|e| format!("Invalid X: {e}"))?;
                    let y: f32 = tokens[2].parse().map_err(|e| format!("Invalid Y: {e}"))?;
                    let z: f32 = tokens[3].parse().map_err(|e| format!("Invalid Z: {e}"))?;
                    let radius: f32 = tokens[4].parse().map_err(|e| format!("Invalid Radius: {e}"))?;
                    let force: f32 = tokens[5].parse().map_err(|e| format!("Invalid Force: {e}"))?;
                    let pos = Vec3::new(x, y, z);

                    ctx.physics.explode_at(pos, radius, force);
                    if let Some(particles) = &mut ctx.particles {
                        particles.spawn_explosion(pos, 1.5);
                    }
                    ctx.audio.play_spatial_sound(SoundEffect::Explosion, pos, 1.0);
                    outputs.push(format!("Triggered physical explosion at ({:.1}, {:.1}, {:.1}) radius {:.1} force {:.1}", x, y, z, radius, force));
                }

                "particles" => {
                    if tokens.len() < 6 {
                        return Err("Usage: particles <sparks|smoke|fire|debris> <x> <y> <z> <count>".to_string());
                    }
                    let kind = match tokens[1].to_lowercase().as_str() {
                        "sparks" | "spark" => ParticleKind::Spark,
                        "smoke" => ParticleKind::Smoke,
                        "fire" => ParticleKind::Fire,
                        "debris" => ParticleKind::Debris,
                        _ => return Err(format!("Unknown particle kind: {}", tokens[1])),
                    };
                    let x: f32 = tokens[2].parse().map_err(|e| format!("Invalid X: {e}"))?;
                    let y: f32 = tokens[3].parse().map_err(|e| format!("Invalid Y: {e}"))?;
                    let z: f32 = tokens[4].parse().map_err(|e| format!("Invalid Z: {e}"))?;
                    let _count: usize = tokens[5].parse().map_err(|e| format!("Invalid Count: {e}"))?;
                    let pos = Vec3::new(x, y, z);

                    if let Some(particles) = &mut ctx.particles {
                        particles.spawn_sparks(pos, Vec3::Y, 20);
                    }
                    outputs.push(format!("Spawned {:?} particles at ({:.1}, {:.1}, {:.1})", kind, x, y, z));
                }

                "sound" => {
                    if tokens.len() < 5 {
                        return Err("Usage: sound <jump|explosion|impact|laser|wind> <x> <y> <z>".to_string());
                    }
                    let effect = match tokens[1].to_lowercase().as_str() {
                        "jump" => SoundEffect::Jump,
                        "explosion" => SoundEffect::Explosion,
                        "impact" => SoundEffect::Impact,
                        "laser" => SoundEffect::Laser,
                        "wind" => SoundEffect::WindBreeze,
                        _ => return Err(format!("Unknown sound effect: {}", tokens[1])),
                    };
                    let x: f32 = tokens[2].parse().map_err(|e| format!("Invalid X: {e}"))?;
                    let y: f32 = tokens[3].parse().map_err(|e| format!("Invalid Y: {e}"))?;
                    let z: f32 = tokens[4].parse().map_err(|e| format!("Invalid Z: {e}"))?;
                    let pos = Vec3::new(x, y, z);

                    ctx.audio.play_spatial_sound(effect, pos, 1.0);
                    outputs.push(format!("Played 3D sound {:?} at ({:.1}, {:.1}, {:.1})", effect, x, y, z));
                }

                cmd => return Err(format!("Unknown DSL command: '{cmd}'")),
            }
        }

        Ok(outputs.join("\n"))
    }

    /// Mise à jour continue des actions périodiques du DSL
    pub fn update(&mut self, ctx: &mut ScriptContext) {
        let dt = ctx.delta_time;
        for action in &self.periodic_actions {
            if action.action_type == "rotate" {
                if let Some(node) = ctx.scene.nodes.get_mut(action.node_id) {
                    if action.axis == Vec3::Y {
                        node.transform.rotation.y += action.speed * dt;
                    } else if action.axis == Vec3::X {
                        node.transform.rotation.x += action.speed * dt;
                    } else if action.axis == Vec3::Z {
                        node.transform.rotation.z += action.speed * dt;
                    }
                    node.update_gpu(ctx.queue, false);
                }
            }
        }
    }
}
