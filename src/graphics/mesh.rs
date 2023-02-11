use super::{material, texture, Context, DrawCommand, Material, Texture};
use crate::{ByteArray, Bytes, Color, Deg, Matrix4, SquareMatrix, Vector2, Vector3};
use std::{cell::RefCell, collections::HashMap, mem, rc::Rc};

#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct Vertex {
    pub position: Vector3<f32>,
    pub color: Color<f32>,
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

#[derive(Debug)]
pub struct Mesh {
    pub vertices: Vec<Vertex>,
    device: Rc<wgpu::Device>,
    queue: Rc<wgpu::Queue>,
    buffer_info: RefCell<BufferInfo>,
    buffer: RefCell<Rc<wgpu::Buffer>>,
}

impl Mesh {
    fn new(device: Rc<wgpu::Device>, queue: Rc<wgpu::Queue>, vertices: &[Vertex]) -> Self {
        let vertices = vertices.to_owned();
        let buffer_info = BufferInfo::new(&vertices);
        let buffer_size = buffer_info.aligned_size;
        let buffer = Self::create_buffer(&device, buffer_size);
        Self::flush_buffer(&queue, &buffer_info, &buffer, &vertices);
        let buffer_info = RefCell::new(buffer_info);
        let buffer = RefCell::new(buffer);
        Self {
            vertices,
            device,
            queue,
            buffer_info,
            buffer,
        }
    }

    pub(crate) fn flush(&self) {
        let buffer_info = BufferInfo::new(&self.vertices);
        let buffer_size = self.buffer_info.borrow().aligned_size;
        if buffer_info.aligned_size > buffer_size {
            self.buffer
                .replace(Self::create_buffer(&self.device, buffer_info.aligned_size));
        }
        self.buffer_info.replace(buffer_info);
        Self::flush_buffer(
            &self.queue,
            &self.buffer_info.borrow(),
            &self.buffer.borrow(),
            &self.vertices,
        );
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

#[derive(Clone, Copy, Debug)]
pub enum Projection {
    Perspective {
        aspect: f32,
        vertical_fov: Deg<f32>,
        near: f32,
        far: f32,
    },
}

impl Projection {
    pub const fn new_perspective(aspect: f32, vertical_fov: Deg<f32>, near: f32, far: f32) -> Self {
        Self::Perspective {
            aspect,
            vertical_fov,
            near,
            far,
        }
    }
}

impl From<Projection> for Matrix4<f32> {
    fn from(value: Projection) -> Self {
        match value {
            Projection::Perspective {
                aspect,
                vertical_fov,
                near,
                far,
            } => cgmath::perspective(vertical_fov, aspect, near, far),
        }
    }
}

#[derive(Clone, Copy, Debug)]
#[repr(C)]
struct Global {
    view: Matrix4<f32>,
    projection: Matrix4<f32>,
}
unsafe impl Bytes for Global {}

#[derive(Clone, Copy, Debug)]
#[repr(C)]
struct Push {
    model: Matrix4<f32>,
}
unsafe impl Bytes for Push {}
unsafe impl ByteArray<64> for Push {}

#[derive(Debug, PartialEq, Eq, Hash)]
struct PipelineIndex {
    material: Material,
    view_dimension: wgpu::TextureViewDimension,
}

#[derive(Debug)]
struct DrawMeshState {
    material: Material,
    view_dimension: wgpu::TextureViewDimension,
    mesh_buffer: Rc<wgpu::Buffer>,
    mesh_buffer_range: (u64, u64),
    mesh_vertex_count: u32,
    texture: Rc<wgpu::Texture>,
    sampler: Rc<wgpu::Sampler>,
    push_data: Push,
}

#[derive(Debug)]
pub struct Renderer {
    device: Rc<wgpu::Device>,
    queue: Rc<wgpu::Queue>,
    default_texture: Rc<Texture>,
    output_format: wgpu::TextureFormat,
    module: wgpu::ShaderModule,
    global_data: Global,
    global_buffer: wgpu::Buffer,
    global_bind_group_layout: wgpu::BindGroupLayout,
    global_bind_group: Rc<wgpu::BindGroup>,
    samplers: HashMap<texture::Sampler, Rc<wgpu::Sampler>>,
    texture_bind_group_layouts: HashMap<wgpu::TextureViewDimension, wgpu::BindGroupLayout>,
    pipelines: HashMap<PipelineIndex, Rc<wgpu::RenderPipeline>>,
    draw_states: Vec<DrawMeshState>,
}

impl Renderer {
    const DEFAULT_SAMPLER: texture::Sampler = texture::Sampler::new(
        texture::FilterMode::Linear,
        texture::AddressMode::ClampToEdge,
    );
    const PUSH_SIZE: usize = 64;

    fn new(
        device: Rc<wgpu::Device>,
        queue: Rc<wgpu::Queue>,
        default_texture: Rc<Texture>,
        output_format: wgpu::TextureFormat,
        projection: Projection,
    ) -> Self {
        let module = device.create_shader_module(wgpu::include_wgsl!("mesh.wgsl"));
        let global_data = Global {
            view: Matrix4::identity(),
            projection: projection.into(),
        };
        let global_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size: mem::size_of::<Global>() as u64,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::UNIFORM,
            mapped_at_creation: false,
        });
        queue.write_buffer(&global_buffer, 0, global_data.as_bytes());
        let global_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: None,
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
            });
        let global_bind_group = Rc::new(device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &global_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: global_buffer.as_entire_binding(),
            }],
        }));
        let samplers = HashMap::new();
        let texture_bind_group_layouts = HashMap::new();
        let pipelines = HashMap::new();
        let draw_states = Vec::new();
        Self {
            device,
            queue,
            default_texture,
            output_format,
            module,
            global_data,
            global_buffer,
            global_bind_group_layout,
            global_bind_group,
            samplers,
            texture_bind_group_layouts,
            pipelines,
            draw_states,
        }
    }

    pub fn set_view(&mut self, view: Matrix4<f32>) {
        self.global_data.view = view.into();
    }

    pub fn set_projection(&mut self, projection: Projection) {
        self.global_data.projection = projection.into();
    }

    pub fn draw_mesh(
        &mut self,
        transform: Matrix4<f32>,
        mesh: &Mesh,
        material: Material,
        texture: Option<&Texture>,
    ) {
        mesh.flush();

        // register texture sampler
        let sampler = texture.map_or(Self::DEFAULT_SAMPLER, |texture| {
            texture.sampler().unwrap_or(Self::DEFAULT_SAMPLER)
        });
        let sampler = self
            .samplers
            .entry(sampler)
            .or_insert_with(|| {
                Rc::new(self.device.create_sampler(&wgpu::SamplerDescriptor {
                    label: None,
                    address_mode_u: sampler.address.into(),
                    address_mode_v: sampler.address.into(),
                    address_mode_w: sampler.address.into(),
                    mag_filter: sampler.filter.into(),
                    min_filter: sampler.filter.into(),
                    ..Default::default()
                }))
            })
            .clone();

        let view_dimension = texture.map_or(self.default_texture.view_dimension(), |texture| {
            texture.view_dimension()
        });

        // register texture bind group layout
        let texture_bind_group_layout = self
            .texture_bind_group_layouts
            .entry(view_dimension)
            .or_insert_with(|| {
                self.device
                    .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                        label: None,
                        entries: &[
                            wgpu::BindGroupLayoutEntry {
                                binding: 0,
                                visibility: wgpu::ShaderStages::FRAGMENT,
                                ty: wgpu::BindingType::Texture {
                                    sample_type: wgpu::TextureSampleType::Float {
                                        filterable: true,
                                    },
                                    view_dimension: view_dimension,
                                    multisampled: false,
                                },
                                count: None,
                            },
                            wgpu::BindGroupLayoutEntry {
                                binding: 1,
                                visibility: wgpu::ShaderStages::FRAGMENT,
                                ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                                count: None,
                            },
                        ],
                    })
            });

        // register pipeline
        let pipeline_index = PipelineIndex {
            material,
            view_dimension,
        };
        self.pipelines.entry(pipeline_index).or_insert_with(|| {
                    let blend = match material.blend {
                        material::BlendMode::Opaque => wgpu::BlendState::REPLACE,
                    };
                    let pipeline_layout =
                        self.device
                            .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                                label: None,
                                bind_group_layouts: &[&self.global_bind_group_layout, texture_bind_group_layout],
                                push_constant_ranges: &[wgpu::PushConstantRange {
                                    stages: wgpu::ShaderStages::VERTEX,
                                    range: 0..Self::PUSH_SIZE as u32,
                                }],
                            });
                    Rc::new(
                        self.device
                            .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                                label: None,
                                layout: Some(&pipeline_layout),
                                vertex: wgpu::VertexState {
                                    module: &self.module,
                                    entry_point: "vs_main",
                                    buffers: &[wgpu::VertexBufferLayout {
                                        array_stride: mem::size_of::<Vertex>() as u64,
                                        step_mode: wgpu::VertexStepMode::Vertex,
                                        attributes: &wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x4, 2 => Float32x2],
                                    }],
                                },
                                primitive: wgpu::PrimitiveState {
                                    topology: wgpu::PrimitiveTopology::TriangleList,
                                    strip_index_format: None,
                                    front_face: wgpu::FrontFace::Cw,
                                    cull_mode: None,
                                    unclipped_depth: false,
                                    polygon_mode: wgpu::PolygonMode::Fill,
                                    conservative: false,
                                },
                                depth_stencil: None,
                                multisample: wgpu::MultisampleState {
                                    count: 1,
                                    mask: !0,
                                    alpha_to_coverage_enabled: false,
                                },
                                fragment: Some(wgpu::FragmentState {
                                    module: &self.module,
                                    entry_point: "fs_main",
                                    targets: &[Some(wgpu::ColorTargetState {
                                        format: self.output_format,
                                        blend: Some(blend),
                                        write_mask: wgpu::ColorWrites::ALL,
                                    })],
                                }),
                                multiview: None,
                            }),
                    )
                });

        // record mesh
        let mesh_buffer = mesh.buffer.borrow().clone();
        let mesh_buffer_range = {
            let buffer_info = mesh.buffer_info.borrow();
            let start = buffer_info.vertices_offset;
            let size = buffer_info.vertices_size;
            (start, start + size)
        };
        let mesh_vertex_count = mesh.vertices.len() as u32;
        let texture = texture.map_or(self.default_texture.handle().clone(), |texture| {
            texture.handle().clone()
        });
        let push_data = Push { model: transform };
        let draw_state = DrawMeshState {
            mesh_buffer,
            mesh_buffer_range,
            mesh_vertex_count,
            material,
            texture,
            view_dimension,
            sampler,
            push_data,
        };
        self.draw_states.push(draw_state);
    }

    pub(crate) fn submit(&mut self) -> Vec<DrawCommand<{ Self::PUSH_SIZE }>> {
        self.queue
            .write_buffer(&self.global_buffer, 0, self.global_data.as_bytes());
        let mut draw_commands = Vec::new();
        for draw_state in self.draw_states.drain(..) {
            let material = (&draw_state.material).clone();
            let view_dimension = (&draw_state.view_dimension).clone();
            let pipeline_index = PipelineIndex {
                material,
                view_dimension,
            };
            let pipeline = self.pipelines[&pipeline_index].clone();
            let texture_bind_group_layout = &self.texture_bind_group_layouts[&view_dimension];
            let texture = (&draw_state.texture).clone();
            let texture_view = texture.create_view(&wgpu::TextureViewDescriptor::default());
            let sampler = (&draw_state.sampler).clone();
            let texture_bind_group =
                Rc::new(self.device.create_bind_group(&wgpu::BindGroupDescriptor {
                    label: None,
                    layout: texture_bind_group_layout,
                    entries: &[
                        wgpu::BindGroupEntry {
                            binding: 0,
                            resource: wgpu::BindingResource::TextureView(&texture_view),
                        },
                        wgpu::BindGroupEntry {
                            binding: 1,
                            resource: wgpu::BindingResource::Sampler(&sampler),
                        },
                    ],
                }));
            let push_data = draw_state.push_data.as_byte_array().clone();
            let buffer = draw_state.mesh_buffer.clone();
            let buffer_range = draw_state.mesh_buffer_range;
            let vertex_count = draw_state.mesh_vertex_count;
            draw_commands.push(DrawCommand::SetPipeline(pipeline));
            draw_commands.push(DrawCommand::SetBindGroup {
                index: 0,
                bind_group: self.global_bind_group.clone(),
            });
            draw_commands.push(DrawCommand::SetBindGroup {
                index: 1,
                bind_group: texture_bind_group,
            });
            draw_commands.push(DrawCommand::SetPushConstant {
                stages: wgpu::ShaderStages::VERTEX,
                offset: 0,
                data: push_data,
            });
            draw_commands.push(DrawCommand::SetVertexBuffer {
                buffer,
                start: buffer_range.0,
                end: buffer_range.1,
            });
            draw_commands.push(DrawCommand::Draw {
                start: 0,
                end: vertex_count,
            });
        }
        draw_commands
    }
}

impl Context {
    pub fn create_mesh(&self, vertices: &[Vertex]) -> Mesh {
        Mesh::new(self.device.clone(), self.queue.clone(), vertices)
    }

    pub(crate) fn create_mesh_renderer(
        &self,
        output_format: wgpu::TextureFormat,
        projection: Projection,
    ) -> Renderer {
        Renderer::new(
            self.device.clone(),
            self.queue.clone(),
            self.default_texture.clone(),
            output_format,
            projection,
        )
    }
}
