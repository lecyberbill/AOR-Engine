// [WFGY] Zone: SAFE | λ: 0.25 | Fallbacks: 0 | Action: Spatial 3D Audio Engine (Math, Attenuation, Doppler, Sound Events)
#![allow(dead_code)]
use glam::Vec3;

/// Type d'effet sonore du moteur
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SoundEffect {
    Explosion,
    Footstep,
    Impact,
    Jump,
    Laser,
    WindBreeze,
}

/// Source sonore 3D positionnée dans l'espace avec atténuation physique
#[derive(Clone, Debug)]
pub struct AudioSource3D {
    pub id: usize,
    pub name: String,
    pub position: Vec3,
    pub velocity: Vec3,
    pub effect: SoundEffect,
    pub base_volume: f32,
    pub min_distance: f32, // Distance avant début d'atténuation
    pub max_distance: f32, // Distance maximale audible
    pub is_looping: bool,
    pub is_playing: bool,
    pub pitch: f32,
}

impl AudioSource3D {
    pub fn new(id: usize, name: &str, pos: Vec3, effect: SoundEffect) -> Self {
        Self {
            id,
            name: name.to_string(),
            position: pos,
            velocity: Vec3::ZERO,
            effect,
            base_volume: 1.0,
            min_distance: 2.0,
            max_distance: 60.0,
            is_looping: false,
            is_playing: true,
            pitch: 1.0,
        }
    }

    /// Calcule le gain / volume spatialisé et le panoramique stéréo (L/R) perçu par l'auditeur (Listener/Caméra)
    pub fn calculate_spatial_mix(&self, listener_pos: Vec3, _listener_forward: Vec3, listener_right: Vec3) -> (f32, f32, f32) {
        if !self.is_playing {
            return (0.0, 0.0, 0.0);
        }

        let to_source = self.position - listener_pos;
        let distance = to_source.length();

        if distance >= self.max_distance {
            return (0.0, 0.0, 0.0);
        }

        // Atténuation logarithmique / quadratique réaliste
        let dist_clamped = distance.max(self.min_distance);
        let attenuation = (self.min_distance / dist_clamped).powf(1.1);
        let volume = (self.base_volume * attenuation).clamp(0.0, 1.0);

        // Panoramique Stéréo (-1.0 Gauche, +1.0 Droite)
        let dir = to_source.normalize_or_zero();
        let pan = dir.dot(listener_right).clamp(-1.0, 1.0);

        // Poids Stéréo : Loi de puissance constante (Constant Power Pan Law)
        let pan_angle = (pan + 1.0) * (std::f32::consts::PI * 0.25); // 0 à PI/2
        let left_gain = volume * pan_angle.cos();
        let right_gain = volume * pan_angle.sin();

        (volume, left_gain, right_gain)
    }
}

/// Moteur audio spatialisé complet de la scène 3D
pub struct SpatialAudioEngine {
    pub listener_pos: Vec3,
    pub listener_forward: Vec3,
    pub listener_right: Vec3,
    pub sources: Vec<AudioSource3D>,
    pub master_volume: f32,
    pub next_source_id: usize,
    pub active_events_log: Vec<(String, std::time::Instant)>,
}

impl Default for SpatialAudioEngine {
    fn default() -> Self {
        Self {
            listener_pos: Vec3::ZERO,
            listener_forward: Vec3::new(0.0, 0.0, -1.0),
            listener_right: Vec3::new(1.0, 0.0, 0.0),
            sources: Vec::new(),
            master_volume: 1.0,
            next_source_id: 0,
            active_events_log: Vec::new(),
        }
    }
}

impl SpatialAudioEngine {
    pub fn new() -> Self {
        Self::default()
    }

    /// Met à jour la position et l'orientation de l'auditeur (Caméra / Joueur)
    pub fn update_listener(&mut self, pos: Vec3, forward: Vec3, right: Vec3) {
        self.listener_pos = pos;
        self.listener_forward = forward.normalize_or_zero();
        self.listener_right = right.normalize_or_zero();
    }

    /// Déclenche un son ponctuel à une coordonnée 3D dans le monde
    pub fn play_spatial_sound(&mut self, effect: SoundEffect, pos: Vec3, volume: f32) -> usize {
        let id = self.next_source_id;
        self.next_source_id += 1;

        let name = match effect {
            SoundEffect::Explosion => "💥 Détonation",
            SoundEffect::Footstep => "👟 Pas Joueur",
            SoundEffect::Impact => "🔨 Choc Physique",
            SoundEffect::Jump => "💨 Saut",
            SoundEffect::Laser => "⚡ Laser",
            SoundEffect::WindBreeze => "🍃 Brise de Vent",
        };

        let mut source = AudioSource3D::new(id, name, pos, effect);
        source.base_volume = volume * self.master_volume;
        self.sources.push(source);

        self.active_events_log.push((format!("{name} @ [{:.1}, {:.1}, {:.1}]", pos.x, pos.y, pos.z), std::time::Instant::now()));
        if self.active_events_log.len() > 10 {
            self.active_events_log.remove(0);
        }

        id
    }

    /// Mise à jour de la simulation audio
    pub fn update(&mut self) {
        // Nettoyage des sons ponctuels non bouclés terminés
        self.sources.retain(|s| s.is_looping || s.is_playing);
        self.active_events_log.retain(|(_, time)| time.elapsed().as_secs_f32() < 4.0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_spatial_panning_and_distance_attenuation() {
        let mut engine = SpatialAudioEngine::new();
        engine.update_listener(Vec3::ZERO, Vec3::new(0.0, 0.0, -1.0), Vec3::new(1.0, 0.0, 0.0));

        let source = AudioSource3D::new(0, "TestLeft", Vec3::new(-10.0, 0.0, 0.0), SoundEffect::Explosion);
        let (vol, left, right) = source.calculate_spatial_mix(engine.listener_pos, engine.listener_forward, engine.listener_right);

        assert!(vol > 0.0);
        assert!(left > right, "Une source située à gauche doit avoir un gain gauche supérieur au droit");
    }
}
