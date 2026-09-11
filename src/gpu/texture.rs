// [WFGY] Zone: SAFE | λ: 0.25 | Fallbacks: 0/None | Action: GPU Texture, DepthTexture, RenderTarget and Procedural Material Generation
use std::path::Path;

pub struct DepthTexture {
    pub texture: wgpu::Texture,
    pub view: wgpu::TextureView,
    pub sampler: wgpu::Sampler,
}

impl DepthTexture {
    pub const DEPTH_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;

    pub fn create_depth_texture(device: &wgpu::Device, width: u32, height: u32, label: &str) -> Self {
        let size = wgpu::Extent3d {
            width: width.max(1),
            height: height.max(1),
            depth_or_array_layers: 1,
        };

        let desc = wgpu::TextureDescriptor {
            label: Some(label),
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: Self::DEPTH_FORMAT,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        };
        let texture = device.create_texture(&desc);

        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Nearest,
            compare: Some(wgpu::CompareFunction::LessEqual),
            lod_min_clamp: 0.0,
            lod_max_clamp: 100.0,
            ..Default::default()
        });

        DepthTexture {
            texture,
            view,
            sampler,
        }
    }

    pub fn create_depth_texture_array(
        device: &wgpu::Device,
        width: u32,
        height: u32,
        layers: u32,
        label: &str,
    ) -> (Self, Vec<wgpu::TextureView>) {
        let size = wgpu::Extent3d {
            width: width.max(1),
            height: height.max(1),
            depth_or_array_layers: layers.max(1),
        };

        let desc = wgpu::TextureDescriptor {
            label: Some(label),
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: Self::DEPTH_FORMAT,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        };
        let texture = device.create_texture(&desc);

        // Vue complète en Texture 2D Array pour le sampling dans les shaders
        let array_view = texture.create_view(&wgpu::TextureViewDescriptor {
            label: Some(&format!("{label} Array View")),
            dimension: Some(wgpu::TextureViewDimension::D2Array),
            ..Default::default()
        });

        // Vues individuelles par couche (layer) pour les render passes
        let mut layer_views = Vec::with_capacity(layers as usize);
        for layer in 0..layers {
            let layer_view = texture.create_view(&wgpu::TextureViewDescriptor {
                label: Some(&format!("{label} Layer {layer} View")),
                dimension: Some(wgpu::TextureViewDimension::D2),
                base_array_layer: layer,
                array_layer_count: Some(1),
                ..Default::default()
            });
            layer_views.push(layer_view);
        }

        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Nearest,
            compare: Some(wgpu::CompareFunction::LessEqual),
            lod_min_clamp: 0.0,
            lod_max_clamp: 100.0,
            ..Default::default()
        });

        (
            DepthTexture {
                texture,
                view: array_view,
                sampler,
            },
            layer_views,
        )
    }
}

pub struct RenderTargetTexture {
    pub texture: wgpu::Texture,
    pub color_view: wgpu::TextureView,
    pub depth_texture: DepthTexture,
    pub width: u32,
    pub height: u32,
    pub format: wgpu::TextureFormat,
}

impl RenderTargetTexture {
    pub fn new(
        device: &wgpu::Device,
        width: u32,
        height: u32,
        format: wgpu::TextureFormat,
        label: Option<&str>,
    ) -> Self {
        let w = width.max(1);
        let h = height.max(1);

        let size = wgpu::Extent3d {
            width: w,
            height: h,
            depth_or_array_layers: 1,
        };

        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label,
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });

        let color_view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let depth_texture = DepthTexture::create_depth_texture(
            device,
            w,
            h,
            &format!("{} Depth", label.unwrap_or("RenderTarget")),
        );

        RenderTargetTexture {
            texture,
            color_view,
            depth_texture,
            width: w,
            height: h,
            format,
        }
    }

    pub fn resize(&mut self, device: &wgpu::Device, width: u32, height: u32) {
        let w = width.max(1);
        let h = height.max(1);

        if self.width == w && self.height == h {
            return;
        }

        *self = Self::new(device, w, h, self.format, Some("Resized RenderTarget"));
    }

    pub fn aspect_ratio(&self) -> f32 {
        self.width as f32 / self.height.max(1) as f32
    }
}

pub struct GpuTexture {
    pub texture: wgpu::Texture,
    pub view: wgpu::TextureView,
    pub sampler: wgpu::Sampler,
    pub width: u32,
    pub height: u32,
    pub mip_levels: u32,
    pub vram_bytes: usize,
}

impl GpuTexture {
    /// Calcule le nombre optimal de niveaux de Mipmaps pour une résolution donnée
    pub fn calculate_mip_levels(width: u32, height: u32) -> u32 {
        let max_dim = width.max(height).max(1);
        ((max_dim as f32).log2().floor() as u32 + 1).max(1)
    }

    /// Crée une texture GPU à partir d'octets RGBA8 bruts avec chaîne de Mipmaps automatique et filtrage Anisotrope 16x
    pub fn from_rgba8(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        rgba: &[u8],
        width: u32,
        height: u32,
        label: Option<&str>,
    ) -> Self {
        let width = width.max(1);
        let height = height.max(1);
        let mip_levels = Self::calculate_mip_levels(width, height);

        let size = wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        };

        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label,
            size,
            mip_level_count: mip_levels,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });

        // Téléversement du niveau de base Mip 0
        queue.write_texture(
            wgpu::ImageCopyTexture {
                texture: &texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            rgba,
            wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(4 * width),
                rows_per_image: Some(height),
            },
            size,
        );

        let mut total_vram_bytes = (width * height * 4) as usize;

        // Génération de la pyramide de Mipmaps (Mip 1 .. N) pour le filtrage trilinéaire et la gestion du cache GPU
        if mip_levels > 1 {
            if let Some(base_img) = image::RgbaImage::from_raw(width, height, rgba.to_vec()) {
                let mut current_img = base_img;
                for level in 1..mip_levels {
                    let mip_w = (width >> level).max(1);
                    let mip_h = (height >> level).max(1);
                    let mip_size = wgpu::Extent3d {
                        width: mip_w,
                        height: mip_h,
                        depth_or_array_layers: 1,
                    };

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
                            origin: wgpu::Origin3d::ZERO,
                            aspect: wgpu::TextureAspect::All,
                        },
                        downscaled.as_raw(),
                        wgpu::ImageDataLayout {
                            offset: 0,
                            bytes_per_row: Some(4 * mip_w),
                            rows_per_image: Some(mip_h),
                        },
                        mip_size,
                    );

                    total_vram_bytes += (mip_w * mip_h * 4) as usize;
                    current_img = downscaled;
                }
            }
        }

        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());

        // Échantillonneur haute fidélité avec Filtrage Trilinéaire et Clamping Anisotrope 16x
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: label.map(|l| format!("{} Aniso16x Sampler", l)).as_deref(),
            address_mode_u: wgpu::AddressMode::Repeat,
            address_mode_v: wgpu::AddressMode::Repeat,
            address_mode_w: wgpu::AddressMode::Repeat,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Linear,
            anisotropy_clamp: 16,
            lod_min_clamp: 0.0,
            lod_max_clamp: 32.0,
            ..Default::default()
        });

        GpuTexture {
            texture,
            view,
            sampler,
            width,
            height,
            mip_levels,
            vram_bytes: total_vram_bytes,
        }
    }

    /// Charge une texture depuis un buffer d'image encodée (PNG, JPG)
    pub fn from_image_bytes(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        img_bytes: &[u8],
        label: Option<&str>,
    ) -> Result<Self, String> {
        let img = image::load_from_memory(img_bytes)
            .map_err(|e| format!("Erreur de décodage de l'image: {}", e))?;
        let rgba = img.to_rgba8();
        let (width, height) = rgba.dimensions();

        Ok(Self::from_rgba8(device, queue, &rgba, width, height, label))
    }

    /// Charge une texture depuis un fichier disque (PNG, JPG)
    pub fn from_file(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        path: &Path,
    ) -> Result<Self, String> {
        let bytes = std::fs::read(path)
            .map_err(|e| format!("Impossible de lire le fichier texture '{:?}': {}", path, e))?;
        let file_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("texture");
        Self::from_image_bytes(device, queue, &bytes, Some(file_name))
    }

    /// Texture Blanche 1x1 neutre
    pub fn create_white_texture(device: &wgpu::Device, queue: &wgpu::Queue) -> Self {
        let white_pixel: [u8; 4] = [255, 255, 255, 255];
        Self::from_rgba8(device, queue, &white_pixel, 1, 1, Some("Default White 1x1 Texture"))
    }

    /// Texture Damier UV technique
    pub fn create_checkered_texture(device: &wgpu::Device, queue: &wgpu::Queue, size: u32, tiles: u32) -> Self {
        let mut pixels = Vec::with_capacity((size * size * 4) as usize);
        let tile_size = (size / tiles).max(1);

        for y in 0..size {
            for x in 0..size {
                let tx = (x / tile_size) % 2;
                let ty = (y / tile_size) % 2;
                let is_dark = (tx ^ ty) == 0;

                if is_dark {
                    pixels.extend_from_slice(&[45, 50, 75, 255]); // Bleu foncé Tokyo Night
                } else {
                    pixels.extend_from_slice(&[190, 205, 250, 255]); // Bleu clair texturé
                }
            }
        }

        Self::from_rgba8(device, queue, &pixels, size, size, Some("Checkered UV Texture"))
    }

    /// Texture Grille Métrique
    pub fn create_grid_texture(device: &wgpu::Device, queue: &wgpu::Queue, size: u32) -> Self {
        let mut pixels = Vec::with_capacity((size * size * 4) as usize);
        let cell_size = 32;

        for y in 0..size {
            for x in 0..size {
                let on_grid = (x % cell_size == 0) || (y % cell_size == 0) || (x % cell_size == cell_size - 1) || (y % cell_size == cell_size - 1);
                if on_grid {
                    pixels.extend_from_slice(&[100, 150, 255, 255]); // Ligne bleue
                } else {
                    pixels.extend_from_slice(&[28, 32, 48, 255]); // Fond sombre
                }
            }
        }

        Self::from_rgba8(device, queue, &pixels, size, size, Some("Metric Grid Texture"))
    }

    /// Texture Procédurale Brique / Maçonnerie
    pub fn create_brick_texture(device: &wgpu::Device, queue: &wgpu::Queue, size: u32) -> Self {
        let mut pixels = Vec::with_capacity((size * size * 4) as usize);
        let brick_h = 32;
        let brick_w = 64;

        for y in 0..size {
            let row = y / brick_h;
            let offset_x = if row % 2 == 1 { brick_w / 2 } else { 0 };

            for x in 0..size {
                let rx = (x + offset_x) % brick_w;
                let ry = y % brick_h;

                let is_mortar = rx < 3 || ry < 3;
                if is_mortar {
                    pixels.extend_from_slice(&[160, 160, 160, 255]); // Joint ciment
                } else {
                    let noise = ((rx * 7 + ry * 13) % 25) as u8;
                    pixels.extend_from_slice(&[185 + noise, 75 + noise / 2, 60, 255]);
                }
            }
        }

        Self::from_rgba8(device, queue, &pixels, size, size, Some("Procedural Brick Texture"))
    }

    /// Texture Procédurale Bois / Parquet
    pub fn create_wood_texture(device: &wgpu::Device, queue: &wgpu::Queue, size: u32) -> Self {
        let mut pixels = Vec::with_capacity((size * size * 4) as usize);

        for y in 0..size {
            for x in 0..size {
                let plank_h = 64;
                let in_plank = y % plank_h;
                let is_gap = in_plank < 2;

                if is_gap {
                    pixels.extend_from_slice(&[60, 40, 20, 255]); // Rainure sombre
                } else {
                    let grain = (((x as f32 * 0.1).sin() * 15.0) + ((y as f32 * 0.05).cos() * 10.0)) as i32;
                    let r = (140 + grain).clamp(0, 255) as u8;
                    let g = (90 + grain / 2).clamp(0, 255) as u8;
                    let b = (50 + grain / 4).clamp(0, 255) as u8;
                    pixels.extend_from_slice(&[r, g, b, 255]);
                }
            }
        }

        Self::from_rgba8(device, queue, &pixels, size, size, Some("Procedural Wood Texture"))
    }

    /// Texture HD Marbre Blanc Carrare (1024x1024 ou 2048x2048 avec veinures procédurales fines)
    pub fn create_marble_hd_texture(device: &wgpu::Device, queue: &wgpu::Queue, size: u32) -> Self {
        let mut pixels = Vec::with_capacity((size * size * 4) as usize);

        for y in 0..size {
            let ny = y as f32 / size as f32;
            for x in 0..size {
                let nx = x as f32 / size as f32;

                // Turbulence multi-octave harmonique
                let turb = ((nx * 12.0 + (ny * 8.0).sin() * 2.5).sin() * 0.5 + 0.5)
                    * 0.6 + ((nx * 28.0 + ny * 35.0).cos() * 0.5 + 0.5) * 0.4;
                let vein = (turb * std::f32::consts::PI * 3.0).sin().abs();

                let base_color = 238.0 + vein * 16.0;
                let r = (base_color - (1.0 - vein) * 45.0).clamp(0.0, 255.0) as u8;
                let g = (base_color - (1.0 - vein) * 40.0).clamp(0.0, 255.0) as u8;
                let b = (base_color - (1.0 - vein) * 32.0).clamp(0.0, 255.0) as u8;

                pixels.extend_from_slice(&[r, g, b, 255]);
            }
        }

        Self::from_rgba8(device, queue, &pixels, size, size, Some("Procedural HD Marble Texture"))
    }

    /// Texture HD Fibre de Carbone / Trame Haute Fréquence (Idéal pour tester le Mipmapping et l'Anti-Aliasing Anisotrope)
    pub fn create_carbon_grid_hd_texture(device: &wgpu::Device, queue: &wgpu::Queue, size: u32) -> Self {
        let mut pixels = Vec::with_capacity((size * size * 4) as usize);
        let cell = 16;

        for y in 0..size {
            let cy = (y / cell) % 2;
            let py = y % cell;

            for x in 0..size {
                let cx = (x / cell) % 2;
                let px = x % cell;

                let is_horiz = (cx ^ cy) == 0;
                let shade = if is_horiz {
                    let grad = ((px as f32 / cell as f32) * std::f32::consts::PI).sin();
                    (35.0 + grad * 45.0) as u8
                } else {
                    let grad = ((py as f32 / cell as f32) * std::f32::consts::PI).sin();
                    (25.0 + grad * 35.0) as u8
                };

                pixels.extend_from_slice(&[shade, shade, shade + 3, 255]);
            }
        }

        Self::from_rgba8(device, queue, &pixels, size, size, Some("Procedural HD Carbon Grid Texture"))
    }

    /// Texture de Normales Plate / Neutre (Vecteur Z = 1.0)
    pub fn create_flat_normal_map(device: &wgpu::Device, queue: &wgpu::Queue) -> Self {
        let flat_pixel: [u8; 4] = [128, 128, 255, 255];
        Self::from_rgba8(device, queue, &flat_pixel, 1, 1, Some("Flat Normal Map 1x1"))
    }

    /// Texture de Normales Procédurale pour Briques
    pub fn create_brick_normal_map(device: &wgpu::Device, queue: &wgpu::Queue, size: u32) -> Self {
        let mut pixels = Vec::with_capacity((size * size * 4) as usize);
        let brick_h = 32i32;
        let brick_w = 64i32;

        for y in 0..size as i32 {
            let row = y / brick_h;
            let offset_x = if row % 2 == 1 { brick_w / 2 } else { 0 };

            for x in 0..size as i32 {
                let rx = (x + offset_x) % brick_w;
                let ry = y % brick_h;

                let mut nx = 0.0f32;
                let mut ny = 0.0f32;
                let mut nz = 1.0f32;

                if rx < 4 {
                    nx = (rx as f32 - 4.0) / 4.0;
                } else if rx > brick_w - 4 {
                    nx = (4.0 - (brick_w - rx) as f32) / 4.0;
                }

                if ry < 4 {
                    ny = (ry as f32 - 4.0) / 4.0;
                } else if ry > brick_h - 4 {
                    ny = (4.0 - (brick_h - ry) as f32) / 4.0;
                }

                let len = (nx * nx + ny * ny + nz * nz).sqrt().max(0.001);
                nx /= len; ny /= len; nz /= len;

                let r = ((nx * 0.5 + 0.5) * 255.0).clamp(0.0, 255.0) as u8;
                let g = ((ny * 0.5 + 0.5) * 255.0).clamp(0.0, 255.0) as u8;
                let b = ((nz * 0.5 + 0.5) * 255.0).clamp(0.0, 255.0) as u8;

                pixels.extend_from_slice(&[r, g, b, 255]);
            }
        }

        Self::from_rgba8(device, queue, &pixels, size, size, Some("Brick Normal Map"))
    }

    /// Texture de Normales Procédurale pour Bois
    pub fn create_wood_normal_map(device: &wgpu::Device, queue: &wgpu::Queue, size: u32) -> Self {
        let mut pixels = Vec::with_capacity((size * size * 4) as usize);

        for y in 0..size {
            for x in 0..size {
                let plank_h = 64;
                let in_plank = y % plank_h;

                let ny = if in_plank < 3 {
                    -0.7
                } else if in_plank > plank_h - 3 {
                    0.7
                } else {
                    ((y as f32 * 0.15).sin() * 0.25).clamp(-0.4, 0.4)
                };

                let nx = ((x as f32 * 0.2).cos() * 0.15).clamp(-0.3, 0.3);
                let nz = (1.0 - nx * nx - ny * ny).max(0.1).sqrt();

                let r = ((nx * 0.5 + 0.5) * 255.0).clamp(0.0, 255.0) as u8;
                let g = ((ny * 0.5 + 0.5) * 255.0).clamp(0.0, 255.0) as u8;
                let b = ((nz * 0.5 + 0.5) * 255.0).clamp(0.0, 255.0) as u8;

                pixels.extend_from_slice(&[r, g, b, 255]);
            }
        }

        Self::from_rgba8(device, queue, &pixels, size, size, Some("Wood Normal Map"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calculate_mip_levels() {
        assert_eq!(GpuTexture::calculate_mip_levels(1, 1), 1);
        assert_eq!(GpuTexture::calculate_mip_levels(256, 256), 9);
        assert_eq!(GpuTexture::calculate_mip_levels(512, 512), 10);
        assert_eq!(GpuTexture::calculate_mip_levels(1024, 1024), 11);
        assert_eq!(GpuTexture::calculate_mip_levels(2048, 2048), 12);
        assert_eq!(GpuTexture::calculate_mip_levels(4096, 4096), 13);
        assert_eq!(GpuTexture::calculate_mip_levels(4096, 2048), 13);
    }
}

