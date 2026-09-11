// [WFGY] Zone: SAFE | λ: 0.3 | Fallbacks: 0/None | Action: Scripting Engine (Lightweight DSL & Rhai VM)
#![allow(dead_code)]
pub mod dsl;
pub mod rhai_engine;

use crate::scene::Scene;
use crate::physics::PhysicsEngine;
use crate::particles::ParticleSystem;
use crate::audio::SpatialAudioEngine;

/// Contexte d'exécution partagé pour les scripts
pub struct ScriptContext<'a> {
    pub scene: &'a mut Scene,
    pub physics: &'a mut PhysicsEngine,
    pub particles: Option<&'a mut ParticleSystem>,
    pub audio: &'a mut SpatialAudioEngine,
    pub device: &'a wgpu::Device,
    pub queue: &'a wgpu::Queue,
    pub delta_time: f32,
    pub current_time: f32,
}

/// Moteur de scripting unifié (DSL léger + Rhai puissant)
pub struct ScriptingEngine {
    pub dsl_runner: dsl::DslRunner,
    pub rhai_runner: rhai_engine::RhaiRunner,
}

impl Default for ScriptingEngine {
    fn default() -> Self {
        Self {
            dsl_runner: dsl::DslRunner::new(),
            rhai_runner: rhai_engine::RhaiRunner::new(),
        }
    }
}

impl ScriptingEngine {
    pub fn new() -> Self {
        Self::default()
    }

    /// Exécute une commande ou un script DSL léger
    pub fn execute_dsl(&mut self, script: &str, ctx: &mut ScriptContext) -> Result<String, String> {
        self.dsl_runner.execute(script, ctx)
    }

    /// Exécute un script Rhai complet
    pub fn execute_rhai(&mut self, script: &str, ctx: &mut ScriptContext) -> Result<String, String> {
        self.rhai_runner.execute(script, ctx)
    }

    /// Met à jour les scripts actifs
    pub fn update(&mut self, ctx: &mut ScriptContext) {
        self.dsl_runner.update(ctx);
        self.rhai_runner.update(ctx);
    }
}
