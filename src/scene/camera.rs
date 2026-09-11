// [WFGY] Zone: SAFE | λ: 0.2 | Fallbacks: 0/None | Action: Real-time 3D Camera system with FPS and Orbital controls
#![allow(dead_code)]
use glam::{Mat4, Vec3};

#[repr(C)]
#[derive(Clone, Copy, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct CameraUniform {
    pub view_proj: [[f32; 4]; 4],
    pub view: [[f32; 4]; 4],
    pub proj: [[f32; 4]; 4],
    pub camera_pos: [f32; 4],
}

impl Default for CameraUniform {
    fn default() -> Self {
        Self {
            view_proj: Mat4::IDENTITY.to_cols_array_2d(),
            view: Mat4::IDENTITY.to_cols_array_2d(),
            proj: Mat4::IDENTITY.to_cols_array_2d(),
            camera_pos: [0.0, 0.0, 0.0, 1.0],
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CameraMode {
    Orbital,
    FirstPerson,
    ThirdPerson,
}

pub struct Camera {
    pub target: Vec3,
    pub distance: f32,
    pub yaw: f32,   // angle horizontal en radians
    pub pitch: f32, // angle vertical en radians
    pub fov_y: f32, // en radians
    pub z_near: f32,
    pub z_far: f32,
    pub mode: CameraMode,
}

impl Camera {
    pub fn new(target: Vec3, distance: f32, yaw: f32, pitch: f32) -> Self {
        Camera {
            target,
            distance,
            yaw,
            pitch,
            fov_y: 45.0_f32.to_radians(),
            z_near: 0.1,
            z_far: 100.0,
            mode: CameraMode::Orbital,
        }
    }

    /// Direction de visée 3D
    pub fn look_direction(&self) -> Vec3 {
        let cos_p = self.pitch.cos();
        Vec3::new(
            cos_p * self.yaw.sin(),
            self.pitch.sin(),
            -cos_p * self.yaw.cos(),
        ).normalize_or_zero()
    }

    pub fn forward(&self) -> Vec3 {
        self.look_direction()
    }

    pub fn position(&self) -> Vec3 {
        match self.mode {
            CameraMode::Orbital => {
                let pos_x = self.target.x + self.distance * self.pitch.cos() * self.yaw.sin();
                let pos_y = self.target.y + self.distance * self.pitch.sin();
                let pos_z = self.target.z - self.distance * self.pitch.cos() * self.yaw.cos();
                Vec3::new(pos_x, pos_y, pos_z)
            }
            CameraMode::FirstPerson => self.target,
            CameraMode::ThirdPerson => {
                let look = self.look_direction();
                self.target - look * self.distance + Vec3::new(0.0, 0.4, 0.0)
            }
        }
    }

    pub fn rotate(&mut self, delta_yaw: f32, delta_pitch: f32) {
        self.yaw += delta_yaw;
        // Clamping vertical à 89 degrés
        let max_pitch = 89.0_f32.to_radians();
        self.pitch = (self.pitch - delta_pitch).clamp(-max_pitch, max_pitch);
    }

    pub fn zoom(&mut self, delta: f32) {
        let zoom_step = delta * (self.distance * 0.15).max(0.4);
        self.distance = (self.distance - zoom_step).clamp(0.5, 200.0);
    }

    pub fn pan(&mut self, delta_x: f32, delta_y: f32) {
        let forward = (self.target - self.position()).normalize_or_zero();
        let right = forward.cross(Vec3::Y).normalize_or_zero();
        let up = right.cross(forward).normalize_or_zero();

        let pan_speed = self.distance * 0.0015;
        self.target -= right * delta_x * pan_speed;
        self.target += up * delta_y * pan_speed;
    }

    pub fn reset(&mut self) {
        self.target = Vec3::ZERO;
        self.distance = 6.0;
        self.yaw = 0.6;
        self.pitch = 0.4;
        self.mode = CameraMode::Orbital;
    }

    pub fn build_view_proj_matrix(&self, aspect_ratio: f32) -> (Mat4, Mat4, Mat4) {
        let pos = self.position();
        let view = match self.mode {
            CameraMode::FirstPerson => {
                let look_target = pos + self.look_direction();
                Mat4::look_at_rh(pos, look_target, Vec3::Y)
            }
            CameraMode::Orbital | CameraMode::ThirdPerson => {
                Mat4::look_at_rh(pos, self.target, Vec3::Y)
            }
        };
        let proj = Mat4::perspective_rh(self.fov_y, aspect_ratio.max(0.001), self.z_near, self.z_far);
        let view_proj = proj * view;
        (view_proj, view, proj)
    }

    pub fn to_uniform_with_time(&self, aspect_ratio: f32, time: f32) -> CameraUniform {
        let (view_proj, view, proj) = self.build_view_proj_matrix(aspect_ratio);
        let pos = self.position();
        CameraUniform {
            view_proj: view_proj.to_cols_array_2d(),
            view: view.to_cols_array_2d(),
            proj: proj.to_cols_array_2d(),
            camera_pos: [pos.x, pos.y, pos.z, time],
        }
    }

    #[allow(dead_code)]
    pub fn to_uniform(&self, aspect_ratio: f32) -> CameraUniform {
        self.to_uniform_with_time(aspect_ratio, 0.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_camera_position_and_matrices() {
        let cam = Camera::new(Vec3::ZERO, 5.0, 0.0, 0.0);
        let pos = cam.position();
        assert!((pos.x).abs() < 1e-5);
        assert!((pos.y).abs() < 1e-5);
        assert!((pos.z + 5.0).abs() < 1e-5);

        let (view_proj, view, proj) = cam.build_view_proj_matrix(16.0 / 9.0);
        assert_ne!(view_proj, Mat4::ZERO);
        assert_ne!(view, Mat4::ZERO);
        assert_ne!(proj, Mat4::ZERO);
    }
}
