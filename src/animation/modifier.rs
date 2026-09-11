// [WFGY] Zone: SAFE | λ: 0.25 | Fallbacks: 0 | Action: Non-destructive Mesh Modifier Stack (Bend, Twist, Taper, Wave, Noise, Squash)
#![allow(dead_code)]
use glam::Vec3;
use crate::scene::mesh::Vertex;

/// Déformateur non destructif applicable à un maillage
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum MeshModifier {
    /// Torsion hélicoïdale le long de l'axe Y
    Twist {
        enabled: bool,
        angle: f32, // en radians
    },
    /// Courbure / Flexion le long de l'axe Y vers X/Z
    Bend {
        enabled: bool,
        angle: f32, // en radians
    },
    /// Évasement / Rétrécissement le long de la hauteur Y
    Taper {
        enabled: bool,
        factor: f32, // facteur d'échelle au sommet (-1.0 à 2.0)
    },
    /// Ondulation dynamique sinusoidale
    Wave {
        enabled: bool,
        amplitude: f32,
        frequency: f32,
        speed: f32,
    },
    /// Bruit / Relief procédural
    Noise {
        enabled: bool,
        strength: f32,
        frequency: f32,
    },
    /// Déformation Squash & Stretch élastique
    Squash {
        enabled: bool,
        intensity: f32,
    },
}

impl MeshModifier {
    pub fn is_enabled(&self) -> bool {
        match self {
            MeshModifier::Twist { enabled, .. } => *enabled,
            MeshModifier::Bend { enabled, .. } => *enabled,
            MeshModifier::Taper { enabled, .. } => *enabled,
            MeshModifier::Wave { enabled, .. } => *enabled,
            MeshModifier::Noise { enabled, .. } => *enabled,
            MeshModifier::Squash { enabled, .. } => *enabled,
        }
    }

    pub fn set_enabled(&mut self, val: bool) {
        match self {
            MeshModifier::Twist { enabled, .. } => *enabled = val,
            MeshModifier::Bend { enabled, .. } => *enabled = val,
            MeshModifier::Taper { enabled, .. } => *enabled = val,
            MeshModifier::Wave { enabled, .. } => *enabled = val,
            MeshModifier::Noise { enabled, .. } => *enabled = val,
            MeshModifier::Squash { enabled, .. } => *enabled = val,
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            MeshModifier::Twist { .. } => "🌀 Twist (Torsion)",
            MeshModifier::Bend { .. } => "🎋 Bend (Courbure)",
            MeshModifier::Taper { .. } => "📐 Taper (Évasement)",
            MeshModifier::Wave { .. } => "🌊 Wave (Vague)",
            MeshModifier::Noise { .. } => "🏔 Noise (Relief)",
            MeshModifier::Squash { .. } => "🍮 Squash & Stretch",
        }
    }
}

/// Évalue une pile complète de modificateurs non destructifs sur une géométrie de base
pub fn evaluate_modifier_stack(
    base_vertices: &[Vertex],
    modifiers: &[MeshModifier],
    time: f32,
) -> Vec<Vertex> {
    if modifiers.is_empty() || !modifiers.iter().any(|m| m.is_enabled()) {
        return base_vertices.to_vec();
    }

    let mut out_vertices = base_vertices.to_vec();

    // Trouver les bornes Y de la géométrie de base pour normaliser les calculs
    let mut min_y = f32::MAX;
    let mut max_y = f32::MIN;
    for v in base_vertices {
        min_y = min_y.min(v.position[1]);
        max_y = max_y.max(v.position[1]);
    }
    let height = (max_y - min_y).max(0.001);

    for modifier in modifiers {
        if !modifier.is_enabled() {
            continue;
        }

        match modifier {
            MeshModifier::Twist { angle, .. } => {
                for v in &mut out_vertices {
                    let norm_y = (v.position[1] - min_y) / height; // 0.0 à 1.0
                    let theta = norm_y * angle;
                    let cos_t = theta.cos();
                    let sin_t = theta.sin();

                    let x = v.position[0];
                    let z = v.position[2];

                    v.position[0] = x * cos_t - z * sin_t;
                    v.position[2] = x * sin_t + z * cos_t;

                    // Rotation normale
                    let nx = v.normal[0];
                    let nz = v.normal[2];
                    v.normal[0] = nx * cos_t - nz * sin_t;
                    v.normal[2] = nx * sin_t + nz * cos_t;
                }
            }
            MeshModifier::Bend { angle, .. } => {
                if angle.abs() < 1e-4 {
                    continue;
                }
                for v in &mut out_vertices {
                    let norm_y = (v.position[1] - min_y) / height;
                    let theta = norm_y * angle;
                    let radius = height / angle;

                    let x = v.position[0];

                    // Déplacement en arc de cercle le long de X
                    v.position[0] = x + (1.0 - theta.cos()) * radius * 0.5;
                    v.position[1] = min_y + theta.sin() * radius;


                    // Ajustement de la normale
                    let cos_t = theta.cos();
                    let sin_t = theta.sin();
                    let ny = v.normal[1];
                    let nx = v.normal[0];
                    v.normal[0] = nx * cos_t - ny * sin_t;
                    v.normal[1] = nx * sin_t + ny * cos_t;
                }
            }
            MeshModifier::Taper { factor, .. } => {
                for v in &mut out_vertices {
                    let norm_y = (v.position[1] - min_y) / height;
                    let scale = (1.0 + norm_y * factor).max(0.01);

                    v.position[0] *= scale;
                    v.position[2] *= scale;
                }
            }
            MeshModifier::Wave { amplitude, frequency, speed, .. } => {
                for v in &mut out_vertices {
                    let dist = (v.position[0] * v.position[0] + v.position[2] * v.position[2]).sqrt();
                    let wave = (dist * frequency - time * speed).sin() * amplitude;
                    v.position[1] += wave;
                }
            }
            MeshModifier::Noise { strength, frequency, .. } => {
                for v in &mut out_vertices {
                    let nx = (v.position[0] * frequency + 1.2).sin();
                    let ny = (v.position[1] * frequency + 2.5).cos();
                    let nz = (v.position[2] * frequency + 0.8).sin();
                    let noise_disp = (nx + ny + nz) * strength * 0.333;

                    v.position[0] += v.normal[0] * noise_disp;
                    v.position[1] += v.normal[1] * noise_disp;
                    v.position[2] += v.normal[2] * noise_disp;
                }
            }
            MeshModifier::Squash { intensity, .. } => {
                let bounce = (time * 6.0).sin() * intensity;
                let factor_y = 1.0 + bounce;
                let factor_xz = 1.0 / (factor_y.max(0.1)).sqrt();

                for v in &mut out_vertices {
                    v.position[0] *= factor_xz;
                    v.position[1] *= factor_y;
                    v.position[2] *= factor_xz;
                }
            }
        }
    }

    out_vertices
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_modifier_twist_non_destructive() {
        let base = vec![
            Vertex {
                position: [1.0, 2.0, 0.0],
                normal: [1.0, 0.0, 0.0],
                tangent: [0.0, 1.0, 0.0, 1.0],
                uv: [0.0, 0.0],
                color: [1.0, 1.0, 1.0, 1.0],
            },
            Vertex {
                position: [0.0, 0.0, 0.0],
                normal: [0.0, 1.0, 0.0],
                tangent: [1.0, 0.0, 0.0, 1.0],
                uv: [0.0, 0.0],
                color: [1.0, 1.0, 1.0, 1.0],
            },
        ];

        let modifiers = vec![MeshModifier::Twist {
            enabled: true,
            angle: std::f32::consts::PI * 0.5,
        }];

        let result = evaluate_modifier_stack(&base, &modifiers, 0.0);
        assert_eq!(result.len(), base.len());
        // La géométrie de base ne doit pas avoir changé
        assert_eq!(base[0].position[0], 1.0);
        // Le résultat déformé a tourné
        assert_ne!(result[0].position[0], 1.0);
    }
}
