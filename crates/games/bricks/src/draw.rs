use crate::logic::{Bricks, BALL_R, H, PADDLE_H, PADDLE_W, PADDLE_Y, W};
use engine::render::snap;
use engine::{Canvas, GameState};

const LCD_BG: &str = "#c3cf9b";
const LCD_GHOST: &str = "#b7c48f";
const LCD_INK: &str = "#2c3a23";
/// Retro "pixel" size; everything snaps to this grid.
const PX: f64 = 4.0;

pub fn retro(g: &Bricks, c: &Canvas) {
    c.color(LCD_BG);
    c.rect(0.0, 0.0, W, H);

    for k in g.bricks.iter().filter(|k| k.alive) {
        let (x, y, w, h) = k.rect();
        // Snap both edges (not the width) so the gaps between bricks stay even.
        let (x0, y0) = (snap(x, PX), snap(y, PX));
        let (x, y, w, h) = (x0, y0, snap(x + w, PX) - x0, snap(y + h, PX) - y0);
        c.color(LCD_INK);
        c.rect(x, y, w, h);
        // Alternate rows are hollow so a monochrome wall still has texture.
        if k.row % 2 == 1 {
            c.color(LCD_GHOST);
            c.rect(x + PX, y + PX, w - 2.0 * PX, h - 2.0 * PX);
        }
    }

    c.color(LCD_INK);
    for p in &g.particles {
        c.rect(snap(p.x, PX), snap(p.y, PX), PX, PX);
    }

    let px = snap(g.paddle_x - PADDLE_W / 2.0, PX);
    c.rect(px, snap(PADDLE_Y, PX), PADDLE_W, PADDLE_H);

    let blink = g.state == GameState::Over && (g.time * 3.0).fract() < 0.5;
    if !blink {
        let s = BALL_R * 2.0;
        c.rect(snap(g.ball.x - BALL_R, PX), snap(g.ball.y - BALL_R, PX), s, s);
    }

    c.text(&format!("LV {}", g.level), W - 12.0, 36.0, "10px 'Press Start 2P', monospace", "right");
    if level_banner(g) {
        let msg = format!("LEVEL {}", g.level);
        c.text(&msg, W / 2.0, H / 2.0, "22px 'Press Start 2P', monospace", "center");
        c.text("PRESS SPACE", W / 2.0, H / 2.0 + 36.0, "10px 'Press Start 2P', monospace", "center");
    }
}

/// Show "Level N" while the ball waits on the paddle at the start of a level.
fn level_banner(g: &Bricks) -> bool {
    g.state == GameState::Playing && g.stuck
}

const ROW_COLORS: [&str; 7] = [
    "#ff5d8f", "#ff8a5b", "#ffc857", "#9be564", "#4dd4ac", "#4cc9f0", "#8f7cff",
];

pub fn modern(g: &Bricks, c: &Canvas) {
    let bg = c.linear_gradient(0.0, 0.0, 0.0, H, &[(0.0, "#141838"), (1.0, "#090c1c")]);
    c.gradient(&bg);
    c.rect(0.0, 0.0, W, H);

    for k in g.bricks.iter().filter(|k| k.alive) {
        let (x, y, w, h) = k.rect();
        c.color(ROW_COLORS[k.row % ROW_COLORS.len()]);
        c.round_rect(x, y, w, h, 4.0);
        // Top highlight for a bit of depth.
        c.alpha(0.25);
        c.color("#ffffff");
        c.round_rect(x + 2.0, y + 2.0, w - 4.0, 4.0, 2.0);
        c.alpha(1.0);
    }

    for p in &g.particles {
        c.alpha(p.life.clamp(0.0, 1.0));
        c.color(ROW_COLORS[p.row % ROW_COLORS.len()]);
        c.circle(p.x, p.y, 2.5);
    }
    c.alpha(1.0);

    let px = g.paddle_x - PADDLE_W / 2.0;
    let grad = c.linear_gradient(px, 0.0, px + PADDLE_W, 0.0, &[(0.0, "#7cf2ff"), (1.0, "#8f7cff")]);
    c.glow(16.0, "#7cc8ff");
    c.gradient(&grad);
    c.round_rect(px, PADDLE_Y, PADDLE_W, PADDLE_H, 6.0);
    c.no_glow();

    // Ball trail, fading out.
    c.color("#bde7ff");
    let n = g.trail.len() as f64;
    for (i, &(x, y)) in g.trail.iter().enumerate().skip(1) {
        let t = 1.0 - i as f64 / n;
        c.alpha(0.35 * t);
        c.circle(x, y, BALL_R * (0.5 + 0.5 * t));
    }
    c.alpha(1.0);

    let ball_col = if g.state == GameState::Over { "#ff5d8f" } else { "#ffffff" };
    c.glow(18.0, "#bde7ff");
    c.color(ball_col);
    c.circle(g.ball.x, g.ball.y, BALL_R);
    c.no_glow();

    c.color("#9aa3c7");
    c.text(&format!("Level {}", g.level), W - 14.0, 36.0, "500 14px 'Space Grotesk', sans-serif", "right");
    if level_banner(g) {
        let pulse = 0.75 + 0.25 * (g.time * 4.0).sin();
        c.alpha(pulse);
        c.glow(20.0, "#7cf2ff");
        c.color("#ffffff");
        c.text(&format!("Level {}", g.level), W / 2.0, H / 2.0, "700 40px 'Space Grotesk', sans-serif", "center");
        c.no_glow();
        c.alpha(1.0);
        c.color("#9aa3c7");
        c.text("Press Space to launch", W / 2.0, H / 2.0 + 40.0, "16px 'Space Grotesk', sans-serif", "center");
    }
}
