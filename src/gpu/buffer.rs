// [WFGY] Zone: SAFE | λ: 0.3 | Fallbacks: 0/None | Action: Zero-copy GPU Buffer abstraction using bytemuck
use std::marker::PhantomData;
use wgpu::util::DeviceExt;

pub struct GpuBuffer {
    pub raw: wgpu::Buffer,
    pub size: u64,
    pub usage: wgpu::BufferUsages,
}

impl GpuBuffer {
    pub fn new_init(
        device: &wgpu::Device,
        label: Option<&str>,
        contents: &[u8],
        usage: wgpu::BufferUsages,
    ) -> Self {
        let raw = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label,
            contents,
            usage,
        });
        GpuBuffer {
            raw,
            size: contents.len() as u64,
            usage,
        }
    }

    pub fn new_empty(
        device: &wgpu::Device,
        label: Option<&str>,
        size: u64,
        usage: wgpu::BufferUsages,
    ) -> Self {
        let raw = device.create_buffer(&wgpu::BufferDescriptor {
            label,
            size,
            usage,
            mapped_at_creation: false,
        });
        GpuBuffer { raw, size, usage }
    }

    pub fn write_bytes(&self, queue: &wgpu::Queue, offset: u64, data: &[u8]) {
        queue.write_buffer(&self.raw, offset, data);
    }
}

pub struct UniformBuffer<T: bytemuck::Pod + bytemuck::Zeroable> {
    pub buffer: GpuBuffer,
    _phantom: PhantomData<T>,
}

impl<T: bytemuck::Pod + bytemuck::Zeroable> UniformBuffer<T> {
    pub fn new(device: &wgpu::Device, label: Option<&str>, initial_val: &T) -> Self {
        let contents = bytemuck::bytes_of(initial_val);
        let buffer = GpuBuffer::new_init(
            device,
            label,
            contents,
            wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        );
        UniformBuffer {
            buffer,
            _phantom: PhantomData,
        }
    }

    pub fn update(&self, queue: &wgpu::Queue, val: &T) {
        let data = bytemuck::bytes_of(val);
        self.buffer.write_bytes(queue, 0, data);
    }

    pub fn as_entire_binding(&self) -> wgpu::BindingResource<'_> {
        self.buffer.raw.as_entire_binding()
    }
}
