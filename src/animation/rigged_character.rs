// [WFGY] Zone: SAFE | λ: 0.25 | Fallbacks: 0 | Action: Rigged Character Generator with Bones, Weights and Animation Clips
#![allow(dead_code)]
use glam::{Quat, Vec3};
use crate::scene::mesh::Vertex;
use crate::animation::skeleton::{Joint, Skeleton};
use crate::animation::clip::{AnimationClip, AnimationPlayer, JointAnimationTrack, Keyframe};

/// Sommet riggé avec assignation d'os et poids
#[derive(Clone, Copy, Debug)]
pub struct SkinnedVertex {
    pub base_pos: Vec3,
    pub base_normal: Vec3,
    pub uv: [f32; 2],
    pub color: [f32; 4],
    pub joint_indices: [usize; 2],
    pub joint_weights: [f32; 2],
}

/// Modèle articulé complet avec squelette, données de maillage et lecteur d'animation
pub struct RiggedModel {
    pub skeleton: Skeleton,
    pub player: AnimationPlayer,
    pub skinned_vertices: Vec<SkinnedVertex>,
    pub indices: Vec<u32>,
}

impl RiggedModel {
    /// Crée un personnage articulé (Robot/Mannequin humanoïde) avec un squelette complet
    pub fn create_humanoid() -> Self {
        let mut skeleton = Skeleton::new("Humanoid");

        // 0: Root / Pelvis (Hanches)
        let mut root = Joint::new(0, "Hips", None);
        root.local_translation = Vec3::new(0.0, 1.0, 0.0);
        skeleton.add_joint(root);

        // 1: Spine / Torso (Torse)
        let mut spine = Joint::new(1, "Spine", Some(0));
        spine.local_translation = Vec3::new(0.0, 0.5, 0.0);
        skeleton.add_joint(spine);

        // 2: Head (Tête)
        let mut head = Joint::new(2, "Head", Some(1));
        head.local_translation = Vec3::new(0.0, 0.5, 0.0);
        skeleton.add_joint(head);

        // 3: Left Arm (Bras Gauche)
        let mut left_arm = Joint::new(3, "LeftArm", Some(1));
        left_arm.local_translation = Vec3::new(0.4, 0.35, 0.0);
        skeleton.add_joint(left_arm);

        // 4: Right Arm (Bras Droit)
        let mut right_arm = Joint::new(4, "RightArm", Some(1));
        right_arm.local_translation = Vec3::new(-0.4, 0.35, 0.0);
        skeleton.add_joint(right_arm);

        // 5: Left Leg (Jambe Gauche)
        let mut left_leg = Joint::new(5, "LeftLeg", Some(0));
        left_leg.local_translation = Vec3::new(0.2, -0.1, 0.0);
        skeleton.add_joint(left_leg);

        // 6: Right Leg (Jambe Droite)
        let mut right_leg = Joint::new(6, "RightLeg", Some(0));
        right_leg.local_translation = Vec3::new(-0.2, -0.1, 0.0);
        skeleton.add_joint(right_leg);

        // Calcul des matrices inverses de pose de référence (Inverse Bind Matrices)
        let globals = skeleton.compute_global_matrices();
        for (idx, joint) in skeleton.joints.iter_mut().enumerate() {
            joint.inverse_bind_matrix = globals[idx].inverse();
        }

        // Construction de la géométrie du mannequin riggé
        let mut skinned_vertices = Vec::new();
        let mut indices = Vec::new();

        // 1. Torse (lié à Spine = 1)
        add_box_skinned(
            &mut skinned_vertices,
            &mut indices,
            Vec3::new(0.0, 1.25, 0.0),
            Vec3::new(0.5, 0.55, 0.25),
            [0.2, 0.6, 0.9, 1.0], // Cyan / Bleu Cyber
            1,
        );

        // 2. Tête (liée à Head = 2)
        add_box_skinned(
            &mut skinned_vertices,
            &mut indices,
            Vec3::new(0.0, 1.75, 0.0),
            Vec3::new(0.3, 0.3, 0.3),
            [0.9, 0.8, 0.3, 1.0], // Or / Ambre
            2,
        );

        // 3. Bras Gauche (lié à LeftArm = 3)
        add_box_skinned(
            &mut skinned_vertices,
            &mut indices,
            Vec3::new(0.45, 1.05, 0.0),
            Vec3::new(0.18, 0.55, 0.18),
            [0.8, 0.3, 0.8, 1.0], // Magenta
            3,
        );

        // 4. Bras Droit (lié à RightArm = 4)
        add_box_skinned(
            &mut skinned_vertices,
            &mut indices,
            Vec3::new(-0.45, 1.05, 0.0),
            Vec3::new(0.18, 0.55, 0.18),
            [0.8, 0.3, 0.8, 1.0],
            4,
        );

        // 5. Jambe Gauche (liée à LeftLeg = 5)
        add_box_skinned(
            &mut skinned_vertices,
            &mut indices,
            Vec3::new(0.2, 0.45, 0.0),
            Vec3::new(0.2, 0.85, 0.2),
            [0.2, 0.8, 0.4, 1.0], // Vert Cyber
            5,
        );

        // 6. Jambe Droite (liée à RightLeg = 6)
        add_box_skinned(
            &mut skinned_vertices,
            &mut indices,
            Vec3::new(-0.2, 0.45, 0.0),
            Vec3::new(0.2, 0.85, 0.2),
            [0.2, 0.8, 0.4, 1.0],
            6,
        );

        // Création des clips d'animation
        let mut player = AnimationPlayer::new();
        player.add_clip(Self::create_walk_clip());
        player.add_clip(Self::create_idle_clip());
        player.add_clip(Self::create_wave_clip());

        Self {
            skeleton,
            player,
            skinned_vertices,
            indices,
        }
    }

    /// Clip 0 : Cycle de marche humanoïde
    pub fn create_walk_clip() -> AnimationClip {
        let mut clip = AnimationClip::new("Walk Cycle", 1.0);

        // Hanches (Léger rebond vertical)
        let mut hips_track = JointAnimationTrack::new(0);
        hips_track.translation_keys.push(Keyframe { time: 0.0, value: Vec3::new(0.0, 1.0, 0.0) });
        hips_track.translation_keys.push(Keyframe { time: 0.25, value: Vec3::new(0.0, 1.04, 0.0) });
        hips_track.translation_keys.push(Keyframe { time: 0.5, value: Vec3::new(0.0, 1.0, 0.0) });
        hips_track.translation_keys.push(Keyframe { time: 0.75, value: Vec3::new(0.0, 1.04, 0.0) });
        hips_track.translation_keys.push(Keyframe { time: 1.0, value: Vec3::new(0.0, 1.0, 0.0) });
        clip.tracks.push(hips_track);

        // Bras Gauche (Balancement avant/arrière)
        let mut left_arm_track = JointAnimationTrack::new(3);
        left_arm_track.rotation_keys.push(Keyframe { time: 0.0, value: Quat::from_rotation_x(0.6) });
        left_arm_track.rotation_keys.push(Keyframe { time: 0.5, value: Quat::from_rotation_x(-0.6) });
        left_arm_track.rotation_keys.push(Keyframe { time: 1.0, value: Quat::from_rotation_x(0.6) });
        clip.tracks.push(left_arm_track);

        // Bras Droit (Opposition avec le bras gauche)
        let mut right_arm_track = JointAnimationTrack::new(4);
        right_arm_track.rotation_keys.push(Keyframe { time: 0.0, value: Quat::from_rotation_x(-0.6) });
        right_arm_track.rotation_keys.push(Keyframe { time: 0.5, value: Quat::from_rotation_x(0.6) });
        right_arm_track.rotation_keys.push(Keyframe { time: 1.0, value: Quat::from_rotation_x(-0.6) });
        clip.tracks.push(right_arm_track);

        // Jambe Gauche
        let mut left_leg_track = JointAnimationTrack::new(5);
        left_leg_track.rotation_keys.push(Keyframe { time: 0.0, value: Quat::from_rotation_x(-0.5) });
        left_leg_track.rotation_keys.push(Keyframe { time: 0.5, value: Quat::from_rotation_x(0.5) });
        left_leg_track.rotation_keys.push(Keyframe { time: 1.0, value: Quat::from_rotation_x(-0.5) });
        clip.tracks.push(left_leg_track);

        // Jambe Droite
        let mut right_leg_track = JointAnimationTrack::new(6);
        right_leg_track.rotation_keys.push(Keyframe { time: 0.0, value: Quat::from_rotation_x(0.5) });
        right_leg_track.rotation_keys.push(Keyframe { time: 0.5, value: Quat::from_rotation_x(-0.5) });
        right_leg_track.rotation_keys.push(Keyframe { time: 1.0, value: Quat::from_rotation_x(0.5) });
        clip.tracks.push(right_leg_track);

        clip
    }

    /// Clip 1 : Respiration au repos (Idle)
    pub fn create_idle_clip() -> AnimationClip {
        let mut clip = AnimationClip::new("Idle Breathing", 2.0);

        // Torse (Légère expansion & inclinaison)
        let mut spine_track = JointAnimationTrack::new(1);
        spine_track.scale_keys.push(Keyframe { time: 0.0, value: Vec3::ONE });
        spine_track.scale_keys.push(Keyframe { time: 1.0, value: Vec3::new(1.05, 1.03, 1.05) });
        spine_track.scale_keys.push(Keyframe { time: 2.0, value: Vec3::ONE });
        clip.tracks.push(spine_track);

        // Tête (Léger hochement naturel)
        let mut head_track = JointAnimationTrack::new(2);
        head_track.rotation_keys.push(Keyframe { time: 0.0, value: Quat::IDENTITY });
        head_track.rotation_keys.push(Keyframe { time: 1.0, value: Quat::from_rotation_x(-0.08) });
        head_track.rotation_keys.push(Keyframe { time: 2.0, value: Quat::IDENTITY });
        clip.tracks.push(head_track);

        clip
    }

    /// Clip 2 : Salut de la main (Victory Wave)
    pub fn create_wave_clip() -> AnimationClip {
        let mut clip = AnimationClip::new("Victory Wave", 1.5);

        let mut arm_track = JointAnimationTrack::new(3);
        // Lever le bras gauche et agiter
        let up = Quat::from_rotation_z(2.2);
        let wave_left = Quat::from_rotation_z(2.5) * Quat::from_rotation_x(0.2);
        let wave_right = Quat::from_rotation_z(1.9) * Quat::from_rotation_x(-0.2);

        arm_track.rotation_keys.push(Keyframe { time: 0.0, value: Quat::IDENTITY });
        arm_track.rotation_keys.push(Keyframe { time: 0.3, value: up });
        arm_track.rotation_keys.push(Keyframe { time: 0.6, value: wave_left });
        arm_track.rotation_keys.push(Keyframe { time: 0.9, value: wave_right });
        arm_track.rotation_keys.push(Keyframe { time: 1.2, value: wave_left });
        arm_track.rotation_keys.push(Keyframe { time: 1.5, value: up });
        clip.tracks.push(arm_track);

        clip
    }

    /// Évalue la pose actuelle et produit les sommets finaux déformés par Linear Blend Skinning
    pub fn compute_skinned_vertices(&self) -> Vec<Vertex> {
        let skinning_matrices = self.skeleton.compute_skinning_matrices();
        let mut out = Vec::with_capacity(self.skinned_vertices.len());

        for sv in &self.skinned_vertices {
            let m0 = skinning_matrices[sv.joint_indices[0]];
            let m1 = skinning_matrices[sv.joint_indices[1]];

            let w0 = sv.joint_weights[0];
            let w1 = sv.joint_weights[1];

            // Blend Matrix: M = w0*M0 + w1*M1
            let blended_mat = m0 * w0 + m1 * w1;

            let pos = blended_mat.transform_point3(sv.base_pos);
            let normal = blended_mat.transform_vector3(sv.base_normal).normalize_or_zero();

            // Tangente approximative
            let tangent = if normal.y.abs() > 0.99 {
                Vec3::new(1.0, 0.0, 0.0)
            } else {
                Vec3::new(0.0, 1.0, 0.0).cross(normal).normalize_or_zero()
            };

            out.push(Vertex {
                position: pos.to_array(),
                normal: normal.to_array(),
                tangent: [tangent.x, tangent.y, tangent.z, 1.0],
                uv: sv.uv,
                color: sv.color,
            });
        }

        out
    }
}

/// Helper pour ajouter un parallélépipède riggé à un os donné
fn add_box_skinned(
    vertices: &mut Vec<SkinnedVertex>,
    indices: &mut Vec<u32>,
    center: Vec3,
    size: Vec3,
    color: [f32; 4],
    joint_idx: usize,
) {
    let half = size * 0.5;
    let base_idx = vertices.len() as u32;

    let min = center - half;
    let max = center + half;

    // 6 Faces (24 sommets)
    let faces = [
        // Front (+Z)
        (Vec3::new(0.0, 0.0, 1.0), [
            (Vec3::new(min.x, min.y, max.z), [0.0, 1.0]),
            (Vec3::new(max.x, min.y, max.z), [1.0, 1.0]),
            (Vec3::new(max.x, max.y, max.z), [1.0, 0.0]),
            (Vec3::new(min.x, max.y, max.z), [0.0, 0.0]),
        ]),
        // Back (-Z)
        (Vec3::new(0.0, 0.0, -1.0), [
            (Vec3::new(max.x, min.y, min.z), [0.0, 1.0]),
            (Vec3::new(min.x, min.y, min.z), [1.0, 1.0]),
            (Vec3::new(min.x, max.y, min.z), [1.0, 0.0]),
            (Vec3::new(max.x, max.y, min.z), [0.0, 0.0]),
        ]),
        // Top (+Y)
        (Vec3::new(0.0, 1.0, 0.0), [
            (Vec3::new(min.x, max.y, max.z), [0.0, 1.0]),
            (Vec3::new(max.x, max.y, max.z), [1.0, 1.0]),
            (Vec3::new(max.x, max.y, min.z), [1.0, 0.0]),
            (Vec3::new(min.x, max.y, min.z), [0.0, 0.0]),
        ]),
        // Bottom (-Y)
        (Vec3::new(0.0, -1.0, 0.0), [
            (Vec3::new(min.x, min.y, min.z), [0.0, 1.0]),
            (Vec3::new(min.x, min.y, min.z), [1.0, 1.0]),
            (Vec3::new(max.x, min.y, max.z), [1.0, 0.0]),
            (Vec3::new(min.x, min.y, max.z), [0.0, 0.0]),
        ]),
        // Right (+X)
        (Vec3::new(1.0, 0.0, 0.0), [
            (Vec3::new(max.x, min.y, max.z), [0.0, 1.0]),
            (Vec3::new(max.x, min.y, min.z), [1.0, 1.0]),
            (Vec3::new(max.x, max.y, min.z), [1.0, 0.0]),
            (Vec3::new(max.x, max.y, max.z), [0.0, 0.0]),
        ]),
        // Left (-X)
        (Vec3::new(-1.0, 0.0, 0.0), [
            (Vec3::new(min.x, min.y, min.z), [0.0, 1.0]),
            (Vec3::new(min.x, min.y, max.z), [1.0, 1.0]),
            (Vec3::new(min.x, max.y, max.z), [1.0, 0.0]),
            (Vec3::new(min.x, max.y, min.z), [0.0, 0.0]),
        ]),
    ];

    for (i, (norm, corners)) in faces.iter().enumerate() {
        let f_base = base_idx + (i * 4) as u32;
        for (pos, uv) in corners {
            vertices.push(SkinnedVertex {
                base_pos: *pos,
                base_normal: *norm,
                uv: *uv,
                color,
                joint_indices: [joint_idx, 0],
                joint_weights: [1.0, 0.0],
            });
        }

        indices.push(f_base);
        indices.push(f_base + 1);
        indices.push(f_base + 2);
        indices.push(f_base);
        indices.push(f_base + 2);
        indices.push(f_base + 3);
    }
}
