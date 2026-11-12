use crate::logic::{MineFinder, Outcome, BAR_H, W};
use engine::Canvas;
use std::f64::consts::TAU;

/// Seconds between each mine appearing in the reveal ripple after a loss.
const RIPPLE: f64 = 0.035;

/// What to draw in one square.
#[derive(Clone, Copy, PartialEq)]
enum Look {
    Hidden,
    Flag,
    /// A flag that turned out to be wrong (shown after losing).
    WrongFlag,
    Open(u8),
    Mine,
    /// The mine that was opened.
    Boom,
}

fn look(g: &MineFinder, i: usize) -> Look {
    let c = g.cells[i];
    match g.outcome {
        Some(Outcome::Lost(b)) if b == i => return Look::Boom,
        Some(Outcome::Lost(b)) => {
            // Mines appear in a ripple spreading out from the one you hit.
            let (bx, by) = g.xy(b);
            let (x, y) = g.xy(i);
            let dist = (bx as f64 - x as f64).hypot(by as f64 - y as f64);
            let shown = g.time - g.ended_at > dist * RIPPLE;
            if c.mine && !c.flag && shown {
                return Look::Mine;
            }
            if c.flag && !c.mine && shown {
                return Look::WrongFlag;
            }
        }
        _ => {}
    }
    if c.open {
        Look::Open(c.near)
    } else if c.flag {
        Look::Flag
    } else {
        Look::Hidden
    }
}

fn clock(g: &MineFinder) -> String {
    format!("{:03}", (g.elapsed.floor() as u32).min(999))
}

/// Calls `f(index, x, y, size)` for every square.
fn each_cell(g: &MineFinder, mut f: impl FnMut(usize, f64, f64, f64)) {
    let l = g.layout();
    for i in 0..g.cells.len() {
        let (x, y) = g.xy(i);
        f(i, l.ox + x as f64 * l.cell, l.oy + y as f64 * l.cell, l.cell);
    }
}

fn highlighted(g: &MineFinder, i: usize) -> bool {
    if g.outcome.is_some() {
        return false;
    }
    if g.show_cursor { g.cursor == i } else { g.hover == Some(i) }
}

// ---------------------------------------------------------------- modern

const BG: &str = "#0b1020";
const TILE: &str = "#26305c";
const TILE_HOVER: &str = "#34417a";
const OPEN: &str = "#121831";
const TEXT: &str = "#e8ecff";
const MUTED: &str = "#9aa3c7";
const FLAG: &str = "#ffc857";
const DANGER: &str = "#ff4f6d";
const NUMBERS: [&str; 9] = [
    "", "#5cc8ff", "#6ee7a8", "#ff6b8b", "#b18cff", "#ffb35c", "#4fe0d0", "#e8ecff", "#9aa3c7",
];

fn flag(c: &Canvas, x: f64, y: f64, s: f64, color: &str) {
    let pole = x + s * 0.38;
    c.line(pole, y + s * 0.22, pole, y + s * 0.78, (s * 0.07).max(1.5), color);
    c.color(color);
    c.polygon(&[(pole, y + s * 0.2), (x + s * 0.74, y + s * 0.34), (pole, y + s * 0.5)]);
    c.rect(x + s * 0.28, y + s * 0.74, s * 0.24, s * 0.06);
}

fn mine(c: &Canvas, x: f64, y: f64, s: f64, body: &str) {
    let (cx, cy) = (x + s / 2.0, y + s / 2.0);
    for k in 0..8 {
        let a = k as f64 * TAU / 8.0;
        let (dx, dy) = (a.cos() * s * 0.32, a.sin() * s * 0.32);
        c.line(cx - dx, cy - dy, cx + dx, cy + dy, (s * 0.07).max(1.5), body);
    }
    c.color(body);
    c.circle(cx, cy, s * 0.22);
    c.color("#ffffff99");
    c.circle(cx - s * 0.07, cy - s * 0.07, s * 0.06);
}

fn bar_modern(g: &MineFinder, c: &Canvas) {
    let y = BAR_H / 2.0 + 4.0;
    let big = "700 22px 'Space Grotesk', sans-serif";
    let small = "600 12px 'Space Grotesk', sans-serif";
    mine(c, 16.0, y - 12.0, 24.0, DANGER);
    c.color(TEXT);
    c.text(&g.mines_left().to_string(), 48.0, y, big, "left");
    c.color(MUTED);
    let mode = if g.flag_mode { format!("{} · FLAG MODE", g.diff.name().to_uppercase()) } else { g.diff.name().to_uppercase() };
    c.text(&mode, W / 2.0, y, small, "center");
    c.color(TEXT);
    c.text(&clock(g), W - 16.0, y, big, "right");
    // Little clock face.
    let (cx, cy) = (W - 78.0, y);
    c.ctx().set_stroke_style_str(MUTED);
    c.ctx().set_line_width(2.0);
    c.ctx().begin_path();
    let _ = c.ctx().arc(cx, cy, 8.0, 0.0, TAU);
    c.ctx().stroke();
    c.line(cx, cy, cx, cy - 5.0, 2.0, MUTED);
    c.line(cx, cy, cx + 4.0, cy, 2.0, MUTED);
}

pub fn modern(g: &MineFinder, c: &Canvas) {
    c.color(BG);
    c.rect(0.0, 0.0, W, crate::logic::H);
    bar_modern(g, c);

    let gap = 2.0;
    let won = g.outcome == Some(Outcome::Won);
    each_cell(g, |i, x, y, s| {
        let (bx, by, bs) = (x + gap / 2.0, y + gap / 2.0, s - gap);
        let r = (s * 0.18).min(6.0);
        match look(g, i) {
            Look::Hidden | Look::Flag | Look::WrongFlag => {
                c.color(if highlighted(g, i) { TILE_HOVER } else { TILE });
                c.round_rect(bx, by, bs, bs, r);
                c.color("#ffffff14");
                c.round_rect(bx, by, bs, bs * 0.45, r);
            }
            Look::Open(_) | Look::Mine => {
                c.color(OPEN);
                c.round_rect(bx, by, bs, bs, r);
            }
            Look::Boom => {
                c.glow(18.0, DANGER);
                c.color(DANGER);
                c.round_rect(bx, by, bs, bs, r);
                c.no_glow();
            }
        }
        match look(g, i) {
            Look::Flag => {
                if won {
                    c.glow(10.0 + 6.0 * (g.time * 4.0).sin().abs(), FLAG);
                }
                flag(c, x, y, s, FLAG);
                c.no_glow();
            }
            Look::WrongFlag => {
                flag(c, x, y, s, MUTED);
                c.line(x + s * 0.25, y + s * 0.25, x + s * 0.75, y + s * 0.75, 2.5, DANGER);
                c.line(x + s * 0.75, y + s * 0.25, x + s * 0.25, y + s * 0.75, 2.5, DANGER);
            }
            Look::Open(n) if n > 0 => {
                c.color(NUMBERS[n as usize]);
                let font = format!("700 {}px 'Space Grotesk', sans-serif", (s * 0.58).round());
                c.text(&n.to_string(), x + s / 2.0, y + s / 2.0 + 1.0, &font, "center");
            }
            Look::Mine => mine(c, x, y, s, "#d8def5"),
            Look::Boom => mine(c, x, y, s, "#1b1f33"),
            _ => {}
        }
        if highlighted(g, i) && g.show_cursor {
            c.ctx().set_stroke_style_str("#7cf2ff");
            c.ctx().set_line_width(2.0);
            c.ctx().stroke_rect(bx + 1.0, by + 1.0, bs - 2.0, bs - 2.0);
        }
    });
}

// ---------------------------------------------------------------- retro

const LCD_BG: &str = "#c3cf9b";
const LCD_GHOST: &str = "#b7c48f";
const LCD_INK: &str = "#2c3a23";

fn pixel_flag(c: &Canvas, x: f64, y: f64, s: f64, ink: &str) {
    let u = (s / 10.0).floor().max(2.0);
    c.color(ink);
    c.rect(x + 4.0 * u, y + 2.0 * u, u, 6.0 * u); // pole
    c.rect(x + 5.0 * u, y + 2.0 * u, 2.0 * u, u);
    c.rect(x + 5.0 * u, y + 3.0 * u, 3.0 * u, u);
    c.rect(x + 5.0 * u, y + 4.0 * u, 2.0 * u, u);
    c.rect(x + 3.0 * u, y + 8.0 * u, 3.0 * u, u); // base
}

fn pixel_mine(c: &Canvas, x: f64, y: f64, s: f64, ink: &str) {
    let u = (s / 10.0).floor().max(2.0);
    let o = (s - 10.0 * u) / 2.0;
    let (x, y) = (x + o, y + o);
    c.color(ink);
    c.rect(x + 3.0 * u, y + 2.0 * u, 4.0 * u, 6.0 * u);
    c.rect(x + 2.0 * u, y + 3.0 * u, 6.0 * u, 4.0 * u);
    c.rect(x + 4.5 * u, y + u, u, 8.0 * u);
    c.rect(x + u, y + 4.5 * u, 8.0 * u, u);
}

pub fn retro(g: &MineFinder, c: &Canvas) {
    c.color(LCD_BG);
    c.rect(0.0, 0.0, W, crate::logic::H);

    let y = BAR_H / 2.0 + 4.0;
    let big = "16px 'Press Start 2P', monospace";
    let small = "9px 'Press Start 2P', monospace";
    pixel_mine(c, 14.0, y - 12.0, 24.0, LCD_INK);
    c.color(LCD_INK);
    c.text(&format!("{:03}", g.mines_left().max(-99)), 44.0, y, big, "left");
    let mode = if g.flag_mode { format!("{} FLAG", g.diff.name().to_uppercase()) } else { g.diff.name().to_uppercase() };
    c.text(&mode, W / 2.0, y, small, "center");
    c.text(&clock(g), W - 16.0, y, big, "right");
    c.rect(0.0, BAR_H - 4.0, W, 3.0);

    let blink = (g.time * 3.0).fract() < 0.6;
    each_cell(g, |i, x, y, s| {
        match look(g, i) {
            Look::Hidden | Look::Flag | Look::WrongFlag => {
                c.color(LCD_INK);
                c.rect(x + 1.0, y + 1.0, s - 2.0, s - 2.0);
                c.color(LCD_GHOST);
                c.rect(x + 3.0, y + 3.0, s - 6.0, s - 6.0);
                if highlighted(g, i) {
                    c.color(LCD_BG);
                    c.rect(x + 5.0, y + 5.0, s - 10.0, s - 10.0);
                }
            }
            Look::Open(_) | Look::Mine => {
                c.color(LCD_GHOST);
                c.rect(x + s / 2.0 - 1.0, y + s / 2.0 - 1.0, 2.0, 2.0);
            }
            Look::Boom => {
                c.color(LCD_INK);
                c.rect(x + 1.0, y + 1.0, s - 2.0, s - 2.0);
            }
        }
        match look(g, i) {
            Look::Flag => pixel_flag(c, x, y, s, LCD_INK),
            Look::WrongFlag => {
                pixel_flag(c, x, y, s, LCD_INK);
                c.line(x + 4.0, y + 4.0, x + s - 4.0, y + s - 4.0, 3.0, LCD_INK);
            }
            Look::Open(n) if n > 0 => {
                c.color(LCD_INK);
                let font = format!("{}px 'Press Start 2P', monospace", (s * 0.45).round());
                c.text(&n.to_string(), x + s / 2.0 + 1.0, y + s / 2.0 + 1.0, &font, "center");
            }
            Look::Mine => pixel_mine(c, x, y, s, LCD_INK),
            Look::Boom => pixel_mine(c, x, y, s, LCD_BG),
            _ => {}
        }
        if g.show_cursor && highlighted(g, i) && blink {
            c.color(LCD_INK);
            c.frame(x - 1.0, y - 1.0, s + 2.0, s + 2.0, 3.0);
        }
    });
}
