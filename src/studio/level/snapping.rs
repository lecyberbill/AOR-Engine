// [WFGY] Zone: SAFE | λ: 0.25 | Fallbacks: 0/None | Action: Gizmo snapping (grid, angle, scale) and surface raycast snapping
#![allow(dead_code)]
use crate::scene::node::Transform;
use crate::scene::raycast::Ray;
use crate::scene::Scene;
use glam::Vec3;

/// Mode d'édition de gizmo
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GizmoMode {
    Translate,
    Rotate,
    Scale,
}

/// Configuration de magnétisme et d'aimantation à la surface
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SnapSettings {
    pub grid: Option<f32>,
    pub angle_deg: Option<f32>,
    pub scale_step: Option<f32>,
    pub surface: bool,
}

impl Default for SnapSettings {
    fn default() -> Self {
        SnapSettings {
            grid: Some(1.0),
            angle_deg: Some(15.0),
            scale_step: Some(0.25),
            surface: true,
        }
    }
}

/// Magnétise une valeur scalaire à un pas donné
pub fn snap_scalar(value: f32, step: f32) -> f32 {
    if step <= 0.0 {
        return value;
    }
    (value / step).round() * step
}

/// Magnétise un vecteur composante par composante
pub fn snap_vec3(v: Vec3, step: f32) -> Vec3 {
    Vec3::new(
        snap_scalar(v.x, step),
        snap_scalar(v.y, step),
        snap_scalar(v.z, step),
    )
}

/// Magnétise un angle (radians) à un pas exprimé en degrés
pub fn snap_angle(radians: f32, step_deg: f32) -> f32 {
    if step_deg <= 0.0 {
        return radians;
    }
    let step = step_deg.to_radians();
    (radians / step).round() * step
}

pub fn snap_translation(t: Vec3, settings: &SnapSettings) -> Vec3 {
    match settings.grid {
        Some(step) => snap_vec3(t, step),
        None => t,
    }
}

pub fn snap_rotation(r: Vec3, settings: &SnapSettings) -> Vec3 {
    match settings.angle_deg {
        Some(step) => Vec3::new(
            snap_angle(r.x, step),
            snap_angle(r.y, step),
            snap_angle(r.z, step),
        ),
        None => r,
    }
}

pub fn snap_scale(s: Vec3, settings: &SnapSettings) -> Vec3 {
    match settings.scale_step {
        Some(step) => snap_vec3(s, step).max(Vec3::splat(0.001)),
        None => s,
    }
}

/// Applique un delta de gizmo à un transform en respectant le magnétisme
pub fn apply_gizmo(
    base: &Transform,
    delta: Vec3,
    mode: GizmoMode,
    settings: &SnapSettings,
) -> Transform {
    let mut out = *base;
    match mode {
        GizmoMode::Translate => {
            let target = base.translation + delta;
            out.translation = snap_translation(target, settings);
        }
        GizmoMode::Rotate => {
            let target = base.rotation + delta;
            out.rotation = snap_rotation(target, settings);
        }
        GizmoMode::Scale => {
            let target = base.scale + delta;
            out.scale = snap_scale(target, settings);
        }
    }
    out
}

/// Résultat d'un raycast de surface
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SurfaceHit {
    pub node_idx: usize,
    pub point: Vec3,
    pub normal: Vec3,
    pub distance: f32,
}

/// Détermine la normale dominante d'une AABB au point d'impact
fn dominant_normal(local_min: Vec3, local_max: Vec3, local_point: Vec3) -> Vec3 {
    let center = (local_min + local_max) * 0.5;
    let half = (local_max - local_min) * 0.5;
    let d = local_point - center;
    let nx = (d.x.abs() / half.x.max(1e-6)).max(0.0);
    let ny = (d.y.abs() / half.y.max(1e-6)).max(0.0);
    let nz = (d.z.abs() / half.z.max(1e-6)).max(0.0);
    if nx >= ny && nx >= nz {
        Vec3::new(d.x.signum(), 0.0, 0.0)
    } else if ny >= nz {
        Vec3::new(0.0, d.y.signum(), 0.0)
    } else {
        Vec3::new(0.0, 0.0, d.z.signum())
    }
}

/// Effectue un raycast et retourne le premier point d'impact solide de la scène
pub fn raycast_surface(scene: &Scene, ray: &Ray) -> Option<SurfaceHit> {
    let mut best: Option<SurfaceHit> = None;
    for (idx, node) in scene.nodes.iter().enumerate() {
        let aabb = node.local_aabb();
        let model = node.transform.matrix();
        if let Some(t) = ray.intersect_obb(&aabb, model) {
            if t >= 0.0 && best.map(|b| t < b.distance).unwrap_or(true) {
                let world_point = ray.at(t);
                let inv = model.inverse();
                let local_point = inv.transform_point3(world_point);
                let local_normal = dominant_normal(aabb.min, aabb.max, local_point);
                let normal =
                    (model.inverse().transpose().transform_vector3(local_normal)).normalize_or_zero();
                best = Some(SurfaceHit {
                    node_idx: idx,
                    point: world_point,
                    normal,
                    distance: t,
                });
            }
        }
    }
    best
}

/// Aimante un transform à la surface touchée par le rayon (retourne true si appliqué)
pub fn snap_to_surface(
    transform: &mut Transform,
    scene: &Scene,
    ray: &Ray,
    settings: &SnapSettings,
    surface_offset: f32,
) -> bool {
    if !settings.surface {
        return false;
    }
    if let Some(hit) = raycast_surface(scene, ray) {
        transform.translation = snap_translation(hit.point + hit.normal * surface_offset, settings);
        true
    } else {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::f32::consts::PI;

    #[test]
    fn test_snap_scalar_and_vec() {
        assert!((snap_scalar(1.24, 0.5) - 1.0).abs() < 1e-5);
        assert!((snap_scalar(1.26, 0.5) - 1.5).abs() < 1e-5);
        let v = snap_vec3(Vec3::new(0.9, 1.1, -0.6), 1.0);
        assert_eq!(v, Vec3::new(1.0, 1.0, -1.0));
    }

    #[test]
    fn test_snap_angle() {
        let a = snap_angle(0.2, 15.0); // ~0.2618 rad = 15°
        assert!((a - 15.0f32.to_radians()).abs() < 1e-4);
        let b = snap_angle(PI, 45.0);
        assert!((b - PI).abs() < 1e-4 || (b - PI).abs() < 1e-4);
    }

    #[test]
    fn test_gizmo_translate_snaps() {
        let base = Transform::default();
        let settings = SnapSettings {
            grid: Some(0.5),
            ..Default::default()
        };
        let out = apply_gizmo(
            &base,
            Vec3::new(1.3, 0.2, -0.4),
            GizmoMode::Translate,
            &settings,
        );
        assert_eq!(out.translation, Vec3::new(1.5, 0.0, -0.5));
    }

    #[test]
    fn test_snap_disabled_passthrough() {
        let settings = SnapSettings {
            grid: None,
            ..Default::default()
        };
        assert_eq!(
            snap_translation(Vec3::new(1.234, 2.345, 3.456), &settings),
            Vec3::new(1.234, 2.345, 3.456)
        );
    }
}
