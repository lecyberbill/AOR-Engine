// [WFGY] Zone: SAFE | λ: 0.25 | Fallbacks: 0/None | Action: Studio UI views built on the ui-widgets (AORUI) tree (Phase 3)
#![allow(dead_code, unused_imports)]
pub mod asset_browser;
pub mod console;
pub mod hierarchy;
pub mod inspector;
pub mod layout;
pub mod node_canvas;
pub mod style;

pub use asset_browser::{AssetBrowserState, AssetRow};
pub use console::{ConsoleState, LogEntry, LogLevel};
pub use hierarchy::{HierarchyState, NodeInfo};
pub use inspector::{InspectorSection, InspectorState};
pub use layout::{
    build_studio_overlays, build_studio_shell, StudioCenterTab, StudioShellLayout,
    StudioShellState, NODE_LIBRARY,
};
pub use node_canvas::{
    build as build_node_canvas, build_painter, hit_node, CanvasTransform,
};
