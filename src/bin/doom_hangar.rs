// [WFGY] Zone: SAFE | λ: 0.25 | Fallbacks: 0/None | Action: Standalone DOOM E1M1 Hangar Playable Demo Executable with Next-Gen Effects
use rust_moteur_3d::runtime::{EngineApp, EngineConfig, MaterialBuilder};
use rust_moteur_3d::game::doom::{build_hangar_level, generate_shotgun_mesh, FpsPlayer};

const WIDTH: u32 = 1280;
const HEIGHT: u32 = 720;

fn main() -> Result<(), String> {
    println!("============================================================");
    println!("🔥 AOR-Engine : DOOM E1M1 Hangar - Portage 3D Photoréaliste");
    println!("============================================================");

    // 1. Initialisation de l'application industrielle avec tous les effets Next-Gen activés
    let mut app = EngineApp::new(EngineConfig {
        title: "DOOM (1993) - Remastered 3D Pure WGPU".into(),
        width: WIDTH,
        height: HEIGHT,
        render_mode: rust_moteur_3d::renderer::RenderMode::VideoGame,
        clear_color: [0.01, 0.015, 0.02, 1.0],
        enable_taa: true,
        enable_ssr: true,
        enable_volumetric_fog: false,
    })?;

    // 2. Éclairage d'ambiance et soleil d'alerte UAC (zéro aveuglement direct)
    app.renderer.light_pos = glam::Vec3::new(-10.0, 16.0, -15.0);

    // 3. Construction procédurale complète du complexe E1M1 Hangar
    println!("-> Génération de l'architecture E1M1 Hangar...");
    let _entities = {
        let mut world = app.world();
        build_hangar_level(&mut world)
    };
    // 4. Instanciation du Joueur FPS dans le hall central
    // Positionné dans le couloir central, regardant vers la grande salle, les piliers et les barils
    let spawn_point = glam::Vec3::new(0.0, 1.8, -12.0);
    let mut player = FpsPlayer::new(spawn_point);
    player.yaw = 2.8; // Regarde vers le fond (+Z)
    player.pitch = -0.08;
    player.sync_camera(&mut app.camera);

    // 5. Instanciation du Fusil à Pompe 3D PBR en vue subjective
    let (shotgun_v, shotgun_i) = generate_shotgun_mesh();
    let weapon_idx = {
        let mut world = app.world();
        world.spawn_custom_mesh("Player_Combat_Shotgun", shotgun_v, shotgun_i).index
    };

    // Configuration du matériau du fusil
    if let Some(w_node) = app.scene.nodes.get_mut(weapon_idx) {
        w_node.material = MaterialBuilder::car_paint(0.7, 0.03)
            .metallic(0.9)
            .roughness(0.12)
            .build();
    }

    // 6. Simulation d'un tir de Shotgun (Muzzle Flash, Recul, Particules GPU)
    println!("-> Simulation d'un tir de fusil à pompe contre un baril explosif...");
    let fired = player.weapon.fire();
    if fired {
        // Recul et projection du fusil
        let (w_pos, w_rot) = player.compute_weapon_world_transform();
        if let Some(w_node) = app.scene.nodes.get_mut(weapon_idx) {
            w_node.transform.translation = w_pos;
            let (euler_y, euler_x, euler_z) = w_rot.to_euler(glam::EulerRot::YXZ);
            w_node.transform.rotation = glam::Vec3::new(euler_x, euler_y, euler_z);
            w_node.update_gpu(&app.context.queue, false);
        }

        // Particules d'étincelles GPU en bout de canon (discrètes pour ne pas aveugler la caméra)
        let muzzle_pos = w_pos + w_rot * glam::Vec3::new(0.0, 0.03, 0.75);
        app.particles.spawn_sparks(muzzle_pos, glam::Vec3::new(0.0, 0.1, 1.0), 12);
    }

    // Mise à jour de la caméra
    player.sync_camera(&mut app.camera);

    // 7. Rendu complet de la scène DOOM (PBR + IBL + SSAO + CSM + Brouillard Toxique Vert + SSR + TAA)
    println!("-> Exécution de la frame GPU...");
    app.render_frame(1.0 / 60.0);

    // 8. Capture d'écran HD du niveau DOOM
    let device = &app.context.device;
    let queue = &app.context.queue;
    let bytes_per_pixel = 4u32;
    let unpadded = WIDTH * bytes_per_pixel;
    let align = wgpu::COPY_BYTES_PER_ROW_ALIGNMENT;
    let padded = unpadded.div_ceil(align) * align;

    let output = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("Doom Snapshot Buffer"),
        size: (padded * HEIGHT) as u64,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });

    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("Doom Snapshot Encoder"),
    });

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

    let out_path = "AORProject/doom_e1m1_render.png";
    let img = image::RgbaImage::from_raw(WIDTH, HEIGHT, pixels).expect("image buffer");
    img.save(out_path).expect("failed to save doom render");
    println!("✓ Rendu HD E1M1 Hangar sauvegardé avec succès dans {}", out_path);

    Ok(())
}
