//! Mine Finder rules: open every square that isn't hiding a mine. Numbers
//! count the mines touching a square. The first square opened is always safe.
//! No browser code here, so it's all unit-testable.

use engine::{GameState, Key, Rng};
use std::collections::VecDeque;

pub const W: f64 = 560.0;
pub const H: f64 = 600.0;
/// Strip across the top of the canvas for the mine counter and timer.
pub const BAR_H: f64 = 56.0;
const MARGIN: f64 = 16.0;

/// How long the board stays visible after the game ends, before the
/// "game over" overlay covers it.
const END_PAUSE_MS: f64 = 1400.0;
const WIN_BONUS: u32 = 1000;
/// Bonus points per second under par when you win.
const TIME_BONUS: u32 = 5;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Difficulty {
    Easy,
    Medium,
    Hard,
}

impl Difficulty {
    /// (columns, rows, mines)
    pub fn spec(self) -> (usize, usize, usize) {
        match self {
            Difficulty::Easy => (9, 9, 10),
            Difficulty::Medium => (16, 16, 40),
            Difficulty::Hard => (24, 20, 99),
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Difficulty::Easy => "Easy",
            Difficulty::Medium => "Medium",
            Difficulty::Hard => "Hard",
        }
    }

    /// Harder boards are worth more per square.
    pub fn multiplier(self) -> u32 {
        self as u32 + 1
    }

    /// Seconds; finishing faster earns a time bonus.
    fn par(self) -> f64 {
        match self {
            Difficulty::Easy => 120.0,
            Difficulty::Medium => 420.0,
            Difficulty::Hard => 900.0,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Cell {
    pub mine: bool,
    /// Mines in the 8 surrounding squares.
    pub near: u8,
    pub open: bool,
    pub flag: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Outcome {
    Won,
    /// The index of the mine that was opened.
    Lost(usize),
}

/// Where the board sits on the canvas.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Layout {
    pub cell: f64,
    pub ox: f64,
    pub oy: f64,
}

pub struct MineFinder {
    pub diff: Difficulty,
    pub cols: usize,
    pub rows: usize,
    pub mines: usize,
    pub cells: Vec<Cell>,
    /// Mines are placed on the first open, away from that square.
    pub placed: bool,
    pub flags: usize,
    pub opened: usize,
    pub score: u32,
    pub state: GameState,
    pub outcome: Option<Outcome>,
    /// Seconds of play on this board (starts at the first open).
    pub elapsed: f64,
    /// Seconds since creation, for animations.
    pub time: f64,
    /// When the game ended, for the reveal animation.
    pub ended_at: f64,
    end_ms: f64,
    /// Keyboard cursor, shown after an arrow key is used.
    pub cursor: usize,
    pub show_cursor: bool,
    pub hover: Option<usize>,
    /// Touch helper: taps place flags instead of opening squares.
    pub flag_mode: bool,
    rng: Rng,
}

impl MineFinder {
    pub fn new(seed: u64) -> MineFinder {
        let mut g = MineFinder {
            diff: Difficulty::Easy,
            cols: 0,
            rows: 0,
            mines: 0,
            cells: Vec::new(),
            placed: false,
            flags: 0,
            opened: 0,
            score: 0,
            state: GameState::Playing,
            outcome: None,
            elapsed: 0.0,
            time: 0.0,
            ended_at: 0.0,
            end_ms: 0.0,
            cursor: 0,
            show_cursor: false,
            hover: None,
            flag_mode: false,
            rng: Rng::new(seed),
        };
        g.new_game(Difficulty::Easy);
        g
    }

    pub fn new_game(&mut self, diff: Difficulty) {
        let (cols, rows, mines) = diff.spec();
        self.diff = diff;
        self.cols = cols;
        self.rows = rows;
        self.mines = mines;
        self.cells = vec![Cell::default(); cols * rows];
        self.placed = false;
        self.flags = 0;
        self.opened = 0;
        self.score = 0;
        self.state = GameState::Playing;
        self.outcome = None;
        self.elapsed = 0.0;
        self.end_ms = 0.0;
        self.cursor = (rows / 2) * cols + cols / 2;
        self.hover = None;
    }

    pub fn layout(&self) -> Layout {
        let cell = ((W - 2.0 * MARGIN) / self.cols as f64)
            .min((H - BAR_H - 2.0 * MARGIN) / self.rows as f64)
            .floor();
        Layout {
            cell,
            ox: ((W - cell * self.cols as f64) / 2.0).round(),
            oy: (BAR_H + (H - BAR_H - cell * self.rows as f64) / 2.0).round(),
        }
    }

    pub fn cell_at(&self, x: f64, y: f64) -> Option<usize> {
        let l = self.layout();
        let (cx, cy) = ((x - l.ox) / l.cell, (y - l.oy) / l.cell);
        if cx < 0.0 || cy < 0.0 || cx >= self.cols as f64 || cy >= self.rows as f64 {
            return None;
        }
        Some(cy as usize * self.cols + cx as usize)
    }

    pub fn xy(&self, i: usize) -> (usize, usize) {
        (i % self.cols, i / self.cols)
    }

    pub fn neighbours(&self, i: usize) -> impl Iterator<Item = usize> + '_ {
        let (x, y) = self.xy(i);
        let (x, y) = (x as i32, y as i32);
        (-1..=1)
            .flat_map(move |dy| (-1..=1).map(move |dx| (x + dx, y + dy)))
            .filter(move |&(nx, ny)| {
                (nx, ny) != (x, y)
                    && (0..self.cols as i32).contains(&nx)
                    && (0..self.rows as i32).contains(&ny)
            })
            .map(move |(nx, ny)| ny as usize * self.cols + nx as usize)
    }

    pub fn mines_left(&self) -> i32 {
        self.mines as i32 - self.flags as i32
    }

    fn active(&self) -> bool {
        self.state == GameState::Playing && self.outcome.is_none()
    }

    /// Puts the mines anywhere except the 3x3 block around `safe`, so the
    /// first square opened is always a blank that opens up an area.
    fn place_mines(&mut self, safe: usize) {
        let keep_clear: Vec<usize> = self.neighbours(safe).chain([safe]).collect();
        let mut spots: Vec<usize> = (0..self.cells.len()).filter(|i| !keep_clear.contains(i)).collect();
        // Tiny boards can't keep a whole 3x3 clear; then only `safe` is protected.
        if spots.len() < self.mines {
            spots = (0..self.cells.len()).filter(|&i| i != safe).collect();
        }
        for k in 0..self.mines {
            let j = k + self.rng.below(spots.len() - k);
            spots.swap(k, j);
            self.cells[spots[k]].mine = true;
        }
        self.count_neighbours();
        self.placed = true;
    }

    fn count_neighbours(&mut self) {
        for i in 0..self.cells.len() {
            let n = self.neighbours(i).filter(|&j| self.cells[j].mine).count();
            self.cells[i].near = n as u8;
        }
    }

    /// Opens a square. Blank squares (no mines nearby) open their neighbours too.
    pub fn open(&mut self, i: usize) {
        if !self.active() || self.cells[i].open || self.cells[i].flag {
            return;
        }
        if !self.placed {
            self.place_mines(i);
        }
        if self.cells[i].mine {
            self.lose(i);
            return;
        }
        let mut queue = VecDeque::from([i]);
        while let Some(j) = queue.pop_front() {
            let c = &mut self.cells[j];
            if c.open || c.flag {
                continue;
            }
            c.open = true;
            self.opened += 1;
            self.score += self.diff.multiplier();
            if c.near == 0 {
                let next: Vec<usize> = self.neighbours(j).filter(|&n| !self.cells[n].open).collect();
                queue.extend(next);
            }
        }
        if self.opened == self.cells.len() - self.mines {
            self.win();
        }
    }

    /// On an open number with that many flags around it, opens the rest of
    /// its neighbours. A wrong flag means this opens a mine.
    pub fn chord(&mut self, i: usize) {
        let c = self.cells[i];
        if !self.active() || !c.open || c.near == 0 {
            return;
        }
        let flags = self.neighbours(i).filter(|&j| self.cells[j].flag).count();
        if flags != c.near as usize {
            return;
        }
        let todo: Vec<usize> = self.neighbours(i).filter(|&j| !self.cells[j].open && !self.cells[j].flag).collect();
        for j in todo {
            self.open(j);
        }
    }

    pub fn toggle_flag(&mut self, i: usize) {
        if !self.active() || self.cells[i].open {
            return;
        }
        let c = &mut self.cells[i];
        c.flag = !c.flag;
        if c.flag {
            self.flags += 1;
        } else {
            self.flags -= 1;
        }
    }

    /// Primary action on a square: open it, flag it in flag mode, or chord a number.
    pub fn act(&mut self, i: usize) {
        if self.cells[i].open {
            self.chord(i);
        } else if self.flag_mode {
            self.toggle_flag(i);
        } else {
            self.open(i);
        }
    }

    fn lose(&mut self, i: usize) {
        self.outcome = Some(Outcome::Lost(i));
        self.cells[i].open = true;
        self.end();
    }

    fn win(&mut self) {
        self.outcome = Some(Outcome::Won);
        // Flag every mine for the victory board.
        for c in self.cells.iter_mut().filter(|c| c.mine) {
            c.flag = true;
        }
        self.flags = self.mines;
        let m = self.diff.multiplier();
        let under_par = (self.diff.par() - self.elapsed).max(0.0) as u32;
        self.score += WIN_BONUS * m + under_par * TIME_BONUS * m;
        self.end();
    }

    fn end(&mut self) {
        self.ended_at = self.time;
        self.end_ms = END_PAUSE_MS;
    }

    fn move_cursor(&mut self, dx: i32, dy: i32) {
        let (x, y) = self.xy(self.cursor);
        let nx = (x as i32 + dx).clamp(0, self.cols as i32 - 1) as usize;
        let ny = (y as i32 + dy).clamp(0, self.rows as i32 - 1) as usize;
        self.cursor = ny * self.cols + nx;
        self.show_cursor = true;
    }

    pub fn handle_key(&mut self, key: Key, pressed: bool) -> bool {
        let used = key.is_direction()
            || matches!(key, Key::Action | Key::Pause | Key::Letter('e' | 'm' | 'h' | 'n' | 'f' | 't'));
        if !pressed || !used {
            return used;
        }
        // Board size and new game work from any state.
        let new = match key {
            Key::Letter('e') => Some(Difficulty::Easy),
            Key::Letter('m') => Some(Difficulty::Medium),
            Key::Letter('h') => Some(Difficulty::Hard),
            Key::Letter('n') => Some(self.diff),
            _ => None,
        };
        if let Some(d) = new {
            self.new_game(d);
            return true;
        }
        match self.state {
            GameState::Paused => {
                if matches!(key, Key::Action | Key::Pause) {
                    self.state = GameState::Playing;
                }
            }
            GameState::Over => {
                if key == Key::Action {
                    self.new_game(self.diff);
                }
            }
            GameState::Ready | GameState::Playing => {
                if key == Key::Pause {
                    if self.outcome.is_none() {
                        self.state = GameState::Paused;
                    }
                    return true;
                }
                match key {
                    Key::Up => self.move_cursor(0, -1),
                    Key::Down => self.move_cursor(0, 1),
                    Key::Left => self.move_cursor(-1, 0),
                    Key::Right => self.move_cursor(1, 0),
                    Key::Action => {
                        self.show_cursor = true;
                        if self.cells[self.cursor].open {
                            self.chord(self.cursor);
                        } else {
                            self.open(self.cursor);
                        }
                    }
                    Key::Letter('f') => {
                        self.show_cursor = true;
                        self.toggle_flag(self.cursor);
                    }
                    Key::Letter('t') => self.flag_mode = !self.flag_mode,
                    _ => {}
                }
            }
        }
        true
    }

    pub fn pointer_down(&mut self, x: f64, y: f64) -> bool {
        let Some(i) = self.cell_at(x, y) else { return false };
        self.cursor = i;
        self.show_cursor = false;
        self.act(i);
        true
    }

    pub fn pointer_alt(&mut self, x: f64, y: f64) -> bool {
        let Some(i) = self.cell_at(x, y) else { return false };
        self.cursor = i;
        self.show_cursor = false;
        if self.cells[i].open {
            self.chord(i);
        } else {
            self.toggle_flag(i);
        }
        true
    }

    pub fn pointer_move(&mut self, x: f64, y: f64) {
        self.hover = self.cell_at(x, y);
        if self.hover.is_some() {
            self.show_cursor = false;
        }
    }

    pub fn update(&mut self, dt_ms: f64) {
        self.time += dt_ms / 1000.0;
        if self.state != GameState::Playing {
            return;
        }
        if self.outcome.is_some() {
            self.end_ms -= dt_ms;
            if self.end_ms <= 0.0 {
                self.state = GameState::Over;
            }
        } else if self.placed {
            self.elapsed += dt_ms / 1000.0;
        }
    }

    pub fn pause(&mut self) {
        if self.state == GameState::Playing && self.outcome.is_none() && self.placed {
            self.state = GameState::Paused;
        }
    }

    pub fn end_message(&self) -> Option<String> {
        match self.outcome? {
            Outcome::Won => Some(format!("Cleared in {}s!", self.elapsed.ceil() as u32)),
            Outcome::Lost(_) => Some("Boom! You hit a mine".into()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Easy board with mines exactly where we say.
    fn board(mines: &[usize]) -> MineFinder {
        let mut g = MineFinder::new(1);
        g.mines = mines.len();
        for &m in mines {
            g.cells[m].mine = true;
        }
        g.count_neighbours();
        g.placed = true;
        g
    }

    fn idx(x: usize, y: usize) -> usize {
        y * 9 + x
    }

    #[test]
    fn first_open_is_safe_with_clear_surroundings() {
        for seed in 0..50 {
            let mut g = MineFinder::new(seed);
            let c = idx(4, 4);
            g.open(c);
            assert!(g.outcome.is_none(), "seed {seed}");
            assert_eq!(g.cells.iter().filter(|c| c.mine).count(), 10);
            assert!(g.neighbours(c).all(|j| !g.cells[j].mine));
            assert_eq!(g.cells[c].near, 0);
            assert!(g.opened >= 9, "a blank opens its neighbours");
        }
    }

    #[test]
    fn numbers_count_touching_mines() {
        let g = board(&[idx(0, 0), idx(2, 0), idx(1, 2)]);
        assert_eq!(g.cells[idx(1, 1)].near, 3);
        assert_eq!(g.cells[idx(1, 0)].near, 2);
        assert_eq!(g.cells[idx(8, 8)].near, 0);
    }

    #[test]
    fn blank_area_floods_open_up_to_numbers() {
        let mut g = board(&[idx(0, 0)]);
        g.open(idx(8, 8));
        // Everything except the mine opens: the only number is around (0,0).
        assert_eq!(g.opened, 80);
        assert_eq!(g.outcome, Some(Outcome::Won));
    }

    #[test]
    fn flags_block_opening_and_count_down() {
        let mut g = board(&[idx(0, 0), idx(8, 8)]);
        g.toggle_flag(idx(3, 3));
        assert_eq!(g.mines_left(), 1);
        g.open(idx(3, 3));
        assert!(!g.cells[idx(3, 3)].open);
        g.toggle_flag(idx(3, 3));
        assert_eq!(g.mines_left(), 2);
    }

    #[test]
    fn opening_a_mine_loses_then_shows_game_over() {
        let mut g = board(&[idx(0, 0), idx(8, 8)]);
        g.open(idx(0, 0));
        assert_eq!(g.outcome, Some(Outcome::Lost(idx(0, 0))));
        assert_eq!(g.state, GameState::Playing, "board stays visible for a moment");
        g.toggle_flag(idx(5, 5));
        assert!(!g.cells[idx(5, 5)].flag, "no moves after losing");
        g.update(END_PAUSE_MS + 1.0);
        assert_eq!(g.state, GameState::Over);
        assert_eq!(g.end_message().as_deref(), Some("Boom! You hit a mine"));
    }

    #[test]
    fn chord_opens_neighbours_when_flags_match() {
        let mut g = board(&[idx(0, 0), idx(8, 8)]);
        g.cells[idx(1, 1)].open = true;
        g.toggle_flag(idx(0, 0));
        g.chord(idx(1, 1));
        assert!(g.cells[idx(1, 0)].open && g.cells[idx(2, 2)].open);
        assert!(g.outcome.is_none());
    }

    #[test]
    fn chord_with_wrong_flag_hits_the_mine() {
        let mut g = board(&[idx(0, 0), idx(8, 8)]);
        g.cells[idx(1, 1)].open = true;
        g.toggle_flag(idx(2, 2));
        g.chord(idx(1, 1));
        assert_eq!(g.outcome, Some(Outcome::Lost(idx(0, 0))));
    }

    #[test]
    fn chord_needs_enough_flags() {
        let mut g = board(&[idx(0, 0), idx(8, 8)]);
        g.cells[idx(1, 1)].open = true;
        g.chord(idx(1, 1));
        assert!(!g.cells[idx(1, 0)].open);
    }

    #[test]
    fn winning_scores_bonus_and_flags_mines() {
        let mut g = board(&[idx(0, 0)]);
        g.elapsed = 20.0;
        g.open(idx(8, 8));
        assert_eq!(g.outcome, Some(Outcome::Won));
        assert!(g.cells[idx(0, 0)].flag);
        assert_eq!(g.score, 80 + WIN_BONUS + 100 * TIME_BONUS);
        assert_eq!(g.end_message().as_deref(), Some("Cleared in 20s!"));
    }

    #[test]
    fn clock_starts_on_first_open() {
        let mut g = MineFinder::new(5);
        g.update(1000.0);
        assert_eq!(g.elapsed, 0.0);
        g.open(idx(4, 4));
        g.update(1000.0);
        assert!((g.elapsed - 1.0).abs() < 1e-9);
    }

    #[test]
    fn difficulty_keys_start_new_boards() {
        let mut g = MineFinder::new(5);
        g.handle_key(Key::Letter('h'), true);
        assert_eq!((g.cols, g.rows, g.mines), (24, 20, 99));
        assert_eq!(g.cells.len(), 480);
        g.open(0);
        assert_eq!(g.cells.iter().filter(|c| c.mine).count(), 99);
        g.handle_key(Key::Letter('m'), true);
        assert_eq!(g.cells.len(), 256);
        assert!(!g.placed);
    }

    #[test]
    fn every_board_fits_the_canvas() {
        for d in [Difficulty::Easy, Difficulty::Medium, Difficulty::Hard] {
            let mut g = MineFinder::new(1);
            g.new_game(d);
            let l = g.layout();
            assert!(l.cell >= 20.0, "{d:?} cells too small");
            assert!(l.ox >= MARGIN - 1.0 && l.ox + l.cell * g.cols as f64 <= W - MARGIN + 1.0);
            assert!(l.oy >= BAR_H && l.oy + l.cell * g.rows as f64 <= H);
            // Clicking a cell centre finds that cell.
            let last = g.cells.len() - 1;
            let (x, y) = g.xy(last);
            let hit = g.cell_at(l.ox + (x as f64 + 0.5) * l.cell, l.oy + (y as f64 + 0.5) * l.cell);
            assert_eq!(hit, Some(last));
        }
    }

    #[test]
    fn keyboard_moves_cursor_and_opens() {
        let mut g = MineFinder::new(9);
        let start = g.cursor;
        g.handle_key(Key::Right, true);
        assert_eq!(g.cursor, start + 1);
        g.handle_key(Key::Action, true);
        assert!(g.cells[start + 1].open);
    }

    #[test]
    fn flag_mode_turns_taps_into_flags() {
        let mut g = board(&[idx(0, 0), idx(8, 8)]);
        g.handle_key(Key::Letter('t'), true);
        g.act(idx(4, 4));
        assert!(g.cells[idx(4, 4)].flag);
        assert!(!g.cells[idx(4, 4)].open);
    }

    #[test]
    fn space_after_game_over_starts_again() {
        let mut g = board(&[idx(0, 0)]);
        g.open(idx(0, 0));
        g.update(END_PAUSE_MS + 1.0);
        g.handle_key(Key::Action, true);
        assert_eq!(g.state, GameState::Playing);
        assert!(g.outcome.is_none() && !g.placed);
    }
}
