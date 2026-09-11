// [WFGY] Zone: SAFE | λ: 0.25 | Fallbacks: 0 | Action: Blit shader with Edge-Preserving Bilateral Denoiser, Exposure and ACES Tone Mapping

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@vertex
fn vs_main(@builtin(vertex_index) in_vertex_index: u32) -> VertexOutput {
    var out: VertexOutput;
    var x = -1.0;
    var y = -1.0;
    if in_vertex_index == 1u {
        x = 3.0;
    } else if in_vertex_index == 2u {
        y = 3.0;
    }
    out.position = vec4<f32>(x, y, 0.0, 1.0);
    out.uv = vec2<f32>((x + 1.0) * 0.5, (1.0 - y) * 0.5);
    return out;
}

struct BlitUniform {
    resolution: vec2<u32>,
    enable_denoise: u32,
    denoise_radius: u32,
    exposure: f32,
    sharpness: f32,
    _pad0: f32,
    _pad1: f32,
};

@group(0) @binding(0) var<uniform> config: BlitUniform;
@group(0) @binding(1) var<storage, read> accum_buffer: array<vec4<f32>>;

// ACES Filmic Tone Mapping Curve (Narkowicz 2015)
fn aces_filmic(x: vec3<f32>) -> vec3<f32> {
    let a = 2.51;
    let b = 0.03;
    let c = 2.43;
    let d = 0.59;
    let e = 0.14;
    return clamp((x * (a * x + b)) / (x * (c * x + d) + e), vec3<f32>(0.0), vec3<f32>(1.0));
}

fn get_pixel_color(x: i32, y: i32) -> vec3<f32> {
    let px = clamp(u32(x), 0u, config.resolution.x - 1u);
    let py = clamp(u32(y), 0u, config.resolution.y - 1u);
    let idx = py * config.resolution.x + px;
    return accum_buffer[idx].rgb;
}

// Filtre bilatéral préservant les contours (Edge-Preserving Bilateral Denoiser)
fn denoise_filter(cx: i32, cy: i32) -> vec3<f32> {
    let center_color = get_pixel_color(cx, cy);
    let center_lum = dot(center_color, vec3<f32>(0.2126, 0.7152, 0.0722));

    var total_weight: f32 = 0.0;
    var filtered_color: vec3<f32> = vec3<f32>(0.0);

    let r = i32(clamp(config.denoise_radius, 1u, 3u));
    let sigma_spatial = f32(r) * 1.0;
    let sigma_color = 0.18;

    for (var dy: i32 = -r; dy <= r; dy += 1) {
        for (var dx: i32 = -r; dx <= r; dx += 1) {
            let sample_col = get_pixel_color(cx + dx, cy + dy);
            let sample_lum = dot(sample_col, vec3<f32>(0.2126, 0.7152, 0.0722));

            let d_spatial_sq = f32(dx * dx + dy * dy);
            let d_color = sample_lum - center_lum;
            let d_color_sq = d_color * d_color;

            // Poids spatial gaussien * Poids de préservation des arêtes/luminance
            let w_spatial = exp(-d_spatial_sq / (2.0 * sigma_spatial * sigma_spatial));
            let w_range = exp(-d_color_sq / (2.0 * sigma_color * sigma_color));
            let w = w_spatial * w_range;

            filtered_color += sample_col * w;
            total_weight += w;
        }
    }

    if total_weight > 0.0001 {
        return filtered_color / total_weight;
    }
    return center_color;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let px = i32(clamp(u32(in.uv.x * f32(config.resolution.x)), 0u, config.resolution.x - 1u));
    let py = i32(clamp(u32(in.uv.y * f32(config.resolution.y)), 0u, config.resolution.y - 1u));

    var hdr_color: vec3<f32>;
    let idx = u32(py) * config.resolution.x + u32(px);
    let raw_color = accum_buffer[idx].rgb;

    if config.enable_denoise == 1u {
        hdr_color = denoise_filter(px, py);
    } else {
        hdr_color = raw_color;
    }

    // Filtre d'accentuation de la netteté (Unsharp Mask adaptatif)
    if config.sharpness > 0.01 {
        let north = get_pixel_color(px, py - 1);
        let south = get_pixel_color(px, py + 1);
        let east  = get_pixel_color(px + 1, py);
        let west  = get_pixel_color(px - 1, py);
        let laplacian = raw_color * 4.0 - (north + south + east + west);
        hdr_color = clamp(hdr_color + laplacian * (config.sharpness * 0.45), vec3<f32>(0.0), vec3<f32>(100.0));
    }

    // Calibrage de l'exposition
    let exp_factor = max(config.exposure, 0.01);
    let exposed_color = hdr_color * exp_factor;

    // Tone Mapping ACES
    let ldr_color = aces_filmic(exposed_color);

    // Correction Gamma sRGB
    let gamma_color = pow(ldr_color, vec3<f32>(1.0 / 2.2));

    return vec4<f32>(gamma_color, 1.0);
}
