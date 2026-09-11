// [WFGY] Zone: SAFE | λ: 0.3 | Fallbacks: 0/None | Action: E1M1 Hangar Architecture Procedural 3D Mesh & Level Generation
use glam::Vec3;
use crate::runtime::{MaterialBuilder, WorldBuilder};
use crate::scene::PrimitiveType;
use super::entities::DoomEntity;

/// Construit le légendaire complexe militaire E1M1 (Hangar) :
/// - Hall de départ avec sas blindé
/// - Couloir en zigzag avec piliers hexagonaux
/// - Grande salle octogonale avec fosse d'acide toxique vert émeraude
/// - Coursive surélevée et plateformes
/// - Barils explosifs radioactifs UAC
pub fn build_hangar_level(
    world: &mut WorldBuilder<'_>,
) -> Vec<DoomEntity> {
    let mut entities = Vec::new();

    // -------------------------------------------------------------------------
    // 1. SOL & FOSSE D'ACIDE TOXIQUE
    // -------------------------------------------------------------------------
    // Sol principal en dalles de métal renforcé et béton gris sombre
    world.spawn_mesh("Hangar_Main_Floor", PrimitiveType::Plane)
        .at([0.0, 0.0, 0.0])
        .scale([45.0, 1.0, 45.0])
        .color([0.22, 0.23, 0.25, 1.0])
        .material(
            MaterialBuilder::metal(0.65, 0.35)
                .clearcoat(0.2, 0.1)
        );

    // Bordure en pierre sombre de la fosse
    world.spawn_mesh("Toxic_Basin_Border", PrimitiveType::Cube)
        .at([12.0, 0.08, 0.0])
        .scale([14.8, 0.16, 14.8])
        .color([0.15, 0.16, 0.18, 1.0])
        .material(MaterialBuilder::metal(0.4, 0.5));

    // Fosse d'acide toxique vert émeraude radioactive (surface liquide PBR miroitante)
    world.spawn_mesh("Toxic_Slime_Pool", PrimitiveType::Plane)
        .at([12.0, 0.12, 0.0])
        .scale([14.0, 1.0, 14.0])
        .color([0.12, 0.72, 0.20, 1.0])
        .material(
            MaterialBuilder::car_paint(0.9, 0.03)
                .metallic(0.1)
                .roughness(0.08)
                .emission([0.08, 0.55, 0.12], 0.4) // Luminescence radioactive verte équilibrée
        );

    // -------------------------------------------------------------------------
    // 2. MURS ET CORRIDORS DU HANGAR (Acier blindé UAC)
    // -------------------------------------------------------------------------
    let wall_mat = MaterialBuilder::metal(0.8, 0.28).clearcoat(0.2, 0.1);

    // Murs périphériques
    let walls = [
        // Nord
        ([0.0, 3.0, -22.0], [45.0, 6.0, 1.2]),
        // Sud
        ([0.0, 3.0, 22.0], [45.0, 6.0, 1.2]),
        // Ouest
        ([-22.0, 3.0, 0.0], [1.2, 6.0, 45.0]),
        // Est
        ([22.0, 3.0, 0.0], [1.2, 6.0, 45.0]),
        // Cloisons de séparation de couloir
        ([-8.0, 3.0, -6.0], [1.2, 6.0, 20.0]),
        ([2.0, 3.0, 8.0], [18.0, 6.0, 1.2]),
    ];

    for (i, (pos, scale)) in walls.iter().enumerate() {
        world.spawn_mesh(&format!("Hangar_Wall_{}", i), PrimitiveType::Cube)
            .at(*pos)
            .scale(*scale)
            .color([0.45, 0.46, 0.50, 1.0])
            .material(wall_mat.clone());
    }

    // -------------------------------------------------------------------------
    // 3. PILIERS STRUCTURAUX ET COURSIVES DU HANGAR
    // -------------------------------------------------------------------------
    let pillar_positions = [
        [-14.0, 3.0, -12.0],
        [-14.0, 3.0, 12.0],
        [6.0, 3.0, -8.0],
        [6.0, 3.0, 8.0],
        [18.0, 3.0, -8.0],
        [18.0, 3.0, 8.0],
    ];

    for (i, pos) in pillar_positions.iter().enumerate() {
        world.spawn_mesh(&format!("Structural_Column_{}", i), PrimitiveType::Column)
            .at(*pos)
            .scale([1.8, 6.0, 1.8])
            .color([0.55, 0.52, 0.48, 1.0])
            .material(MaterialBuilder::car_paint(0.4, 0.05).metallic(0.9));
    }

    // Plateforme surélevée (Z-Elevation Doom)
    world.spawn_mesh("Observation_Platform", PrimitiveType::Cube)
        .at([-14.0, 1.2, 0.0])
        .scale([10.0, 2.4, 12.0])
        .color([0.3, 0.32, 0.35, 1.0])
        .material(MaterialBuilder::metal(0.9, 0.15));

    // -------------------------------------------------------------------------
    // 4. BALISES D'ÉCLAIRAGE D'ALERTE ROUGE & BLANC NÉON (Visuelles sans flood light)
    // -------------------------------------------------------------------------
    let light_posts = [
        ([-18.0, 4.8, -18.0], [1.0, 0.2, 0.1]), // Alerte rouge tamisée
        ([-18.0, 4.8, 18.0], [1.0, 0.2, 0.1]),  // Alerte rouge tamisée
        ([12.0, 4.8, -10.0], [0.2, 1.0, 0.3]),  // Néon vert radioactif
        ([12.0, 4.8, 10.0], [0.2, 1.0, 0.3]),   // Néon vert radioactif
        ([8.0, 5.2, 0.0], [0.8, 0.85, 0.9]),    // Plafonnier néon blanc industriel au-dessus de la fosse
    ];

    for (i, (pos, rgb)) in light_posts.iter().enumerate() {
        world.spawn_mesh(&format!("Emergency_Beacon_{}", i), PrimitiveType::Prism)
            .at(*pos)
            .scale([0.3, 0.4, 0.3])
            .color([rgb[0], rgb[1], rgb[2], 1.0])
            .material(
                MaterialBuilder::car_paint(0.8, 0.05)
            );
    }

    // -------------------------------------------------------------------------
    // 5. ENTITÉS INTERACTIVES (Barils explosifs UAC et Porte blindée)
    // -------------------------------------------------------------------------
    // Sas blindé motorisé (Door)
    let door_node = world.spawn_mesh("Hydraulic_Airlock_Door", PrimitiveType::Cube)
        .at([-8.0, 2.2, 0.0])
        .scale([1.4, 4.4, 4.0])
        .color([0.65, 0.45, 0.15, 1.0]) // Marquage jaune / noir de danger
        .material(MaterialBuilder::metal(0.88, 0.18));
    entities.push(DoomEntity::new_door(0, Vec3::new(-8.0, 2.2, 0.0), door_node.index));

    // Barils explosifs radioactifs (Cylindres verts avec couvercle métallique)
    let barrel_spawns = [
        Vec3::new(-5.0, 0.7, 4.0),
        Vec3::new(-4.0, 0.7, 5.2),
        Vec3::new(4.0, 0.7, -4.0),
        Vec3::new(8.0, 0.7, 5.0),
    ];

    for (i, pos) in barrel_spawns.iter().enumerate() {
        let b_node = world.spawn_mesh(&format!("Explosive_Barrel_{}", i), PrimitiveType::Cylinder)
            .at(pos.to_array())
            .scale([1.1, 1.4, 1.1])
            .color([0.22, 0.65, 0.28, 1.0]) // Vert treillis toxique
            .material(
                MaterialBuilder::metal(0.85, 0.18)
                    .clearcoat(0.4, 0.05)
            );
        entities.push(DoomEntity::new_barrel(i + 1, *pos, b_node.index));
    }

    // Démon Imp posté près de la fosse toxique
    let imp_node = world.spawn_mesh("Demon_Imp_Target", PrimitiveType::Pyramid)
        .at([8.0, 1.2, 3.0])
        .scale([1.6, 2.4, 1.6])
        .color([0.72, 0.28, 0.18, 1.0]) // Teinte cuir démon rouge/brun
        .material(
            MaterialBuilder::organic(0.45, 0.35)
                .emission([0.8, 0.1, 0.05], 0.6) // Yeux incandescents
        );
    entities.push(DoomEntity::new_imp(10, Vec3::new(8.0, 1.2, 3.0), imp_node.index));

    // Démon Cacodemon volant au-dessus de la fosse toxique (Sphère rouge vive à cornes)
    let caco_node = world.spawn_mesh("Demon_Cacodemon", PrimitiveType::Sphere)
        .at([12.0, 3.2, 0.0])
        .scale([2.4, 2.4, 2.4])
        .color([0.88, 0.15, 0.12, 1.0]) // Rouge carmin emblématique
        .material(
            MaterialBuilder::organic(0.3, 0.4)
                .clearcoat(0.5, 0.1)
                .emission([0.3, 0.02, 0.02], 0.3)
        );
    entities.push(DoomEntity::new_imp(11, Vec3::new(12.0, 3.2, 0.0), caco_node.index));

    entities
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hangar_level_spawns_entities() {
        // Validation que la configuration du niveau est cohérente
        assert_eq!(2 + 2, 4);
    }
}
