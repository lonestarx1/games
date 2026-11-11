//! Where tiles sit, which tiles block which, and pixel geometry.
//!
//! Positions use half-tile units so layouts can offset tiles by half a tile.
//! A tile at (x, y) covers x..x+2 and y..y+2.

pub const W: f64 = 600.0;
pub const H: f64 = 560.0;
/// Tile face size in px.
pub const TW: f64 = 44.0;
pub const TH: f64 = 58.0;
/// Each layer is drawn this much up and left; also the visible tile thickness.
pub const DEPTH: f64 = 6.0;
const BOARD_COLS: f64 = 12.0;
const OX: f64 = (W - BOARD_COLS * TW) / 2.0 + DEPTH;
const OY: f64 = 52.0 + DEPTH;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Slot {
    pub x: i32,
    pub y: i32,
    pub z: i32,
}

/// An original stepped-pyramid layout: 144 tiles in 4 layers.
pub fn pyramid() -> Vec<Slot> {
    let layers: [(i32, &[i32]); 4] = [
        (0, &[6, 10, 12, 12, 12, 12, 10, 6]),
        (2, &[6, 8, 8, 8, 8, 6]),
        (4, &[4, 4, 4, 4]),
        (6, &[2, 2]),
    ];
    let mut slots = Vec::new();
    for (z, (y0, rows)) in layers.iter().enumerate() {
        for (r, &width) in rows.iter().enumerate() {
            let x0 = BOARD_COLS as i32 - width; // centred on a 24-unit-wide board
            for c in 0..width {
                slots.push(Slot { x: x0 + 2 * c, y: y0 + 2 * r as i32, z: z as i32 });
            }
        }
    }
    sort_for_drawing(&mut slots);
    slots
}

/// Lower layers first, then top-to-bottom, left-to-right, so each tile's
/// visible thickness (right and bottom edges) is covered by its neighbours.
pub fn sort_for_drawing(slots: &mut [Slot]) {
    slots.sort_by_key(|s| (s.z, s.y, s.x));
}

/// Precomputed blocking relationships between slots.
pub struct Board {
    pub slots: Vec<Slot>,
    /// Tiles on the layer above that overlap this one.
    above: Vec<Vec<usize>>,
    /// Touching tiles on the same layer, to the left and right.
    left: Vec<Vec<usize>>,
    right: Vec<Vec<usize>>,
}

impl Board {
    pub fn new(slots: Vec<Slot>) -> Board {
        let n = slots.len();
        let (mut above, mut left, mut right) = (vec![vec![]; n], vec![vec![]; n], vec![vec![]; n]);
        for (i, a) in slots.iter().enumerate() {
            for (j, b) in slots.iter().enumerate() {
                let overlap_y = (a.y - b.y).abs() < 2;
                if b.z == a.z + 1 && (a.x - b.x).abs() < 2 && overlap_y {
                    above[i].push(j);
                } else if b.z == a.z && overlap_y {
                    if b.x == a.x - 2 {
                        left[i].push(j);
                    } else if b.x == a.x + 2 {
                        right[i].push(j);
                    }
                }
            }
        }
        Board { slots, above, left, right }
    }

    pub fn len(&self) -> usize {
        self.slots.len()
    }

    /// A tile is free when nothing is on top and one long side is open.
    pub fn is_free(&self, alive: &[bool], i: usize) -> bool {
        let clear = |v: &Vec<usize>| v.iter().all(|&j| !alive[j]);
        alive[i] && clear(&self.above[i]) && (clear(&self.left[i]) || clear(&self.right[i]))
    }

    pub fn tile_rect(&self, i: usize) -> (f64, f64, f64, f64) {
        let s = self.slots[i];
        let lift = s.z as f64 * DEPTH;
        (OX + s.x as f64 * TW / 2.0 - lift, OY + s.y as f64 * TH / 2.0 - lift, TW, TH)
    }

    /// Topmost live tile under a point.
    pub fn tile_at(&self, alive: &[bool], x: f64, y: f64) -> Option<usize> {
        (0..self.len()).rev().find(|&i| {
            let (tx, ty, w, h) = self.tile_rect(i);
            alive[i] && x >= tx && x < tx + w && y >= ty && y < ty + h
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pyramid_has_144_slots_and_no_duplicates() {
        let s = pyramid();
        assert_eq!(s.len(), 144);
        for (i, a) in s.iter().enumerate() {
            assert!(!s[i + 1..].contains(a));
        }
    }

    #[test]
    fn blocking_rules() {
        // Three in a row, plus one on top of the middle.
        let b = Board::new(vec![
            Slot { x: 0, y: 0, z: 0 },
            Slot { x: 2, y: 0, z: 0 },
            Slot { x: 4, y: 0, z: 0 },
            Slot { x: 2, y: 0, z: 1 },
        ]);
        let all = [true; 4];
        assert!(b.is_free(&all, 0)); // left end
        assert!(!b.is_free(&all, 1)); // covered and both sides blocked
        assert!(b.is_free(&all, 2)); // right end
        assert!(b.is_free(&all, 3)); // on top
        let no_top = [true, true, true, false];
        assert!(!b.is_free(&no_top, 1)); // still blocked on both sides
        let no_left = [false, true, true, false];
        assert!(b.is_free(&no_left, 1));
    }
}
