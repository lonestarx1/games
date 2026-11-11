mod draw;
pub mod logic;

use engine::{Canvas, Game, GameState, Key, Theme};
pub use logic::Snake;

impl Game for Snake {
    const WIDTH: f64 = logic::COLS as f64 * logic::CELL;
    const HEIGHT: f64 = logic::ROWS as f64 * logic::CELL;

    fn new(seed: u64) -> Self {
        Snake::new(seed)
    }
    fn key(&mut self, key: Key, pressed: bool) -> bool {
        self.handle_key(key, pressed)
    }
    fn update(&mut self, dt_ms: f64) {
        Snake::update(self, dt_ms)
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
        Snake::pause(self)
    }
}

engine::export_game!(Snake);
