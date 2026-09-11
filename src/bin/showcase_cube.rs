// [WFGY] Zone: SAFE | λ: 0.2 | Fallbacks: 0/None | Action: Ultra AAA Showcase: Single Cube with all Next-Gen Effects Rendered to HD PNG
use rust_moteur_3d::runtime::{EngineApp, EngineConfig, MaterialBuilder};
use rust_moteur_3d::scene::PrimitiveType;

fn main() -> Result<(), String> {
    println!("=== AOR-Engine : Démonstrateur Ultra AAA - Cube avec Tous les Effets ===");

    const WIDTH: u32 = 1280;
    const HEIGHT: u32 = 720;

    // 1. Initialisation de l'application industrielle avec tous les effets Next-Gen activés
    let mut app = EngineApp::new(EngineConfig {
        title: "AOR Ultra Showcase".into(),
        width: WIDTH,
        height: HEIGHT,
        render_mode: rust_moteur_3d::renderer::RenderMode::VideoGame,
        clear_color: [0.015, 0.01, 0.03, 1.0],
        enable_taa: true,
        enable_ssr: true,
        enable_volumetric_fog: true,
    })?;

    // 2. Caméra cinématographique pointée sur le cube
    app.camera = rust_moteur_3d::scene::Camera::new(
        glam::Vec3::new(0.0, 2.0, 0.0), // Cible au centre du cube
        7.5,                            // Distance
        0.85,                           // Yaw (angle horizontal)
        0.40,                           // Pitch (angle vertical)
    );
    app.renderer.light_pos = glam::Vec3::new(6.0, 10.0, 6.0);

    // 3. Spawne le Cube Photorealiste combinant TOUS les effets :
    // - PBR Métallique GGX (Or / Titane iridescent)
    // - Vernis Clearcoat multicouche ultra-brillant (reflets nets du ciel IBL)
    // - Diffusion Sous-Surfacique (SSS) interne
    // - Émission HDR néon cyan interne sur les arrêtes
    // - Occlusion Ambiante SSAO sous les arêtes
    // - Ombres portées CSM 4 cascades nettes
    app.world()
        .spawn_mesh("Ultra_Masterpiece_Cube", PrimitiveType::Cube)
            .at([0.0, 2.0, 0.0])
            .scale([2.6, 2.6, 2.6])
            .rotate_deg([25.0, 42.0, 15.0])
            .color([0.98, 0.65, 0.22, 1.0]) // Teinte ambre / or brillant
            .material(
                MaterialBuilder::car_paint(1.0, 0.01) // Clearcoat 100% lisse
                    .metallic(0.92)
                    .roughness(0.08)
                    .subsurface(0.35) // SSS doux
                    .emission([0.2, 2.2, 4.0], 2.2) // Glow néon cyan intense
            );

    // 4. Ajoute un socle récepteur d'ombres douces et de reflets SSR
    app.world()
        .spawn_mesh("Showcase_Pedestal", PrimitiveType::Cylinder)
            .at([0.0, 0.1, 0.0])
            .scale([9.0, 0.25, 9.0])
            .color([0.28, 0.32, 0.42, 1.0])
            .material(
                MaterialBuilder::metal(0.85, 0.12)
                    .clearcoat(0.8, 0.02)
            );

    // 5. Ajoute des balises émissives d'ambiance projetant de la lumière ponctuelle PBR
    app.world()
        .spawn_mesh("Light_Pillar_A", PrimitiveType::Prism)
            .at([4.5, 1.0, 3.5])
            .scale([0.5, 2.0, 0.5])
            .color([0.1, 1.0, 1.0, 1.0])
            .material(
                MaterialBuilder::new()
                    .emission([0.1, 2.5, 5.0], 3.0)
            );

    app.world()
        .spawn_mesh("Light_Pillar_B", PrimitiveType::Prism)
            .at([-4.5, 1.0, -3.5])
            .scale([0.5, 2.0, 0.5])
            .color([1.0, 0.2, 0.8, 1.0])
            .material(
                MaterialBuilder::new()
                    .emission([5.0, 0.2, 2.5], 3.0)
            );

    // 6. Particules d'ambiance et étincelles GPU
    app.particles.spawn_explosion(glam::Vec3::new(0.0, 2.0, 0.0), 1.5);
    app.particles.emit_thruster_plume(
        glam::Vec3::new(0.0, 0.4, 0.0),
        glam::Vec3::Y,
        80,
    );

    // 6. Rendu complet de la scène avec passes combinées (PBR, IBL, SSAO, CSM, Fog, SSR, TAA)
    app.render_frame(1.0 / 60.0);

    // 7. Readback du buffer résolu et sauvegarde en PNG HD
    let device = &app.context.device;
    let queue = &app.context.queue;
    let bytes_per_pixel = 4u32;
    let unpadded = WIDTH * bytes_per_pixel;
    let align = wgpu::COPY_BYTES_PER_ROW_ALIGNMENT;
    let padded = unpadded.div_ceil(align) * align;

    let output = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("Showcase Readback Buffer"),
        size: (padded * HEIGHT) as u64,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });

    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("Showcase Readback Encoder"),
    });

    // On lit la texture couleur résolue
    encoder.copy_texture_to_buffer(
        wgpu::ImageCopyTexture {
            texture: &app.render_target.texture,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        wgpu::ImageCopyBuffer {
            buffer: &output,
            layout: wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(padded),
                rows_per_image: Some(HEIGHT),
            },
        },
        wgpu::Extent3d {
            width: WIDTH,
            height: HEIGHT,
            depth_or_array_layers: 1,
        },
    );
    queue.submit(std::iter::once(encoder.finish()));

    let slice = output.slice(..);
    let (tx, rx) = std::sync::mpsc::channel();
    slice.map_async(wgpu::MapMode::Read, move |r| {
        let _ = tx.send(r);
    });
    device.poll(wgpu::Maintain::Wait);
    let _ = rx.recv();

    let data = slice.get_mapped_range();
    let mut pixels = Vec::with_capacity((WIDTH * HEIGHT * 4) as usize);
    for row in 0..HEIGHT {
        let start = (row * padded) as usize;
        pixels.extend_from_slice(&data[start..start + unpadded as usize]);
    }
    drop(data);
    output.unmap();

    let out_path = "AORProject/showcase_cube_render.png";
    let img = image::RgbaImage::from_raw(WIDTH, HEIGHT, pixels).expect("image buffer");
    img.save(out_path).expect("failed to save showcase png");
    println!("✓ Snapshot Ultra AAA enregistré avec succès dans {}", out_path);

    Ok(())
}
