use crate::logic::{NumberMerge, Tile, TileKind, BOARD, BOARD_X, BOARD_Y, H, POP_MS, RING_MS, SLIDE_MS, SPARK_MS, W};
use engine::Canvas;
use std::f64::consts::{PI, TAU};

/// Top-left of a (possibly fractional) cell, in canvas pixels.
fn cell_xy(g: &NumberMerge, (r, c): (f64, f64)) -> (f64, f64) {
    let (gap, cell) = g.metrics();
    (BOARD_X + gap + c * (cell + gap), BOARD_Y + gap + r * (cell + gap))
}

/// Canvas position of a point given in cell units (0.5 = middle of the first cell).
fn point_xy(g: &NumberMerge, y: f64, x: f64) -> (f64, f64) {
    let (gap, cell) = g.metrics();
    let pitch = cell + gap;
    let at = |u: f64| gap + (u - 0.5) * pitch + cell / 2.0;
    (BOARD_X + at(x), BOARD_Y + at(y))
}

/// Slide progress 0..1 (eased) and pop progress 0..1 for the last move.
fn progress(g: &NumberMerge) -> (f64, f64) {
    let slide = (g.anim_ms / SLIDE_MS).clamp(0.0, 1.0);
    let pop = ((g.anim_ms - SLIDE_MS) / POP_MS).clamp(0.0, 1.0);
    (1.0 - (1.0 - slide).powi(3), pop)
}

/// A tile to draw: value, top-left, scale, and how much white flash to add.
struct Drawn {
    value: u32,
    x: f64,
    y: f64,
    scale: f64,
    flash: f64,
}

fn placed(g: &NumberMerge) -> Vec<Drawn> {
    // While dragging, tiles sit wherever the pointer has pushed them.
    if let Some(view) = g.drag_view() {
        return view
            .into_iter()
            .map(|(value, r, c)| {
                let (x, y) = cell_xy(g, (r, c));
                Drawn { value, x, y, scale: 1.0, flash: 0.0 }
            })
            .collect();
    }
    let (p, pop) = progress(g);
    let at = |t: &Tile| {
        let (fx, fy) = cell_xy(g, t.from);
        let (tx, ty) = cell_xy(g, (t.row as f64, t.col as f64));
        (fx + (tx - fx) * p, fy + (ty - fy) * p)
    };
    let mut out = Vec::new();
    for t in &g.ghosts {
        let (x, y) = at(t);
        out.push(Drawn { value: t.value, x, y, scale: 1.0, flash: 0.0 });
    }
    for t in &g.tiles {
        let (x, y) = at(t);
        let (scale, flash) = match t.kind {
            TileKind::Moved => (1.0, 0.0),
            _ if p < 1.0 => continue, // appears once the slide is done
            TileKind::Spawned => (0.3 + 0.7 * pop, 0.0),
            // Merges punch out big, then settle, with a white flash.
            TileKind::Merged => (1.0 + 0.3 * (pop * PI).sin() * (1.0 - pop * 0.4), (1.0 - pop) * 0.75),
        };
        out.push(Drawn { value: t.value, x, y, scale, flash });
    }
    // Merged tiles on top so the pop covers its neighbours.
    out.sort_by(|a, b| a.scale.total_cmp(&b.scale));
    out
}

fn font_size(value: u32, cell: f64) -> f64 {
    let digits = value.to_string().len();
    cell * match digits {
        1 | 2 => 0.46,
        3 => 0.38,
        4 => 0.3,
        _ => 0.24,
    }
}

// ---------------------------------------------------------------- modern

/// Cool blues for small numbers, warming up to hot pink and violet.
fn tile_colors(value: u32) -> (&'static str, &'static str) {
    match value {
        2 => ("#1f3b73", "#dbe6ff"),
        4 => ("#23558a", "#dbf0ff"),
        8 => ("#1f7a8c", "#e0fbff"),
        16 => ("#1f9e89", "#eafff9"),
        32 => ("#3fbf7f", "#06281a"),
        64 => ("#8fd14f", "#17260a"),
        128 => ("#f2c94c", "#2e2203"),
        256 => ("#f2994a", "#2e1603"),
        512 => ("#eb5757", "#fff0f0"),
        1024 => ("#d6409f", "#fff0fa"),
        2048 => ("#9b51e0", "#ffffff"),
        _ => ("#f4f7ff", "#2a1155"),
    }
}

/// Brighter version of a tile colour for sparks and rings.
fn spark_color(value: u32) -> &'static str {
    match value {
        0..=4 => "#7fb2ff",
        8 | 16 => "#5ef0d8",
        32 | 64 => "#b6f36a",
        128 | 256 => "#ffd76a",
        512 => "#ff7f7f",
        1024 => "#ff6fcf",
        _ => "#d3a6ff",
    }
}

pub fn modern(g: &NumberMerge, c: &Canvas) {
    c.color("#070a17");
    c.rect(0.0, 0.0, W, H);
    bar(g, c, "#9aa3c7", "#e8ecff", "600 12px 'Space Grotesk', sans-serif", "700 20px 'Space Grotesk', sans-serif");

    let (sx, sy) = g.shake();
    let ctx = c.ctx();
    ctx.save();
    let _ = ctx.translate(sx, sy);

    let (_, cell) = g.metrics();
    c.color("#11162c");
    c.round_rect(BOARD_X, BOARD_Y, BOARD, BOARD, 16.0);
    c.color("#1a2142");
    for r in 0..g.size {
        for col in 0..g.size {
            let (x, y) = cell_xy(g, (r as f64, col as f64));
            c.round_rect(x, y, cell, cell, 10.0);
        }
    }

    // Shockwaves under the tiles.
    for ring in g.rings.iter().filter(|r| r.age > 0.0) {
        let k = ring.age / RING_MS;
        let (x, y) = cell_xy(g, (ring.row as f64, ring.col as f64));
        let (cx, cy) = (x + cell / 2.0, y + cell / 2.0);
        c.alpha((1.0 - k) * 0.9);
        ctx.set_stroke_style_str(spark_color(ring.value));
        ctx.set_line_width(6.0 * (1.0 - k) + 1.0);
        ctx.begin_path();
        let _ = ctx.arc(cx, cy, cell * (0.45 + 0.75 * k), 0.0, TAU);
        ctx.stroke();
        c.alpha(1.0);
    }

    for d in placed(g) {
        let s = cell * d.scale;
        let (x, y) = (d.x + (cell - s) / 2.0, d.y + (cell - s) / 2.0);
        let (bg, fg) = tile_colors(d.value);
        if d.value >= 512 || d.flash > 0.0 {
            c.glow(if d.value >= 2048 { 24.0 } else { 14.0 }, spark_color(d.value));
        }
        c.color(bg);
        c.round_rect(x, y, s, s, 10.0 * d.scale);
        c.no_glow();
        c.color("#ffffff1c");
        c.round_rect(x + 4.0, y + 4.0, s - 8.0, s * 0.3, 7.0 * d.scale);
        if d.flash > 0.0 {
            c.alpha(d.flash);
            c.color("#ffffff");
            c.round_rect(x, y, s, s, 10.0 * d.scale);
            c.alpha(1.0);
        }
        c.color(fg);
        let font = format!("700 {}px 'Space Grotesk', sans-serif", (font_size(d.value, cell) * d.scale).round());
        c.text(&d.value.to_string(), x + s / 2.0, y + s / 2.0 + 1.0, &font, "center");
    }

    // Sparks fly over everything.
    for s in g.sparks.iter().filter(|s| s.age > 0.0) {
        let k = s.age / SPARK_MS;
        let (x, y) = point_xy(g, s.y, s.x);
        let col = spark_color(s.value);
        c.alpha(1.0 - k);
        c.glow(10.0, col);
        c.color(col);
        c.circle(x, y, s.size * cell * (1.0 - k * 0.6));
        c.no_glow();
        c.alpha(1.0);
    }
    ctx.restore();

    if let Some(text) = g.popup.as_deref().filter(|_| g.popup_ms > 0.0) {
        c.alpha((g.popup_ms / 400.0).min(1.0));
        c.color("#070a17cc");
        c.round_rect(W / 2.0 - 150.0, BOARD_Y + BOARD / 2.0 - 32.0, 300.0, 64.0, 32.0);
        c.glow(18.0, "#c79bff");
        c.color("#e4ccff");
        c.text(text, W / 2.0, BOARD_Y + BOARD / 2.0, "700 26px 'Space Grotesk', sans-serif", "center");
        c.no_glow();
        c.alpha(1.0);
    }
}

/// Strip above the board: best tile, board size, undos left.
fn bar(g: &NumberMerge, c: &Canvas, muted: &str, text: &str, small: &str, big: &str) {
    let label_y = 18.0;
    let value_y = 40.0;
    c.color(muted);
    c.text("BEST TILE", BOARD_X, label_y, small, "left");
    c.text("BOARD", W / 2.0, label_y, small, "center");
    c.text("UNDO", W - BOARD_X, label_y, small, "right");
    c.color(text);
    c.text(&g.best_tile().to_string(), BOARD_X, value_y, big, "left");
    c.text(&format!("{0} × {0}", g.size), W / 2.0, value_y, big, "center");
    let undo = if g.undos_left == 0 { "used up".to_string() } else { format!("{} left", g.undos_left) };
    c.text(&undo, W - BOARD_X, value_y, big, "right");
}

// ---------------------------------------------------------------- retro

const LCD_BG: &str = "#c3cf9b";
const LCD_GHOST: &str = "#b7c48f";
const LCD_INK: &str = "#2c3a23";

pub fn retro(g: &NumberMerge, c: &Canvas) {
    c.color(LCD_BG);
    c.rect(0.0, 0.0, W, H);
    bar(g, c, LCD_INK, LCD_INK, "8px 'Press Start 2P', monospace", "14px 'Press Start 2P', monospace");

    // Retro shake snaps to whole pixels.
    let (sx, sy) = g.shake();
    let ctx = c.ctx();
    ctx.save();
    let _ = ctx.translate(sx.round(), sy.round());

    let (_, cell) = g.metrics();
    c.color(LCD_INK);
    c.frame(BOARD_X, BOARD_Y, BOARD, BOARD, 4.0);
    c.color(LCD_GHOST);
    for r in 0..g.size {
        for col in 0..g.size {
            let (x, y) = cell_xy(g, (r as f64, col as f64));
            c.rect(x, y, cell, cell);
        }
    }

    for ring in g.rings.iter().filter(|r| r.age > 0.0) {
        let k = ring.age / RING_MS;
        // Expanding square in steps of 4px.
        let grow = ((k * cell * 0.7) / 4.0).floor() * 4.0;
        let (x, y) = cell_xy(g, (ring.row as f64, ring.col as f64));
        c.color(LCD_INK);
        c.frame(x - grow, y - grow, cell + grow * 2.0, cell + grow * 2.0, 3.0);
    }

    for d in placed(g) {
        // Chunky retro: no smooth scaling, just a pop of a few pixels.
        let grow = if d.scale > 1.05 { 6.0 } else if d.scale < 1.0 { -cell * (1.0 - d.scale) / 2.0 } else { 0.0 };
        let (x, y, s) = (d.x - grow, d.y - grow, cell + grow * 2.0);
        c.color(if d.flash > 0.4 { LCD_BG } else { LCD_INK });
        c.rect(x, y, s, s);
        if d.flash > 0.4 {
            c.color(LCD_INK);
            c.frame(x, y, s, s, 4.0);
        }
        // Bigger numbers get extra rings so they stand out at a glance.
        let rings = match d.value {
            0..=64 => 0,
            65..=512 => 1,
            _ => 2,
        };
        c.color(if d.flash > 0.4 { LCD_INK } else { LCD_BG });
        for k in 0..rings {
            let o = 4.0 + k as f64 * 5.0;
            c.frame(x + o, y + o, s - 2.0 * o, s - 2.0 * o, 2.0);
        }
        let font = format!("{}px 'Press Start 2P', monospace", (font_size(d.value, cell) * 0.72).round());
        c.text(&d.value.to_string(), x + s / 2.0 + 1.0, y + s / 2.0 + 2.0, &font, "center");
    }

    // Sparks as square pixels.
    c.color(LCD_INK);
    for s in g.sparks.iter().filter(|s| s.age > 0.0 && s.age < SPARK_MS * 0.8) {
        let (x, y) = point_xy(g, s.y, s.x);
        let px = (s.size * cell).max(4.0).round();
        c.rect((x / 2.0).round() * 2.0, (y / 2.0).round() * 2.0, px, px);
    }
    ctx.restore();

    if let Some(text) = g.popup.as_deref().filter(|_| g.popup_ms > 0.0) {
        let (cx, cy) = (W / 2.0, BOARD_Y + BOARD / 2.0);
        c.color(LCD_BG);
        c.rect(cx - 160.0, cy - 24.0, 320.0, 48.0);
        c.color(LCD_INK);
        c.frame(cx - 160.0, cy - 24.0, 320.0, 48.0, 3.0);
        c.text(text, cx, cy + 1.0, "12px 'Press Start 2P', monospace", "center");
    }
}
