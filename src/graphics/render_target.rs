use crate::{Bytes, Color, Extent2d};
use std::rc::Rc;
use winit::window::Window;

use super::{DrawCommand, DrawCommandList, TargetFormat};

#[derive(Debug, PartialEq, Eq)]
pub enum Error {
    CreateSurfaceFailed(wgpu::CreateSurfaceError),
    AdapterInvalid,
    AcquireTextureFailed(wgpu::SurfaceError),
    SizeInvalid,
    CommandsInvalid,
}

#[derive(Debug)]
pub struct RenderTarget {
    device: Rc<wgpu::Device>,
    queue: Rc<wgpu::Queue>,
    surface: wgpu::Surface,
    surface_config: wgpu::SurfaceConfiguration,
    depth_attachment: Option<(wgpu::TextureFormat, wgpu::TextureView)>,
}

impl RenderTarget {
    pub(super) fn new(
        instance: &wgpu::Instance,
        adapter: &wgpu::Adapter,
        device: Rc<wgpu::Device>,
        queue: Rc<wgpu::Queue>,
        window: &Window,
        vsync: bool,
        depth: bool,
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
            wgpu::PresentMode::AutoVsync
        } else {
            wgpu::PresentMode::AutoNoVsync
        };
        surface.configure(&device, &surface_config);
        let depth_attachment = if depth {
            Some(Self::create_depth_attachment(&device, window_size.into()))
        } else {
            None
        };
        Ok(Self {
            device,
            queue,
            surface,
            surface_config,
            depth_attachment,
        })
    }

    pub(crate) fn target_format(&self) -> TargetFormat {
        TargetFormat {
            color_format: self.surface_config.format,
            depth_format: self
                .depth_attachment
                .as_ref()
                .map(|(format, _)| format)
                .copied(),
        }
    }

    pub(crate) fn draw_pass<const PUSH_SIZE: usize, I: Into<DrawCommandList<PUSH_SIZE>>>(
        &self,
        clear_color: Option<Color<f64>>,
        clear_depth: Option<f32>,
        commands: I,
    ) -> Result<(), Error> {
        let command_list: DrawCommandList<PUSH_SIZE> = commands.into();
        if command_list.target_format != self.target_format() {
            return Err(Error::CommandsInvalid);
        }
        let surface_texture = self
            .surface
            .get_current_texture()
            .map_err(|error| Error::AcquireTextureFailed(error))?;
        let surface_texture_view = surface_texture
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let color_ops = {
            let load = match clear_color {
                Some(color) => wgpu::LoadOp::Clear(wgpu::Color::from(color)),
                None => wgpu::LoadOp::Load,
            };
            wgpu::Operations { load, store: true }
        };

        let depth_stencil_attachment = self.depth_attachment.as_ref().map(|(_, texture_view)| {
            let depth_ops = {
                let load = match clear_depth {
                    Some(depth) => wgpu::LoadOp::Clear(depth),
                    None => wgpu::LoadOp::Load,
                };
                Some(wgpu::Operations { load, store: true })
            };
            wgpu::RenderPassDepthStencilAttachment {
                view: texture_view,
                depth_ops,
                stencil_ops: None,
            }
        });
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: None,
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &surface_texture_view,
                    resolve_target: None,
                    ops: color_ops,
                })],
                depth_stencil_attachment,
            });
            for draw_command in command_list.draw_commands.iter() {
                match draw_command {
                    DrawCommand::SetPipeline(pipeline) => render_pass.set_pipeline(pipeline),
                    DrawCommand::SetBindGroup { index, bind_group } => {
                        render_pass.set_bind_group(*index, bind_group, &[])
                    }
                    DrawCommand::SetPushConstant {
                        stages,
                        offset,
                        data,
                    } => {
                        render_pass.set_push_constants(*stages, *offset, data.as_slice().as_bytes())
                    }
                    DrawCommand::SetVertexBuffer { buffer, start, end } => {
                        render_pass.set_vertex_buffer(0, buffer.slice(*start..*end))
                    }
                    DrawCommand::SetIndexBuffer { buffer, start, end } => render_pass
                        .set_index_buffer(buffer.slice(*start..*end), wgpu::IndexFormat::Uint32),
                    DrawCommand::Draw { start, end } => render_pass.draw(*start..*end, 0..1),
                    DrawCommand::DrawIndexed { start, end } => {
                        render_pass.draw_indexed(*start..*end, 0, 0..1)
                    }
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
        if let Some((format, texture_view)) = self.depth_attachment.as_mut() {
            (*format, *texture_view) = Self::create_depth_attachment(&self.device, size);
        }
        Ok(())
    }

    #[inline]
    fn create_depth_attachment(
        device: &wgpu::Device,
        size: Extent2d<u32>,
    ) -> (wgpu::TextureFormat, wgpu::TextureView) {
        let size = wgpu::Extent3d {
            width: size.width,
            height: size.height,
            ..Default::default()
        };
        let format = wgpu::TextureFormat::Depth24PlusStencil8;
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: None,
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });
        (
            format,
            texture.create_view(&wgpu::TextureViewDescriptor::default()),
        )
    }
}
