# ⚡ AOR-Engine (AOR 3D Pure Graphics & Physics Engine)

> **High-performance pure Rust 3D rendering and physics engine built on WGPU / WebGPU.**  
> Delivering real-time photorealism and AAA video-game capabilities with zero external game engine dependencies.

---

## 🌟 Key Features

### 🎨 1. Photorealistic Graphics Pipeline
- **Physically-Based Rendering (PBR)**: Cook-Torrance GGX, Metallic-Roughness workflow, Fresnel-Schlick approximation, Normal mapping, Transmission & HDR Emission.
- **Image-Based Lighting (IBL)**: Atmospheric HDR Procedural Cubemap, Diffuse Irradiance, and Roughness-filtered Specular Radiance.
- **Cascaded Shadow Maps (CSM)**: 4-split view-frustum shadow maps with sub-texel stabilization and `TextureViewDimension::D2Array`.
- **Screen-Space Ambient Occlusion (SSAO)**: 16-sample hemisphere kernel with depth pre-pass and cross-bilateral blur.
- **Volumetric Fog & God Rays**: GPU raymarching with Henyey-Greenstein (Mie $g = 0.65$) phase scattering and interleaved gradient noise dithering.
- **Temporal Anti-Aliasing (TAA)**: 8-phase Halton sub-pixel jittering, motion vector reprojection, and YCoCg $3 \times 3$ neighborhood color clamping (zero ghosting).
- **Screen-Space Reflections (SSR)**: Screen-space depth raymarching with edge fading and distance falloff.
- **Hardware Instancing**: High-density mesh rendering with GPU instanced buffers.
- **Compute Path Tracer**: Standalone GPU Monte-Carlo path tracer with hierarchical BVH for reference rendering and global illumination.

### 💥 2. GPU Compute & Physics Simulation
- **GPU Particle System**: Ping-pong compute simulation (`particles_compute.wgsl`) for 100,000+ particles with gravity, aerodynamic drag, and ground plane restitution.
- **Physics Engine**: Rigid body impulse integration, ray-triangle & capsule-mesh collision detection, and responsive character controllers.

### 📦 3. Industrial Data Container (`.aor`)
- Hybrid 16-byte POD binary + JSON manifest container with in-place mutation and deterministic SHA-256 integrity verification.

---

## 🏗️ Architecture

```
AOR-Engine/
├── src/
│   ├── gpu/          # WGPU device abstractions, buffers, pipelines, textures, cubemaps
│   ├── renderer/     # PBR ForwardRenderer, CSM, SSAO, SSR, TAA, Volumetric Fog, PathTracer
│   ├── shaders/      # WGSL shaders (PBR, Skybox, Shadows, SSAO, SSR, TAA, Fog, Compute)
│   ├── particles/    # GPU billboard & compute particle systems
│   ├── scene/        # Scene graph, BVH, Camera, Culling, Bundles, Persistence (.aor)
│   ├── physics/      # Rigid body simulation, collision shapes, character controllers
│   ├── studio/       # Visual node compiler, level tools, virtual filesystem & packager
│   ├── scripting/    # Rhai engine & custom DSL integration
│   └── bin/          # Standalone probes and offline render snapshot tools
```

---

## 🚀 Getting Started

### Prerequisites
- [Rust](https://www.rust-lang.org/) (2021 edition or newer)
- Vulkan / DirectX 12 / Metal capable GPU

### Running the Test Suite
```bash
cargo test --lib
```

### Capturing an Offline GPU Render Snapshot
```bash
cargo run --bin render_snapshot
```

---

## 📜 License
Licensed under MIT or Apache-2.0 at your option.
