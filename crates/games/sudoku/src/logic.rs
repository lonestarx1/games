//! Sudoku rules and puzzle generator. Fill the 9x9 grid so every row,
//! column and 3x3 box holds 1-9 once. Every generated puzzle has exactly one
//! solution. No browser code here, so it's all unit-testable.

use engine::{GameState, Key, Rng};

pub const W: f64 = 468.0;
pub const H: f64 = 580.0;
pub const BAR_H: f64 = 44.0;
pub const CELL: f64 = 48.0;
pub const GRID_X: f64 = (W - 9.0 * CELL) / 2.0;
pub const GRID_Y: f64 = BAR_H + 4.0;
pub const PAD_Y: f64 = GRID_Y + 9.0 * CELL + 18.0;
pub const PAD_H: f64 = 60.0;

/// Celebration before the "solved" overlay appears.
const END_PAUSE_MS: f64 = 1800.0;
const HINT_PENALTY: i32 = 150;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Difficulty {
    Easy,
    Medium,
    Hard,
}

impl Difficulty {
    /// Clues left on the board (Hard may end up with a few more if no more
    /// can be removed without allowing a second solution).
    fn clues(self) -> usize {
        match self {
            Difficulty::Easy => 38,
            Difficulty::Medium => 30,
            Difficulty::Hard => 24,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Difficulty::Easy => "Easy",
            Difficulty::Medium => "Medium",
            Difficulty::Hard => "Hard",
        }
    }

    fn base_points(self) -> i32 {
        match self {
            Difficulty::Easy => 1000,
            Difficulty::Medium => 2000,
            Difficulty::Hard => 3000,
        }
    }

    fn par_seconds(self) -> f64 {
        match self {
            Difficulty::Easy => 600.0,
            Difficulty::Medium => 900.0,
            Difficulty::Hard => 1500.0,
        }
    }
}

pub fn row_of(i: usize) -> usize {
    i / 9
}
pub fn col_of(i: usize) -> usize {
    i % 9
}
pub fn box_of(i: usize) -> usize {
    (i / 27) * 3 + (i % 9) / 3
}

/// The 20 other cells sharing a row, column or box with `i`.
pub fn peers(i: usize) -> impl Iterator<Item = usize> {
    (0..81).filter(move |&j| j != i && (row_of(j) == row_of(i) || col_of(j) == col_of(i) || box_of(j) == box_of(i)))
}

// ---------------------------------------------------------------- solver

/// Digits (as bits 1..=9) that can still go in cell `i`.
fn candidates(grid: &[u8; 81], i: usize) -> u16 {
    let mut used = 0u16;
    for j in peers(i) {
        used |= 1 << grid[j];
    }
    !used & 0b11_1111_1110
}

/// Counts solutions, stopping at `limit`. Leaves `grid` as it found it.
fn count_solutions(grid: &mut [u8; 81], limit: usize) -> usize {
    // Most constrained empty cell first.
    let mut best: Option<(usize, u16)> = None;
    for i in 0..81 {
        if grid[i] == 0 {
            let c = candidates(grid, i);
            if c == 0 {
                return 0;
            }
            if best.is_none_or(|(_, b)| c.count_ones() < b.count_ones()) {
                best = Some((i, c));
            }
        }
    }
    let Some((i, cands)) = best else { return 1 };
    let mut found = 0;
    for d in 1..=9u8 {
        if cands & (1 << d) != 0 {
            grid[i] = d;
            found += count_solutions(grid, limit - found);
            if found >= limit {
                break;
            }
        }
    }
    grid[i] = 0;
    found
}

/// Fills an empty grid with a random complete solution.
fn fill_random(grid: &mut [u8; 81], rng: &mut Rng) -> bool {
    let Some(i) = grid.iter().position(|&v| v == 0) else { return true };
    let cands = candidates(grid, i);
    let mut digits: Vec<u8> = (1..=9).filter(|d| cands & (1 << d) != 0).collect();
    for k in (1..digits.len()).rev() {
        digits.swap(k, rng.below(k + 1));
    }
    for d in digits {
        grid[i] = d;
        if fill_random(grid, rng) {
            return true;
        }
    }
    grid[i] = 0;
    false
}

/// A puzzle with exactly one solution, and that solution.
pub fn generate(diff: Difficulty, rng: &mut Rng) -> ([u8; 81], [u8; 81]) {
    let mut solution = [0u8; 81];
    fill_random(&mut solution, rng);
    let mut puzzle = solution;
    // Remove clues in symmetric pairs, keeping the solution unique.
    let mut order: Vec<usize> = (0..41).collect();
    for k in (1..order.len()).rev() {
        order.swap(k, rng.below(k + 1));
    }
    let mut clues = 81;
    for i in order {
        let pair = [i, 80 - i];
        let removing = if i == 40 { 1 } else { 2 };
        if clues - removing < diff.clues() {
            continue;
        }
        let saved = (puzzle[pair[0]], puzzle[pair[1]]);
        puzzle[pair[0]] = 0;
        puzzle[pair[1]] = 0;
        let mut probe = puzzle;
        if count_solutions(&mut probe, 2) == 1 {
            clues -= removing;
        } else {
            puzzle[pair[0]] = saved.0;
            puzzle[pair[1]] = saved.1;
        }
    }
    (puzzle, solution)
}

// ---------------------------------------------------------------- game

#[derive(Clone)]
struct Snapshot {
    values: [u8; 81],
    notes: [u16; 81],
}

pub struct Sudoku {
    pub diff: Difficulty,
    pub given: [bool; 81],
    pub values: [u8; 81],
    /// Pencil marks: bit d set means d is noted.
    pub notes: [u16; 81],
    solution: [u8; 81],
    pub selected: usize,
    pub notes_mode: bool,
    pub hints: u32,
    pub score: u32,
    pub state: GameState,
    pub solved: bool,
    pub elapsed: f64,
    pub time: f64,
    pub solved_at: f64,
    started: bool,
    end_ms: f64,
    history: Vec<Snapshot>,
    rng: Rng,
}

impl Sudoku {
    pub fn new(seed: u64) -> Sudoku {
        let mut g = Sudoku {
            diff: Difficulty::Easy,
            given: [false; 81],
            values: [0; 81],
            notes: [0; 81],
            solution: [0; 81],
            selected: 40,
            notes_mode: false,
            hints: 0,
            score: 0,
            state: GameState::Playing,
            solved: false,
            elapsed: 0.0,
            time: 0.0,
            solved_at: 0.0,
            started: false,
            end_ms: 0.0,
            history: Vec::new(),
            rng: Rng::new(seed),
        };
        g.new_game(Difficulty::Easy);
        g
    }

    pub fn new_game(&mut self, diff: Difficulty) {
        let (puzzle, solution) = generate(diff, &mut self.rng);
        self.load(diff, puzzle, solution);
    }

    fn load(&mut self, diff: Difficulty, puzzle: [u8; 81], solution: [u8; 81]) {
        self.diff = diff;
        self.values = puzzle;
        self.given = puzzle.map(|v| v != 0);
        self.notes = [0; 81];
        self.solution = solution;
        self.selected = 40;
        self.notes_mode = false;
        self.hints = 0;
        self.score = 0;
        self.state = GameState::Playing;
        self.solved = false;
        self.elapsed = 0.0;
        self.started = false;
        self.end_ms = 0.0;
        self.history.clear();
    }

    fn active(&self) -> bool {
        self.state == GameState::Playing && !self.solved
    }

    /// Is the number in cell `i` repeated in its row, column or box?
    pub fn conflict(&self, i: usize) -> bool {
        let v = self.values[i];
        v != 0 && peers(i).any(|j| self.values[j] == v)
    }

    /// How many of digit `d` are on the board.
    pub fn count(&self, d: u8) -> usize {
        self.values.iter().filter(|&&v| v == d).count()
    }

    fn snapshot(&mut self) {
        self.history.push(Snapshot { values: self.values, notes: self.notes });
        self.started = true;
    }

    /// Puts digit `d` in the selected cell (or toggles it as a note).
    pub fn enter(&mut self, d: u8) {
        let i = self.selected;
        if !self.active() || self.given[i] || !(1..=9).contains(&d) {
            return;
        }
        if self.notes_mode {
            if self.values[i] != 0 {
                return;
            }
            self.snapshot();
            self.notes[i] ^= 1 << d;
            return;
        }
        self.snapshot();
        if self.values[i] == d {
            self.values[i] = 0; // typing the same digit again clears it
            return;
        }
        self.values[i] = d;
        self.notes[i] = 0;
        // Placing a number rubs out that note around it.
        for j in peers(i) {
            self.notes[j] &= !(1 << d);
        }
        self.check_solved();
    }

    pub fn erase(&mut self) {
        let i = self.selected;
        if !self.active() || self.given[i] || (self.values[i] == 0 && self.notes[i] == 0) {
            return;
        }
        self.snapshot();
        self.values[i] = 0;
        self.notes[i] = 0;
    }

    pub fn undo(&mut self) {
        if !self.active() {
            return;
        }
        if let Some(s) = self.history.pop() {
            self.values = s.values;
            self.notes = s.notes;
        }
    }

    /// Fixes a wrong number if there is one; otherwise fills the selected
    /// cell (or the first empty one) with the right answer.
    pub fn hint(&mut self) {
        if !self.active() {
            return;
        }
        let wrong = (0..81).find(|&i| self.values[i] != 0 && self.values[i] != self.solution[i]);
        let target = wrong.or_else(|| {
            let sel = self.selected;
            if self.values[sel] == 0 { Some(sel) } else { (0..81).find(|&i| self.values[i] == 0) }
        });
        let Some(i) = target else { return };
        self.snapshot();
        self.selected = i;
        self.values[i] = self.solution[i];
        self.notes[i] = 0;
        let d = self.solution[i];
        for j in peers(i) {
            self.notes[j] &= !(1 << d);
        }
        self.hints += 1;
        self.check_solved();
    }

    fn check_solved(&mut self) {
        if self.values == self.solution {
            self.solved = true;
            self.solved_at = self.time;
            let time_bonus = ((self.diff.par_seconds() - self.elapsed).max(0.0) * 2.0) as i32;
            let points = self.diff.base_points() + time_bonus - self.hints as i32 * HINT_PENALTY;
            self.score = points.max(100) as u32;
            self.end_ms = END_PAUSE_MS;
        }
    }

    fn move_sel(&mut self, dx: i32, dy: i32) {
        let r = (row_of(self.selected) as i32 + dy).rem_euclid(9);
        let c = (col_of(self.selected) as i32 + dx).rem_euclid(9);
        self.selected = (r * 9 + c) as usize;
    }

    pub fn handle_key(&mut self, key: Key, pressed: bool) -> bool {
        let used = key.is_direction()
            || matches!(key, Key::Action | Key::Pause | Key::Digit(_) | Key::Erase | Key::Letter('n' | 'z' | 'h' | 'e' | 'm' | 'x'));
        if !pressed || !used {
            return used;
        }
        let new = match key {
            Key::Letter('e') => Some(Difficulty::Easy),
            Key::Letter('m') => Some(Difficulty::Medium),
            Key::Letter('x') => Some(Difficulty::Hard),
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
            GameState::Ready | GameState::Playing => match key {
                Key::Up => self.move_sel(0, -1),
                Key::Down => self.move_sel(0, 1),
                Key::Left => self.move_sel(-1, 0),
                Key::Right => self.move_sel(1, 0),
                Key::Digit(0) | Key::Erase => self.erase(),
                Key::Digit(d) => self.enter(d),
                Key::Letter('n') => self.notes_mode = !self.notes_mode,
                Key::Letter('z') => self.undo(),
                Key::Letter('h') => self.hint(),
                Key::Pause => self.pause(),
                _ => {}
            },
        }
        true
    }

    /// Number pad button under a point: 1-9, or 0 for none.
    pub fn pad_at(x: f64, y: f64) -> u8 {
        if !(PAD_Y..PAD_Y + PAD_H).contains(&y) {
            return 0;
        }
        let k = ((x - GRID_X) / CELL).floor();
        if (0.0..9.0).contains(&k) { k as u8 + 1 } else { 0 }
    }

    pub fn pointer_down(&mut self, x: f64, y: f64) -> bool {
        if !self.active() {
            return false;
        }
        let (cx, cy) = ((x - GRID_X) / CELL, (y - GRID_Y) / CELL);
        if (0.0..9.0).contains(&cx) && (0.0..9.0).contains(&cy) {
            self.selected = cy as usize * 9 + cx as usize;
            return true;
        }
        match Sudoku::pad_at(x, y) {
            0 => false,
            d => {
                self.enter(d);
                true
            }
        }
    }

    pub fn update(&mut self, dt_ms: f64) {
        self.time += dt_ms / 1000.0;
        if self.state != GameState::Playing {
            return;
        }
        if self.solved {
            self.end_ms -= dt_ms;
            if self.end_ms <= 0.0 {
                self.state = GameState::Over;
            }
        } else if self.started {
            self.elapsed += dt_ms / 1000.0;
        }
    }

    pub fn pause(&mut self) {
        if self.state == GameState::Playing && !self.solved && self.started {
            self.state = GameState::Paused;
        }
    }

    pub fn end_message(&self) -> Option<String> {
        let s = self.elapsed.ceil() as u32;
        Some(format!("Solved in {}:{:02}!", s / 60, s % 60))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn valid_solution(g: &[u8; 81]) -> bool {
        (0..81).all(|i| g[i] != 0 && peers(i).all(|j| g[j] != g[i]))
    }

    /// A game with a known puzzle: the solution minus a few cells.
    fn known(empty: &[usize]) -> Sudoku {
        let mut g = Sudoku::new(1);
        let solution = g.solution;
        let mut puzzle = solution;
        for &i in empty {
            puzzle[i] = 0;
        }
        g.load(Difficulty::Easy, puzzle, solution);
        g
    }

    #[test]
    fn peers_are_twenty_cells() {
        assert_eq!(peers(0).count(), 20);
        assert!(peers(0).any(|j| j == 10), "same box");
        assert!(!peers(0).any(|j| j == 13));
    }

    #[test]
    fn generated_puzzles_have_exactly_one_solution() {
        let mut rng = Rng::new(42);
        for diff in [Difficulty::Easy, Difficulty::Medium, Difficulty::Hard] {
            let (puzzle, solution) = generate(diff, &mut rng);
            assert!(valid_solution(&solution));
            let clues = puzzle.iter().filter(|&&v| v != 0).count();
            assert!(clues >= diff.clues() && clues <= diff.clues() + 8, "{diff:?}: {clues} clues");
            for i in 0..81 {
                assert!(puzzle[i] == 0 || puzzle[i] == solution[i]);
            }
            let mut probe = puzzle;
            assert_eq!(count_solutions(&mut probe, 2), 1, "{diff:?}");
        }
    }

    #[test]
    fn givens_cannot_be_changed() {
        let mut g = known(&[0]);
        g.selected = 1;
        let before = g.values[1];
        g.enter(if before == 9 { 1 } else { before + 1 });
        g.erase();
        assert_eq!(g.values[1], before);
    }

    #[test]
    fn entering_and_clearing_a_number() {
        let mut g = known(&[0, 1]);
        g.selected = 0;
        g.enter(5);
        assert_eq!(g.values[0], 5);
        g.enter(5);
        assert_eq!(g.values[0], 0, "same digit again clears");
        g.enter(5);
        g.handle_key(Key::Erase, true);
        assert_eq!(g.values[0], 0);
    }

    #[test]
    fn conflicts_are_detected() {
        let mut g = known(&[0, 1]);
        g.selected = 0;
        let clash = g.values[2]; // already in row 0
        g.enter(clash);
        assert!(g.conflict(0));
        assert!(g.conflict(2));
    }

    #[test]
    fn notes_toggle_and_get_cleaned_up() {
        let mut g = known(&[0, 1]);
        g.notes_mode = true;
        g.selected = 1;
        g.enter(4);
        g.enter(7);
        assert_eq!(g.notes[1], (1 << 4) | (1 << 7));
        g.enter(4);
        assert_eq!(g.notes[1], 1 << 7);
        // Placing a 7 next door removes the 7 note.
        g.notes_mode = false;
        g.selected = 0;
        g.enter(7);
        assert_eq!(g.notes[1], 0);
    }

    #[test]
    fn undo_steps_back() {
        let mut g = known(&[0]);
        g.selected = 0;
        g.enter(3);
        g.enter(4);
        g.undo();
        assert_eq!(g.values[0], 3);
        g.undo();
        assert_eq!(g.values[0], 0);
    }

    #[test]
    fn hint_fixes_mistakes_first() {
        let mut g = known(&[0, 40]);
        let right = g.solution[40];
        g.selected = 40;
        g.enter(if right == 9 { 1 } else { right + 1 });
        g.selected = 0;
        g.hint();
        assert_eq!(g.values[40], right);
        assert_eq!(g.hints, 1);
        g.hint();
        assert_eq!(g.values[0], g.solution[0]);
        assert!(g.solved);
    }

    #[test]
    fn solving_scores_and_ends() {
        let mut g = known(&[10]);
        g.selected = 10;
        g.elapsed = 100.0;
        g.enter(g.solution[10]);
        assert!(g.solved);
        assert_eq!(g.score, (1000.0 + (600.0 - 100.0) * 2.0) as u32);
        g.update(END_PAUSE_MS + 10.0);
        assert_eq!(g.state, GameState::Over);
        assert!(g.end_message().unwrap().starts_with("Solved"));
    }

    #[test]
    fn hints_cost_points() {
        let mut g = known(&[10]);
        g.elapsed = 700.0; // past par: no time bonus
        g.hint();
        assert_eq!(g.score, (1000 - HINT_PENALTY) as u32);
    }

    #[test]
    fn number_keys_and_pad_enter_digits() {
        let mut g = known(&[0, 1]);
        g.selected = 0;
        g.handle_key(Key::Digit(6), true);
        assert_eq!(g.values[0], 6);
        // Tap cell 1, then the pad's "2".
        g.pointer_down(GRID_X + CELL * 1.5, GRID_Y + CELL * 0.5);
        assert_eq!(g.selected, 1);
        g.pointer_down(GRID_X + CELL * 1.5, PAD_Y + 10.0);
        assert_eq!(g.values[1], 2);
    }

    #[test]
    fn arrows_wrap_around_the_grid() {
        let mut g = known(&[]);
        g.selected = 0;
        g.handle_key(Key::Left, true);
        assert_eq!(g.selected, 8);
        g.handle_key(Key::Up, true);
        assert_eq!(g.selected, 80);
    }
}
