/// Keys games care about, mapped from `KeyboardEvent.code`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Key {
    Up,
    Down,
    Left,
    Right,
    /// Space or Enter: start, launch, resume, restart.
    Action,
    /// P or Escape.
    Pause,
    /// Any other letter key, lowercase (e.g. 'h' for KeyH).
    Letter(char),
    Other,
}

impl Key {
    pub fn from_code(code: &str) -> Key {
        match code {
            "ArrowUp" | "KeyW" => Key::Up,
            "ArrowDown" | "KeyS" => Key::Down,
            "ArrowLeft" | "KeyA" => Key::Left,
            "ArrowRight" | "KeyD" => Key::Right,
            "Space" | "Enter" => Key::Action,
            "KeyP" | "Escape" => Key::Pause,
            c if c.len() == 4 && c.starts_with("Key") => {
                Key::Letter(c.as_bytes()[3].to_ascii_lowercase() as char)
            }
            _ => Key::Other,
        }
    }

    pub fn is_direction(self) -> bool {
        matches!(self, Key::Up | Key::Down | Key::Left | Key::Right)
    }
}
