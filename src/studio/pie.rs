// [WFGY] Zone: SAFE | λ: 0.25 | Fallbacks: 0/None | Action: Play-in-Editor state machine with full ECS/scene snapshot & restore
#![allow(dead_code)]
use crate::scene::persistence::{deserialize_world, serialize_world, WorldData};
use crate::scene::Scene;
use glam::Vec3;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PieState {
    Stopped,
    Playing,
    Paused,
}

/// Contrôleur Play-in-Editor : capture l'état de la scène au lancement et le restaure à l'arrêt
#[derive(Debug)]
pub struct PlayInEditor {
    pub state: PieState,
    snapshot: Option<WorldData>,
    pub elapsed: f32,
    pub time_scale: f32,
    pub light_pos: Vec3,
}

impl Default for PlayInEditor {
    fn default() -> Self {
        PlayInEditor {
            state: PieState::Stopped,
            snapshot: None,
            elapsed: 0.0,
            time_scale: 1.0,
            light_pos: Vec3::new(3.0, 5.0, 4.0),
        }
    }
}

impl PlayInEditor {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn is_playing(&self) -> bool {
        self.state == PieState::Playing
    }

    pub fn is_active(&self) -> bool {
        matches!(self.state, PieState::Playing | PieState::Paused)
    }

    /// Démarre une session PIE à partir d'un monde sérialisé (déjà en mémoire)
    pub fn play_from_world(&mut self, world: WorldData) {
        self.snapshot = Some(world);
        self.state = PieState::Playing;
        self.elapsed = 0.0;
    }

    /// Capture l'état courant de la scène GPU et démarre la lecture
    pub fn play(&mut self, scene: &Scene, light_pos: Vec3) -> Result<(), String> {
        let json = serialize_world(&scene.nodes, light_pos)?;
        let world: WorldData =
            serde_json::from_str(&json).map_err(|e| format!("snapshot PIE invalide: {}", e))?;
        self.light_pos = light_pos;
        self.play_from_world(world);
        Ok(())
    }

    pub fn pause(&mut self) {
        if self.state == PieState::Playing {
            self.state = PieState::Paused;
        }
    }

    pub fn resume(&mut self) {
        if self.state == PieState::Paused {
            self.state = PieState::Playing;
        }
    }

    pub fn toggle_pause(&mut self) {
        match self.state {
            PieState::Playing => self.pause(),
            PieState::Paused => self.resume(),
            PieState::Stopped => {}
        }
    }

    /// Arrête la lecture et retourne l'état d'origine pour restauration
    pub fn stop(&mut self) -> Option<WorldData> {
        self.state = PieState::Stopped;
        self.elapsed = 0.0;
        self.snapshot.take()
    }

    /// Avance le temps de jeu (retourne le delta effectif, 0 si en pause/arrêt)
    pub fn tick(&mut self, dt: f32) -> f32 {
        if self.state == PieState::Playing {
            let scaled = dt * self.time_scale;
            self.elapsed += scaled;
            scaled
        } else {
            0.0
        }
    }

    /// Reconstruit les nœuds GPU depuis le snapshot (utile pour réinitialiser la scène)
    pub fn restore_nodes(
        &self,
        device: &wgpu::Device,
        layout: &wgpu::BindGroupLayout,
        default_texture: std::sync::Arc<crate::gpu::GpuTexture>,
        default_normal: std::sync::Arc<crate::gpu::GpuTexture>,
    ) -> Result<(Vec<crate::scene::node::SceneNode>, Vec3), String> {
        let world = self
            .snapshot
            .as_ref()
            .ok_or_else(|| "aucun snapshot PIE disponible".to_string())?;
        let json = serde_json::to_string(world).map_err(|e| e.to_string())?;
        deserialize_world(device, layout, default_texture, default_normal, &json)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scene::persistence::WorldData;

    fn sample_world() -> WorldData {
        WorldData {
            version: 1,
            generator: "test".into(),
            light_position: [1.0, 2.0, 3.0],
            nodes: Vec::new(),
        }
    }

    #[test]
    fn test_play_pause_stop_lifecycle() {
        let mut pie = PlayInEditor::new();
        assert!(!pie.is_active());

        pie.play_from_world(sample_world());
        assert!(pie.is_playing());

        assert!((pie.tick(0.016) - 0.016).abs() < 1e-6);

        pie.pause();
        assert_eq!(pie.state, PieState::Paused);
        assert_eq!(pie.tick(0.016), 0.0, "aucune avance en pause");

        pie.resume();
        assert!(pie.is_playing());

        let restored = pie.stop();
        assert!(restored.is_some());
        assert_eq!(pie.state, PieState::Stopped);
        assert_eq!(pie.elapsed, 0.0);
    }

    #[test]
    fn test_time_scale() {
        let mut pie = PlayInEditor::new();
        pie.time_scale = 0.5;
        pie.play_from_world(sample_world());
        assert!((pie.tick(1.0) - 0.5).abs() < 1e-6);
        assert!((pie.elapsed - 0.5).abs() < 1e-6);
    }
}
