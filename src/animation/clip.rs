// [WFGY] Zone: SAFE | λ: 0.25 | Fallbacks: 0 | Action: Keyframe Animation Tracks, Interpolation (LERP/SLERP) and Animation Player
#![allow(dead_code)]
use glam::{Quat, Vec3};
use crate::animation::skeleton::Skeleton;

/// Piste d'images clés temporelles
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Keyframe<T> {
    pub time: f32, // en secondes
    pub value: T,
}

#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct JointAnimationTrack {
    pub joint_index: usize,
    pub translation_keys: Vec<Keyframe<Vec3>>,
    pub rotation_keys: Vec<Keyframe<Quat>>,
    pub scale_keys: Vec<Keyframe<Vec3>>,
}

impl JointAnimationTrack {
    pub fn new(joint_index: usize) -> Self {
        Self {
            joint_index,
            translation_keys: Vec::new(),
            rotation_keys: Vec::new(),
            scale_keys: Vec::new(),
        }
    }

    /// Échantillonnage avec interpolation linéaire (LERP) pour la position
    pub fn sample_translation(&self, time: f32) -> Option<Vec3> {
        if self.translation_keys.is_empty() {
            return None;
        }
        if self.translation_keys.len() == 1 || time <= self.translation_keys[0].time {
            return Some(self.translation_keys[0].value);
        }
        let last_idx = self.translation_keys.len() - 1;
        if time >= self.translation_keys[last_idx].time {
            return Some(self.translation_keys[last_idx].value);
        }

        for i in 0..last_idx {
            let k0 = &self.translation_keys[i];
            let k1 = &self.translation_keys[i + 1];
            if time >= k0.time && time <= k1.time {
                let factor = (time - k0.time) / (k1.time - k0.time).max(1e-5);
                return Some(k0.value.lerp(k1.value, factor));
            }
        }

        Some(self.translation_keys[last_idx].value)
    }

    /// Échantillonnage avec interpolation sphérique (SLERP) pour les rotations
    pub fn sample_rotation(&self, time: f32) -> Option<Quat> {
        if self.rotation_keys.is_empty() {
            return None;
        }
        if self.rotation_keys.len() == 1 || time <= self.rotation_keys[0].time {
            return Some(self.rotation_keys[0].value);
        }
        let last_idx = self.rotation_keys.len() - 1;
        if time >= self.rotation_keys[last_idx].time {
            return Some(self.rotation_keys[last_idx].value);
        }

        for i in 0..last_idx {
            let k0 = &self.rotation_keys[i];
            let k1 = &self.rotation_keys[i + 1];
            if time >= k0.time && time <= k1.time {
                let factor = (time - k0.time) / (k1.time - k0.time).max(1e-5);
                return Some(k0.value.slerp(k1.value, factor));
            }
        }

        Some(self.rotation_keys[last_idx].value)
    }

    /// Échantillonnage avec interpolation linéaire pour l'échelle
    pub fn sample_scale(&self, time: f32) -> Option<Vec3> {
        if self.scale_keys.is_empty() {
            return None;
        }
        if self.scale_keys.len() == 1 || time <= self.scale_keys[0].time {
            return Some(self.scale_keys[0].value);
        }
        let last_idx = self.scale_keys.len() - 1;
        if time >= self.scale_keys[last_idx].time {
            return Some(self.scale_keys[last_idx].value);
        }

        for i in 0..last_idx {
            let k0 = &self.scale_keys[i];
            let k1 = &self.scale_keys[i + 1];
            if time >= k0.time && time <= k1.time {
                let factor = (time - k0.time) / (k1.time - k0.time).max(1e-5);
                return Some(k0.value.lerp(k1.value, factor));
            }
        }

        Some(self.scale_keys[last_idx].value)
    }
}

/// Clip complet d'animation (ex: Marche, Course, Saut, Idle)
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct AnimationClip {
    pub name: String,
    pub duration: f32, // en secondes
    pub tracks: Vec<JointAnimationTrack>,
}

impl AnimationClip {
    pub fn new(name: &str, duration: f32) -> Self {
        Self {
            name: name.to_string(),
            duration: duration.max(0.01),
            tracks: Vec::new(),
        }
    }

    /// Applique les poses du clip d'animation au squelette à un instant T donné
    pub fn apply_to_skeleton(&self, time: f32, skeleton: &mut Skeleton) {
        let t = if self.duration > 0.0 {
            time % self.duration
        } else {
            0.0
        };

        for track in &self.tracks {
            if track.joint_index < skeleton.joints.len() {
                let joint = &mut skeleton.joints[track.joint_index];
                if let Some(pos) = track.sample_translation(t) {
                    joint.local_translation = pos;
                }
                if let Some(rot) = track.sample_rotation(t) {
                    joint.local_rotation = rot;
                }
                if let Some(scale) = track.sample_scale(t) {
                    joint.local_scale = scale;
                }
            }
        }
    }
}

/// Lecteur d'animations pour piloter la lecture, le temps, la boucle et la vitesse
#[derive(Clone, Debug)]
pub struct AnimationPlayer {
    pub current_time: f32,
    pub speed: f32,
    pub is_playing: bool,
    pub is_looping: bool,
    pub current_clip_idx: usize,
    pub clips: Vec<AnimationClip>,
}

impl Default for AnimationPlayer {
    fn default() -> Self {
        Self {
            current_time: 0.0,
            speed: 1.0,
            is_playing: true,
            is_looping: true,
            current_clip_idx: 0,
            clips: Vec::new(),
        }
    }
}

impl AnimationPlayer {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_clip(&mut self, clip: AnimationClip) -> usize {
        let idx = self.clips.len();
        self.clips.push(clip);
        idx
    }

    pub fn update(&mut self, dt: f32, skeleton: &mut Skeleton) {
        if !self.is_playing || self.clips.is_empty() {
            return;
        }

        let clip = &self.clips[self.current_clip_idx];
        self.current_time += dt * self.speed;

        if self.current_time >= clip.duration {
            if self.is_looping {
                self.current_time %= clip.duration;
            } else {
                self.current_time = clip.duration;
                self.is_playing = false;
            }
        }

        clip.apply_to_skeleton(self.current_time, skeleton);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_animation_track_interpolation() {
        let mut track = JointAnimationTrack::new(0);
        track.translation_keys.push(Keyframe { time: 0.0, value: Vec3::ZERO });
        track.translation_keys.push(Keyframe { time: 1.0, value: Vec3::new(10.0, 0.0, 0.0) });

        let mid = track.sample_translation(0.5);
        assert_eq!(mid, Some(Vec3::new(5.0, 0.0, 0.0)));
    }
}
