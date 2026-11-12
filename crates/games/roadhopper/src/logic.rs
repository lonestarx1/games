//! Road Hopper rules. Hop a bunny up an endless trail of meadows, roads,
//! rivers and railway lines. Cars and trains flatten you, water drowns you
//! unless you're riding a log, and the view keeps creeping up behind you.
//! No browser code here, so it's all unit-testable.

use engine::{GameState, Key, Rng};
use std::collections::VecDeque;

pub const COLS: usize = 9;
pub const CELL: f64 = 48.0;
pub const VISIBLE_ROWS: f64 = 13.0;
pub const W: f64 = COLS as f64 * CELL;
pub const H: f64 = VISIBLE_ROWS * CELL;

pub const HOP_MS: f64 = 130.0;
/// Rows of safe meadow at the start.
const START_ROWS: i64 = 4;
/// The camera keeps the bunny about this many rows above the bottom edge.
const FOLLOW_ROWS: f64 = 4.0;
/// Movers live on a loop a bit wider than the screen so they slide in from off-screen.
const LOOP_PAD: f64 = 4.0;
const DEATH_MS: f64 = 1100.0;
const BERRY_POINTS: u32 = 10;
/// Train: how long its warning light flashes, and its speed in cells/s.
pub const WARN_S: f64 = 1.3;
const TRAIN_SPEED: f64 = 26.0;
const TRAIN_LEN: f64 = 14.0;
const SWIPE_MIN: f64 = 18.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LaneKind {
    Grass,
    Road,
    River,
    Rail,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Mover {
    /// Left edge, in cells.
    pub x: f64,
    pub len: f64,
    /// Picks a colour/style in the renderer.
    pub style: u8,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Lane {
    pub kind: LaneKind,
    /// +1 moves right, -1 moves left.
    pub dir: f64,
    /// Cells per second.
    pub speed: f64,
    pub movers: Vec<Mover>,
    /// Grass only: trees and rocks you can't hop into.
    pub blocked: [bool; COLS],
    pub berry: Option<usize>,
    /// Rail only: seconds until the next train arrives (negative while it passes).
    pub train_in: f64,
}

impl Lane {
    fn grass() -> Lane {
        Lane { kind: LaneKind::Grass, dir: 1.0, speed: 0.0, movers: Vec::new(), blocked: [false; COLS], berry: None, train_in: 0.0 }
    }

    /// Rail lanes: is the warning light on?
    pub fn warning(&self) -> bool {
        self.kind == LaneKind::Rail && self.train_in > 0.0 && self.train_in < WARN_S
    }

    /// Does anything on this lane cover the span [a, b] (in cells)?
    fn covers(&self, a: f64, b: f64) -> bool {
        self.movers.iter().any(|m| m.x < b && m.x + m.len > a)
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Hop {
    pub from: (f64, i64),
    pub to: (f64, i64),
    pub t: f64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Death {
    Squashed,
    Splashed,
    SweptAway,
    TooSlow,
}

pub struct RoadHopper {
    /// lanes[0] is row `base_row`; rows count upward from 0 at the start.
    pub lanes: VecDeque<Lane>,
    pub base_row: i64,
    /// Bunny column (fractional while riding a log) and row.
    pub px: f64,
    pub prow: i64,
    pub hop: Option<Hop>,
    queued: Option<(i32, i32)>,
    /// Facing for the sprite: (dx, dy) of the last hop.
    pub facing: (i32, i32),
    /// Bottom visible row (fractional, scrolls smoothly).
    pub cam: f64,
    pub best_row: i64,
    pub berries: u32,
    pub state: GameState,
    pub death: Option<Death>,
    pub death_ms: f64,
    pub time: f64,
    moved: bool,
    swipe_from: Option<(f64, f64)>,
    rng: Rng,
    /// Lane generation: what to repeat next, and how many more of it.
    run: (LaneKind, u32),
}

impl RoadHopper {
    pub fn new(seed: u64) -> RoadHopper {
        let mut g = RoadHopper {
            lanes: VecDeque::new(),
            base_row: 0,
            px: (COLS / 2) as f64,
            prow: 0,
            hop: None,
            queued: None,
            facing: (0, 1),
            cam: 0.0,
            best_row: 0,
            berries: 0,
            state: GameState::Ready,
            death: None,
            death_ms: 0.0,
            time: 0.0,
            moved: false,
            swipe_from: None,
            rng: Rng::new(seed),
            run: (LaneKind::Grass, 0),
        };
        g.reset();
        g
    }

    pub fn reset(&mut self) {
        self.lanes.clear();
        self.base_row = 0;
        self.px = (COLS / 2) as f64;
        self.prow = 0;
        self.hop = None;
        self.queued = None;
        self.facing = (0, 1);
        self.cam = 0.0;
        self.best_row = 0;
        self.berries = 0;
        self.state = GameState::Ready;
        self.death = None;
        self.death_ms = 0.0;
        self.moved = false;
        self.run = (LaneKind::Grass, 0);
        self.ensure_lanes();
    }

    pub fn score(&self) -> u32 {
        self.best_row.max(0) as u32 + self.berries * BERRY_POINTS
    }

    pub fn lane(&self, row: i64) -> Option<&Lane> {
        usize::try_from(row - self.base_row).ok().and_then(|i| self.lanes.get(i))
    }

    fn lane_mut(&mut self, row: i64) -> Option<&mut Lane> {
        usize::try_from(row - self.base_row).ok().and_then(move |i| self.lanes.get_mut(i))
    }

    /// Keeps lanes generated a little past the top of the screen and drops
    /// the ones far below.
    fn ensure_lanes(&mut self) {
        let top = self.cam.floor() as i64 + VISIBLE_ROWS as i64 + 3;
        while self.base_row + (self.lanes.len() as i64) <= top {
            let row = self.base_row + self.lanes.len() as i64;
            let lane = self.make_lane(row);
            self.lanes.push_back(lane);
        }
        while self.base_row < self.cam.floor() as i64 - 3 {
            self.lanes.pop_front();
            self.base_row += 1;
        }
    }

    fn make_lane(&mut self, row: i64) -> Lane {
        if row < START_ROWS {
            let mut lane = Lane::grass();
            // A few trees on the edges of the start meadow, never in the middle.
            if row > 0 {
                lane.blocked[0] = self.rng.unit() < 0.5;
                lane.blocked[COLS - 1] = self.rng.unit() < 0.5;
            }
            return lane;
        }
        if self.run.1 == 0 {
            // Pick the next kind of terrain and how many lanes of it.
            let roll = self.rng.unit();
            let kind = if self.run.0 != LaneKind::Grass && roll < 0.35 {
                LaneKind::Grass
            } else if roll < 0.62 {
                LaneKind::Road
            } else if roll < 0.86 || row < 15 {
                LaneKind::River
            } else {
                LaneKind::Rail
            };
            let count = match kind {
                LaneKind::Grass => 1 + self.rng.below(2) as u32,
                LaneKind::Road => 1 + self.rng.below(4) as u32,
                LaneKind::River => 1 + self.rng.below(3) as u32,
                LaneKind::Rail => 1 + self.rng.below(2) as u32,
            };
            // Gentle start: short stretches of road and river early on.
            let count = if row < 15 { count.min(2) } else { count };
            // Never two runs of the same kind back to back (grass breaks them up).
            let kind = if kind == self.run.0 && kind != LaneKind::Grass { LaneKind::Grass } else { kind };
            self.run = (kind, count);
        }
        self.run.1 -= 1;
        let level = (row as f64 / 25.0).min(4.0); // difficulty 0..4
        let dir = if self.rng.unit() < 0.5 { -1.0 } else { 1.0 };
        match self.run.0 {
            LaneKind::Grass => {
                let mut lane = Lane::grass();
                let trees = self.rng.below(4);
                for _ in 0..trees {
                    lane.blocked[self.rng.below(COLS)] = true;
                }
                // Always leave a way through.
                let open = self.rng.below(COLS);
                lane.blocked[open] = false;
                if self.rng.unit() < 0.25 {
                    let c = self.rng.below(COLS);
                    if !lane.blocked[c] {
                        lane.berry = Some(c);
                    }
                }
                lane
            }
            LaneKind::Road => {
                let speed = self.rng.range(1.4, 2.4) + level * 0.7;
                let truck = self.rng.unit() < 0.3;
                let (len, gap_min, gap_max) = if truck { (2.0, 3.5, 6.0) } else { (1.0, 2.5 - level * 0.2, 5.0 - level * 0.4) };
                let movers = self.fill(len, len, gap_min, gap_max, if truck { 10 } else { 0 });
                Lane { kind: LaneKind::Road, dir, speed, movers, blocked: [false; COLS], berry: None, train_in: 0.0 }
            }
            LaneKind::River => {
                let speed = self.rng.range(0.9, 1.6) + level * 0.35;
                let movers = self.fill(2.0, 4.0 - level * 0.3, 1.0, 2.6 + level * 0.3, 20);
                Lane { kind: LaneKind::River, dir, speed, movers, blocked: [false; COLS], berry: None, train_in: 0.0 }
            }
            LaneKind::Rail => {
                let wait = self.rng.range(2.5, 6.0);
                Lane { kind: LaneKind::Rail, dir, speed: TRAIN_SPEED, movers: Vec::new(), blocked: [false; COLS], berry: None, train_in: wait }
            }
        }
    }

    /// Places movers around the lane's loop with random lengths and gaps.
    fn fill(&mut self, len_min: f64, len_max: f64, gap_min: f64, gap_max: f64, style_base: u8) -> Vec<Mover> {
        let span = COLS as f64 + 2.0 * LOOP_PAD;
        let mut x = -LOOP_PAD + self.rng.range(0.0, 2.0);
        let mut out = Vec::new();
        loop {
            let len = self.rng.range(len_min, len_max.max(len_min)).round().max(1.0);
            if x + len > span - LOOP_PAD + 0.01 {
                break;
            }
            out.push(Mover { x, len, style: style_base + self.rng.below(5) as u8 });
            x += len + self.rng.range(gap_min.max(1.0), gap_max.max(gap_min.max(1.0) + 0.5));
        }
        out
    }

    // ------------------------------------------------------------ input

    pub fn handle_key(&mut self, key: Key, pressed: bool) -> bool {
        let used = key.is_direction() || matches!(key, Key::Action | Key::Pause);
        if !pressed || !used {
            return used;
        }
        let dir = match key {
            Key::Up => Some((0, 1)),
            Key::Down => Some((0, -1)),
            Key::Left => Some((-1, 0)),
            Key::Right => Some((1, 0)),
            _ => None,
        };
        match self.state {
            GameState::Ready => {
                self.state = GameState::Playing;
                self.try_hop(dir.unwrap_or((0, 1)));
            }
            GameState::Playing => match (key, dir) {
                (Key::Pause, _) => self.pause(),
                (_, Some(d)) => self.try_hop(d),
                (Key::Action, None) => self.try_hop((0, 1)),
                _ => {}
            },
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
        }
        true
    }

    pub fn pointer_down(&mut self, x: f64, y: f64) -> bool {
        self.swipe_from = Some((x, y));
        true
    }

    /// Tap hops forward; a swipe hops that way.
    pub fn pointer_up(&mut self, x: f64, y: f64) {
        let Some((sx, sy)) = self.swipe_from.take() else { return };
        let (dx, dy) = (x - sx, y - sy);
        let dir = if dx.abs().max(dy.abs()) < SWIPE_MIN {
            (0, 1)
        } else if dx.abs() > dy.abs() {
            (dx.signum() as i32, 0)
        } else {
            (0, if dy < 0.0 { 1 } else { -1 })
        };
        match self.state {
            GameState::Ready => {
                self.state = GameState::Playing;
                self.try_hop(dir);
            }
            GameState::Playing => self.try_hop(dir),
            _ => {}
        }
    }

    fn try_hop(&mut self, (dx, dy): (i32, i32)) {
        if self.death.is_some() {
            return;
        }
        if self.hop.is_some() {
            self.queued = Some((dx, dy)); // buffer one hop
            return;
        }
        // Land on whole columns (a bunny riding a log snaps to the nearest one).
        let col = (self.px.round() + dx as f64).clamp(0.0, COLS as f64 - 1.0);
        let row = self.prow + dy as i64;
        if row < self.cam.floor() as i64 {
            return; // can't hop off the bottom of the screen
        }
        if let Some(lane) = self.lane(row) {
            if lane.kind == LaneKind::Grass && lane.blocked[col as usize] {
                self.facing = (dx, dy);
                return;
            }
        }
        self.facing = (dx, dy);
        self.hop = Some(Hop { from: (self.px, self.prow), to: (col, row), t: 0.0 });
        self.moved = true;
    }

    pub fn pause(&mut self) {
        if self.state == GameState::Playing && self.death.is_none() {
            self.state = GameState::Paused;
        }
        self.swipe_from = None;
    }

    // ------------------------------------------------------------ simulation

    /// How fast the view creeps up once you've started, in rows per second.
    pub fn creep_speed(&self) -> f64 {
        (0.28 + self.best_row as f64 * 0.006).min(0.9)
    }

    pub fn update(&mut self, dt_ms: f64) {
        let dt = (dt_ms / 1000.0).min(0.05);
        self.time += dt;
        if self.state != GameState::Playing {
            return;
        }
        self.move_lanes(dt);
        if self.death.is_some() {
            self.death_ms -= dt_ms;
            if self.death_ms <= 0.0 {
                self.state = GameState::Over;
            }
            return;
        }
        self.advance_hop(dt_ms);
        self.ride(dt);
        self.move_camera(dt);
        self.check_hazards();
        self.ensure_lanes();
    }

    fn move_lanes(&mut self, dt: f64) {
        let span = COLS as f64 + 2.0 * LOOP_PAD;
        for lane in &mut self.lanes {
            match lane.kind {
                LaneKind::Road | LaneKind::River => {
                    for m in &mut lane.movers {
                        m.x += lane.dir * lane.speed * dt;
                        // Wrap around the loop.
                        if m.x > COLS as f64 + LOOP_PAD {
                            m.x -= span + m.len;
                        } else if m.x + m.len < -LOOP_PAD {
                            m.x += span + m.len;
                        }
                    }
                }
                LaneKind::Rail => {
                    let was = lane.train_in;
                    lane.train_in -= dt;
                    if was > 0.0 && lane.train_in <= 0.0 {
                        let x = if lane.dir > 0.0 { -TRAIN_LEN - 1.0 } else { COLS as f64 + 1.0 };
                        lane.movers = vec![Mover { x, len: TRAIN_LEN, style: 30 }];
                    }
                    for m in &mut lane.movers {
                        m.x += lane.dir * lane.speed * dt;
                    }
                    lane.movers.retain(|m| m.x < COLS as f64 + 2.0 && m.x + m.len > -2.0);
                    if lane.train_in <= 0.0 && lane.movers.is_empty() {
                        lane.train_in = 3.0 + (lane.speed * 7.3 + was * 13.1).sin().abs() * 4.0;
                    }
                }
                LaneKind::Grass => {}
            }
        }
    }

    fn advance_hop(&mut self, dt_ms: f64) {
        let Some(mut hop) = self.hop else { return };
        hop.t += dt_ms / HOP_MS;
        if hop.t < 1.0 {
            self.hop = Some(hop);
            self.px = hop.from.0 + (hop.to.0 - hop.from.0) * hop.t;
            return;
        }
        // Landed.
        self.hop = None;
        self.px = hop.to.0;
        self.prow = hop.to.1;
        if self.prow > self.best_row {
            self.best_row = self.prow;
        }
        let (row, col) = (self.prow, self.px as usize);
        if let Some(lane) = self.lane_mut(row) {
            if lane.berry == Some(col) {
                lane.berry = None;
                self.berries += 1;
            }
        }
        if let Some(d) = self.queued.take() {
            self.try_hop(d);
        }
    }

    /// The row the bunny counts as being on (it switches halfway through a hop).
    pub fn current_row(&self) -> i64 {
        match self.hop {
            Some(h) if h.t >= 0.5 => h.to.1,
            _ => self.prow,
        }
    }

    /// Riding a log carries you along.
    fn ride(&mut self, dt: f64) {
        if self.hop.is_some() {
            return;
        }
        if let Some(lane) = self.lane(self.prow) {
            if lane.kind == LaneKind::River && lane.covers(self.px + 0.3, self.px + 0.7) {
                self.px += lane.dir * lane.speed * dt;
            }
        }
    }

    fn move_camera(&mut self, dt: f64) {
        // Follow the bunny up, and creep on our own once the game is going.
        let follow = self.current_row() as f64 - FOLLOW_ROWS;
        if follow > self.cam {
            self.cam += (follow - self.cam) * (dt * 8.0).min(1.0);
        }
        if self.moved {
            self.cam += self.creep_speed() * dt;
        }
    }

    fn check_hazards(&mut self) {
        // Fell behind the view.
        if (self.current_row() as f64) < self.cam - 0.35 {
            return self.die(Death::TooSlow);
        }
        let row = self.current_row();
        let Some(lane) = self.lane(row) else { return };
        let (a, b) = (self.px + 0.2, self.px + 0.8);
        match lane.kind {
            LaneKind::Road | LaneKind::Rail if lane.covers(a, b) => self.die(Death::Squashed),
            LaneKind::River if self.hop.is_none() => {
                if self.px < -0.45 || self.px > COLS as f64 - 0.55 {
                    self.die(Death::SweptAway);
                } else if !lane.covers(self.px + 0.3, self.px + 0.7) {
                    self.die(Death::Splashed);
                }
            }
            _ => {}
        }
    }

    fn die(&mut self, how: Death) {
        self.death = Some(how);
        self.death_ms = DEATH_MS;
        self.hop = None;
        self.queued = None;
    }

    pub fn end_message(&self) -> Option<String> {
        let what = match self.death? {
            Death::Squashed => "Squashed!",
            Death::Splashed => "Splash!",
            Death::SweptAway => "Swept away!",
            Death::TooSlow => "Too slow!",
        };
        Some(format!("{what} · {} rows", self.best_row))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn playing() -> RoadHopper {
        let mut g = RoadHopper::new(3);
        g.state = GameState::Playing;
        g
    }

    /// Replaces row `row` with a lane we control.
    fn set_lane(g: &mut RoadHopper, row: i64, lane: Lane) {
        *g.lane_mut(row).unwrap() = lane;
    }

    fn road(movers: Vec<Mover>, speed: f64) -> Lane {
        Lane { kind: LaneKind::Road, dir: 1.0, speed, movers, blocked: [false; COLS], berry: None, train_in: 0.0 }
    }

    fn river(movers: Vec<Mover>, speed: f64) -> Lane {
        Lane { kind: LaneKind::River, dir: 1.0, speed, movers, blocked: [false; COLS], berry: None, train_in: 0.0 }
    }

    fn run(g: &mut RoadHopper, ms: f64) {
        let mut t = 0.0;
        while t < ms {
            g.update(16.0);
            t += 16.0;
        }
    }

    #[test]
    fn starts_on_safe_meadow() {
        let g = RoadHopper::new(1);
        for r in 0..START_ROWS {
            assert_eq!(g.lane(r).unwrap().kind, LaneKind::Grass);
        }
        assert!(g.lane(0).unwrap().blocked.iter().all(|b| !b));
    }

    #[test]
    fn hopping_forward_scores_a_row() {
        let mut g = playing();
        g.handle_key(Key::Up, true);
        run(&mut g, HOP_MS + 20.0);
        assert_eq!(g.prow, 1);
        assert_eq!(g.score(), 1);
        g.handle_key(Key::Down, true);
        run(&mut g, HOP_MS + 20.0);
        assert_eq!(g.prow, 0);
        assert_eq!(g.score(), 1, "going back doesn't lose or add points");
    }

    #[test]
    fn trees_block_hops() {
        let mut g = playing();
        let mut lane = Lane::grass();
        lane.blocked[4] = true;
        set_lane(&mut g, 1, lane);
        g.handle_key(Key::Up, true);
        run(&mut g, HOP_MS + 20.0);
        assert_eq!(g.prow, 0);
    }

    #[test]
    fn edges_stop_the_bunny() {
        let mut g = playing();
        for _ in 0..8 {
            g.handle_key(Key::Left, true);
            run(&mut g, HOP_MS + 20.0);
        }
        assert_eq!(g.px, 0.0);
    }

    #[test]
    fn a_car_squashes() {
        let mut g = playing();
        set_lane(&mut g, 1, road(vec![Mover { x: 4.0, len: 1.0, style: 0 }], 0.0));
        g.handle_key(Key::Up, true);
        run(&mut g, HOP_MS + 20.0);
        assert_eq!(g.death, Some(Death::Squashed));
        run(&mut g, DEATH_MS + 50.0);
        assert_eq!(g.state, GameState::Over);
        assert!(g.end_message().unwrap().starts_with("Squashed"));
    }

    #[test]
    fn empty_road_is_safe() {
        let mut g = playing();
        set_lane(&mut g, 1, road(vec![Mover { x: 8.0, len: 1.0, style: 0 }], 0.0));
        g.handle_key(Key::Up, true);
        run(&mut g, HOP_MS + 100.0);
        assert!(g.death.is_none());
    }

    #[test]
    fn water_drowns_but_logs_carry() {
        let mut g = playing();
        set_lane(&mut g, 1, river(vec![], 1.0));
        g.handle_key(Key::Up, true);
        run(&mut g, HOP_MS + 20.0);
        assert_eq!(g.death, Some(Death::Splashed));

        let mut g = playing();
        set_lane(&mut g, 1, river(vec![Mover { x: 3.0, len: 3.0, style: 20 }], 1.0));
        g.handle_key(Key::Up, true);
        run(&mut g, HOP_MS + 500.0);
        assert!(g.death.is_none());
        assert!(g.px > 4.2, "carried right by the log");
    }

    #[test]
    fn riding_off_the_edge_sweeps_you_away() {
        let mut g = playing();
        set_lane(&mut g, 1, river(vec![Mover { x: 3.0, len: 9.0, style: 20 }], 3.0));
        g.handle_key(Key::Up, true);
        run(&mut g, 3000.0);
        assert_eq!(g.death, Some(Death::SweptAway));
    }

    #[test]
    fn hopping_off_a_log_lands_on_a_whole_column() {
        let mut g = playing();
        set_lane(&mut g, 1, river(vec![Mover { x: 3.0, len: 4.0, style: 20 }], 1.0));
        g.handle_key(Key::Up, true);
        run(&mut g, HOP_MS + 300.0);
        g.handle_key(Key::Down, true);
        run(&mut g, HOP_MS + 20.0);
        assert_eq!(g.px, g.px.round());
    }

    #[test]
    fn the_view_creeps_up_and_catches_dawdlers() {
        let mut g = playing();
        g.handle_key(Key::Up, true);
        run(&mut g, HOP_MS + 20.0);
        g.handle_key(Key::Down, true);
        run(&mut g, 8000.0);
        assert_eq!(g.death, Some(Death::TooSlow));
    }

    #[test]
    fn berries_score_bonus() {
        let mut g = playing();
        let mut lane = Lane::grass();
        lane.berry = Some(4);
        set_lane(&mut g, 1, lane);
        g.handle_key(Key::Up, true);
        run(&mut g, HOP_MS + 20.0);
        assert_eq!(g.berries, 1);
        assert_eq!(g.score(), 1 + BERRY_POINTS);
        assert_eq!(g.lane(1).unwrap().berry, None);
    }

    #[test]
    fn trains_warn_then_pass() {
        let mut g = playing();
        let rail = Lane { kind: LaneKind::Rail, dir: 1.0, speed: TRAIN_SPEED, movers: vec![], blocked: [false; COLS], berry: None, train_in: WARN_S + 0.5 };
        set_lane(&mut g, 1, rail);
        assert!(!g.lane(1).unwrap().warning());
        g.move_lanes(0.6);
        assert!(g.lane(1).unwrap().warning(), "light flashes before the train");
        let mut arrived = false;
        for _ in 0..40 {
            g.move_lanes(0.05);
            arrived |= g.lane(1).unwrap().movers.len() == 1;
        }
        assert!(arrived, "train arrives");
        for _ in 0..100 {
            g.move_lanes(0.05);
        }
        let lane = g.lane(1).unwrap();
        assert!(lane.movers.is_empty() && lane.train_in > 0.0, "train gone, next one scheduled");
    }

    #[test]
    fn lanes_keep_generating_and_old_ones_drop() {
        let mut g = playing();
        g.cam = 200.0;
        g.ensure_lanes();
        assert!(g.lane(215).is_some());
        assert!(g.lane(10).is_none());
        // Every grass lane leaves a gap to walk through.
        for lane in &g.lanes {
            if lane.kind == LaneKind::Grass {
                assert!(lane.blocked.iter().any(|b| !b));
            }
        }
    }

    #[test]
    fn swipes_and_taps_hop() {
        let mut g = RoadHopper::new(8);
        g.pointer_down(200.0, 300.0);
        g.pointer_up(201.0, 301.0);
        assert_eq!(g.state, GameState::Playing);
        assert_eq!(g.hop.unwrap().to.1, 1, "tap hops forward");
        run(&mut g, HOP_MS + 20.0);
        g.pointer_down(200.0, 300.0);
        g.pointer_up(140.0, 305.0);
        assert_eq!(g.hop.unwrap().to.0, 3.0, "swipe left");
    }

    #[test]
    fn space_restarts_after_game_over() {
        let mut g = playing();
        g.best_row = 30;
        g.state = GameState::Over;
        g.handle_key(Key::Action, true);
        assert_eq!(g.state, GameState::Playing);
        assert_eq!(g.score(), 0);
    }
}
