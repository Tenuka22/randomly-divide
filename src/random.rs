use rand::RngExt;

use crate::person::GROUP_COUNT;

/// Assign a uniformly random group in `1..=GROUP_COUNT`.
///
/// `rand`'s thread-local generator in this version is seeded from OS
/// entropy and driven by a 12-round ChaCha stream cipher (the same cipher
/// family used in ChaCha20-Poly1305), periodically reseeding itself — a
/// genuine CSPRNG rather than a fast/insecure PRNG, so group placement
/// cannot be predicted or biased by an observer.
pub fn random_group() -> u8 {
    rand::rng().random_range(1..=GROUP_COUNT)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn random_group_is_always_in_bounds() {
        for _ in 0..1000 {
            let g = random_group();
            assert!((1..=GROUP_COUNT).contains(&g));
        }
    }
}
