// [WFGY] Zone: SAFE | λ: 0.25 | Fallbacks: 0/None | Action: Node kind registry, pin layouts and categories for the material graph
#![allow(dead_code)]
use super::graph::ParamValue;
use serde::{Deserialize, Serialize};

/// Type de donnée transportée par un pin de nœud
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PinDataType {
    Float,
    Vec2,
    Vec3,
    Vec4,
    Texture2D,
    Sampler,
}

impl PinDataType {
    /// Représentation WGSL du type
    pub fn wgsl(&self) -> &'static str {
        match self {
            PinDataType::Float => "f32",
            PinDataType::Vec2 => "vec2<f32>",
            PinDataType::Vec3 => "vec3<f32>",
            PinDataType::Vec4 => "vec4<f32>",
            PinDataType::Texture2D => "texture_2d<f32>",
            PinDataType::Sampler => "sampler",
        }
    }

    /// Nombre de composantes scalaires (0 pour les ressources)
    pub fn component_count(&self) -> usize {
        match self {
            PinDataType::Float => 1,
            PinDataType::Vec2 => 2,
            PinDataType::Vec3 => 3,
            PinDataType::Vec4 => 4,
            _ => 0,
        }
    }

    pub fn is_texture_like(&self) -> bool {
        matches!(self, PinDataType::Texture2D | PinDataType::Sampler)
    }
}

/// Catégorie fonctionnelle d'un nœud (utilisée pour l'organisation du canvas)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum NodeCategory {
    Input,
    Math,
    Procedural,
    Color,
    Texture,
    Effect,
    Output,
}

/// Toutes les opérations disponibles dans l'éditeur de matériaux nodaux
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum NodeKind {
    // Input
    Uv,
    WorldPosition,
    WorldNormal,
    ViewDirection,
    Time,
    VertexColor,
    // Math
    Add,
    Sub,
    Mul,
    Div,
    Sin,
    Cos,
    Pow,
    Clamp,
    Mix,
    Remap,
    // Procedural
    PerlinNoise,
    SimplexNoise,
    Voronoi,
    Checkerboard,
    Gradient,
    // Color
    Rgb,
    Rgba,
    ColorRamp,
    Hsv,
    Desaturate,
    Invert,
    // Texture
    SampleTexture2D,
    NormalMapUnpack,
    TriplanarMapping,
    // Effect
    Fresnel,
    ParallaxOcclusion,
    DistanceToEdge,
    WaveDisplacement,
    // Output
    MaterialOutput,
}

/// Spécification d'un pin (nom + type)
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PinSpec {
    pub name: &'static str,
    pub data_type: PinDataType,
}

fn pin(name: &'static str, data_type: PinDataType) -> PinSpec {
    PinSpec { name, data_type }
}

use PinDataType::*;

impl NodeKind {
    pub fn category(&self) -> NodeCategory {
        use NodeKind::*;
        match self {
            Uv | WorldPosition | WorldNormal | ViewDirection | Time | VertexColor => {
                NodeCategory::Input
            }
            Add | Sub | Mul | Div | Sin | Cos | Pow | Clamp | Mix | Remap => NodeCategory::Math,
            PerlinNoise | SimplexNoise | Voronoi | Checkerboard | Gradient => {
                NodeCategory::Procedural
            }
            Rgb | Rgba | ColorRamp | Hsv | Desaturate | Invert => NodeCategory::Color,
            SampleTexture2D | NormalMapUnpack | TriplanarMapping => NodeCategory::Texture,
            Fresnel | ParallaxOcclusion | DistanceToEdge | WaveDisplacement => NodeCategory::Effect,
            MaterialOutput => NodeCategory::Output,
        }
    }

    pub fn label(&self) -> &'static str {
        use NodeKind::*;
        match self {
            Uv => "UV Coordinates",
            WorldPosition => "World Position",
            WorldNormal => "World Normal",
            ViewDirection => "View Direction",
            Time => "Time",
            VertexColor => "Vertex Color",
            Add => "Add",
            Sub => "Subtract",
            Mul => "Multiply",
            Div => "Divide",
            Sin => "Sin",
            Cos => "Cos",
            Pow => "Power",
            Clamp => "Clamp",
            Mix => "Mix / Lerp",
            Remap => "Remap",
            PerlinNoise => "Perlin Noise",
            SimplexNoise => "Simplex Noise",
            Voronoi => "Voronoi / Cellular",
            Checkerboard => "Checkerboard",
            Gradient => "Gradient",
            Rgb => "RGB",
            Rgba => "RGBA",
            ColorRamp => "Color Ramp",
            Hsv => "HSV",
            Desaturate => "Desaturate",
            Invert => "Invert",
            SampleTexture2D => "Sample Texture 2D",
            NormalMapUnpack => "Normal Map Unpack",
            TriplanarMapping => "Triplanar Mapping",
            Fresnel => "Fresnel Effect",
            ParallaxOcclusion => "Parallax Occlusion",
            DistanceToEdge => "Distance To Edge",
            WaveDisplacement => "Wave Displacement",
            MaterialOutput => "PBR Material Output",
        }
    }

    pub fn inputs(&self) -> Vec<PinSpec> {
        use NodeKind::*;
        match self {
            Uv | WorldPosition | WorldNormal | ViewDirection | Time | VertexColor => vec![],
            Add | Sub | Mul | Div => vec![pin("A", Vec4), pin("B", Vec4)],
            Sin | Cos => vec![pin("In", Float)],
            Pow => vec![pin("Base", Float), pin("Exponent", Float)],
            Clamp => vec![pin("Value", Float), pin("Min", Float), pin("Max", Float)],
            Mix => vec![pin("A", Vec4), pin("B", Vec4), pin("T", Float)],
            Remap => vec![
                pin("Value", Float),
                pin("In Min", Float),
                pin("In Max", Float),
                pin("Out Min", Float),
                pin("Out Max", Float),
            ],
            PerlinNoise | SimplexNoise | Voronoi => vec![pin("UV", Vec2)],
            Checkerboard => vec![pin("UV", Vec2)],
            Gradient => vec![pin("T", Float)],
            Rgb => vec![pin("R", Float), pin("G", Float), pin("B", Float)],
            Rgba => vec![pin("R", Float), pin("G", Float), pin("B", Float), pin("A", Float)],
            ColorRamp => vec![pin("T", Float)],
            Hsv | Invert => vec![pin("Color", Vec4)],
            Desaturate => vec![pin("Color", Vec4), pin("Amount", Float)],
            SampleTexture2D => vec![pin("UV", Vec2)],
            NormalMapUnpack => vec![pin("Normal Map", Vec4)],
            TriplanarMapping => vec![pin("Position", Vec3), pin("Normal", Vec3)],
            Fresnel => vec![pin("Normal", Vec3), pin("View Dir", Vec3)],
            ParallaxOcclusion => vec![pin("UV", Vec2), pin("Height", Float)],
            DistanceToEdge => vec![pin("UV", Vec2)],
            WaveDisplacement => vec![pin("Position", Vec3), pin("Time", Float)],
            MaterialOutput => vec![
                pin("Albedo", Vec4),
                pin("Normal", Vec3),
                pin("Metallic", Float),
                pin("Roughness", Float),
                pin("Emissive", Vec4),
                pin("Ambient Occlusion", Float),
                pin("Alpha", Float),
            ],
        }
    }

    pub fn outputs(&self) -> Vec<PinSpec> {
        use NodeKind::*;
        match self {
            Uv => vec![pin("UV", Vec2)],
            WorldPosition => vec![pin("Position", Vec3)],
            WorldNormal => vec![pin("Normal", Vec3)],
            ViewDirection => vec![pin("View Dir", Vec3)],
            Time => vec![pin("Time", Float)],
            VertexColor => vec![pin("Color", Vec4)],
            Add | Sub | Mul | Div | Mix => vec![pin("Out", Vec4)],
            Sin | Cos | Pow | Clamp | Remap | PerlinNoise | SimplexNoise | Voronoi
            | Checkerboard | Gradient | Fresnel | DistanceToEdge => vec![pin("Out", Float)],
            Rgb | Rgba | ColorRamp | Hsv | Desaturate | Invert | SampleTexture2D
            | TriplanarMapping => vec![pin("Color", Vec4)],
            NormalMapUnpack | WaveDisplacement => vec![pin("Out", Vec3)],
            ParallaxOcclusion => vec![pin("UV", Vec2)],
            MaterialOutput => vec![],
        }
    }

    /// Paramètres par défaut éditables d'un nœud
    pub fn default_params(&self) -> Vec<(&'static str, ParamValue)> {
        use NodeKind::*;
        match self {
            Rgb => vec![
                ("r", ParamValue::Float(1.0)),
                ("g", ParamValue::Float(1.0)),
                ("b", ParamValue::Float(1.0)),
            ],
            Rgba => vec![
                ("r", ParamValue::Float(1.0)),
                ("g", ParamValue::Float(1.0)),
                ("b", ParamValue::Float(1.0)),
                ("a", ParamValue::Float(1.0)),
            ],
            PerlinNoise | SimplexNoise | Voronoi => {
                vec![("scale", ParamValue::Float(8.0))]
            }
            Checkerboard => vec![("scale", ParamValue::Float(8.0))],
            Fresnel => vec![
                ("power", ParamValue::Float(2.0)),
                ("bias", ParamValue::Float(0.0)),
            ],
            WaveDisplacement => vec![
                ("amplitude", ParamValue::Float(0.05)),
                ("frequency", ParamValue::Float(8.0)),
                ("speed", ParamValue::Float(1.5)),
            ],
            _ => vec![],
        }
    }

    /// Valeur par défaut (au format WGSL) d'un input non connecté
    pub fn default_input_expr(&self, pin_name: &str) -> String {
        use NodeKind::*;
        match (self, pin_name) {
            (Mix, "A") | (Rgb, _) => "1.0".to_string(),
            (_, "Color") => "vec4<f32>(1.0)".to_string(),
            (MaterialOutput, "Albedo") => "vec4<f32>(1.0, 1.0, 1.0, 1.0)".to_string(),
            (MaterialOutput, "Normal") => "vec3<f32>(0.0, 0.0, 1.0)".to_string(),
            (MaterialOutput, "Emissive") => "vec4<f32>(0.0)".to_string(),
            (MaterialOutput, "Alpha") => "1.0".to_string(),
            (SampleTexture2D, "UV") => "in.uv".to_string(),
            (TriplanarMapping, "Position") => "in.world_pos".to_string(),
            (TriplanarMapping, "Normal") => "in.world_normal".to_string(),
            (Fresnel, "Normal") => "in.world_normal".to_string(),
            (Fresnel, "View Dir") => "normalize(scene.camera_pos - in.world_pos)".to_string(),
            (ParallaxOcclusion, "UV") => "in.uv".to_string(),
            (DistanceToEdge, "UV") => "in.uv".to_string(),
            (WaveDisplacement, "Position") => "in.world_pos".to_string(),
            (WaveDisplacement, "Time") => "scene.time".to_string(),
            (PerlinNoise, "UV") | (SimplexNoise, "UV") | (Voronoi, "UV")
            | (Checkerboard, "UV") => "in.uv".to_string(),
            (_, "Amount") => "1.0".to_string(),
            (_, "Min") => "0.0".to_string(),
            (_, "Max") => "1.0".to_string(),
            (_, "Exponent") => "2.0".to_string(),
            (_, "T") => "0.5".to_string(),
            (_, "Height") => "0.0".to_string(),
            (_, "Out Min") => "0.0".to_string(),
            (_, "Out Max") => "1.0".to_string(),
            (_, "In Min") => "0.0".to_string(),
            (_, "In Max") => "1.0".to_string(),
            (_, "Value") | (_, "Base") => "0.5".to_string(),
            _ => {
                let ty = self
                    .inputs()
                    .into_iter()
                    .find(|p| p.name == pin_name)
                    .map(|p| p.data_type)
                    .unwrap_or(PinDataType::Float);
                match ty {
                    PinDataType::Float => "0.5",
                    PinDataType::Vec2 => "vec2<f32>(0.5)",
                    PinDataType::Vec3 => "vec3<f32>(0.5)",
                    PinDataType::Vec4 => "vec4<f32>(0.5)",
                    _ => "0.0",
                }
                .to_string()
            }
        }
    }

    /// Toutes les variantes pour l'itération (palette UI)
    pub fn all() -> Vec<NodeKind> {
        use NodeKind::*;
        vec![
            Uv,
            WorldPosition,
            WorldNormal,
            ViewDirection,
            Time,
            VertexColor,
            Add,
            Sub,
            Mul,
            Div,
            Sin,
            Cos,
            Pow,
            Clamp,
            Mix,
            Remap,
            PerlinNoise,
            SimplexNoise,
            Voronoi,
            Checkerboard,
            Gradient,
            Rgb,
            Rgba,
            ColorRamp,
            Hsv,
            Desaturate,
            Invert,
            SampleTexture2D,
            NormalMapUnpack,
            TriplanarMapping,
            Fresnel,
            ParallaxOcclusion,
            DistanceToEdge,
            WaveDisplacement,
            MaterialOutput,
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_all_kinds_have_consistent_layout() {
        for kind in NodeKind::all() {
            // Chaque input possède une expression par défaut non vide
            for input in kind.inputs() {
                let expr = kind.default_input_expr(input.name);
                assert!(!expr.is_empty(), "{:?}.{} sans défaut", kind, input.name);
            }
            // Le nœud de sortie n'a pas de pin de sortie
            if kind == NodeKind::MaterialOutput {
                assert!(kind.outputs().is_empty());
            } else {
                assert!(!kind.outputs().is_empty(), "{:?} sans sortie", kind);
            }
        }
    }

    #[test]
    fn test_categories_assignment() {
        assert_eq!(NodeKind::PerlinNoise.category(), NodeCategory::Procedural);
        assert_eq!(NodeKind::MaterialOutput.category(), NodeCategory::Output);
        assert_eq!(NodeKind::Mix.category(), NodeCategory::Math);
        assert_eq!(NodeKind::Fresnel.category(), NodeCategory::Effect);
    }
}
