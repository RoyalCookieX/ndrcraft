mod game;
mod types;
mod voxel;

pub use game::Game;
pub use types::*;
pub use voxel::Voxel;

fn main() {
    let game = Game::new(game::Settings {
        window_mode: game::WindowMode::Windowed(Extent2d::new(1424, 720)),
        vsync: false,
        world_size: Extent3d::new(8, 4, 8),
    });
    game.run().expect("valid game loop");
}
