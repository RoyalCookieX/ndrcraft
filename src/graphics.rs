pub mod material {
    #[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
    pub enum BlendMode {
        Opaque,
    }

    #[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
    pub struct Material {
        pub blend: BlendMode,
    }
}
pub mod mesh;
pub mod render_target;
pub mod texture;

pub use material::Material;
pub use mesh::Mesh;
pub use render_target::RenderTarget;
pub use texture::Texture;

use crate::{error_cast, Extent2d};
use pollster::block_on;
use std::rc::Rc;
use winit::window::Window;

pub(crate) enum DrawCommand {
    SetPipeline(Rc<wgpu::RenderPipeline>),
    SetBindGroup {
        index: u32,
        bind_group: Rc<wgpu::BindGroup>,
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

#[derive(Debug, PartialEq, Eq)]
pub enum Error {
    RequestAdapterFailed,
    RequestDeviceFailed(wgpu::RequestDeviceError),
    Texture(texture::Error),
    Mesh(mesh::Error),
    RenderTarget(render_target::Error),
}

error_cast!(Graphics => crate::game::Error);

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
    ) -> Result<RenderTarget, Error> {
        RenderTarget::new(
            &self.instance,
            &self.adapter,
            self.device.clone(),
            self.queue.clone(),
            window,
            vsync,
        )
        .map_err(|error| error.into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::*;

    macro_rules! assert_err {
        ($expr:expr => $err:expr) => {
            let Err(error) = $expr else { panic!("value is valid") };
            assert_eq!(error, $err);
        };
    }

    #[test]
    fn mesh_test() {
        let graphics = Context::new().expect("valid context");
        let vertices = [
            mesh::Vertex {
                position: Vector3::new(-0.5, -0.5, -1.0),
                color: Vector4::new(1.0, 0.0, 0.0, 1.0),
                uv: Vector2::new(0.0, 0.0),
            },
            mesh::Vertex {
                position: Vector3::new(0.0, 0.5, -1.0),
                color: Vector4::new(0.0, 1.0, 0.0, 1.0),
                uv: Vector2::new(0.5, 1.0),
            },
            mesh::Vertex {
                position: Vector3::new(0.5, -0.5, -1.0),
                color: Vector4::new(0.0, 0.0, 1.0, 1.0),
                uv: Vector2::new(1.0, 0.0),
            },
        ];
        assert_err!(graphics.create_mesh(&[]) => Error::Mesh(mesh::Error::VerticesInvalid));
        let mut mesh = graphics.create_mesh(&vertices).expect("valid mesh");
        mesh.vertices[0].color = Vector4::new(0.5, 0.1, 0.9, 1.0);
        mesh.flush();
    }

    #[test]
    fn texture_test() {
        let graphics = Context::new().expect("valid context");
        assert_err!(graphics
            .create_texture(
                texture::Size::D2(Extent2d::new(0, 0)),
                texture::Format::Rgba8Unorm,
                None,
                None,
            ) => Error::Texture(texture::Error::SizeInvalid));
        assert_err!(graphics
            .create_texture(
                texture::Size::D2(Extent2d::new(4, 4)),
                texture::Format::Rgba8Unorm,
                Some(texture::Sampler::new(
                    texture::FilterMode::Linear,
                    texture::AddressMode::ClampToEdge,
                )),
                Some(&[]),
            ) => Error::Texture(texture::Error::PixelsInvalid));
        let texture = graphics
            .create_texture(
                texture::Size::D2(Extent2d::new(4, 4)),
                texture::Format::Rgba8Unorm,
                Some(texture::Sampler::new(
                    texture::FilterMode::Linear,
                    texture::AddressMode::ClampToEdge,
                )),
                Some(&[0x00, 0x00, 0x00, 0xFF].repeat(4 * 4)),
            )
            .expect("valid texture");
        assert_err!(texture
            .write(Offset3d::default(), Extent3d::new(0, 0, 0), &[]) => texture::Error::SizeInvalid);
        assert_err!(texture
                .write(Offset3d { x: 32, y: 32, z: 32 }, Extent3d::new(1, 1, 1), &[]) => texture::Error::OriginInvalid);
        assert_err!(texture
                    .write(Offset3d::default(), Extent3d::new(1, 1, 1), &[]) => texture::Error::PixelsInvalid);
        texture
            .write(
                Offset3d::default(),
                Extent3d::new(1, 1, 1),
                &[0xFF, 0xFF, 0xFF, 0xFF].repeat(1 * 1),
            )
            .expect("valid texture write");
    }
}
