pub mod material {
    #[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
    pub enum BlendMode {
        Opaque,
    }

    #[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
    pub enum CullMode {
        None,
        Front,
        Back,
    }

    #[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
    pub struct Material {
        pub blend: BlendMode,
        pub cull: CullMode,
    }
}
pub mod mesh;
pub mod render_target;
pub mod texture;

pub use material::Material;
pub use mesh::Mesh;
pub use render_target::RenderTarget;
pub use texture::Texture;

use crate::{impl_from_error, Extent2d};
use pollster::block_on;
use std::rc::Rc;
use winit::window::Window;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) struct TargetFormat {
    pub color_format: wgpu::TextureFormat,
    pub depth_format: Option<wgpu::TextureFormat>,
}

#[derive(Debug)]
pub(crate) enum DrawCommand<const PUSH_SIZE: usize = 256> {
    SetPipeline(Rc<wgpu::RenderPipeline>),
    SetBindGroup {
        index: u32,
        bind_group: Rc<wgpu::BindGroup>,
    },
    SetPushConstant {
        stages: wgpu::ShaderStages,
        offset: u32,
        data: [u8; PUSH_SIZE],
    },
    SetVertexBuffer {
        buffer: Rc<wgpu::Buffer>,
        start: u64,
        end: u64,
    },
    Draw {
        start: u32,
        end: u32,
    },
}

#[derive(Debug)]
pub(crate) struct DrawCommandList<const PUSH_SIZE: usize> {
    pub target_format: TargetFormat,
    pub draw_commands: Vec<DrawCommand<PUSH_SIZE>>,
}

#[derive(Debug, PartialEq, Eq)]
pub enum Error {
    RequestAdapterFailed,
    RequestDeviceFailed(wgpu::RequestDeviceError),
    Texture(texture::Error),
    RenderTarget(render_target::Error),
}

impl_from_error!(texture::Error, Error, Texture);
impl_from_error!(render_target::Error, Error, RenderTarget);

#[derive(Debug)]
pub struct Context {
    instance: wgpu::Instance,
    adapter: wgpu::Adapter,
    pub(crate) device: Rc<wgpu::Device>,
    pub(crate) queue: Rc<wgpu::Queue>,
    pub(crate) default_texture: Rc<Texture>,
}

impl Context {
    pub(crate) fn new() -> Result<Self, Error> {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::VULKAN,
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
                features: adapter.features(),
                limits: adapter.limits(),
            },
            None,
        ))
        .map(|(device, queue)| (Rc::new(device), Rc::new(queue)))
        .map_err(|error| Error::RequestDeviceFailed(error))?;
        let default_texture = Rc::new(Texture::new(
            &device,
            queue.clone(),
            texture::Size::D2(Extent2d::new(1, 1)),
            texture::Format::Rgba8Unorm,
            None,
            Some(&[0xFF, 0xFF, 0xFF, 0xFF]),
        )?);
        Ok(Self {
            instance,
            adapter,
            device,
            queue,
            default_texture,
        })
    }

    pub fn create_texture(
        &self,
        size: texture::Size,
        format: texture::Format,
        sampler: Option<texture::Sampler>,
        pixels: Option<&[u8]>,
    ) -> Result<Texture, Error> {
        Texture::new(
            &self.device,
            self.queue.clone(),
            size,
            format,
            sampler,
            pixels,
        )
        .map_err(|error| error.into())
    }

    pub(crate) fn create_render_target(
        &self,
        window: &Window,
        vsync: bool,
        depth: bool,
    ) -> Result<RenderTarget, Error> {
        RenderTarget::new(
            &self.instance,
            &self.adapter,
            self.device.clone(),
            self.queue.clone(),
            window,
            vsync,
            depth,
        )
        .map_err(|error| error.into())
    }
}
