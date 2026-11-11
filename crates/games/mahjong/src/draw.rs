use crate::layout::{DEPTH, H, W};
use crate::logic::Mahjong;
use crate::tiles::Face;
use engine::{Canvas, GameState};

const CJK: &str = "'PingFang SC','Hiragino Sans GB','Noto Sans CJK SC','Microsoft YaHei',sans-serif";

/// Colors used to draw tile faces. Retro maps everything to one ink.
struct Ink {
    red: &'static str,
    green: &'static str,
    blue: &'static str,
    dark: &'static str,
    face: &'static str,
}

const MODERN_INK: Ink = Ink {
    red: "#d33a3a",
    green: "#1f8a4c",
    blue: "#2457b8",
    dark: "#1d2430",
    face: "#fbf7ec",
};

// ---------------------------------------------------------------- modern

pub fn modern(m: &Mahjong, c: &Canvas) {
    let bg = c.linear_gradient(0.0, 0.0, 0.0, H, &[(0.0, "#123236"), (1.0, "#081619")]);
    c.gradient(&bg);
    c.rect(0.0, 0.0, W, H);
    status_bar(m, c, "#9fc4c0", "500 14px 'Space Grotesk', sans-serif");

    for i in (0..m.board.len()).filter(|&i| m.alive[i]) {
        let (x, y, w, h) = m.board.tile_rect(i);
        // Raised tiles cast a soft shadow so layers are easy to tell apart.
        if m.board.slots[i].z > 0 {
            c.alpha(0.35);
            c.color("#000000");
            c.round_rect(x + DEPTH * 2.0, y + DEPTH * 2.0, w, h, 7.0);
            c.alpha(1.0);
        }
        // Thickness: two shades on the right/bottom.
        c.color("#8f7b52");
        c.round_rect(x + DEPTH, y + DEPTH, w - 1.0, h - 1.0, 6.0);
        c.color("#d9c9a0");
        c.round_rect(x + DEPTH * 0.5, y + DEPTH * 0.5, w - 1.0, h - 1.0, 6.0);

        let selected = m.selected == Some(i);
        let hinted = m.hint.is_some_and(|(a, b)| a == i || b == i);
        let face = if selected {
            "#c9f1ff"
        } else if hinted {
            "#fff1a8"
        } else if m.flash == Some(i) {
            "#ffc9c9"
        } else if m.hover == Some(i) {
            "#ffffff"
        } else {
            MODERN_INK.face
        };
        c.color(face);
        c.round_rect(x, y, w - 1.0, h - 1.0, 6.0);
        draw_face(c, m.faces[i], x, y, w - 1.0, h - 1.0, &MODERN_INK);

        if selected {
            c.color("#1aa6d9");
            c.frame(x, y, w - 1.0, h - 1.0, 2.0);
        } else if !m.free[i] {
            // Dim tiles you can't pick yet.
            c.alpha(0.16);
            c.color("#000000");
            c.round_rect(x, y, w - 1.0, h - 1.0, 6.0);
            c.alpha(1.0);
        }
    }

    if stuck(m) {
        banner(m, c, "#081619d0", "#ffffff", "700 28px 'Space Grotesk', sans-serif", "15px 'Space Grotesk', sans-serif");
    }
}

// ---------------------------------------------------------------- retro

const LCD_BG: &str = "#c3cf9b";
const LCD_FACE: &str = "#d4deae";
const LCD_INK: &str = "#2c3a23";
const RETRO_INK: Ink = Ink {
    red: LCD_INK,
    green: LCD_INK,
    blue: LCD_INK,
    dark: LCD_INK,
    face: LCD_FACE,
};
const RETRO_INVERTED: Ink = Ink {
    red: LCD_FACE,
    green: LCD_FACE,
    blue: LCD_FACE,
    dark: LCD_FACE,
    face: LCD_INK,
};

pub fn retro(m: &Mahjong, c: &Canvas) {
    c.color(LCD_BG);
    c.rect(0.0, 0.0, W, H);
    c.color(LCD_INK);
    status_bar(m, c, LCD_INK, "10px 'Press Start 2P', monospace");

    let blink = (m.time * 4.0).fract() < 0.5;
    for i in (0..m.board.len()).filter(|&i| m.alive[i]) {
        let (x, y, w, h) = m.board.tile_rect(i);
        let (w, h) = (w - 2.0, h - 2.0);
        c.color(LCD_INK);
        c.rect(x + DEPTH, y + DEPTH, w, h);
        c.rect(x, y, w, h);

        let hinted = m.hint.is_some_and(|(a, b)| a == i || b == i) && blink;
        let invert = m.selected == Some(i) || hinted || m.flash == Some(i);
        let ink = if invert { &RETRO_INVERTED } else { &RETRO_INK };
        c.color(ink.face);
        c.rect(x + 2.0, y + 2.0, w - 4.0, h - 4.0);
        draw_face(c, m.faces[i], x, y, w, h, ink);
        if m.hover == Some(i) && !invert {
            c.color(LCD_INK);
            c.frame(x + 3.0, y + 3.0, w - 6.0, h - 6.0, 1.0);
        }
    }

    if stuck(m) {
        banner(m, c, "#c3cf9be6", LCD_INK, "16px 'Press Start 2P', monospace", "9px 'Press Start 2P', monospace");
    }
}

// ---------------------------------------------------------------- shared

fn stuck(m: &Mahjong) -> bool {
    m.state == GameState::Playing && m.moves == 0 && m.tiles_left() > 0
}

fn status_bar(m: &Mahjong, c: &Canvas, color: &str, font: &str) {
    let secs = m.elapsed as u32;
    c.color(color);
    c.text(&format!("Time {}:{:02}", secs / 60, secs % 60), 18.0, 24.0, font, "left");
    c.text(&format!("Tiles {}", m.tiles_left()), W / 2.0, 24.0, font, "center");
    c.text(&format!("Moves {}", m.moves), W - 18.0, 24.0, font, "right");
}

fn banner(_m: &Mahjong, c: &Canvas, panel: &str, text: &str, big: &str, small: &str) {
    c.color(panel);
    c.rect(0.0, H / 2.0 - 50.0, W, 100.0);
    c.color(text);
    c.text("No moves left", W / 2.0, H / 2.0 - 12.0, big, "center");
    c.text("Press R or click Shuffle", W / 2.0, H / 2.0 + 22.0, small, "center");
}

/// Draws the symbol for `face` inside the tile rect.
fn draw_face(c: &Canvas, face: Face, x: f64, y: f64, w: f64, h: f64, ink: &Ink) {
    // Inner drawing area.
    let (ix, iy, iw, ih) = (x + 5.0, y + 5.0, w - 10.0, h - 10.0);
    let at = |px: f64, py: f64| (ix + px * iw, iy + py * ih);
    match face {
        Face::Dots(n) => {
            let r = match n {
                1 => 0.42,
                2..=4 => 0.24,
                5 | 6 => 0.2,
                _ => 0.16,
            } * iw;
            for (k, &(px, py)) in dot_layout(n).iter().enumerate() {
                let (cx, cy) = at(px, py);
                c.color(dot_color(n, k, ink));
                c.circle(cx, cy, r);
                c.color(ink.face);
                c.circle(cx, cy, r * 0.55);
                c.color(dot_color(n, k, ink));
                c.circle(cx, cy, r * 0.22);
            }
        }
        Face::Bamboo(n) => {
            if n == 1 {
                // A single tall stalk with leaves.
                let (cx, cy) = at(0.5, 0.5);
                c.color(ink.green);
                c.round_rect(cx - 3.0, cy - ih * 0.4, 6.0, ih * 0.8, 3.0);
                c.circle(cx - 7.0, cy - 6.0, 4.0);
                c.circle(cx + 7.0, cy + 4.0, 4.0);
                c.color(ink.red);
                c.circle(cx, cy - ih * 0.4, 3.5);
                return;
            }
            let (sticks, stick_h) = bamboo_layout(n);
            for &(px, py, red) in sticks {
                let (cx, cy) = at(px, py);
                let sh = stick_h * ih;
                c.color(if red { ink.red } else { ink.green });
                c.round_rect(cx - 2.5, cy - sh / 2.0, 5.0, sh, 2.5);
                c.color(ink.face);
                c.rect(cx - 2.5, cy - 0.5, 5.0, 1.0);
            }
        }
        Face::Chars(n) => {
            const NUMS: [&str; 9] = ["一", "二", "三", "四", "五", "六", "七", "八", "九"];
            c.color(ink.dark);
            c.text(NUMS[n as usize - 1], x + w / 2.0, y + h * 0.3, &format!("700 {}px {CJK}", (h * 0.32) as i32), "center");
            c.color(ink.red);
            c.text("萬", x + w / 2.0, y + h * 0.7, &format!("700 {}px {CJK}", (h * 0.34) as i32), "center");
        }
        Face::Wind(n) => {
            const WINDS: [&str; 4] = ["東", "南", "西", "北"];
            c.color(ink.dark);
            c.text(WINDS[n as usize], x + w / 2.0, y + h / 2.0, &format!("700 {}px {CJK}", (h * 0.5) as i32), "center");
        }
        Face::Dragon(2) => {
            c.color(ink.blue);
            c.frame(ix + 3.0, iy + 4.0, iw - 6.0, ih - 8.0, 3.0);
        }
        Face::Dragon(n) => {
            let (glyph, color) = if n == 0 { ("中", ink.red) } else { ("發", ink.green) };
            c.color(color);
            c.text(glyph, x + w / 2.0, y + h / 2.0, &format!("700 {}px {CJK}", (h * 0.52) as i32), "center");
        }
        Face::Flower(n) | Face::Season(n) => {
            let (glyph, color) = if matches!(face, Face::Flower(_)) {
                (["梅", "蘭", "菊", "竹"][n as usize], [ink.red, ink.blue, ink.red, ink.green][n as usize])
            } else {
                (["春", "夏", "秋", "冬"][n as usize], [ink.green, ink.red, ink.red, ink.blue][n as usize])
            };
            c.color(color);
            c.text(glyph, x + w / 2.0, y + h / 2.0, &format!("700 {}px {CJK}", (h * 0.46) as i32), "center");
        }
    }
}

fn dot_layout(n: u8) -> &'static [(f64, f64)] {
    match n {
        1 => &[(0.5, 0.5)],
        2 => &[(0.5, 0.25), (0.5, 0.75)],
        3 => &[(0.25, 0.2), (0.5, 0.5), (0.75, 0.8)],
        4 => &[(0.27, 0.27), (0.73, 0.27), (0.27, 0.73), (0.73, 0.73)],
        5 => &[(0.25, 0.22), (0.75, 0.22), (0.5, 0.5), (0.25, 0.78), (0.75, 0.78)],
        6 => &[(0.28, 0.18), (0.72, 0.18), (0.28, 0.5), (0.72, 0.5), (0.28, 0.82), (0.72, 0.82)],
        7 => &[(0.2, 0.12), (0.5, 0.24), (0.8, 0.36), (0.3, 0.62), (0.7, 0.62), (0.3, 0.87), (0.7, 0.87)],
        8 => &[(0.3, 0.12), (0.7, 0.12), (0.3, 0.37), (0.7, 0.37), (0.3, 0.63), (0.7, 0.63), (0.3, 0.88), (0.7, 0.88)],
        _ => &[(0.2, 0.17), (0.5, 0.17), (0.8, 0.17), (0.2, 0.5), (0.5, 0.5), (0.8, 0.5), (0.2, 0.83), (0.5, 0.83), (0.8, 0.83)],
    }
}

fn dot_color<'a>(n: u8, k: usize, ink: &'a Ink) -> &'a str {
    match (n, k) {
        (1, _) => ink.red,
        (2, 0) | (4, 1) | (4, 2) => ink.green,
        (3, 1) | (5, 2) => ink.red,
        (3, 2) => ink.green,
        (6, 0) | (6, 1) | (7, 0..=2) => ink.green,
        (6, _) | (7, _) => ink.red,
        (9, 3..=5) => ink.red,
        (9, 6..=8) => ink.green,
        _ => ink.blue,
    }
}

/// Stick positions (x, y, is_red) and stick height as a fraction of the face.
fn bamboo_layout(n: u8) -> (&'static [(f64, f64, bool)], f64) {
    match n {
        2 => (&[(0.5, 0.27, false), (0.5, 0.73, false)], 0.4),
        3 => (&[(0.5, 0.27, false), (0.28, 0.73, false), (0.72, 0.73, false)], 0.4),
        4 => (&[(0.3, 0.27, false), (0.7, 0.27, false), (0.3, 0.73, false), (0.7, 0.73, false)], 0.4),
        5 => (&[(0.22, 0.27, false), (0.78, 0.27, false), (0.5, 0.5, true), (0.22, 0.73, false), (0.78, 0.73, false)], 0.4),
        6 => (&[(0.2, 0.27, false), (0.5, 0.27, false), (0.8, 0.27, false), (0.2, 0.73, false), (0.5, 0.73, false), (0.8, 0.73, false)], 0.4),
        7 => (&[(0.5, 0.17, true), (0.2, 0.5, false), (0.5, 0.5, false), (0.8, 0.5, false), (0.2, 0.83, false), (0.5, 0.83, false), (0.8, 0.83, false)], 0.28),
        8 => (&[(0.14, 0.27, false), (0.38, 0.27, false), (0.62, 0.27, false), (0.86, 0.27, false), (0.14, 0.73, false), (0.38, 0.73, false), (0.62, 0.73, false), (0.86, 0.73, false)], 0.4),
        _ => (&[(0.2, 0.17, false), (0.5, 0.17, true), (0.8, 0.17, false), (0.2, 0.5, false), (0.5, 0.5, true), (0.8, 0.5, false), (0.2, 0.83, false), (0.5, 0.83, true), (0.8, 0.83, false)], 0.28),
    }
}
