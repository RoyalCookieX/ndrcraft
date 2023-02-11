use crate::{Extent2d, Extent3d, Offset3d};
use std::{num::NonZeroU32, rc::Rc};

impl Sampler {
    pub const fn new(filter: FilterMode, address: AddressMode) -> Self {
        Self { filter, address }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Size {
    D2(Extent2d<u32>),
}

impl Size {
    pub fn is_valid(&self) -> bool {
        match self {
            Size::D2(size) => size.is_valid(),
        }
    }

    pub(crate) fn get_extent_dimension(&self) -> (wgpu::Extent3d, wgpu::TextureViewDimension) {
        match self {
            Self::D2(size) => (
                wgpu::Extent3d {
                    width: size.width,
                    height: size.height,
                    ..Default::default()
                },
                wgpu::TextureViewDimension::D2,
            ),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Format {
    Rgba8Unorm,
}

impl From<Format> for wgpu::TextureFormat {
    fn from(value: Format) -> Self {
        match value {
            Format::Rgba8Unorm => Self::Rgba8Unorm,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum FilterMode {
    Nearest,
    Linear,
}

impl From<FilterMode> for wgpu::FilterMode {
    fn from(value: FilterMode) -> Self {
        match value {
            FilterMode::Nearest => Self::Nearest,
            FilterMode::Linear => Self::Linear,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum AddressMode {
    ClampToEdge,
    Repeat,
    MirrorRepeat,
}

impl From<AddressMode> for wgpu::AddressMode {
    fn from(value: AddressMode) -> Self {
        match value {
            AddressMode::ClampToEdge => Self::ClampToEdge,
            AddressMode::Repeat => Self::Repeat,
            AddressMode::MirrorRepeat => Self::MirrorRepeat,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Sampler {
    pub filter: FilterMode,
    pub address: AddressMode,
}

#[derive(Debug, PartialEq, Eq)]
pub enum Error {
    PixelsInvalid,
    OriginInvalid,
    SizeInvalid,
}

#[derive(Debug)]
pub struct Texture {
    queue: Rc<wgpu::Queue>,
    size: Size,
    format: Format,
    sampler: Option<Sampler>,
    handle: Rc<wgpu::Texture>,
}

impl Texture {
    pub(super) fn new(
        device: &wgpu::Device,
        queue: Rc<wgpu::Queue>,
        size: Size,
        format: Format,
        sampler: Option<Sampler>,
        pixels: Option<&[u8]>,
    ) -> Result<Self, Error> {
        if !size.is_valid() {
            return Err(Error::SizeInvalid);
        }
        let (texture_size, format_size, handle) = {
            let (size, dimension) = size.get_extent_dimension();
            let format: wgpu::TextureFormat = format.into();
            let format_size = format.describe().block_size;
            let usage = wgpu::TextureUsages::COPY_DST | wgpu::TextureUsages::TEXTURE_BINDING;
            (
                size,
                format_size,
                Rc::new(device.create_texture(&wgpu::TextureDescriptor {
                    label: None,
                    size,
                    mip_level_count: 1,
                    sample_count: 1,
                    dimension: dimension.compatible_texture_dimension(),
                    format,
                    usage,
                    view_formats: &[],
                })),
            )
        };
        if let Some(pixels) = pixels {
            let origin = Offset3d::default();
            let size = texture_size.into();
            Self::write_texture(&queue, &handle, size, format_size, origin, size, pixels)?;
        }
        Ok(Self {
            queue,
            size,
            format,
            handle,
            sampler,
        })
    }

    pub fn size(&self) -> Extent3d<u32> {
        let (size, _) = self.size.get_extent_dimension();
        size.into()
    }

    pub fn sampler(&self) -> Option<Sampler> {
        self.sampler
    }

    pub(crate) fn view_dimension(&self) -> wgpu::TextureViewDimension {
        let (_, view_dimension) = self.size.get_extent_dimension();
        view_dimension
    }

    pub(crate) fn handle(&self) -> &Rc<wgpu::Texture> {
        &self.handle
    }

    pub fn write(
        &self,
        origin: Offset3d<u32>,
        size: Extent3d<u32>,
        pixels: &[u8],
    ) -> Result<(), Error> {
        let texture_size = self.size();
        let format_size = wgpu::TextureFormat::from(self.format).describe().block_size;
        Self::write_texture(
            &self.queue,
            &self.handle,
            texture_size,
            format_size,
            origin,
            size,
            pixels,
        )
    }

    #[inline]
    fn write_texture(
        queue: &wgpu::Queue,
        texture: &wgpu::Texture,
        texture_size: Extent3d<u32>,
        format_size: u8,
        origin: Offset3d<u32>,
        size: Extent3d<u32>,
        pixels: &[u8],
    ) -> Result<(), Error> {
        if !size.is_valid() {
            return Err(Error::SizeInvalid);
        };
        let origin_size = Extent3d::new(
            origin.x + size.width,
            origin.y + size.height,
            origin.z + size.depth,
        );
        if origin_size.width > texture_size.width
            || origin_size.height > texture_size.height
            || origin_size.depth > texture_size.depth
        {
            return Err(Error::OriginInvalid);
        }
        if pixels.len() < (format_size as u32 * size.width * size.height * size.depth) as usize {
            return Err(Error::PixelsInvalid);
        }
        queue.write_texture(
            wgpu::ImageCopyTexture {
                texture: &texture,
                mip_level: 0,
                origin: origin.into(),
                aspect: wgpu::TextureAspect::All,
            },
            pixels,
            wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: NonZeroU32::new(format_size as u32 * size.width),
                rows_per_image: NonZeroU32::new(size.height),
            },
            size.into(),
        );
        Ok(())
    }
}
