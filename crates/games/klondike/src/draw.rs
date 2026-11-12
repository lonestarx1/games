use crate::logic::{slot, Card, Klondike, Pile, CH, CW, FOOTER, H, SUITS, W};
use engine::Canvas;

/// Every card on the table in drawing order, with where it is drawn.
/// Moving cards (flying or dragged) come last so they pass over the rest.
fn layout(g: &Klondike) -> Vec<(Card, f64, f64)> {
    let mut resting = Vec::new();
    let mut moving = Vec::new();
    let dragged = |p: Pile, i: usize| g.drag.as_ref().is_some_and(|d| d.from == p && i >= d.idx);
    let piles = [Pile::Stock, Pile::Waste]
        .into_iter()
        .chain((0..4).map(Pile::Found))
        .chain((0..7).map(Pile::Tab));
    for p in piles {
        let cards = g.pile(p);
        // Deep stock, waste and foundation piles only need their top few cards drawn.
        let from = match p {
            Pile::Tab(_) => 0,
            _ => cards.len().saturating_sub(4),
        };
        for (i, &c) in cards.iter().enumerate().skip(from) {
            if dragged(p, i) {
                continue;
            }
            let (x, y) = g.draw_pos(p, i);
            if g.in_flight(p, i) {
                moving.push((c, x, y));
            } else {
                resting.push((c, x, y));
            }
        }
    }
    if let Some(d) = &g.drag {
        for (k, &c) in g.pile(d.from)[d.idx..].iter().enumerate() {
            let (x, y) = g.drag_pos(k).expect("dragging");
            moving.push((c, x, y));
        }
    }
    resting.extend(moving);
    resting
}

/// Rectangles to outline for the current hint: the card (or stock) and its target.
fn hint_boxes(g: &Klondike) -> Vec<(f64, f64, f64)> {
    let Some(h) = g.hint else { return vec![] };
    let mut boxes = Vec::new();
    match h.to {
        None => {
            let (x, y) = slot(Pile::Stock);
            boxes.push((x, y, CH));
        }
        Some(to) => {
            let (x, y) = g.card_pos(h.from, h.idx);
            let bottom = g.card_pos(h.from, g.pile(h.from).len() - 1).1 + CH;
            boxes.push((x, y, bottom - y));
            let len = g.pile(to).len();
            let (tx, ty) = if len == 0 { slot(to) } else { g.card_pos(to, len - 1) };
            boxes.push((tx, ty, CH));
        }
    }
    boxes
}

fn footer_text(g: &Klondike) -> (String, String) {
    let s = g.elapsed.floor() as u32;
    let left = format!("{} · {}:{:02}", if g.draw_three { "Draw 3" } else { "Draw 1" }, s / 60, s % 60);
    let right = if g.no_moves_ms > 0.0 {
        "No moves left. Try Undo or a new game.".to_string()
    } else if g.can_auto_finish() {
        "Finishing…".to_string()
    } else {
        String::new()
    };
    (left, right)
}

// ---------------------------------------------------------------- modern

const RED: &str = "#d6334a";
const BLACK: &str = "#1f2433";
const FACE: &str = "#fbfaf6";

fn modern_back(c: &Canvas, x: f64, y: f64) {
    let grad = c.linear_gradient(x, y, x + CW, y + CH, &[(0.0, "#4c5bd4"), (1.0, "#7a4fd0")]);
    c.gradient(&grad);
    c.round_rect(x, y, CW, CH, 8.0);
    let ctx = c.ctx();
    ctx.set_stroke_style_str("#ffffff55");
    ctx.set_line_width(1.5);
    ctx.stroke_rect(x + 6.0, y + 6.0, CW - 12.0, CH - 12.0);
    // Diamond lattice.
    ctx.save();
    ctx.begin_path();
    ctx.rect(x + 6.0, y + 6.0, CW - 12.0, CH - 12.0);
    ctx.clip();
    ctx.set_stroke_style_str("#ffffff26");
    ctx.set_line_width(1.0);
    ctx.begin_path();
    let mut k = -CH;
    while k < CW + CH {
        ctx.move_to(x + k, y);
        ctx.line_to(x + k + CH, y + CH);
        ctx.move_to(x + k + CH, y);
        ctx.line_to(x + k, y + CH);
        k += 14.0;
    }
    ctx.stroke();
    ctx.restore();
}

fn modern_face(c: &Canvas, card: Card, x: f64, y: f64) {
    c.color(FACE);
    c.round_rect(x, y, CW, CH, 8.0);
    let ink = if card.red() { RED } else { BLACK };
    let suit = SUITS[card.suit as usize];
    c.color(ink);
    let corner = "700 17px 'Space Grotesk', sans-serif";
    c.text(card.rank_label(), x + 8.0, y + 14.0, corner, "left");
    c.text(suit, x + 8.0, y + 32.0, "16px sans-serif", "left");
    c.text(suit, x + CW - 8.0, y + CH - 14.0, "16px sans-serif", "right");
    if card.rank > 10 {
        // Court cards: a tinted panel with a big letter.
        c.color(if card.red() { "#d6334a1f" } else { "#1f24331a" });
        c.round_rect(x + 22.0, y + 22.0, CW - 34.0, CH - 44.0, 6.0);
        c.color(ink);
        c.text(card.rank_label(), x + CW / 2.0 + 5.0, y + CH / 2.0, "700 40px 'Space Grotesk', sans-serif", "center");
    } else {
        c.text(suit, x + CW / 2.0 + 4.0, y + CH / 2.0 + 4.0, "44px sans-serif", "center");
    }
}

fn modern_slot(c: &Canvas, x: f64, y: f64, label: &str) {
    c.color("#ffffff10");
    c.round_rect(x, y, CW, CH, 8.0);
    c.ctx().set_stroke_style_str("#ffffff2a");
    c.ctx().set_line_width(1.5);
    c.ctx().stroke_rect(x + 0.75, y + 0.75, CW - 1.5, CH - 1.5);
    c.color("#ffffff40");
    c.text(label, x + CW / 2.0, y + CH / 2.0, "600 26px 'Space Grotesk', sans-serif", "center");
}

pub fn modern(g: &Klondike, c: &Canvas) {
    let felt = c.linear_gradient(0.0, 0.0, 0.0, H, &[(0.0, "#0f3a3c"), (1.0, "#08201f")]);
    c.gradient(&felt);
    c.rect(0.0, 0.0, W, H);

    let (sx, sy) = slot(Pile::Stock);
    modern_slot(c, sx, sy, if g.waste.is_empty() { "" } else { "↻" });
    for f in 0..4 {
        let (x, y) = slot(Pile::Found(f));
        modern_slot(c, x, y, "A");
    }
    for t in 0..7 {
        let (x, y) = slot(Pile::Tab(t));
        modern_slot(c, x, y, "K");
    }

    let ctx = c.ctx();
    for (card, x, y) in layout(g) {
        ctx.set_shadow_color("#0000004d");
        ctx.set_shadow_blur(6.0);
        ctx.set_shadow_offset_y(2.0);
        if card.up { modern_face(c, card, x, y) } else { modern_back(c, x, y) }
        c.no_glow();
        ctx.set_shadow_offset_y(0.0);
    }

    let pulse = 0.6 + 0.4 * (g.time * 6.0).sin().abs();
    for (x, y, h) in hint_boxes(g) {
        c.glow(14.0, "#7cf2ff");
        ctx.set_stroke_style_str(&format!("rgba(124,242,255,{pulse:.2})"));
        ctx.set_line_width(3.0);
        ctx.stroke_rect(x - 2.0, y - 2.0, CW + 4.0, h + 4.0);
        c.no_glow();
    }

    let (left, right) = footer_text(g);
    let font = "500 15px 'Space Grotesk', sans-serif";
    c.color("#cfe9e6aa");
    c.text(&left, 14.0, H - FOOTER / 2.0, font, "left");
    c.text(&right, W - 14.0, H - FOOTER / 2.0, font, "right");
}

// ---------------------------------------------------------------- retro

const LCD_BG: &str = "#c3cf9b";
const LCD_GHOST: &str = "#b7c48f";
const LCD_INK: &str = "#2c3a23";
/// Monochrome screen: red suits are drawn hollow, black suits solid.
const RETRO_SUITS: [&str; 4] = ["♠", "♡", "♢", "♣"];

fn retro_back(c: &Canvas, x: f64, y: f64) {
    c.color(LCD_INK);
    c.rect(x, y, CW, CH);
    c.color(LCD_BG);
    c.frame(x + 4.0, y + 4.0, CW - 8.0, CH - 8.0, 2.0);
    let mut yy = y + 10.0;
    let mut row = 0;
    while yy < y + CH - 12.0 {
        let mut xx = x + 10.0 + if row % 2 == 0 { 0.0 } else { 6.0 };
        while xx < x + CW - 12.0 {
            c.rect(xx, yy, 4.0, 4.0);
            xx += 12.0;
        }
        yy += 8.0;
        row += 1;
    }
}

fn retro_face(c: &Canvas, card: Card, x: f64, y: f64) {
    c.color(LCD_BG);
    c.rect(x, y, CW, CH);
    c.color(LCD_INK);
    c.frame(x, y, CW, CH, 3.0);
    let suit = RETRO_SUITS[card.suit as usize];
    let font = "11px 'Press Start 2P', monospace";
    let suit_small = "18px 'Press Start 2P', 'Segoe UI Symbol', sans-serif";
    c.text(card.rank_label(), x + 8.0, y + 15.0, font, "left");
    c.text(suit, x + 7.0, y + 33.0, suit_small, "left");
    if card.rank > 10 {
        c.frame(x + 20.0, y + 24.0, CW - 34.0, CH - 48.0, 2.0);
        c.text(card.rank_label(), x + CW / 2.0 + 3.0, y + CH / 2.0 + 2.0, "22px 'Press Start 2P', monospace", "center");
    } else {
        c.text(suit, x + CW / 2.0 + 4.0, y + CH / 2.0 + 6.0, "42px 'Press Start 2P', 'Segoe UI Symbol', sans-serif", "center");
    }
}

fn retro_slot(c: &Canvas, x: f64, y: f64, label: &str) {
    c.color(LCD_GHOST);
    c.rect(x, y, CW, CH);
    c.color(LCD_INK);
    let mut k = 0.0;
    while k < CW {
        c.rect(x + k, y, 4.0, 2.0);
        c.rect(x + k, y + CH - 2.0, 4.0, 2.0);
        k += 8.0;
    }
    let mut k = 0.0;
    while k < CH {
        c.rect(x, y + k, 2.0, 4.0);
        c.rect(x + CW - 2.0, y + k, 2.0, 4.0);
        k += 8.0;
    }
    c.text(label, x + CW / 2.0 + 2.0, y + CH / 2.0 + 2.0, "16px 'Press Start 2P', monospace", "center");
}

pub fn retro(g: &Klondike, c: &Canvas) {
    c.color(LCD_BG);
    c.rect(0.0, 0.0, W, H);
    let (sx, sy) = slot(Pile::Stock);
    retro_slot(c, sx, sy, if g.waste.is_empty() { "" } else { "O" });
    for f in 0..4 {
        let (x, y) = slot(Pile::Found(f));
        retro_slot(c, x, y, "A");
    }
    for t in 0..7 {
        let (x, y) = slot(Pile::Tab(t));
        retro_slot(c, x, y, "K");
    }
    for (card, x, y) in layout(g) {
        if card.up { retro_face(c, card, x, y) } else { retro_back(c, x, y) }
    }
    if (g.time * 3.0).fract() < 0.6 {
        c.color(LCD_INK);
        for (x, y, h) in hint_boxes(g) {
            c.frame(x - 5.0, y - 5.0, CW + 10.0, h + 10.0, 3.0);
        }
    }
    let (left, right) = footer_text(g);
    let font = "9px 'Press Start 2P', monospace";
    c.color(LCD_INK);
    c.text(&left, 14.0, H - FOOTER / 2.0, font, "left");
    c.text(&right, W - 14.0, H - FOOTER / 2.0, font, "right");
}
