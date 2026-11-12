mod draw;
pub mod logic;

use engine::{Canvas, Game, GameState, Key, Theme};
pub use logic::StarDefender;

impl Game for StarDefender {
    const WIDTH: f64 = logic::W;
    const HEIGHT: f64 = logic::H;

    fn new(seed: u64) -> Self {
        StarDefender::new(seed)
    }
    fn key(&mut self, key: Key, pressed: bool) -> bool {
        self.handle_key(key, pressed)
    }
    fn update(&mut self, dt_ms: f64) {
        StarDefender::update(self, dt_ms)
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
        StarDefender::pause(self)
    }
    fn pointer_down(&mut self, x: f64, y: f64) -> bool {
        StarDefender::pointer_down(self, x, y)
    }
    fn pointer_move(&mut self, x: f64, y: f64) {
        StarDefender::pointer_move(self, x, y)
    }
    fn pointer_up(&mut self, x: f64, y: f64) {
        StarDefender::pointer_up(self, x, y)
    }
    fn end_message(&self) -> Option<String> {
        StarDefender::end_message(self)
    }
}

engine::export_game!(StarDefender);
