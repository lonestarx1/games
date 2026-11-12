//! Number Merge rules: slide every tile one way; two equal tiles that run
//! into each other merge into their sum. A new 2 (sometimes a 4) appears
//! after every move. The game ends when no move changes the board.
//! No browser code here, so it's all unit-testable.

use engine::{GameState, Key, Rng};

pub const W: f64 = 480.0;
pub const H: f64 = 536.0;
/// Strip above the board for best tile and undos left.
pub const BAR_H: f64 = 56.0;
pub const BOARD: f64 = 448.0;
pub const BOARD_X: f64 = (W - BOARD) / 2.0;
pub const BOARD_Y: f64 = BAR_H + 16.0;

/// Tile slide time, then the "pop" of new and merged tiles.
pub const SLIDE_MS: f64 = 110.0;
pub const POP_MS: f64 = 200.0;
/// Pause on the final board before "game over" covers it.
const END_PAUSE_MS: f64 = 900.0;
pub const UNDOS: u32 = 3;
pub const GOAL: u32 = 2048;
/// Chance that a new tile is a 4 instead of a 2.
const FOUR_CHANCE: f64 = 0.1;
/// Minimum swipe length, in canvas pixels.
const SWIPE_MIN: f64 = 24.0;
/// Merge burst: particle lifetime, shockwave lifetime, board shake.
pub const SPARK_MS: f64 = 650.0;
pub const RING_MS: f64 = 380.0;
const SHAKE_MS: f64 = 220.0;
/// Merges this big or bigger shake the board.
const SHAKE_FROM: u32 = 64;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Dir {
    Up,
    Down,
    Left,
    Right,
}

impl Dir {
    /// (row, col) step.
    fn step(self) -> (f64, f64) {
        match self {
            Dir::Up => (-1.0, 0.0),
            Dir::Down => (1.0, 0.0),
            Dir::Left => (0.0, -1.0),
            Dir::Right => (0.0, 1.0),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TileKind {
    Moved,
    Spawned,
    Merged,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Tile {
    pub value: u32,
    pub row: usize,
    pub col: usize,
    /// Where it started its last move, in cells (fractional while it was
    /// being dragged), for the slide animation.
    pub from: (f64, f64),
    pub kind: TileKind,
}

impl Tile {
    fn at(&self) -> (f64, f64) {
        (self.row as f64, self.col as f64)
    }
}

/// A position in cells (row, col), fractional while moving.
type Cellf = (f64, f64);

/// The result of sliding one way, before it's applied.
struct Plan {
    tiles: Vec<Tile>,
    ghosts: Vec<Tile>,
    gained: u32,
    changed: bool,
}

/// A flying spark from a merge, in cell units.
pub struct Spark {
    pub y: f64,
    pub x: f64,
    pub vy: f64,
    pub vx: f64,
    pub size: f64,
    pub value: u32,
    /// Milliseconds alive; negative while waiting for the slide to finish.
    pub age: f64,
}

/// Expanding shockwave where two tiles merged.
pub struct Ring {
    pub row: usize,
    pub col: usize,
    pub value: u32,
    pub age: f64,
}

pub struct NumberMerge {
    pub size: usize,
    pub tiles: Vec<Tile>,
    /// Tiles that merged away: drawn sliding into their partner, then gone.
    pub ghosts: Vec<Tile>,
    pub score: u32,
    pub state: GameState,
    /// Milliseconds since the last move, drives the animations.
    pub anim_ms: f64,
    pub undos_left: u32,
    history: Option<(Vec<Tile>, u32)>,
    pub reached_goal: bool,
    /// Shown briefly, e.g. "2048!".
    pub popup: Option<String>,
    pub popup_ms: f64,
    pub sparks: Vec<Spark>,
    pub rings: Vec<Ring>,
    pub shake_ms: f64,
    pub shake_power: f64,
    end_ms: f64,
    /// Pointer press position and where it is now, while dragging.
    drag: Option<((f64, f64), (f64, f64))>,
    rng: Rng,
}

impl NumberMerge {
    pub fn new(seed: u64) -> NumberMerge {
        let mut g = NumberMerge {
            size: 4,
            tiles: Vec::new(),
            ghosts: Vec::new(),
            score: 0,
            state: GameState::Playing,
            anim_ms: f64::MAX,
            undos_left: UNDOS,
            history: None,
            reached_goal: false,
            popup: None,
            popup_ms: 0.0,
            sparks: Vec::new(),
            rings: Vec::new(),
            shake_ms: 0.0,
            shake_power: 0.0,
            end_ms: 0.0,
            drag: None,
            rng: Rng::new(seed),
        };
        g.new_game(4);
        g
    }

    pub fn new_game(&mut self, size: usize) {
        self.size = size;
        self.tiles.clear();
        self.ghosts.clear();
        self.sparks.clear();
        self.rings.clear();
        self.score = 0;
        self.state = GameState::Playing;
        self.undos_left = UNDOS;
        self.history = None;
        self.reached_goal = false;
        self.popup = None;
        self.end_ms = 0.0;
        self.anim_ms = 0.0;
        self.drag = None;
        self.spawn();
        self.spawn();
    }

    /// Gap between cells and cell size, in canvas pixels.
    pub fn metrics(&self) -> (f64, f64) {
        let gap = if self.size == 4 { 12.0 } else { 9.0 };
        (gap, (BOARD - gap * (self.size as f64 + 1.0)) / self.size as f64)
    }

    pub fn best_tile(&self) -> u32 {
        self.tiles.iter().map(|t| t.value).max().unwrap_or(0)
    }

    /// Value at each cell (0 = empty).
    pub fn grid(&self) -> Vec<Vec<u32>> {
        let mut g = vec![vec![0; self.size]; self.size];
        for t in &self.tiles {
            g[t.row][t.col] = t.value;
        }
        g
    }

    fn spawn(&mut self) {
        let grid = self.grid();
        let empty: Vec<(usize, usize)> = (0..self.size)
            .flat_map(|r| (0..self.size).map(move |c| (r, c)))
            .filter(|&(r, c)| grid[r][c] == 0)
            .collect();
        if empty.is_empty() {
            return;
        }
        let (row, col) = empty[self.rng.below(empty.len())];
        let value = if self.rng.unit() < FOUR_CHANCE { 4 } else { 2 };
        self.tiles.push(Tile { value, row, col, from: (row as f64, col as f64), kind: TileKind::Spawned });
    }

    /// Cells of line `k`, ordered from the edge the tiles slide toward.
    fn line(&self, dir: Dir, k: usize) -> Vec<(usize, usize)> {
        let n = self.size;
        (0..n)
            .map(|i| match dir {
                Dir::Left => (k, i),
                Dir::Right => (k, n - 1 - i),
                Dir::Up => (i, k),
                Dir::Down => (n - 1 - i, k),
            })
            .collect()
    }

    /// Works out a slide without applying it.
    fn plan(&self, dir: Dir) -> Plan {
        let mut at = vec![vec![None; self.size]; self.size];
        for (i, t) in self.tiles.iter().enumerate() {
            at[t.row][t.col] = Some(i);
        }
        let mut plan = Plan { tiles: Vec::new(), ghosts: Vec::new(), gained: 0, changed: false };
        for k in 0..self.size {
            let cells = self.line(dir, k);
            // Group the line's tiles into output slots; a pair of equal
            // neighbours fills one slot (and only merges once per move).
            let mut slots: Vec<(u32, Vec<usize>)> = Vec::new();
            for &(r, c) in &cells {
                let Some(i) = at[r][c] else { continue };
                let v = self.tiles[i].value;
                match slots.last_mut() {
                    Some((sv, src)) if src.len() == 1 && *sv == v => {
                        *sv *= 2;
                        src.push(i);
                    }
                    _ => slots.push((v, vec![i])),
                }
            }
            for (j, (value, src)) in slots.into_iter().enumerate() {
                let (row, col) = cells[j];
                let moved = |t: &Tile| Tile { row, col, from: t.at(), kind: TileKind::Moved, ..*t };
                if let [i] = src[..] {
                    let t = self.tiles[i];
                    plan.changed |= (t.row, t.col) != (row, col);
                    plan.tiles.push(moved(&t));
                } else {
                    plan.changed = true;
                    plan.gained += value;
                    for &i in &src {
                        plan.ghosts.push(moved(&self.tiles[i]));
                    }
                    let from = (row as f64, col as f64);
                    plan.tiles.push(Tile { value, row, col, from, kind: TileKind::Merged });
                }
            }
        }
        plan
    }

    /// Slides everything one way. Returns true if anything moved.
    pub fn slide(&mut self, dir: Dir) -> bool {
        let plan = self.plan(dir);
        self.apply(plan)
    }

    fn apply(&mut self, plan: Plan) -> bool {
        if self.state != GameState::Playing || self.end_ms > 0.0 || !plan.changed {
            return false;
        }
        self.history = Some((self.tiles.clone(), self.score));
        self.tiles = plan.tiles;
        self.ghosts = plan.ghosts;
        self.score += plan.gained;
        self.anim_ms = 0.0;
        let merges: Vec<Tile> = self.tiles.iter().filter(|t| t.kind == TileKind::Merged).copied().collect();
        for t in merges {
            self.burst(&t);
        }
        self.spawn();
        if !self.reached_goal && self.best_tile() >= GOAL {
            self.reached_goal = true;
            self.popup = Some(format!("{GOAL}! Keep going"));
            self.popup_ms = 2200.0;
        }
        if !self.can_move() {
            self.end_ms = END_PAUSE_MS;
        }
        true
    }

    /// Sparks, a shockwave and (for big tiles) a shake when a merge lands.
    fn burst(&mut self, t: &Tile) {
        let power = (t.value as f64).log2(); // 2 for a 4, 11 for a 2048
        let count = (10.0 + power * 2.0).min(32.0) as usize;
        let (cy, cx) = (t.row as f64 + 0.5, t.col as f64 + 0.5);
        for k in 0..count {
            let angle = k as f64 / count as f64 * std::f64::consts::TAU + self.rng.range(-0.3, 0.3);
            let speed = self.rng.range(1.6, 2.6 + power * 0.25);
            self.sparks.push(Spark {
                y: cy,
                x: cx,
                vy: angle.sin() * speed,
                vx: angle.cos() * speed,
                size: self.rng.range(0.06, 0.13),
                value: t.value,
                age: -SLIDE_MS,
            });
        }
        self.rings.push(Ring { row: t.row, col: t.col, value: t.value, age: -SLIDE_MS });
        if t.value >= SHAKE_FROM {
            self.shake_ms = SHAKE_MS + SLIDE_MS;
            self.shake_power = self.shake_power.max((power - 5.0).min(5.0) + 2.0);
        }
    }

    pub fn can_move(&self) -> bool {
        let g = self.grid();
        let n = self.size;
        (0..n).any(|r| {
            (0..n).any(|c| {
                g[r][c] == 0
                    || (c + 1 < n && g[r][c] == g[r][c + 1])
                    || (r + 1 < n && g[r][c] == g[r + 1][c])
            })
        })
    }

    /// Takes back the last move (three times per game).
    pub fn undo(&mut self) {
        if self.undos_left == 0 {
            return;
        }
        let Some((tiles, score)) = self.history.take() else { return };
        self.tiles = tiles.into_iter().map(|t| Tile { from: t.at(), kind: TileKind::Moved, ..t }).collect();
        self.ghosts.clear();
        self.sparks.clear();
        self.rings.clear();
        self.score = score;
        self.undos_left -= 1;
        self.end_ms = 0.0;
        self.state = GameState::Playing;
        self.anim_ms = f64::MAX;
    }

    pub fn can_undo(&self) -> bool {
        self.undos_left > 0 && self.history.is_some()
    }

    pub fn handle_key(&mut self, key: Key, pressed: bool) -> bool {
        let used = key.is_direction() || matches!(key, Key::Action | Key::Pause | Key::Letter('z' | 'g' | 'n'));
        if !pressed || !used {
            return used;
        }
        // New game, board size and undo work from any state.
        match key {
            Key::Letter('n') => self.new_game(self.size),
            Key::Letter('g') => self.new_game(if self.size == 4 { 5 } else { 4 }),
            Key::Letter('z') => self.undo(),
            _ => {}
        }
        if matches!(key, Key::Letter(_)) {
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
                    self.new_game(self.size);
                }
            }
            GameState::Ready | GameState::Playing => match key {
                Key::Up => _ = self.slide(Dir::Up),
                Key::Down => _ = self.slide(Dir::Down),
                Key::Left => _ = self.slide(Dir::Left),
                Key::Right => _ = self.slide(Dir::Right),
                Key::Pause => self.pause(),
                _ => {}
            },
        }
        true
    }

    /// The drag's direction and length in canvas pixels, once it's long enough to read.
    fn drag_dir(&self) -> Option<(Dir, f64)> {
        let ((sx, sy), (x, y)) = self.drag?;
        let (dx, dy) = (x - sx, y - sy);
        if dx.abs().max(dy.abs()) < 6.0 {
            return None;
        }
        Some(if dx.abs() > dy.abs() {
            (if dx > 0.0 { Dir::Right } else { Dir::Left }, dx.abs())
        } else {
            (if dy > 0.0 { Dir::Down } else { Dir::Up }, dy.abs())
        })
    }

    /// Where each tile of `plan` sits while being dragged `px` pixels: every
    /// tile follows the pointer until it reaches its stopping place.
    fn dragged(&self, plan: &Plan, dir: Dir, px: f64) -> (Vec<Cellf>, Vec<Cellf>) {
        let (gap, cell) = self.metrics();
        let reach = px / (gap + cell);
        let (sy, sx) = dir.step();
        let place = |t: &Tile| {
            let travel = ((t.row as f64 - t.from.0).abs() + (t.col as f64 - t.from.1).abs()).min(reach);
            (t.from.0 + sy * travel, t.from.1 + sx * travel)
        };
        // Merged tiles aren't on the board yet; their two halves are the ghosts.
        let tiles = plan.tiles.iter().filter(|t| t.kind != TileKind::Merged).map(place).collect();
        let ghosts = plan.ghosts.iter().map(place).collect();
        (tiles, ghosts)
    }

    /// Tiles to draw mid-drag: (value, row, col) in fractional cells.
    pub fn drag_view(&self) -> Option<Vec<(u32, f64, f64)>> {
        if self.state != GameState::Playing || self.end_ms > 0.0 {
            return None;
        }
        let (dir, px) = self.drag_dir()?;
        let plan = self.plan(dir);
        let (tiles, ghosts) = self.dragged(&plan, dir, px);
        let still = plan.tiles.iter().filter(|t| t.kind != TileKind::Merged).zip(tiles);
        Some(
            still
                .chain(plan.ghosts.iter().zip(ghosts))
                .map(|(t, (r, c))| (t.value, r, c))
                .collect(),
        )
    }

    pub fn pointer_down(&mut self, x: f64, y: f64) -> bool {
        self.drag = Some(((x, y), (x, y)));
        // Finish any animation still playing so the drag starts from rest.
        if self.anim_ms < SLIDE_MS {
            self.anim_ms = SLIDE_MS;
            self.ghosts.clear();
        }
        true
    }

    pub fn pointer_move(&mut self, x: f64, y: f64) {
        if let Some((start, _)) = self.drag {
            if x >= 0.0 || y >= 0.0 {
                self.drag = Some((start, (x, y)));
            }
        }
    }

    pub fn pointer_up(&mut self, x: f64, y: f64) {
        self.pointer_move(x, y);
        let dir = self.drag_dir();
        self.drag = None;
        let Some((dir, px)) = dir else { return };
        if self.state != GameState::Playing || self.end_ms > 0.0 {
            return;
        }
        let mut plan = self.plan(dir);
        let (tiles, ghosts) = self.dragged(&plan, dir, px);
        // Where the drag left each tile, keyed by its starting cell.
        let starts = plan.tiles.iter().filter(|t| t.kind != TileKind::Merged).chain(&plan.ghosts).map(|t| t.from);
        let left_at: Vec<((f64, f64), (f64, f64))> = starts.zip(tiles.into_iter().chain(ghosts)).collect();
        let lookup = |cell: (f64, f64)| left_at.iter().find(|(start, _)| *start == cell).map(|(_, p)| *p);
        if px >= SWIPE_MIN && plan.changed {
            // Carry on sliding from where the drag left each tile.
            for t in plan.tiles.iter_mut().filter(|t| t.kind != TileKind::Merged).chain(plan.ghosts.iter_mut()) {
                t.from = lookup(t.from).unwrap_or(t.from);
            }
            self.apply(plan);
            return;
        }
        // Too short, or nothing can move that way: spring back into place.
        for t in &mut self.tiles {
            if let Some(p) = lookup(t.at()) {
                t.from = p;
                t.kind = TileKind::Moved;
            }
        }
        self.anim_ms = 0.0;
    }

    pub fn update(&mut self, dt_ms: f64) {
        if self.anim_ms < f64::MAX / 2.0 {
            self.anim_ms += dt_ms;
        }
        if self.anim_ms > SLIDE_MS {
            self.ghosts.clear();
        }
        let dt = dt_ms / 1000.0;
        for s in &mut self.sparks {
            s.age += dt_ms;
            if s.age > 0.0 {
                s.x += s.vx * dt;
                s.y += s.vy * dt;
                let drag = (1.0 - 3.5 * dt).max(0.0);
                s.vx *= drag;
                s.vy *= drag;
            }
        }
        self.sparks.retain(|s| s.age < SPARK_MS);
        for r in &mut self.rings {
            r.age += dt_ms;
        }
        self.rings.retain(|r| r.age < RING_MS);
        if self.shake_ms > 0.0 {
            self.shake_ms -= dt_ms;
            if self.shake_ms <= 0.0 {
                self.shake_power = 0.0;
            }
        }
        if self.popup_ms > 0.0 {
            self.popup_ms -= dt_ms;
        }
        if self.state == GameState::Playing && self.end_ms > 0.0 {
            self.end_ms -= dt_ms;
            if self.end_ms <= 0.0 {
                self.state = GameState::Over;
            }
        }
    }

    /// Board offset for the merge shake, in pixels.
    pub fn shake(&self) -> (f64, f64) {
        if self.shake_ms <= 0.0 || self.shake_ms > SHAKE_MS {
            return (0.0, 0.0);
        }
        let k = self.shake_ms / SHAKE_MS * self.shake_power;
        let t = self.shake_ms;
        ((t * 0.11).sin() * k, (t * 0.087).cos() * k)
    }

    pub fn pause(&mut self) {
        if self.state == GameState::Playing && self.end_ms <= 0.0 {
            self.state = GameState::Paused;
        }
        self.drag = None;
    }

    pub fn end_message(&self) -> Option<String> {
        let undo = if self.can_undo() { " · Z to undo" } else { "" };
        Some(format!("No moves left · best tile {}{undo}", self.best_tile()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A board laid out from rows of values (0 = empty), with spawning
    /// made predictable by checking only the non-new tiles.
    fn board(rows: &[&[u32]]) -> NumberMerge {
        let mut g = NumberMerge::new(1);
        g.size = rows.len();
        g.tiles.clear();
        for (r, row) in rows.iter().enumerate() {
            for (c, &v) in row.iter().enumerate() {
                if v != 0 {
                    g.tiles.push(Tile { value: v, row: r, col: c, from: (r as f64, c as f64), kind: TileKind::Moved });
                }
            }
        }
        g
    }

    /// The grid without the tile spawned by the last move.
    fn settled(g: &NumberMerge) -> Vec<Vec<u32>> {
        let mut grid = vec![vec![0; g.size]; g.size];
        for t in g.tiles.iter().filter(|t| t.kind != TileKind::Spawned) {
            grid[t.row][t.col] = t.value;
        }
        grid
    }

    #[test]
    fn starts_with_two_tiles() {
        let g = NumberMerge::new(9);
        assert_eq!(g.tiles.len(), 2);
        assert!(g.tiles.iter().all(|t| t.value == 2 || t.value == 4));
    }

    #[test]
    fn slides_to_the_edge() {
        let mut g = board(&[&[0, 2, 0, 4], &[0; 4], &[0; 4], &[0; 4]]);
        assert!(g.slide(Dir::Left));
        assert_eq!(settled(&g)[0], vec![2, 4, 0, 0]);
        assert_eq!(g.tiles.len(), 3, "one new tile");
    }

    #[test]
    fn equal_tiles_merge_and_score() {
        let mut g = board(&[&[2, 2, 0, 0], &[0; 4], &[0; 4], &[0; 4]]);
        g.slide(Dir::Right);
        assert_eq!(settled(&g)[0], vec![0, 0, 0, 4]);
        assert_eq!(g.score, 4);
        assert_eq!(g.ghosts.len(), 2);
    }

    #[test]
    fn each_tile_merges_once_per_move() {
        let mut g = board(&[&[2, 2, 2, 2], &[4, 4, 8, 0], &[0; 4], &[0; 4]]);
        g.slide(Dir::Left);
        let s = settled(&g);
        assert_eq!(s[0], vec![4, 4, 0, 0]);
        assert_eq!(s[1], vec![8, 8, 0, 0], "new 8 doesn't merge with the old 8");
        assert_eq!(g.score, 4 + 4 + 8);
    }

    #[test]
    fn merge_happens_at_the_leading_edge() {
        let mut g = board(&[&[2, 2, 2, 0], &[0; 4], &[0; 4], &[0; 4]]);
        g.slide(Dir::Right);
        assert_eq!(settled(&g)[0], vec![0, 0, 2, 4]);
    }

    #[test]
    fn vertical_moves() {
        let mut g = board(&[&[2, 0, 0, 0], &[2, 0, 0, 0], &[4, 0, 0, 0], &[0; 4]]);
        g.slide(Dir::Down);
        let s = settled(&g);
        assert_eq!((s[2][0], s[3][0]), (4, 4));
        let mut g = board(&[&[0; 4], &[0; 4], &[0, 8, 0, 0], &[0, 8, 0, 0]]);
        g.slide(Dir::Up);
        assert_eq!(settled(&g)[0][1], 16);
    }

    #[test]
    fn a_move_that_changes_nothing_adds_no_tile() {
        let mut g = board(&[&[2, 4, 0, 0], &[0; 4], &[0; 4], &[0; 4]]);
        assert!(!g.slide(Dir::Left));
        assert_eq!(g.tiles.len(), 2);
    }

    #[test]
    fn full_board_without_pairs_ends_the_game() {
        let mut g = board(&[&[2, 4, 2, 4], &[4, 2, 4, 2], &[2, 4, 2, 4], &[4, 2, 4, 0]]);
        assert!(g.can_move());
        // Sliding right fills the gap; whatever spawns, check the outcome matches can_move.
        g.slide(Dir::Right);
        if !g.can_move() {
            g.update(END_PAUSE_MS + 1.0);
            assert_eq!(g.state, GameState::Over);
            assert!(g.end_message().unwrap().starts_with("No moves left"));
        }
        let stuck = board(&[&[2, 4, 2, 4], &[4, 2, 4, 2], &[2, 4, 2, 4], &[4, 2, 4, 2]]);
        assert!(!stuck.can_move());
    }

    #[test]
    fn undo_restores_board_and_score_three_times() {
        let mut g = board(&[&[2, 2, 0, 0], &[0; 4], &[0; 4], &[0; 4]]);
        g.slide(Dir::Left);
        g.undo();
        assert_eq!(g.grid()[0], vec![2, 2, 0, 0]);
        assert_eq!(g.score, 0);
        assert_eq!(g.undos_left, UNDOS - 1);
        assert!(!g.can_undo(), "one step back only");
        for _ in 0..5 {
            g.slide(Dir::Left);
            g.slide(Dir::Right);
            g.undo();
        }
        assert_eq!(g.undos_left, 0);
    }

    #[test]
    fn undo_after_game_over_resumes() {
        let mut g = board(&[&[2, 4, 2, 4], &[4, 2, 4, 2], &[2, 4, 2, 4], &[4, 2, 0, 8]]);
        g.slide(Dir::Left);
        g.update(END_PAUSE_MS + 1.0);
        g.undo();
        assert_eq!(g.state, GameState::Playing);
    }

    #[test]
    fn reaching_the_goal_shows_a_message_and_play_continues() {
        let mut g = board(&[&[1024, 1024, 0, 0], &[0; 4], &[0; 4], &[0; 4]]);
        g.slide(Dir::Left);
        assert!(g.reached_goal);
        assert!(g.popup.as_deref().unwrap().starts_with("2048"));
        assert_eq!(g.state, GameState::Playing);
    }

    #[test]
    fn swipes_pick_the_longer_axis() {
        let mut g = board(&[&[0, 0, 0, 2], &[0; 4], &[0; 4], &[0; 4]]);
        g.pointer_down(300.0, 300.0);
        g.pointer_move(250.0, 290.0);
        g.pointer_up(200.0, 280.0);
        assert_eq!(settled(&g)[0][0], 2);
        let tiles = g.tiles.len();
        g.pointer_down(100.0, 100.0);
        g.pointer_up(105.0, 110.0);
        assert_eq!(g.tiles.len(), tiles, "tiny drags are ignored");
    }

    #[test]
    fn g_switches_board_size() {
        let mut g = NumberMerge::new(2);
        g.handle_key(Key::Letter('g'), true);
        assert_eq!(g.size, 5);
        assert_eq!(g.tiles.len(), 2);
        g.handle_key(Key::Letter('g'), true);
        assert_eq!(g.size, 4);
    }

    #[test]
    fn dragging_moves_tiles_with_the_pointer_until_they_stop() {
        let mut g = board(&[&[0, 0, 0, 2], &[0; 4], &[0; 4], &[0, 4, 0, 0]]);
        let (gap, cell) = g.metrics();
        let pitch = gap + cell;
        g.pointer_down(400.0, 300.0);
        g.pointer_move(400.0 - pitch * 1.5, 300.0);
        let view = g.drag_view().unwrap();
        let two = view.iter().find(|v| v.0 == 2).unwrap();
        let four = view.iter().find(|v| v.0 == 4).unwrap();
        assert!((two.2 - 1.5).abs() < 1e-9, "2 has followed the pointer 1.5 cells");
        assert!((four.2 - 0.0).abs() < 1e-9, "4 stopped at the wall after 1 cell");
        assert_eq!(g.tiles.len(), 2, "nothing committed while dragging");
    }

    #[test]
    fn short_drag_springs_back_without_a_move() {
        let mut g = board(&[&[0, 0, 0, 2], &[0; 4], &[0; 4], &[0; 4]]);
        g.pointer_down(300.0, 300.0);
        g.pointer_move(290.0, 300.0);
        g.pointer_up(290.0, 300.0);
        assert_eq!(g.tiles.len(), 1);
        assert_eq!((g.tiles[0].row, g.tiles[0].col), (0, 3));
        assert!(g.tiles[0].from.1 < 3.0, "animates back from where it was dragged");
    }

    #[test]
    fn released_drag_continues_from_the_dragged_spot() {
        let mut g = board(&[&[0, 0, 0, 2], &[0; 4], &[0; 4], &[0; 4]]);
        let (gap, cell) = g.metrics();
        g.pointer_down(400.0, 300.0);
        g.pointer_up(400.0 - (gap + cell), 300.0);
        let t = g.tiles.iter().find(|t| t.kind == TileKind::Moved).unwrap();
        assert_eq!((t.row, t.col), (0, 0));
        assert!((t.from.1 - 2.0).abs() < 1e-9);
    }

    #[test]
    fn merges_burst_with_sparks_and_big_ones_shake() {
        let mut g = board(&[&[2, 2, 64, 64], &[0; 4], &[0; 4], &[0; 4]]);
        g.slide(Dir::Left);
        assert_eq!(g.rings.len(), 2);
        assert!(g.sparks.len() > 20);
        assert!(g.shake_power > 0.0, "the 128 shakes the board");
        for _ in 0..60 {
            g.update(16.0);
        }
        assert!(g.sparks.is_empty() && g.rings.is_empty());
        assert_eq!(g.shake(), (0.0, 0.0));
    }
}
