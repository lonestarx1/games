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
    /// Number keys, top row or numpad: 0 to 9.
    Digit(u8),
    /// Backspace or Delete.
    Erase,
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
            "Backspace" | "Delete" => Key::Erase,
            c if (c.len() == 6 && c.starts_with("Digit")) || (c.len() == 7 && c.starts_with("Numpad")) => {
                match c.as_bytes()[c.len() - 1] {
                    d @ b'0'..=b'9' => Key::Digit(d - b'0'),
                    _ => Key::Other,
                }
            }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_codes() {
        assert_eq!(Key::from_code("ArrowUp"), Key::Up);
        assert_eq!(Key::from_code("KeyD"), Key::Right, "WASD doubles as arrows");
        assert_eq!(Key::from_code("KeyH"), Key::Letter('h'));
        assert_eq!(Key::from_code("Digit7"), Key::Digit(7));
        assert_eq!(Key::from_code("Numpad0"), Key::Digit(0));
        assert_eq!(Key::from_code("NumpadEnter"), Key::Other);
        assert_eq!(Key::from_code("Backspace"), Key::Erase);
    }
}
