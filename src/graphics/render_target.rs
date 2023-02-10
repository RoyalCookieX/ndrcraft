use crate::{impl_from_error, Extent2d};
use std::rc::Rc;
use winit::window::Window;

#[derive(Debug, PartialEq, Eq)]
pub enum Error {
    CreateSurfaceFailed(wgpu::CreateSurfaceError),
    AdapterInvalid,
    AcquireTextureFailed(wgpu::SurfaceError),
    SizeInvalid,
}

impl_from_error!(RenderTarget);

#[derive(Debug)]
pub struct RenderTarget {
    device: Rc<wgpu::Device>,
    queue: Rc<wgpu::Queue>,
    surface: wgpu::Surface,
    surface_config: wgpu::SurfaceConfiguration,
}

impl RenderTarget {
    pub(super) fn new(
        instance: &wgpu::Instance,
        adapter: &wgpu::Adapter,
        device: Rc<wgpu::Device>,
        queue: Rc<wgpu::Queue>,
        window: &Window,
        vsync: bool,
    ) -> Result<Self, Error> {
        let surface = unsafe {
            instance
                .create_surface(window)
                .map_err(|error| Error::CreateSurfaceFailed(error))?
        };
        let window_size = window.inner_size();
        let mut surface_config = surface
            .get_default_config(adapter, window_size.width, window_size.height)
            .ok_or(Error::AdapterInvalid)?;
        surface_config.present_mode = if vsync {
            wgpu::PresentMode::AutoNoVsync
        } else {
            wgpu::PresentMode::AutoVsync
        };
        surface.configure(&device, &surface_config);
        Ok(Self {
            device,
            queue,
            surface,
            surface_config,
        })
    }

    pub(crate) fn output_format(&self) -> wgpu::TextureFormat {
        self.surface_config.format
    }

    pub(crate) fn set_size(&mut self, size: Extent2d<u32>) -> Result<(), Error> {
        if !size.is_valid() {
            return Err(Error::SizeInvalid);
        }
        self.surface_config.width = size.width;
        self.surface_config.height = size.height;
        self.surface.configure(&self.device, &self.surface_config);
        Ok(())
    }
}
