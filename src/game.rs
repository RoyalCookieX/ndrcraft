use crate::Extent2d;
use winit::{
    dpi::PhysicalPosition,
    error::OsError,
    event::{Event, WindowEvent},
    event_loop::EventLoop,
    window::WindowBuilder,
};

#[derive(Clone, Copy, Debug)]
pub enum WindowMode {
    Windowed(Extent2d<u32>),
}

#[derive(Clone, Copy, Debug)]
pub struct Settings {
    pub window_mode: WindowMode,
    pub vsync: bool,
}

#[derive(Debug)]
pub enum Error {
    CreateWindowFailed(OsError),
}

pub struct Game {
    settings: Settings,
}

impl Game {
    pub fn new(settings: Settings) -> Self {
        Self { settings }
    }

    pub fn run(self) -> Result<(), Error> {
        // create event_loop and window from settings
        let event_loop = EventLoop::new();
        let window = {
            let mut builder = WindowBuilder::new().with_title("NdrCraft");
            builder = match self.settings.window_mode {
                WindowMode::Windowed(size) => builder.with_inner_size(size),
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

        event_loop.run(move |event, _, flow| match event {
            // main events
            Event::NewEvents(_) => {}
            Event::MainEventsCleared => {}
            Event::LoopDestroyed => {}

            // window events
            Event::WindowEvent { window_id, event } if window_id == window.id() => match event {
                WindowEvent::CloseRequested => {
                    flow.set_exit();
                    return;
                }
                _ => {}
            },
            Event::RedrawRequested(window_id) if window_id == window.id() => {}
            _ => {}
        });
    }
}
