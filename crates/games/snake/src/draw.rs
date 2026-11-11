use crate::logic::{Cell, Dir, Snake, CELL, COLS, ROWS};
use engine::render::Rgb;
use engine::{Canvas, GameState};

const W: f64 = COLS as f64 * CELL;
const H: f64 = ROWS as f64 * CELL;

// Monochrome LCD palette.
const LCD_BG: &str = "#c3cf9b";
const LCD_GHOST: &str = "#b7c48f";
const LCD_INK: &str = "#2c3a23";

pub fn retro(s: &Snake, c: &Canvas) {
    c.color(LCD_BG);
    c.rect(0.0, 0.0, W, H);
    // Faint "unlit" LCD cells.
    c.color(LCD_GHOST);
    for y in 0..ROWS {
        for x in 0..COLS {
            c.rect(x as f64 * CELL + 2.0, y as f64 * CELL + 2.0, CELL - 4.0, CELL - 4.0);
        }
    }

    c.color(LCD_INK);
    // Food: a small pixel diamond.
    let (fx, fy) = origin(s.food);
    for (dx, dy) in [(8.0, 3.0), (3.0, 8.0), (13.0, 8.0), (8.0, 13.0)] {
        c.rect(fx + dx, fy + dy, 4.0, 4.0);
    }

    // Blink the snake on game over.
    if s.state == GameState::Over && (s.time * 3.0).fract() < 0.5 {
        return;
    }
    for (i, &cell) in s.body.iter().enumerate() {
        let (x, y) = origin(cell);
        c.color(LCD_INK);
        c.rect(x + 2.0, y + 2.0, CELL - 4.0, CELL - 4.0);
        if i == 0 {
            // Eye pixel facing forward.
            let (dx, dy) = s.dir.delta();
            c.color(LCD_BG);
            c.rect(x + 8.0 + dx as f64 * 4.0, y + 8.0 + dy as f64 * 4.0, 4.0, 4.0);
        } else if i % 2 == 0 {
            c.color(LCD_GHOST);
            c.rect(x + 8.0, y + 8.0, 4.0, 4.0);
        }
    }
}

const BG: &str = "#0b1020";
const GRID: &str = "#18203d";
const HEAD: Rgb = Rgb(0x8a, 0xff, 0xc1);
const TAIL: Rgb = Rgb(0x1f, 0x8f, 0xa3);
const FOOD: &str = "#ff4f7b";
const DEAD: &str = "#ff4f7b";

pub fn modern(s: &Snake, c: &Canvas) {
    c.color(BG);
    c.rect(0.0, 0.0, W, H);
    c.color(GRID);
    for y in 0..ROWS {
        for x in 0..COLS {
            c.circle(x as f64 * CELL + CELL / 2.0, y as f64 * CELL + CELL / 2.0, 1.2);
        }
    }

    // Food pulses and glows.
    let (fx, fy) = center(s.food);
    let pulse = 6.0 + (s.time * 6.0).sin() * 1.2;
    c.glow(18.0, FOOD);
    c.color(FOOD);
    c.circle(fx, fy, pulse);
    c.no_glow();

    let n = s.body.len().max(2) as f64;
    // Draw tail first so the head ends on top.
    for (i, &cell) in s.body.iter().enumerate().rev() {
        let col = HEAD.lerp(TAIL, i as f64 / (n - 1.0)).css();
        c.color(&col);
        let (x, y) = origin(cell);
        // Bridge to the next segment so the body looks continuous.
        if let Some(&next) = s.body.get(i + 1) {
            let (ax, ay) = center(cell);
            let (bx, by) = center(next);
            if (ax - bx).abs() + (ay - by).abs() <= CELL {
                let (l, t) = (ax.min(bx), ay.min(by));
                c.rect(l - 6.0, t - 6.0, (ax - bx).abs() + 12.0, (ay - by).abs() + 12.0);
            }
        }
        if i == 0 {
            let head_col = if s.state == GameState::Over { DEAD } else { col.as_str() };
            c.glow(14.0, head_col);
            c.color(head_col);
            c.round_rect(x + 1.0, y + 1.0, CELL - 2.0, CELL - 2.0, 7.0);
            c.no_glow();
            eyes(s.dir, cell, c);
        } else {
            c.round_rect(x + 2.0, y + 2.0, CELL - 4.0, CELL - 4.0, 5.0);
        }
    }
}

fn eyes(dir: Dir, head: Cell, c: &Canvas) {
    let (cx, cy) = center(head);
    let (dx, dy) = dir.delta();
    let (dx, dy) = (dx as f64, dy as f64);
    // Perpendicular offset for the two eyes.
    let (px, py) = (-dy, dx);
    c.color(BG);
    for side in [-1.0, 1.0] {
        c.circle(cx + dx * 3.0 + px * 4.5 * side, cy + dy * 3.0 + py * 4.5 * side, 2.2);
    }
}

fn origin(cell: Cell) -> (f64, f64) {
    (cell.x as f64 * CELL, cell.y as f64 * CELL)
}

fn center(cell: Cell) -> (f64, f64) {
    (cell.x as f64 * CELL + CELL / 2.0, cell.y as f64 * CELL + CELL / 2.0)
}
