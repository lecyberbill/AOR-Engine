// [WFGY] Zone: SAFE | λ: 0.2 | Fallbacks: 0/None | Action: AOR Game Studio subsystem root
#![allow(dead_code, unused_imports)]
pub mod level;
pub mod nodes;
pub mod packager;
pub mod pie;
pub mod session;
pub mod ui;
pub mod vfs;

pub use level::{Spline, SnapSettings, TriggerAction, TriggerSystem, TriggerVolume};
pub use packager::{build_pak, build_standalone, collect_dependencies, read_pak, write_pak, PakStats};
pub use pie::{PlayInEditor, PieState};
pub use session::StudioSession;

pub use nodes::{
    compile, compile_and_validate, compile_engine_shader, validate_wgsl, CompiledShader,
    EngineShader, GraphError, GraphNode,
    MaterialNodeGraph, NodeCategory, NodeKind, NodeLink, NodePin, ParamValue, PinDataType,
};
pub use vfs::{
    AssetChange, AssetDatabase, AssetId, AssetMetadata, AssetType, ChangeKind, FileWatcher,
    HotReloadManager, ImportSettings, ProjectLayout, ProjectManifest, ReloadBatch,
};
