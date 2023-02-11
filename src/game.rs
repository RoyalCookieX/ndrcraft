use crate::{graphics, impl_from_error, types::*, voxel, Voxel};
use winit::{
    dpi::{PhysicalPosition, PhysicalSize},
    error::OsError,
    event::{Event, WindowEvent},
    event_loop::EventLoop,
    window::WindowBuilder,
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
}

impl Game {
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
            for y in -height..=0 {
                for x in -depth..depth {
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
        Ok(Self {
            settings,
            graphics,
            world,
        })
    }

    pub fn run(self) -> Result<(), Error> {
        #[inline]
        fn get_projection(size: Extent2d<u32>) -> graphics::mesh::Projection {
            let aspect = size.width as f32 / size.height as f32;
            graphics::mesh::Projection::new_perspective(aspect, Deg(70.0), 0.0001, 100.0)
        }

        // create event_loop and window from settings
        let event_loop = EventLoop::new();
        let window = {
            let mut builder = WindowBuilder::new().with_title("NdrCraft");
            builder = match self.settings.window_mode {
                WindowMode::Windowed(size) => builder.with_inner_size(PhysicalSize::from(size)),
            };
            builder.build(&event_loop)
        }
        .map_err(|error| Error::CreateWindowFailed(error))?;

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
        let camera = Matrix4::from_translation(Vector3::new(0.0, 2.0, 3.0))
            * Matrix4::from_angle_x(Deg(-30.0));
        mesh_renderer.set_view(camera.inverse_transform().unwrap());

        // create renderables
        let material = graphics::Material {
            blend: graphics::material::BlendMode::Opaque,
        };

        event_loop.run(move |event, _, flow| match event {
            // main events
            Event::NewEvents(_) => {}
            Event::MainEventsCleared => {
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
            _ => {}
        });
    }
}
