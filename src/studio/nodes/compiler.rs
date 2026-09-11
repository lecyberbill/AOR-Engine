// [WFGY] Zone: RISK | λ: 0.3 | Fallbacks: 0/validate | Action: DAG traversal & WGSL code generation with Naga validation
#![allow(dead_code)]
use super::graph::{GraphError, MaterialNodeGraph, ParamValue};
use super::helpers::{build_helpers, HelperFlags};
use super::kinds::{NodeKind, PinDataType};
use std::collections::HashMap;
use uuid::Uuid;

/// Résultat d'une compilation de graphe nodal en WGSL
#[derive(Debug, Clone)]
pub struct CompiledShader {
    pub wgsl: String,
    pub helpers: Vec<&'static str>,
    pub texture_bindings: usize,
    pub node_count: usize,
}

impl CompiledShader {
    /// Valide le WGSL généré via l'analyseur Naga (aucun crash, erreurs propres)
    pub fn validate(&self) -> Result<(), String> {
        validate_wgsl(&self.wgsl)
    }
}

/// En-tête WGSL commun (structures et uniforms)
fn header() -> String {
    r#"
struct SceneUniforms {
    time: f32,
    camera_pos: vec3<f32>,
};

@group(0) @binding(0) var<uniform> scene: SceneUniforms;

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) world_pos: vec3<f32>,
    @location(2) world_normal: vec3<f32>,
    @location(3) color: vec4<f32>,
};

struct FragmentOutput {
    @location(0) albedo: vec4<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) metallic_roughness: vec4<f32>,
    @location(3) emissive: vec4<f32>,
};
"#
    .to_string()
}

/// Déclarations de texture/sampler pour les nœuds d'échantillonnage
fn texture_decls(count: usize) -> String {
    if count == 0 {
        return String::new();
    }
    let mut s = String::from("\n@group(1) @binding(0) var aor_tex_sampler: sampler;\n");
    for i in 0..count {
        s.push_str(&format!(
            "@group(1) @binding({}) var aor_tex_{}: texture_2d<f32>;\n",
            i + 1,
            i
        ));
    }
    s
}

fn sanitize(name: &str) -> String {
    name.chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
        .collect()
}

fn default_expr(kind: NodeKind, pin_name: &str) -> String {
    kind.default_input_expr(pin_name)
}

/// Associe un pin d'entrée à une clé de paramètre éditable (constantes de couleur)
fn param_key_for(kind: NodeKind, pin: &str) -> Option<&'static str> {
    use NodeKind::*;
    match (kind, pin) {
        (Rgb, "R") | (Rgba, "R") => Some("r"),
        (Rgb, "G") | (Rgba, "G") => Some("g"),
        (Rgb, "B") | (Rgba, "B") => Some("b"),
        (Rgba, "A") => Some("a"),
        _ => None,
    }
}

/// Valeur par défaut d'une entrée : paramètre éditable si présent, sinon défaut du type
fn input_default(
    kind: NodeKind,
    params: &HashMap<String, ParamValue>,
    pin: &str,
) -> String {
    if let Some(key) = param_key_for(kind, pin) {
        if let Some(v) = params.get(key) {
            return v.as_f32().to_string();
        }
    }
    default_expr(kind, pin)
}

/// Promotion/élagage de type pour raccorder une sortie à une entrée
pub fn coerce(expr: &str, from: PinDataType, to: PinDataType) -> String {
    if from == to {
        return expr.to_string();
    }
    use PinDataType::*;
    match (from, to) {
        (Float, Vec2) => format!("vec2<f32>({})", expr),
        (Float, Vec3) => format!("vec3<f32>({})", expr),
        (Float, Vec4) => format!("vec4<f32>({})", expr),
        (Vec2, Vec3) => format!("vec3<f32>({}, 0.0)", expr),
        (Vec2, Vec4) => format!("vec4<f32>({}, 0.0, 1.0)", expr),
        (Vec3, Vec4) => format!("vec4<f32>({}, 1.0)", expr),
        (Vec4, Vec3) => format!("({}).xyz", expr),
        (Vec4, Vec2) => format!("({}).xy", expr),
        (Vec3, Vec2) => format!("({}).xy", expr),
        (Vec3, Float) => format!("({}).x", expr),
        (Vec4, Float) => format!("({}).x", expr),
        (Vec2, Float) => format!("({}).x", expr),
        _ => expr.to_string(),
    }
}

/// Génère les expressions WGSL de sortie d'un nœud (alignées sur `node.outputs`)
fn emit_node(
    kind: NodeKind,
    inputs: &HashMap<String, String>,
    params: &HashMap<String, ParamValue>,
    flags: &mut HelperFlags,
    tex_count: &mut usize,
) -> Vec<String> {
    let i = |n: &str| -> String {
        inputs
            .get(n)
            .cloned()
            .unwrap_or_else(|| default_expr(kind, n))
    };
    let pf = |key: &str, default: f32| -> f32 {
        params.get(key).map(|p| p.as_f32()).unwrap_or(default)
    };
    let fmt_f = |v: f32| -> String {
        if v.fract() == 0.0 {
            format!("{:.1}", v)
        } else {
            format!("{}", v)
        }
    };

    use NodeKind::*;
    match kind {
        Uv => vec!["in.uv".to_string()],
        WorldPosition => vec!["in.world_pos".to_string()],
        WorldNormal => vec!["in.world_normal".to_string()],
        ViewDirection => vec!["normalize(scene.camera_pos - in.world_pos)".to_string()],
        Time => vec!["scene.time".to_string()],
        VertexColor => vec!["in.color".to_string()],

        Add => vec![format!("(({}) + ({}))", i("A"), i("B"))],
        Sub => vec![format!("(({}) - ({}))", i("A"), i("B"))],
        Mul => vec![format!("(({}) * ({}))", i("A"), i("B"))],
        Div => vec![format!("(({}) / ({}))", i("A"), i("B"))],
        Sin => vec![format!("sin({})", i("In"))],
        Cos => vec![format!("cos({})", i("In"))],
        Pow => vec![format!("pow({}, {})", i("Base"), i("Exponent"))],
        Clamp => vec![format!("clamp({}, {}, {})", i("Value"), i("Min"), i("Max"))],
        Mix => vec![format!("mix({}, {}, {})", i("A"), i("B"), i("T"))],
        Remap => {
            flags.remap = true;
            vec![format!(
                "aor_remap({}, {}, {}, {}, {})",
                i("Value"),
                i("In Min"),
                i("In Max"),
                i("Out Min"),
                i("Out Max")
            )]
        }

        PerlinNoise => {
            flags.perlin = true;
            vec![format!("aor_perlin2({} * {})", i("UV"), fmt_f(pf("scale", 8.0)))]
        }
        SimplexNoise => {
            flags.simplex = true;
            vec![format!("aor_simplex2({} * {})", i("UV"), fmt_f(pf("scale", 8.0)))]
        }
        Voronoi => {
            flags.voronoi = true;
            vec![format!("aor_voronoi2({} * {})", i("UV"), fmt_f(pf("scale", 8.0)))]
        }
        Checkerboard => {
            flags.checker = true;
            vec![format!("aor_checker({}, {})", i("UV"), fmt_f(pf("scale", 8.0)))]
        }
        Gradient => {
            flags.gradient = true;
            vec![format!("aor_gradient({})", i("T"))]
        }

        Rgb => vec![format!(
            "vec4<f32>({}, {}, {}, 1.0)",
            i("R"),
            i("G"),
            i("B")
        )],
        Rgba => vec![format!(
            "vec4<f32>({}, {}, {}, {})",
            i("R"),
            i("G"),
            i("B"),
            i("A")
        )],
        ColorRamp => {
            let t = i("T");
            vec![format!(
                "vec4<f32>(clamp({t}, 0.0, 1.0), clamp({t}, 0.0, 1.0), clamp({t}, 0.0, 1.0), 1.0)",
                t = t
            )]
        }
        Hsv => {
            flags.hsv = true;
            vec![format!("aor_hsv_to_rgb({})", i("Color"))]
        }
        Desaturate => {
            flags.desaturate = true;
            vec![format!("aor_desaturate({}, {})", i("Color"), i("Amount"))]
        }
        Invert => vec![format!("(vec4<f32>(1.0) - ({}))", i("Color"))],

        SampleTexture2D => {
            let idx = *tex_count;
            *tex_count += 1;
            vec![format!(
                "textureSample(aor_tex_{}, aor_tex_sampler, {})",
                idx,
                i("UV")
            )]
        }
        NormalMapUnpack => vec![format!(
            "normalize(({}).rgb * 2.0 - 1.0)",
            i("Normal Map")
        )],
        TriplanarMapping => {
            let idx = *tex_count;
            *tex_count += 1;
            vec![format!(
                "textureSample(aor_tex_{}, aor_tex_sampler, ({}).xy * 0.5 + 0.5)",
                idx,
                i("Position")
            )]
        }

        Fresnel => {
            flags.fresnel = true;
            vec![format!(
                "aor_fresnel({}, {}, {}, {})",
                i("Normal"),
                i("View Dir"),
                fmt_f(pf("power", 2.0)),
                fmt_f(pf("bias", 0.0))
            )]
        }
        ParallaxOcclusion => {
            flags.parallax = true;
            vec![format!(
                "aor_parallax_occlusion({}, {}, 0.05)",
                i("UV"),
                i("Height")
            )]
        }
        DistanceToEdge => {
            flags.distance_edge = true;
            vec![format!("aor_distance_to_edge({}, 40.0)", i("UV"))]
        }
        WaveDisplacement => {
            flags.wave = true;
            vec![format!(
                "aor_wave_displacement({}, {}, {}, {}, {})",
                i("Position"),
                i("Time"),
                fmt_f(pf("amplitude", 0.05)),
                fmt_f(pf("frequency", 8.0)),
                fmt_f(pf("speed", 1.5))
            )]
        }

        MaterialOutput => vec![],
    }
}

/// Génère les affectations finales du nœud PBR Material Output
fn emit_output_assignments(inputs: &HashMap<String, String>) -> String {
    let g = |n: &str, def: &str| inputs.get(n).cloned().unwrap_or_else(|| def.to_string());
    let albedo = g("Albedo", "vec4<f32>(1.0)");
    let normal = g("Normal", "vec3<f32>(0.0, 0.0, 1.0)");
    let metallic = g("Metallic", "0.0");
    let roughness = g("Roughness", "0.5");
    let emissive = g("Emissive", "vec4<f32>(0.0)");
    let ao = g("Ambient Occlusion", "1.0");
    let alpha = g("Alpha", "1.0");

    let mut s = String::new();
    s.push_str("    var out: FragmentOutput;\n");
    s.push_str(&format!("    out.albedo = {};\n", albedo));
    s.push_str(&format!("    out.normal = {};\n", normal));
    s.push_str(&format!(
        "    out.metallic_roughness = vec4<f32>({}, {}, {}, {});\n",
        metallic, roughness, ao, alpha
    ));
    s.push_str(&format!("    out.emissive = {};\n", emissive));
    s
}

/// Compile un graphe nodal en WGSL en effectuant un tri topologique depuis la sortie
pub fn compile(graph: &MaterialNodeGraph) -> Result<CompiledShader, GraphError> {
    let order = graph.topological_order()?;
    let mut flags = HelperFlags::default();
    let mut var_of_pin: HashMap<Uuid, (String, PinDataType)> = HashMap::new();
    let mut body = String::new();
    let mut tex_count = 0usize;
    let mut out_assign = String::new();

    for (idx, node_id) in order.iter().enumerate() {
        let node = graph
            .node(*node_id)
            .ok_or(GraphError::NodeNotFound(*node_id))?;

        // Résolution des entrées (liaison ou valeur par défaut), avec coercition de type
        let mut inputs: HashMap<String, String> = HashMap::new();
        for pin in &node.inputs {
            let expr = if let Some(link) = graph.link_into(pin.id) {
                match var_of_pin.get(&link.from_pin) {
                    Some((src_var, src_ty)) => coerce(src_var, *src_ty, pin.data_type),
                    None => input_default(node.kind, &node.params, &pin.name),
                }
            } else {
                input_default(node.kind, &node.params, &pin.name)
            };
            inputs.insert(pin.name.clone(), expr);
        }

        if node.kind == NodeKind::MaterialOutput {
            out_assign = emit_output_assignments(&inputs);
            continue;
        }

        let exprs = emit_node(node.kind, &inputs, &node.params, &mut flags, &mut tex_count);
        for (i, out_pin) in node.outputs.iter().enumerate() {
            let expr = exprs.get(i).cloned().unwrap_or_else(|| match out_pin.data_type {
                PinDataType::Vec4 => "vec4<f32>(0.0)".to_string(),
                PinDataType::Vec3 => "vec3<f32>(0.0)".to_string(),
                PinDataType::Vec2 => "vec2<f32>(0.0)".to_string(),
                _ => "0.0".to_string(),
            });
            let var = format!("v{}_{}", idx, sanitize(&out_pin.name));
            body.push_str(&format!("    let {} = {};\n", var, expr));
            var_of_pin.insert(out_pin.id, (var, out_pin.data_type));
        }
    }

    let mut wgsl = String::new();
    wgsl.push_str("// AOR Material Graph — auto-generated WGSL\n");
    wgsl.push_str(&header());
    wgsl.push_str(&build_helpers(&flags));
    wgsl.push_str(&texture_decls(tex_count));
    wgsl.push_str("\n@fragment\nfn fs_pbr_material(in: VertexOutput) -> FragmentOutput {\n");
    wgsl.push_str(&body);
    wgsl.push_str(&out_assign);
    wgsl.push_str("    return out;\n}\n");

    Ok(CompiledShader {
        wgsl,
        helpers: flags.active(),
        texture_bindings: tex_count,
        node_count: order.len(),
    })
}

/// Compile puis valide immédiatement le WGSL généré
pub fn compile_and_validate(graph: &MaterialNodeGraph) -> Result<CompiledShader, String> {
    let compiled = compile(graph).map_err(|e| e.to_string())?;
    compiled.validate()?;
    Ok(compiled)
}

/// Valide un shader WGSL via Naga (front-end + validateur)
pub fn validate_wgsl(source: &str) -> Result<(), String> {
    let module = naga::front::wgsl::parse_str(source)
        .map_err(|e| e.emit_to_string(source))?;

    let mut validator = naga::valid::Validator::new(
        naga::valid::ValidationFlags::all(),
        naga::valid::Capabilities::all(),
    );
    validator
        .validate(&module)
        .map_err(|e| format!("{:?}", e))?;
    Ok(())
}

// =========================================================================
// Génération d'un shader COMPLET compatible avec le pipeline du moteur
// (mêmes bind groups, même layout de vertex, entrée `fs_main`).
// =========================================================================

/// Shader prêt à être injecté dans `create_shader_module` / `create_render_pipeline`
#[derive(Debug, Clone)]
pub struct EngineShader {
    pub wgsl: String,
    pub texture_bindings: usize,
    pub node_count: usize,
}

const ENGINE_HEADER: &str = r#"
struct CameraUniform {
    view_proj: mat4x4<f32>,
    view: mat4x4<f32>,
    proj: mat4x4<f32>,
    camera_pos: vec4<f32>,
};

struct PointLightGpu {
    position: vec4<f32>,
    color: vec4<f32>,
};

struct LightUniform {
    position: vec4<f32>,
    color: vec4<f32>,
    ambient: vec4<f32>,
    fog_color: vec4<f32>,
    light_space_matrix: mat4x4<f32>,
    point_light_count: u32,
    viewport_width: f32,
    viewport_height: f32,
    render_flags: u32,
    point_lights: array<PointLightGpu, 16>,
};

struct ModelUniform {
    model: mat4x4<f32>,
    color: vec4<f32>,
    selected: u32,
    _pad0: u32,
    _pad1: u32,
    _pad2: u32,
};

struct MaterialUniform {
    roughness: f32,
    metallic: f32,
    uv_tiling: vec2<f32>,
    uv_offset: vec2<f32>,
    ior: f32,
    transmission: f32,
    emission_color: vec4<f32>,
    use_normal_map: u32,
    _pad1: u32,
    _pad2: u32,
    _pad3: u32,
};

@group(0) @binding(0) var<uniform> camera: CameraUniform;
@group(0) @binding(1) var<uniform> light: LightUniform;
@group(1) @binding(0) var<uniform> model: ModelUniform;
@group(1) @binding(1) var<uniform> material: MaterialUniform;
"#;

const ENGINE_TEXTURE_BINDINGS: &str = r#"
@group(1) @binding(2) var t_diffuse: texture_2d<f32>;
@group(1) @binding(4) var s_sampler: sampler;
"#;

const ENGINE_VERTEX: &str = r#"
struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) tangent: vec4<f32>,
    @location(3) uv: vec2<f32>,
    @location(4) color: vec4<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) world_pos: vec3<f32>,
    @location(1) world_normal: vec3<f32>,
    @location(2) world_tangent: vec4<f32>,
    @location(3) uv: vec2<f32>,
    @location(4) color: vec4<f32>,
};

@vertex
fn vs_main(in: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    let world = model.model * vec4<f32>(in.position, 1.0);
    out.world_pos = world.xyz;
    let nrm = mat3x3<f32>(model.model[0].xyz, model.model[1].xyz, model.model[2].xyz);
    out.world_normal = normalize(nrm * in.normal);
    out.world_tangent = vec4<f32>(normalize(nrm * in.tangent.xyz), in.tangent.w);
    out.uv = in.uv * material.uv_tiling + material.uv_offset;
    if (model.color.a > 0.0) {
        out.color = model.color;
    } else {
        out.color = in.color;
    }
    out.clip_position = camera.view_proj * world;
    return out;
}
"#;

const ENGINE_LIGHTING: &str = r#"
    let N = normalize(in.world_normal);
    let L = normalize(light.position.xyz);
    let V = normalize(camera.camera_pos.xyz - in.world_pos);
    let H = normalize(L + V);
    let ndl = max(dot(N, L), 0.0);
    let spec_pow = mix(4.0, 128.0, clamp(1.0 - roughness_v, 0.0, 1.0));
    let spec = pow(max(dot(N, H), 0.0), spec_pow) * metallic_v;
    let lit = albedo_v.rgb * (light.ambient.rgb + light.color.rgb * ndl)
            + vec3<f32>(spec)
            + emissive_v.rgb;
    return vec4<f32>(lit, albedo_v.a);
}
"#;

/// Déclarations de variables locales pour la sortie PBR (mode pipeline moteur)
fn emit_output_vars(inputs: &HashMap<String, String>) -> String {
    let g = |n: &str, def: &str| inputs.get(n).cloned().unwrap_or_else(|| def.to_string());
    let albedo = g("Albedo", "vec4<f32>(1.0)");
    let normal = g("Normal", "vec3<f32>(0.0, 0.0, 1.0)");
    let metallic = g("Metallic", "0.0");
    let roughness = g("Roughness", "0.5");
    let emissive = g("Emissive", "vec4<f32>(0.0)");

    format!(
        "    let albedo_v = {};\n    let normal_v = {};\n    let metallic_v = {};\n    let roughness_v = {};\n    let emissive_v = {};\n",
        albedo, normal, metallic, roughness, emissive
    )
}

/// Compile le graphe en un shader WGSL complet, directement injectable dans WGPU.
pub fn compile_engine_shader(graph: &MaterialNodeGraph) -> Result<EngineShader, GraphError> {
    let order = graph.topological_order()?;
    let mut flags = HelperFlags::default();
    let mut var_of_pin: HashMap<Uuid, (String, PinDataType)> = HashMap::new();
    let mut body = String::new();
    let mut tex_count = 0usize;
    let mut out_vars = String::new();

    for (idx, node_id) in order.iter().enumerate() {
        let node = graph
            .node(*node_id)
            .ok_or(GraphError::NodeNotFound(*node_id))?;

        let mut inputs: HashMap<String, String> = HashMap::new();
        for pin in &node.inputs {
            let expr = if let Some(link) = graph.link_into(pin.id) {
                match var_of_pin.get(&link.from_pin) {
                    Some((src_var, src_ty)) => coerce(src_var, *src_ty, pin.data_type),
                    None => input_default(node.kind, &node.params, &pin.name),
                }
            } else {
                input_default(node.kind, &node.params, &pin.name)
            };
            inputs.insert(pin.name.clone(), expr);
        }

        if node.kind == NodeKind::MaterialOutput {
            out_vars = emit_output_vars(&inputs);
            continue;
        }

        let exprs = emit_node(node.kind, &inputs, &node.params, &mut flags, &mut tex_count);
        for (i, out_pin) in node.outputs.iter().enumerate() {
            let expr = exprs.get(i).cloned().unwrap_or_else(|| match out_pin.data_type {
                PinDataType::Vec4 => "vec4<f32>(0.0)".to_string(),
                PinDataType::Vec3 => "vec3<f32>(0.0)".to_string(),
                PinDataType::Vec2 => "vec2<f32>(0.0)".to_string(),
                _ => "0.0".to_string(),
            });
            let var = format!("v{}_{}", idx, sanitize(&out_pin.name));
            body.push_str(&format!("    let {} = {};\n", var, expr));
            var_of_pin.insert(out_pin.id, (var, out_pin.data_type));
        }
    }

    // Adapte le corps généré aux noms du moteur (uniform `camera`, textures `t_diffuse`)
    body = body
        .replace("scene.time", "camera.camera_pos.w")
        .replace("scene.camera_pos", "camera.camera_pos.xyz");
    if tex_count > 0 {
        body = body.replace("aor_tex_sampler", "s_sampler");
        for i in 0..tex_count {
            body = body.replace(&format!("aor_tex_{}", i), "t_diffuse");
        }
    }

    let mut wgsl = String::new();
    wgsl.push_str("// AOR Material Graph — engine-compatible WGSL\n");
    wgsl.push_str(ENGINE_HEADER);
    if tex_count > 0 {
        wgsl.push_str(ENGINE_TEXTURE_BINDINGS);
    }
    wgsl.push_str(&build_helpers(&flags));
    wgsl.push_str(ENGINE_VERTEX);
    wgsl.push_str("\n@fragment\nfn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {\n");
    wgsl.push_str(&body);
    wgsl.push_str(&out_vars);
    wgsl.push_str(ENGINE_LIGHTING);

    Ok(EngineShader {
        wgsl,
        texture_bindings: tex_count,
        node_count: order.len(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::studio::nodes::graph::MaterialNodeGraph;

    #[test]
    fn test_minimal_graph_compiles_and_validates() {
        let g = MaterialNodeGraph::new("Minimal");
        let compiled = compile(&g).expect("compilation");
        assert!(compiled.wgsl.contains("fs_pbr_material"));
        compiled.validate().expect("le WGSL minimal doit être valide");
    }

    #[test]
    fn test_full_pbr_graph_validates() {
        let mut g = MaterialNodeGraph::new("Neon PBR");
        let uv = g.add_node(NodeKind::Uv, [0.0, 0.0]);
        let noise = g.add_node(NodeKind::PerlinNoise, [200.0, 0.0]);
        let rgb = g.add_node(NodeKind::Rgb, [200.0, 150.0]);
        let mix = g.add_node(NodeKind::Mix, [400.0, 80.0]);
        let fres = g.add_node(NodeKind::Fresnel, [200.0, 300.0]);
        let out = g.output_node_id;

        g.connect(uv, "UV", noise, "UV").unwrap();
        g.connect(noise, "Out", mix, "T").unwrap();
        g.connect(rgb, "Color", mix, "A").unwrap();
        g.connect(mix, "Out", out, "Albedo").unwrap();
        g.connect(fres, "Out", out, "Emissive").unwrap();

        let compiled = compile(&g).expect("compilation");
        assert!(compiled.helpers.contains(&"perlin"));
        assert!(compiled.helpers.contains(&"fresnel"));
        compiled.validate().expect("le WGSL PBR doit être valide");
    }

    #[test]
    fn test_procedural_helpers_graph_validates() {
        let mut g = MaterialNodeGraph::new("Procedural");
        let uv = g.add_node(NodeKind::Uv, [0.0, 0.0]);
        let vor = g.add_node(NodeKind::Voronoi, [200.0, 0.0]);
        let chk = g.add_node(NodeKind::Checkerboard, [200.0, 100.0]);
        let add = g.add_node(NodeKind::Add, [400.0, 50.0]);
        let out = g.output_node_id;

        g.connect(uv, "UV", vor, "UV").unwrap();
        g.connect(uv, "UV", chk, "UV").unwrap();
        g.connect(vor, "Out", add, "A").unwrap();
        g.connect(chk, "Out", add, "B").unwrap();
        g.connect(add, "Out", out, "Albedo").unwrap();

        let compiled = compile(&g).unwrap();
        assert!(compiled.helpers.contains(&"voronoi"));
        assert!(compiled.helpers.contains(&"checker"));
        compiled.validate().unwrap();
    }

    #[test]
    fn test_cycle_is_reported() {
        let mut g = MaterialNodeGraph::new("Cycle");
        let a = g.add_node(NodeKind::Add, [0.0, 0.0]);
        let b = g.add_node(NodeKind::Add, [100.0, 0.0]);
        g.connect(a, "Out", b, "A").unwrap();
        // Force un cycle en inspectant directement les liaisons (connect() le refuse)
        g.links.push(crate::studio::nodes::graph::NodeLink {
            id: uuid::Uuid::new_v4(),
            from_node: b,
            from_pin: g.node(b).unwrap().outputs[0].id,
            to_node: a,
            to_pin: g.node(a).unwrap().inputs[0].id,
        });
        assert!(g.has_cycle());
        let err = compile(&g).unwrap_err();
        assert!(matches!(err, GraphError::Cycle(_)));
    }

    #[test]
    fn test_texture_node_generates_bindings() {
        let mut g = MaterialNodeGraph::new("Textured");
        let tex = g.add_node(NodeKind::SampleTexture2D, [0.0, 0.0]);
        let out = g.output_node_id;
        g.connect(tex, "Color", out, "Albedo").unwrap();
        let compiled = compile(&g).unwrap();
        assert_eq!(compiled.texture_bindings, 1);
        assert!(compiled.wgsl.contains("aor_tex_0"));
        compiled.validate().unwrap();
    }

    #[test]
    fn test_engine_shader_is_valid_and_uses_engine_bindings() {
        let mut g = MaterialNodeGraph::new("Engine Neon");
        let uv = g.add_node(NodeKind::Uv, [0.0, 0.0]);
        let noise = g.add_node(NodeKind::PerlinNoise, [220.0, 0.0]);
        let mix = g.add_node(NodeKind::Mix, [440.0, 80.0]);
        let tex = g.add_node(NodeKind::SampleTexture2D, [220.0, 220.0]);
        let fres = g.add_node(NodeKind::Fresnel, [220.0, 340.0]);
        let out = g.output_node_id;

        g.connect(uv, "UV", noise, "UV").unwrap();
        g.connect(uv, "UV", tex, "UV").unwrap();
        g.connect(noise, "Out", mix, "T").unwrap();
        g.connect(tex, "Color", mix, "A").unwrap();
        g.connect(mix, "Out", out, "Albedo").unwrap();
        g.connect(fres, "Out", out, "Emissive").unwrap();

        let shader = compile_engine_shader(&g).expect("engine shader");
        assert_eq!(shader.texture_bindings, 1);
        // Entrées moteur
        assert!(shader.wgsl.contains("fn vs_main"));
        assert!(shader.wgsl.contains("fn fs_main"));
        assert!(shader.wgsl.contains("@group(0) @binding(0) var<uniform> camera"));
        assert!(shader.wgsl.contains("t_diffuse"));
        assert!(!shader.wgsl.contains("scene.time"));
        // Le shader complet doit être syntaxiquement et sémantiquement valide
        validate_wgsl(&shader.wgsl).expect("le shader moteur doit être valide");
    }

    #[test]
    fn test_engine_shader_minimal_valid() {
        let g = MaterialNodeGraph::new("MinimalEngine");
        let shader = compile_engine_shader(&g).unwrap();
        assert_eq!(shader.texture_bindings, 0);
        validate_wgsl(&shader.wgsl).unwrap();
    }

    #[test]
    fn test_node_params_drive_codegen() {
        let mut g = MaterialNodeGraph::new("Params");
        let rgb = g.add_node(NodeKind::Rgb, [0.0, 0.0]);
        let out = g.output_node_id;
        g.connect(rgb, "Color", out, "Albedo").unwrap();

        let before = compile_engine_shader(&g).unwrap().wgsl;
        assert!(before.contains("1.0"), "couleur par défaut blanche attendue");

        g.node_mut(rgb).unwrap().set_param_f32("r", 0.25);
        let after = compile_engine_shader(&g).unwrap().wgsl;
        assert_ne!(before, after, "le paramètre doit modifier le shader");
        assert!(after.contains("0.25"));
        validate_wgsl(&after).unwrap();
    }

    #[test]
    fn test_fresnel_params_defaults_present() {
        let node = crate::studio::nodes::graph::GraphNode::new(
            NodeKind::Fresnel,
            [0.0, 0.0],
        );
        assert_eq!(node.param_f32("power", -1.0), 2.0);
        assert_eq!(node.param_f32("bias", -1.0), 0.0);
    }
}
