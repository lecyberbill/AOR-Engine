// [WFGY] Zone: SAFE | λ: 0.1 | Fallbacks: 0 | Action: True GPU Gaussian Bloom & Glow via Shadow & Mesh
use egui::{epaint::{Mesh, Vertex}, Color32, Painter, Pos2, Rect, Rounding};

/// Dessine un halo néon diffus et continu
pub fn paint_glow_rect(
    painter: &Painter,
    rect: Rect,
    rounding: Rounding,
    glow_color: Color32,
    blur_radius: f32,
) {
    if blur_radius <= 0.0 || glow_color.a() == 0 {
        return;
    }

    let glow_shape = egui::epaint::Shadow {
        offset: egui::Vec2::ZERO,
        blur: blur_radius,
        spread: blur_radius * 0.25,
        color: glow_color,
    };

    let shape = glow_shape.tessellate(rect, rounding);
    painter.add(shape);
}

/// Indicateur LED avec vrai dégradé radial (Mesh GPU)
pub fn paint_led_indicator(
    painter: &Painter,
    center: Pos2,
    radius: f32,
    color: Color32,
    is_active: bool,
) {
    if is_active {
        // Vrai dégradé radial doux du centre vers l'extérieur (Mesh radial)
        let segments = 24;
        let halo_radius = radius * 3.2;
        let mut mesh = Mesh::default();

        let center_idx = mesh.vertices.len() as u32;
        let inner_glow = Color32::from_rgba_premultiplied(color.r(), color.g(), color.b(), 130);
        let outer_glow = Color32::from_rgba_premultiplied(color.r(), color.g(), color.b(), 0);

        mesh.vertices.push(Vertex {
            pos: center,
            uv: egui::epaint::WHITE_UV,
            color: inner_glow,
        });

        for i in 0..=segments {
            let angle = (i as f32 / segments as f32) * std::f32::consts::TAU;
            let pos = Pos2::new(
                center.x + angle.cos() * halo_radius,
                center.y + angle.sin() * halo_radius,
            );
            mesh.vertices.push(Vertex {
                pos,
                uv: egui::epaint::WHITE_UV,
                color: outer_glow,
            });

            if i > 0 {
                mesh.indices.push(center_idx);
                mesh.indices.push(center_idx + i);
                mesh.indices.push(center_idx + i + 1);
            }
        }

        painter.add(mesh);
    }

    // Noyau de la LED
    painter.circle_filled(center, radius, color);
    painter.circle_filled(center, radius * 0.45, Color32::WHITE);
}
