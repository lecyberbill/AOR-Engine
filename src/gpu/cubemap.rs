// [WFGY] Zone: SAFE | λ: 0.25 | Fallbacks: 0/None | Action: GPU Cubemap Texture & Procedural HDR Environment Generator for IBL
#![allow(dead_code)]

use std::sync::Arc;

pub struct GpuCubemap {
    pub texture: wgpu::Texture,
    pub view: wgpu::TextureView,
    pub sampler: wgpu::Sampler,
    pub size: u32,
    pub mip_levels: u32,
}

impl GpuCubemap {
    /// Crée une cubemap GPU à 6 faces avec pyramide de mipmaps complète pour l'IBL spéculaire
    pub fn create_cubemap(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        size: u32,
        faces_rgba: [&[u8]; 6],
        label: Option<&str>,
    ) -> Self {
        let size = size.max(1);
        let mip_levels = (size as f32).log2().floor() as u32 + 1;

        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label,
            size: wgpu::Extent3d {
                width: size,
                height: size,
                depth_or_array_layers: 6,
            },
            mip_level_count: mip_levels,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });

        // Téléversement du niveau de base Mip 0 pour chaque face de la cubemap
        for (layer_idx, face_data) in faces_rgba.iter().enumerate() {
            queue.write_texture(
                wgpu::ImageCopyTexture {
                    texture: &texture,
                    mip_level: 0,
                    origin: wgpu::Origin3d {
                        x: 0,
                        y: 0,
                        z: layer_idx as u32,
                    },
                    aspect: wgpu::TextureAspect::All,
                },
                face_data,
                wgpu::ImageDataLayout {
                    offset: 0,
                    bytes_per_row: Some(4 * size),
                    rows_per_image: Some(size),
                },
                wgpu::Extent3d {
                    width: size,
                    height: size,
                    depth_or_array_layers: 1,
                },
            );

            // Génération des Mipmaps de rugosité (Roughness Mips) pour chaque face
            if mip_levels > 1 {
                if let Some(base_img) = image::RgbaImage::from_raw(size, size, face_data.to_vec()) {
                    let mut current_img = base_img;
                    for level in 1..mip_levels {
                        let mip_w = (size >> level).max(1);
                        let mip_h = (size >> level).max(1);

                        let downscaled = image::imageops::resize(
                            &current_img,
                            mip_w,
                            mip_h,
                            image::imageops::FilterType::Triangle,
                        );

                        queue.write_texture(
                            wgpu::ImageCopyTexture {
                                texture: &texture,
                                mip_level: level,
                                origin: wgpu::Origin3d {
                                    x: 0,
                                    y: 0,
                                    z: layer_idx as u32,
                                },
                                aspect: wgpu::TextureAspect::All,
                            },
                            downscaled.as_raw(),
                            wgpu::ImageDataLayout {
                                offset: 0,
                                bytes_per_row: Some(4 * mip_w),
                                rows_per_image: Some(mip_h),
                            },
                            wgpu::Extent3d {
                                width: mip_w,
                                height: mip_h,
                                depth_or_array_layers: 1,
                            },
                        );

                        current_img = downscaled;
                    }
                }
            }
        }

        let view = texture.create_view(&wgpu::TextureViewDescriptor {
            label: label.map(|l| format!("{} View", l)).as_deref(),
            dimension: Some(wgpu::TextureViewDimension::Cube),
            format: Some(wgpu::TextureFormat::Rgba8UnormSrgb),
            ..Default::default()
        });

        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: label.map(|l| format!("{} Sampler", l)).as_deref(),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Linear,
            lod_min_clamp: 0.0,
            lod_max_clamp: mip_levels as f32,
            ..Default::default()
        });

        GpuCubemap {
            texture,
            view,
            sampler,
            size,
            mip_levels,
        }
    }

    /// Génère procéduralement un environnement de ciel Cyberpunk / Coucher de Soleil HDR
    /// (Ciel supérieur bleu nuit / violet, horizon doré lumineux et sol sombre)
    pub fn create_procedural_skybox(device: &wgpu::Device, queue: &wgpu::Queue, size: u32) -> Arc<Self> {
        let mut faces_data: Vec<Vec<u8>> = Vec::with_capacity(6);

        // Ordre des faces Cubemap WebGPU : +X, -X, +Y, -Y, +Z, -Z
        for face_idx in 0..6 {
            let mut pixels = Vec::with_capacity((size * size * 4) as usize);

            for y in 0..size {
                for x in 0..size {
                    // Convertir (x, y) de la face en direction 3D normalisée
                    let u = (x as f32 + 0.5) / size as f32 * 2.0 - 1.0;
                    let v = (y as f32 + 0.5) / size as f32 * 2.0 - 1.0;

                    let dir = match face_idx {
                        0 => glam::Vec3::new(1.0, -v, -u),  // +X (Right)
                        1 => glam::Vec3::new(-1.0, -v, u),  // -X (Left)
                        2 => glam::Vec3::new(u, 1.0, v),    // +Y (Top)
                        3 => glam::Vec3::new(u, -1.0, -v),  // -Y (Bottom)
                        4 => glam::Vec3::new(u, -v, 1.0),   // +Z (Front)
                        5 => glam::Vec3::new(-u, -v, -1.0), // -Z (Back)
                        _ => glam::Vec3::Y,
                    }.normalize();

                    // Calcul de la couleur atmosphérique selon l'angle d'élévation
                    let height = dir.y; // -1.0 (sol) .. +1.0 (zénith)
                    let sun_dir = glam::Vec3::new(0.6, 0.45, -0.65).normalize();
                    let sun_dot = dir.dot(sun_dir).max(0.0);

                    // Gradient de Ciel (Bleu nuit profond en haut, violet / rose à l'horizon, sol bleu acier)
                    let sky_zenith = glam::Vec3::new(0.02, 0.04, 0.12);
                    let sky_horizon = glam::Vec3::new(0.18, 0.08, 0.28);
                    let ground_color = glam::Vec3::new(0.03, 0.02, 0.05);

                    let base_color = if height > 0.0 {
                        sky_horizon.lerp(sky_zenith, height.powf(0.5))
                    } else {
                        sky_horizon.lerp(ground_color, (-height).min(1.0))
                    };

                    // Disque Solaire & Couronne atmosphérique éclatante
                    let sun_disk = sun_dot.powf(400.0) * 12.0;
                    let sun_glow = sun_dot.powf(16.0) * glam::Vec3::new(1.0, 0.65, 0.35) * 1.8;
                    let sun_total = glam::Vec3::new(1.0, 0.92, 0.8) * sun_disk + sun_glow;

                    let final_color = base_color + sun_total;

                    // Tonemapping et encodage sRGB pour la texture 8-bit
                    let r = ((final_color.x.min(1.0)) * 255.0) as u8;
                    let g = ((final_color.y.min(1.0)) * 255.0) as u8;
                    let b = ((final_color.z.min(1.0)) * 255.0) as u8;

                    pixels.extend_from_slice(&[r, g, b, 255]);
                }
            }
            faces_data.push(pixels);
        }

        let faces_refs: [&[u8]; 6] = [
            &faces_data[0],
            &faces_data[1],
            &faces_data[2],
            &faces_data[3],
            &faces_data[4],
            &faces_data[5],
        ];

        Arc::new(Self::create_cubemap(
            device,
            queue,
            size,
            faces_refs,
            Some("Procedural Atmospheric HDR Skybox"),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cubemap_mip_calculation() {
        let size = 256u32;
        let mip_levels = (size as f32).log2().floor() as u32 + 1;
        assert_eq!(mip_levels, 9);
    }
}
