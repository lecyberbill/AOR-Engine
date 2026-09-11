// [WFGY] Zone: SAFE | λ: 0.2 | Fallbacks: 0/None | Action: Interactive 3D Translation Gizmo decoupled from egui
use glam::{Mat4, Vec2, Vec3};
use crate::scene::raycast::{world_to_screen, Ray};
use crate::scene::SceneNode;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GizmoAxis {
    None,
    X,
    Y,
    Z,
    Center,
}

pub struct Gizmo {
    pub active_axis: GizmoAxis,
    pub hovered_axis: GizmoAxis,
    pub drag_start_translation: Option<Vec3>,
    pub drag_start_hit_point: Option<Vec3>,
    pub drag_plane_normal: Option<Vec3>,
    pub axis_length: f32, // En unités 3D
}

impl Default for Gizmo {
    fn default() -> Self {
        Self {
            active_axis: GizmoAxis::None,
            hovered_axis: GizmoAxis::None,
            drag_start_translation: None,
            drag_start_hit_point: None,
            drag_plane_normal: None,
            axis_length: 1.6,
        }
    }
}

impl Gizmo {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn is_interacting(&self) -> bool {
        self.active_axis != GizmoAxis::None
    }

    /// Détecte le survol et gère la manipulation de l'axe actif
    /// viewport_rect: [x, y, width, height]
    pub fn update(
        &mut self,
        node: &mut SceneNode,
        camera_ray: &Ray,
        view_proj: Mat4,
        viewport_rect: [f32; 4],
        mouse_pos: Option<Vec2>,
        is_primary_down: bool,
    ) -> bool {
        let node_pos = node.transform.translation;

        let center_screen = match world_to_screen(node_pos, view_proj, viewport_rect) {
            Some(p) => p,
            None => {
                self.hovered_axis = GizmoAxis::None;
                return false;
            }
        };

        let x_end_screen = world_to_screen(node_pos + Vec3::X * self.axis_length, view_proj, viewport_rect);
        let y_end_screen = world_to_screen(node_pos + Vec3::Y * self.axis_length, view_proj, viewport_rect);
        let z_end_screen = world_to_screen(node_pos + Vec3::Z * self.axis_length, view_proj, viewport_rect);

        // 1. Détection du survol (Hover detection en 2D écran)
        if let Some(mouse) = mouse_pos {
            if !is_primary_down {
                self.active_axis = GizmoAxis::None;
                self.drag_start_translation = None;
                self.drag_start_hit_point = None;
                self.drag_plane_normal = None;

                let hit_threshold = 14.0;

                if mouse.distance(center_screen) <= 12.0 {
                    self.hovered_axis = GizmoAxis::Center;
                } else if let Some(x_screen) = x_end_screen {
                    if distance_to_segment(mouse, center_screen, x_screen) <= hit_threshold {
                        self.hovered_axis = GizmoAxis::X;
                    } else if let Some(y_screen) = y_end_screen {
                        if distance_to_segment(mouse, center_screen, y_screen) <= hit_threshold {
                            self.hovered_axis = GizmoAxis::Y;
                        } else if let Some(z_screen) = z_end_screen {
                            if distance_to_segment(mouse, center_screen, z_screen) <= hit_threshold {
                                self.hovered_axis = GizmoAxis::Z;
                            } else {
                                self.hovered_axis = GizmoAxis::None;
                            }
                        } else {
                            self.hovered_axis = GizmoAxis::None;
                        }
                    } else {
                        self.hovered_axis = GizmoAxis::None;
                    }
                } else {
                    self.hovered_axis = GizmoAxis::None;
                }
            }
        } else {
            self.hovered_axis = GizmoAxis::None;
        }

        // 2. Début du drag (Initialisation du plan d'intersection 3D)
        if is_primary_down && self.active_axis == GizmoAxis::None && self.hovered_axis != GizmoAxis::None {
            self.active_axis = self.hovered_axis;
            self.drag_start_translation = Some(node.transform.translation);

            let plane_normal = match self.active_axis {
                GizmoAxis::Center => -camera_ray.direction,
                GizmoAxis::X => {
                    let n = Vec3::X.cross(camera_ray.direction).cross(Vec3::X);
                    if n.length_squared() < 1e-4 { Vec3::Y } else { n.normalize() }
                }
                GizmoAxis::Y => {
                    let n = Vec3::Y.cross(camera_ray.direction).cross(Vec3::Y);
                    if n.length_squared() < 1e-4 { Vec3::Z } else { n.normalize() }
                }
                GizmoAxis::Z => {
                    let n = Vec3::Z.cross(camera_ray.direction).cross(Vec3::Z);
                    if n.length_squared() < 1e-4 { Vec3::Y } else { n.normalize() }
                }
                GizmoAxis::None => Vec3::Y,
            };

            self.drag_plane_normal = Some(plane_normal);

            if let Some(t) = camera_ray.intersect_plane(node.transform.translation, plane_normal) {
                self.drag_start_hit_point = Some(camera_ray.at(t));
            }
        }

        // 3. Traitement du déplacement 3D lors du drag
        if is_primary_down && self.active_axis != GizmoAxis::None {
            if let (Some(initial_trans), Some(initial_hit), Some(plane_normal)) = (
                self.drag_start_translation,
                self.drag_start_hit_point,
                self.drag_plane_normal,
            ) {
                if let Some(t) = camera_ray.intersect_plane(initial_trans, plane_normal) {
                    let current_hit = camera_ray.at(t);
                    let delta_world = current_hit - initial_hit;

                    match self.active_axis {
                        GizmoAxis::X => {
                            let delta_x = delta_world.dot(Vec3::X);
                            node.transform.translation = initial_trans + Vec3::X * delta_x;
                        }
                        GizmoAxis::Y => {
                            let delta_y = delta_world.dot(Vec3::Y);
                            node.transform.translation = initial_trans + Vec3::Y * delta_y;
                        }
                        GizmoAxis::Z => {
                            let delta_z = delta_world.dot(Vec3::Z);
                            node.transform.translation = initial_trans + Vec3::Z * delta_z;
                        }
                        GizmoAxis::Center => {
                            node.transform.translation = initial_trans + delta_world;
                        }
                        GizmoAxis::None => {}
                    }
                }
            }
            return true;
        }

        self.hovered_axis != GizmoAxis::None
    }

    /// Génère les commandes de dessin 2D vectoriel (CustomPaint) pour afficher le Gizmo Cyber-Glass
    pub fn generate_paint_commands(
        &self,
        node_pos: Vec3,
        view_proj: Mat4,
        viewport_rect: [f32; 4],
    ) -> Vec<ui_widgets::PaintCommand> {
        let mut painter = ui_widgets::Painter::new();

        let center_screen = match world_to_screen(node_pos, view_proj, viewport_rect) {
            Some(p) => p,
            None => return painter.finish(),
        };

        let x_end_screen = world_to_screen(node_pos + Vec3::X * self.axis_length, view_proj, viewport_rect);
        let y_end_screen = world_to_screen(node_pos + Vec3::Y * self.axis_length, view_proj, viewport_rect);
        let z_end_screen = world_to_screen(node_pos + Vec3::Z * self.axis_length, view_proj, viewport_rect);

        let center_p = [center_screen.x, center_screen.y];

        // 1. Cercle Central (GizmoAxis::Center)
        let is_center_active = self.active_axis == GizmoAxis::Center;
        let is_center_hovered = self.hovered_axis == GizmoAxis::Center;
        let center_color = if is_center_active {
            [1.0, 1.0, 1.0, 1.0]
        } else if is_center_hovered {
            [0.2, 0.8, 1.0, 1.0]
        } else {
            [0.2, 0.6, 1.0, 0.75]
        };
        painter.circle(
            center_p,
            8.0,
            Some([center_color[0], center_color[1], center_color[2], 0.35]),
            Some((center_color, 2.0)),
        );

        // 2. Axe X (Rouge Cyber #FF3B30 / #FF6B60)
        if let Some(x_end) = x_end_screen {
            let x_p = [x_end.x, x_end.y];
            let is_x_active = self.active_axis == GizmoAxis::X;
            let is_x_hovered = self.hovered_axis == GizmoAxis::X;
            let x_col = if is_x_active {
                [1.0, 1.0, 0.4, 1.0] // Jaune éclatant lors du drag
            } else if is_x_hovered {
                [1.0, 0.45, 0.45, 1.0] // Rouge clair
            } else {
                [0.95, 0.20, 0.25, 0.90] // Rouge standard
            };
            let stroke_w = if is_x_hovered || is_x_active { 4.0 } else { 2.5 };
            painter.line(center_p, x_p, stroke_w, x_col);
            painter.circle(x_p, 5.0, Some(x_col), Some(([1.0, 1.0, 1.0, 0.8], 1.0)));
            painter.text([x_p[0] + 6.0, x_p[1] - 8.0], "X", 13.0, x_col);
        }

        // 3. Axe Y (Vert Cyber #34C759 / #5CE682)
        if let Some(y_end) = y_end_screen {
            let y_p = [y_end.x, y_end.y];
            let is_y_active = self.active_axis == GizmoAxis::Y;
            let is_y_hovered = self.hovered_axis == GizmoAxis::Y;
            let y_col = if is_y_active {
                [1.0, 1.0, 0.4, 1.0] // Jaune éclatant lors du drag
            } else if is_y_hovered {
                [0.45, 1.0, 0.55, 1.0] // Vert clair
            } else {
                [0.20, 0.85, 0.35, 0.90] // Vert standard
            };
            let stroke_w = if is_y_hovered || is_y_active { 4.0 } else { 2.5 };
            painter.line(center_p, y_p, stroke_w, y_col);
            painter.circle(y_p, 5.0, Some(y_col), Some(([1.0, 1.0, 1.0, 0.8], 1.0)));
            painter.text([y_p[0] + 6.0, y_p[1] - 8.0], "Y", 13.0, y_col);
        }

        // 4. Axe Z (Bleu Cyber #0A84FF / #64D2FF)
        if let Some(z_end) = z_end_screen {
            let z_p = [z_end.x, z_end.y];
            let is_z_active = self.active_axis == GizmoAxis::Z;
            let is_z_hovered = self.hovered_axis == GizmoAxis::Z;
            let z_col = if is_z_active {
                [1.0, 1.0, 0.4, 1.0] // Jaune éclatant lors du drag
            } else if is_z_hovered {
                [0.45, 0.85, 1.0, 1.0] // Bleu clair
            } else {
                [0.15, 0.55, 1.0, 0.90] // Bleu standard
            };
            let stroke_w = if is_z_hovered || is_z_active { 4.0 } else { 2.5 };
            painter.line(center_p, z_p, stroke_w, z_col);
            painter.circle(z_p, 5.0, Some(z_col), Some(([1.0, 1.0, 1.0, 0.8], 1.0)));
            painter.text([z_p[0] + 6.0, z_p[1] - 8.0], "Z", 13.0, z_col);
        }

        painter.finish()
    }
}

/// Calcule la distance minimale entre un point P et un segment AB
fn distance_to_segment(p: Vec2, a: Vec2, b: Vec2) -> f32 {
    let ab = b - a;
    let ab_len_sq = ab.length_squared();
    if ab_len_sq < 1e-6 {
        return p.distance(a);
    }

    let ap = p - a;
    let t = (ap.dot(ab) / ab_len_sq).clamp(0.0, 1.0);
    let closest = a + ab * t;
    p.distance(closest)
}
