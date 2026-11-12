//! Shared engine for every game on the site.
//!
//! A game implements [`Game`] (pure logic + a `render` method) and calls
//! [`export_game!`] once. That generates a JS class named `Game` with the same
//! API for every game, so the website can load any game the same way.

pub mod input;
pub mod render;
pub mod rng;

pub use input::Key;
pub use render::Canvas;
pub use rng::Rng;

// Re-exported so `export_game!` works without games depending on these directly.
pub use web_sys;

/// Visual style. Every game must draw both.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Theme {
    Retro,
    Modern,
}

impl Theme {
    pub fn from_name(name: &str) -> Theme {
        if name.eq_ignore_ascii_case("retro") {
            Theme::Retro
        } else {
            Theme::Modern
        }
    }
}

/// Lifecycle shared by all games. The numeric values are part of the JS API.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum GameState {
    Ready = 0,
    Playing = 1,
    Paused = 2,
    Over = 3,
}

/// What every game implements. Keep logic free of browser calls so it can be
/// unit tested natively with `cargo test`; only `render` touches the canvas.
pub trait Game {
    /// Logical canvas size in CSS pixels.
    const WIDTH: f64;
    const HEIGHT: f64;

    fn new(seed: u64) -> Self;
    /// Returns true when the game uses this key (the page then stops the
    /// browser's default action, such as scrolling).
    fn key(&mut self, key: Key, pressed: bool) -> bool;
    fn update(&mut self, dt_ms: f64);
    fn render(&self, canvas: &Canvas, theme: Theme);
    fn score(&self) -> u32;
    fn state(&self) -> GameState;
    /// Called when the tab loses focus.
    fn pause(&mut self);

    /// Mouse/touch press at logical canvas coordinates. Return true if used.
    fn pointer_down(&mut self, _x: f64, _y: f64) -> bool {
        false
    }
    /// Secondary press (right-click, or long-press on touch when the game's
    /// catalogue entry sets `longPress`). Return true if used.
    fn pointer_alt(&mut self, _x: f64, _y: f64) -> bool {
        false
    }
    /// Pointer moved; (-1, -1) means it left the canvas.
    fn pointer_move(&mut self, _x: f64, _y: f64) {}
    /// Custom game-over title (e.g. "Board cleared!"). None uses the default.
    fn end_message(&self) -> Option<String> {
        None
    }
}

/// Exports a [`Game`] implementation to JavaScript as class `Game`.
#[macro_export]
macro_rules! export_game {
    ($ty:ty) => {
        mod __wasm_export {
            use super::*;
            use ::wasm_bindgen::prelude::*;
            use $crate::Game as _;

            #[wasm_bindgen(js_name = Game)]
            pub struct WasmGame {
                game: $ty,
                canvas: $crate::Canvas,
                theme: $crate::Theme,
            }

            #[wasm_bindgen(js_class = Game)]
            impl WasmGame {
                #[wasm_bindgen(constructor)]
                pub fn new(
                    ctx: $crate::web_sys::CanvasRenderingContext2d,
                    dpr: f64,
                    seed: f64,
                ) -> WasmGame {
                    WasmGame {
                        game: <$ty as $crate::Game>::new(seed as u64),
                        canvas: $crate::Canvas::new(ctx, dpr),
                        theme: $crate::Theme::Modern,
                    }
                }
                pub fn width(&self) -> f64 {
                    <$ty as $crate::Game>::WIDTH
                }
                pub fn height(&self) -> f64 {
                    <$ty as $crate::Game>::HEIGHT
                }
                pub fn set_theme(&mut self, name: &str) {
                    self.theme = $crate::Theme::from_name(name);
                }
                pub fn key_down(&mut self, code: &str) -> bool {
                    self.game.key($crate::Key::from_code(code), true)
                }
                pub fn key_up(&mut self, code: &str) -> bool {
                    self.game.key($crate::Key::from_code(code), false)
                }
                pub fn update(&mut self, dt_ms: f64) {
                    self.game.update(dt_ms);
                }
                pub fn render(&self) {
                    self.canvas.begin_frame();
                    self.game.render(&self.canvas, self.theme);
                }
                pub fn score(&self) -> u32 {
                    self.game.score()
                }
                pub fn state(&self) -> u8 {
                    self.game.state() as u8
                }
                pub fn pause(&mut self) {
                    self.game.pause();
                }
                pub fn pointer_down(&mut self, x: f64, y: f64) -> bool {
                    self.game.pointer_down(x, y)
                }
                pub fn pointer_alt(&mut self, x: f64, y: f64) -> bool {
                    self.game.pointer_alt(x, y)
                }
                pub fn pointer_move(&mut self, x: f64, y: f64) {
                    self.game.pointer_move(x, y);
                }
                pub fn end_message(&self) -> Option<String> {
                    self.game.end_message()
                }
            }
        }
    };
}
