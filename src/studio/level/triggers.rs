// [WFGY] Zone: SAFE | λ: 0.25 | Fallbacks: 0/None | Action: Visual trigger volumes, event detection and chainable gameplay actions
#![allow(dead_code)]
use crate::scene::node::Transform;
use glam::Vec3;
use std::collections::{HashMap, HashSet};

/// Forme géométrique d'un volume déclencheur
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TriggerShape {
    Box { half_extents: Vec3 },
    Sphere { radius: f32 },
}

impl TriggerShape {
    pub fn contains(&self, center: Vec3, point: Vec3) -> bool {
        match self {
            TriggerShape::Box { half_extents } => {
                let d = (point - center).abs();
                d.x <= half_extents.x && d.y <= half_extents.y && d.z <= half_extents.z
            }
            TriggerShape::Sphere { radius } => (point - center).length_squared() <= radius * radius,
        }
    }
}

/// Événements disponibles pour un volume
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TriggerEvent {
    OnEnter,
    OnExit,
    OnStay,
    OnInteract,
    OnTimer,
    OnScoreChange,
}

/// Actions chaînables déclenchées par un événement
#[derive(Debug, Clone, PartialEq)]
pub enum TriggerAction {
    PlaySound {
        id: String,
        position: Vec3,
        volume: f32,
    },
    SpawnPrefab {
        prefab: String,
        transform: Transform,
    },
    EmitParticles {
        id: String,
        count: u32,
    },
    CameraShake {
        intensity: f32,
        duration: f32,
    },
    AddScore {
        points: i32,
    },
    LoadScene {
        name: String,
    },
}

/// Volume déclencheur avec ses actions par événement
#[derive(Debug, Clone)]
pub struct TriggerVolume {
    pub id: u32,
    pub name: String,
    pub shape: TriggerShape,
    pub position: Vec3,
    pub is_trigger: bool,
    pub on_enter: Vec<TriggerAction>,
    pub on_exit: Vec<TriggerAction>,
    pub on_stay: Vec<TriggerAction>,
    pub on_interact: Vec<TriggerAction>,
}

impl TriggerVolume {
    pub fn new_box(name: impl Into<String>, position: Vec3, half_extents: Vec3) -> Self {
        TriggerVolume {
            id: 0,
            name: name.into(),
            shape: TriggerShape::Box { half_extents },
            position,
            is_trigger: true,
            on_enter: Vec::new(),
            on_exit: Vec::new(),
            on_stay: Vec::new(),
            on_interact: Vec::new(),
        }
    }

    pub fn new_sphere(name: impl Into<String>, position: Vec3, radius: f32) -> Self {
        TriggerVolume {
            id: 0,
            name: name.into(),
            shape: TriggerShape::Sphere { radius },
            position,
            is_trigger: true,
            on_enter: Vec::new(),
            on_exit: Vec::new(),
            on_stay: Vec::new(),
            on_interact: Vec::new(),
        }
    }

    pub fn on_enter(mut self, action: TriggerAction) -> Self {
        self.on_enter.push(action);
        self
    }

    pub fn on_exit(mut self, action: TriggerAction) -> Self {
        self.on_exit.push(action);
        self
    }

    pub fn on_stay(mut self, action: TriggerAction) -> Self {
        self.on_stay.push(action);
        self
    }

    pub fn on_interact(mut self, action: TriggerAction) -> Self {
        self.on_interact.push(action);
        self
    }
}

/// Événement déclenché, journalisé par le système
#[derive(Debug, Clone, PartialEq)]
pub struct FiredTrigger {
    pub event: TriggerEvent,
    pub volume_id: u32,
    pub volume_name: String,
    pub action: TriggerAction,
}

/// Système central gérant la détection et l'exécution des actions
#[derive(Debug, Default)]
pub struct TriggerSystem {
    pub volumes: Vec<TriggerVolume>,
    pub inside: HashSet<u32>,
    pub score: i32,
    pub fired: Vec<FiredTrigger>,
    pub timers: HashMap<u32, f32>,
    next_id: u32,
}

impl TriggerSystem {
    pub fn new() -> Self {
        TriggerSystem::default()
    }

    pub fn add_volume(&mut self, mut volume: TriggerVolume) -> u32 {
        volume.id = self.next_id;
        self.next_id += 1;
        let id = volume.id;
        self.volumes.push(volume);
        id
    }

    pub fn volume(&self, id: u32) -> Option<&TriggerVolume> {
        self.volumes.iter().find(|v| v.id == id)
    }

    /// Teste le volume sous une position (retourne son id si contenu)
    pub fn volume_at(&self, point: Vec3) -> Option<u32> {
        self.volumes
            .iter()
            .find(|v| v.is_trigger && v.shape.contains(v.position, point))
            .map(|v| v.id)
    }

    fn apply(&mut self, event: TriggerEvent, volume: &TriggerVolume, actions: &[TriggerAction]) {
        for action in actions {
            if let TriggerAction::AddScore { points } = action {
                self.score += points;
            }
            self.fired.push(FiredTrigger {
                event,
                volume_id: volume.id,
                volume_name: volume.name.clone(),
                action: action.clone(),
            });
        }
    }

    /// Met à jour la détection des déclencheurs par rapport à la position (ex: joueur)
    pub fn update(&mut self, _dt: f32, entity_pos: Vec3) -> Vec<TriggerAction> {
        let mut actions = Vec::new();
        let volumes = self.volumes.clone();

        for volume in &volumes {
            let now_inside = volume.is_trigger && volume.shape.contains(volume.position, entity_pos);
            let was_inside = self.inside.contains(&volume.id);

            if now_inside && !was_inside {
                self.inside.insert(volume.id);
                self.apply(TriggerEvent::OnEnter, volume, &volume.on_enter);
                actions.extend(volume.on_enter.clone());
            } else if !now_inside && was_inside {
                self.inside.remove(&volume.id);
                self.apply(TriggerEvent::OnExit, volume, &volume.on_exit);
                actions.extend(volume.on_exit.clone());
            } else if now_inside {
                self.apply(TriggerEvent::OnStay, volume, &volume.on_stay);
                actions.extend(volume.on_stay.clone());
            }
        }

        actions
    }

    /// Déclenche les actions d'interaction d'un volume
    pub fn interact(&mut self, volume_id: u32) -> Vec<TriggerAction> {
        if let Some(volume) = self.volume(volume_id).cloned() {
            self.apply(TriggerEvent::OnInteract, &volume, &volume.on_interact);
            volume.on_interact
        } else {
            Vec::new()
        }
    }

    /// Récupère et vide le journal des déclenchements
    pub fn drain_fired(&mut self) -> Vec<FiredTrigger> {
        std::mem::take(&mut self.fired)
    }

    pub fn reset(&mut self) {
        self.inside.clear();
        self.fired.clear();
        self.score = 0;
        self.timers.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_box_contains() {
        let v = TriggerVolume::new_box("Zone", Vec3::ZERO, Vec3::splat(2.0));
        assert!(v.shape.contains(Vec3::ZERO, Vec3::new(1.0, 1.0, 1.0)));
        assert!(!v.shape.contains(Vec3::ZERO, Vec3::new(3.0, 0.0, 0.0)));
    }

    #[test]
    fn test_enter_exit_and_score() {
        let mut sys = TriggerSystem::new();
        let id = sys.add_volume(
            TriggerVolume::new_sphere("Checkpoint", Vec3::ZERO, 3.0)
                .on_enter(TriggerAction::AddScore { points: 500 })
                .on_enter(TriggerAction::PlaySound {
                    id: "laser".into(),
                    position: Vec3::ZERO,
                    volume: 1.0,
                })
                .on_exit(TriggerAction::AddScore { points: -10 }),
        );

        // Dehors -> dedans
        let actions = sys.update(0.016, Vec3::new(5.0, 0.0, 0.0));
        assert!(actions.is_empty());

        let actions = sys.update(0.016, Vec3::ZERO);
        assert_eq!(actions.len(), 2);
        assert_eq!(sys.score, 500);
        assert!(sys.inside.contains(&id));

        // Rester sur place ne doit pas redéclencher OnEnter
        sys.update(0.016, Vec3::ZERO);
        assert_eq!(sys.score, 500);

        // Sortir déclenche OnExit
        sys.update(0.016, Vec3::new(10.0, 0.0, 0.0));
        assert_eq!(sys.score, 490);
        assert!(!sys.inside.contains(&id));
    }

    #[test]
    fn test_interact_and_drain() {
        let mut sys = TriggerSystem::new();
        let id = sys.add_volume(
            TriggerVolume::new_box("Bouton", Vec3::ZERO, Vec3::ONE).on_interact(
                TriggerAction::LoadScene {
                    name: "NeonTunnel".into(),
                },
            ),
        );
        let actions = sys.interact(id);
        assert_eq!(actions.len(), 1);
        let fired = sys.drain_fired();
        assert_eq!(fired.len(), 1);
        assert_eq!(fired[0].event, TriggerEvent::OnInteract);
        assert!(sys.fired.is_empty());
    }
}
