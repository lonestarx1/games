use crate::logic::{BlockDrop, Kind, Piece, BLAST_MS, CLEAR_MS, COLS, PREVIEW, ROWS};
use engine::render::Rgb;
use engine::{Canvas, GameState};

pub const CELL: f64 = 26.0;
const PAD: f64 = 14.0;
const BOARD_W: f64 = COLS as f64 * CELL;
const BOARD_H: f64 = ROWS as f64 * CELL;
const PANEL_X: f64 = PAD * 2.0 + BOARD_W;
const PANEL_W: f64 = 112.0;
pub const W: f64 = PANEL_X + PANEL_W + PAD;
pub const H: f64 = BOARD_H + PAD * 2.0;

const MINI: f64 = 16.0;
const NEXT_SLOT: f64 = 56.0;
const NEXT_Y: f64 = PAD + 22.0;
const HOLD_Y: f64 = NEXT_Y + NEXT_SLOT * PREVIEW as f64 + 36.0;
const HOLD_H: f64 = 64.0;
const STATS_Y: f64 = HOLD_Y + HOLD_H + 34.0;

/// Panel layout: (label, label y) for each box.
fn labels() -> [(&'static str, f64); 4] {
    [
        ("NEXT", NEXT_Y - 12.0),
        ("HOLD", HOLD_Y - 12.0),
        ("LEVEL", STATS_Y),
        ("ROWS", STATS_Y + 60.0),
    ]
}

fn cell_xy(x: i32, y: i32) -> (f64, f64) {
    (PAD + x as f64 * CELL, PAD + y as f64 * CELL)
}

/// Calls `f(x, y, size, is_bomb)` for every block of a shape drawn small in a box.
fn mini_shape(kind: Kind, bomb: Option<usize>, cx: f64, cy: f64, mut f: impl FnMut(f64, f64, f64, bool)) {
    let cells = kind.cells(0);
    let x0 = cells.iter().map(|c| c.0).min().unwrap();
    let x1 = cells.iter().map(|c| c.0).max().unwrap();
    let y0 = cells.iter().map(|c| c.1).min().unwrap();
    let y1 = cells.iter().map(|c| c.1).max().unwrap();
    let ox = cx - (x1 - x0 + 1) as f64 * MINI / 2.0 - x0 as f64 * MINI;
    let oy = cy - (y1 - y0 + 1) as f64 * MINI / 2.0 - y0 as f64 * MINI;
    for (i, (x, y)) in cells.into_iter().enumerate() {
        f(ox + x as f64 * MINI, oy + y as f64 * MINI, MINI, bomb == Some(i));
    }
}

/// Visible blocks of a piece with a flag for the bomb block.
fn piece_blocks(p: &Piece) -> impl Iterator<Item = (i32, i32, bool)> {
    let bomb = p.bomb;
    p.blocks()
        .into_iter()
        .enumerate()
        .filter(|&(_, (_, y))| y >= 0)
        .map(move |(i, (x, y))| (x, y, bomb == Some(i)))
}

/// For each column the piece covers: (column, lowest piece row, landing row).
fn drop_columns(g: &BlockDrop) -> Vec<(i32, i32, i32)> {
    let land = g.landing();
    let mut cols: Vec<(i32, i32, i32)> = Vec::new();
    for ((x, y), (_, ly)) in g.piece.blocks().into_iter().zip(land.blocks()) {
        match cols.iter_mut().find(|c| c.0 == x) {
            Some(c) => {
                c.1 = c.1.max(y);
                c.2 = c.2.max(ly);
            }
            None => cols.push((x, y, ly)),
        }
    }
    cols
}

/// Fade-out amount for the clear flash, 1 → 0.
fn clear_t(g: &BlockDrop) -> f64 {
    (g.clear_ms / CLEAR_MS).clamp(0.0, 1.0)
}

fn show_piece(g: &BlockDrop) -> bool {
    g.clearing.is_empty() && g.state != GameState::Ready
}

// ---------------------------------------------------------------- modern

const BG: &str = "#070a17";
const WELL: &str = "#0b1020";
const GRID: &str = "#141b36";
const PANEL: &str = "#11162c";
const TEXT: &str = "#e8ecff";
const MUTED: &str = "#9aa3c7";
const PIECE: &str = "#f4f7ff";

/// Each level has its own colour pair: the pile shades from the first colour
/// at the bottom of the well to the second at the top.
const PALETTES: [(Rgb, Rgb); 6] = [
    (Rgb(0xff, 0x5f, 0x6d), Rgb(0xff, 0xc3, 0x71)), // ember
    (Rgb(0x0c, 0xb4, 0x8a), Rgb(0x7c, 0xf2, 0xff)), // lagoon
    (Rgb(0x7a, 0x5c, 0xff), Rgb(0xff, 0x7a, 0xe0)), // dusk
    (Rgb(0x2f, 0x80, 0xed), Rgb(0x9b, 0xff, 0xb0)), // aurora
    (Rgb(0xe8, 0x5d, 0x04), Rgb(0xff, 0xe0, 0x66)), // desert
    (Rgb(0xb0, 0x3a, 0xff), Rgb(0x5e, 0xe7, 0xff)), // neon
];

fn palette(level: u32) -> (Rgb, Rgb) {
    PALETTES[(level as usize - 1) % PALETTES.len()]
}

fn row_color(level: u32, y: f64) -> String {
    let (bottom, top) = palette(level);
    bottom.lerp(top, 1.0 - y / (ROWS as f64 - 1.0)).css()
}

fn tile(c: &Canvas, x: f64, y: f64, s: f64, color: &str) {
    c.color(color);
    c.round_rect(x + 1.5, y + 1.5, s - 3.0, s - 3.0, s * 0.22);
    // Translucent colours instead of global alpha, so callers can fade a whole shape.
    c.color("#ffffff47");
    c.round_rect(x + 4.0, y + 3.5, s - 8.0, (s - 8.0) * 0.32, s * 0.1);
}

fn bomb(c: &Canvas, x: f64, y: f64, s: f64, t: f64) {
    let (cx, cy) = (x + s / 2.0, y + s / 2.0 + s * 0.04);
    c.color("#1b1f33");
    c.circle(cx, cy, s * 0.36);
    c.color("#ffffff80");
    c.circle(cx - s * 0.12, cy - s * 0.12, s * 0.08);
    // Fuse spark flickers.
    let spark = 0.7 + 0.3 * (t * 18.0).sin();
    c.glow(10.0, "#ffb347");
    c.color("#ffb347");
    c.circle(cx + s * 0.24, cy - s * 0.3, s * 0.11 * spark);
    c.no_glow();
}

pub fn modern(g: &BlockDrop, c: &Canvas) {
    c.color(BG);
    c.rect(0.0, 0.0, W, H);

    // Well with a faint grid.
    c.color(WELL);
    c.round_rect(PAD - 4.0, PAD - 4.0, BOARD_W + 8.0, BOARD_H + 8.0, 10.0);
    c.color(GRID);
    for x in 1..COLS {
        c.rect(PAD + x as f64 * CELL, PAD, 1.0, BOARD_H);
    }
    for y in 1..ROWS {
        c.rect(PAD, PAD + y as f64 * CELL, BOARD_W, 1.0);
    }

    for (y, row) in g.board.iter().enumerate() {
        let col = row_color(g.level, y as f64);
        for (x, &filled) in row.iter().enumerate() {
            if filled {
                let (px, py) = cell_xy(x as i32, y as i32);
                tile(c, px, py, CELL, &col);
            }
        }
    }

    // Rows being cleared flash white, then fade.
    let t = clear_t(g);
    for &r in &g.clearing {
        c.alpha(t * 0.9);
        c.color("#ffffff");
        c.glow(20.0, "#ffffff");
        c.rect(PAD, PAD + r as f64 * CELL, BOARD_W, CELL);
        c.no_glow();
        c.alpha(1.0);
    }

    if let Some((bx, by)) = g.blast {
        let k = (g.blast_ms / BLAST_MS).clamp(0.0, 1.0);
        let (px, py) = cell_xy(bx, by);
        let (cx, cy) = (px + CELL / 2.0, py + CELL / 2.0);
        c.alpha(k * 0.6);
        c.color("#ffb347");
        c.round_rect(px - CELL, py - CELL, CELL * 3.0, CELL * 3.0, 8.0);
        c.alpha(k);
        c.ctx().set_stroke_style_str("#fff1c1");
        c.ctx().set_line_width(3.0);
        c.ctx().begin_path();
        let _ = c.ctx().arc(cx, cy, CELL * (0.5 + 2.2 * (1.0 - k)), 0.0, std::f64::consts::TAU);
        c.ctx().stroke();
        c.alpha(1.0);
    }

    if show_piece(g) {
        let (_, top) = palette(g.level);
        // A soft light beam shows which columns the shape will fall down.
        for (x, y, land) in drop_columns(g) {
            let (px, py) = cell_xy(x, y.max(-1) + 1);
            let bottom = PAD + (land + 1) as f64 * CELL;
            if bottom > py {
                let grad = c.linear_gradient(0.0, py, 0.0, bottom, &[(0.0, "#ffffff00"), (1.0, "#ffffff22")]);
                c.gradient(&grad);
                c.rect(px + 2.0, py, CELL - 4.0, bottom - py);
            }
            c.color(&top.css());
            c.rect(px + 3.0, bottom - 3.0, CELL - 6.0, 3.0);
        }
        c.glow(16.0, &top.css());
        for (x, y, is_bomb) in piece_blocks(&g.piece) {
            let (px, py) = cell_xy(x, y);
            tile(c, px, py, CELL, PIECE);
            if is_bomb {
                bomb(c, px, py, CELL, g.time);
            }
        }
        c.no_glow();
    }

    // Side panel.
    let label_font = "600 11px 'Space Grotesk', sans-serif";
    let value_font = "700 26px 'Space Grotesk', sans-serif";
    for (text, y) in labels() {
        c.color(MUTED);
        c.text(text, PANEL_X + 4.0, y, label_font, "left");
    }
    c.color(PANEL);
    c.round_rect(PANEL_X, NEXT_Y, PANEL_W, NEXT_SLOT * PREVIEW as f64, 12.0);
    c.round_rect(PANEL_X, HOLD_Y, PANEL_W, HOLD_H, 12.0);
    let shade = palette(g.level).1.css();
    for (i, s) in g.queue.iter().take(PREVIEW).enumerate() {
        c.alpha(if i == 0 { 1.0 } else { 0.55 });
        mini_shape(s.kind, s.bomb, PANEL_X + PANEL_W / 2.0, NEXT_Y + NEXT_SLOT * (i as f64 + 0.5), |x, y, sz, b| {
            tile(c, x, y, sz, &shade);
            if b {
                bomb(c, x, y, sz, g.time);
            }
        });
    }
    c.alpha(1.0);
    if let Some(s) = g.hold {
        c.alpha(if g.can_hold { 1.0 } else { 0.35 });
        mini_shape(s.kind, s.bomb, PANEL_X + PANEL_W / 2.0, HOLD_Y + HOLD_H / 2.0, |x, y, sz, b| {
            tile(c, x, y, sz, PIECE);
            if b {
                bomb(c, x, y, sz, g.time);
            }
        });
        c.alpha(1.0);
    }
    c.color(TEXT);
    c.text(&g.level.to_string(), PANEL_X + 4.0, STATS_Y + 26.0, value_font, "left");
    c.text(&g.lines.to_string(), PANEL_X + 4.0, STATS_Y + 86.0, value_font, "left");

    if let Some(text) = g.popup.filter(|_| g.popup_ms > 0.0) {
        c.alpha((g.popup_ms / 400.0).min(1.0));
        c.glow(18.0, &shade);
        c.color(&shade);
        c.text(text, PAD + BOARD_W / 2.0, PAD + BOARD_H * 0.35, "700 30px 'Space Grotesk', sans-serif", "center");
        c.no_glow();
        c.alpha(1.0);
    }
}

// ---------------------------------------------------------------- retro

const LCD_BG: &str = "#c3cf9b";
const LCD_GHOST: &str = "#b7c48f";
const LCD_INK: &str = "#2c3a23";

/// Settled block: hollow square with a centre dot.
fn brick(c: &Canvas, x: f64, y: f64, s: f64) {
    let u = (s / 8.0).round().max(2.0);
    c.color(LCD_INK);
    c.frame(x + 1.0, y + 1.0, s - 2.0, s - 2.0, u);
    c.rect(x + s / 2.0 - u, y + s / 2.0 - u, u * 2.0, u * 2.0);
}

/// Falling block: solid.
fn solid(c: &Canvas, x: f64, y: f64, s: f64) {
    c.color(LCD_INK);
    c.rect(x + 1.0, y + 1.0, s - 2.0, s - 2.0);
}

fn retro_bomb(c: &Canvas, x: f64, y: f64, s: f64, t: f64) {
    let u = (s / 8.0).round().max(2.0);
    c.color(LCD_BG);
    c.rect(x + 1.0, y + 1.0, s - 2.0, s - 2.0);
    c.color(LCD_INK);
    // Chunky pixel circle.
    c.rect(x + 2.0 * u, y + u * 2.0, s - 4.0 * u, s - 3.0 * u);
    c.rect(x + u, y + 3.0 * u, s - 2.0 * u, s - 5.0 * u);
    if (t * 4.0).fract() < 0.5 {
        c.rect(x + s - 2.0 * u, y, u, u);
    }
}

pub fn retro(g: &BlockDrop, c: &Canvas) {
    c.color(LCD_BG);
    c.rect(0.0, 0.0, W, H);
    c.color(LCD_INK);
    c.frame(PAD - 6.0, PAD - 6.0, BOARD_W + 12.0, BOARD_H + 12.0, 3.0);
    c.color(LCD_GHOST);
    for y in 0..ROWS {
        for x in 0..COLS {
            let (px, py) = cell_xy(x as i32, y as i32);
            c.rect(px + 1.0, py + 1.0, CELL - 2.0, CELL - 2.0);
        }
    }

    // Cleared rows blink before vanishing.
    let blink_off = !g.clearing.is_empty() && ((1.0 - clear_t(g)) * 8.0) as i32 % 2 == 0;
    for (y, row) in g.board.iter().enumerate() {
        if blink_off && g.clearing.contains(&y) {
            continue;
        }
        for (x, &filled) in row.iter().enumerate() {
            if filled {
                let (px, py) = cell_xy(x as i32, y as i32);
                brick(c, px, py, CELL);
            }
        }
    }

    if let Some((bx, by)) = g.blast {
        if ((g.blast_ms / BLAST_MS) * 6.0) as i32 % 2 == 0 {
            let (px, py) = cell_xy(bx - 1, by - 1);
            c.color(LCD_INK);
            c.frame(px, py, CELL * 3.0, CELL * 3.0, 3.0);
        }
    }

    if show_piece(g) {
        // Tick marks on the floor where the shape will land.
        c.color(LCD_INK);
        for (x, _, land) in drop_columns(g) {
            let (px, py) = cell_xy(x, land);
            for i in 0..3 {
                c.rect(px + 4.0 + i as f64 * 7.0, py + CELL - 4.0, 4.0, 3.0);
            }
        }
        for (x, y, is_bomb) in piece_blocks(&g.piece) {
            let (px, py) = cell_xy(x, y);
            if is_bomb {
                retro_bomb(c, px, py, CELL, g.time);
            } else {
                solid(c, px, py, CELL);
            }
        }
    }

    let font = "9px 'Press Start 2P', monospace";
    let big = "16px 'Press Start 2P', monospace";
    c.color(LCD_INK);
    for (text, y) in labels() {
        c.text(text, PANEL_X, y, font, "left");
    }
    c.frame(PANEL_X, NEXT_Y, PANEL_W, NEXT_SLOT * PREVIEW as f64, 3.0);
    c.frame(PANEL_X, HOLD_Y, PANEL_W, HOLD_H, 3.0);
    let draw_mini = |x: f64, y: f64, sz: f64, b: bool| {
        if b { retro_bomb(c, x, y, sz, g.time) } else { brick(c, x, y, sz) }
    };
    for (i, s) in g.queue.iter().take(PREVIEW).enumerate() {
        mini_shape(s.kind, s.bomb, PANEL_X + PANEL_W / 2.0, NEXT_Y + NEXT_SLOT * (i as f64 + 0.5), draw_mini);
    }
    if let Some(s) = g.hold {
        if g.can_hold || (g.time * 4.0).fract() < 0.7 {
            mini_shape(s.kind, s.bomb, PANEL_X + PANEL_W / 2.0, HOLD_Y + HOLD_H / 2.0, draw_mini);
        }
    }
    c.color(LCD_INK);
    c.text(&g.level.to_string(), PANEL_X, STATS_Y + 26.0, big, "left");
    c.text(&g.lines.to_string(), PANEL_X, STATS_Y + 86.0, big, "left");

    if let Some(text) = g.popup.filter(|_| g.popup_ms > 0.0) {
        let (cx, cy) = (PAD + BOARD_W / 2.0, PAD + BOARD_H * 0.35);
        c.color(LCD_BG);
        c.rect(cx - 100.0, cy - 18.0, 200.0, 36.0);
        c.color(LCD_INK);
        c.frame(cx - 100.0, cy - 18.0, 200.0, 36.0, 3.0);
        c.text(text, cx, cy + 1.0, "12px 'Press Start 2P', monospace", "center");
    }
}
