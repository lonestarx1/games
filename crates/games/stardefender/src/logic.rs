//! Star Defender rules. Waves of aliens fly in, hover in a swaying formation
//! that creeps down and speeds up, and send divers at your ship. Shoot them
//! all to start the next, harder wave. No browser code here, so it's testable.

use engine::{GameState, Key, Rng};
use std::f64::consts::PI;

pub const W: f64 = 480.0;
pub const H: f64 = 640.0;

pub const SHIP_Y: f64 = H - 56.0;
pub const SHIP_W: f64 = 34.0;
pub const SHIP_H: f64 = 22.0;
const SHIP_SPEED: f64 = 300.0;
const SHOT_SPEED: f64 = 640.0;
const FIRE_MS: f64 = 280.0;
const RAPID_FIRE_MS: f64 = 120.0;
const INVULN_S: f64 = 2.0;
const START_LIVES: u32 = 3;
const EXTRA_LIFE_EVERY: u32 = 5000;

pub const COLS: usize = 8;
pub const ROWS: usize = 4;
pub const SPACING_X: f64 = 46.0;
pub const SPACING_Y: f64 = 40.0;
const FORM_TOP: f64 = 90.0;
const SWAY: f64 = 44.0;
/// Aliens this close to the ship's row have landed: game over.
const LAND_Y: f64 = SHIP_Y - 34.0;
pub const ALIEN_R: f64 = 15.0;

const ENTER_S: f64 = 1.4;
const ENTER_STAGGER_S: f64 = 0.05;
const RETURN_S: f64 = 1.1;
pub const BANNER_S: f64 = 1.8;
const DEATH_PAUSE_MS: f64 = 1500.0;

const POWER_CHANCE: f64 = 0.09;
const POWER_SPEED: f64 = 120.0;
const TWIN_S: f64 = 12.0;
const RAPID_S: f64 = 10.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AlienKind {
    /// Basic flyer, one hit.
    Drone,
    /// Likes to dive at you, one hit.
    Stinger,
    /// Armoured: two hits.
    Warden,
}

impl AlienKind {
    pub fn points(self) -> u32 {
        match self {
            AlienKind::Drone => 10,
            AlienKind::Stinger => 20,
            AlienKind::Warden => 40,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    Entering,
    Formation,
    Diving,
    Returning,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Alien {
    pub kind: AlienKind,
    pub hp: u8,
    pub col: usize,
    pub row: usize,
    pub mode: Mode,
    pub x: f64,
    pub y: f64,
    /// Seconds in the current mode (negative = waiting to enter).
    pub t: f64,
    /// Where the current flight started.
    sx: f64,
    sy: f64,
    /// Dive target and wobble.
    aim_x: f64,
    phase: f64,
    fired: bool,
    /// Seconds of white flash after being hit.
    pub flash: f64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Power {
    Twin,
    Rapid,
    Shield,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Drop {
    pub kind: Power,
    pub x: f64,
    pub y: f64,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Shot {
    pub x: f64,
    pub y: f64,
    pub vx: f64,
    pub vy: f64,
}

/// Explosion debris. `hue` picks the colour in the renderer.
pub struct Particle {
    pub x: f64,
    pub y: f64,
    pub vx: f64,
    pub vy: f64,
    pub life: f64,
    pub max: f64,
    pub hue: u8,
}

pub struct Star {
    pub x: f64,
    pub y: f64,
    /// 0 far (slow, dim) to 2 near (fast, bright).
    pub layer: u8,
}

pub struct StarDefender {
    pub ship_x: f64,
    pub lives: u32,
    pub score: u32,
    pub wave: u32,
    pub state: GameState,
    pub aliens: Vec<Alien>,
    pub shots: Vec<Shot>,
    pub bolts: Vec<Shot>,
    pub drops: Vec<Drop>,
    pub particles: Vec<Particle>,
    pub stars: Vec<Star>,
    /// Formation sway clock and how far it has crept down.
    pub sway_t: f64,
    pub descent: f64,
    pub invuln: f64,
    pub shield: bool,
    pub twin: f64,
    pub rapid: f64,
    /// Seconds the "Wave N" banner has left.
    pub banner: f64,
    pub shake: f64,
    /// Seconds since creation, for animations.
    pub time: f64,
    pub landed: bool,
    end_ms: f64,
    fire_cd: f64,
    dive_cd: f64,
    next_life: u32,
    left: bool,
    right: bool,
    firing: bool,
    /// Touch steering: the ship glides toward this x while a finger is down.
    touch_x: Option<f64>,
    rng: Rng,
}

fn ease(t: f64) -> f64 {
    let t = t.clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

impl StarDefender {
    pub fn new(seed: u64) -> StarDefender {
        let mut rng = Rng::new(seed);
        let stars = (0..70)
            .map(|i| Star { x: rng.range(0.0, W), y: rng.range(0.0, H), layer: (i % 3) as u8 })
            .collect();
        let mut g = StarDefender {
            ship_x: W / 2.0,
            lives: START_LIVES,
            score: 0,
            wave: 0,
            state: GameState::Ready,
            aliens: Vec::new(),
            shots: Vec::new(),
            bolts: Vec::new(),
            drops: Vec::new(),
            particles: Vec::new(),
            stars,
            sway_t: 0.0,
            descent: 0.0,
            invuln: 0.0,
            shield: false,
            twin: 0.0,
            rapid: 0.0,
            banner: 0.0,
            shake: 0.0,
            time: 0.0,
            landed: false,
            end_ms: 0.0,
            fire_cd: 0.0,
            dive_cd: 0.0,
            next_life: EXTRA_LIFE_EVERY,
            left: false,
            right: false,
            firing: false,
            touch_x: None,
            rng,
        };
        g.reset();
        g
    }

    pub fn reset(&mut self) {
        self.ship_x = W / 2.0;
        self.lives = START_LIVES;
        self.score = 0;
        self.wave = 0;
        self.shots.clear();
        self.bolts.clear();
        self.drops.clear();
        self.particles.clear();
        self.invuln = 0.0;
        self.shield = false;
        self.twin = 0.0;
        self.rapid = 0.0;
        self.landed = false;
        self.end_ms = 0.0;
        self.next_life = EXTRA_LIFE_EVERY;
        self.state = GameState::Ready;
        self.next_wave();
    }

    fn next_wave(&mut self) {
        self.wave += 1;
        self.descent = 0.0;
        self.banner = BANNER_S;
        self.dive_cd = self.dive_interval() + 1.5;
        self.aliens.clear();
        for row in 0..ROWS {
            for col in 0..COLS {
                let kind = match row {
                    0 => AlienKind::Warden,
                    1 => AlienKind::Stinger,
                    _ => AlienKind::Drone,
                };
                // Fly in from alternating sides, one after another.
                let side = if col < COLS / 2 { -1.0 } else { 1.0 };
                let order = (row * COLS + col) as f64;
                self.aliens.push(Alien {
                    kind,
                    hp: if kind == AlienKind::Warden { 2 } else { 1 },
                    col,
                    row,
                    mode: Mode::Entering,
                    x: W / 2.0 + side * (W / 2.0 + 40.0),
                    y: -40.0,
                    t: -order * ENTER_STAGGER_S,
                    sx: W / 2.0 + side * (W / 2.0 + 40.0),
                    sy: -40.0,
                    aim_x: 0.0,
                    phase: 0.0,
                    fired: false,
                    flash: 0.0,
                });
            }
        }
    }

    // ------------------------------------------------------------ tuning per wave

    fn dive_interval(&self) -> f64 {
        (3.2 - 0.3 * self.wave as f64).max(0.8)
    }

    fn descent_speed(&self) -> f64 {
        2.5 + 1.2 * self.wave as f64
    }

    /// Chance per second that each front-line alien fires.
    fn fire_rate(&self) -> f64 {
        0.05 + 0.025 * self.wave as f64
    }

    fn bolt_speed(&self) -> f64 {
        210.0 + 14.0 * self.wave as f64
    }

    /// The formation sways faster the lower it gets and the fewer are left.
    pub fn sway_speed(&self) -> f64 {
        let total = (COLS * ROWS) as f64;
        let left = self.aliens.len().max(1) as f64;
        0.9 + self.descent / 140.0 + (1.0 - left / total) * 1.4 + self.wave as f64 * 0.08
    }

    pub fn slot(&self, col: usize, row: usize) -> (f64, f64) {
        let cx = W / 2.0 + (self.sway_t).sin() * SWAY;
        (
            cx + (col as f64 - (COLS as f64 - 1.0) / 2.0) * SPACING_X,
            FORM_TOP + self.descent + row as f64 * SPACING_Y,
        )
    }

    // ------------------------------------------------------------ input

    pub fn handle_key(&mut self, key: Key, pressed: bool) -> bool {
        match key {
            Key::Left => self.left = pressed,
            Key::Right => self.right = pressed,
            Key::Action | Key::Up => self.firing = pressed,
            _ => {}
        }
        let used = matches!(key, Key::Left | Key::Right | Key::Up | Key::Action | Key::Pause);
        if !pressed || !used {
            return used;
        }
        match self.state {
            GameState::Ready => {
                if key != Key::Pause {
                    self.state = GameState::Playing;
                }
            }
            GameState::Playing => {
                if key == Key::Pause {
                    self.pause();
                }
            }
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

    pub fn pointer_down(&mut self, x: f64, _y: f64) -> bool {
        if self.state == GameState::Ready {
            self.state = GameState::Playing;
        }
        if self.state != GameState::Playing {
            return false;
        }
        self.touch_x = Some(x);
        self.firing = true;
        true
    }

    pub fn pointer_move(&mut self, x: f64, _y: f64) {
        if self.touch_x.is_some() && x >= 0.0 {
            self.touch_x = Some(x);
        }
    }

    pub fn pointer_up(&mut self, _x: f64, _y: f64) {
        self.touch_x = None;
        self.firing = false;
    }

    pub fn pause(&mut self) {
        if self.state == GameState::Playing && self.end_ms <= 0.0 {
            self.state = GameState::Paused;
        }
        self.left = false;
        self.right = false;
        self.firing = false;
        self.touch_x = None;
    }

    // ------------------------------------------------------------ simulation

    pub fn update(&mut self, dt_ms: f64) {
        let dt = (dt_ms / 1000.0).min(0.05);
        self.time += dt;
        self.update_stars(dt);
        if self.state == GameState::Paused {
            return;
        }
        self.update_particles(dt);
        if self.state != GameState::Playing {
            return;
        }
        self.shake = (self.shake - dt * 30.0).max(0.0);
        if self.end_ms > 0.0 {
            self.end_ms -= dt_ms;
            if self.end_ms <= 0.0 {
                self.state = GameState::Over;
            }
            return;
        }
        self.banner = (self.banner - dt).max(0.0);
        self.invuln = (self.invuln - dt).max(0.0);
        self.twin = (self.twin - dt).max(0.0);
        self.rapid = (self.rapid - dt).max(0.0);

        self.move_ship(dt);
        self.fire(dt);
        self.move_formation(dt);
        self.move_aliens(dt);
        self.check_landing();
        if self.end_ms > 0.0 {
            return;
        }
        self.alien_fire(dt);
        self.move_shots(dt);
        self.collide();

        if self.aliens.is_empty() {
            self.score += 100 * self.wave;
            self.next_wave();
        }
    }

    fn update_stars(&mut self, dt: f64) {
        for s in &mut self.stars {
            s.y += (14.0 + 26.0 * s.layer as f64) * dt;
            if s.y > H {
                s.y -= H;
            }
        }
    }

    fn update_particles(&mut self, dt: f64) {
        for p in &mut self.particles {
            p.x += p.vx * dt;
            p.y += p.vy * dt;
            p.vx *= 1.0 - 2.0 * dt;
            p.vy *= 1.0 - 2.0 * dt;
            p.life -= dt;
        }
        self.particles.retain(|p| p.life > 0.0);
    }

    fn move_ship(&mut self, dt: f64) {
        let mut dir = (self.right as i32 - self.left as i32) as f64;
        if let Some(tx) = self.touch_x {
            let d = tx - self.ship_x;
            dir = if d.abs() < 4.0 { d / 4.0 } else { d.signum() };
        }
        let half = SHIP_W / 2.0 + 6.0;
        self.ship_x = (self.ship_x + dir * SHIP_SPEED * dt).clamp(half, W - half);
    }

    fn fire(&mut self, dt: f64) {
        self.fire_cd -= dt * 1000.0;
        if !self.firing || self.fire_cd > 0.0 {
            return;
        }
        self.fire_cd = if self.rapid > 0.0 { RAPID_FIRE_MS } else { FIRE_MS };
        let y = SHIP_Y - SHIP_H / 2.0;
        if self.twin > 0.0 {
            for dx in [-9.0, 9.0] {
                self.shots.push(Shot { x: self.ship_x + dx, y, vx: 0.0, vy: -SHOT_SPEED });
            }
        } else {
            self.shots.push(Shot { x: self.ship_x, y, vx: 0.0, vy: -SHOT_SPEED });
        }
    }

    fn move_formation(&mut self, dt: f64) {
        self.sway_t += self.sway_speed() * dt;
        // Only creep down once everyone has arrived.
        if self.aliens.iter().all(|a| a.mode != Mode::Entering) {
            self.descent += self.descent_speed() * dt;
        }
    }

    fn check_landing(&mut self) {
        let lowest = self
            .aliens
            .iter()
            .filter(|a| a.mode == Mode::Formation)
            .map(|a| a.y)
            .fold(f64::MIN, f64::max);
        if lowest + ALIEN_R >= LAND_Y {
            self.landed = true;
            self.lives = 0;
            self.explode(self.ship_x, SHIP_Y, 0, 40);
            self.end_ms = DEATH_PAUSE_MS;
        }
    }

    fn move_aliens(&mut self, dt: f64) {
        // Send a diver now and then, from the front of the formation.
        self.dive_cd -= dt;
        if self.dive_cd <= 0.0 {
            self.dive_cd = self.dive_interval() * self.rng.range(0.7, 1.3);
            self.start_dive();
        }
        let wave = self.wave as f64;
        for i in 0..self.aliens.len() {
            let (slot_x, slot_y) = self.slot(self.aliens[i].col, self.aliens[i].row);
            let a = &mut self.aliens[i];
            a.t += dt;
            a.flash = (a.flash - dt).max(0.0);
            match a.mode {
                Mode::Entering => {
                    if a.t < 0.0 {
                        continue;
                    }
                    let p = ease(a.t / ENTER_S);
                    let side = if a.sx < W / 2.0 { 1.0 } else { -1.0 };
                    a.x = a.sx + (slot_x - a.sx) * p + (p * PI).sin() * 110.0 * side;
                    a.y = a.sy + (slot_y - a.sy) * p + (p * PI).sin() * 30.0;
                    if a.t >= ENTER_S {
                        a.mode = Mode::Formation;
                    }
                }
                Mode::Formation => {
                    a.x = slot_x;
                    a.y = slot_y;
                }
                Mode::Diving => {
                    let speed = 150.0 + 22.0 * wave;
                    let steer = (a.t / 1.1).min(1.0);
                    a.x = a.sx + (a.aim_x - a.sx) * ease(steer) + (a.t * 3.2 + a.phase).sin() * 46.0;
                    a.y = a.sy + speed * a.t + 60.0 * a.t * a.t;
                    if a.y > H + 30.0 {
                        a.mode = Mode::Returning;
                        a.t = 0.0;
                        a.sx = slot_x;
                        a.sy = -40.0;
                    }
                }
                Mode::Returning => {
                    let p = ease(a.t / RETURN_S);
                    a.x = a.sx + (slot_x - a.sx) * p;
                    a.y = a.sy + (slot_y - a.sy) * p;
                    if a.t >= RETURN_S {
                        a.mode = Mode::Formation;
                    }
                }
            }
        }
        // Divers fire once on the way down.
        let speed = self.bolt_speed();
        for i in 0..self.aliens.len() {
            let a = self.aliens[i];
            if a.mode == Mode::Diving && !a.fired && a.t > 0.45 && a.y < SHIP_Y - 120.0 {
                self.aliens[i].fired = true;
                self.aim_bolt(a.x, a.y + ALIEN_R, speed);
            }
        }
    }

    fn start_dive(&mut self) {
        // Stingers prefer to dive; anyone at the front of a column may.
        let front: Vec<usize> = (0..self.aliens.len())
            .filter(|&i| {
                let a = &self.aliens[i];
                a.mode == Mode::Formation
                    && !self.aliens.iter().any(|b| b.col == a.col && b.row > a.row && b.mode == Mode::Formation)
            })
            .collect();
        let stingers: Vec<usize> = (0..self.aliens.len())
            .filter(|&i| self.aliens[i].mode == Mode::Formation && self.aliens[i].kind == AlienKind::Stinger)
            .collect();
        let pool = if !stingers.is_empty() && self.rng.unit() < 0.6 { stingers } else { front };
        if pool.is_empty() {
            return;
        }
        let i = pool[self.rng.below(pool.len())];
        let phase = self.rng.range(0.0, PI * 2.0);
        let aim = (self.ship_x + self.rng.range(-40.0, 40.0)).clamp(20.0, W - 20.0);
        let a = &mut self.aliens[i];
        a.mode = Mode::Diving;
        a.t = 0.0;
        a.sx = a.x;
        a.sy = a.y;
        a.aim_x = aim;
        a.phase = phase;
        a.fired = false;
    }

    fn aim_bolt(&mut self, x: f64, y: f64, speed: f64) {
        let (dx, dy) = (self.ship_x - x, SHIP_Y - y);
        let len = dx.hypot(dy).max(1.0);
        // Mostly downward, leaning toward the ship.
        let vx = (dx / len * speed).clamp(-speed * 0.35, speed * 0.35);
        self.bolts.push(Shot { x, y, vx, vy: speed });
    }

    fn alien_fire(&mut self, dt: f64) {
        let rate = self.fire_rate();
        let speed = self.bolt_speed();
        for col in 0..COLS {
            let front = self
                .aliens
                .iter()
                .filter(|a| a.col == col && a.mode == Mode::Formation)
                .max_by(|a, b| a.row.cmp(&b.row))
                .map(|a| (a.x, a.y));
            if let Some((x, y)) = front {
                if self.rng.unit() < rate * dt {
                    self.bolts.push(Shot { x, y: y + ALIEN_R, vx: 0.0, vy: speed });
                }
            }
        }
    }

    fn move_shots(&mut self, dt: f64) {
        for s in self.shots.iter_mut().chain(self.bolts.iter_mut()) {
            s.x += s.vx * dt;
            s.y += s.vy * dt;
        }
        self.shots.retain(|s| s.y > -20.0);
        self.bolts.retain(|s| s.y < H + 20.0 && s.x > -20.0 && s.x < W + 20.0);
        for d in &mut self.drops {
            d.y += POWER_SPEED * dt;
        }
        self.drops.retain(|d| d.y < H + 20.0);
    }

    fn ship_hit(&self, x: f64, y: f64, r: f64) -> bool {
        (x - self.ship_x).abs() < SHIP_W / 2.0 + r && (y - SHIP_Y).abs() < SHIP_H / 2.0 + r
    }

    fn collide(&mut self) {
        // Player shots against aliens.
        let mut i = 0;
        while i < self.shots.len() {
            let s = self.shots[i];
            let target = self.aliens.iter().position(|a| {
                (a.mode != Mode::Entering || a.t >= 0.0)
                    && (a.x - s.x).abs() < ALIEN_R + 2.0
                    && (a.y - s.y).abs() < ALIEN_R + 6.0
            });
            if let Some(k) = target {
                self.shots.swap_remove(i);
                self.hit_alien(k);
            } else {
                i += 1;
            }
        }

        // Alien bolts and divers against the ship.
        if self.invuln > 0.0 {
            return;
        }
        if let Some(b) = self.bolts.iter().position(|b| self.ship_hit(b.x, b.y, 3.0)) {
            self.bolts.swap_remove(b);
            self.hurt();
            return;
        }
        if let Some(k) = self.aliens.iter().position(|a| a.mode == Mode::Diving && self.ship_hit(a.x, a.y, ALIEN_R - 4.0)) {
            let a = self.aliens.swap_remove(k);
            self.add_score(a.kind.points());
            self.explode(a.x, a.y, 1, 20);
            self.hurt();
        }

        // Power-ups.
        while let Some(d) = self.drops.iter().position(|d| self.ship_hit(d.x, d.y, 10.0)) {
            let d = self.drops.swap_remove(d);
            match d.kind {
                Power::Twin => self.twin = TWIN_S,
                Power::Rapid => self.rapid = RAPID_S,
                Power::Shield => self.shield = true,
            }
        }
    }

    fn hit_alien(&mut self, k: usize) {
        let a = &mut self.aliens[k];
        a.hp -= 1;
        a.flash = 0.12;
        if a.hp > 0 {
            return;
        }
        let a = self.aliens.swap_remove(k);
        // Shooting a diver out of the air is worth double.
        let points = a.kind.points() * if a.mode == Mode::Diving { 2 } else { 1 };
        self.add_score(points);
        let hue = match a.kind {
            AlienKind::Drone => 1,
            AlienKind::Stinger => 2,
            AlienKind::Warden => 3,
        };
        self.explode(a.x, a.y, hue, 18);
        if self.rng.unit() < POWER_CHANCE {
            let kind = [Power::Twin, Power::Rapid, Power::Shield][self.rng.below(3)];
            self.drops.push(Drop { kind, x: a.x, y: a.y });
        }
    }

    fn add_score(&mut self, points: u32) {
        self.score += points;
        while self.score >= self.next_life {
            self.lives += 1;
            self.next_life += EXTRA_LIFE_EVERY;
        }
    }

    fn hurt(&mut self) {
        self.shake = 10.0;
        if self.shield {
            self.shield = false;
            self.invuln = 0.6;
            return;
        }
        self.explode(self.ship_x, SHIP_Y, 0, 36);
        self.lives = self.lives.saturating_sub(1);
        self.twin = 0.0;
        self.rapid = 0.0;
        self.bolts.clear();
        if self.lives == 0 {
            self.end_ms = DEATH_PAUSE_MS;
        } else {
            self.invuln = INVULN_S;
        }
    }

    fn explode(&mut self, x: f64, y: f64, hue: u8, count: usize) {
        for _ in 0..count {
            let a = self.rng.range(0.0, PI * 2.0);
            let v = self.rng.range(40.0, 220.0);
            let life = self.rng.range(0.35, 0.8);
            self.particles.push(Particle { x, y, vx: a.cos() * v, vy: a.sin() * v, life, max: life, hue });
        }
    }

    pub fn ship_visible(&self) -> bool {
        self.end_ms <= 0.0 && (self.invuln <= 0.0 || (self.time * 12.0).fract() < 0.6)
    }

    pub fn end_message(&self) -> Option<String> {
        Some(if self.landed {
            format!("They landed! · reached wave {}", self.wave)
        } else {
            format!("Ship lost · reached wave {}", self.wave)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn playing() -> StarDefender {
        let mut g = StarDefender::new(5);
        g.state = GameState::Playing;
        g
    }

    /// Fast-forward until every alien has flown into formation.
    fn settled() -> StarDefender {
        let mut g = playing();
        g.dive_cd = 1e9;
        while g.aliens.iter().any(|a| a.mode != Mode::Formation) {
            g.update(16.0);
        }
        g.bolts.clear();
        g
    }

    #[test]
    fn first_wave_flies_into_formation() {
        let g = settled();
        assert_eq!(g.aliens.len(), COLS * ROWS);
        for a in &g.aliens {
            let (x, y) = g.slot(a.col, a.row);
            assert!((a.x - x).abs() < 1e-6 && (a.y - y).abs() < 1e-6);
        }
    }

    #[test]
    fn a_shot_destroys_a_drone_and_scores() {
        let mut g = settled();
        let target = *g.aliens.iter().find(|a| a.kind == AlienKind::Drone).unwrap();
        g.shots.push(Shot { x: target.x, y: target.y + 5.0, vx: 0.0, vy: -SHOT_SPEED });
        g.collide();
        assert_eq!(g.aliens.len(), COLS * ROWS - 1);
        assert_eq!(g.score, AlienKind::Drone.points());
        assert!(!g.particles.is_empty());
    }

    #[test]
    fn wardens_take_two_hits() {
        let mut g = settled();
        let w = *g.aliens.iter().find(|a| a.kind == AlienKind::Warden).unwrap();
        for _ in 0..2 {
            g.shots.push(Shot { x: w.x, y: w.y, vx: 0.0, vy: 0.0 });
            g.collide();
        }
        assert_eq!(g.aliens.len(), COLS * ROWS - 1);
        assert_eq!(g.score, AlienKind::Warden.points());
    }

    #[test]
    fn holding_fire_shoots_at_the_fire_rate() {
        let mut g = settled();
        g.handle_key(Key::Action, true);
        for _ in 0..60 {
            g.fire(1.0 / 60.0);
        }
        assert_eq!(g.shots.len(), 4, "one second at 280 ms per shot");
        g.handle_key(Key::Action, false);
        g.shots.clear();
        g.fire(1.0);
        assert!(g.shots.is_empty());
    }

    #[test]
    fn twin_power_fires_two_shots() {
        let mut g = settled();
        g.twin = 5.0;
        g.firing = true;
        g.fire(0.016);
        assert_eq!(g.shots.len(), 2);
    }

    #[test]
    fn a_bolt_costs_a_life_then_brief_invulnerability() {
        let mut g = settled();
        g.bolts.push(Shot { x: g.ship_x, y: SHIP_Y, vx: 0.0, vy: 0.0 });
        g.collide();
        assert_eq!(g.lives, START_LIVES - 1);
        assert!(g.invuln > 0.0);
        g.bolts.push(Shot { x: g.ship_x, y: SHIP_Y, vx: 0.0, vy: 0.0 });
        g.collide();
        assert_eq!(g.lives, START_LIVES - 1, "invulnerable right after a hit");
    }

    #[test]
    fn shield_absorbs_one_hit() {
        let mut g = settled();
        g.drops.push(Drop { kind: Power::Shield, x: g.ship_x, y: SHIP_Y });
        g.collide();
        assert!(g.shield);
        g.bolts.push(Shot { x: g.ship_x, y: SHIP_Y, vx: 0.0, vy: 0.0 });
        g.collide();
        assert!(!g.shield);
        assert_eq!(g.lives, START_LIVES);
    }

    #[test]
    fn losing_the_last_life_ends_the_game_after_a_pause() {
        let mut g = settled();
        g.lives = 1;
        g.bolts.push(Shot { x: g.ship_x, y: SHIP_Y, vx: 0.0, vy: 0.0 });
        g.collide();
        assert_eq!(g.state, GameState::Playing);
        g.update(DEATH_PAUSE_MS + 20.0);
        assert_eq!(g.state, GameState::Over);
        assert!(g.end_message().unwrap().starts_with("Ship lost"));
    }

    #[test]
    fn clearing_a_wave_starts_a_harder_one() {
        let mut g = settled();
        let slow = g.dive_interval();
        g.aliens.clear();
        g.update(16.0);
        assert_eq!(g.wave, 2);
        assert_eq!(g.aliens.len(), COLS * ROWS);
        assert!(g.banner > 0.0);
        assert!(g.dive_interval() < slow);
        assert_eq!(g.score, 100);
    }

    #[test]
    fn formation_speeds_up_as_it_descends_and_thins_out() {
        let mut g = settled();
        let base = g.sway_speed();
        g.descent = 120.0;
        assert!(g.sway_speed() > base);
        let lower = g.sway_speed();
        g.aliens.truncate(4);
        assert!(g.sway_speed() > lower);
    }

    #[test]
    fn reaching_the_ship_line_is_game_over() {
        let mut g = settled();
        g.descent = LAND_Y;
        g.update(16.0);
        assert!(g.landed);
        g.update(DEATH_PAUSE_MS + 20.0);
        assert_eq!(g.state, GameState::Over);
        assert!(g.end_message().unwrap().starts_with("They landed"));
    }

    #[test]
    fn divers_swoop_down_and_return_to_formation() {
        let mut g = settled();
        g.invuln = 1e9; // don't let the diver hit the ship
        g.start_dive();
        let k = g.aliens.iter().position(|a| a.mode == Mode::Diving).unwrap();
        let (col, row) = (g.aliens[k].col, g.aliens[k].row);
        let mut went_low = false;
        for _ in 0..600 {
            g.dive_cd = 1e9;
            g.update(16.0);
            let a = g.aliens.iter().find(|a| a.col == col && a.row == row).unwrap();
            went_low |= a.y > SHIP_Y;
            if went_low && a.mode == Mode::Formation {
                return;
            }
        }
        panic!("diver never came back");
    }

    #[test]
    fn extra_life_every_5000() {
        let mut g = settled();
        g.score = 4990;
        g.add_score(20);
        assert_eq!(g.lives, START_LIVES + 1);
    }

    #[test]
    fn touch_steers_and_fires() {
        let mut g = settled();
        g.pointer_down(60.0, 500.0);
        for _ in 0..120 {
            g.move_ship(1.0 / 60.0);
        }
        assert!((g.ship_x - 60.0).abs() < 5.0);
        assert!(g.firing);
        g.pointer_up(60.0, 500.0);
        assert!(!g.firing);
    }

    #[test]
    fn space_starts_and_restarts() {
        let mut g = StarDefender::new(1);
        g.handle_key(Key::Action, true);
        assert_eq!(g.state, GameState::Playing);
        g.state = GameState::Over;
        g.score = 999;
        g.handle_key(Key::Action, true);
        assert_eq!(g.state, GameState::Playing);
        assert_eq!((g.score, g.wave, g.lives), (0, 1, START_LIVES));
    }
}
