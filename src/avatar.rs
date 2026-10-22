//! Random, hashed avatars.
//!
//! Each person gets a random `u64` seed (drawn from the same CSPRNG used for
//! group assignment) at creation time. The seed is hashed with SHA-256 to
//! deterministically derive a 5x5 mirrored identicon grid and an accent
//! color — so the avatar looks the same every time it's redrawn or reloaded
//! from CSV/DB, without storing the pixels themselves.

use rand::RngExt;
use ratatui::style::Color;
use sha2::{Digest, Sha256};

/// Draw a fresh random avatar seed using the app's CSPRNG.
pub fn random_seed() -> u64 {
    rand::rng().random()
}

fn hash_bytes(seed: u64) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(seed.to_le_bytes());
    hasher.finalize().into()
}

/// A 5x5 boolean grid (mirrored left-right, GitHub-identicon style) derived
/// from the seed's hash. `true` = filled cell.
pub fn grid(seed: u64) -> [[bool; 5]; 5] {
    let bytes = hash_bytes(seed);
    let mut cells = [[false; 5]; 5];
    // Only need bits for the left 3 columns; columns 3-4 mirror 1-0.
    let mut bit_idx = 0usize;
    for row in 0..5 {
        for col in 0..3 {
            let byte = bytes[bit_idx % bytes.len()];
            let bit = (byte >> (bit_idx % 8)) & 1;
            cells[row][col] = bit == 1;
            cells[row][4 - col] = bit == 1;
            bit_idx += 1;
        }
    }
    cells
}

/// A stable accent color derived from the hash, chosen from a palette that
/// stays readable on both light and dark terminal themes.
pub fn color(seed: u64) -> Color {
    const PALETTE: [Color; 8] = [
        Color::Red,
        Color::Green,
        Color::Yellow,
        Color::Blue,
        Color::Magenta,
        Color::Cyan,
        Color::LightRed,
        Color::LightGreen,
    ];
    let bytes = hash_bytes(seed);
    PALETTE[bytes[31] as usize % PALETTE.len()]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn same_seed_always_renders_the_same_avatar() {
        let seed = 42;
        assert_eq!(grid(seed), grid(seed));
        assert_eq!(color(seed), color(seed));
    }

    #[test]
    fn grid_is_left_right_mirrored() {
        for seed in [1u64, 2, 3, 123456789] {
            let g = grid(seed);
            for row in g {
                assert_eq!(row[0], row[4]);
                assert_eq!(row[1], row[3]);
            }
        }
    }
}
