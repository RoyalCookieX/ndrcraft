use crate::{graphics, types::*, voxel, Voxel};
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
}

pub struct Game {
    settings: Settings,
    graphics: graphics::Context,
    world: voxel::World,
}

impl Game {
    pub fn new(settings: Settings) -> Result<Self, Error> {
        let graphics = graphics::Context::new().map_err(|error| Error::Graphics(error))?;
        let mut world = voxel::World::new(&graphics, settings.world_size);
        world.set_voxel(Offset3d::new(0, 0, 0), Voxel::Tile(0));
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
            .create_render_target(&window, self.settings.vsync)
            .map_err(|error| Error::Graphics(error.into()))?;

        // create renderers
        let mut mesh_renderer = self.graphics.create_mesh_renderer(
            render_target.output_format(),
            get_projection(window.inner_size().into()),
        );
        mesh_renderer.set_view(
            Matrix4::from_translation(Vector3::new(0.0, 1.0, 3.0))
                .inverse_transform()
                .unwrap(),
        );

        // create renderables
        let material = graphics::Material {
            blend: graphics::material::BlendMode::Opaque,
        };
        let texture = self.graphics.create_texture(
            graphics::texture::Size::D2(Extent2d::new(2, 2)),
            graphics::texture::Format::Rgba8Unorm,
            Some(graphics::texture::Sampler::new(
                graphics::texture::FilterMode::Linear,
                graphics::texture::AddressMode::ClampToEdge,
            )),
            Some(&[
                0xFF, 0x00, 0x00, 0xFF, // red
                0x00, 0xFF, 0x00, 0xFF, // green
                0x00, 0x00, 0xFF, 0xFF, // blue
                0xFF, 0xFF, 0xFF, 0xFF, // white
            ]),
        )?;

        let t0 = std::time::Instant::now();
        let mut transform = Matrix4::identity();

        event_loop.run(move |event, _, flow| match event {
            // main events
            Event::NewEvents(_) => {}
            Event::MainEventsCleared => {
                let t1 = std::time::Instant::now();
                let t = t1.duration_since(t0).as_secs_f32();
                transform = Matrix4::from_angle_y(Deg(t * 30.0));
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
                mesh_renderer.draw_mesh(transform, &self.world.mesh(), material, Some(&texture));
                log_on_err!(render_target.draw_pass(Some(Color::black()), mesh_renderer.submit()));
            }
            _ => {}
        });
    }
}
