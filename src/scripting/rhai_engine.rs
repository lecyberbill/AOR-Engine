// [WFGY] Zone: SAFE | λ: 0.3 | Fallbacks: 0/None | Action: Advanced Rhai Embedded Scripting Engine
#![allow(dead_code)]
use glam::Vec3;
use rhai::{Engine, Scope, AST, Dynamic};
use std::sync::{Arc, Mutex};
use super::ScriptContext;
use crate::scene::PrimitiveType;
use crate::audio::SoundEffect;
use crate::physics::{ColliderShape, RigidBody};

/// File d'actions générée par l'environnement de script Rhai
#[derive(Clone, Debug)]
pub enum RhaiEngineAction {
    SpawnPrimitive { name: String, prim_type: String, pos: Vec3 },
    SetPosition { node_id: usize, pos: Vec3 },
    SetRotation { node_id: usize, rot: Vec3 },
    SetScale { node_id: usize, scale: Vec3 },
    ApplyImpulse { node_id: usize, force: Vec3 },
    TriggerExplosion { pos: Vec3, radius: f32, force: f32 },
    SpawnParticles { kind: String, pos: Vec3, count: usize },
    PlaySound { sound: String, pos: Vec3 },
    LogMessage(String),
}

/// Moteur d'exécution Rhai connecté au moteur 3D
pub struct RhaiRunner {
    pub engine: Engine,
    pub active_ast: Option<AST>,
    pub action_queue: Arc<Mutex<Vec<RhaiEngineAction>>>,
    pub logs: Vec<String>,
}

impl Default for RhaiRunner {
    fn default() -> Self {
        let mut engine = Engine::new();
        let action_queue = Arc::new(Mutex::new(Vec::new()));
        let queue_clone = Arc::clone(&action_queue);

        // Bindings Rhai natifs pour le moteur 3D
        {
            let q = Arc::clone(&queue_clone);
            engine.register_fn("spawn_primitive", move |name: &str, prim_type: &str, x: f64, y: f64, z: f64| {
                if let Ok(mut lock) = q.lock() {
                    lock.push(RhaiEngineAction::SpawnPrimitive {
                        name: name.to_string(),
                        prim_type: prim_type.to_string(),
                        pos: Vec3::new(x as f32, y as f32, z as f32),
                    });
                }
            });
        }

        {
            let q = Arc::clone(&queue_clone);
            engine.register_fn("set_pos", move |node_id: i64, x: f64, y: f64, z: f64| {
                if let Ok(mut lock) = q.lock() {
                    lock.push(RhaiEngineAction::SetPosition {
                        node_id: node_id as usize,
                        pos: Vec3::new(x as f32, y as f32, z as f32),
                    });
                }
            });
        }

        {
            let q = Arc::clone(&queue_clone);
            engine.register_fn("set_rot", move |node_id: i64, pitch: f64, yaw: f64, roll: f64| {
                if let Ok(mut lock) = q.lock() {
                    lock.push(RhaiEngineAction::SetRotation {
                        node_id: node_id as usize,
                        rot: Vec3::new(pitch as f32, yaw as f32, roll as f32),
                    });
                }
            });
        }

        {
            let q = Arc::clone(&queue_clone);
            engine.register_fn("set_scale", move |node_id: i64, sx: f64, sy: f64, sz: f64| {
                if let Ok(mut lock) = q.lock() {
                    lock.push(RhaiEngineAction::SetScale {
                        node_id: node_id as usize,
                        scale: Vec3::new(sx as f32, sy as f32, sz as f32),
                    });
                }
            });
        }

        {
            let q = Arc::clone(&queue_clone);
            engine.register_fn("apply_impulse", move |node_id: i64, fx: f64, fy: f64, fz: f64| {
                if let Ok(mut lock) = q.lock() {
                    lock.push(RhaiEngineAction::ApplyImpulse {
                        node_id: node_id as usize,
                        force: Vec3::new(fx as f32, fy as f32, fz as f32),
                    });
                }
            });
        }

        {
            let q = Arc::clone(&queue_clone);
            engine.register_fn("trigger_explosion", move |x: f64, y: f64, z: f64, radius: f64, force: f64| {
                if let Ok(mut lock) = q.lock() {
                    lock.push(RhaiEngineAction::TriggerExplosion {
                        pos: Vec3::new(x as f32, y as f32, z as f32),
                        radius: radius as f32,
                        force: force as f32,
                    });
                }
            });
        }

        {
            let q = Arc::clone(&queue_clone);
            engine.register_fn("spawn_particles", move |kind: &str, x: f64, y: f64, z: f64, count: i64| {
                if let Ok(mut lock) = q.lock() {
                    lock.push(RhaiEngineAction::SpawnParticles {
                        kind: kind.to_string(),
                        pos: Vec3::new(x as f32, y as f32, z as f32),
                        count: count as usize,
                    });
                }
            });
        }

        {
            let q = Arc::clone(&queue_clone);
            engine.register_fn("play_sound", move |name: &str, x: f64, y: f64, z: f64| {
                if let Ok(mut lock) = q.lock() {
                    lock.push(RhaiEngineAction::PlaySound {
                        sound: name.to_string(),
                        pos: Vec3::new(x as f32, y as f32, z as f32),
                    });
                }
            });
        }

        {
            let q = Arc::clone(&queue_clone);
            engine.register_fn("log", move |msg: &str| {
                if let Ok(mut lock) = q.lock() {
                    lock.push(RhaiEngineAction::LogMessage(msg.to_string()));
                }
            });
        }

        Self {
            engine,
            active_ast: None,
            action_queue,
            logs: Vec::new(),
        }
    }
}

impl RhaiRunner {
    pub fn new() -> Self {
        Self::default()
    }

    /// Compile et exécute un script Rhai
    pub fn execute(&mut self, script: &str, ctx: &mut ScriptContext) -> Result<String, String> {
        let ast = self.engine.compile(script).map_err(|e| format!("Compilation Error: {e}"))?;
        let mut scope = Scope::new();
        scope.push("delta_time", ctx.delta_time as f64);
        scope.push("time", ctx.current_time as f64);

        let result: Dynamic = self.engine.eval_ast_with_scope(&mut scope, &ast)
            .map_err(|e| format!("Runtime Error: {e}"))?;

        // On conserve l'AST s'il contient une fonction update()
        self.active_ast = Some(ast);

        // Appliquer immédiatement les actions produites
        self.flush_actions(ctx);

        Ok(format!("Script executed successfully. Return: {:?}", result))
    }

    /// Exécute la fonction 'on_update(dt, time)' définie dans le script actif
    pub fn update(&mut self, ctx: &mut ScriptContext) {
        if let Some(ref ast) = self.active_ast {
            let mut scope = Scope::new();
            let _ = self.engine.call_fn::<()>(&mut scope, ast, "on_update", (ctx.delta_time as f64, ctx.current_time as f64));
            self.flush_actions(ctx);
        }
    }

    /// Décharge et applique la file d'actions sur le contexte du moteur 3D
    fn flush_actions(&mut self, ctx: &mut ScriptContext) {
        let mut actions = Vec::new();
        if let Ok(mut lock) = self.action_queue.lock() {
            actions.append(&mut *lock);
        }

        for action in actions {
            match action {
                RhaiEngineAction::SpawnPrimitive { name, prim_type, pos } => {
                    let p = match prim_type.to_lowercase().as_str() {
                        "cube" => PrimitiveType::Cube,
                        "sphere" => PrimitiveType::Sphere,
                        "plane" => PrimitiveType::Plane,
                        "cylinder" => PrimitiveType::Cylinder,
                        _ => PrimitiveType::Cube,
                    };
                    let node_idx = ctx.scene.add_node_at(ctx.device, p, pos);
                    if let Some(node) = ctx.scene.nodes.get_mut(node_idx) {
                        node.name = name;
                    }
                    let body = RigidBody::new_dynamic(pos, 10.0, ColliderShape::Box { half_extents: Vec3::splat(0.5) });
                    ctx.physics.add_body(body);
                }
                RhaiEngineAction::SetPosition { node_id, pos } => {
                    if let Some(node) = ctx.scene.nodes.get_mut(node_id) {
                        node.transform.translation = pos;
                        node.update_gpu(ctx.queue, false);
                    }
                    if let Some(rb) = ctx.physics.bodies.get_mut(node_id) {
                        rb.position = pos;
                    }
                }
                RhaiEngineAction::SetRotation { node_id, rot } => {
                    if let Some(node) = ctx.scene.nodes.get_mut(node_id) {
                        node.transform.rotation = rot;
                        node.update_gpu(ctx.queue, false);
                    }
                }
                RhaiEngineAction::SetScale { node_id, scale } => {
                    if let Some(node) = ctx.scene.nodes.get_mut(node_id) {
                        node.transform.scale = scale;
                        node.update_gpu(ctx.queue, false);
                    }
                }
                RhaiEngineAction::ApplyImpulse { node_id, force } => {
                    if let Some(rb) = ctx.physics.bodies.get_mut(node_id) {
                        rb.apply_impulse(force);
                    }
                }
                RhaiEngineAction::TriggerExplosion { pos, radius, force } => {
                    ctx.physics.explode_at(pos, radius, force);
                    if let Some(particles) = &mut ctx.particles {
                        particles.spawn_explosion(pos, 1.5);
                    }
                    ctx.audio.play_spatial_sound(SoundEffect::Explosion, pos, 1.0);
                }
                RhaiEngineAction::SpawnParticles { kind: _, pos, count } => {
                    if let Some(particles) = &mut ctx.particles {
                        particles.spawn_sparks(pos, Vec3::Y, count.max(10));
                    }
                }
                RhaiEngineAction::PlaySound { sound, pos } => {
                    let s = match sound.to_lowercase().as_str() {
                        "jump" => SoundEffect::Jump,
                        "explosion" => SoundEffect::Explosion,
                        "impact" => SoundEffect::Impact,
                        "laser" => SoundEffect::Laser,
                        "wind" => SoundEffect::WindBreeze,
                        _ => SoundEffect::Impact,
                    };
                    ctx.audio.play_spatial_sound(s, pos, 1.0);
                }
                RhaiEngineAction::LogMessage(msg) => {
                    self.logs.push(msg);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rhai_engine_compilation_and_registration() {
        let rhai = RhaiRunner::new();
        let script = r#"
            spawn_primitive("RhaiCube", "cube", 15.0, 3.0, 0.0);
            set_scale(0, 3.0, 3.0, 3.0);
            apply_impulse(0, 0.0, 100.0, 0.0);
            trigger_explosion(0.0, 0.0, 0.0, 8.0, 20.0);
            play_sound("explosion", 0.0, 0.0, 0.0);
            log("Rhai test complete!");
            42
        "#;

        let ast = rhai.engine.compile(script);
        assert!(ast.is_ok());
    }
}
