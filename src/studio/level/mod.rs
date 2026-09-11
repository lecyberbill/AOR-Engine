// [WFGY] Zone: SAFE | λ: 0.25 | Fallbacks: 0/None | Action: Level Design, Prefabs & Cyberpunk track tools (Phase 4)
#![allow(dead_code, unused_imports)]
pub mod scatter;
pub mod snapping;
pub mod spline;
pub mod triggers;

pub use scatter::{
    scatter_along_spline, scatter_on_grid, Rng, ScatterSettings,
};
pub use snapping::{
    apply_gizmo, raycast_surface, snap_angle, snap_rotation, snap_scale, snap_scalar,
    snap_to_surface, snap_translation, snap_vec3, GizmoMode, SnapSettings, SurfaceHit,
};
pub use spline::{
    catmull_rom, generate_ribbon_mesh, generate_tube_mesh, Spline,
};
pub use triggers::{
    FiredTrigger, TriggerAction, TriggerEvent, TriggerShape, TriggerSystem, TriggerVolume,
};
