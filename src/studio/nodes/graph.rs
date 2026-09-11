// [WFGY] Zone: SAFE | λ: 0.25 | Fallbacks: 0/None | Action: Material node graph data model with typed pins, links and cycle validation
#![allow(dead_code)]
use super::kinds::{NodeKind, PinDataType, PinSpec};
use crate::studio::vfs::AssetId;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// Valeur de paramètre éditable d'un nœud (constantes, couleurs, échelles...)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", content = "value", rename_all = "snake_case")]
pub enum ParamValue {
    Float(f32),
    Vec2([f32; 2]),
    Vec3([f32; 3]),
    Vec4([f32; 4]),
}

impl ParamValue {
    pub fn as_f32(&self) -> f32 {
        match self {
            ParamValue::Float(v) => *v,
            ParamValue::Vec2(v) => v[0],
            ParamValue::Vec3(v) => v[0],
            ParamValue::Vec4(v) => v[0],
        }
    }

    pub fn as_vec4(&self) -> [f32; 4] {
        match self {
            ParamValue::Float(v) => [*v, *v, *v, 1.0],
            ParamValue::Vec2(v) => [v[0], v[1], 0.0, 1.0],
            ParamValue::Vec3(v) => [v[0], v[1], v[2], 1.0],
            ParamValue::Vec4(v) => *v,
        }
    }
}

/// Pin concret instancié sur un nœud
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NodePin {
    pub id: Uuid,
    pub name: String,
    pub data_type: PinDataType,
}

impl NodePin {
    pub fn from_spec(spec: &PinSpec) -> Self {
        NodePin {
            id: Uuid::new_v4(),
            name: spec.name.to_string(),
            data_type: spec.data_type,
        }
    }
}

/// Nœud d'un graphe de matériau
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GraphNode {
    pub id: Uuid,
    pub kind: NodeKind,
    pub position: [f32; 2],
    pub inputs: Vec<NodePin>,
    pub outputs: Vec<NodePin>,
    #[serde(default)]
    pub params: HashMap<String, ParamValue>,
}

impl GraphNode {
    pub fn new(kind: NodeKind, position: [f32; 2]) -> Self {
        let params = kind
            .default_params()
            .into_iter()
            .map(|(k, v)| (k.to_string(), v))
            .collect();
        GraphNode {
            id: Uuid::new_v4(),
            kind,
            position,
            inputs: kind.inputs().iter().map(NodePin::from_spec).collect(),
            outputs: kind.outputs().iter().map(NodePin::from_spec).collect(),
            params,
        }
    }

    pub fn param(&self, key: &str) -> Option<&ParamValue> {
        self.params.get(key)
    }

    pub fn param_f32(&self, key: &str, default: f32) -> f32 {
        self.params.get(key).map(|p| p.as_f32()).unwrap_or(default)
    }

    pub fn param_vec4(&self, key: &str, default: [f32; 4]) -> [f32; 4] {
        self.params.get(key).map(|p| p.as_vec4()).unwrap_or(default)
    }

    pub fn set_param(&mut self, key: &str, value: ParamValue) {
        self.params.insert(key.to_string(), value);
    }

    pub fn set_param_f32(&mut self, key: &str, value: f32) {
        self.params.insert(key.to_string(), ParamValue::Float(value));
    }

    pub fn input_pin(&self, name: &str) -> Option<&NodePin> {
        self.inputs.iter().find(|p| p.name == name)
    }

    pub fn output_pin(&self, name: &str) -> Option<&NodePin> {
        self.outputs.iter().find(|p| p.name == name)
    }

    pub fn input_pin_by_id(&self, id: Uuid) -> Option<&NodePin> {
        self.inputs.iter().find(|p| p.id == id)
    }

    pub fn output_pin_by_id(&self, id: Uuid) -> Option<&NodePin> {
        self.outputs.iter().find(|p| p.id == id)
    }
}

/// Liaison orientée entre un pin de sortie et un pin d'entrée
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NodeLink {
    pub id: Uuid,
    pub from_node: Uuid,
    pub from_pin: Uuid,
    pub to_node: Uuid,
    pub to_pin: Uuid,
}

/// Erreurs manipulables du graphe
#[derive(Debug, Clone, PartialEq)]
pub enum GraphError {
    NodeNotFound(Uuid),
    PinNotFound(Uuid, String),
    TypeMismatch(PinDataType, PinDataType),
    SelfLink,
    WouldCreateCycle,
    MissingOutputNode,
    Cycle(Vec<Uuid>),
}

impl std::fmt::Display for GraphError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GraphError::NodeNotFound(id) => write!(f, "nœud introuvable: {}", id),
            GraphError::PinNotFound(id, name) => write!(f, "pin '{}' introuvable sur {}", name, id),
            GraphError::TypeMismatch(a, b) => write!(f, "types incompatibles: {:?} -> {:?}", a, b),
            GraphError::SelfLink => write!(f, "un nœud ne peut pas se connecter à lui-même"),
            GraphError::WouldCreateCycle => write!(f, "la liaison créerait un cycle"),
            GraphError::MissingOutputNode => write!(f, "nœud de sortie manquant"),
            GraphError::Cycle(path) => write!(f, "cycle détecté: {:?}", path),
        }
    }
}

impl std::error::Error for GraphError {}

/// Vérifie la compatibilité de type entre une sortie et une entrée.
/// Les promotions scalaires -> vecteurs sont autorisées, pas les réductions.
pub fn can_connect(from: PinDataType, to: PinDataType) -> bool {
    if from == to {
        return true;
    }
    if from.is_texture_like() || to.is_texture_like() {
        return false;
    }
    let from_c = from.component_count();
    let to_c = to.component_count();
    from_c >= 1 && to_c >= from_c
}

/// Graphe nodal complet d'un matériau PBR
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MaterialNodeGraph {
    pub id: AssetId,
    pub name: String,
    pub nodes: HashMap<Uuid, GraphNode>,
    pub links: Vec<NodeLink>,
    pub output_node_id: Uuid,
}

impl MaterialNodeGraph {
    /// Crée un graphe vide muni d'un unique nœud de sortie PBR Material Output
    pub fn new(name: impl Into<String>) -> Self {
        let mut nodes = HashMap::new();
        let output = GraphNode::new(NodeKind::MaterialOutput, [480.0, 120.0]);
        let output_id = output.id;
        nodes.insert(output_id, output);
        MaterialNodeGraph {
            id: AssetId::new(),
            name: name.into(),
            nodes,
            links: Vec::new(),
            output_node_id: output_id,
        }
    }

    /// Ajoute un nœud et retourne son identifiant
    pub fn add_node(&mut self, kind: NodeKind, position: [f32; 2]) -> Uuid {
        let node = GraphNode::new(kind, position);
        let id = node.id;
        self.nodes.insert(id, node);
        id
    }

    pub fn remove_node(&mut self, id: Uuid) -> bool {
        if id == self.output_node_id {
            return false;
        }
        let removed = self.nodes.remove(&id).is_some();
        self.links.retain(|l| l.from_node != id && l.to_node != id);
        removed
    }

    pub fn node(&self, id: Uuid) -> Option<&GraphNode> {
        self.nodes.get(&id)
    }

    pub fn node_mut(&mut self, id: Uuid) -> Option<&mut GraphNode> {
        self.nodes.get_mut(&id)
    }

    pub fn output_node(&self) -> Option<&GraphNode> {
        self.nodes.get(&self.output_node_id)
    }

    /// Connecte deux nœuds via leurs noms de pins
    pub fn connect(
        &mut self,
        from_node: Uuid,
        from_pin_name: &str,
        to_node: Uuid,
        to_pin_name: &str,
    ) -> Result<Uuid, GraphError> {
        if from_node == to_node {
            return Err(GraphError::SelfLink);
        }

        let from_pin = self
            .nodes
            .get(&from_node)
            .ok_or(GraphError::NodeNotFound(from_node))?
            .output_pin(from_pin_name)
            .ok_or(GraphError::PinNotFound(from_node, from_pin_name.to_string()))?
            .clone();

        let to_pin = self
            .nodes
            .get(&to_node)
            .ok_or(GraphError::NodeNotFound(to_node))?
            .input_pin(to_pin_name)
            .ok_or(GraphError::PinNotFound(to_node, to_pin_name.to_string()))?
            .clone();

        if !can_connect(from_pin.data_type, to_pin.data_type) {
            return Err(GraphError::TypeMismatch(from_pin.data_type, to_pin.data_type));
        }

        // Un pin d'entrée n'accepte qu'une seule liaison : remplace l'ancienne
        self.links.retain(|l| l.to_pin != to_pin.id);

        let link = NodeLink {
            id: Uuid::new_v4(),
            from_node,
            from_pin: from_pin.id,
            to_node,
            to_pin: to_pin.id,
        };
        let link_id = link.id;
        self.links.push(link);

        if self.detect_cycle().is_some() {
            self.links.retain(|l| l.id != link_id);
            return Err(GraphError::WouldCreateCycle);
        }

        Ok(link_id)
    }

    pub fn disconnect_pin(&mut self, to_pin: Uuid) {
        self.links.retain(|l| l.to_pin != to_pin);
    }

    /// Connecte deux pins identifiés par leurs Uuid (utilisé par l'éditeur nodal)
    pub fn connect_by_pin_ids(
        &mut self,
        from_node: Uuid,
        from_pin: Uuid,
        to_node: Uuid,
        to_pin: Uuid,
    ) -> Result<Uuid, GraphError> {
        let from_name = self
            .nodes
            .get(&from_node)
            .and_then(|n| n.output_pin_by_id(from_pin))
            .map(|p| p.name.clone())
            .ok_or(GraphError::PinNotFound(from_node, from_pin.to_string()))?;
        let to_name = self
            .nodes
            .get(&to_node)
            .and_then(|n| n.input_pin_by_id(to_pin))
            .map(|p| p.name.clone())
            .ok_or(GraphError::PinNotFound(to_node, to_pin.to_string()))?;
        self.connect(from_node, &from_name, to_node, &to_name)
    }

    /// Retourne la liaison alimentant un pin d'entrée
    pub fn link_into(&self, to_pin: Uuid) -> Option<&NodeLink> {
        self.links.iter().find(|l| l.to_pin == to_pin)
    }

    /// Détecte un éventuel cycle via un parcours DFS à trois états.
    /// Retourne le chemin du cycle si présent.
    pub fn detect_cycle(&self) -> Option<Vec<Uuid>> {
        #[derive(Clone, Copy, PartialEq)]
        enum Color {
            White,
            Gray,
            Black,
        }

        let mut color: HashMap<Uuid, Color> =
            self.nodes.keys().map(|k| (*k, Color::White)).collect();
        let mut stack: Vec<Uuid> = Vec::new();

        fn dfs(
            graph: &MaterialNodeGraph,
            node: Uuid,
            color: &mut HashMap<Uuid, Color>,
            stack: &mut Vec<Uuid>,
        ) -> Option<Vec<Uuid>> {
            color.insert(node, Color::Gray);
            stack.push(node);

            // On remonte le flux : pour chaque entrée, le nœud source qui l'alimente
            for link in graph.links.iter().filter(|l| l.to_node == node) {
                match color.get(&link.from_node).copied().unwrap_or(Color::White) {
                    Color::Gray => {
                        let mut cycle = stack.clone();
                        cycle.push(link.from_node);
                        return Some(cycle);
                    }
                    Color::White => {
                        if let Some(c) = dfs(graph, link.from_node, color, stack) {
                            return Some(c);
                        }
                    }
                    Color::Black => {}
                }
            }

            stack.pop();
            color.insert(node, Color::Black);
            None
        }

        for node in self.nodes.keys().copied().collect::<Vec<_>>() {
            if color.get(&node) == Some(&Color::White) {
                if let Some(cycle) = dfs(self, node, &mut color, &mut stack) {
                    return Some(cycle);
                }
            }
        }
        None
    }

    pub fn has_cycle(&self) -> bool {
        self.detect_cycle().is_some()
    }

    /// Tri topologique depuis le nœud de sortie vers les feuilles.
    /// L'ordre retourné garantit que chaque nœud apparaît après ses dépendances.
    pub fn topological_order(&self) -> Result<Vec<Uuid>, GraphError> {
        if !self.nodes.contains_key(&self.output_node_id) {
            return Err(GraphError::MissingOutputNode);
        }
        if let Some(cycle) = self.detect_cycle() {
            return Err(GraphError::Cycle(cycle));
        }

        let mut visited = std::collections::HashSet::new();
        let mut order = Vec::new();

        fn visit(
            graph: &MaterialNodeGraph,
            node: Uuid,
            visited: &mut std::collections::HashSet<Uuid>,
            order: &mut Vec<Uuid>,
        ) {
            if !visited.insert(node) {
                return;
            }
            for link in graph.links.iter().filter(|l| l.to_node == node) {
                visit(graph, link.from_node, visited, order);
            }
            order.push(node);
        }

        visit(self, self.output_node_id, &mut visited, &mut order);
        Ok(order)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_graph_has_output() {
        let g = MaterialNodeGraph::new("Test");
        assert!(g.output_node().is_some());
        assert_eq!(g.nodes.len(), 1);
        assert!(!g.has_cycle());
    }

    #[test]
    fn test_connect_and_reject_cycle() {
        let mut g = MaterialNodeGraph::new("Test");
        let noise = g.add_node(NodeKind::PerlinNoise, [0.0, 0.0]);
        let mix = g.add_node(NodeKind::Mix, [200.0, 0.0]);
        let out = g.output_node_id;

        // noise -> mix.A, mix -> output.Albedo
        g.connect(noise, "Out", mix, "A").unwrap();
        g.connect(mix, "Out", out, "Albedo").unwrap();
        assert!(!g.has_cycle());

        // Tentative de créer un cycle : out n'a pas de pin de sortie, on teste via deux math
        let a = g.add_node(NodeKind::Add, [0.0, 200.0]);
        let b = g.add_node(NodeKind::Add, [200.0, 200.0]);
        g.connect(a, "Out", b, "A").unwrap();
        let cyclic = g.connect(b, "Out", a, "A"); // doit échouer (cycle)
        assert!(matches!(cyclic, Err(GraphError::WouldCreateCycle)));
        assert!(!g.has_cycle(), "la liaison cyclique doit être annulée");
    }

    #[test]
    fn test_type_mismatch_rejected() {
        let mut g = MaterialNodeGraph::new("Test");
        let tex = g.add_node(NodeKind::SampleTexture2D, [0.0, 0.0]);
        let out = g.output_node_id;
        // Une texture ne peut pas alimenter Metallic (float)
        let res = g.connect(tex, "Color", out, "Metallic");
        assert!(matches!(res, Err(GraphError::TypeMismatch(..))));
    }

    #[test]
    fn test_topological_order_places_dependencies_first() {
        let mut g = MaterialNodeGraph::new("Test");
        let noise = g.add_node(NodeKind::PerlinNoise, [0.0, 0.0]);
        let mix = g.add_node(NodeKind::Mix, [200.0, 0.0]);
        let out = g.output_node_id;
        g.connect(noise, "Out", mix, "T").unwrap();
        g.connect(mix, "Out", out, "Albedo").unwrap();

        let order = g.topological_order().unwrap();
        let pos_noise = order.iter().position(|n| *n == noise).unwrap();
        let pos_mix = order.iter().position(|n| *n == mix).unwrap();
        let pos_out = order.iter().position(|n| *n == out).unwrap();
        assert!(pos_noise < pos_mix);
        assert!(pos_mix < pos_out);
    }

    #[test]
    fn test_graph_serde_roundtrip() {
        let mut g = MaterialNodeGraph::new("SerdeGraph");
        let n = g.add_node(NodeKind::Fresnel, [10.0, 20.0]);
        g.connect(n, "Out", g.output_node_id, "Emissive").ok();
        let json = serde_json::to_string(&g).unwrap();
        let back: MaterialNodeGraph = serde_json::from_str(&json).unwrap();
        assert_eq!(g, back);
    }
}
