use crate::logic::{Alien, AlienKind, Power, StarDefender, BANNER_S, H, SHIP_Y, W};
use engine::Canvas;
use std::f64::consts::{PI, TAU};

fn shake(g: &StarDefender) -> (f64, f64) {
    if g.shake <= 0.0 {
        return (0.0, 0.0);
    }
    ((g.time * 71.0).sin() * g.shake * 0.6, (g.time * 53.0).cos() * g.shake * 0.6)
}

fn power_letter(p: Power) -> &'static str {
    match p {
        Power::Twin => "T",
        Power::Rapid => "R",
        Power::Shield => "S",
    }
}

/// Labels for active power-ups, e.g. "TWIN 7".
fn power_labels(g: &StarDefender) -> Vec<String> {
    let mut v = Vec::new();
    if g.twin > 0.0 {
        v.push(format!("TWIN {}", g.twin.ceil()));
    }
    if g.rapid > 0.0 {
        v.push(format!("RAPID {}", g.rapid.ceil()));
    }
    if g.shield {
        v.push("SHIELD".to_string());
    }
    v
}

// ---------------------------------------------------------------- modern

const SHIP: &str = "#7cf2ff";
const DRONE: &str = "#5ef0a0";
const STINGER: &str = "#ff5dbb";
const WARDEN: &str = "#ffb547";
const BOLT: &str = "#ff4f7b";

fn hue(h: u8) -> &'static str {
    match h {
        0 => SHIP,
        1 => DRONE,
        2 => STINGER,
        _ => WARDEN,
    }
}

fn ship_shape(c: &Canvas, x: f64, y: f64, color: &str, t: f64) {
    // Engine flame.
    let flame = 7.0 + (t * 40.0).sin().abs() * 6.0;
    c.color("#ffb347");
    c.polygon(&[(x - 5.0, y + 8.0), (x + 5.0, y + 8.0), (x, y + 8.0 + flame)]);
    c.glow(16.0, color);
    c.color(color);
    c.polygon(&[
        (x, y - 15.0),
        (x + 7.0, y - 2.0),
        (x + 18.0, y + 8.0),
        (x + 18.0, y + 11.0),
        (x + 6.0, y + 7.0),
        (x, y + 10.0),
        (x - 6.0, y + 7.0),
        (x - 18.0, y + 11.0),
        (x - 18.0, y + 8.0),
        (x - 7.0, y - 2.0),
    ]);
    c.no_glow();
    c.color("#0b1020");
    c.ellipse(x, y - 2.0, 3.0, 5.0, 0.0);
}

fn alien_modern(c: &Canvas, a: &Alien, ship_x: f64, t: f64) {
    let (x, y) = (a.x, a.y);
    let white = a.flash > 0.0;
    match a.kind {
        AlienKind::Drone => {
            let col = if white { "#ffffff" } else { DRONE };
            c.glow(12.0, col);
            c.color(col);
            c.circle(x, y, 12.0);
            c.no_glow();
            // Little fins.
            c.polygon(&[(x - 11.0, y - 2.0), (x - 18.0, y + 6.0), (x - 9.0, y + 6.0)]);
            c.polygon(&[(x + 11.0, y - 2.0), (x + 18.0, y + 6.0), (x + 9.0, y + 6.0)]);
            // One big eye that watches your ship.
            c.color("#0b1020");
            c.circle(x, y, 6.5);
            let look = ((ship_x - x) / 120.0).clamp(-1.0, 1.0) * 2.5;
            c.color("#ffffff");
            c.circle(x + look, y + 1.5, 2.6);
        }
        AlienKind::Stinger => {
            let col = if white { "#ffffff" } else { STINGER };
            let flap = (t * 12.0 + a.col as f64).sin() * 5.0;
            c.color(if white { "#ffffff" } else { "#ff5dbb88" });
            c.polygon(&[(x - 4.0, y - 4.0), (x - 19.0, y - 10.0 + flap), (x - 14.0, y + 4.0)]);
            c.polygon(&[(x + 4.0, y - 4.0), (x + 19.0, y - 10.0 + flap), (x + 14.0, y + 4.0)]);
            c.glow(12.0, col);
            c.color(col);
            c.polygon(&[(x - 10.0, y - 10.0), (x + 10.0, y - 10.0), (x + 4.0, y + 4.0), (x, y + 15.0), (x - 4.0, y + 4.0)]);
            c.no_glow();
            c.color("#0b1020");
            c.rect(x - 6.0, y - 6.0, 4.0, 3.0);
            c.rect(x + 2.0, y - 6.0, 4.0, 3.0);
        }
        AlienKind::Warden => {
            let col = if white { "#ffffff" } else { WARDEN };
            c.glow(12.0, col);
            c.color(col);
            c.round_rect(x - 14.0, y - 9.0, 28.0, 18.0, 6.0);
            c.polygon(&[(x - 14.0, y - 4.0), (x - 19.0, y - 14.0), (x - 9.0, y - 9.0)]);
            c.polygon(&[(x + 14.0, y - 4.0), (x + 19.0, y - 14.0), (x + 9.0, y - 9.0)]);
            c.no_glow();
            c.color("#0b1020");
            c.rect(x - 9.0, y - 3.0, 18.0, 4.0);
            c.color("#ff4f7b");
            let scan = (t * 3.0 + a.col as f64).sin() * 6.0;
            c.rect(x + scan - 2.0, y - 3.0, 4.0, 4.0);
            if a.hp > 1 {
                // Armour: a glowing arc under it, gone after the first hit.
                let ctx = c.ctx();
                c.glow(10.0, "#bdf6ff");
                ctx.set_stroke_style_str("#bdf6ffcc");
                ctx.set_line_width(2.5);
                ctx.begin_path();
                let _ = ctx.arc(x, y - 4.0, 20.0, 0.15 * PI, 0.85 * PI);
                ctx.stroke();
                c.no_glow();
            }
        }
    }
}

pub fn modern(g: &StarDefender, c: &Canvas) {
    let bg = c.linear_gradient(0.0, 0.0, 0.0, H, &[(0.0, "#04050d"), (1.0, "#0d1233")]);
    c.gradient(&bg);
    c.rect(0.0, 0.0, W, H);
    for s in &g.stars {
        c.alpha(0.25 + 0.3 * s.layer as f64);
        c.color("#cfd8ff");
        c.rect(s.x, s.y, 1.0 + s.layer as f64 * 0.5, 1.0 + s.layer as f64 * 0.5);
    }
    c.alpha(1.0);

    let (sx, sy) = shake(g);
    let ctx = c.ctx();
    ctx.save();
    let _ = ctx.translate(sx, sy);

    for d in &g.drops {
        let col = match d.kind {
            Power::Twin => "#7cf2ff",
            Power::Rapid => "#ffd166",
            Power::Shield => "#b38bff",
        };
        let bob = (g.time * 5.0).sin() * 2.0;
        c.glow(14.0, col);
        c.color(col);
        c.round_rect(d.x - 12.0, d.y - 8.0 + bob, 24.0, 16.0, 8.0);
        c.no_glow();
        c.color("#0b1020");
        c.text(power_letter(d.kind), d.x, d.y + bob + 1.0, "700 12px 'Space Grotesk', sans-serif", "center");
    }

    for a in &g.aliens {
        if a.y > -30.0 {
            alien_modern(c, a, g.ship_x, g.time);
        }
    }

    c.glow(10.0, SHIP);
    c.color(SHIP);
    for s in &g.shots {
        c.round_rect(s.x - 1.5, s.y - 8.0, 3.0, 16.0, 1.5);
    }
    c.glow(10.0, BOLT);
    c.color(BOLT);
    for b in &g.bolts {
        c.ellipse(b.x, b.y, 2.6, 7.0, -(b.vx / b.vy.max(1.0)).atan());
    }
    c.no_glow();

    if g.ship_visible() {
        ship_shape(c, g.ship_x, SHIP_Y, SHIP, g.time);
        if g.shield {
            let pulse = 0.5 + 0.3 * (g.time * 6.0).sin();
            c.glow(14.0, "#b38bff");
            ctx.set_stroke_style_str(&format!("rgba(179,139,255,{pulse:.2})"));
            ctx.set_line_width(2.5);
            ctx.begin_path();
            let _ = ctx.arc(g.ship_x, SHIP_Y, 26.0, 0.0, TAU);
            ctx.stroke();
            c.no_glow();
        }
    }

    for p in &g.particles {
        let k = p.life / p.max;
        c.alpha(k);
        c.color(hue(p.hue));
        c.circle(p.x, p.y, 1.0 + 2.2 * k);
    }
    c.alpha(1.0);
    ctx.restore();

    // HUD inside the canvas: wave, power-ups, lives.
    let small = "600 12px 'Space Grotesk', sans-serif";
    c.color("#9aa3c7");
    c.text(&format!("WAVE {}", g.wave), 14.0, 20.0, small, "left");
    for (i, label) in power_labels(g).iter().enumerate() {
        c.color("#e8ecff");
        c.text(label, W - 14.0, 20.0 + i as f64 * 18.0, small, "right");
    }
    for i in 0..g.lives.min(8) {
        let x = 22.0 + i as f64 * 26.0;
        ctx.save();
        let _ = ctx.translate(x, H - 18.0);
        let _ = ctx.scale(0.5, 0.5);
        ship_shape(c, 0.0, 0.0, "#7cf2ffaa", 0.0);
        ctx.restore();
    }
    if g.banner > 0.0 && g.state == engine::GameState::Playing {
        let k = (g.banner / BANNER_S).min(1.0);
        c.alpha((k * 2.0).min(1.0));
        c.glow(20.0, "#8f7cff");
        c.color("#e8ecff");
        c.text(&format!("WAVE {}", g.wave), W / 2.0, H * 0.45, "700 40px 'Space Grotesk', sans-serif", "center");
        c.no_glow();
        c.alpha(1.0);
    }
}

// ---------------------------------------------------------------- retro

const LCD_BG: &str = "#c3cf9b";
const LCD_GHOST: &str = "#b7c48f";
const LCD_INK: &str = "#2c3a23";
const PX: f64 = 3.0;

const SHIP_ART: [&str; 7] = [
    ".....X.....",
    "....XXX....",
    "....X.X....",
    ".X.XXXXX.X.",
    "XXXXXXXXXXX",
    "XXXXX.XXXXX",
    "XX..XXX..XX",
];
const DRONE_ART: [&str; 7] = [
    "...XXX...",
    ".XXXXXXX.",
    "XXX...XXX",
    "XX..X..XX",
    "XXX...XXX",
    ".XXXXXXX.",
    "..X.X.X..",
];
const STINGER_ART: [&str; 7] = [
    "X.......X",
    "XX.....XX",
    "XXX.X.XXX",
    ".XXXXXXX.",
    "..XXXXX..",
    "...XXX...",
    "....X....",
];
const WARDEN_ART: [&str; 7] = [
    "X.........X",
    "XX.XXXXX.XX",
    ".XXXXXXXXX.",
    "XXX.....XXX",
    "XXXXXXXXXXX",
    ".XXXXXXXXX.",
    "..X.X.X.X..",
];

fn sprite(c: &Canvas, art: &[&str], cx: f64, cy: f64, px: f64) {
    let w = art[0].len() as f64 * px;
    let h = art.len() as f64 * px;
    let (x0, y0) = ((cx - w / 2.0).round(), (cy - h / 2.0).round());
    for (r, row) in art.iter().enumerate() {
        for (k, ch) in row.bytes().enumerate() {
            if ch == b'X' {
                c.rect(x0 + k as f64 * px, y0 + r as f64 * px, px, px);
            }
        }
    }
}

pub fn retro(g: &StarDefender, c: &Canvas) {
    c.color(LCD_BG);
    c.rect(0.0, 0.0, W, H);
    c.color(LCD_GHOST);
    for s in &g.stars {
        if s.layer > 0 {
            c.rect((s.x / 3.0).floor() * 3.0, (s.y / 3.0).floor() * 3.0, 3.0, 3.0);
        }
    }
    let (sx, sy) = shake(g);
    let ctx = c.ctx();
    ctx.save();
    let _ = ctx.translate(sx.round(), sy.round());

    c.color(LCD_INK);
    for d in &g.drops {
        c.frame(d.x - 11.0, d.y - 8.0, 22.0, 16.0, 2.0);
        c.text(power_letter(d.kind), d.x + 1.0, d.y + 1.0, "8px 'Press Start 2P', monospace", "center");
    }
    for a in &g.aliens {
        if a.y < -30.0 || (a.flash > 0.0 && (g.time * 30.0).fract() < 0.5) {
            continue;
        }
        // Two-frame animation: sprites shift a pixel side to side.
        let wiggle = if (g.time * 4.0 + a.col as f64 * 0.5).fract() < 0.5 { 0.0 } else { PX };
        let art: &[&str] = match a.kind {
            AlienKind::Drone => &DRONE_ART,
            AlienKind::Stinger => &STINGER_ART,
            AlienKind::Warden => &WARDEN_ART,
        };
        c.color(LCD_INK);
        sprite(c, art, a.x + wiggle - PX / 2.0, a.y, PX);
        if a.kind == AlienKind::Warden && a.hp > 1 {
            c.rect(a.x - 15.0, a.y + 14.0, 30.0, 3.0);
        }
    }
    c.color(LCD_INK);
    for s in &g.shots {
        c.rect(s.x.round() - 1.5, s.y.round() - 7.0, 3.0, 12.0);
    }
    for b in &g.bolts {
        // Zig-zag bolt.
        let zig = if (b.y / 6.0) as i32 % 2 == 0 { -3.0 } else { 0.0 };
        c.rect(b.x.round() - 1.5 + zig, b.y.round() - 6.0, 3.0, 6.0);
        c.rect(b.x.round() - 1.5 - zig, b.y.round(), 3.0, 6.0);
    }
    if g.ship_visible() {
        sprite(c, &SHIP_ART, g.ship_x, SHIP_Y, PX);
        if g.shield && (g.time * 4.0).fract() < 0.7 {
            let mut k = 0.0;
            while k < TAU {
                c.rect((g.ship_x + k.cos() * 26.0).round(), (SHIP_Y + k.sin() * 26.0).round(), 3.0, 3.0);
                k += TAU / 24.0;
            }
        }
    }
    for p in &g.particles {
        if p.life / p.max > 0.25 {
            c.rect((p.x / 3.0).round() * 3.0, (p.y / 3.0).round() * 3.0, 3.0, 3.0);
        }
    }
    ctx.restore();

    let font = "9px 'Press Start 2P', monospace";
    c.color(LCD_INK);
    c.text(&format!("WAVE {}", g.wave), 12.0, 18.0, font, "left");
    for (i, label) in power_labels(g).iter().enumerate() {
        c.text(label, W - 12.0, 18.0 + i as f64 * 16.0, font, "right");
    }
    for i in 0..g.lives.min(8) {
        sprite(c, &SHIP_ART, 22.0 + i as f64 * 24.0, H - 16.0, 2.0);
    }
    if g.banner > 0.0 && g.state == engine::GameState::Playing && (g.banner * 4.0).fract() < 0.7 {
        c.text(&format!("WAVE {}", g.wave), W / 2.0, H * 0.45, "20px 'Press Start 2P', monospace", "center");
    }
}
