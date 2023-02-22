use crate::{graphics, impl_from_error, input, performance, types::*, voxel, Controller, Voxel};
use std::time;
use winit::{
    dpi::{PhysicalPosition, PhysicalSize},
    error::OsError,
    event::{DeviceEvent, ElementState, Event, VirtualKeyCode, WindowEvent},
    event_loop::EventLoop,
    window::{CursorGrabMode, WindowBuilder},
};

macro_rules! log_on_err {
    ($expr:expr) => {
        if let Err(error) = $expr {
            log::error!("{error:?}");
        }
    };
}

#[derive(Clone, Copy, Debug)]
pub enum WindowMode {
    Windowed(Extent2d<u32>),
}

#[derive(Clone, Copy, Debug)]
pub struct Descriptor {
    pub window: WindowMode,
    pub vsync: bool,

    pub world_size: Extent3d<u32>,
}

#[derive(Debug)]
pub enum Error {
    CreateWindowFailed(OsError),
    Graphics(graphics::Error),
    World(voxel::WorldError),
}

impl_from_error!(graphics::Error, Error, Graphics);
impl_from_error!(voxel::WorldError, Error, World);

pub struct Game {
    settings: Descriptor,
    graphics: graphics::Context,
    world: voxel::World,
    controller: Controller,
}

impl Game {
    const TITLE: &'static str = "NdrCraft";

    pub fn new(descriptor: Descriptor) -> Result<Self, Error> {
        let graphics = graphics::Context::new()?;
        let mut world = voxel::World::new(&graphics, descriptor.world_size, 3)?;

        let voxel_0 = image::io::Reader::open("assets/textures/voxel_0.png")
            .unwrap()
            .decode()
            .unwrap();
        let voxel_1 = image::io::Reader::open("assets/textures/voxel_1.png")
            .unwrap()
            .decode()
            .unwrap();
        let voxel_2 = image::io::Reader::open("assets/textures/voxel_2.png")
            .unwrap()
            .decode()
            .unwrap();
        world.set_voxel_texture(0, voxel::TextureLayout::Single, voxel_0.as_bytes())?;
        world.set_voxel_texture(1, voxel::TextureLayout::Single, voxel_1.as_bytes())?;
        world.set_voxel_texture(2, voxel::TextureLayout::Single, voxel_2.as_bytes())?;

        {
            performance::ScopedTimer::new("Generating world");
            let width = (world.size().width / 2) as i32;
            let height = (world.size().height / 2) as i32;
            let depth = (world.size().depth / 2) as i32;
            for z in -width..width {
                for y in -height..=0 {
                    for x in -depth..depth {
                        let threshold =
                            (x as f32 * 0.12).sin() * 1.2 + (z as f32 * 0.05).cos() * 0.5;
                        if y as f32 > threshold {
                            continue;
                        }
                        use std::collections::hash_map::DefaultHasher;
                        use std::hash::{Hash, Hasher};

                        let tile_index = {
                            let mut hasher = DefaultHasher::new();
                            z.hash(&mut hasher);
                            y.hash(&mut hasher);
                            x.hash(&mut hasher);
                            (hasher.finish() % 3) as u32
                        };
                        world.set_voxel(Offset3d::new(x, y, z), Voxel::Tile(tile_index))?;
                    }
                }
            }
            world.set_voxel(Offset3d::new(1, 4, 0), Voxel::Tile(0))?;
            world.set_voxel(Offset3d::new(0, 5, 0), Voxel::Tile(1))?;
            world.set_voxel(Offset3d::new(0, 4, 1), Voxel::Tile(2))?;
            world.set_voxel(Offset3d::new(-2, -2, 0), Voxel::Void)?;
            world.set_voxel(Offset3d::new(0, -2, 0), Voxel::Void)?;
            world.set_voxel(Offset3d::new(2, -2, 0), Voxel::Void)?;
            let void_pos = Offset3d::new(0, -4, 0);
            if let Some(_) = world.get_voxel(void_pos) {
                world.set_voxel(void_pos, Voxel::Void)?;
            }
        }
        {
            performance::ScopedTimer::new("Generating world mesh");
            world.generate_mesh();
        }
        let controller = Controller::new(Vector3::new(0.0, 2.0, 3.0), Deg(0.0), Deg(-30.0));
        Ok(Self {
            settings: descriptor,
            graphics,
            world,
            controller,
        })
    }

    pub fn run(mut self) -> Result<(), Error> {
        #[inline]
        fn get_projection(size: Extent2d<u32>) -> graphics::mesh::Projection {
            let aspect = size.width as f32 / size.height as f32;
            graphics::mesh::Projection::new_perspective(aspect, Deg(70.0), 0.0001, 100.0)
        }

        // create event_loop and window from settings
        let event_loop = EventLoop::new();
        let window = {
            let mut builder = WindowBuilder::new().with_title(Self::TITLE);
            builder = match self.settings.window {
                WindowMode::Windowed(size) => builder.with_inner_size(PhysicalSize::from(size)),
            };
            builder.build(&event_loop)
        }
        .map_err(|error| Error::CreateWindowFailed(error))?;
        window.set_cursor_grab(CursorGrabMode::Confined).unwrap();
        window.set_cursor_visible(false);

        // center window to monitor if the window mode is windowed
        if let (WindowMode::Windowed(_), Some(monitor)) =
            (self.settings.window, event_loop.primary_monitor())
        {
            let monitor_size = monitor.size();
            let window_size = window.outer_size();
            window.set_outer_position(PhysicalPosition::new(
                (monitor_size.width - window_size.width) as i32 / 2,
                (monitor_size.height - window_size.height) as i32 / 2,
            ));
        }

        // create render target
        let mut render_target =
            self.graphics
                .create_render_target(&window, self.settings.vsync, true)?;

        // create renderers
        let mut mesh_renderer = self.graphics.create_mesh_renderer(
            render_target.target_format(),
            get_projection(window.inner_size().into()),
        );

        // create renderables
        let material = graphics::Material {
            blend: graphics::material::BlendMode::Opaque,
            cull: graphics::material::CullMode::Back,
        };

        // timekeeping data (delta time, frame count)
        let mut frame_count = 0u64;
        let start = time::Instant::now();
        let mut delta_time = time::Duration::default();
        let mut last_tick = start;
        let mut last_second = start;

        // used to update the controller
        let lateral_speed = 10.0;
        let vertical_speed = 10.0;
        let look_speed = Vector2::new(0.2, 0.2);
        let mut right_axis = input::Axis::default();
        let mut forward_axis = input::Axis::default();
        let mut up_axis = input::Axis::default();
        let mut look_delta = Vector2::new(0.0, 0.0);

        event_loop.run(move |event, _, flow| match event {
            // main events
            Event::NewEvents(_) => {
                look_delta = Vector2::zero();

                // get delta time
                delta_time = last_tick.elapsed();
                last_tick = time::Instant::now();

                // update title every second
                if last_second.elapsed().as_secs() >= 1 {
                    last_second = time::Instant::now();
                    let fps = frame_count / start.elapsed().as_secs();
                    let ms = delta_time.as_millis();
                    let title = format!("{} [{fps} fps, {ms} ms]", Self::TITLE);
                    window.set_title(&title);
                }

                frame_count += 1;
            }
            Event::MainEventsCleared => {
                let delta_time = delta_time.as_secs_f32();

                look_delta.normalize();

                // move controller laterally (local right & forward)
                if let Some(input_direction) = {
                    match (right_axis.get_value(), forward_axis.get_value()) {
                        (None, None) => None,
                        (right, forward) => Some(
                            Vector2::new(right.unwrap_or_default(), forward.unwrap_or_default())
                                .normalize(),
                        ),
                    }
                } {
                    let translation = Vector3::new(input_direction.x, 0.0, input_direction.y)
                        * lateral_speed
                        * delta_time;
                    self.controller.translate_local(translation);
                }

                // move controller vertically (global up)
                if let Some(input_direction) = up_axis.get_value() {
                    let translation =
                        Vector3::unit_y() * input_direction * vertical_speed * delta_time;
                    self.controller.translate_global(translation);
                }

                let look_direction = look_delta.mul_element_wise(look_speed);
                self.controller.rotate_yaw(-Deg(look_direction.x));
                self.controller.rotate_pitch(-Deg(look_direction.y));

                mesh_renderer.set_view(
                    self.controller
                        .get_transform_matrix()
                        .inverse_transform()
                        .unwrap(),
                );
                window.request_redraw();
            }
            Event::LoopDestroyed => {}

            // window events
            Event::WindowEvent { window_id, event } if window_id == window.id() => match event {
                WindowEvent::CloseRequested => {
                    flow.set_exit();
                    return;
                }
                WindowEvent::Resized(_) => {
                    let window_size = window.inner_size().into();
                    log_on_err!(render_target.set_size(window_size));
                    mesh_renderer.set_projection(get_projection(window_size));
                }
                WindowEvent::KeyboardInput { input, .. } => {
                    match input.state {
                        ElementState::Pressed => match input.virtual_keycode {
                            // close window
                            Some(VirtualKeyCode::Escape) => flow.set_exit(),

                            // movement
                            Some(VirtualKeyCode::A) => right_axis.negative = true,
                            Some(VirtualKeyCode::D) => right_axis.positive = true,
                            Some(VirtualKeyCode::W) => forward_axis.negative = true,
                            Some(VirtualKeyCode::S) => forward_axis.positive = true,
                            Some(VirtualKeyCode::Q) => up_axis.negative = true,
                            Some(VirtualKeyCode::E) => up_axis.positive = true,
                            _ => {}
                        },
                        ElementState::Released => match input.virtual_keycode {
                            // movement
                            Some(VirtualKeyCode::A) => right_axis.negative = false,
                            Some(VirtualKeyCode::D) => right_axis.positive = false,
                            Some(VirtualKeyCode::W) => forward_axis.negative = false,
                            Some(VirtualKeyCode::S) => forward_axis.positive = false,
                            Some(VirtualKeyCode::Q) => up_axis.negative = false,
                            Some(VirtualKeyCode::E) => up_axis.positive = false,
                            _ => {}
                        },
                    }
                }
                _ => {}
            },
            Event::RedrawRequested(window_id) if window_id == window.id() => {
                log_on_err!(mesh_renderer.draw_mesh(
                    Matrix4::identity(),
                    self.world.mesh(),
                    &[graphics::mesh::MaterialTexture {
                        material,
                        texture: Some(self.world.texture()),
                    }],
                ));
                log_on_err!(render_target.draw_pass(
                    Some(Color::black()),
                    Some(1.0),
                    mesh_renderer.submit()
                ));
            }

            // mouse motion events
            Event::DeviceEvent { event, .. } => match event {
                DeviceEvent::MouseMotion { delta: (x, y) } => {
                    look_delta.x += x as f32;
                    look_delta.y += y as f32;
                }
                _ => {}
            },
            _ => {}
        });
    }
}
