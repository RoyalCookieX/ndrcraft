use crate::{error_cast, Color, Extent2d};
use std::rc::Rc;
use winit::window::Window;

use super::DrawCommand;

#[derive(Debug, PartialEq, Eq)]
pub enum Error {
    CreateSurfaceFailed(wgpu::CreateSurfaceError),
    AdapterInvalid,
    AcquireTextureFailed(wgpu::SurfaceError),
    SizeInvalid,
}

error_cast!(RenderTarget => super::Error);

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

    pub(crate) fn draw_pass<I: IntoIterator<Item = DrawCommand>>(
        &self,
        clear_color: Option<Color<f64>>,
        commands: I,
    ) -> Result<(), Error> {
        let commands: Vec<_> = commands.into_iter().collect();
        let surface_texture = self
            .surface
            .get_current_texture()
            .map_err(|error| Error::AcquireTextureFailed(error))?;
        let surface_texture_view = surface_texture
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let load_color = match clear_color {
            Some(color) => wgpu::LoadOp::Clear(wgpu::Color::from(color)),
            None => wgpu::LoadOp::Load,
        };
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: None,
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &surface_texture_view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: load_color,
                        store: true,
                    },
                })],
                depth_stencil_attachment: None,
            });
            for command in &commands {
                match command {
                    DrawCommand::SetPipeline(pipeline) => render_pass.set_pipeline(pipeline),
                    DrawCommand::SetBindGroup { index, bind_group } => {
                        render_pass.set_bind_group(*index, bind_group, &[])
                    }
                    DrawCommand::SetVertexBuffer { buffer, start, end } => {
                        render_pass.set_vertex_buffer(0, buffer.slice(*start..*end))
                    }
                    DrawCommand::Draw { start, end } => render_pass.draw(*start..*end, 0..1),
                }
            }
        }
        self.queue.submit(Some(encoder.finish()));
        surface_texture.present();
        Ok(())
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
