use crate::logic::{Death, Lane, LaneKind, RoadHopper, CELL, COLS, H, VISIBLE_ROWS, W};
use engine::Canvas;
use std::f64::consts::{PI, TAU};

/// Screen y of the top of a row.
fn row_y(g: &RoadHopper, row: f64) -> f64 {
    H - (row - g.cam + 1.0) * CELL
}

/// Visible rows, bottom to top.
fn visible_rows(g: &RoadHopper) -> impl Iterator<Item = i64> {
    let first = g.cam.floor() as i64;
    first..first + VISIBLE_ROWS as i64 + 2
}

/// Bunny centre on screen, and how high it is in its hop.
fn bunny_pos(g: &RoadHopper) -> (f64, f64, f64) {
    let (col, row, t) = match g.hop {
        Some(h) => (
            h.from.0 + (h.to.0 - h.from.0) * h.t,
            h.from.1 as f64 + (h.to.1 - h.from.1) as f64 * h.t,
            h.t,
        ),
        None => (g.px, g.prow as f64, 0.0),
    };
    let lift = (t * PI).sin() * 12.0;
    (col * CELL + CELL / 2.0, row_y(g, row) + CELL / 2.0, lift)
}

// ---------------------------------------------------------------- modern

const CAR_COLORS: [&str; 5] = ["#ff5d6c", "#ffc145", "#4cc9f0", "#9b7bff", "#f8f8f2"];

fn draw_lane_modern(g: &RoadHopper, c: &Canvas, row: i64, lane: &Lane) {
    let y = row_y(g, row as f64);
    match lane.kind {
        LaneKind::Grass => {
            c.color(if row % 2 == 0 { "#3f8f4e" } else { "#46995a" });
            c.rect(0.0, y, W, CELL);
            // Tufts.
            c.color("#ffffff14");
            for k in 0..COLS {
                if (row * 7 + k as i64 * 3) % 5 == 0 {
                    c.rect(k as f64 * CELL + 12.0, y + 30.0, 3.0, 6.0);
                    c.rect(k as f64 * CELL + 17.0, y + 27.0, 3.0, 9.0);
                }
            }
        }
        LaneKind::Road => {
            c.color("#2a2f3a");
            c.rect(0.0, y, W, CELL);
            // Dashes between neighbouring road lanes.
            if g.lane(row + 1).is_some_and(|l| l.kind == LaneKind::Road) {
                c.color("#e8ecff55");
                let mut x = 6.0;
                while x < W {
                    c.rect(x, y - 1.5, 18.0, 3.0);
                    x += 36.0;
                }
            }
        }
        LaneKind::River => {
            c.color("#2e6fb5");
            c.rect(0.0, y, W, CELL);
            c.color("#ffffff22");
            let drift = (g.time * 20.0 * lane.dir * lane.speed) % 48.0;
            let mut x = -48.0 + drift;
            while x < W + 48.0 {
                c.rect(x, y + 14.0 + (row % 3) as f64 * 8.0, 16.0, 2.0);
                x += 48.0;
            }
        }
        LaneKind::Rail => {
            c.color("#5a5550");
            c.rect(0.0, y, W, CELL);
            c.color("#6e4b2e");
            let mut x = 2.0;
            while x < W {
                c.rect(x, y + 8.0, 6.0, CELL - 16.0);
                x += 14.0;
            }
            c.color("#c9ccd6");
            c.rect(0.0, y + 14.0, W, 3.0);
            c.rect(0.0, y + CELL - 17.0, W, 3.0);
        }
    }
}

fn draw_things_modern(g: &RoadHopper, c: &Canvas, row: i64, lane: &Lane) {
    let y = row_y(g, row as f64);
    match lane.kind {
        LaneKind::Grass => {
            for (k, &tree) in lane.blocked.iter().enumerate() {
                if tree {
                    let (cx, cy) = (k as f64 * CELL + CELL / 2.0, y + CELL / 2.0);
                    c.color("#00000033");
                    c.ellipse(cx + 3.0, cy + 10.0, 17.0, 7.0, 0.0);
                    c.color("#5b3a22");
                    c.rect(cx - 3.0, cy + 2.0, 6.0, 10.0);
                    c.color("#2c6e3f");
                    c.circle(cx, cy - 2.0, 16.0);
                    c.color("#3c8a52");
                    c.circle(cx - 4.0, cy - 6.0, 9.0);
                }
            }
            if let Some(k) = lane.berry {
                let (cx, cy) = (k as f64 * CELL + CELL / 2.0, y + CELL / 2.0);
                let bob = (g.time * 4.0).sin() * 1.5;
                c.color("#2c6e3f");
                c.ellipse(cx + 3.0, cy - 9.0 + bob, 5.0, 2.5, -0.5);
                c.glow(8.0, "#ff4f7b");
                c.color("#ff4f7b");
                for (dx, dy) in [(-4.0, 0.0), (4.0, 0.0), (0.0, 5.0)] {
                    c.circle(cx + dx, cy + dy + bob, 4.5);
                }
                c.no_glow();
            }
        }
        LaneKind::Road => {
            for m in &lane.movers {
                let x = m.x * CELL;
                let len = m.len * CELL;
                let body = CAR_COLORS[(m.style % 5) as usize];
                c.color("#00000040");
                c.round_rect(x + 4.0, y + 10.0, len - 8.0, CELL - 14.0, 8.0);
                c.color(body);
                c.round_rect(x + 3.0, y + 7.0, len - 6.0, CELL - 14.0, 8.0);
                c.color("#0b1020bb");
                let front = if lane.dir > 0.0 { x + len - 18.0 } else { x + 8.0 };
                c.round_rect(front, y + 11.0, 10.0, CELL - 22.0, 3.0);
                if m.len > 1.5 {
                    // Truck: cab and a container.
                    c.color("#00000030");
                    let split = if lane.dir > 0.0 { x + len - 24.0 } else { x + 22.0 };
                    c.rect(split, y + 7.0, 2.0, CELL - 14.0);
                }
                // Headlights.
                c.color("#fff6c2");
                let hx = if lane.dir > 0.0 { x + len - 6.0 } else { x + 3.0 };
                c.rect(hx, y + 10.0, 3.0, 5.0);
                c.rect(hx, y + CELL - 15.0, 3.0, 5.0);
            }
        }
        LaneKind::River => {
            for m in &lane.movers {
                let x = m.x * CELL;
                let len = m.len * CELL;
                c.color("#00000030");
                c.round_rect(x + 3.0, y + 11.0, len - 4.0, CELL - 16.0, 12.0);
                c.color("#8a5a34");
                c.round_rect(x + 2.0, y + 8.0, len - 4.0, CELL - 16.0, 12.0);
                c.color("#a5703f");
                c.rect(x + 12.0, y + 13.0, len - 24.0, 3.0);
                c.color("#c99a66");
                c.circle(x + 12.0, y + CELL / 2.0, 6.0);
                c.color("#8a5a34");
                c.circle(x + 12.0, y + CELL / 2.0, 2.5);
            }
        }
        LaneKind::Rail => {
            // Warning light at the side.
            let on = lane.warning() && (g.time * 6.0).fract() < 0.5;
            let lx = if lane.dir > 0.0 { 10.0 } else { W - 10.0 };
            c.color("#20232b");
            c.rect(lx - 2.0, y + 4.0, 4.0, 16.0);
            if on {
                c.glow(16.0, "#ff3b3b");
            }
            c.color(if on { "#ff3b3b" } else { "#5a2020" });
            c.circle(lx, y + 6.0, 5.0);
            c.no_glow();
            for m in &lane.movers {
                let x = m.x * CELL;
                let len = m.len * CELL;
                c.color("#c63d3d");
                c.round_rect(x, y + 5.0, len, CELL - 10.0, 6.0);
                c.color("#f2f2f2");
                let mut wx = x + 10.0;
                while wx < x + len - 16.0 {
                    c.rect(wx, y + 13.0, 14.0, 8.0);
                    wx += 24.0;
                }
                c.color("#00000033");
                let mut sx = x + 96.0;
                while sx < x + len {
                    c.rect(sx, y + 5.0, 3.0, CELL - 10.0);
                    sx += 96.0;
                }
            }
        }
    }
}

fn bunny_modern(g: &RoadHopper, c: &Canvas) {
    let (x, y, lift) = bunny_pos(g);
    match g.death {
        Some(Death::Squashed) => {
            c.color("#f4f1ea");
            c.ellipse(x, y + 4.0, 20.0, 6.0, 0.0);
            c.color("#ff9eb5");
            c.ellipse(x - 14.0, y + 2.0, 6.0, 3.0, 0.3);
            c.ellipse(x + 14.0, y + 2.0, 6.0, 3.0, -0.3);
            return;
        }
        Some(Death::Splashed) | Some(Death::SweptAway) => {
            let k = 1.0 - g.death_ms / 1100.0;
            let ctx = c.ctx();
            ctx.set_stroke_style_str("#e8f4ff");
            ctx.set_line_width(2.5);
            for r in [0.0, 0.3] {
                let kk = (k - r).max(0.0);
                c.alpha((1.0 - kk).max(0.0));
                ctx.begin_path();
                let _ = ctx.ellipse(x, y, 6.0 + kk * 26.0, 3.0 + kk * 12.0, 0.0, 0.0, TAU);
                ctx.stroke();
            }
            c.alpha(1.0);
            return;
        }
        _ => {}
    }
    // Shadow stays on the ground while the bunny jumps.
    c.color("#00000040");
    c.ellipse(x, y + 12.0, 14.0 - lift * 0.3, 5.0, 0.0);
    let y = y - lift;
    let squash = if g.hop.is_some() { 1.08 } else { 1.0 };
    let (fx, _) = g.facing;
    c.color("#f4f1ea");
    // Ears.
    c.ellipse(x - 6.0 + fx as f64 * 2.0, y - 18.0, 4.0, 10.0, -0.15);
    c.ellipse(x + 6.0 + fx as f64 * 2.0, y - 18.0, 4.0, 10.0, 0.15);
    c.color("#ff9eb5");
    c.ellipse(x - 6.0 + fx as f64 * 2.0, y - 18.0, 2.0, 6.0, -0.15);
    c.ellipse(x + 6.0 + fx as f64 * 2.0, y - 18.0, 2.0, 6.0, 0.15);
    c.color("#f4f1ea");
    c.ellipse(x, y, 14.0, 13.0 * squash, 0.0);
    // Face toward where it's going.
    let ex = fx as f64 * 3.0;
    c.color("#1b1f33");
    c.circle(x - 5.0 + ex, y - 3.0, 2.2);
    c.circle(x + 5.0 + ex, y - 3.0, 2.2);
    c.color("#ff7f9e");
    c.circle(x + ex, y + 2.0, 2.2);
}

pub fn modern(g: &RoadHopper, c: &Canvas) {
    c.color("#2f6b3c");
    c.rect(0.0, 0.0, W, H);
    let rows: Vec<i64> = visible_rows(g).collect();
    for &r in &rows {
        if let Some(lane) = g.lane(r) {
            draw_lane_modern(g, c, r, lane);
        }
    }
    // Things in top-to-bottom order so lower ones overlap the ones behind.
    let bunny_row = g.current_row();
    for &r in rows.iter().rev() {
        if let Some(lane) = g.lane(r) {
            draw_things_modern(g, c, r, lane);
        }
        if r == bunny_row {
            bunny_modern(g, c);
        }
    }
    hud(g, c, "#ffffff", "700 15px 'Space Grotesk', sans-serif");
}

fn hud(g: &RoadHopper, c: &Canvas, color: &str, font: &str) {
    c.color("#00000055");
    c.round_rect(8.0, 8.0, 150.0, 28.0, 14.0);
    c.color(color);
    c.text(&format!("ROWS {}  ·  🍓 {}", g.best_row, g.berries), 20.0, 22.0, font, "left");
}

// ---------------------------------------------------------------- retro

const LCD_BG: &str = "#c3cf9b";
const LCD_GHOST: &str = "#b7c48f";
const LCD_INK: &str = "#2c3a23";

const BUNNY: [&str; 10] = [
    "..X...X..",
    ".XX..XX..",
    ".XX..XX..",
    "..XXXXX..",
    ".XXXXXXX.",
    ".X.XXX.X.",
    ".XXXXXXX.",
    ".XXX.XXX.",
    "..XXXXX..",
    ".XX...XX.",
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

fn draw_lane_retro(g: &RoadHopper, c: &Canvas, row: i64, lane: &Lane) {
    let y = row_y(g, row as f64).round();
    match lane.kind {
        LaneKind::Grass => {
            c.color(LCD_GHOST);
            for k in 0..COLS * 3 {
                if (row * 5 + k as i64) % 4 == 0 {
                    c.rect(k as f64 * 16.0 + 6.0, y + 20.0 + (k % 2) as f64 * 10.0, 3.0, 3.0);
                }
            }
            c.color(LCD_INK);
            for (k, &tree) in lane.blocked.iter().enumerate() {
                if tree {
                    let (cx, cy) = (k as f64 * CELL + CELL / 2.0, y + CELL / 2.0);
                    c.rect(cx - 12.0, cy - 15.0, 24.0, 21.0);
                    c.rect(cx - 15.0, cy - 9.0, 30.0, 9.0);
                    c.rect(cx - 3.0, cy + 6.0, 6.0, 9.0);
                }
            }
            if let Some(k) = lane.berry {
                if (g.time * 3.0).fract() < 0.75 {
                    let (cx, cy) = (k as f64 * CELL + CELL / 2.0, y + CELL / 2.0);
                    c.rect(cx - 6.0, cy - 3.0, 6.0, 6.0);
                    c.rect(cx + 1.0, cy - 3.0, 6.0, 6.0);
                    c.rect(cx - 3.0, cy + 3.0, 6.0, 6.0);
                }
            }
        }
        LaneKind::Road => {
            c.color(LCD_GHOST);
            c.rect(0.0, y, W, CELL);
            c.color(LCD_INK);
            if g.lane(row + 1).is_some_and(|l| l.kind == LaneKind::Road) {
                let mut x = 6.0;
                while x < W {
                    c.rect(x, y - 1.0, 15.0, 3.0);
                    x += 30.0;
                }
            }
            for m in &lane.movers {
                let x = (m.x * CELL).round();
                let len = m.len * CELL;
                c.rect(x + 3.0, y + 9.0, len - 6.0, CELL - 18.0);
                c.color(LCD_GHOST);
                let win = if lane.dir > 0.0 { x + len - 15.0 } else { x + 9.0 };
                c.rect(win, y + 15.0, 6.0, CELL - 30.0);
                c.color(LCD_INK);
                // Wheels.
                for wx in [x + 6.0, x + len - 15.0] {
                    c.rect(wx, y + 6.0, 9.0, 3.0);
                    c.rect(wx, y + CELL - 9.0, 9.0, 3.0);
                }
            }
        }
        LaneKind::River => {
            c.color(LCD_INK);
            let drift = ((g.time * 12.0 * lane.dir) % 24.0).round();
            let mut x = -24.0 + drift;
            while x < W + 24.0 {
                c.rect(x, y + 18.0, 6.0, 3.0);
                c.rect(x + 6.0, y + 15.0, 6.0, 3.0);
                c.rect(x + 12.0, y + 18.0, 6.0, 3.0);
                x += 24.0;
            }
            for m in &lane.movers {
                let x = (m.x * CELL).round();
                let len = m.len * CELL;
                c.color(LCD_BG);
                c.rect(x + 2.0, y + 6.0, len - 4.0, CELL - 12.0);
                c.color(LCD_INK);
                c.frame(x + 2.0, y + 6.0, len - 4.0, CELL - 12.0, 3.0);
                c.rect(x + 12.0, y + 21.0, len - 24.0, 3.0);
            }
        }
        LaneKind::Rail => {
            c.color(LCD_INK);
            c.rect(0.0, y + 12.0, W, 3.0);
            c.rect(0.0, y + CELL - 15.0, W, 3.0);
            let mut x = 0.0;
            while x < W {
                c.rect(x, y + 8.0, 3.0, CELL - 16.0);
                x += 12.0;
            }
            if lane.warning() && (g.time * 6.0).fract() < 0.5 {
                let lx = if lane.dir > 0.0 { 6.0 } else { W - 18.0 };
                c.rect(lx, y + 2.0, 12.0, 12.0);
            }
            for m in &lane.movers {
                let x = (m.x * CELL).round();
                c.rect(x, y + 4.0, m.len * CELL, CELL - 8.0);
                c.color(LCD_BG);
                let mut wx = x + 9.0;
                while wx < x + m.len * CELL - 12.0 {
                    c.rect(wx, y + 12.0, 12.0, 9.0);
                    wx += 21.0;
                }
                c.color(LCD_INK);
            }
        }
    }
}

pub fn retro(g: &RoadHopper, c: &Canvas) {
    c.color(LCD_BG);
    c.rect(0.0, 0.0, W, H);
    for r in visible_rows(g) {
        if let Some(lane) = g.lane(r) {
            draw_lane_retro(g, c, r, lane);
        }
    }
    let (x, y, lift) = bunny_pos(g);
    c.color(LCD_INK);
    match g.death {
        Some(Death::Squashed) => c.rect(x - 15.0, y, 30.0, 6.0),
        Some(Death::Splashed) | Some(Death::SweptAway) => {
            let k = ((1.0 - g.death_ms / 1100.0) * 4.0).floor();
            c.frame(x - 6.0 - k * 5.0, y - 3.0 - k * 2.0, 12.0 + k * 10.0, 6.0 + k * 4.0, 3.0);
        }
        _ => {
            if g.hop.is_some() || (g.time * 2.0).fract() < 0.9 {
                sprite(c, &BUNNY, x, y - (lift / 3.0).round() * 3.0, 3.0);
            }
        }
    }
    c.color(LCD_BG);
    c.rect(6.0, 6.0, 168.0, 24.0);
    c.color(LCD_INK);
    c.frame(6.0, 6.0, 168.0, 24.0, 2.0);
    c.text(&format!("ROWS {} B {}", g.best_row, g.berries), 14.0, 19.0, "10px 'Press Start 2P', monospace", "left");
}
