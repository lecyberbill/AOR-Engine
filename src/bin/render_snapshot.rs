// [WFGY] Zone: SAFE | λ: 0.25 | Fallbacks: 0/None | Action: Standalone Cyber City & Ecosystem Render Snapshot Probe
use glam::Vec3;
use rust_moteur_3d::gpu::{GpuContext, RenderTargetTexture};
use rust_moteur_3d::renderer::{ForwardRenderer, RenderMode};
use rust_moteur_3d::scene::{Camera, Scene};
use rust_moteur_3d::game::{generate_cyber_megalopolis, generate_cyber_spinner_mesh};

const WIDTH: u32 = 1280;
const HEIGHT: u32 = 720;

fn main() {
    println!("=== AOR 3D Engine — Snapshot de Rendu PBR + IBL + SSAO + CSM ===");

    let context = match GpuContext::new_standalone() {
        Ok(c) => c,
        Err(e) => {
            eprintln!("GPU non disponible: {e}");
            std::process::exit(1);
        }
    };
    println!("GPU: {} ({:?})", context.adapter_info.name, context.adapter_info.backend);

    let device = context.device.clone();
    let queue = context.queue.clone();
    let format = wgpu::TextureFormat::Rgba8UnormSrgb;

    // 1. Création de la scène Cyber City avec spinner et eau
    let mut scene = Scene::new(&device, &queue);
    generate_cyber_megalopolis(&mut scene, &device);

    let (v_spin, i_spin) = generate_cyber_spinner_mesh();
    let s_idx = scene.nodes.len();
    scene.add_custom_mesh(&device, "Cyber_Spinner_Vehicle", v_spin, i_spin);
    if let Some(node) = scene.nodes.get_mut(s_idx) {
        node.transform.translation = Vec3::new(0.0, 3.5, -4.0);
        node.transform.scale = Vec3::splat(1.5);
        node.material = rust_moteur_3d::scene::MaterialUniform {
            roughness: 0.15,
            metallic: 0.9,
            uv_tiling: [1.0, 1.0],
            uv_offset: [0.0, 0.0],
            ior: 1.5,
            transmission: 0.0,
            emission_color: [0.2, 2.0, 3.5, 1.0],
            use_normal_map: 0,
            _pad1: 0, _pad2: 0, _pad3: 0,
        };
        node.update_gpu(&queue, false);
    }

    let mut renderer = ForwardRenderer::new(&context, format, &scene.model_bind_group_layout);

    // 2. Caméra cinématographique
    let camera = Camera::new(Vec3::new(0.0, 4.0, 0.0), 16.0, 0.55, 0.28);
    let rt = RenderTargetTexture::new(&device, WIDTH, HEIGHT as u32, format, Some("Scene Snapshot RT"));

    renderer.render(
        &context,
        &rt.color_view,
        &rt.depth_texture.view,
        &scene,
        &camera,
        WIDTH as f32 / HEIGHT as f32,
        wgpu::Color { r: 0.03, g: 0.02, b: 0.06, a: 1.0 },
        1.5,
        WIDTH as f32,
        HEIGHT as f32,
        RenderMode::VideoGame,
        None,
    );

    // Résolution TAA multi-trames pour convergence temporelle
    renderer.resolve_taa(
        &context,
        &rt.color_view,
        &rt.depth_texture.view,
        &camera,
        WIDTH as f32 / HEIGHT as f32,
        WIDTH,
        HEIGHT as u32,
        format,
    );

    // 3. Readback et sauvegarde PNG depuis la texture TAA résolue
    let bytes_per_pixel = 4u32;
    let unpadded = WIDTH * bytes_per_pixel;
    let align = wgpu::COPY_BYTES_PER_ROW_ALIGNMENT;
    let padded = unpadded.div_ceil(align) * align;

    let output = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("Snapshot Readback Buffer"),
        size: (padded * HEIGHT as u32) as u64,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });

    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("Snapshot Readback Encoder"),
    });
    encoder.copy_texture_to_buffer(
        wgpu::ImageCopyTexture {
            texture: &renderer.taa.output_texture,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        wgpu::ImageCopyBuffer {
            buffer: &output,
            layout: wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(padded),
                rows_per_image: Some(HEIGHT as u32),
            },
        },
        wgpu::Extent3d {
            width: WIDTH,
            height: HEIGHT as u32,
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
    let mut pixels = Vec::with_capacity((WIDTH * HEIGHT as u32 * 4) as usize);
    for row in 0..HEIGHT as u32 {
        let start = (row * padded) as usize;
        pixels.extend_from_slice(&data[start..start + unpadded as usize]);
    }
    drop(data);
    output.unmap();

    let out_path = "AORProject/engine_render_snapshot.png";
    let img = image::RgbaImage::from_raw(WIDTH, HEIGHT as u32, pixels).expect("image buffer");
    img.save(out_path).expect("failed to save snapshot png");
    println!("✓ Snapshot HD enregistré dans {}", out_path);
}
