use crate::{impl_from_error, Bytes, Vector2, Vector3, Vector4};
use std::{cell::RefCell, rc::Rc};

#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct Vertex {
    pub position: Vector3<f32>,
    pub color: Vector4<f32>,
    pub uv: Vector2<f32>,
}
unsafe impl Bytes for Vertex {}

#[derive(Debug)]
struct BufferInfo {
    vertices_offset: u64,
    vertices_size: u64,
    aligned_size: u64,
}

impl BufferInfo {
    const ALIGNMENT: u64 = 256;

    fn new(vertices: &[Vertex]) -> Self {
        let vertices_offset = 0;
        let vertices_size = vertices.as_bytes().len() as u64;
        let aligned_size = Self::get_aligned_size(vertices_size);
        Self {
            vertices_offset,
            vertices_size,
            aligned_size,
        }
    }

    #[inline]
    const fn get_aligned_size(value: u64) -> u64 {
        let remainder = value % Self::ALIGNMENT;
        value + (Self::ALIGNMENT - remainder)
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum Error {
    VerticesInvalid,
}

impl_from_error!(Mesh);

#[derive(Debug)]
pub struct Mesh {
    pub vertices: Vec<Vertex>,
    device: Rc<wgpu::Device>,
    queue: Rc<wgpu::Queue>,
    buffer_info: RefCell<BufferInfo>,
    buffer: RefCell<Rc<wgpu::Buffer>>,
}

impl Mesh {
    pub(super) fn new(
        device: Rc<wgpu::Device>,
        queue: Rc<wgpu::Queue>,
        vertices: &[Vertex],
    ) -> Result<Self, Error> {
        Self::validate_vertices(&vertices)?;

        let vertices = vertices.to_owned();
        let buffer_info = BufferInfo::new(&vertices);
        let buffer_size = buffer_info.aligned_size;
        let buffer = Self::create_buffer(&device, buffer_size);
        Self::flush_buffer(&queue, &buffer_info, &buffer, &vertices);
        let buffer_info = RefCell::new(buffer_info);
        let buffer = RefCell::new(buffer);
        Ok(Self {
            vertices,
            device,
            queue,
            buffer_info,
            buffer,
        })
    }

    pub(crate) fn flush(&self) {
        let buffer_info = BufferInfo::new(&self.vertices);
        let buffer_size = self.buffer_info.borrow().aligned_size;
        if buffer_info.aligned_size > buffer_size {
            self.buffer_info.replace(buffer_info);
            self.buffer
                .replace(Self::create_buffer(&self.device, buffer_size));
        }
        Self::flush_buffer(
            &self.queue,
            &self.buffer_info.borrow(),
            &self.buffer.borrow(),
            &self.vertices,
        );
    }

    #[inline]
    fn validate_vertices(vertices: &[Vertex]) -> Result<(), Error> {
        if vertices.len() == 0 {
            return Err(Error::VerticesInvalid);
        }
        Ok(())
    }

    #[inline]
    fn create_buffer(device: &wgpu::Device, size: u64) -> Rc<wgpu::Buffer> {
        Rc::new(device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::VERTEX,
            mapped_at_creation: false,
        }))
    }

    #[inline]
    fn flush_buffer(
        queue: &wgpu::Queue,
        buffer_info: &BufferInfo,
        buffer: &wgpu::Buffer,
        vertices: &[Vertex],
    ) {
        queue.write_buffer(buffer, buffer_info.vertices_offset, vertices.as_bytes());
    }
}
