mod controller;
mod game;
mod graphics;
mod input;
mod performance;
mod types;
mod voxel;

pub use controller::Controller;
pub use game::Game;
pub use types::*;
pub use voxel::Voxel;

fn main() {
    env_logger::builder()
        .filter(None, log::LevelFilter::Info)
        .filter(Some("wgpu_hal"), log::LevelFilter::Warn)
        .filter(Some("wgpu_core"), log::LevelFilter::Warn)
        .init();

    let game = Game::new(game::Descriptor {
        window: game::WindowMode::Windowed(Extent2d::new(1424, 720)),
        vsync: false,
        world_size: Extent3d::new(100, 12, 100),
    })
    .expect("valid game");
    game.run().expect("valid game loop");
}
