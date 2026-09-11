// [WFGY] Zone: SAFE | λ: 0.3 | Fallbacks: 0/None | Action: GPU Context encapsulation with Device, Queue and Adapter info
use std::sync::Arc;

#[derive(Clone)]
pub struct GpuContext {
    pub device: Arc<wgpu::Device>,
    pub queue: Arc<wgpu::Queue>,
    pub surface_format: wgpu::TextureFormat,
    pub adapter_info: wgpu::AdapterInfo,
}

impl GpuContext {
    pub fn new_standalone() -> Result<Self, String> {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::PRIMARY,
            ..Default::default()
        });

        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            compatible_surface: None,
            force_fallback_adapter: false,
        }))
        .ok_or_else(|| "Impossible de trouver un adaptateur GPU compatible".to_string())?;

        let adapter_info = adapter.get_info();

        let (device, queue) = pollster::block_on(adapter.request_device(
            &wgpu::DeviceDescriptor {
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::default(),
                label: Some("Primary GPU Device"),
            },
            None,
        ))
        .map_err(|e| format!("Erreur d'initialisation du périphérique GPU : {:?}", e))?;

        Ok(GpuContext {
            device: Arc::new(device),
            queue: Arc::new(queue),
            surface_format: wgpu::TextureFormat::Rgba8UnormSrgb,
            adapter_info,
        })
    }

    pub fn from_shared(
        device: Arc<wgpu::Device>,
        queue: Arc<wgpu::Queue>,
        surface_format: wgpu::TextureFormat,
        adapter_info: wgpu::AdapterInfo,
    ) -> Self {
        GpuContext {
            device,
            queue,
            surface_format,
            adapter_info,
        }
    }
}
