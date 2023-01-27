mod game;
mod types;

pub use game::Game;
pub use types::*;

fn main() {
    let game = Game::new(game::Settings {
        window_mode: game::WindowMode::Windowed(Extent2d::new(1424, 720)),
        vsync: false,
    });
    game.run().expect("valid game loop");
}
