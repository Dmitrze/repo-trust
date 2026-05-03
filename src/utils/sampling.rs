//! Deterministic stargazer sampling helpers.

use rand::seq::SliceRandom;
use rand_chacha::ChaCha20Rng;
use rand_chacha::rand_core::SeedableRng;

/// Derive a stable 64-bit seed from `(repo, scoring_version)` via blake3.
///
/// This means a user gets the same sample for the same repo across runs
/// without having to pass `--seed` explicitly, while still allowing override.
#[must_use]
pub fn derive_seed(repo: &str, scoring_version: &str) -> u64 {
    let mut hasher = blake3::Hasher::new();
    hasher.update(repo.as_bytes());
    hasher.update(b"\0");
    hasher.update(scoring_version.as_bytes());
    let hash = hasher.finalize();
    let bytes = hash.as_bytes();
    u64::from_le_bytes(bytes[..8].try_into().expect("blake3 always returns 32 bytes"))
}

/// Sample without replacement from a slice using a seeded ChaCha20 RNG.
///
/// Order in the returned vec is the order produced by `partial_shuffle`,
/// which is deterministic given the same seed.
#[must_use]
pub fn sample<'a, T: Clone>(items: &'a [T], n: usize, seed: u64) -> Vec<T> {
    let mut rng = ChaCha20Rng::seed_from_u64(seed);
    let mut indices: Vec<usize> = (0..items.len()).collect();
    let take = n.min(items.len());
    let (chosen, _) = indices.partial_shuffle(&mut rng, take);
    chosen.iter().map(|&i| items[i].clone()).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn derive_seed_is_stable() {
        let s1 = derive_seed("octocat/Hello-World", "1.0.0");
        let s2 = derive_seed("octocat/Hello-World", "1.0.0");
        assert_eq!(s1, s2);
    }

    #[test]
    fn derive_seed_differs_by_repo() {
        let s1 = derive_seed("a/b", "1.0.0");
        let s2 = derive_seed("a/c", "1.0.0");
        assert_ne!(s1, s2);
    }

    #[test]
    fn derive_seed_differs_by_version() {
        let s1 = derive_seed("a/b", "1.0.0");
        let s2 = derive_seed("a/b", "1.1.0");
        assert_ne!(s1, s2);
    }

    #[test]
    fn sample_is_deterministic() {
        let items: Vec<i32> = (0..100).collect();
        let s1 = sample(&items, 10, 42);
        let s2 = sample(&items, 10, 42);
        assert_eq!(s1, s2);
    }

    #[test]
    fn sample_changes_with_seed() {
        let items: Vec<i32> = (0..100).collect();
        let s1 = sample(&items, 10, 1);
        let s2 = sample(&items, 10, 2);
        assert_ne!(s1, s2);
    }

    #[test]
    fn sample_caps_at_input_length() {
        let items: Vec<i32> = (0..5).collect();
        let out = sample(&items, 100, 1);
        assert_eq!(out.len(), 5);
    }
}
