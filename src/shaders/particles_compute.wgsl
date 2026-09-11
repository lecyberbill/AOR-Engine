// [WFGY] Zone: SAFE | λ: 0.3 | Fallbacks: 0/None | Action: GPU Particle Physics Compute Simulation Shader
struct ParticleGpu {
    position: vec4<f32>, // xyz = pos, w = scale
    velocity: vec4<f32>, // xyz = vel, w = life
    color: vec4<f32>,    // rgba
    extra: vec4<f32>,    // x = rotation, y = rot_speed, z = kind, w = max_life
};

struct ParticleParamsUniform {
    delta_time: f32,
    gravity: f32,
    drag: f32,
    bounce: f32,
    particle_count: u32,
    ground_height: f32,
    _pad0: u32,
    _pad1: u32,
};

@group(0) @binding(0)
var<uniform> params: ParticleParamsUniform;

@group(0) @binding(1)
var<storage, read> particles_in: array<ParticleGpu>;

@group(0) @binding(2)
var<storage, read_write> particles_out: array<ParticleGpu>;

@compute @workgroup_size(64, 1, 1)
fn cs_main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let index = global_id.x;
    if (index >= params.particle_count) {
        return;
    }

    var p = particles_in[index];
    let dt = params.delta_time;

    // Check lifetime
    var life = p.velocity.w - dt;
    if (life <= 0.0) {
        // Expired particle
        p.position.w = 0.0;
        p.velocity.w = 0.0;
        particles_out[index] = p;
        return;
    }

    var pos = p.position.xyz;
    var vel = p.velocity.xyz;
    let max_life = p.extra.w;

    // Apply gravity
    vel.y = vel.y - params.gravity * dt;

    // Apply drag / air resistance
    vel = vel * (1.0 - params.drag * dt);

    // Integrate position
    pos = pos + vel * dt;

    // Ground collision & bounce (y = ground_height)
    if (pos.y <= params.ground_height) {
        pos.y = params.ground_height;
        vel.y = -vel.y * params.bounce;
        vel.x = vel.x * 0.85; // Ground friction
        vel.z = vel.z * 0.85;
    }

    // Update rotation
    var rot = p.extra.x + p.extra.y * dt;

    // Fade color alpha over lifetime
    let life_ratio = clamp(life / max(max_life, 0.001), 0.0, 1.0);
    p.color.a = p.color.a * life_ratio;

    // Write back
    p.position = vec4<f32>(pos, p.position.w);
    p.velocity = vec4<f32>(vel, life);
    p.extra.x = rot;

    particles_out[index] = p;
}
