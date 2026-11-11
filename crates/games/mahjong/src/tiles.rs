//! The traditional 144-tile set.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Face {
    /// 1..=9
    Dots(u8),
    Bamboo(u8),
    Chars(u8),
    /// 0..4: east, south, west, north
    Wind(u8),
    /// 0..3: red, green, white
    Dragon(u8),
    /// 0..4
    Flower(u8),
    /// 0..4
    Season(u8),
}

impl Face {
    /// Tiles match only when they are identical.
    pub fn group(self) -> u16 {
        match self {
            Face::Dots(n) => n as u16,
            Face::Bamboo(n) => 10 + n as u16,
            Face::Chars(n) => 20 + n as u16,
            Face::Wind(n) => 30 + n as u16,
            Face::Dragon(n) => 40 + n as u16,
            Face::Flower(n) => 50 + n as u16,
            Face::Season(n) => 60 + n as u16,
        }
    }

    pub fn matches(self, other: Face) -> bool {
        self.group() == other.group()
    }
}

/// Every pair a deal can draw from: two pairs of each of the 34 main kinds,
/// plus one identical pair of each flower and season (76 pairs). A deal uses
/// as many as the layout needs, so a few pairs are left out each game.
pub fn full_set_pairs() -> Vec<(Face, Face)> {
    let mut kinds = Vec::new();
    for n in 1..=9 {
        kinds.extend([Face::Dots(n), Face::Bamboo(n), Face::Chars(n)]);
    }
    kinds.extend((0..4).map(Face::Wind));
    kinds.extend((0..3).map(Face::Dragon));

    let mut pairs: Vec<(Face, Face)> = kinds.iter().flat_map(|&k| [(k, k), (k, k)]).collect();
    pairs.extend((0..4).map(|n| (Face::Flower(n), Face::Flower(n))));
    pairs.extend((0..4).map(|n| (Face::Season(n), Face::Season(n))));
    pairs
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn enough_pairs_for_a_144_tile_layout_and_all_identical() {
        let pairs = full_set_pairs();
        assert!(pairs.len() * 2 >= 144);
        assert!(pairs.iter().all(|(a, b)| a == b));
    }

    #[test]
    fn only_identical_tiles_match() {
        assert!(Face::Flower(2).matches(Face::Flower(2)));
        assert!(!Face::Flower(0).matches(Face::Flower(3)));
        assert!(!Face::Season(1).matches(Face::Season(2)));
        assert!(!Face::Dots(1).matches(Face::Bamboo(1)));
    }
}
