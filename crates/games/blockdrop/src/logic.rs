//! Block Drop rules: four-cell shapes fall into a 12x18 well. Fill a row
//! completely to clear it. Every 10 rows cleared raises the level, and each
//! level makes the shapes fall faster. Every few shapes, one block is a bomb
//! that blasts a 3x3 hole when it lands. No browser code here, so it's testable.

use engine::{GameState, Key, Rng};
use std::collections::VecDeque;

pub const COLS: usize = 12;
pub const ROWS: usize = 18;
/// How many upcoming shapes the side panel shows.
pub const PREVIEW: usize = 3;
/// Every Nth shape carries a bomb.
pub const BOMB_EVERY: u32 = 8;

/// Holding left/right: wait this long, then repeat every ARR_MS.
const DAS_MS: f64 = 170.0;
const ARR_MS: f64 = 50.0;
/// Fall speed while the down key is held.
const SOFT_MS: f64 = 35.0;
/// A shape resting on something locks after this long...
const LOCK_MS: f64 = 500.0;
/// ...but moving or rotating it restarts that timer, up to this many times.
const MAX_LOCK_RESETS: u32 = 15;
/// Length of the flash when rows clear.
pub const CLEAR_MS: f64 = 320.0;
/// Length of the bomb's blast animation (play carries on during it).
pub const BLAST_MS: f64 = 450.0;
const POPUP_MS: f64 = 1200.0;
const LINES_PER_LEVEL: u32 = 10;
/// Points for clearing 1, 2, 3 or 4 rows at once, times the level.
const LINE_POINTS: [u32; 5] = [0, 100, 300, 500, 800];
/// Points per block a bomb destroys, times the level.
const BLAST_POINTS: u32 = 10;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    I,
    O,
    T,
    S,
    Z,
    J,
    L,
}

pub const KINDS: [Kind; 7] = [Kind::I, Kind::O, Kind::T, Kind::S, Kind::Z, Kind::J, Kind::L];

impl Kind {
    pub fn index(self) -> usize {
        self as usize
    }

    /// Side of the square box the shape rotates in.
    pub fn size(self) -> i32 {
        match self {
            Kind::I => 4,
            Kind::O => 2,
            _ => 3,
        }
    }

    fn base(self) -> [(i32, i32); 4] {
        match self {
            Kind::I => [(0, 1), (1, 1), (2, 1), (3, 1)],
            Kind::O => [(0, 0), (1, 0), (0, 1), (1, 1)],
            Kind::T => [(1, 0), (0, 1), (1, 1), (2, 1)],
            Kind::S => [(1, 0), (2, 0), (0, 1), (1, 1)],
            Kind::Z => [(0, 0), (1, 0), (1, 1), (2, 1)],
            Kind::J => [(0, 0), (0, 1), (1, 1), (2, 1)],
            Kind::L => [(2, 0), (0, 1), (1, 1), (2, 1)],
        }
    }

    /// Cells inside the shape's box after `rot` clockwise quarter turns.
    /// Each cell keeps its index, so a bomb stays on the same block.
    pub fn cells(self, rot: u8) -> [(i32, i32); 4] {
        let n = self.size();
        let mut c = self.base();
        for _ in 0..rot % 4 {
            for p in c.iter_mut() {
                *p = (n - 1 - p.1, p.0);
            }
        }
        c
    }
}

/// A shape waiting in the queue or the hold box.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Shape {
    pub kind: Kind,
    /// Which of the four cells (if any) is a bomb.
    pub bomb: Option<usize>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Piece {
    pub kind: Kind,
    pub bomb: Option<usize>,
    pub rot: u8,
    /// Top-left of the rotation box, in board cells.
    pub x: i32,
    pub y: i32,
}

impl Piece {
    fn spawn(s: Shape) -> Piece {
        // The I shape sits on row 1 of its box, so lift it to start on row 0.
        let y = if s.kind == Kind::I { -1 } else { 0 };
        Piece { kind: s.kind, bomb: s.bomb, rot: 0, x: (COLS as i32 - s.kind.size()) / 2, y }
    }

    fn shape(&self) -> Shape {
        Shape { kind: self.kind, bomb: self.bomb }
    }

    /// Board cells this piece covers (y may be negative above the well).
    pub fn blocks(&self) -> [(i32, i32); 4] {
        self.kind.cells(self.rot).map(|(cx, cy)| (self.x + cx, self.y + cy))
    }

    fn moved(self, dx: i32, dy: i32) -> Piece {
        Piece { x: self.x + dx, y: self.y + dy, ..self }
    }
}

/// Offsets tried, in order, when a rotation doesn't fit where it is.
const KICKS: [(i32, i32); 8] = [(0, 0), (-1, 0), (1, 0), (-2, 0), (2, 0), (0, -1), (-1, -1), (1, -1)];

pub struct BlockDrop {
    /// true where a block has settled.
    pub board: [[bool; COLS]; ROWS],
    pub piece: Piece,
    pub hold: Option<Shape>,
    /// Hold can be used once per shape.
    pub can_hold: bool,
    pub queue: VecDeque<Shape>,
    pub score: u32,
    pub lines: u32,
    pub level: u32,
    pub state: GameState,
    /// Rows flashing before they're removed; no piece is in play meanwhile.
    pub clearing: Vec<usize>,
    pub clear_ms: f64,
    /// Centre of the latest bomb blast, and time left on its animation.
    pub blast: Option<(i32, i32)>,
    pub blast_ms: f64,
    /// Short message like "Triple!" and how long it stays.
    pub popup: Option<&'static str>,
    pub popup_ms: f64,
    /// Seconds since creation, for animations.
    pub time: f64,
    dealt: u32,
    fall_acc: f64,
    lock_ms: f64,
    lock_resets: u32,
    left: bool,
    right: bool,
    down: bool,
    /// Direction being auto-repeated (-1, 0, 1) and time until the next step.
    das_dir: i32,
    das_ms: f64,
    rng: Rng,
}

impl BlockDrop {
    pub fn new(seed: u64) -> BlockDrop {
        let mut g = BlockDrop {
            board: [[false; COLS]; ROWS],
            piece: Piece::spawn(Shape { kind: Kind::O, bomb: None }),
            hold: None,
            can_hold: true,
            queue: VecDeque::new(),
            score: 0,
            lines: 0,
            level: 1,
            state: GameState::Ready,
            clearing: Vec::new(),
            clear_ms: 0.0,
            blast: None,
            blast_ms: 0.0,
            popup: None,
            popup_ms: 0.0,
            time: 0.0,
            dealt: 0,
            fall_acc: 0.0,
            lock_ms: 0.0,
            lock_resets: 0,
            left: false,
            right: false,
            down: false,
            das_dir: 0,
            das_ms: 0.0,
            rng: Rng::new(seed),
        };
        g.reset();
        g
    }

    pub fn reset(&mut self) {
        self.board = [[false; COLS]; ROWS];
        self.queue.clear();
        self.dealt = 0;
        self.hold = None;
        self.score = 0;
        self.lines = 0;
        self.level = 1;
        self.clearing.clear();
        self.blast = None;
        self.popup = None;
        self.release_keys();
        self.state = GameState::Ready;
        self.spawn_next();
    }

    /// Milliseconds per row at the current level.
    pub fn gravity_ms(&self) -> f64 {
        (800.0 * 0.82f64.powi(self.level as i32 - 1)).max(50.0)
    }

    pub fn fits(&self, p: &Piece) -> bool {
        p.blocks().iter().all(|&(x, y)| {
            (0..COLS as i32).contains(&x)
                && y < ROWS as i32
                && (y < 0 || !self.board[y as usize][x as usize])
        })
    }

    /// Where the piece would land if dropped now.
    pub fn landing(&self) -> Piece {
        let mut g = self.piece;
        while self.fits(&g.moved(0, 1)) {
            g = g.moved(0, 1);
        }
        g
    }

    fn grounded(&self) -> bool {
        !self.fits(&self.piece.moved(0, 1))
    }

    fn try_move(&mut self, dx: i32, dy: i32) -> bool {
        let p = self.piece.moved(dx, dy);
        let ok = self.fits(&p);
        if ok {
            self.piece = p;
        }
        ok
    }

    /// Moving or rotating a resting piece buys it a little more time.
    fn bump_lock(&mut self) {
        if self.lock_resets < MAX_LOCK_RESETS {
            self.lock_ms = 0.0;
            self.lock_resets += 1;
        }
    }

    fn shift(&mut self, dx: i32) -> bool {
        let ok = self.try_move(dx, 0);
        if ok {
            self.bump_lock();
        }
        ok
    }

    /// `dir` is 1 for clockwise, -1 for counter-clockwise.
    pub fn rotate(&mut self, dir: i32) -> bool {
        let rot = (self.piece.rot as i32 + 4 + dir).rem_euclid(4) as u8;
        let turned = Piece { rot, ..self.piece };
        for (dx, dy) in KICKS {
            let p = turned.moved(dx, dy);
            if self.fits(&p) {
                self.piece = p;
                self.bump_lock();
                return true;
            }
        }
        false
    }

    pub fn hard_drop(&mut self) {
        let mut rows = 0;
        while self.try_move(0, 1) {
            rows += 1;
        }
        self.score += 2 * rows;
        self.lock_piece();
    }

    pub fn hold_piece(&mut self) {
        if !self.can_hold {
            return;
        }
        let current = self.piece.shape();
        match self.hold.replace(current) {
            Some(s) => {
                self.piece = Piece::spawn(s);
                self.reset_piece_timers();
                if !self.fits(&self.piece) {
                    self.state = GameState::Over;
                }
            }
            None => self.spawn_next(),
        }
        self.can_hold = false;
    }

    fn refill(&mut self) {
        // "Bag" randomiser: every run of 7 shapes contains each shape once,
        // so you never wait forever for the one you need.
        while self.queue.len() <= PREVIEW {
            let mut bag = KINDS;
            for i in (1..bag.len()).rev() {
                bag.swap(i, self.rng.below(i + 1));
            }
            for kind in bag {
                self.dealt += 1;
                let bomb = self.dealt.is_multiple_of(BOMB_EVERY).then(|| self.rng.below(4));
                self.queue.push_back(Shape { kind, bomb });
            }
        }
    }

    fn reset_piece_timers(&mut self) {
        self.fall_acc = 0.0;
        self.lock_ms = 0.0;
        self.lock_resets = 0;
    }

    fn spawn_next(&mut self) {
        self.refill();
        let shape = self.queue.pop_front().expect("queue refilled");
        self.refill();
        self.piece = Piece::spawn(shape);
        self.can_hold = true;
        self.reset_piece_timers();
        if !self.fits(&self.piece) {
            self.state = GameState::Over;
        }
    }

    fn lock_piece(&mut self) {
        let blocks = self.piece.blocks();
        for &(x, y) in &blocks {
            if y >= 0 {
                self.board[y as usize][x as usize] = true;
            }
        }
        if blocks.iter().any(|&(_, y)| y < 0) {
            // Locked partly above the well: it's full.
            self.state = GameState::Over;
            return;
        }
        self.popup = None;
        if let Some(i) = self.piece.bomb {
            let (bx, by) = blocks[i];
            self.explode(bx, by);
        }
        let full: Vec<usize> = (0..ROWS).filter(|&r| self.board[r].iter().all(|&c| c)).collect();
        if full.is_empty() {
            self.spawn_next();
            return;
        }
        let n = full.len().min(4);
        self.score += LINE_POINTS[n] * self.level;
        self.lines += full.len() as u32;
        self.level = self.lines / LINES_PER_LEVEL + 1;
        self.popup = match n {
            2 => Some("Double!"),
            3 => Some("Triple!"),
            4 => Some("Four rows!"),
            _ => self.popup,
        };
        self.popup_ms = POPUP_MS;
        self.clearing = full;
        self.clear_ms = CLEAR_MS;
    }

    /// Clears the 3x3 square around (bx, by). Blocks resting above the hole
    /// in those three columns then fall to fill it.
    fn explode(&mut self, bx: i32, by: i32) {
        let mut destroyed = 0;
        for y in by - 1..=by + 1 {
            for x in bx - 1..=bx + 1 {
                if (0..COLS as i32).contains(&x) && (0..ROWS as i32).contains(&y) {
                    let cell = &mut self.board[y as usize][x as usize];
                    destroyed += *cell as u32;
                    *cell = false;
                }
            }
        }
        let bottom = (by + 1).min(ROWS as i32 - 1) as usize;
        for x in (bx - 1).max(0)..=(bx + 1).min(COLS as i32 - 1) {
            let x = x as usize;
            let filled = (0..=bottom).filter(|&y| self.board[y][x]).count();
            for y in 0..=bottom {
                self.board[y][x] = y + filled > bottom;
            }
        }
        self.score += BLAST_POINTS * destroyed * self.level;
        self.blast = Some((bx, by));
        self.blast_ms = BLAST_MS;
        self.popup = Some("Boom!");
        self.popup_ms = POPUP_MS;
    }

    fn finish_clear(&mut self) {
        let mut rows: Vec<[bool; COLS]> = (0..ROWS)
            .filter(|r| !self.clearing.contains(r))
            .map(|r| self.board[r])
            .collect();
        let empty = vec![[false; COLS]; ROWS - rows.len()];
        rows.splice(0..0, empty);
        self.board.copy_from_slice(&rows);
        self.clearing.clear();
        self.spawn_next();
    }

    fn release_keys(&mut self) {
        self.left = false;
        self.right = false;
        self.down = false;
        self.das_dir = 0;
    }

    fn start_repeat(&mut self, dir: i32) {
        self.das_dir = dir;
        self.das_ms = DAS_MS;
    }

    pub fn handle_key(&mut self, key: Key, pressed: bool) -> bool {
        let used = key.is_direction()
            || matches!(key, Key::Action | Key::Pause | Key::Letter('z' | 'x' | 'c'));
        match key {
            Key::Left => self.left = pressed,
            Key::Right => self.right = pressed,
            Key::Down => self.down = pressed,
            _ => {}
        }
        if !pressed {
            // Releasing one arrow while the other is held switches to it.
            if key == Key::Left && self.das_dir == -1 {
                if self.right { self.start_repeat(1) } else { self.das_dir = 0 }
            } else if key == Key::Right && self.das_dir == 1 {
                if self.left { self.start_repeat(-1) } else { self.das_dir = 0 }
            }
            return used;
        }
        if !used {
            return false;
        }
        match self.state {
            GameState::Ready => self.state = GameState::Playing,
            GameState::Paused => {
                if matches!(key, Key::Action | Key::Pause) {
                    self.state = GameState::Playing;
                }
            }
            GameState::Over => {
                if key == Key::Action {
                    self.reset();
                    self.state = GameState::Playing;
                }
            }
            GameState::Playing => {
                if key == Key::Pause {
                    self.state = GameState::Paused;
                    self.release_keys();
                    return true;
                }
                if !self.clearing.is_empty() {
                    return true;
                }
                match key {
                    Key::Left => {
                        self.shift(-1);
                        self.start_repeat(-1);
                    }
                    Key::Right => {
                        self.shift(1);
                        self.start_repeat(1);
                    }
                    Key::Down => {
                        if self.try_move(0, 1) {
                            self.score += 1;
                            self.fall_acc = 0.0;
                        }
                    }
                    Key::Up | Key::Letter('x') => {
                        self.rotate(1);
                    }
                    Key::Letter('z') => {
                        self.rotate(-1);
                    }
                    Key::Letter('c') => self.hold_piece(),
                    Key::Action => self.hard_drop(),
                    _ => {}
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
        if self.popup_ms > 0.0 {
            self.popup_ms -= dt_ms;
        }
        if self.blast_ms > 0.0 {
            self.blast_ms -= dt_ms;
            if self.blast_ms <= 0.0 {
                self.blast = None;
            }
        }
        if !self.clearing.is_empty() {
            self.clear_ms -= dt_ms;
            if self.clear_ms <= 0.0 {
                self.finish_clear();
            }
            return;
        }

        if self.das_dir != 0 {
            self.das_ms -= dt_ms;
            while self.das_ms <= 0.0 {
                self.das_ms += ARR_MS;
                if !self.shift(self.das_dir) {
                    break;
                }
            }
        }

        let gravity = self.gravity_ms();
        let interval = if self.down { gravity.min(SOFT_MS) } else { gravity };
        self.fall_acc += dt_ms;
        while self.fall_acc >= interval {
            self.fall_acc -= interval;
            if self.try_move(0, 1) {
                self.lock_ms = 0.0;
                if self.down {
                    self.score += 1;
                }
            } else {
                self.fall_acc = 0.0;
                break;
            }
        }

        if self.grounded() {
            self.lock_ms += dt_ms;
            if self.lock_ms >= LOCK_MS {
                self.lock_piece();
            }
        } else {
            self.lock_ms = 0.0;
        }
    }

    pub fn pause(&mut self) {
        if self.state == GameState::Playing {
            self.state = GameState::Paused;
        }
        // Key-up events can be lost when the window loses focus.
        self.release_keys();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn playing() -> BlockDrop {
        let mut g = BlockDrop::new(7);
        g.state = GameState::Playing;
        g
    }

    fn with_piece(kind: Kind) -> BlockDrop {
        let mut g = playing();
        g.piece = Piece::spawn(Shape { kind, bomb: None });
        g
    }

    fn press(g: &mut BlockDrop, key: Key) {
        g.handle_key(key, true);
        g.handle_key(key, false);
    }

    fn fill_row(g: &mut BlockDrop, row: usize, gap: std::ops::Range<usize>) {
        for x in 0..COLS {
            g.board[row][x] = !gap.contains(&x);
        }
    }

    #[test]
    fn four_turns_return_to_start() {
        for k in KINDS {
            assert_eq!(k.cells(0), k.cells(4));
            assert_ne!(k.cells(0), k.cells(1), "{k:?}");
        }
    }

    #[test]
    fn o_shape_is_the_same_every_way_round() {
        let mut a = Kind::O.cells(1);
        let mut b = Kind::O.cells(0);
        a.sort();
        b.sort();
        assert_eq!(a, b);
    }

    #[test]
    fn first_seven_shapes_are_all_different() {
        let g = BlockDrop::new(99);
        let mut seen = vec![g.piece.kind];
        seen.extend(g.queue.iter().take(6).map(|s| s.kind));
        seen.sort_by_key(|k| k.index());
        seen.dedup();
        assert_eq!(seen.len(), 7);
    }

    #[test]
    fn every_eighth_shape_has_a_bomb() {
        let mut g = BlockDrop::new(3);
        let mut shapes = vec![g.piece.shape()];
        while shapes.len() < 24 {
            g.spawn_next();
            shapes.push(g.piece.shape());
        }
        for (i, s) in shapes.iter().enumerate() {
            assert_eq!(s.bomb.is_some(), (i + 1) % BOMB_EVERY as usize == 0, "shape {i}");
        }
    }

    #[test]
    fn walls_stop_sideways_moves() {
        let mut g = with_piece(Kind::T);
        for _ in 0..20 {
            press(&mut g, Key::Left);
        }
        assert_eq!(g.piece.blocks().iter().map(|b| b.0).min(), Some(0));
    }

    #[test]
    fn holding_an_arrow_slides_to_the_wall() {
        let mut g = with_piece(Kind::T);
        g.handle_key(Key::Right, true);
        g.update(700.0);
        assert_eq!(g.piece.blocks().iter().map(|b| b.0).max(), Some(COLS as i32 - 1));
    }

    #[test]
    fn hard_drop_lands_scores_and_spawns_next() {
        let mut g = with_piece(Kind::O);
        let next = g.queue[0].kind;
        press(&mut g, Key::Action);
        assert!(g.board[ROWS - 1][5] && g.board[ROWS - 1][6]);
        assert!(g.board[ROWS - 2][5] && g.board[ROWS - 2][6]);
        assert_eq!(g.score, 2 * (ROWS as u32 - 2));
        assert_eq!(g.piece.kind, next);
    }

    #[test]
    fn resting_piece_locks_after_delay() {
        let mut g = with_piece(Kind::O);
        g.piece.y = ROWS as i32 - 2;
        g.update(LOCK_MS - 10.0);
        assert!(!g.board[ROWS - 1][5]);
        g.update(20.0);
        assert!(g.board[ROWS - 1][5]);
    }

    #[test]
    fn full_row_clears_after_flash() {
        let mut g = with_piece(Kind::I);
        fill_row(&mut g, ROWS - 1, 4..8);
        g.board[ROWS - 2][0] = true; // sits above the cleared row
        g.hard_drop();
        assert_eq!(g.clearing, vec![ROWS - 1]);
        assert_eq!(g.lines, 1);
        assert_eq!(g.score, 2 * (ROWS as u32 - 1) + 100);
        g.update(CLEAR_MS + 1.0);
        assert!(g.clearing.is_empty());
        assert!(g.board[ROWS - 1][0], "row above drops down");
        assert!(g.board[ROWS - 1][1..].iter().all(|&c| !c));
    }

    #[test]
    fn four_rows_at_once_score_800() {
        let mut g = with_piece(Kind::I);
        g.piece.rot = 1; // vertical, occupies column x+2
        g.piece.x = -2;
        for r in ROWS - 4..ROWS {
            fill_row(&mut g, r, 0..1);
        }
        g.hard_drop();
        assert_eq!(g.lines, 4);
        assert_eq!(g.popup, Some("Four rows!"));
        assert_eq!(g.score, 2 * (ROWS as u32 - 3) + 800, "starts one row above the well");
    }

    #[test]
    fn ten_rows_raise_level_and_speed() {
        let mut g = with_piece(Kind::I);
        let slow = g.gravity_ms();
        g.lines = 9;
        fill_row(&mut g, ROWS - 1, 4..8);
        g.hard_drop();
        assert_eq!(g.level, 2);
        assert!(g.gravity_ms() < slow);
    }

    #[test]
    fn bomb_blasts_a_square_and_blocks_above_fall() {
        let mut g = with_piece(Kind::O);
        g.piece.bomb = Some(2); // bottom-left block of the O
        // A pile two rows high, with a tower in column 3 standing above the blast.
        for y in ROWS - 2..ROWS {
            fill_row(&mut g, y, 0..1);
        }
        for y in ROWS - 6..ROWS - 2 {
            g.board[y][3] = true;
        }
        g.piece.x = 4;
        g.score = 0;
        g.hard_drop();
        // O landed on rows ROWS-4..ROWS-3 at columns 4-5; the bomb is at (4, ROWS-3).
        assert_eq!(g.blast, Some((4, ROWS as i32 - 3)));
        assert_eq!(g.popup, Some("Boom!"));
        // Columns 3-5 lost the 3x3 square around the bomb, rows ROWS-4..=ROWS-2.
        assert!(!g.board[ROWS - 2][4] && !g.board[ROWS - 2][5]);
        assert!(g.board[ROWS - 1][4], "row below the blast is untouched");
        // Column 3: the tower's two cells above the blast fell to the hole's bottom.
        assert!(g.board[ROWS - 2][3] && g.board[ROWS - 3][3]);
        assert!(!g.board[ROWS - 4][3]);
        // Hard drop of 14 rows, plus 9 destroyed blocks (2 tower, 3 pile, 4 of the O).
        assert_eq!(g.score, 2 * 14 + 9 * BLAST_POINTS);
    }

    #[test]
    fn rotating_against_a_wall_kicks_away() {
        let mut g = with_piece(Kind::I);
        g.piece.rot = 1;
        g.piece.x = -2; // vertical bar hugging the left wall
        g.piece.y = 5;
        assert!(g.rotate(1));
        assert!(g.piece.blocks().iter().all(|b| b.0 >= 0));
    }

    #[test]
    fn hold_swaps_once_per_shape() {
        let mut g = with_piece(Kind::T);
        let next = g.queue[0].kind;
        g.hold_piece();
        assert_eq!(g.hold.map(|s| s.kind), Some(Kind::T));
        assert_eq!(g.piece.kind, next);
        g.hold_piece();
        assert_eq!(g.piece.kind, next, "second hold ignored");
        g.hard_drop();
        g.hold_piece();
        assert_eq!(g.piece.kind, Kind::T);
    }

    #[test]
    fn blocked_spawn_ends_game() {
        let mut g = playing();
        for r in 0..ROWS {
            fill_row(&mut g, r, 0..1);
        }
        g.board[0][0] = false;
        g.spawn_next();
        assert_eq!(g.state, GameState::Over);
    }

    #[test]
    fn space_restarts_after_game_over() {
        let mut g = playing();
        g.state = GameState::Over;
        g.score = 1234;
        g.board[ROWS - 1][0] = true;
        press(&mut g, Key::Action);
        assert_eq!(g.state, GameState::Playing);
        assert_eq!(g.score, 0);
        assert!(!g.board[ROWS - 1][0]);
    }

    #[test]
    fn pause_releases_held_keys() {
        let mut g = with_piece(Kind::T);
        g.handle_key(Key::Left, true);
        g.pause();
        g.handle_key(Key::Action, true);
        let x = g.piece.x;
        g.update(500.0);
        assert_eq!(g.piece.x, x);
    }
}
