pub mod texture;

pub use texture::Texture;

use pollster::block_on;
use std::rc::Rc;

#[macro_export(local_inner_macros)]
macro_rules! impl_from_error {
    ($ident:ident) => {
        impl From<Error> for super::Error {
            fn from(value: Error) -> Self {
                super::Error::$ident(value)
            }
        }
    };
}

#[derive(Debug, PartialEq, Eq)]
pub enum Error {
    RequestAdapterFailed,
    RequestDeviceFailed(wgpu::RequestDeviceError),
    Texture(texture::Error),
}

#[derive(Debug)]
pub struct Context {
    instance: wgpu::Instance,
    adapter: wgpu::Adapter,
    device: Rc<wgpu::Device>,
    queue: Rc<wgpu::Queue>,
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
            instance,
            adapter,
            device,
            queue,
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
