// [WFGY] Zone: RISK | λ: 0.3 | Fallbacks: 0/None | Action: Interactive node graph canvas (pan/zoom, bezier links, hit testing)
#![allow(dead_code)]
use super::style::leaf;
use crate::studio::nodes::graph::{MaterialNodeGraph, NodePin};
use crate::studio::nodes::kinds::{NodeCategory, NodeKind};
use ui_layout::NodeId;
use ui_widgets::{Painter, WidgetId, WidgetTree};
use uuid::Uuid;

pub const NODE_WIDTH: f32 = 190.0;
pub const NODE_HEADER: f32 = 26.0;
pub const PIN_ROW: f32 = 20.0;

/// Transform 2D pan/zoom du canvas nodal
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CanvasTransform {
    pub pan: [f32; 2],
    pub zoom: f32,
}

impl Default for CanvasTransform {
    fn default() -> Self {
        CanvasTransform {
            pan: [40.0, 40.0],
            zoom: 1.0,
        }
    }
}

impl CanvasTransform {
    pub fn graph_to_screen(&self, p: [f32; 2]) -> [f32; 2] {
        [
            p[0] * self.zoom + self.pan[0],
            p[1] * self.zoom + self.pan[1],
        ]
    }

    pub fn screen_to_graph(&self, s: [f32; 2]) -> [f32; 2] {
        [
            (s[0] - self.pan[0]) / self.zoom,
            (s[1] - self.pan[1]) / self.zoom,
        ]
    }

    pub fn pan_by(&mut self, delta: [f32; 2]) {
        self.pan[0] += delta[0];
        self.pan[1] += delta[1];
    }

    /// Zoom en conservant le point ancré sous le curseur
    pub fn zoom_at(&mut self, anchor_screen: [f32; 2], factor: f32) {
        let before = self.screen_to_graph(anchor_screen);
        self.zoom = (self.zoom * factor).clamp(0.2, 4.0);
        let after = self.graph_to_screen(before);
        self.pan[0] += anchor_screen[0] - after[0];
        self.pan[1] += anchor_screen[1] - after[1];
    }

    /// Cadre le graphe entier dans un viewport de taille donnée
    pub fn fit(&mut self, graph: &MaterialNodeGraph, viewport: [f32; 2]) {
        if graph.nodes.is_empty() {
            return;
        }
        let mut min = [f32::MAX, f32::MAX];
        let mut max = [f32::MIN, f32::MIN];
        for node in graph.nodes.values() {
            let (w, h) = node_size(node.kind, node.inputs.len(), node.outputs.len());
            min[0] = min[0].min(node.position[0]);
            min[1] = min[1].min(node.position[1]);
            max[0] = max[0].max(node.position[0] + w);
            max[1] = max[1].max(node.position[1] + h);
        }
        let graph_w = (max[0] - min[0]).max(1.0);
        let graph_h = (max[1] - min[1]).max(1.0);
        let zoom = ((viewport[0] - 80.0) / graph_w)
            .min((viewport[1] - 80.0) / graph_h)
            .clamp(0.2, 2.0);
        self.zoom = zoom;
        self.pan = [
            -min[0] * zoom + (viewport[0] - graph_w * zoom) * 0.5,
            -min[1] * zoom + (viewport[1] - graph_h * zoom) * 0.5,
        ];
    }
}

/// Taille (largeur, hauteur) d'une carte de nœud
pub fn node_size(kind: NodeKind, inputs: usize, outputs: usize) -> (f32, f32) {
    let rows = inputs.max(outputs) as f32;
    let h = NODE_HEADER + rows * PIN_ROW + 12.0;
    let w = if kind == NodeKind::MaterialOutput {
        NODE_WIDTH + 40.0
    } else {
        NODE_WIDTH
    };
    (w, h)
}

fn category_color(cat: NodeCategory) -> [f32; 4] {
    match cat {
        NodeCategory::Input => [0.20, 0.70, 1.00, 1.0],
        NodeCategory::Math => [0.60, 0.65, 0.75, 1.0],
        NodeCategory::Procedural => [0.75, 0.35, 1.00, 1.0],
        NodeCategory::Color => [1.00, 0.55, 0.20, 1.0],
        NodeCategory::Texture => [0.20, 0.90, 0.65, 1.0],
        NodeCategory::Effect => [1.00, 0.30, 0.55, 1.0],
        NodeCategory::Output => [0.15, 0.95, 1.00, 1.0],
    }
}

fn pin_color(t: crate::studio::nodes::kinds::PinDataType) -> [f32; 4] {
    use crate::studio::nodes::kinds::PinDataType::*;
    match t {
        Float => [0.85, 0.85, 0.55, 1.0],
        Vec2 => [0.45, 0.85, 0.55, 1.0],
        Vec3 => [0.35, 0.65, 1.00, 1.0],
        Vec4 => [0.90, 0.45, 0.85, 1.0],
        Texture2D | Sampler => [0.95, 0.40, 0.35, 1.0],
    }
}

/// Position locale d'un pin (entrée) dans la carte du nœud
pub fn input_pin_local(index: usize) -> [f32; 2] {
    [0.0, NODE_HEADER + 10.0 + index as f32 * PIN_ROW]
}

/// Position locale d'un pin (sortie)
pub fn output_pin_local(kind: NodeKind, inputs: usize, outputs: usize, index: usize) -> [f32; 2] {
    let (w, _) = node_size(kind, inputs, outputs);
    [w, NODE_HEADER + 10.0 + index as f32 * PIN_ROW]
}

/// Calcule la position écran du centre d'un pin de sortie
pub fn output_pin_screen(
    graph: &MaterialNodeGraph,
    tf: &CanvasTransform,
    node_id: Uuid,
    pin: &NodePin,
) -> Option<[f32; 2]> {
    let node = graph.node(node_id)?;
    let idx = node.outputs.iter().position(|p| p.id == pin.id)?;
    let local = output_pin_local(node.kind, node.inputs.len(), node.outputs.len(), idx);
    let world = [node.position[0] + local[0], node.position[1] + local[1]];
    Some(tf.graph_to_screen(world))
}

/// Calcule la position écran du centre d'un pin d'entrée
pub fn input_pin_screen(
    graph: &MaterialNodeGraph,
    tf: &CanvasTransform,
    node_id: Uuid,
    pin: &NodePin,
) -> Option<[f32; 2]> {
    let node = graph.node(node_id)?;
    let idx = node.inputs.iter().position(|p| p.id == pin.id)?;
    let local = input_pin_local(idx);
    let world = [node.position[0] + local[0], node.position[1] + local[1]];
    Some(tf.graph_to_screen(world))
}

/// Retourne l'identifiant du nœud sous le curseur (coordonnées écran locales au canvas)
pub fn hit_node(
    graph: &MaterialNodeGraph,
    tf: &CanvasTransform,
    screen: [f32; 2],
) -> Option<Uuid> {
    let g = tf.screen_to_graph(screen);
    // Priorité aux nœuds dessinés en dernier (les plus récents)
    let mut candidates: Vec<_> = graph.nodes.iter().collect();
    candidates.sort_by(|a, b| b.1.position[1].partial_cmp(&a.1.position[1]).unwrap_or(std::cmp::Ordering::Equal));
    for (id, node) in candidates {
        let (w, h) = node_size(node.kind, node.inputs.len(), node.outputs.len());
        if g[0] >= node.position[0]
            && g[0] <= node.position[0] + w
            && g[1] >= node.position[1]
            && g[1] <= node.position[1] + h
        {
            return Some(*id);
        }
    }
    None
}

/// Résultat d'un test de survol d'un pin
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PinHit {
    pub node: Uuid,
    pub pin: Uuid,
    pub is_output: bool,
}

/// Retourne le pin sous le curseur (coordonnées écran locales au canvas) dans un rayon donné
pub fn hit_pin(
    graph: &MaterialNodeGraph,
    tf: &CanvasTransform,
    screen: [f32; 2],
    radius: f32,
) -> Option<PinHit> {
    let r2 = radius * radius;
    for (id, node) in &graph.nodes {
        for (i, pin) in node.inputs.iter().enumerate() {
            let local = input_pin_local(i);
            let p = tf.graph_to_screen([node.position[0] + local[0], node.position[1] + local[1]]);
            let d2 = (p[0] - screen[0]).powi(2) + (p[1] - screen[1]).powi(2);
            if d2 <= r2 {
                return Some(PinHit {
                    node: *id,
                    pin: pin.id,
                    is_output: false,
                });
            }
        }
        for (i, pin) in node.outputs.iter().enumerate() {
            let local = output_pin_local(node.kind, node.inputs.len(), node.outputs.len(), i);
            let p = tf.graph_to_screen([node.position[0] + local[0], node.position[1] + local[1]]);
            let d2 = (p[0] - screen[0]).powi(2) + (p[1] - screen[1]).powi(2);
            if d2 <= r2 {
                return Some(PinHit {
                    node: *id,
                    pin: pin.id,
                    is_output: true,
                });
            }
        }
    }
    None
}

/// Construit la liste des commandes de peinture du graphe
pub fn build_painter(
    graph: &MaterialNodeGraph,
    tf: &CanvasTransform,
    selected: Option<Uuid>,
    pending_link: Option<(Uuid, Uuid)>,
    cursor_pos: [f32; 2],
    viewport: [f32; 2],
) -> Vec<ui_widgets::PaintCommand> {
    let mut painter = Painter::new();

    // 0. Fond délimité du canvas nodal (Cyber-Glass foncé avec bordure fine)
    painter.rect(
        [0.0, 0.0, viewport[0], viewport[1]],
        8.0,
        Some([0.04, 0.05, 0.08, 0.98]),
        Some(([0.25, 0.40, 0.65, 0.35], 1.0)),
    );

    // 1. Grille de fond subtile Cyber-Glass
    let step = 32.0 * tf.zoom;
    if step > 6.0 {
        let origin = tf.graph_to_screen([0.0, 0.0]);
        let grid_color = [1.0, 1.0, 1.0, 0.035];
        let mut x = origin[0] % step;
        while x < viewport[0] {
            painter.line([x, 0.0], [x, viewport[1]], 1.0, grid_color);
            x += step;
        }
        let mut y = origin[1] % step;
        while y < viewport[1] {
            painter.line([0.0, y], [viewport[0], y], 1.0, grid_color);
            y += step;
        }
    }

    // 2. Liaisons (courbes Bézier élégantes et lumineuses, sans ligne parasite)
    for link in &graph.links {
        let from = graph
            .node(link.from_node)
            .and_then(|n| n.output_pin_by_id(link.from_pin))
            .and_then(|p| output_pin_screen(graph, tf, link.from_node, p));
        let to = graph
            .node(link.to_node)
            .and_then(|n| n.input_pin_by_id(link.to_pin))
            .and_then(|p| input_pin_screen(graph, tf, link.to_node, p));
        if let (Some(from), Some(to)) = (from, to) {
            let dx = (to[0] - from[0]).abs().max(40.0) * 0.45;
            let c1 = [from[0] + dx, from[1]];
            let c2 = [to[0] - dx, to[1]];
            let ty = graph
                .node(link.from_node)
                .and_then(|n| n.output_pin_by_id(link.from_pin))
                .map(|p| pin_color(p.data_type))
                .unwrap_or([0.35, 0.75, 1.0, 1.0]);

            // Halo discret (glow) + ligne centrale fine et nette
            painter.bezier(from, c1, c2, to, 3.5, [ty[0], ty[1], ty[2], 0.25]);
            painter.bezier(from, c1, c2, to, 1.5, ty);
        }
    }

    // 2b. Fil en cours de connexion (pending_link)
    if let Some((from_node, from_pin)) = pending_link {
        let from = graph
            .node(from_node)
            .and_then(|n| n.output_pin_by_id(from_pin))
            .and_then(|p| output_pin_screen(graph, tf, from_node, p));
        if let Some(from) = from {
            let to = cursor_pos;
            let dx = (to[0] - from[0]).abs().max(40.0) * 0.45;
            let c1 = [from[0] + dx, from[1]];
            let c2 = [to[0] - dx, to[1]];
            let link_color = [0.35, 0.85, 1.0, 0.9];
            painter.bezier(from, c1, c2, to, 3.5, [0.35, 0.85, 1.0, 0.2]);
            painter.bezier(from, c1, c2, to, 1.5, link_color);
        }
    }

    // 3. Cartes de nœuds
    for (id, node) in &graph.nodes {
        let (w, h) = node_size(node.kind, node.inputs.len(), node.outputs.len());
        let screen = tf.graph_to_screen(node.position);
        let bounds = [screen[0], screen[1], w * tf.zoom, h * tf.zoom];
        let is_sel = selected == Some(*id);
        let header_color = category_color(node.kind.category());
        let body = [0.07, 0.09, 0.14, 0.95];
        let stroke = if is_sel {
            ([1.0, 1.0, 1.0, 0.95], 2.0)
        } else {
            ([header_color[0], header_color[1], header_color[2], 0.45], 1.0)
        };

        painter.rect(bounds, 6.0 * tf.zoom, Some(body), Some(stroke));
        painter.rect(
            [screen[0], screen[1], w * tf.zoom, NODE_HEADER * tf.zoom],
            6.0 * tf.zoom,
            Some([header_color[0] * 0.8, header_color[1] * 0.8, header_color[2] * 0.8, 0.90]),
            None,
        );
        let title_font_size = 12.0 * tf.zoom;
        let title_y = screen[1] + ((NODE_HEADER * tf.zoom - title_font_size) * 0.5);
        painter.text(
            [screen[0] + 8.0 * tf.zoom, title_y],
            node.kind.label(),
            title_font_size,
            [0.98, 0.98, 1.0, 1.0],
        );

        for (i, pin) in node.inputs.iter().enumerate() {
            let local = input_pin_local(i);
            let p = tf.graph_to_screen([node.position[0] + local[0], node.position[1] + local[1]]);
            painter.circle(p, 4.0 * tf.zoom, Some(pin_color(pin.data_type)), None);
            painter.text(
                [p[0] + 8.0 * tf.zoom, p[1] + 4.0 * tf.zoom],
                &pin.name,
                10.5 * tf.zoom,
                [0.75, 0.80, 0.88, 1.0],
            );
        }
        for (i, pin) in node.outputs.iter().enumerate() {
            let local = output_pin_local(node.kind, node.inputs.len(), node.outputs.len(), i);
            let p = tf.graph_to_screen([node.position[0] + local[0], node.position[1] + local[1]]);
            painter.circle(p, 4.0 * tf.zoom, Some(pin_color(pin.data_type)), None);
            painter.text(
                [p[0] - 8.0 * tf.zoom - (pin.name.len() as f32 * 6.0 * tf.zoom), p[1] + 4.0 * tf.zoom],
                &pin.name,
                10.5 * tf.zoom,
                [0.75, 0.80, 0.88, 1.0],
            );
        }
    }

    painter.finish()
}

/// Construit le widget canvas interactif du graphe nodal
pub fn build(
    tree: &mut WidgetTree,
    graph: &MaterialNodeGraph,
    tf: &CanvasTransform,
    selected: Option<Uuid>,
    pending_link: Option<(Uuid, Uuid)>,
    cursor_pos: [f32; 2],
    width: f32,
    height: f32,
) -> NodeId {
    let commands = build_painter(graph, tf, selected, pending_link, cursor_pos, [width, height]);
    tree.custom_paint(
        WidgetId::new("studio_node_canvas"),
        commands,
        leaf(width, height),
    )
    .expect("custom_paint node canvas")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_transform_roundtrip() {
        let mut tf = CanvasTransform::default();
        tf.pan = [100.0, 50.0];
        tf.zoom = 1.75;
        let p = [123.0, -45.0];
        let s = tf.graph_to_screen(p);
        let back = tf.screen_to_graph(s);
        assert!((back[0] - p[0]).abs() < 1e-3);
        assert!((back[1] - p[1]).abs() < 1e-3);
    }

    #[test]
    fn test_zoom_keeps_anchor_fixed() {
        let mut tf = CanvasTransform::default();
        let anchor = [200.0, 150.0];
        let before = tf.screen_to_graph(anchor);
        tf.zoom_at(anchor, 1.6);
        let after = tf.screen_to_graph(anchor);
        assert!((before[0] - after[0]).abs() < 1e-3);
        assert!((before[1] - after[1]).abs() < 1e-3);
    }

    #[test]
    fn test_hit_node() {
        let mut g = MaterialNodeGraph::new("Hit");
        let n = g.add_node(NodeKind::PerlinNoise, [200.0, 100.0]);
        let tf = CanvasTransform {
            pan: [0.0, 0.0],
            zoom: 1.0,
        };
        assert_eq!(hit_node(&g, &tf, [210.0, 130.0]), Some(n));
        assert_eq!(hit_node(&g, &tf, [10.0, 10.0]), None);
    }

    #[test]
    fn test_painter_produces_commands() {
        let mut g = MaterialNodeGraph::new("Paint");
        let a = g.add_node(NodeKind::Uv, [0.0, 0.0]);
        let b = g.add_node(NodeKind::PerlinNoise, [240.0, 0.0]);
        g.connect(a, "UV", b, "UV").unwrap();
        let tf = CanvasTransform::default();
        let cmds = build_painter(&g, &tf, Some(b), None, [0.0, 0.0], [800.0, 600.0]);
        assert!(!cmds.is_empty());
        // Au moins 3 nœuds (uv, noise, output) + liaisons
        assert!(cmds.len() > 6);
    }
}
