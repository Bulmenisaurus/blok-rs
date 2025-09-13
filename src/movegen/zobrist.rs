use once_cell::sync::Lazy;
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};

pub static PLAYER_A_ZOBRIST: Lazy<[u64; 196]> = Lazy::new(|| {
    let mut rng = StdRng::seed_from_u64(0);
    [0u64; 196].map(|_| rng.random())
});

pub static PLAYER_B_ZOBRIST: Lazy<[u64; 196]> = Lazy::new(|| {
    let mut rng = StdRng::seed_from_u64(1);
    [0u64; 196].map(|_| rng.random())
});

pub static NULL_MOVE_COUNT_ZOBRIST: Lazy<[u64; 3]> = Lazy::new(|| {
    let mut rng = StdRng::seed_from_u64(2);
    [0u64; 3].map(|_| rng.random())
});
