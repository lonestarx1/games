//! Brick breaker rules: one ball, endless levels. Each cleared wall starts a
//! new level with a faster ball.

use engine::{GameState, Key, Rng};
use std::collections::VecDeque;

pub const W: f64 = 480.0;
pub const H: f64 = 640.0;

pub const COLS: usize = 10;
pub const ROWS: usize = 7;
const MARGIN: f64 = 12.0;
const GAP: f64 = 4.0;
/// Space above the wall so the ball can get behind it.
const WALL_TOP: f64 = 72.0;
pub const BRICK_W: f64 = (W - 2.0 * MARGIN - (COLS as f64 - 1.0) * GAP) / COLS as f64;
pub const BRICK_H: f64 = 18.0;

pub const PADDLE_W: f64 = 88.0;
pub const PADDLE_H: f64 = 12.0;
pub const PADDLE_Y: f64 = H - 48.0;
pub const BALL_R: f64 = 6.0;

/// Ball speed in px/s. Level 1 is slow; every level adds LEVEL_STEP, up to MAX.
/// Speed stays the same for the whole level.
const BASE_SPEED: f64 = 260.0;
const LEVEL_STEP: f64 = 70.0;
const MAX_SPEED: f64 = 1150.0;
/// Hitting the paddle's edge sends the ball out at up to 60 degrees.
const MAX_BOUNCE: f64 = 60.0 * std::f64::consts::PI / 180.0;
const TRAIL_LEN: usize = 10;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Brick {
    pub col: usize,
    pub row: usize,
    pub alive: bool,
}

impl Brick {
    pub fn rect(&self) -> (f64, f64, f64, f64) {
        (
            MARGIN + self.col as f64 * (BRICK_W + GAP),
            WALL_TOP + self.row as f64 * (BRICK_H + GAP),
            BRICK_W,
            BRICK_H,
        )
    }

    /// Higher rows are harder to reach, so they're worth more.
    pub fn points(&self) -> u32 {
        (ROWS - self.row) as u32 * 10
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Ball {
    pub x: f64,
    pub y: f64,
    /// Unit direction; speed is separate.
    pub dx: f64,
    pub dy: f64,
}

pub struct Particle {
    pub x: f64,
    pub y: f64,
    pub vx: f64,
    pub vy: f64,
    pub life: f64,
    pub row: usize,
}

pub struct Bricks {
    pub paddle_x: f64,
    pub ball: Ball,
    /// Ball resting on the paddle, waiting for launch.
    pub stuck: bool,
    pub bricks: Vec<Brick>,
    pub score: u32,
    pub state: GameState,
    pub level: u32,
    pub particles: Vec<Particle>,
    pub trail: VecDeque<(f64, f64)>,
    pub time: f64,
    left: bool,
    right: bool,
    rng: Rng,
}

impl Bricks {
    pub fn new(seed: u64) -> Bricks {
        let mut b = Bricks {
            paddle_x: W / 2.0,
            ball: Ball { x: 0.0, y: 0.0, dx: 0.0, dy: -1.0 },
            stuck: true,
            bricks: Vec::new(),
            score: 0,
            state: GameState::Ready,
            level: 1,
            particles: Vec::new(),
            trail: VecDeque::new(),
            time: 0.0,
            left: false,
            right: false,
            rng: Rng::new(seed),
        };
        b.reset();
        b
    }

    pub fn reset(&mut self) {
        self.paddle_x = W / 2.0;
        self.stuck = true;
        self.score = 0;
        self.level = 1;
        self.state = GameState::Ready;
        self.particles.clear();
        self.build_wall();
        self.rest_ball();
    }

    fn build_wall(&mut self) {
        self.bricks = (0..ROWS)
            .flat_map(|row| (0..COLS).map(move |col| Brick { col, row, alive: true }))
            .collect();
    }

    fn rest_ball(&mut self) {
        self.ball.x = self.paddle_x;
        self.ball.y = PADDLE_Y - BALL_R - 1.0;
        self.trail.clear();
    }

    pub fn speed(&self) -> f64 {
        (BASE_SPEED + LEVEL_STEP * (self.level - 1) as f64).min(MAX_SPEED)
    }

    fn paddle_speed(&self) -> f64 {
        560.0 + 0.4 * self.speed()
    }

    fn launch(&mut self) {
        let a = self.rng.range(-0.35, 0.35);
        self.ball.dx = a.sin();
        self.ball.dy = -a.cos();
        self.stuck = false;
    }

    pub fn handle_key(&mut self, key: Key, pressed: bool) -> bool {
        match key {
            Key::Left => self.left = pressed,
            Key::Right => self.right = pressed,
            _ => {}
        }
        let used = matches!(key, Key::Left | Key::Right | Key::Up | Key::Action | Key::Pause);
        if !pressed || !used {
            return used;
        }
        let launch_key = matches!(key, Key::Action | Key::Up);
        match self.state {
            GameState::Ready if launch_key => {
                self.state = GameState::Playing;
                self.launch();
            }
            GameState::Playing if self.stuck && launch_key => self.launch(),
            GameState::Playing if matches!(key, Key::Action | Key::Pause) => {
                self.state = GameState::Paused;
            }
            GameState::Paused if matches!(key, Key::Action | Key::Pause) => {
                self.state = GameState::Playing;
            }
            GameState::Over if key == Key::Action => {
                self.reset();
                self.state = GameState::Playing;
                self.launch();
            }
            _ => {}
        }
        true
    }

    pub fn update(&mut self, dt_ms: f64) {
        let dt = (dt_ms / 1000.0).min(0.05);
        self.time += dt;
        if self.state != GameState::Paused {
            self.update_particles(dt);
        }
        match self.state {
            GameState::Ready => {
                self.move_paddle(dt);
                self.rest_ball();
            }
            GameState::Playing if self.stuck => {
                self.move_paddle(dt);
                self.rest_ball();
            }
            GameState::Playing => {
                let speed = self.speed();
                // Sub-steps keep the ball from tunnelling through bricks at high speed.
                let steps = ((speed * dt) / (BALL_R * 0.5)).ceil().max(1.0) as usize;
                let h = dt / steps as f64;
                for _ in 0..steps {
                    self.step(h, speed);
                    if self.state != GameState::Playing || self.stuck {
                        break;
                    }
                }
                self.trail.push_front((self.ball.x, self.ball.y));
                self.trail.truncate(TRAIL_LEN);
            }
            _ => {}
        }
    }

    fn move_paddle(&mut self, dt: f64) {
        let dir = self.right as i32 - self.left as i32;
        let half = PADDLE_W / 2.0;
        self.paddle_x = (self.paddle_x + dir as f64 * self.paddle_speed() * dt).clamp(half, W - half);
    }

    fn step(&mut self, h: f64, speed: f64) {
        self.move_paddle(h);
        let b = &mut self.ball;
        b.x += b.dx * speed * h;
        b.y += b.dy * speed * h;

        // Side and top walls.
        if b.x < BALL_R {
            b.x = BALL_R;
            b.dx = b.dx.abs();
        } else if b.x > W - BALL_R {
            b.x = W - BALL_R;
            b.dx = -b.dx.abs();
        }
        if b.y < BALL_R {
            b.y = BALL_R;
            b.dy = b.dy.abs();
        }

        // Paddle: the bounce angle depends on where the ball lands.
        let reach = PADDLE_W / 2.0 + BALL_R;
        if b.dy > 0.0
            && b.y + BALL_R >= PADDLE_Y
            && b.y < PADDLE_Y + PADDLE_H
            && (b.x - self.paddle_x).abs() <= reach
        {
            let offset = ((b.x - self.paddle_x) / reach).clamp(-1.0, 1.0);
            let angle = offset * MAX_BOUNCE;
            b.dx = angle.sin();
            b.dy = -angle.cos();
            b.y = PADDLE_Y - BALL_R;
        }

        if b.y - BALL_R > H {
            self.state = GameState::Over;
            return;
        }

        self.hit_brick();
    }

    fn hit_brick(&mut self) {
        let b = &mut self.ball;
        let Some(brick) = self.bricks.iter_mut().find(|k| {
            if !k.alive {
                return false;
            }
            let (x, y, w, h) = k.rect();
            let cx = b.x.clamp(x, x + w);
            let cy = b.y.clamp(y, y + h);
            (b.x - cx).powi(2) + (b.y - cy).powi(2) < BALL_R * BALL_R
        }) else {
            return;
        };

        // Bounce off whichever side the ball overlaps least.
        let (x, y, w, h) = brick.rect();
        let over_x = (b.x + BALL_R - x).min(x + w - (b.x - BALL_R));
        let over_y = (b.y + BALL_R - y).min(y + h - (b.y - BALL_R));
        if over_x < over_y {
            if b.x < x + w / 2.0 {
                b.dx = -b.dx.abs();
                b.x = x - BALL_R;
            } else {
                b.dx = b.dx.abs();
                b.x = x + w + BALL_R;
            }
        } else if b.y < y + h / 2.0 {
            b.dy = -b.dy.abs();
            b.y = y - BALL_R;
        } else {
            b.dy = b.dy.abs();
            b.y = y + h + BALL_R;
        }

        brick.alive = false;
        self.score += brick.points();
        let (row, cx, cy) = (brick.row, x + w / 2.0, y + h / 2.0);
        self.burst(cx, cy, row);

        if self.bricks.iter().all(|k| !k.alive) {
            // Next level: a fresh wall, ball back on the paddle, faster ball.
            self.level += 1;
            self.build_wall();
            self.stuck = true;
            self.rest_ball();
        }
    }

    fn burst(&mut self, x: f64, y: f64, row: usize) {
        for _ in 0..10 {
            let a = self.rng.range(0.0, std::f64::consts::TAU);
            let v = self.rng.range(60.0, 220.0);
            self.particles.push(Particle { x, y, vx: a.cos() * v, vy: a.sin() * v, life: 1.0, row });
        }
    }

    fn update_particles(&mut self, dt: f64) {
        for p in &mut self.particles {
            p.x += p.vx * dt;
            p.y += p.vy * dt;
            p.vy += 500.0 * dt;
            p.life -= dt * 1.8;
        }
        self.particles.retain(|p| p.life > 0.0);
    }

    pub fn pause(&mut self) {
        self.left = false;
        self.right = false;
        if self.state == GameState::Playing {
            self.state = GameState::Paused;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn in_play(x: f64, y: f64, dx: f64, dy: f64) -> Bricks {
        let mut g = Bricks::new(7);
        g.state = GameState::Playing;
        g.stuck = false;
        let len = (dx * dx + dy * dy).sqrt();
        g.ball = Ball { x, y, dx: dx / len, dy: dy / len };
        g
    }

    #[test]
    fn launches_upward() {
        let mut g = Bricks::new(1);
        g.handle_key(Key::Action, true);
        assert_eq!(g.state, GameState::Playing);
        assert!(!g.stuck);
        assert!(g.ball.dy < 0.0);
    }

    #[test]
    fn bounces_off_side_wall() {
        let mut g = in_play(BALL_R + 1.0, 400.0, -1.0, -0.2);
        g.update(16.0);
        assert!(g.ball.dx > 0.0);
    }

    #[test]
    fn paddle_center_sends_ball_straight_up_and_edge_angles_it() {
        let mut g = in_play(W / 2.0, PADDLE_Y - BALL_R - 1.0, 0.0, 1.0);
        g.update(16.0);
        assert!(g.ball.dy < 0.0);
        assert!(g.ball.dx.abs() < 1e-6);

        let mut g = in_play(W / 2.0 + 40.0, PADDLE_Y - BALL_R - 1.0, 0.0, 1.0);
        g.update(16.0);
        assert!(g.ball.dy < 0.0 && g.ball.dx > 0.3);
    }

    #[test]
    fn missing_the_ball_ends_the_game() {
        let mut g = in_play(20.0, H - 10.0, 0.0, 1.0);
        g.paddle_x = W - PADDLE_W;
        for _ in 0..10 {
            g.update(16.0);
        }
        assert_eq!(g.state, GameState::Over);
    }

    #[test]
    fn hitting_a_brick_scores_and_bounces() {
        let (x, y, w, h) = g_bottom_brick_rect();
        let mut g = in_play(x + w / 2.0, y + h + BALL_R + 2.0, 0.0, -1.0);
        g.update(16.0);
        assert!(g.score > 0);
        assert!(g.ball.dy > 0.0);
        assert_eq!(g.bricks.iter().filter(|k| !k.alive).count(), 1);
    }

    fn g_bottom_brick_rect() -> (f64, f64, f64, f64) {
        Brick { col: 4, row: ROWS - 1, alive: true }.rect()
    }

    #[test]
    fn clearing_the_wall_starts_a_new_one() {
        let (x, y, w, h) = g_bottom_brick_rect();
        let mut g = in_play(x + w / 2.0, y + h + BALL_R + 2.0, 0.0, -1.0);
        for k in &mut g.bricks {
            k.alive = k.col == 4 && k.row == ROWS - 1;
        }
        g.update(16.0);
        assert_eq!(g.level, 2);
        assert!(g.stuck);
        assert!(g.bricks.iter().all(|k| k.alive));
    }

    #[test]
    fn speed_is_fixed_within_a_level_and_rises_per_level() {
        let mut g = in_play(W / 2.0, 400.0, 0.3, -1.0);
        let s1 = g.speed();
        for _ in 0..120 {
            g.update(16.0);
        }
        assert_eq!(g.speed(), s1);
        g.level = 2;
        assert!(g.speed() > s1);
        g.level = 1_000;
        assert_eq!(g.speed(), MAX_SPEED);
    }

    #[test]
    fn fast_ball_does_not_tunnel_through_bricks() {
        let (x, y, w, h) = g_bottom_brick_rect();
        let mut g = in_play(x + w / 2.0, y + h + 30.0, 0.0, -1.0);
        g.level = 1_000; // max speed
        g.update(50.0);
        assert!(g.score > 0);
    }
}
