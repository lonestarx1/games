use std::f64::consts::TAU;
use web_sys::{CanvasGradient, CanvasRenderingContext2d};

/// Thin wrapper over the 2D canvas with the drawing helpers games share.
/// All coordinates are logical (CSS) pixels; device pixel ratio is handled here.
pub struct Canvas {
    ctx: CanvasRenderingContext2d,
    dpr: f64,
}

impl Canvas {
    pub fn new(ctx: CanvasRenderingContext2d, dpr: f64) -> Canvas {
        Canvas { ctx, dpr }
    }

    /// Raw context for anything the helpers don't cover.
    pub fn ctx(&self) -> &CanvasRenderingContext2d {
        &self.ctx
    }

    pub fn begin_frame(&self) {
        let _ = self.ctx.set_transform(self.dpr, 0.0, 0.0, self.dpr, 0.0, 0.0);
        self.ctx.set_global_alpha(1.0);
        self.no_glow();
    }

    pub fn color(&self, css: &str) {
        self.ctx.set_fill_style_str(css);
    }

    pub fn gradient(&self, g: &CanvasGradient) {
        self.ctx.set_fill_style_canvas_gradient(g);
    }

    pub fn alpha(&self, a: f64) {
        self.ctx.set_global_alpha(a);
    }

    pub fn glow(&self, blur: f64, css: &str) {
        self.ctx.set_shadow_blur(blur);
        self.ctx.set_shadow_color(css);
    }

    pub fn no_glow(&self) {
        self.ctx.set_shadow_blur(0.0);
        self.ctx.set_shadow_color("transparent");
    }

    pub fn rect(&self, x: f64, y: f64, w: f64, h: f64) {
        self.ctx.fill_rect(x, y, w, h);
    }

    pub fn round_rect(&self, x: f64, y: f64, w: f64, h: f64, r: f64) {
        let r = r.min(w / 2.0).min(h / 2.0);
        let c = &self.ctx;
        c.begin_path();
        c.move_to(x + r, y);
        let _ = c.arc_to(x + w, y, x + w, y + h, r);
        let _ = c.arc_to(x + w, y + h, x, y + h, r);
        let _ = c.arc_to(x, y + h, x, y, r);
        let _ = c.arc_to(x, y, x + w, y, r);
        c.close_path();
        c.fill();
    }

    /// Rectangle outline drawn with filled bars (uses the fill color).
    pub fn frame(&self, x: f64, y: f64, w: f64, h: f64, t: f64) {
        self.rect(x, y, w, t);
        self.rect(x, y + h - t, w, t);
        self.rect(x, y, t, h);
        self.rect(x + w - t, y, t, h);
    }

    pub fn circle(&self, x: f64, y: f64, r: f64) {
        self.ctx.begin_path();
        let _ = self.ctx.arc(x, y, r, 0.0, TAU);
        self.ctx.fill();
    }

    /// Straight line with round caps.
    pub fn line(&self, x0: f64, y0: f64, x1: f64, y1: f64, width: f64, css: &str) {
        let c = &self.ctx;
        c.set_stroke_style_str(css);
        c.set_line_width(width);
        c.set_line_cap("round");
        c.begin_path();
        c.move_to(x0, y0);
        c.line_to(x1, y1);
        c.stroke();
    }

    /// Filled polygon (uses the fill color).
    pub fn polygon(&self, points: &[(f64, f64)]) {
        let Some(&(x0, y0)) = points.first() else { return };
        let c = &self.ctx;
        c.begin_path();
        c.move_to(x0, y0);
        for &(x, y) in &points[1..] {
            c.line_to(x, y);
        }
        c.close_path();
        c.fill();
    }

    /// Filled ellipse, rotated by `rot` radians (uses the fill color).
    pub fn ellipse(&self, x: f64, y: f64, rx: f64, ry: f64, rot: f64) {
        self.ctx.begin_path();
        let _ = self.ctx.ellipse(x, y, rx, ry, rot, 0.0, TAU);
        self.ctx.fill();
    }

    /// Draws text. `font` is a CSS font string, e.g. "16px 'Press Start 2P'".
    /// `align` is "left", "center" or "right"; `y` is the vertical middle.
    pub fn text(&self, s: &str, x: f64, y: f64, font: &str, align: &str) {
        self.ctx.set_font(font);
        self.ctx.set_text_align(align);
        self.ctx.set_text_baseline("middle");
        let _ = self.ctx.fill_text(s, x, y);
    }

    pub fn linear_gradient(
        &self,
        x0: f64,
        y0: f64,
        x1: f64,
        y1: f64,
        stops: &[(f32, &str)],
    ) -> CanvasGradient {
        let g = self.ctx.create_linear_gradient(x0, y0, x1, y1);
        for (offset, css) in stops {
            let _ = g.add_color_stop(*offset, css);
        }
        g
    }
}

/// `#rrggbb` color math for gradients along a snake, fading particles, etc.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Rgb(pub u8, pub u8, pub u8);

impl Rgb {
    pub fn lerp(self, other: Rgb, t: f64) -> Rgb {
        let t = t.clamp(0.0, 1.0);
        let mix = |a: u8, b: u8| (a as f64 + (b as f64 - a as f64) * t).round() as u8;
        Rgb(mix(self.0, other.0), mix(self.1, other.1), mix(self.2, other.2))
    }

    pub fn css(self) -> String {
        format!("rgb({},{},{})", self.0, self.1, self.2)
    }
}

/// Snap to a coarse grid for the chunky retro look.
pub fn snap(v: f64, px: f64) -> f64 {
    (v / px).round() * px
}
