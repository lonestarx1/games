mod draw;
pub mod logic;

use engine::{Canvas, Game, GameState, Key, Theme};
pub use logic::Sudoku;

impl Game for Sudoku {
    const WIDTH: f64 = logic::W;
    const HEIGHT: f64 = logic::H;

    fn new(seed: u64) -> Self {
        Sudoku::new(seed)
    }
    fn key(&mut self, key: Key, pressed: bool) -> bool {
        self.handle_key(key, pressed)
    }
    fn update(&mut self, dt_ms: f64) {
        Sudoku::update(self, dt_ms)
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
        Sudoku::pause(self)
    }
    fn pointer_down(&mut self, x: f64, y: f64) -> bool {
        Sudoku::pointer_down(self, x, y)
    }
    fn end_message(&self) -> Option<String> {
        Sudoku::end_message(self)
    }
}

engine::export_game!(Sudoku);
