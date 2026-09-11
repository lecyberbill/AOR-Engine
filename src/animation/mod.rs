// [WFGY] Zone: SAFE | λ: 0.25 | Fallbacks: 0 | Action: Animation and Mesh Deformation module
#![allow(unused_imports, dead_code)]
pub mod skeleton;
pub mod clip;
pub mod deformation;
pub mod rigged_character;
pub mod modifier;

pub use skeleton::{Joint, Skeleton};
pub use clip::{AnimationClip, AnimationPlayer, JointAnimationTrack, Keyframe};
pub use deformation::MeshDeformer;
pub use rigged_character::{RiggedModel, SkinnedVertex};
pub use modifier::{evaluate_modifier_stack, MeshModifier};


