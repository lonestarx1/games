mod draw;
pub mod logic;

use engine::{Canvas, Game, GameState, Key, Theme};
pub use logic::BlockDrop;

impl Game for BlockDrop {
    const WIDTH: f64 = draw::W;
    const HEIGHT: f64 = draw::H;

    fn new(seed: u64) -> Self {
        BlockDrop::new(seed)
    }
    fn key(&mut self, key: Key, pressed: bool) -> bool {
        self.handle_key(key, pressed)
    }
    fn update(&mut self, dt_ms: f64) {
        BlockDrop::update(self, dt_ms)
    }
    fn render(&self, canvas: &Canvas, theme: Theme) {
        match theme {
            Theme::Retro => draw::retro(self, canvas),
            Theme::Modern => draw::modern(self, canvas),
        }
    }
    fn score(&self) -> u32 {
        self.score
    }
    fn state(&self) -> GameState {
        self.state
    }
    fn pause(&mut self) {
        BlockDrop::pause(self)
    }
}

engine::export_game!(BlockDrop);
