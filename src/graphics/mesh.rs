use super::{
    material, texture, Context, DrawCommand, DrawCommandList, Material, TargetFormat, Texture,
};
use crate::{ByteArray, Bytes, Color, Deg, Matrix4, SquareMatrix, Vector2, Vector3, Zero};
use std::{cell::RefCell, collections::HashMap, mem, rc::Rc};

#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct Vertex {
    pub position: Vector3<f32>,
    pub color: Color<f32>,
    pub uv: Vector2<f32>,
}

impl Vertex {
    pub const fn new(position: Vector3<f32>, color: Color<f32>, uv: Vector2<f32>) -> Self {
        Self {
            position,
            color,
            uv,
        }
    }
}

impl Default for Vertex {
    fn default() -> Self {
        Self {
            position: Vector3::zero(),
            color: Color::white(),
            uv: Vector2::zero(),
        }
    }
}

unsafe impl Bytes for Vertex {}

#[derive(Debug)]
pub struct Submesh {
    pub indices: Vec<u32>,
}

impl Submesh {
    pub fn new(indices: &[u32]) -> Self {
        let indices = indices.to_owned();
        Self { indices }
    }
}

#[derive(Debug)]
struct BufferInfo {
    pub vertices_offset_size: (u64, u64),
    pub indices_offset_sizes: Vec<(u64, u64)>,
    pub aligned_size: u64,
}

impl BufferInfo {
    const ALIGNMENT: u64 = 256;

    fn new(vertices: &[Vertex], submeshes: &[Submesh]) -> Self {
        let vertices_offset = 0;
        let vertices_size = vertices.as_bytes().len() as u64;
        let mut aligned_size = Self::get_aligned_size(vertices_size);
        let mut indices_offset_sizes = Vec::with_capacity(submeshes.len());
        for submesh in submeshes {
            let indices_offset = aligned_size;
            let indices_size = submesh.indices.as_slice().as_bytes().len() as u64;
            indices_offset_sizes.push((indices_offset, indices_size));
            aligned_size += Self::get_aligned_size(indices_size);
        }
        let vertices_offset_size = (vertices_offset, vertices_size);
        Self {
            vertices_offset_size,
            indices_offset_sizes,
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
    pub submeshes: Vec<Submesh>,
    device: Rc<wgpu::Device>,
    queue: Rc<wgpu::Queue>,
    buffer_info: RefCell<BufferInfo>,
    buffer: RefCell<Rc<wgpu::Buffer>>,
}

impl Mesh {
    fn new(
        device: Rc<wgpu::Device>,
        queue: Rc<wgpu::Queue>,
        vertices: &[Vertex],
        indices: &[&[u32]],
    ) -> Self {
        let vertices = vertices.to_owned();
        let submeshes: Vec<_> = indices
            .into_iter()
            .map(|indices| Submesh::new(indices))
            .collect();
        let buffer_info = BufferInfo::new(&vertices, &submeshes);
        let buffer_size = buffer_info.aligned_size;
        let buffer = Self::create_buffer(&device, buffer_size);
        Self::flush_buffer(&queue, &buffer_info, &buffer, &vertices, &submeshes);
        let buffer_info = RefCell::new(buffer_info);
        let buffer = RefCell::new(buffer);
        Self {
            vertices,
            submeshes,
            device,
            queue,
            buffer_info,
            buffer,
        }
    }

    pub(crate) fn flush(&self) {
        let buffer_info = BufferInfo::new(&self.vertices, &self.submeshes);
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
            &self.submeshes,
        );
    }

    #[inline]
    fn create_buffer(device: &wgpu::Device, size: u64) -> Rc<wgpu::Buffer> {
        Rc::new(device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size,
            usage: wgpu::BufferUsages::COPY_DST
                | wgpu::BufferUsages::VERTEX
                | wgpu::BufferUsages::INDEX,
            mapped_at_creation: false,
        }))
    }

    #[inline]
    fn flush_buffer(
        queue: &wgpu::Queue,
        buffer_info: &BufferInfo,
        buffer: &wgpu::Buffer,
        vertices: &[Vertex],
        submeshes: &[Submesh],
    ) {
        queue.write_buffer(
            buffer,
            buffer_info.vertices_offset_size.0,
            vertices.as_bytes(),
        );
        for ((offset, _), submesh) in buffer_info
            .indices_offset_sizes
            .iter()
            .zip(submeshes.iter())
        {
            queue.write_buffer(buffer, *offset, submesh.indices.as_slice().as_bytes())
        }
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
pub struct MaterialTexture<'a> {
    pub material: Material,
    pub texture: Option<&'a Texture>,
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

#[derive(Clone, Copy, Debug)]
struct DrawBufferState {
    range: (u64, u64),
    element_count: u32,
}

#[derive(Debug)]
struct DrawMeshState {
    material: Material,
    view_dimension: wgpu::TextureViewDimension,
    buffer: Rc<wgpu::Buffer>,
    vertex_buffer: DrawBufferState,
    index_buffer: Option<DrawBufferState>,
    texture: Rc<wgpu::Texture>,
    sampler: Rc<wgpu::Sampler>,
    push_data: Push,
}

#[derive(Debug, PartialEq, Eq)]
pub enum RendererError {
    MaterialTexturesInvalid,
}

#[derive(Debug)]
pub struct Renderer {
    device: Rc<wgpu::Device>,
    queue: Rc<wgpu::Queue>,
    default_texture: Rc<Texture>,
    target_format: TargetFormat,
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
        target_format: TargetFormat,
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
            target_format,
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
        material_textures: &[MaterialTexture<'_>],
    ) -> Result<(), RendererError> {
        if material_textures.len() == 0 || material_textures.len() < mesh.submeshes.len() {
            return Err(RendererError::MaterialTexturesInvalid);
        }

        mesh.flush();
        let is_indexed = !mesh.submeshes.is_empty();

        // common draw state
        let buffer = mesh.buffer.borrow();
        let buffer_info = mesh.buffer_info.borrow();
        let vertex_buffer_range = {
            let start = buffer_info.vertices_offset_size.0;
            let size = buffer_info.vertices_offset_size.1;
            (start, start + size)
        };
        let vertex_count = mesh.vertices.len() as u32;
        let vertex_buffer = DrawBufferState {
            range: vertex_buffer_range,
            element_count: vertex_count,
        };
        let push_data = Push { model: transform };

        // record draw state per material-texture
        for (i, MaterialTexture { material, texture }) in material_textures.iter().enumerate() {
            let material = *material;

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
                                    ty: wgpu::BindingType::Sampler(
                                        wgpu::SamplerBindingType::Filtering,
                                    ),
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
            let cull_mode = match material.cull {
                material::CullMode::None => None,
                material::CullMode::Front => Some(wgpu::Face::Front),
                material::CullMode::Back => Some(wgpu::Face::Back),
            };
            let depth_stencil =
                self.target_format
                    .depth_format
                    .map(|format| wgpu::DepthStencilState {
                        format,
                        depth_write_enabled: true,
                        depth_compare: wgpu::CompareFunction::Less,
                        stencil: wgpu::StencilState::default(),
                        bias: wgpu::DepthBiasState::default(),
                    });
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
                                    cull_mode,
                                    unclipped_depth: false,
                                    polygon_mode: wgpu::PolygonMode::Fill,
                                    conservative: false,
                                },
                                depth_stencil,
                                multisample: wgpu::MultisampleState {
                                    count: 1,
                                    mask: !0,
                                    alpha_to_coverage_enabled: false,
                                },
                                fragment: Some(wgpu::FragmentState {
                                    module: &self.module,
                                    entry_point: "fs_main",
                                    targets: &[Some(wgpu::ColorTargetState {
                                        format: self.target_format.color_format,
                                        blend: Some(blend),
                                        write_mask: wgpu::ColorWrites::ALL,
                                    })],
                                }),
                                multiview: None,
                            }),
                    )
                });

            let index_buffer = if is_indexed {
                let index_buffer_range = {
                    let start = buffer_info.indices_offset_sizes[i].0;
                    let size = buffer_info.indices_offset_sizes[i].1;
                    (start, start + size)
                };
                let index_count = mesh.submeshes[i].indices.len() as u32;
                Some(DrawBufferState {
                    range: index_buffer_range,
                    element_count: index_count,
                })
            } else {
                None
            };

            let texture = texture.map_or_else(
                || self.default_texture.handle().clone(),
                |texture| texture.handle().clone(),
            );

            let draw_state = DrawMeshState {
                buffer: buffer.clone(),
                vertex_buffer,
                index_buffer,
                material,
                texture,
                view_dimension,
                sampler,
                push_data,
            };
            self.draw_states.push(draw_state);
        }
        Ok(())
    }

    pub(crate) fn submit(&mut self) -> DrawCommandList<{ Self::PUSH_SIZE }> {
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
            let buffer = draw_state.buffer.clone();
            let vertex_buffer_range = draw_state.vertex_buffer.range;
            let vertex_count = draw_state.vertex_buffer.element_count;
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
                buffer: buffer.clone(),
                start: vertex_buffer_range.0,
                end: vertex_buffer_range.1,
            });
            if let Some(index_buffer) = draw_state.index_buffer {
                let index_range = index_buffer.range;
                let index_count = index_buffer.element_count;
                draw_commands.push(DrawCommand::SetIndexBuffer {
                    buffer,
                    start: index_range.0,
                    end: index_range.1,
                });
                draw_commands.push(DrawCommand::DrawIndexed {
                    start: 0,
                    end: index_count,
                })
            } else {
                draw_commands.push(DrawCommand::Draw {
                    start: 0,
                    end: vertex_count,
                });
            }
        }
        DrawCommandList {
            target_format: self.target_format,
            draw_commands,
        }
    }
}

impl Context {
    pub fn create_mesh(&self, vertices: &[Vertex], indices: &[&[u32]]) -> Mesh {
        Mesh::new(self.device.clone(), self.queue.clone(), vertices, indices)
    }

    pub(crate) fn create_mesh_renderer(
        &self,
        target_format: TargetFormat,
        projection: Projection,
    ) -> Renderer {
        Renderer::new(
            self.device.clone(),
            self.queue.clone(),
            self.default_texture.clone(),
            target_format,
            projection,
        )
    }
}
