use crate::logic::{box_of, col_of, row_of, Sudoku, BAR_H, CELL, GRID_X, GRID_Y, H, PAD_H, PAD_Y, W};
use engine::Canvas;

fn cell_xy(i: usize) -> (f64, f64) {
    (GRID_X + col_of(i) as f64 * CELL, GRID_Y + row_of(i) as f64 * CELL)
}

fn clock(g: &Sudoku) -> String {
    let s = g.elapsed.floor() as u32;
    format!("{}:{:02}", s / 60, s % 60)
}

/// After solving, a glow sweeps diagonally across the grid: 0..1 per cell.
fn wave(g: &Sudoku, i: usize) -> f64 {
    if !g.solved {
        return 0.0;
    }
    let t = (g.time - g.solved_at) * 2.2 - (row_of(i) + col_of(i)) as f64 * 0.08;
    if (0.0..1.0).contains(&t) { (t * std::f64::consts::PI).sin() } else { 0.0 }
}

/// How each cell relates to the selected one.
fn related(g: &Sudoku, i: usize) -> (bool, bool, bool) {
    let s = g.selected;
    let same_area = row_of(i) == row_of(s) || col_of(i) == col_of(s) || box_of(i) == box_of(s);
    let same_digit = g.values[s] != 0 && g.values[i] == g.values[s];
    (i == s, same_area, same_digit)
}

// ---------------------------------------------------------------- modern

const BG: &str = "#070a17";
const BOARD: &str = "#11162c";
const TEXT: &str = "#e8ecff";
const MUTED: &str = "#9aa3c7";
const ENTRY: &str = "#7cf2ff";
const BAD: &str = "#ff6b8b";

pub fn modern(g: &Sudoku, c: &Canvas) {
    c.color(BG);
    c.rect(0.0, 0.0, W, H);

    // Top bar.
    let small = "600 12px 'Space Grotesk', sans-serif";
    let big = "700 18px 'Space Grotesk', sans-serif";
    c.color(MUTED);
    c.text(&g.diff.name().to_uppercase(), GRID_X, BAR_H / 2.0, small, "left");
    c.text(&format!("HINTS {}", g.hints), W - GRID_X, BAR_H / 2.0, small, "right");
    c.color(TEXT);
    c.text(&clock(g), W / 2.0, BAR_H / 2.0, big, "center");
    if g.notes_mode {
        c.color("#8f7cff");
        c.round_rect(GRID_X + 66.0, BAR_H / 2.0 - 11.0, 92.0, 22.0, 11.0);
        c.color("#070a17");
        c.text("NOTES ON", GRID_X + 112.0, BAR_H / 2.0 + 1.0, "700 11px 'Space Grotesk', sans-serif", "center");
    }

    c.color(BOARD);
    c.round_rect(GRID_X - 4.0, GRID_Y - 4.0, 9.0 * CELL + 8.0, 9.0 * CELL + 8.0, 12.0);
    for i in 0..81 {
        let (x, y) = cell_xy(i);
        let (sel, area, digit) = related(g, i);
        let bg = if sel {
            "#7cf2ff40"
        } else if g.conflict(i) {
            "#ff6b8b26"
        } else if digit {
            "#8f7cff40"
        } else if area {
            "#ffffff0d"
        } else {
            ""
        };
        if !bg.is_empty() {
            c.color(bg);
            c.rect(x, y, CELL, CELL);
        }
        let w = wave(g, i);
        if w > 0.0 {
            c.alpha(w * 0.55);
            c.color("#7cf2ff");
            c.rect(x, y, CELL, CELL);
            c.alpha(1.0);
        }
        let v = g.values[i];
        if v != 0 {
            let (font, color) = if g.given[i] {
                ("700 26px 'Space Grotesk', sans-serif", TEXT)
            } else {
                ("500 26px 'Space Grotesk', sans-serif", if g.conflict(i) { BAD } else { ENTRY })
            };
            c.color(color);
            c.text(&v.to_string(), x + CELL / 2.0, y + CELL / 2.0 + 1.0, font, "center");
        } else if g.notes[i] != 0 {
            c.color(MUTED);
            for d in 1..=9u8 {
                if g.notes[i] & (1 << d) != 0 {
                    let k = (d - 1) as f64;
                    let (nx, ny) = (x + 9.0 + (k % 3.0) * 15.0, y + 9.0 + (k / 3.0).floor() * 15.0);
                    c.text(&d.to_string(), nx, ny, "500 11px 'Space Grotesk', sans-serif", "center");
                }
            }
        }
    }
    // Grid lines: thin between cells, thick between boxes.
    for k in 0..=9 {
        let (color, width) = if k % 3 == 0 { ("#e8ecff66", 2.0) } else { ("#ffffff1a", 1.0) };
        c.color(color);
        let o = k as f64 * CELL - width / 2.0;
        c.rect(GRID_X + o, GRID_Y, width, 9.0 * CELL);
        c.rect(GRID_X, GRID_Y + o, 9.0 * CELL, width);
    }

    // Number pad, with how many of each digit are still to place.
    for d in 1..=9u8 {
        let x = GRID_X + (d - 1) as f64 * CELL;
        let left = 9usize.saturating_sub(g.count(d));
        let done = left == 0;
        c.color(if g.notes_mode { "#8f7cff26" } else { "#171e3b" });
        c.round_rect(x + 3.0, PAD_Y, CELL - 6.0, PAD_H, 10.0);
        c.color(if done { "#ffffff33" } else if g.notes_mode { "#c4b8ff" } else { TEXT });
        let font = if g.notes_mode { "500 22px 'Space Grotesk', sans-serif" } else { "700 24px 'Space Grotesk', sans-serif" };
        c.text(&d.to_string(), x + CELL / 2.0, PAD_Y + 25.0, font, "center");
        c.color(if done { "#ffffff22" } else { MUTED });
        c.text(&left.to_string(), x + CELL / 2.0, PAD_Y + 47.0, "500 11px 'Space Grotesk', sans-serif", "center");
    }
}

// ---------------------------------------------------------------- retro

const LCD_BG: &str = "#c3cf9b";
const LCD_GHOST: &str = "#b7c48f";
const LCD_INK: &str = "#2c3a23";

pub fn retro(g: &Sudoku, c: &Canvas) {
    c.color(LCD_BG);
    c.rect(0.0, 0.0, W, H);
    let small = "9px 'Press Start 2P', monospace";
    c.color(LCD_INK);
    c.text(&g.diff.name().to_uppercase(), GRID_X, BAR_H / 2.0, small, "left");
    c.text(&format!("HINT {}", g.hints), W - GRID_X, BAR_H / 2.0, small, "right");
    c.text(&clock(g), W / 2.0, BAR_H / 2.0, "12px 'Press Start 2P', monospace", "center");
    if g.notes_mode && (g.time * 2.0).fract() < 0.8 {
        c.text("NOTES", GRID_X + 80.0, BAR_H / 2.0, small, "left");
    }

    let blink = (g.time * 3.0).fract() < 0.55;
    for i in 0..81 {
        let (x, y) = cell_xy(i);
        let (sel, area, digit) = related(g, i);
        let inverted = sel || wave(g, i) > 0.5;
        if inverted {
            c.color(LCD_INK);
            c.rect(x, y, CELL, CELL);
        } else if digit || area {
            c.color(LCD_GHOST);
            c.rect(x, y, CELL, CELL);
        }
        let ink = if inverted { LCD_BG } else { LCD_INK };
        let v = g.values[i];
        if v != 0 {
            if g.conflict(i) && !blink {
                continue;
            }
            c.color(ink);
            c.text(&v.to_string(), x + CELL / 2.0 + 1.0, y + CELL / 2.0 + 2.0, "18px 'Press Start 2P', monospace", "center");
            if !g.given[i] {
                // Your numbers get a little underline; printed clues don't.
                c.rect(x + 16.0, y + CELL - 9.0, CELL - 32.0, 2.0);
            }
        } else if g.notes[i] != 0 {
            c.color(ink);
            for d in 1..=9u8 {
                if g.notes[i] & (1 << d) != 0 {
                    let k = (d - 1) as f64;
                    let (nx, ny) = (x + 10.0 + (k % 3.0) * 14.0, y + 10.0 + (k / 3.0).floor() * 14.0);
                    c.text(&d.to_string(), nx, ny, "7px 'Press Start 2P', monospace", "center");
                }
            }
        }
    }
    c.color(LCD_INK);
    for k in 0..=9 {
        let width = if k % 3 == 0 { 3.0 } else { 1.0 };
        let o = (k as f64 * CELL - width / 2.0).round();
        c.rect(GRID_X + o, GRID_Y, width, 9.0 * CELL);
        c.rect(GRID_X, GRID_Y + o, 9.0 * CELL, width);
    }
    for d in 1..=9u8 {
        let x = GRID_X + (d - 1) as f64 * CELL;
        let left = 9usize.saturating_sub(g.count(d));
        c.color(LCD_INK);
        if g.notes_mode {
            c.frame(x + 3.0, PAD_Y, CELL - 6.0, PAD_H, 2.0);
        } else {
            c.frame(x + 3.0, PAD_Y, CELL - 6.0, PAD_H, 3.0);
        }
        if left > 0 {
            c.text(&d.to_string(), x + CELL / 2.0 + 1.0, PAD_Y + 24.0, "16px 'Press Start 2P', monospace", "center");
            c.text(&left.to_string(), x + CELL / 2.0 + 1.0, PAD_Y + 47.0, "7px 'Press Start 2P', monospace", "center");
        }
    }
}
