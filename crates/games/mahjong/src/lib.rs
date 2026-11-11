mod draw;
pub mod layout;
pub mod logic;
pub mod tiles;

use engine::{Canvas, Game, GameState, Key, Theme};
pub use logic::{Mahjong, Outcome};

impl Game for Mahjong {
    const WIDTH: f64 = layout::W;
    const HEIGHT: f64 = layout::H;

    fn new(seed: u64) -> Self {
        Mahjong::new(seed)
    }
    fn key(&mut self, key: Key, pressed: bool) -> bool {
        self.handle_key(key, pressed)
    }
    fn update(&mut self, dt_ms: f64) {
        Mahjong::update(self, dt_ms)
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
        Mahjong::pause(self)
    }
    fn pointer_down(&mut self, x: f64, y: f64) -> bool {
        Mahjong::pointer_down(self, x, y)
    }
    fn pointer_move(&mut self, x: f64, y: f64) {
        Mahjong::pointer_move(self, x, y)
    }
    fn end_message(&self) -> Option<String> {
        self.outcome.map(|o| match o {
            Outcome::Cleared => "Board cleared!".to_string(),
            Outcome::Stuck => "No moves left".to_string(),
        })
    }
}

engine::export_game!(Mahjong);
