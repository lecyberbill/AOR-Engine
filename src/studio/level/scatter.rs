// [WFGY] Zone: SAFE | λ: 0.25 | Fallbacks: 0/None | Action: Scatter/paint tool placing prefabs along splines with deterministic RNG
#![allow(dead_code)]
use super::spline::Spline;
use crate::scene::node::Transform;
use glam::{Quat, Vec3};

/// Générateur pseudo-aléatoire déterministe (PCG-like) pour un scatter reproductible
#[derive(Debug, Clone)]
pub struct Rng {
    state: u64,
}

impl Rng {
    pub fn new(seed: u64) -> Self {
        Rng {
            state: seed.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407),
        }
    }

    pub fn next_u32(&mut self) -> u32 {
        self.state = self
            .state
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        (self.state >> 33) as u32
    }

    /// Valeur dans [0, 1)
    pub fn next_f32(&mut self) -> f32 {
        (self.next_u32() as f32) / (u32::MAX as f32 + 1.0)
    }

    /// Valeur dans [min, max)
    pub fn range(&mut self, min: f32, max: f32) -> f32 {
        min + self.next_f32() * (max - min)
    }
}

/// Réglages de l'outil de dispersion (pinceau de placement)
#[derive(Debug, Clone, PartialEq)]
pub struct ScatterSettings {
    pub count: usize,
    pub lateral_spread: f32,
    pub position_jitter: f32,
    pub min_scale: f32,
    pub max_scale: f32,
    pub random_yaw: bool,
    pub seed: u64,
}

impl Default for ScatterSettings {
    fn default() -> Self {
        ScatterSettings {
            count: 24,
            lateral_spread: 2.0,
            position_jitter: 0.15,
            min_scale: 0.8,
            max_scale: 1.4,
            random_yaw: true,
            seed: 0xC0FFEE,
        }
    }
}

/// Positionne `count` objets le long d'une spline (lampadaires, débris, anneaux...)
pub fn scatter_along_spline(spline: &Spline, settings: &ScatterSettings) -> Vec<Transform> {
    if !spline.is_valid() || settings.count == 0 {
        return Vec::new();
    }
    let mut rng = Rng::new(settings.seed);
    let mut out = Vec::with_capacity(settings.count);

    for i in 0..settings.count {
        let t = (i as f32 + 0.5) / settings.count as f32;
        let center = spline.sample(t);
        let tangent = spline.tangent(t);
        let right = tangent.cross(Vec3::Y).normalize_or_zero();

        let lateral = rng.range(-settings.lateral_spread, settings.lateral_spread);
        let jitter = Vec3::new(
            rng.range(-settings.position_jitter, settings.position_jitter),
            0.0,
            rng.range(-settings.position_jitter, settings.position_jitter),
        );
        let translation = center + right * lateral + jitter;

        let yaw = if settings.random_yaw {
            rng.range(0.0, std::f32::consts::TAU)
        } else {
            tangent.y.atan2(tangent.x)
        };
        let scale = rng.range(settings.min_scale, settings.max_scale);
        let rotation = Quat::from_rotation_y(yaw).to_euler(glam::EulerRot::XYZ);

        out.push(Transform {
            translation,
            rotation: Vec3::new(rotation.0, rotation.1, rotation.2),
            scale: Vec3::splat(scale),
        });
    }

    out
}

/// Disperse des objets sur une grille de terrain avec bruit déterministe
pub fn scatter_on_grid(
    origin: Vec3,
    extent: f32,
    settings: &ScatterSettings,
) -> Vec<Transform> {
    let mut rng = Rng::new(settings.seed ^ 0x9E3779B9);
    let mut out = Vec::with_capacity(settings.count);
    for _ in 0..settings.count {
        let pos = origin
            + Vec3::new(
                rng.range(-extent, extent),
                0.0,
                rng.range(-extent, extent),
            );
        let scale = rng.range(settings.min_scale, settings.max_scale);
        let yaw = rng.range(0.0, std::f32::consts::TAU);
        out.push(Transform {
            translation: pos,
            rotation: Vec3::new(0.0, yaw, 0.0),
            scale: Vec3::splat(scale),
        });
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn line_spline() -> Spline {
        Spline::new(
            vec![Vec3::new(0.0, 0.0, 0.0), Vec3::new(0.0, 0.0, 50.0)],
            false,
        )
    }

    #[test]
    fn test_scatter_count_and_determinism() {
        let s = line_spline();
        let settings = ScatterSettings::default();
        let a = scatter_along_spline(&s, &settings);
        let b = scatter_along_spline(&s, &settings);
        assert_eq!(a.len(), settings.count);
        assert_eq!(a, b, "le scatter doit être déterministe");
    }

    #[test]
    fn test_scatter_positions_near_spline() {
        let s = line_spline();
        let settings = ScatterSettings {
            lateral_spread: 1.0,
            position_jitter: 0.1,
            ..Default::default()
        };
        for t in scatter_along_spline(&s, &settings) {
            // La spline est l'axe Z de x=0
            assert!(t.translation.x.abs() <= 1.1);
        }
    }

    #[test]
    fn test_different_seed_differs() {
        let s = line_spline();
        let a = scatter_along_spline(
            &s,
            &ScatterSettings {
                seed: 1,
                ..Default::default()
            },
        );
        let b = scatter_along_spline(
            &s,
            &ScatterSettings {
                seed: 2,
                ..Default::default()
            },
        );
        assert_ne!(a, b);
    }
}
