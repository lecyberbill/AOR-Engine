// [WFGY] Zone: RISK | λ: 0.3 | Fallbacks: 0/headless-skip | Action: Headless GPU probe — compile node graph, create real pipeline, render & save PNG
#![allow(dead_code)]
use glam::Vec3;
use rust_moteur_3d::gpu::{GpuContext, RenderTargetTexture};
use rust_moteur_3d::renderer::{ForwardRenderer, RenderMode};
use rust_moteur_3d::scene::{Camera, Scene};
use rust_moteur_3d::studio::{compile_engine_shader, validate_wgsl, MaterialNodeGraph, NodeKind};

const WIDTH: u32 = 960;
const HEIGHT: u32 = 640;

fn build_demo_graph() -> MaterialNodeGraph {
    let mut g = MaterialNodeGraph::new("Probe PBR");
    let uv = g.add_node(NodeKind::Uv, [0.0, 0.0]);
    let noise = g.add_node(NodeKind::PerlinNoise, [220.0, 0.0]);
    let rgb = g.add_node(NodeKind::Rgb, [220.0, 160.0]);
    let mix = g.add_node(NodeKind::Mix, [460.0, 80.0]);
    let fres = g.add_node(NodeKind::Fresnel, [460.0, 280.0]);
    let out = g.output_node_id;
    g.connect(uv, "UV", noise, "UV").expect("uv->noise");
    g.connect(noise, "Out", mix, "T").expect("noise->mix");
    g.connect(rgb, "Color", mix, "A").expect("rgb->mix");
    g.connect(mix, "Out", out, "Albedo").expect("mix->out");
    g.connect(fres, "Out", out, "Emissive").expect("fresnel->out");

    // Paramètres néon (démontre l'édition live des nœuds)
    if let Some(n) = g.node_mut(rgb) {
        n.set_param_f32("r", 0.0);
        n.set_param_f32("g", 0.85);
        n.set_param_f32("b", 1.0);
    }
    if let Some(n) = g.node_mut(noise) {
        n.set_param_f32("scale", 5.0);
    }
    if let Some(n) = g.node_mut(fres) {
        n.set_param_f32("power", 3.0);
        n.set_param_f32("bias", 0.25);
    }
    g
}

fn main() {
    println!("=== AOR Studio — Sonde GPU de pipeline matériau nodal ===");

    let context = match GpuContext::new_standalone() {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Aucun GPU disponible: {e}");
            std::process::exit(3);
        }
    };
    println!("GPU: {} ({:?})", context.adapter_info.name, context.adapter_info.backend);

    let device = context.device.clone();
    let queue = context.queue.clone();
    let format = wgpu::TextureFormat::Rgba8UnormSrgb;

    // 1. Scène PBR standard (cube au centre) + renderer forward
    let scene = Scene::new(&device, &queue);
    let mut renderer = ForwardRenderer::new(&context, format, &scene.model_bind_group_layout);

    // 2. Compilation du graphe nodal -> shader moteur
    let graph = build_demo_graph();
    let shader = match compile_engine_shader(&graph) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Compilation du graphe échouée: {e}");
            std::process::exit(2);
        }
    };
    if let Err(e) = validate_wgsl(&shader.wgsl) {
        eprintln!("Shader moteur invalide (Naga): {e}");
        std::process::exit(2);
    }
    println!(
        "Shader moteur généré: {} nœuds, {} binding(s) texture, {} octets WGSL",
        shader.node_count,
        shader.texture_bindings,
        shader.wgsl.len()
    );

    // 3. Création à chaud du pipeline WGPU réel
    if let Err(e) = renderer.build_material_pipeline(
        &device,
        &shader.wgsl,
        &scene.model_bind_group_layout,
        format,
    ) {
        eprintln!("Échec create_render_pipeline: {e}");
        std::process::exit(2);
    }
    println!("Pipeline WGPU nodal créé avec succès.");

    // 4. Rendu hors écran
    let rt = RenderTargetTexture::new(&device, WIDTH, HEIGHT, format, Some("Probe Offscreen"));
    let camera = Camera::new(Vec3::new(0.0, 0.0, 0.0), 6.5, 0.7, 0.35);

    renderer.render(
        &context,
        &rt.color_view,
        &rt.depth_texture.view,
        &scene,
        &camera,
        WIDTH as f32 / HEIGHT as f32,
        wgpu::Color { r: 0.05, g: 0.07, b: 0.12, a: 1.0 },
        1.0,
        WIDTH as f32,
        HEIGHT as f32,
        RenderMode::VideoGame,
        None,
    );

    // 5. Lecture des pixels (readback)
    let bytes_per_pixel = 4u32;
    let unpadded = WIDTH * bytes_per_pixel;
    let align = wgpu::COPY_BYTES_PER_ROW_ALIGNMENT;
    let padded = unpadded.div_ceil(align) * align;

    let output = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("Probe Readback Buffer"),
        size: (padded * HEIGHT) as u64,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });

    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("Probe Readback Encoder"),
    });
    encoder.copy_texture_to_buffer(
        wgpu::ImageCopyTexture {
            texture: &rt.texture,
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

    // 6. Statistiques : on échantillonne le fond au coin supérieur gauche puis on
    //    compte les pixels qui s'en écartent (l'objet éclairé doit apparaître).
    let bg = [pixels[0] as i32, pixels[1] as i32, pixels[2] as i32];
    let mut distinct = 0usize;
    let mut max_channel = 0u8;
    for px in pixels.chunks_exact(4) {
        let diff = (px[0] as i32 - bg[0]).abs()
            + (px[1] as i32 - bg[1]).abs()
            + (px[2] as i32 - bg[2]).abs();
        if diff > 60 {
            distinct += 1;
        }
        max_channel = max_channel.max(px[0]).max(px[1]).max(px[2]);
    }
    let coverage = distinct as f32 / (WIDTH * HEIGHT) as f32;

    // 7. Sauvegarde PNG
    let out_path = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "AORProject/material_preview.png".to_string());
    if let Some(parent) = std::path::Path::new(&out_path).parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let img = image::RgbaImage::from_raw(WIDTH, HEIGHT, pixels).expect("image buffer");
    match img.save(&out_path) {
        Ok(()) => println!("Aperçu enregistré: {}", out_path),
        Err(e) => eprintln!("Échec sauvegarde PNG: {e}"),
    }

    println!(
        "Rendu: {:.1}% de pixels d'objet, canal max = {}",
        coverage * 100.0,
        max_channel
    );

    if coverage < 0.01 || max_channel < 60 {
        eprintln!("⚠ Rendu suspect : la pipeline n'a peut-être pas dessiné le matériau.");
        std::process::exit(1);
    }
    println!("✓ Pipeline nodal exécuté avec succès sur GPU.");
}
