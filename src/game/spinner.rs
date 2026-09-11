// [WFGY] Zone: SAFE | λ: 0.3 | Fallbacks: 0/None | Action: Blade Runner Cyber Spinner Vehicle Controller & Procedural Mesh
#![allow(dead_code)]
use glam::{Mat4, Quat, Vec3};
use crate::scene::mesh::Vertex;

/// État du véhicule volant (Spinner)
#[derive(Clone, Debug)]
pub struct CyberSpinner {
    pub position: Vec3,
    pub velocity: Vec3,
    pub rotation: Vec3, // (Pitch, Yaw, Roll) en radians
    pub speed: f32,
    pub max_speed: f32,
    pub boost_speed: f32,
    pub is_boosting: bool,
    pub boost_fuel: f32, // 0.0 à 100.0%
    pub bank_angle: f32, // Inclinaison dynamique dans les virages
    pub engine_glow_intensity: f32,
}

impl Default for CyberSpinner {
    fn default() -> Self {
        Self {
            position: Vec3::new(0.0, 15.0, -30.0),
            velocity: Vec3::ZERO,
            rotation: Vec3::ZERO,
            speed: 0.0,
            max_speed: 45.0,      // ~160 km/h
            boost_speed: 90.0,    // ~320 km/h
            is_boosting: false,
            boost_fuel: 100.0,
            bank_angle: 0.0,
            engine_glow_intensity: 1.0,
        }
    }
}

impl CyberSpinner {
    pub fn new(spawn_pos: Vec3) -> Self {
        let mut s = Self::default();
        s.position = spawn_pos;
        s
    }

    /// Met à jour la physique et la dynamique de vol du Spinner
    pub fn update(
        &mut self,
        dt: f32,
        throttle: f32, // -1.0 (frein/arrière) à +1.0 (avant)
        steer_yaw: f32, // -1.0 (gauche) à +1.0 (droite)
        pitch_input: f32, // -1.0 (piquer) à +1.0 (cabrer)
        strafe: f32, // -1.0 (gauche) à +1.0 (droite)
        vertical: f32, // -1.0 (descendre) à +1.0 (monter)
        boost_requested: bool,
    ) {
        // 1. Gestion du Boost Post-Combustion
        if boost_requested && self.boost_fuel > 5.0 && throttle > 0.1 {
            self.is_boosting = true;
            self.boost_fuel = (self.boost_fuel - dt * 25.0).max(0.0);
            self.engine_glow_intensity = (self.engine_glow_intensity + dt * 5.0).min(3.5);
        } else {
            self.is_boosting = false;
            self.boost_fuel = (self.boost_fuel + dt * 10.0).min(100.0);
            self.engine_glow_intensity = (self.engine_glow_intensity - dt * 3.0).max(1.0);
        }

        let target_max_speed = if self.is_boosting { self.boost_speed } else { self.max_speed };

        // 2. Accélération longitudinale & freinage avec traînée aérodynamique
        let target_speed = throttle * target_max_speed;
        let accel_rate = if throttle > 0.0 { 25.0 } else { 35.0 };
        self.speed += (target_speed - self.speed) * (accel_rate * dt).min(1.0);

        // 3. Contrôle angulaire (Yaw, Pitch, Roll)
        let turn_speed = 1.6;
        self.rotation.y -= steer_yaw * turn_speed * dt; // Yaw
        self.rotation.x += pitch_input * 1.2 * dt;      // Pitch
        self.rotation.x = self.rotation.x.clamp(-0.4, 0.4); // Limiter l'assiette

        // Auto-inclinaison (Bank Roll) fluide dans les virages
        let target_bank = -steer_yaw * 0.55;
        self.bank_angle += (target_bank - self.bank_angle) * (8.0 * dt).min(1.0);
        self.rotation.z = self.bank_angle;

        // 4. Calcul des vecteurs directionnels locaux
        let rot_quat = Quat::from_euler(glam::EulerRot::YXZ, self.rotation.y, self.rotation.x, self.rotation.z);
        let forward = rot_quat * Vec3::new(0.0, 0.0, 1.0);
        let right = rot_quat * Vec3::new(1.0, 0.0, 0.0);
        let _up = rot_quat * Vec3::new(0.0, 1.0, 0.0);

        // 5. Vitesse linéaire & déplacements 3D (Forward + Strafe + Vertical)
        let strafe_vel = right * (strafe * 18.0);
        let vert_vel = Vec3::Y * (vertical * 14.0);
        let forward_vel = forward * self.speed;

        let target_velocity = forward_vel + strafe_vel + vert_vel;
        self.velocity += (target_velocity - self.velocity) * (10.0 * dt).min(1.0);
        self.position += self.velocity * dt;

        // Limite de plancher de sécurité au-dessus du sol
        if self.position.y < 2.5 {
            self.position.y = 2.5;
            if self.velocity.y < 0.0 {
                self.velocity.y = 0.0;
            }
        }
    }

    /// Matrice de transformation mondiale pour le rendu du Spinner
    pub fn transform_matrix(&self) -> Mat4 {
        let rot = Quat::from_euler(glam::EulerRot::YXZ, self.rotation.y, self.rotation.x, self.rotation.z);
        Mat4::from_scale_rotation_translation(Vec3::ONE, rot, self.position)
    }

    /// Positions mondiales des deux réacteurs arrières pour les particules de plasma
    pub fn exhaust_positions(&self) -> (Vec3, Vec3) {
        let rot = Quat::from_euler(glam::EulerRot::YXZ, self.rotation.y, self.rotation.x, self.rotation.z);
        let left_offset = rot * Vec3::new(-0.8, -0.1, -1.8);
        let right_offset = rot * Vec3::new(0.8, -0.1, -1.8);
        (self.position + left_offset, self.position + right_offset)
    }

    /// Vitesse en kilomètres par heure
    pub fn speed_kmh(&self) -> f32 {
        self.speed * 3.6
    }
}

/// Génère la géométrie procédurale du Spinner de police Cyberpunk Blade Runner
pub fn generate_cyber_spinner_mesh() -> (Vec<Vertex>, Vec<u32>) {
    let mut vertices = Vec::new();
    let mut indices = Vec::new();

    // Helper pour ajouter un quad
    let mut add_quad = |p0: Vec3, p1: Vec3, p2: Vec3, p3: Vec3, normal: Vec3, uv0: [f32; 2], uv1: [f32; 2]| {
        let start = vertices.len() as u32;
        vertices.push(Vertex { position: p0.to_array(), normal: normal.to_array(), tangent: [1.0, 0.0, 0.0, 1.0], uv: [uv0[0], uv0[1]], color: [1.0, 1.0, 1.0, 1.0] });
        vertices.push(Vertex { position: p1.to_array(), normal: normal.to_array(), tangent: [1.0, 0.0, 0.0, 1.0], uv: [uv1[0], uv0[1]], color: [1.0, 1.0, 1.0, 1.0] });
        vertices.push(Vertex { position: p2.to_array(), normal: normal.to_array(), tangent: [1.0, 0.0, 0.0, 1.0], uv: [uv1[0], uv1[1]], color: [1.0, 1.0, 1.0, 1.0] });
        vertices.push(Vertex { position: p3.to_array(), normal: normal.to_array(), tangent: [1.0, 0.0, 0.0, 1.0], uv: [uv0[0], uv1[1]], color: [1.0, 1.0, 1.0, 1.0] });

        indices.extend_from_slice(&[start, start + 1, start + 2, start, start + 2, start + 3]);
    };

    // 1. Châssis principal aérodynamique (profil cunéiforme / biseauté)
    let nose = Vec3::new(0.0, -0.2, 2.2);
    let front_left = Vec3::new(-0.9, -0.2, 1.4);
    let front_right = Vec3::new(0.9, -0.2, 1.4);
    let mid_left = Vec3::new(-1.1, 0.0, 0.0);
    let mid_right = Vec3::new(1.1, 0.0, 0.0);
    let rear_left = Vec3::new(-1.0, 0.1, -1.8);
    let rear_right = Vec3::new(1.0, 0.1, -1.8);
    
    // Toit / Cockpit verrière
    let roof_front = Vec3::new(0.0, 0.6, 0.3);
    let roof_back = Vec3::new(0.0, 0.7, -0.8);
    let roof_left = Vec3::new(-0.6, 0.5, -0.4);
    let roof_right = Vec3::new(0.6, 0.5, -0.4);

    // Capot avant effilé
    add_quad(nose, front_right, roof_front, front_left, Vec3::new(0.0, 0.7, 0.7).normalize(), [0.0, 0.0], [1.0, 1.0]);
    // Toit du cockpit
    add_quad(roof_front, roof_right, roof_back, roof_left, Vec3::Y, [0.0, 0.0], [1.0, 1.0]);
    // Flancs latéraux
    add_quad(front_left, roof_left, rear_left, mid_left, -Vec3::X, [0.0, 0.0], [1.0, 1.0]);
    add_quad(front_right, mid_right, rear_right, roof_right, Vec3::X, [0.0, 0.0], [1.0, 1.0]);
    // Arrière / Diffuseur
    add_quad(rear_left, roof_back, rear_right, Vec3::new(0.0, -0.3, -1.8), -Vec3::Z, [0.0, 0.0], [1.0, 1.0]);
    // Plancher inférieur
    add_quad(front_left, nose, front_right, Vec3::new(0.0, -0.4, 0.0), -Vec3::Y, [0.0, 0.0], [1.0, 1.0]);

    // 2. Doubles Tuyères de Réacteurs Arrières Néon Cyan
    let left_exhaust_c = Vec3::new(-0.7, 0.0, -1.85);
    let right_exhaust_c = Vec3::new(0.7, 0.0, -1.85);
    let r = 0.28;

    for (c, normal) in [(left_exhaust_c, -Vec3::Z), (right_exhaust_c, -Vec3::Z)] {
        let p0 = c + Vec3::new(-r, -r, 0.0);
        let p1 = c + Vec3::new(r, -r, 0.0);
        let p2 = c + Vec3::new(r, r, 0.0);
        let p3 = c + Vec3::new(-r, r, 0.0);
        add_quad(p0, p1, p2, p3, normal, [0.0, 0.0], [1.0, 1.0]);
    }

    (vertices, indices)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cyber_spinner_flight_dynamics_and_boost() {
        let mut spinner = CyberSpinner::new(Vec3::new(0.0, 10.0, 0.0));
        
        // Accélération classique
        spinner.update(0.1, 1.0, 0.0, 0.0, 0.0, 0.0, false);
        assert!(spinner.speed > 0.0);
        assert_eq!(spinner.is_boosting, false);

        // Activation du boost
        spinner.update(0.1, 1.0, 0.0, 0.0, 0.0, 0.0, true);
        assert!(spinner.is_boosting);
        assert!(spinner.boost_fuel < 100.0);
        assert!(spinner.engine_glow_intensity > 1.0);

        // Virage et inclinaison
        spinner.update(0.1, 1.0, 1.0, 0.0, 0.0, 0.0, false);
        assert!(spinner.bank_angle < 0.0); // Incliné à gauche pour virage droit
    }
}
