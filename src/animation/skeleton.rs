// [WFGY] Zone: SAFE | λ: 0.25 | Fallbacks: 0 | Action: Skeletal Animation Joint hierarchy and Skinning Matrix palette
#![allow(dead_code)]
use glam::{Mat4, Quat, Vec3};

/// Os / Articulation du squelette 3D
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Joint {
    pub id: usize,
    pub name: String,
    pub parent_index: Option<usize>,
    pub local_translation: Vec3,
    pub local_rotation: Quat,
    pub local_scale: Vec3,
    pub inverse_bind_matrix: Mat4,
}

impl Joint {
    pub fn new(id: usize, name: &str, parent_index: Option<usize>) -> Self {
        Self {
            id,
            name: name.to_string(),
            parent_index,
            local_translation: Vec3::ZERO,
            local_rotation: Quat::IDENTITY,
            local_scale: Vec3::ONE,
            inverse_bind_matrix: Mat4::IDENTITY,
        }
    }

    pub fn local_matrix(&self) -> Mat4 {
        Mat4::from_scale_rotation_translation(self.local_scale, self.local_rotation, self.local_translation)
    }
}

/// Squelette d'animation composé d'une hiérarchie d'articulations (Joints)
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct Skeleton {
    pub name: String,
    pub joints: Vec<Joint>,
}

impl Skeleton {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            joints: Vec::new(),
        }
    }

    pub fn add_joint(&mut self, mut joint: Joint) -> usize {
        let idx = self.joints.len();
        joint.id = idx;
        self.joints.push(joint);
        idx
    }

    /// Calcule les matrices mondiales / globales de chaque articulation dans l'ordre hiérarchique
    pub fn compute_global_matrices(&self) -> Vec<Mat4> {
        let mut globals = vec![Mat4::IDENTITY; self.joints.len()];

        for (idx, joint) in self.joints.iter().enumerate() {
            let local_mat = joint.local_matrix();
            globals[idx] = if let Some(parent_idx) = joint.parent_index {
                globals[parent_idx] * local_mat
            } else {
                local_mat
            };
        }

        globals
    }

    /// Calcule la palette finale de matrices de skinning pour le GPU (Global * InverseBindMatrix)
    pub fn compute_skinning_matrices(&self) -> Vec<Mat4> {
        let globals = self.compute_global_matrices();
        let mut skinning_matrices = Vec::with_capacity(self.joints.len());

        for (idx, joint) in self.joints.iter().enumerate() {
            skinning_matrices.push(globals[idx] * joint.inverse_bind_matrix);
        }

        skinning_matrices
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_skeleton_hierarchy_transforms() {
        let mut skel = Skeleton::new("TestArm");
        let root = Joint::new(0, "Root", None);
        let mut child = Joint::new(1, "Elbow", Some(0));
        child.local_translation = Vec3::new(0.0, 1.0, 0.0);

        skel.add_joint(root);
        skel.add_joint(child);

        let globals = skel.compute_global_matrices();
        assert_eq!(globals.len(), 2);
        assert_eq!(globals[1].w_axis.y, 1.0);
    }
}
