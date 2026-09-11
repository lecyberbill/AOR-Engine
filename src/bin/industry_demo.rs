// [WFGY] Zone: SAFE | λ: 0.2 | Fallbacks: 0/None | Action: Industrial High-Level API Ergonomics Demo
use rust_moteur_3d::runtime::{EngineApp, EngineConfig, MaterialBuilder};
use rust_moteur_3d::scene::PrimitiveType;

fn main() -> Result<(), String> {
    println!("=== AOR-Engine : Démonstration Couche d'Ergonomie Industrielle ===");

    // 1. Initialisation de l'Application en 1 ligne
    let mut app = EngineApp::new(EngineConfig {
        title: "Démonstration Industrielle AOR".into(),
        width: 1280,
        height: 720,
        ..Default::default()
    })?;

    // 2. Construction Déclarative et Fluide de la Scène
    app.world()
        // Cœur d'énergie central (Verre réfractif + Glow interne)
        .spawn_mesh("Core_Crystal", PrimitiveType::Prism)
            .at([0.0, 3.0, 0.0])
            .scale([1.5, 3.0, 1.5])
            .material(
                MaterialBuilder::glass(1.52, 0.05)
                    .emission([0.0, 2.5, 4.0], 1.8)
                    .clearcoat(1.0, 0.02)
            );

    app.world()
        // Monolithe de carrosserie vernie (Car Paint PBR)
        .spawn_mesh("Vehicle_Chassis", PrimitiveType::Cube)
            .at([0.0, 1.0, -4.0])
            .scale([2.4, 0.8, 5.0])
            .material(
                MaterialBuilder::car_paint(1.0, 0.01)
                    .metallic(0.95)
                    .roughness(0.12)
            );

    app.world()
        // Sculpture organique translucide (Subsurface Scattering SSS)
        .spawn_mesh("Organic_Statue", PrimitiveType::Pyramid)
            .at([4.0, 1.5, -2.0])
            .scale_uniform(1.8)
            .material(
                MaterialBuilder::organic(0.75, 0.3)
                    .emission([0.8, 0.3, 0.1], 0.2)
            );

    // 3. Exécution d'une frame de simulation & rendu GPU
    app.render_frame(1.0 / 60.0);

    println!("✓ Scène photoréaliste instanciée et rendue sur GPU avec succès !");
    Ok(())
}
