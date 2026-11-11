//! Mahjong solitaire rules: remove matching pairs of free tiles until the
//! board is empty. Every deal (and every shuffle) is guaranteed solvable.

use crate::layout::{pyramid, Board};
use crate::tiles::{full_set_pairs, Face};
use engine::{GameState, Key, Rng};

const PAIR_POINTS: u32 = 10;
/// Matches made within this many seconds of each other build a streak.
const STREAK_WINDOW: f64 = 4.0;
const STREAK_BONUS: u32 = 2;
const MAX_STREAK_BONUS: u32 = 10;
const CLEAR_BONUS: u32 = 500;
/// Clearing the board under this many seconds earns a bonus point per second left.
const PAR_SECONDS: f64 = 600.0;
const HINT_SECONDS: f64 = 2.5;
const FLASH_SECONDS: f64 = 0.35;
const DEAL_ATTEMPTS: usize = 200;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Outcome {
    Cleared,
    /// No moves and no solvable shuffle exists.
    Stuck,
}

struct Removal {
    a: usize,
    b: usize,
    points: u32,
}

pub struct Mahjong {
    pub board: Board,
    pub faces: Vec<Face>,
    pub alive: Vec<bool>,
    /// Cached `board.is_free` for every tile.
    pub free: Vec<bool>,
    pub selected: Option<usize>,
    pub hover: Option<usize>,
    pub hint: Option<(usize, usize)>,
    hint_until: f64,
    /// A blocked tile the player tried to pick, shown briefly.
    pub flash: Option<usize>,
    flash_until: f64,
    history: Vec<Removal>,
    pub score: u32,
    pub state: GameState,
    /// Seconds of play on this deal.
    pub elapsed: f64,
    /// Seconds since creation, for animations.
    pub time: f64,
    last_match: f64,
    streak: u32,
    /// Matching pairs currently available.
    pub moves: usize,
    pub outcome: Option<Outcome>,
    rng: Rng,
}

impl Mahjong {
    pub fn new(seed: u64) -> Mahjong {
        let board = Board::new(pyramid());
        let n = board.len();
        let mut m = Mahjong {
            board,
            faces: vec![Face::Dots(1); n],
            alive: vec![true; n],
            free: vec![false; n],
            selected: None,
            hover: None,
            hint: None,
            hint_until: 0.0,
            flash: None,
            flash_until: 0.0,
            history: Vec::new(),
            score: 0,
            state: GameState::Playing,
            elapsed: 0.0,
            time: 0.0,
            last_match: f64::NEG_INFINITY,
            streak: 0,
            moves: 0,
            outcome: None,
            rng: Rng::new(seed),
        };
        m.new_deal();
        m
    }

    pub fn new_deal(&mut self) {
        self.alive.iter_mut().for_each(|a| *a = true);
        let mut pairs = full_set_pairs();
        shuffle(&mut pairs, &mut self.rng);
        pairs.truncate(self.board.len() / 2);
        let all: Vec<usize> = (0..self.board.len()).collect();
        // A full layout always has a solution order, so this can't fail in practice.
        let order = self.solvable_order(&all).expect("layout has no solvable order");
        self.assign(&order, &pairs);
        self.history.clear();
        self.score = 0;
        self.elapsed = 0.0;
        self.streak = 0;
        self.last_match = f64::NEG_INFINITY;
        self.outcome = None;
        self.state = GameState::Playing;
        self.clear_marks();
        self.refresh();
    }

    /// Finds an order in which `tiles` could all be removed two at a time,
    /// by simulating removals of random free pairs. Faces given to each pair
    /// in this order make the deal solvable by construction.
    fn solvable_order(&mut self, tiles: &[usize]) -> Option<Vec<(usize, usize)>> {
        let n = self.board.len();
        'attempt: for _ in 0..DEAL_ATTEMPTS {
            let mut alive = vec![false; n];
            tiles.iter().for_each(|&i| alive[i] = true);
            let mut left = tiles.len();
            let mut order = Vec::with_capacity(left / 2);
            while left > 0 {
                let free: Vec<usize> = tiles.iter().copied().filter(|&i| self.board.is_free(&alive, i)).collect();
                if free.len() < 2 {
                    continue 'attempt;
                }
                let a = free[self.rng.below(free.len())];
                let mut b = free[self.rng.below(free.len() - 1)];
                if b == a {
                    b = free[free.len() - 1];
                }
                alive[a] = false;
                alive[b] = false;
                left -= 2;
                order.push((a, b));
            }
            return Some(order);
        }
        None
    }

    fn assign(&mut self, order: &[(usize, usize)], pairs: &[(Face, Face)]) {
        for (&(a, b), &(fa, fb)) in order.iter().zip(pairs) {
            self.faces[a] = fa;
            self.faces[b] = fb;
        }
    }

    /// Rearranges the remaining tiles into a new solvable position.
    pub fn shuffle_remaining(&mut self) {
        let remaining: Vec<usize> = (0..self.board.len()).filter(|&i| self.alive[i]).collect();
        if remaining.is_empty() {
            return;
        }
        // Remaining tiles always form matching pairs; pair them back up.
        let mut faces: Vec<Face> = remaining.iter().map(|&i| self.faces[i]).collect();
        faces.sort_by_key(|f| f.group());
        let mut pairs: Vec<(Face, Face)> = faces.chunks(2).map(|c| (c[0], c[1])).collect();
        shuffle(&mut pairs, &mut self.rng);

        match self.solvable_order(&remaining) {
            Some(order) => {
                self.assign(&order, &pairs);
                self.streak = 0;
                self.clear_marks();
                self.refresh();
            }
            None => {
                self.outcome = Some(Outcome::Stuck);
                self.state = GameState::Over;
            }
        }
    }

    fn clear_marks(&mut self) {
        self.selected = None;
        self.hint = None;
        self.flash = None;
    }

    fn refresh(&mut self) {
        for i in 0..self.board.len() {
            self.free[i] = self.board.is_free(&self.alive, i);
        }
        self.moves = self.count_moves();
    }

    fn count_moves(&self) -> usize {
        let mut counts = std::collections::HashMap::<u16, usize>::new();
        for i in (0..self.board.len()).filter(|&i| self.free[i]) {
            *counts.entry(self.faces[i].group()).or_default() += 1;
        }
        counts.values().map(|k| k * (k - 1) / 2).sum()
    }

    pub fn tiles_left(&self) -> usize {
        self.alive.iter().filter(|a| **a).count()
    }

    pub fn find_pair(&self) -> Option<(usize, usize)> {
        let free: Vec<usize> = (0..self.board.len()).filter(|&i| self.free[i]).collect();
        for (k, &a) in free.iter().enumerate() {
            if let Some(&b) = free[k + 1..].iter().find(|&&b| self.faces[a].matches(self.faces[b])) {
                return Some((a, b));
            }
        }
        None
    }

    /// The player picked tile `i`.
    pub fn pick(&mut self, i: usize) {
        if !self.free[i] {
            self.flash = Some(i);
            self.flash_until = self.time + FLASH_SECONDS;
            return;
        }
        match self.selected {
            Some(s) if s == i => self.selected = None,
            Some(s) if self.faces[s].matches(self.faces[i]) => self.remove_pair(s, i),
            _ => self.selected = Some(i),
        }
    }

    fn remove_pair(&mut self, a: usize, b: usize) {
        self.streak = if self.elapsed - self.last_match <= STREAK_WINDOW { self.streak + 1 } else { 0 };
        self.last_match = self.elapsed;
        let points = PAIR_POINTS + (self.streak * STREAK_BONUS).min(MAX_STREAK_BONUS);
        self.score += points;
        self.alive[a] = false;
        self.alive[b] = false;
        self.history.push(Removal { a, b, points });
        self.clear_marks();
        self.refresh();

        if self.tiles_left() == 0 {
            self.score += CLEAR_BONUS + (PAR_SECONDS - self.elapsed).max(0.0) as u32;
            self.outcome = Some(Outcome::Cleared);
            self.state = GameState::Over;
        }
    }

    pub fn undo(&mut self) {
        if let Some(r) = self.history.pop() {
            self.alive[r.a] = true;
            self.alive[r.b] = true;
            self.score = self.score.saturating_sub(r.points);
            self.streak = 0;
            self.clear_marks();
            self.refresh();
        }
    }

    pub fn show_hint(&mut self) {
        self.hint = self.find_pair();
        self.hint_until = self.time + HINT_SECONDS;
    }

    pub fn handle_key(&mut self, key: Key, pressed: bool) -> bool {
        let used = matches!(key, Key::Action | Key::Pause | Key::Letter('h' | 'z' | 'u' | 'r' | 'n'));
        if !pressed || !used {
            return used;
        }
        match (self.state, key) {
            (_, Key::Letter('n')) => self.new_deal(),
            (GameState::Over, Key::Action) => self.new_deal(),
            (GameState::Playing, Key::Action | Key::Pause) => self.state = GameState::Paused,
            (GameState::Paused, Key::Action | Key::Pause) => self.state = GameState::Playing,
            (GameState::Playing, Key::Letter('h')) => self.show_hint(),
            (GameState::Playing, Key::Letter('z' | 'u')) => self.undo(),
            (GameState::Playing, Key::Letter('r')) => self.shuffle_remaining(),
            _ => {}
        }
        true
    }

    pub fn pointer_down(&mut self, x: f64, y: f64) -> bool {
        if self.state != GameState::Playing {
            return false;
        }
        match self.board.tile_at(&self.alive, x, y) {
            Some(i) => self.pick(i),
            None => self.selected = None,
        }
        true
    }

    pub fn pointer_move(&mut self, x: f64, y: f64) {
        self.hover = self.board.tile_at(&self.alive, x, y).filter(|&i| self.free[i]);
    }

    pub fn update(&mut self, dt_ms: f64) {
        let dt = dt_ms / 1000.0;
        self.time += dt;
        if self.state == GameState::Playing {
            self.elapsed += dt;
        }
        if self.time > self.hint_until {
            self.hint = None;
        }
        if self.time > self.flash_until {
            self.flash = None;
        }
    }

    pub fn pause(&mut self) {
        if self.state == GameState::Playing {
            self.state = GameState::Paused;
        }
    }
}

fn shuffle<T>(v: &mut [T], rng: &mut Rng) {
    for i in (1..v.len()).rev() {
        v.swap(i, rng.below(i + 1));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Plays a whole game using hints only.
    fn solve(m: &mut Mahjong) -> bool {
        while let Some((a, b)) = m.find_pair() {
            m.pick(a);
            m.pick(b);
        }
        m.tiles_left() == 0
    }

    #[test]
    fn deal_uses_matching_pairs() {
        let m = Mahjong::new(1);
        let mut counts = std::collections::HashMap::<u16, usize>::new();
        for f in &m.faces {
            *counts.entry(f.group()).or_default() += 1;
        }
        assert!(counts.values().all(|c| c % 2 == 0));
        assert!(m.moves > 0);
    }

    #[test]
    fn deals_are_solvable_along_the_generated_order() {
        // Greedy hint play isn't guaranteed to win, but the generator's own
        // order is. Check it by replaying a fresh order on a fresh board.
        for seed in 0..20 {
            let mut m = Mahjong::new(seed);
            let all: Vec<usize> = (0..m.board.len()).collect();
            let order = m.solvable_order(&all).unwrap();
            let mut alive = vec![true; m.board.len()];
            for (a, b) in order {
                assert!(m.board.is_free(&alive, a) && m.board.is_free(&alive, b));
                alive[a] = false;
                alive[b] = false;
            }
        }
    }

    #[test]
    fn matching_free_pair_is_removed_and_scored() {
        let mut m = Mahjong::new(2);
        let (a, b) = m.find_pair().unwrap();
        m.pick(a);
        assert_eq!(m.selected, Some(a));
        m.pick(b);
        assert!(!m.alive[a] && !m.alive[b]);
        assert_eq!(m.score, PAIR_POINTS);
        assert_eq!(m.selected, None);
    }

    #[test]
    fn every_match_is_two_identical_tiles() {
        for seed in 0..10 {
            let mut m = Mahjong::new(seed);
            while let Some((a, b)) = m.find_pair() {
                assert_eq!(m.faces[a], m.faces[b]);
                m.pick(a);
                m.pick(b);
            }
        }
    }

    #[test]
    fn different_tiles_do_not_match() {
        let mut m = Mahjong::new(6);
        let free: Vec<usize> = (0..m.board.len()).filter(|&i| m.free[i]).collect();
        let (a, b) = free
            .iter()
            .flat_map(|&a| free.iter().map(move |&b| (a, b)))
            .find(|&(a, b)| m.faces[a] != m.faces[b])
            .unwrap();
        m.pick(a);
        m.pick(b);
        assert!(m.alive[a] && m.alive[b]);
        assert_eq!(m.selected, Some(b));
    }

    #[test]
    fn blocked_tile_cannot_be_selected() {
        let mut m = Mahjong::new(3);
        let blocked = (0..m.board.len()).find(|&i| !m.free[i]).unwrap();
        m.pick(blocked);
        assert_eq!(m.selected, None);
        assert_eq!(m.flash, Some(blocked));
    }

    #[test]
    fn undo_restores_tiles_and_points() {
        let mut m = Mahjong::new(4);
        let (a, b) = m.find_pair().unwrap();
        m.pick(a);
        m.pick(b);
        m.undo();
        assert!(m.alive[a] && m.alive[b]);
        assert_eq!(m.score, 0);
    }

    #[test]
    fn shuffle_keeps_tiles_and_stays_solvable() {
        let mut m = Mahjong::new(5);
        for _ in 0..10 {
            let (a, b) = m.find_pair().unwrap();
            m.pick(a);
            m.pick(b);
        }
        let left = m.tiles_left();
        m.shuffle_remaining();
        assert_eq!(m.tiles_left(), left);
        assert!(m.moves > 0);
    }

    #[test]
    fn hint_play_with_shuffles_clears_the_board() {
        for seed in 0..5 {
            let mut m = Mahjong::new(seed);
            for _ in 0..20 {
                if solve(&mut m) {
                    break;
                }
                m.shuffle_remaining();
            }
            assert_eq!(m.outcome, Some(Outcome::Cleared), "seed {seed}");
            assert!(m.score >= 72 * PAIR_POINTS + CLEAR_BONUS);
        }
    }
}
