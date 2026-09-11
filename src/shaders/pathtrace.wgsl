// [WFGY] Zone: SAFE | λ: 0.3 | Fallbacks: 0 | Action: GPU Path Tracing Compute Shader with Robust BVH and Physical Lighting

struct PathTraceUniform {
    camera_pos: vec4<f32>,            // xyz: pos, w: frame_count
    inv_view_proj: mat4x4<f32>,       // 64 octets
    sun_dir: vec4<f32>,               // xyz: sun_dir, w: sky_intensity
    sun_color: vec4<f32>,             // xyz: color, w: sun_intensity
    resolution: vec2<u32>,            // width, height
    bounces: u32,                     // max ray bounces
    samples_per_pass: u32,            // SPP per compute dispatch
};

struct GpuTriangle {
    v0: vec3<f32>,
    material_id: u32,
    v1: vec3<f32>,
    _pad0: u32,
    v2: vec3<f32>,
    _pad1: u32,
    n0: vec3<f32>,
    _pad2: u32,
    n1: vec3<f32>,
    _pad3: u32,
    n2: vec3<f32>,
    _pad4: u32,
    uv0: vec2<f32>,
    uv1: vec2<f32>,
    uv2: vec2<f32>,
    _pad5: vec2<f32>,
};

struct GpuMaterial {
    albedo: vec4<f32>,
    emission: vec4<f32>,
    roughness: f32,
    metallic: f32,
    ior: f32,
    transmission: f32,
    use_texture: u32,
    _pad0: u32,
    _pad1: u32,
    _pad2: u32,
};

struct GpuBvhNode {
    aabb_min: vec3<f32>,
    left_or_first: i32,
    aabb_max: vec3<f32>,
    count: i32, // <= 0: Feuille (-count triangles), > 0: Nœud interne (count = right child)
};

@group(0) @binding(0) var<uniform> config: PathTraceUniform;
@group(0) @binding(1) var<storage, read> triangles: array<GpuTriangle>;
@group(0) @binding(2) var<storage, read> bvh_nodes: array<GpuBvhNode>;
@group(0) @binding(3) var<storage, read> materials: array<GpuMaterial>;
@group(0) @binding(4) var<storage, read_write> accum_buffer: array<vec4<f32>>;
@group(0) @binding(5) var albedo_texture: texture_2d<f32>;
@group(0) @binding(6) var texture_sampler: sampler;

// --- Générateur de Nombres Pseudo-Aléatoires (PCG Hash) ---
var<private> rng_state: u32;

fn init_rng(pixel: vec2<u32>, frame: u32) {
    rng_state = pixel.x * 1973u + pixel.y * 9277u + frame * 26699u | 1u;
}

fn rand_f32() -> f32 {
    rng_state = rng_state * 747796405u + 2891336453u;
    var word: u32 = ((rng_state >> ((rng_state >> 28u) + 4u)) ^ rng_state) * 277803737u;
    word = (word >> 22u) ^ word;
    return f32(word) / 4294967295.0;
}

fn rand_vec3_sphere() -> vec3<f32> {
    let u = rand_f32();
    let v = rand_f32();
    let theta = u * 6.28318530718;
    let phi = acos(2.0 * v - 1.0);
    let r = pow(rand_f32(), 1.0 / 3.0);
    let sin_phi = sin(phi);
    return vec3<f32>(
        r * sin_phi * cos(theta),
        r * sin_phi * sin(theta),
        r * cos(phi)
    );
}

// --- Structures Ray & Hit ---
struct Ray {
    origin: vec3<f32>,
    direction: vec3<f32>,
};

struct HitInfo {
    hit: bool,
    t: f32,
    position: vec3<f32>,
    normal: vec3<f32>,
    uv: vec2<f32>,
    material_id: u32,
};

// Intersection Rayon - Boîte AABB vectorisée avec directions inverses précalculées
fn intersect_aabb_fast(origin: vec3<f32>, inv_d: vec3<f32>, aabb_min: vec3<f32>, aabb_max: vec3<f32>, t_max: f32) -> bool {
    let t0 = (aabb_min - origin) * inv_d;
    let t1 = (aabb_max - origin) * inv_d;
    let t_near = min(t0, t1);
    let t_far = max(t0, t1);

    let tmin = max(max(t_near.x, t_near.y), t_near.z);
    let tmax = min(min(t_far.x, t_far.y), t_far.z);

    return tmax >= max(tmin, 0.001) && tmin < t_max;
}

// Intersection Rayon - Triangle (Möller–Trumbore)
fn intersect_triangle(ray: Ray, tri: GpuTriangle, t_max: f32) -> HitInfo {
    var hit: HitInfo;
    hit.hit = false;
    hit.t = t_max;

    let edge1 = tri.v1 - tri.v0;
    let edge2 = tri.v2 - tri.v0;
    let h = cross(ray.direction, edge2);
    let a = dot(edge1, h);

    if abs(a) < 0.000001 {
        return hit;
    }

    let f = 1.0 / a;
    let s = ray.origin - tri.v0;
    let u = f * dot(s, h);

    if u < 0.0 || u > 1.0 {
        return hit;
    }

    let q = cross(s, edge1);
    let v = f * dot(ray.direction, q);

    if v < 0.0 || u + v > 1.0 {
        return hit;
    }

    let t = f * dot(edge2, q);
    if t > 0.001 && t < t_max {
        hit.hit = true;
        hit.t = t;
        hit.position = ray.origin + ray.direction * t;
        let w = 1.0 - u - v;
        var n = normalize(w * tri.n0 + u * tri.n1 + v * tri.n2);
        if length(n) < 0.1 {
            n = normalize(cross(edge1, edge2));
        }
        if dot(n, ray.direction) > 0.0 {
            n = -n;
        }
        hit.normal = n;
        hit.uv = w * tri.uv0 + u * tri.uv1 + v * tri.uv2;
        hit.material_id = tri.material_id;
    }

    return hit;
}

// Traversée BVH Ultra-Rapide avec pré-calcul d'inversion vectorielle
fn trace_scene(ray: Ray) -> HitInfo {
    var closest_hit: HitInfo;
    closest_hit.hit = false;
    closest_hit.t = 1e30;

    let num_nodes = arrayLength(&bvh_nodes);
    if num_nodes == 0u {
        return closest_hit;
    }

    let eps = 0.000001;
    let inv_d = vec3<f32>(
        1.0 / select(ray.direction.x, eps, abs(ray.direction.x) < eps),
        1.0 / select(ray.direction.y, eps, abs(ray.direction.y) < eps),
        1.0 / select(ray.direction.z, eps, abs(ray.direction.z) < eps)
    );

    var stack: array<i32, 48>;
    var stack_ptr: i32 = 0;
    stack[0] = 0;
    stack_ptr = 1;

    while stack_ptr > 0 {
        stack_ptr -= 1;
        let node_idx = stack[stack_ptr];
        let node = bvh_nodes[node_idx];

        if intersect_aabb_fast(ray.origin, inv_d, node.aabb_min, node.aabb_max, closest_hit.t) {
            if node.count <= 0 {
                // Feuille : count négatif
                let first = node.left_or_first;
                let count = -node.count;
                for (var i: i32 = 0; i < count; i += 1) {
                    let tri = triangles[first + i];
                    let h = intersect_triangle(ray, tri, closest_hit.t);
                    if h.hit && h.t < closest_hit.t {
                        closest_hit = h;
                    }
                }
            } else {
                // Nœud interne : empiler les enfants gauche (left_or_first) et droit (count)
                if stack_ptr < 46 {
                    let left = node.left_or_first;
                    let right = node.count;
                    stack[stack_ptr] = right;
                    stack_ptr += 1;
                    stack[stack_ptr] = left;
                    stack_ptr += 1;
                }
            }
        }
    }

    return closest_hit;
}

// Ciel physique & Soleil avec horizon réaliste
fn sample_sky(dir: vec3<f32>) -> vec3<f32> {
    let sun_dir = normalize(config.sun_dir.xyz);
    let sun_dot = max(dot(dir, sun_dir), 0.0);
    
    var sky: vec3<f32>;
    if dir.y >= 0.0 {
        let t = dir.y;
        let horizon = vec3<f32>(0.70, 0.78, 0.90);
        let zenith = vec3<f32>(0.15, 0.35, 0.75);
        sky = mix(horizon, zenith, t) * config.sun_dir.w;
    } else {
        let t = -dir.y;
        let horizon = vec3<f32>(0.70, 0.78, 0.90);
        let ground = vec3<f32>(0.12, 0.14, 0.16);
        sky = mix(horizon, ground, min(t * 2.0, 1.0)) * config.sun_dir.w;
    }

    let sun = pow(sun_dot, 128.0) * config.sun_color.xyz * config.sun_color.w;
    return sky + sun;
}

// Tracé d'un rayon avec rebonds physiques (Path Tracing Monte Carlo)
fn trace_path(start_ray: Ray, max_bounces: u32) -> vec3<f32> {
    var ray = start_ray;
    var throughput = vec3<f32>(1.0, 1.0, 1.0);
    var accumulated_light = vec3<f32>(0.0, 0.0, 0.0);

    for (var bounce: u32 = 0u; bounce < max_bounces; bounce += 1u) {
        let hit = trace_scene(ray);

        if !hit.hit {
            // Touché le ciel / environnement
            accumulated_light += throughput * sample_sky(ray.direction);
            break;
        }

        let mat = materials[hit.material_id];

        // Échantillonnage de la texture diffuse/albédo uniquement si le matériau utilise une texture
        var surface_albedo = mat.albedo.xyz;
        if (mat.use_texture == 1u) {
            let tex_sample = textureSampleLevel(albedo_texture, texture_sampler, hit.uv, 0.0);
            surface_albedo = surface_albedo * tex_sample.xyz;
        }

        // 1. Émission de lumière propre (Glow)
        accumulated_light += throughput * mat.emission.xyz * mat.emission.w;

        // 2. Échantillonnage Direct de la Lumière Solaire (Shadow Ray / Next Event Estimation)
        let sun_dir = normalize(config.sun_dir.xyz);
        let NdotL = dot(hit.normal, sun_dir);
        if NdotL > 0.0 {
            var shadow_ray: Ray;
            shadow_ray.origin = hit.position + hit.normal * 0.002;
            let sun_jitter = rand_vec3_sphere() * 0.04;
            shadow_ray.direction = normalize(sun_dir + sun_jitter);

            let shadow_hit = trace_scene(shadow_ray);
            if !shadow_hit.hit {
                let sun_light = config.sun_color.xyz * config.sun_color.w * NdotL;
                accumulated_light += throughput * surface_albedo * sun_light * (1.0 - mat.metallic);
            }
        }

        // 3. Choix du rebond physique : Verre (Diélectrique) vs Métal (Spéculaire) vs Diffus (Lambertien)
        let is_glass = mat.transmission > 0.01 && rand_f32() < mat.transmission;

        var next_dir: vec3<f32>;
        var next_origin: vec3<f32>;

        if is_glass {
            // Verre / Diélectrique avec Loi de Snell-Descartes et Approximation de Schlick pour Fresnel
            var n = hit.normal;
            let mat_ior = max(mat.ior, 1.01);
            var eta = 1.0 / mat_ior; // Rayon entrant de l'air (1.0) dans le matériau (mat_ior)
            var cos_theta = -dot(ray.direction, n);

            if cos_theta < 0.0 {
                // Rayon sortant du matériau vers l'air
                n = -n;
                eta = mat_ior;
                cos_theta = -dot(ray.direction, n);
            }

            let cos_i = clamp(cos_theta, 0.0, 1.0);
            let sin2_t = eta * eta * (1.0 - cos_i * cos_i);

            // Approximation de Schlick pour le coefficient de réflexion de Fresnel
            var r0 = (1.0 - mat_ior) / (1.0 + mat_ior);
            r0 = r0 * r0;
            let fresnel = r0 + (1.0 - r0) * pow(1.0 - cos_i, 5.0);

            if sin2_t > 1.0 || rand_f32() < fresnel {
                // Réflexion interne totale ou réflexion de surface de Fresnel
                let reflect_dir = reflect(ray.direction, n);
                let rough_offset = rand_vec3_sphere() * mat.roughness * 0.1;
                next_dir = normalize(reflect_dir + rough_offset);
                next_origin = hit.position + n * 0.002;
            } else {
                // Réfraction (Transmission à travers la surface)
                let refract_dir = refract(ray.direction, n, eta);
                let rough_offset = rand_vec3_sphere() * mat.roughness * 0.1;
                next_dir = normalize(refract_dir + rough_offset);
                next_origin = hit.position - n * 0.002;
            }
            throughput *= surface_albedo;
        } else {
            let is_specular = rand_f32() < mat.metallic;
            if is_specular {
                // Réflexion métallique spéculaire avec rugosité
                let reflect_dir = reflect(ray.direction, hit.normal);
                let rough_offset = rand_vec3_sphere() * mat.roughness * 0.5;
                next_dir = normalize(reflect_dir + rough_offset);
                next_origin = hit.position + hit.normal * 0.002;
                throughput *= surface_albedo;
            } else {
                // Diffusion Lambertienne cosinus
                let diffuse_dir = normalize(hit.normal + rand_vec3_sphere());
                next_dir = diffuse_dir;
                next_origin = hit.position + hit.normal * 0.002;
                throughput *= surface_albedo;
            }
        }

        ray.origin = next_origin;
        ray.direction = next_dir;

        // Roulette russe
        if bounce > 2u {
            let p = max(throughput.r, max(throughput.g, throughput.b));
            if rand_f32() > p {
                break;
            }
            throughput *= 1.0 / p;
        }
    }

    return accumulated_light;
}

@compute @workgroup_size(8, 8, 1)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let pixel = global_id.xy;
    if pixel.x >= config.resolution.x || pixel.y >= config.resolution.y {
        return;
    }

    let frame = u32(config.camera_pos.w);
    init_rng(pixel, frame);

    let res = vec2<f32>(config.resolution);
    var color_sum = vec3<f32>(0.0);

    let spp = max(config.samples_per_pass, 1u);
    for (var s: u32 = 0u; s < spp; s += 1u) {
        // Anti-aliasing subpixel jitter
        let jitter = vec2<f32>(rand_f32() - 0.5, rand_f32() - 0.5);
        let uv = (vec2<f32>(pixel) + 0.5 + jitter) / res;
        let ndc = vec2<f32>(uv.x * 2.0 - 1.0, 1.0 - uv.y * 2.0);

        // Ray generation depuis la caméra
        let near_clip = config.inv_view_proj * vec4<f32>(ndc, 0.0, 1.0);
        let far_clip = config.inv_view_proj * vec4<f32>(ndc, 1.0, 1.0);
        let ray_origin = near_clip.xyz / near_clip.w;
        let ray_target = far_clip.xyz / far_clip.w;
        let ray_dir = normalize(ray_target - ray_origin);

        var ray: Ray;
        ray.origin = ray_origin;
        ray.direction = ray_dir;

        color_sum += trace_path(ray, config.bounces);
    }

    let new_sample = color_sum / f32(spp);
    let idx = pixel.y * config.resolution.x + pixel.x;

    // Accumulation progressive dans le Storage Buffer
    var final_color: vec4<f32>;
    if frame <= 1u {
        final_color = vec4<f32>(new_sample, 1.0);
    } else {
        let prev_color = accum_buffer[idx].rgb;
        let blend = 1.0 / f32(frame);
        let accum = mix(prev_color, new_sample, blend);
        final_color = vec4<f32>(accum, 1.0);
    }

    accum_buffer[idx] = final_color;
}
