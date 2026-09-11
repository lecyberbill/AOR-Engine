// [WFGY] Zone: SAFE | λ: 0.25 | Fallbacks: 0/None | Action: Node Graph Texture & Shader Compiler subsystem (Phase 2)
#![allow(dead_code, unused_imports)]
pub mod compiler;
pub mod graph;
pub mod helpers;
pub mod kinds;

pub use compiler::{
    coerce, compile, compile_and_validate, compile_engine_shader, validate_wgsl, CompiledShader,
    EngineShader,
};
pub use graph::{
    can_connect, GraphError, GraphNode, MaterialNodeGraph, NodeLink, NodePin, ParamValue,
};
pub use helpers::{build_helpers, HelperFlags};
pub use kinds::{NodeCategory, NodeKind, PinDataType, PinSpec};
