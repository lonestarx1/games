//! Klondike solitaire rules. Build four foundation piles from Ace to King,
//! one per suit, using seven tableau columns that stack downward in
//! alternating colours. No browser code here, so it's all unit-testable.

use engine::{GameState, Key, Rng};

pub const W: f64 = 700.0;
pub const H: f64 = 680.0;
pub const CW: f64 = 84.0;
pub const CH: f64 = 118.0;
pub const GAP: f64 = (W - 7.0 * CW) / 8.0;
pub const TOP: f64 = 16.0;
pub const TAB_Y: f64 = TOP + CH + 24.0;
/// Bottom strip for the draw mode and clock.
pub const FOOTER: f64 = 26.0;
const DOWN_FAN: f64 = 12.0;
const UP_FAN: f64 = 28.0;
pub const WASTE_FAN: f64 = 22.0;

/// How long a card takes to slide to its new place.
pub const FLIGHT_MS: f64 = 170.0;
/// Delay between cards when the game finishes itself.
const AUTO_MS: f64 = 90.0;
const HINT_MS: f64 = 2200.0;
/// Time to enjoy the finished foundations before the overlay appears.
const END_PAUSE_MS: f64 = 1600.0;
/// A press that moves less than this is a click, not a drag.
const DRAG_SLOP: f64 = 5.0;

const TO_FOUNDATION: i32 = 10;
const WASTE_TO_TABLEAU: i32 = 5;
const TURN_OVER: i32 = 5;
const FOUNDATION_TO_TABLEAU: i32 = -15;
/// Turning the waste back into the stock, in Draw 1 only.
const RECYCLE: i32 = -20;
const WIN_BONUS: i32 = 500;
/// Bonus points per second under PAR_SECONDS when you win.
const TIME_BONUS: i32 = 2;
const PAR_SECONDS: f64 = 600.0;

pub const SUITS: [&str; 4] = ["♠", "♥", "♦", "♣"];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Card {
    /// 1 (Ace) to 13 (King).
    pub rank: u8,
    /// Index into SUITS.
    pub suit: u8,
    pub up: bool,
}

impl Card {
    pub fn red(self) -> bool {
        self.suit == 1 || self.suit == 2
    }

    pub fn rank_label(self) -> &'static str {
        ["", "A", "2", "3", "4", "5", "6", "7", "8", "9", "10", "J", "Q", "K"][self.rank as usize]
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Pile {
    Stock,
    Waste,
    Found(usize),
    Tab(usize),
}

/// Everything undo needs to restore.
#[derive(Clone)]
struct Snapshot {
    stock: Vec<Card>,
    waste: Vec<Card>,
    found: [Vec<Card>; 4],
    tab: [Vec<Card>; 7],
    points: i32,
}

/// A card sliding from `from` to its place in `pile`.
pub struct Flight {
    pub pile: Pile,
    pub idx: usize,
    pub from: (f64, f64),
    pub t: f64,
}

pub struct Drag {
    pub from: Pile,
    pub idx: usize,
    /// Pointer position minus the grabbed card's top-left.
    pub grab: (f64, f64),
    pub start: (f64, f64),
    pub at: (f64, f64),
    pub moved: bool,
}

/// A suggested move: the card to move and where (None means "draw a card").
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Hint {
    pub from: Pile,
    pub idx: usize,
    pub to: Option<Pile>,
}

pub struct Klondike {
    pub stock: Vec<Card>,
    pub waste: Vec<Card>,
    pub found: [Vec<Card>; 4],
    pub tab: [Vec<Card>; 7],
    pub draw_three: bool,
    /// Can go below zero; the shown score stops at zero.
    pub points: i32,
    pub state: GameState,
    pub won: bool,
    /// Seconds of play (starts at the first move).
    pub elapsed: f64,
    /// Seconds since creation, for animations.
    pub time: f64,
    pub started: bool,
    end_ms: f64,
    history: Vec<Snapshot>,
    pub flights: Vec<Flight>,
    pub drag: Option<Drag>,
    pub hint: Option<Hint>,
    hint_ms: f64,
    /// Shown briefly when a hint finds nothing.
    pub no_moves_ms: f64,
    auto_ms: f64,
    rng: Rng,
}

pub fn col_x(i: usize) -> f64 {
    GAP + i as f64 * (CW + GAP)
}

/// Top-left of an empty pile's slot.
pub fn slot(p: Pile) -> (f64, f64) {
    match p {
        Pile::Stock => (col_x(0), TOP),
        Pile::Waste => (col_x(1), TOP),
        Pile::Found(f) => (col_x(3 + f), TOP),
        Pile::Tab(t) => (col_x(t), TAB_Y),
    }
}

fn inside(px: f64, py: f64, (x, y): (f64, f64)) -> bool {
    px >= x && px < x + CW && py >= y && py < y + CH
}

fn overlap((ax, ay): (f64, f64), (bx, by): (f64, f64)) -> f64 {
    let w = (ax.min(bx) + CW - ax.max(bx)).max(0.0);
    let h = (ay.min(by) + CH - ay.max(by)).max(0.0);
    w * h
}

impl Klondike {
    pub fn new(seed: u64) -> Klondike {
        let mut g = Klondike {
            stock: Vec::new(),
            waste: Vec::new(),
            found: Default::default(),
            tab: Default::default(),
            draw_three: false,
            points: 0,
            state: GameState::Playing,
            won: false,
            elapsed: 0.0,
            time: 0.0,
            started: false,
            end_ms: 0.0,
            history: Vec::new(),
            flights: Vec::new(),
            drag: None,
            hint: None,
            hint_ms: 0.0,
            no_moves_ms: 0.0,
            auto_ms: 0.0,
            rng: Rng::new(seed),
        };
        g.deal();
        g
    }

    pub fn deal(&mut self) {
        let mut deck: Vec<Card> = (0..4u8)
            .flat_map(|suit| (1..=13u8).map(move |rank| Card { rank, suit, up: false }))
            .collect();
        for i in (1..deck.len()).rev() {
            deck.swap(i, self.rng.below(i + 1));
        }
        self.found = Default::default();
        self.tab = Default::default();
        for t in 0..7 {
            for k in 0..=t {
                let mut c = deck.pop().expect("52 cards");
                c.up = k == t;
                self.tab[t].push(c);
            }
        }
        self.stock = deck;
        self.waste.clear();
        self.points = 0;
        self.state = GameState::Playing;
        self.won = false;
        self.elapsed = 0.0;
        self.started = false;
        self.end_ms = 0.0;
        self.history.clear();
        self.flights.clear();
        self.drag = None;
        self.hint = None;
    }

    pub fn score(&self) -> u32 {
        self.points.max(0) as u32
    }

    pub fn pile(&self, p: Pile) -> &Vec<Card> {
        match p {
            Pile::Stock => &self.stock,
            Pile::Waste => &self.waste,
            Pile::Found(f) => &self.found[f],
            Pile::Tab(t) => &self.tab[t],
        }
    }

    fn pile_mut(&mut self, p: Pile) -> &mut Vec<Card> {
        match p {
            Pile::Stock => &mut self.stock,
            Pile::Waste => &mut self.waste,
            Pile::Found(f) => &mut self.found[f],
            Pile::Tab(t) => &mut self.tab[t],
        }
    }

    fn active(&self) -> bool {
        self.state == GameState::Playing && !self.won
    }

    /// Vertical gap between face-up cards in a column, squeezed to fit.
    fn up_gap(&self, t: usize) -> f64 {
        let col = &self.tab[t];
        let downs = col.iter().filter(|c| !c.up).count() as f64;
        let ups = col.iter().filter(|c| c.up).count();
        if ups <= 1 {
            return UP_FAN;
        }
        let room = H - FOOTER - TAB_Y - CH - downs * DOWN_FAN;
        UP_FAN.min(room / (ups - 1) as f64)
    }

    /// Top-left of a card at its resting place.
    pub fn card_pos(&self, p: Pile, idx: usize) -> (f64, f64) {
        let (x, y) = slot(p);
        match p {
            Pile::Waste => {
                let shown = self.waste.len().min(if self.draw_three { 3 } else { 1 });
                let first = self.waste.len() - shown;
                (x + idx.saturating_sub(first) as f64 * WASTE_FAN, y)
            }
            Pile::Tab(t) => {
                let gap = self.up_gap(t);
                let dy: f64 = self.tab[t][..idx].iter().map(|c| if c.up { gap } else { DOWN_FAN }).sum();
                (x, y + dy)
            }
            _ => (x, y),
        }
    }

    /// Where a card is drawn right now (mid-flight cards are between places).
    pub fn draw_pos(&self, p: Pile, idx: usize) -> (f64, f64) {
        let (tx, ty) = self.card_pos(p, idx);
        match self.flights.iter().find(|f| f.pile == p && f.idx == idx) {
            Some(f) => {
                let k = 1.0 - (1.0 - f.t).powi(3); // ease out
                (f.from.0 + (tx - f.from.0) * k, f.from.1 + (ty - f.from.1) * k)
            }
            None => (tx, ty),
        }
    }

    pub fn in_flight(&self, p: Pile, idx: usize) -> bool {
        self.flights.iter().any(|f| f.pile == p && f.idx == idx)
    }

    /// The pile and card under a point. The stock counts even when empty.
    pub fn card_at(&self, x: f64, y: f64) -> Option<(Pile, usize)> {
        if inside(x, y, slot(Pile::Stock)) {
            return Some((Pile::Stock, 0));
        }
        if let Some(i) = self.waste.len().checked_sub(1) {
            if inside(x, y, self.card_pos(Pile::Waste, i)) {
                return Some((Pile::Waste, i));
            }
        }
        for f in 0..4 {
            if let Some(i) = self.found[f].len().checked_sub(1) {
                if inside(x, y, slot(Pile::Found(f))) {
                    return Some((Pile::Found(f), i));
                }
            }
        }
        for t in 0..7 {
            for i in (0..self.tab[t].len()).rev() {
                if inside(x, y, self.card_pos(Pile::Tab(t), i)) {
                    return Some((Pile::Tab(t), i));
                }
            }
        }
        None
    }

    /// Can the run starting at `idx` be picked up?
    pub fn movable(&self, p: Pile, idx: usize) -> bool {
        let pile = self.pile(p);
        match p {
            Pile::Stock => false,
            Pile::Tab(_) => pile.get(idx).is_some_and(|c| c.up),
            _ => idx + 1 == pile.len(),
        }
    }

    pub fn can_drop(&self, card: Card, run_len: usize, to: Pile) -> bool {
        let dest = self.pile(to);
        match to {
            Pile::Found(_) => {
                run_len == 1
                    && match dest.last() {
                        None => card.rank == 1,
                        Some(top) => top.suit == card.suit && card.rank == top.rank + 1,
                    }
            }
            Pile::Tab(_) => match dest.last() {
                None => card.rank == 13,
                Some(top) => top.up && top.red() != card.red() && card.rank + 1 == top.rank,
            },
            _ => false,
        }
    }

    fn snapshot(&mut self) {
        self.history.push(Snapshot {
            stock: self.stock.clone(),
            waste: self.waste.clone(),
            found: self.found.clone(),
            tab: self.tab.clone(),
            points: self.points,
        });
    }

    pub fn undo(&mut self) {
        if !self.active() {
            return;
        }
        if let Some(s) = self.history.pop() {
            self.stock = s.stock;
            self.waste = s.waste;
            self.found = s.found;
            self.tab = s.tab;
            self.points = s.points;
            self.flights.clear();
            self.hint = None;
        }
    }

    /// Moves the run starting at `idx` of `from` onto `to`, if legal. Cards
    /// slide from `start` (their drawn positions) to their new places.
    fn try_move_from(&mut self, from: Pile, idx: usize, to: Pile, start: Vec<(f64, f64)>) -> bool {
        if from == to || !self.movable(from, idx) {
            return false;
        }
        let run_len = self.pile(from).len() - idx;
        let card = self.pile(from)[idx];
        if !self.can_drop(card, run_len, to) {
            return false;
        }
        self.snapshot();
        self.started = true;
        let run: Vec<Card> = self.pile_mut(from).split_off(idx);
        let base = self.pile(to).len();
        self.pile_mut(to).extend(run);
        for (k, from_xy) in start.into_iter().enumerate() {
            self.flights.push(Flight { pile: to, idx: base + k, from: from_xy, t: 0.0 });
        }
        self.points += match (from, to) {
            (_, Pile::Found(_)) => TO_FOUNDATION,
            (Pile::Waste, Pile::Tab(_)) => WASTE_TO_TABLEAU,
            (Pile::Found(_), Pile::Tab(_)) => FOUNDATION_TO_TABLEAU,
            _ => 0,
        };
        if let Pile::Tab(t) = from {
            if let Some(top) = self.tab[t].last_mut() {
                if !top.up {
                    top.up = true;
                    self.points += TURN_OVER;
                }
            }
        }
        self.hint = None;
        self.check_win();
        true
    }

    pub fn try_move(&mut self, from: Pile, idx: usize, to: Pile) -> bool {
        let start = (idx..self.pile(from).len()).map(|i| self.draw_pos(from, i)).collect();
        self.try_move_from(from, idx, to, start)
    }

    /// A single click: send the card to a foundation if it can go there,
    /// otherwise to the first tableau column that takes it.
    pub fn auto_move(&mut self, from: Pile, idx: usize) -> bool {
        if !self.movable(from, idx) {
            return false;
        }
        if !matches!(from, Pile::Found(_)) {
            for f in 0..4 {
                if self.try_move(from, idx, Pile::Found(f)) {
                    return true;
                }
            }
        }
        // Columns with cards first; an empty column only helps if it frees something.
        let already_bottom = matches!(from, Pile::Tab(_)) && idx == 0;
        for want_empty in [false, true] {
            for t in 0..7 {
                if self.tab[t].is_empty() != want_empty || (want_empty && already_bottom) {
                    continue;
                }
                if self.try_move(from, idx, Pile::Tab(t)) {
                    return true;
                }
            }
        }
        false
    }

    /// Turns over the next card (or three), or recycles the waste when the
    /// stock is empty.
    pub fn draw(&mut self) {
        if !self.active() || (self.stock.is_empty() && self.waste.is_empty()) {
            return;
        }
        self.snapshot();
        self.started = true;
        self.hint = None;
        if self.stock.is_empty() {
            self.stock = self.waste.drain(..).rev().map(|c| Card { up: false, ..c }).collect();
            if !self.draw_three {
                self.points += RECYCLE;
            }
            return;
        }
        let n = self.stock.len().min(if self.draw_three { 3 } else { 1 });
        let from = slot(Pile::Stock);
        for _ in 0..n {
            let mut c = self.stock.pop().expect("checked");
            c.up = true;
            self.waste.push(c);
        }
        let len = self.waste.len();
        for idx in len - n..len {
            self.flights.push(Flight { pile: Pile::Waste, idx, from, t: 0.0 });
        }
    }

    fn check_win(&mut self) {
        if self.found.iter().all(|f| f.len() == 13) {
            self.won = true;
            let under_par = (PAR_SECONDS - self.elapsed).max(0.0) as i32;
            self.points += WIN_BONUS + under_par * TIME_BONUS;
            self.end_ms = END_PAUSE_MS;
        }
    }

    /// Everything is face up and dealt out, so the rest is automatic.
    pub fn can_auto_finish(&self) -> bool {
        self.active()
            && self.stock.is_empty()
            && self.waste.is_empty()
            && self.tab.iter().flatten().all(|c| c.up)
            && self.drag.is_none()
    }

    fn auto_step(&mut self) {
        // Lowest card first, so nothing gets stuck behind a higher one.
        let mut best: Option<(u8, usize, usize)> = None;
        for t in 0..7 {
            if let Some(&c) = self.tab[t].last() {
                for f in 0..4 {
                    if self.can_drop(c, 1, Pile::Found(f)) && best.is_none_or(|b| c.rank < b.0) {
                        best = Some((c.rank, t, f));
                    }
                }
            }
        }
        if let Some((_, t, f)) = best {
            let idx = self.tab[t].len() - 1;
            self.try_move(Pile::Tab(t), idx, Pile::Found(f));
        }
    }

    pub fn find_hint(&self) -> Option<Hint> {
        let tops = (0..7).map(Pile::Tab).chain([Pile::Waste]);
        // 1. Anything that can go up to a foundation.
        for p in tops.clone() {
            if let Some(idx) = self.pile(p).len().checked_sub(1) {
                for f in 0..4 {
                    if self.can_drop(self.pile(p)[idx], 1, Pile::Found(f)) {
                        return Some(Hint { from: p, idx, to: Some(Pile::Found(f)) });
                    }
                }
            }
        }
        // 2. Moving a whole face-up run that uncovers a card or empties a column.
        for t in 0..7 {
            let Some(idx) = self.tab[t].iter().position(|c| c.up) else { continue };
            let card = self.tab[t][idx];
            let run = self.tab[t].len() - idx;
            for d in 0..7 {
                let useful = idx > 0 || !self.tab[d].is_empty();
                if d != t && useful && self.can_drop(card, run, Pile::Tab(d)) {
                    return Some(Hint { from: Pile::Tab(t), idx, to: Some(Pile::Tab(d)) });
                }
            }
        }
        // 3. The waste card onto the tableau.
        if let Some(idx) = self.waste.len().checked_sub(1) {
            for d in 0..7 {
                if self.can_drop(self.waste[idx], 1, Pile::Tab(d)) {
                    return Some(Hint { from: Pile::Waste, idx, to: Some(Pile::Tab(d)) });
                }
            }
        }
        // 4. Otherwise turn over more cards.
        if !self.stock.is_empty() || !self.waste.is_empty() {
            return Some(Hint { from: Pile::Stock, idx: 0, to: None });
        }
        None
    }

    pub fn show_hint(&mut self) {
        if !self.active() {
            return;
        }
        self.hint = self.find_hint();
        if self.hint.is_some() {
            self.hint_ms = HINT_MS;
        } else {
            self.no_moves_ms = HINT_MS;
        }
    }

    pub fn handle_key(&mut self, key: Key, pressed: bool) -> bool {
        let used = matches!(key, Key::Action | Key::Pause | Key::Letter('z' | 'h' | 'n' | 't'));
        if !pressed || !used {
            return used;
        }
        match key {
            Key::Letter('n') => {
                self.deal();
                return true;
            }
            // T for "three". (D can't be used: W A S D double as arrow keys.)
            Key::Letter('t') => {
                self.draw_three = !self.draw_three;
                self.deal();
                return true;
            }
            _ => {}
        }
        match self.state {
            GameState::Paused => {
                if matches!(key, Key::Action | Key::Pause) {
                    self.state = GameState::Playing;
                }
            }
            GameState::Over => {
                if key == Key::Action {
                    self.deal();
                }
            }
            GameState::Ready | GameState::Playing => match key {
                Key::Pause => self.pause(),
                Key::Action => self.draw(),
                Key::Letter('z') => self.undo(),
                Key::Letter('h') => self.show_hint(),
                _ => {}
            },
        }
        true
    }

    pub fn pointer_down(&mut self, x: f64, y: f64) -> bool {
        if !self.active() {
            return false;
        }
        self.hint = None;
        match self.card_at(x, y) {
            Some((Pile::Stock, _)) => {
                self.draw();
                true
            }
            Some((p, idx)) if self.movable(p, idx) => {
                let (cx, cy) = self.draw_pos(p, idx);
                // Grabbing a card mid-flight: let it land where it is.
                self.flights.retain(|f| f.pile != p || f.idx < idx);
                self.drag = Some(Drag {
                    from: p,
                    idx,
                    grab: (x - cx, y - cy),
                    start: (x, y),
                    at: (x, y),
                    moved: false,
                });
                true
            }
            _ => false,
        }
    }

    pub fn pointer_move(&mut self, x: f64, y: f64) {
        if let Some(d) = &mut self.drag {
            if x < 0.0 && y < 0.0 {
                return; // "left the canvas" signal; the page still tracks the drag
            }
            d.at = (x, y);
            if (x - d.start.0).hypot(y - d.start.1) > DRAG_SLOP {
                d.moved = true;
            }
        }
    }

    /// Top-left where the k-th card of a drag is drawn.
    fn dragged_card_pos(&self, d: &Drag, k: usize) -> (f64, f64) {
        let (bx, by) = self.card_pos(d.from, d.idx);
        let (cx, cy) = self.card_pos(d.from, d.idx + k);
        (d.at.0 - d.grab.0 + (cx - bx), d.at.1 - d.grab.1 + (cy - by))
    }

    pub fn drag_pos(&self, k: usize) -> Option<(f64, f64)> {
        self.drag.as_ref().map(|d| self.dragged_card_pos(d, k))
    }

    pub fn pointer_up(&mut self, x: f64, y: f64) {
        self.pointer_move(x, y);
        let Some(d) = self.drag.take() else { return };
        if !d.moved {
            self.auto_move(d.from, d.idx);
            return;
        }
        let run = self.pile(d.from).len() - d.idx;
        let starts: Vec<(f64, f64)> = (0..run).map(|k| self.dragged_card_pos(&d, k)).collect();
        let top = starts[0];
        let card = self.pile(d.from)[d.idx];
        // Drop on whichever legal pile the card overlaps most.
        let target = (0..4)
            .map(Pile::Found)
            .chain((0..7).map(Pile::Tab))
            .filter(|&p| p != d.from && self.can_drop(card, run, p))
            .map(|p| {
                let len = self.pile(p).len();
                let at = if len == 0 { slot(p) } else { self.card_pos(p, len - 1) };
                (overlap(top, at), p)
            })
            .filter(|(area, _)| *area > 0.0)
            .max_by(|a, b| a.0.total_cmp(&b.0));
        if let Some((_, to)) = target {
            if self.try_move_from(d.from, d.idx, to, starts.clone()) {
                return;
            }
        }
        // Not a legal drop: slide back home.
        for (k, from) in starts.into_iter().enumerate() {
            self.flights.push(Flight { pile: d.from, idx: d.idx + k, from, t: 0.0 });
        }
    }

    pub fn update(&mut self, dt_ms: f64) {
        self.time += dt_ms / 1000.0;
        if self.state != GameState::Playing {
            return;
        }
        for f in &mut self.flights {
            f.t += dt_ms / FLIGHT_MS;
        }
        self.flights.retain(|f| f.t < 1.0);
        if self.hint_ms > 0.0 {
            self.hint_ms -= dt_ms;
            if self.hint_ms <= 0.0 {
                self.hint = None;
            }
        }
        if self.no_moves_ms > 0.0 {
            self.no_moves_ms -= dt_ms;
        }
        if self.won {
            if self.flights.is_empty() {
                self.end_ms -= dt_ms;
                if self.end_ms <= 0.0 {
                    self.state = GameState::Over;
                }
            }
            return;
        }
        if self.started {
            self.elapsed += dt_ms / 1000.0;
        }
        if self.can_auto_finish() {
            self.auto_ms -= dt_ms;
            if self.auto_ms <= 0.0 {
                self.auto_ms = AUTO_MS;
                self.auto_step();
            }
        }
    }

    pub fn pause(&mut self) {
        if self.state == GameState::Playing && self.started && !self.won {
            self.state = GameState::Paused;
            self.drag = None;
        }
    }

    pub fn end_message(&self) -> Option<String> {
        self.won.then(|| {
            let s = self.elapsed.ceil() as u32;
            format!("You won in {}:{:02}!", s / 60, s % 60)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn c(rank: u8, suit: u8) -> Card {
        Card { rank, suit, up: true }
    }

    /// An empty table to arrange by hand.
    fn empty() -> Klondike {
        let mut g = Klondike::new(1);
        g.stock.clear();
        g.waste.clear();
        g.tab = Default::default();
        g.found = Default::default();
        g
    }

    #[test]
    fn deal_lays_out_all_52_cards() {
        let g = Klondike::new(42);
        for t in 0..7 {
            assert_eq!(g.tab[t].len(), t + 1);
            assert!(g.tab[t].last().unwrap().up);
            assert!(g.tab[t][..t].iter().all(|c| !c.up));
        }
        assert_eq!(g.stock.len(), 24);
        let mut all: Vec<(u8, u8)> = g.tab.iter().flatten().chain(&g.stock).map(|c| (c.suit, c.rank)).collect();
        all.sort();
        all.dedup();
        assert_eq!(all.len(), 52);
    }

    #[test]
    fn tableau_needs_alternating_colours_one_lower() {
        let mut g = empty();
        g.tab[0] = vec![c(8, 0)]; // black 8
        assert!(g.can_drop(c(7, 1), 1, Pile::Tab(0)), "red 7 on black 8");
        assert!(!g.can_drop(c(7, 3), 1, Pile::Tab(0)), "black 7 on black 8");
        assert!(!g.can_drop(c(6, 1), 1, Pile::Tab(0)), "must be one lower");
        assert!(g.can_drop(c(13, 2), 1, Pile::Tab(1)), "king on empty column");
        assert!(!g.can_drop(c(12, 2), 1, Pile::Tab(1)), "only kings on empty columns");
    }

    #[test]
    fn foundations_build_up_by_suit_from_ace() {
        let mut g = empty();
        assert!(g.can_drop(c(1, 2), 1, Pile::Found(0)));
        assert!(!g.can_drop(c(2, 2), 1, Pile::Found(0)));
        g.found[0] = vec![c(1, 2)];
        assert!(g.can_drop(c(2, 2), 1, Pile::Found(0)));
        assert!(!g.can_drop(c(2, 1), 1, Pile::Found(0)), "same suit only");
        assert!(!g.can_drop(c(2, 2), 2, Pile::Found(0)), "one card at a time");
    }

    #[test]
    fn moving_a_run_turns_over_the_card_beneath() {
        let mut g = empty();
        g.tab[0] = vec![Card { up: false, ..c(2, 0) }, c(9, 3), c(8, 1)];
        g.tab[1] = vec![c(10, 2)];
        assert!(g.try_move(Pile::Tab(0), 1, Pile::Tab(1)));
        assert_eq!(g.tab[1].len(), 3);
        assert!(g.tab[0][0].up);
        assert_eq!(g.points, TURN_OVER);
    }

    #[test]
    fn click_sends_aces_and_twos_to_foundations() {
        let mut g = empty();
        g.tab[0] = vec![c(1, 1)];
        g.tab[1] = vec![c(2, 1)];
        assert!(g.auto_move(Pile::Tab(0), 0));
        assert!(g.auto_move(Pile::Tab(1), 0));
        assert_eq!(g.found.iter().map(Vec::len).sum::<usize>(), 2);
        assert_eq!(g.points, 2 * TO_FOUNDATION);
    }

    #[test]
    fn click_prefers_a_column_with_cards_over_an_empty_one() {
        let mut g = empty();
        g.tab[0] = vec![c(5, 0), c(13, 1)]; // red king on top of something
        g.tab[3] = vec![];
        assert!(g.auto_move(Pile::Tab(0), 1));
        assert!(g.tab.iter().any(|t| t.len() == 1 && t[0].rank == 13));
        // A king already at the bottom of a column stays put.
        let mut g = empty();
        g.tab[0] = vec![c(13, 1)];
        assert!(!g.auto_move(Pile::Tab(0), 0));
    }

    #[test]
    fn draw_one_and_draw_three() {
        let mut g = Klondike::new(3);
        g.draw();
        assert_eq!((g.stock.len(), g.waste.len()), (23, 1));
        assert!(g.waste[0].up);
        g.draw_three = true;
        g.deal();
        g.draw();
        assert_eq!((g.stock.len(), g.waste.len()), (21, 3));
    }

    #[test]
    fn empty_stock_recycles_the_waste_in_order() {
        let mut g = empty();
        g.stock = vec![Card { up: false, ..c(1, 0) }, Card { up: false, ..c(2, 0) }];
        g.draw();
        g.draw();
        let order: Vec<u8> = g.waste.iter().map(|c| c.rank).collect();
        g.draw();
        assert!(g.waste.is_empty());
        assert_eq!(g.points, RECYCLE);
        g.draw();
        g.draw();
        assert_eq!(g.waste.iter().map(|c| c.rank).collect::<Vec<_>>(), order);
    }

    #[test]
    fn undo_restores_cards_and_score() {
        let mut g = empty();
        g.tab[0] = vec![c(1, 0)];
        g.auto_move(Pile::Tab(0), 0);
        g.undo();
        assert_eq!(g.tab[0].len(), 1);
        assert!(g.found[0].is_empty());
        assert_eq!(g.points, 0);
    }

    #[test]
    fn drag_and_drop_onto_a_legal_column() {
        let mut g = empty();
        g.tab[0] = vec![c(9, 0)];
        g.tab[4] = vec![c(8, 1)];
        let (sx, sy) = g.card_pos(Pile::Tab(4), 0);
        assert!(g.pointer_down(sx + 10.0, sy + 10.0));
        let (tx, ty) = g.card_pos(Pile::Tab(0), 0);
        g.pointer_move(tx + 10.0, ty + 30.0);
        g.pointer_up(tx + 10.0, ty + 30.0);
        assert_eq!(g.tab[0].len(), 2);
        assert!(g.tab[4].is_empty());
    }

    #[test]
    fn illegal_drop_slides_back() {
        let mut g = empty();
        g.tab[0] = vec![c(9, 1)]; // red 9: red 8 can't go here
        g.tab[4] = vec![c(8, 1)];
        let (sx, sy) = g.card_pos(Pile::Tab(4), 0);
        g.pointer_down(sx + 10.0, sy + 10.0);
        let (tx, ty) = g.card_pos(Pile::Tab(0), 0);
        g.pointer_up(tx + 10.0, ty + 30.0);
        assert_eq!(g.tab[4].len(), 1);
        assert_eq!(g.flights.len(), 1, "animates home");
    }

    #[test]
    fn tap_without_dragging_auto_moves() {
        let mut g = empty();
        g.tab[2] = vec![c(1, 3)];
        let (x, y) = g.card_pos(Pile::Tab(2), 0);
        g.pointer_down(x + 5.0, y + 5.0);
        g.pointer_up(x + 6.0, y + 6.0);
        assert_eq!(g.found[0].len(), 1);
    }

    #[test]
    fn finishes_itself_and_wins() {
        let mut g = empty();
        for f in 0..4 {
            g.found[f] = (1..=11).map(|r| c(r, f as u8)).collect();
        }
        // Queens and kings left, all face up.
        g.tab[0] = vec![c(13, 0), c(12, 1)];
        g.tab[1] = vec![c(13, 1), c(12, 0)];
        g.tab[2] = vec![c(13, 2), c(12, 3)];
        g.tab[3] = vec![c(13, 3), c(12, 2)];
        g.started = true;
        assert!(g.can_auto_finish());
        for _ in 0..200 {
            g.update(50.0);
        }
        assert!(g.won);
        assert_eq!(g.state, GameState::Over);
        assert!(g.end_message().unwrap().starts_with("You won"));
        assert!(g.points >= 8 * TO_FOUNDATION + WIN_BONUS);
    }

    #[test]
    fn hint_suggests_foundation_first_then_drawing() {
        let mut g = empty();
        g.tab[0] = vec![c(1, 0)];
        g.stock = vec![Card { up: false, ..c(5, 1) }];
        assert_eq!(g.find_hint().unwrap().to, Some(Pile::Found(0)));
        g.tab[0].clear();
        g.tab[0].push(c(5, 0));
        assert_eq!(g.find_hint().unwrap(), Hint { from: Pile::Stock, idx: 0, to: None });
    }

    #[test]
    fn long_columns_squeeze_to_fit() {
        let mut g = empty();
        g.tab[0] = (0..6).map(|_| Card { up: false, ..c(2, 0) }).collect();
        g.tab[0].extend((1..=13).rev().map(|r| c(r, r % 2)));
        let last = g.tab[0].len() - 1;
        let (_, y) = g.card_pos(Pile::Tab(0), last);
        assert!(y + CH <= H - FOOTER + 0.01);
    }

    #[test]
    fn t_switches_draw_mode_with_a_new_deal() {
        let mut g = Klondike::new(8);
        g.handle_key(Key::Letter('t'), true);
        assert!(g.draw_three);
        assert_eq!(g.stock.len(), 24);
    }
}
