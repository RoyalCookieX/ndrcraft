use pollster::block_on;
use std::rc::Rc;

#[derive(Debug)]
pub enum Error {
    RequestAdapterFailed,
    RequestDeviceFailed(wgpu::RequestDeviceError),
}

pub struct Context {
    _instance: wgpu::Instance,
    _adapter: wgpu::Adapter,
    _device: Rc<wgpu::Device>,
    _queue: Rc<wgpu::Queue>,
}

impl Context {
    pub(crate) fn new() -> Result<Self, Error> {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            dx12_shader_compiler: wgpu::Dx12Compiler::default(),
        });
        let adapter = block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            force_fallback_adapter: false,
            compatible_surface: None,
        }))
        .ok_or(Error::RequestAdapterFailed)?;
        let (device, queue) = block_on(adapter.request_device(
            &wgpu::DeviceDescriptor {
                label: None,
                features: wgpu::Features::empty(),
                limits: wgpu::Limits::default(),
            },
            None,
        ))
        .map(|(device, queue)| (Rc::new(device), Rc::new(queue)))
        .map_err(|error| Error::RequestDeviceFailed(error))?;
        Ok(Self {
            _instance: instance,
            _adapter: adapter,
            _device: device,
            _queue: queue,
        })
    }
}
