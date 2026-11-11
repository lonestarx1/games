//! Snake rules. No browser code here, so it's all unit-testable.

use engine::{GameState, Key, Rng};
use std::collections::VecDeque;

pub const COLS: i32 = 24;
pub const ROWS: i32 = 24;
pub const CELL: f64 = 20.0;

const START_STEP_MS: f64 = 130.0;
const MIN_STEP_MS: f64 = 65.0;
/// Each food makes the snake a little faster (1.5%), down to MIN_STEP_MS.
const SPEEDUP: f64 = 0.985;
const POINTS_PER_FOOD: u32 = 10;
/// Turns typed faster than the snake moves are queued, up to this many.
const MAX_QUEUED_TURNS: usize = 2;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Cell {
    pub x: i32,
    pub y: i32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Dir {
    Up,
    Down,
    Left,
    Right,
}

impl Dir {
    pub fn delta(self) -> (i32, i32) {
        match self {
            Dir::Up => (0, -1),
            Dir::Down => (0, 1),
            Dir::Left => (-1, 0),
            Dir::Right => (1, 0),
        }
    }

    fn opposite(self) -> Dir {
        match self {
            Dir::Up => Dir::Down,
            Dir::Down => Dir::Up,
            Dir::Left => Dir::Right,
            Dir::Right => Dir::Left,
        }
    }

    fn from_key(key: Key) -> Option<Dir> {
        match key {
            Key::Up => Some(Dir::Up),
            Key::Down => Some(Dir::Down),
            Key::Left => Some(Dir::Left),
            Key::Right => Some(Dir::Right),
            _ => None,
        }
    }
}

pub struct Snake {
    /// Front is the head.
    pub body: VecDeque<Cell>,
    pub dir: Dir,
    queued: VecDeque<Dir>,
    pub food: Cell,
    pub score: u32,
    pub state: GameState,
    pub step_ms: f64,
    acc_ms: f64,
    /// Seconds since creation, for animations.
    pub time: f64,
    rng: Rng,
}

impl Snake {
    pub fn new(seed: u64) -> Snake {
        let mut s = Snake {
            body: VecDeque::new(),
            dir: Dir::Right,
            queued: VecDeque::new(),
            food: Cell { x: 0, y: 0 },
            score: 0,
            state: GameState::Ready,
            step_ms: START_STEP_MS,
            acc_ms: 0.0,
            time: 0.0,
            rng: Rng::new(seed),
        };
        s.reset();
        s
    }

    pub fn reset(&mut self) {
        let (cx, cy) = (COLS / 2, ROWS / 2);
        self.body = (0..4).map(|i| Cell { x: cx - i, y: cy }).collect();
        self.dir = Dir::Right;
        self.queued.clear();
        self.score = 0;
        self.step_ms = START_STEP_MS;
        self.acc_ms = 0.0;
        self.state = GameState::Ready;
        self.spawn_food();
    }

    pub fn head(&self) -> Cell {
        self.body[0]
    }

    /// Queue a turn. Reversing into yourself or repeating a direction is ignored.
    pub fn turn(&mut self, d: Dir) {
        let last = self.queued.back().copied().unwrap_or(self.dir);
        if d != last && d != last.opposite() && self.queued.len() < MAX_QUEUED_TURNS {
            self.queued.push_back(d);
        }
    }

    pub fn handle_key(&mut self, key: Key, pressed: bool) -> bool {
        let used = key.is_direction() || matches!(key, Key::Action | Key::Pause);
        if !pressed || !used {
            return used;
        }
        let dir = Dir::from_key(key);
        match self.state {
            GameState::Ready | GameState::Paused => {
                if let Some(d) = dir {
                    self.turn(d);
                }
                self.state = GameState::Playing;
            }
            GameState::Playing => match dir {
                Some(d) => self.turn(d),
                None => self.state = GameState::Paused,
            },
            GameState::Over => {
                if key == Key::Action {
                    self.reset();
                    self.state = GameState::Playing;
                }
            }
        }
        true
    }

    pub fn update(&mut self, dt_ms: f64) {
        self.time += dt_ms / 1000.0;
        if self.state != GameState::Playing {
            return;
        }
        self.acc_ms += dt_ms;
        while self.acc_ms >= self.step_ms && self.state == GameState::Playing {
            self.acc_ms -= self.step_ms;
            self.step();
        }
    }

    /// Advance one grid cell.
    pub fn step(&mut self) {
        if let Some(d) = self.queued.pop_front() {
            self.dir = d;
        }
        let (dx, dy) = self.dir.delta();
        let h = self.head();
        // Walls wrap around to the other side.
        let next = Cell {
            x: (h.x + dx).rem_euclid(COLS),
            y: (h.y + dy).rem_euclid(ROWS),
        };
        let grows = next == self.food;
        // The tail moves out of the way this step unless we're growing.
        let solid = if grows { self.body.len() } else { self.body.len() - 1 };
        if self.body.iter().take(solid).any(|c| *c == next) {
            self.state = GameState::Over;
            return;
        }
        self.body.push_front(next);
        if grows {
            self.score += POINTS_PER_FOOD;
            self.step_ms = (self.step_ms * SPEEDUP).max(MIN_STEP_MS);
            self.spawn_food();
        } else {
            self.body.pop_back();
        }
    }

    fn spawn_food(&mut self) {
        let free: Vec<Cell> = (0..ROWS)
            .flat_map(|y| (0..COLS).map(move |x| Cell { x, y }))
            .filter(|c| !self.body.contains(c))
            .collect();
        if free.is_empty() {
            // Board full: you win, which ends the game.
            self.state = GameState::Over;
            return;
        }
        self.food = free[self.rng.below(free.len())];
    }

    pub fn pause(&mut self) {
        if self.state == GameState::Playing {
            self.state = GameState::Paused;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn playing() -> Snake {
        let mut s = Snake::new(42);
        s.state = GameState::Playing;
        s.food = Cell { x: 0, y: 0 };
        s
    }

    #[test]
    fn moves_right_by_default() {
        let mut s = playing();
        let h = s.head();
        s.step();
        assert_eq!(s.head(), Cell { x: h.x + 1, y: h.y });
        assert_eq!(s.body.len(), 4);
    }

    #[test]
    fn wraps_through_walls() {
        let mut s = playing();
        s.body = VecDeque::from(vec![Cell { x: COLS - 1, y: 5 }, Cell { x: COLS - 2, y: 5 }]);
        s.step();
        assert_eq!(s.head(), Cell { x: 0, y: 5 });
        s.turn(Dir::Up);
        s.body = VecDeque::from(vec![Cell { x: 3, y: 0 }, Cell { x: 3, y: 1 }]);
        s.step();
        assert_eq!(s.head(), Cell { x: 3, y: ROWS - 1 });
    }

    #[test]
    fn cannot_reverse() {
        let mut s = playing();
        s.turn(Dir::Left);
        let h = s.head();
        s.step();
        assert_eq!(s.head().x, h.x + 1);
    }

    #[test]
    fn quick_double_turn_is_queued_not_reversed() {
        let mut s = playing();
        s.turn(Dir::Up);
        s.turn(Dir::Left);
        let h = s.head();
        s.step();
        assert_eq!(s.head(), Cell { x: h.x, y: h.y - 1 });
        s.step();
        assert_eq!(s.head(), Cell { x: h.x - 1, y: h.y - 1 });
        assert_eq!(s.state, GameState::Playing);
    }

    #[test]
    fn eating_grows_scores_and_speeds_up() {
        let mut s = playing();
        let h = s.head();
        s.food = Cell { x: h.x + 1, y: h.y };
        s.step();
        assert_eq!(s.body.len(), 5);
        assert_eq!(s.score, POINTS_PER_FOOD);
        assert!(s.step_ms < START_STEP_MS);
        assert!(!s.body.contains(&s.food));
    }

    #[test]
    fn biting_yourself_ends_game() {
        let mut s = playing();
        // A loop where turning down runs into the body.
        s.body = VecDeque::from(vec![
            Cell { x: 5, y: 5 },
            Cell { x: 4, y: 5 },
            Cell { x: 4, y: 6 },
            Cell { x: 5, y: 6 },
            Cell { x: 6, y: 6 },
        ]);
        s.dir = Dir::Right;
        s.turn(Dir::Down);
        s.step();
        assert_eq!(s.state, GameState::Over);
    }

    #[test]
    fn moving_into_the_tail_cell_is_safe() {
        let mut s = playing();
        s.body = VecDeque::from(vec![
            Cell { x: 5, y: 5 },
            Cell { x: 5, y: 6 },
            Cell { x: 6, y: 6 },
            Cell { x: 6, y: 5 },
        ]);
        s.dir = Dir::Up;
        s.turn(Dir::Right);
        s.step();
        assert_eq!(s.state, GameState::Playing);
    }

    #[test]
    fn space_restarts_after_game_over() {
        let mut s = playing();
        s.state = GameState::Over;
        s.score = 50;
        s.handle_key(Key::Action, true);
        assert_eq!(s.state, GameState::Playing);
        assert_eq!(s.score, 0);
    }
}
