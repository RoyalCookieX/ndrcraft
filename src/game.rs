use crate::{graphics, impl_from_error, types::*, voxel, Controller, Voxel};
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
pub struct Settings {
    pub window_mode: WindowMode,
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
    settings: Settings,
    graphics: graphics::Context,
    world: voxel::World,
    controller: Controller,
}

impl Game {
    const TITLE: &'static str = "NdrCraft";

    pub fn new(settings: Settings) -> Result<Self, Error> {
        let graphics = graphics::Context::new()?;
        let mut world = voxel::World::new(&graphics, settings.world_size, 3)?;
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
        let width = (world.size().width / 2) as i32;
        let height = (world.size().height / 2) as i32;
        let depth = (world.size().depth / 2) as i32;
        for z in -width..width {
            for y in -height..height {
                for x in -depth..depth {
                    let threshold = (x as f32 * 0.12).sin() * 1.2 + (z as f32 * 0.05).cos() * 0.5;
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
        world.generate_mesh();
        let controller = Controller::new(Vector3::new(0.0, 2.0, 3.0), Deg(-30.0), Deg(0.0));
        Ok(Self {
            settings,
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
            builder = match self.settings.window_mode {
                WindowMode::Windowed(size) => builder.with_inner_size(PhysicalSize::from(size)),
            };
            builder.build(&event_loop)
        }
        .map_err(|error| Error::CreateWindowFailed(error))?;
        window.set_cursor_grab(CursorGrabMode::Confined).unwrap();
        window.set_cursor_visible(false);

        // center window to monitor if `settings.window_mode` is `Windowed`
        if let (WindowMode::Windowed(_), Some(monitor)) =
            (self.settings.window_mode, event_loop.primary_monitor())
        {
            let monitor_size = monitor.size();
            let window_size = window.outer_size();
            window.set_outer_position(PhysicalPosition::new(
                (monitor_size.width - window_size.width) as i32 / 2,
                (monitor_size.height - window_size.height) as i32 / 2,
            ));
        }

        // create render target
        let mut render_target = self
            .graphics
            .create_render_target(&window, self.settings.vsync, true)
            .map_err(|error| Error::Graphics(error.into()))?;

        // create renderers
        let mut mesh_renderer = self.graphics.create_mesh_renderer(
            render_target.target_format(),
            get_projection(window.inner_size().into()),
        );

        // create renderables
        let material = graphics::Material {
            blend: graphics::material::BlendMode::Opaque,
        };

        // timekeeping data (delta time, frame count)
        let start = time::Instant::now();
        let mut delta_time = 0.0;
        let mut last_tick = start;
        let mut last_second = start;
        let mut frame_count = 0u128;

        // used to update the controller
        let lateral_speed = 10.0;
        let vertical_speed = 10.0;
        let look_speed = Vector2::new(10.0, 15.0);
        let mut lateral_direction = Vector2::zero();
        let mut vertical_direction = 0.0;

        event_loop.run(move |event, _, flow| match event {
            // main events
            Event::NewEvents(_) => {
                // get delta time
                delta_time = last_tick.elapsed().as_secs_f32();
                last_tick = time::Instant::now();

                // update title every second
                if last_second.elapsed().as_secs_f32() > 1.0 {
                    last_second = time::Instant::now();
                    let fps = frame_count as f32 / start.elapsed().as_secs_f32();
                    let ms = delta_time * 1000.0;
                    let title = format!("{} [{fps:.2} fps, {ms:.2} ms]", Self::TITLE);
                    window.set_title(&title);
                }

                frame_count += 1;
            }
            Event::MainEventsCleared => {
                // move controller
                lateral_direction.normalize();
                let lateral_translation =
                    Vector3::new(lateral_direction.x, 0.0, lateral_direction.y)
                        * (lateral_speed * delta_time);
                let vertical_translation =
                    Vector3::unit_y() * vertical_direction * vertical_speed * delta_time;
                self.controller.translate_local(lateral_translation);
                self.controller.translate_global(vertical_translation);
                mesh_renderer
                    .set_view(self.controller.get_transform().inverse_transform().unwrap());

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
                WindowEvent::KeyboardInput { input, .. } => match input.state {
                    ElementState::Pressed => match input.virtual_keycode {
                        // close window
                        Some(VirtualKeyCode::Escape) => flow.set_exit(),

                        // movement
                        Some(VirtualKeyCode::A) => lateral_direction.x = -1.0,
                        Some(VirtualKeyCode::D) => lateral_direction.x = 1.0,
                        Some(VirtualKeyCode::S) => lateral_direction.y = 1.0,
                        Some(VirtualKeyCode::W) => lateral_direction.y = -1.0,
                        Some(VirtualKeyCode::Q) => vertical_direction = -1.0,
                        Some(VirtualKeyCode::E) => vertical_direction = 1.0,
                        _ => {}
                    },
                    ElementState::Released => match input.virtual_keycode {
                        // movement
                        Some(VirtualKeyCode::A) | Some(VirtualKeyCode::D) => {
                            lateral_direction.x = 0.0
                        }
                        Some(VirtualKeyCode::S) | Some(VirtualKeyCode::W) => {
                            lateral_direction.y = 0.0
                        }
                        Some(VirtualKeyCode::Q) | Some(VirtualKeyCode::E) => {
                            vertical_direction = 0.0
                        }
                        _ => {}
                    },
                },
                _ => {}
            },
            Event::RedrawRequested(window_id) if window_id == window.id() => {
                mesh_renderer.draw_mesh(
                    Matrix4::identity(),
                    self.world.mesh(),
                    material,
                    Some(self.world.texture()),
                );
                log_on_err!(render_target.draw_pass(
                    Some(Color::black()),
                    Some(1.0),
                    mesh_renderer.submit()
                ));
            }

            // mouse motion events
            Event::DeviceEvent { event, .. } => match event {
                DeviceEvent::MouseMotion { delta: (x, y) } => {
                    let (x, y) = (x as f32, y as f32);
                    self.controller
                        .rotate_yaw(-Deg(x * look_speed.x * delta_time));
                    self.controller
                        .rotate_pitch(-Deg(y * look_speed.y * delta_time));
                }
                _ => {}
            },
            _ => {}
        });
    }
}
