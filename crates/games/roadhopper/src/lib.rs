mod draw;
pub mod logic;

use engine::{Canvas, Game, GameState, Key, Theme};
pub use logic::RoadHopper;

impl Game for RoadHopper {
    const WIDTH: f64 = logic::W;
    const HEIGHT: f64 = logic::H;

    fn new(seed: u64) -> Self {
        RoadHopper::new(seed)
    }
    fn key(&mut self, key: Key, pressed: bool) -> bool {
        self.handle_key(key, pressed)
    }
    fn update(&mut self, dt_ms: f64) {
        RoadHopper::update(self, dt_ms)
    }
    fn render(&self, canvas: &Canvas, theme: Theme) {
        match theme {
            Theme::Retro => draw::retro(self, canvas),
            Theme::Modern => draw::modern(self, canvas),
        }
    }
    fn score(&self) -> u32 {
        RoadHopper::score(self)
    }
    fn state(&self) -> GameState {
        self.state
    }
    fn pause(&mut self) {
        RoadHopper::pause(self)
    }
    fn pointer_down(&mut self, x: f64, y: f64) -> bool {
        RoadHopper::pointer_down(self, x, y)
    }
    fn pointer_up(&mut self, x: f64, y: f64) {
        RoadHopper::pointer_up(self, x, y)
    }
    fn end_message(&self) -> Option<String> {
        RoadHopper::end_message(self)
    }
}

engine::export_game!(RoadHopper);
