use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};

pub fn player_a_zobrist() -> [u64; 196] {
    let mut rng = StdRng::seed_from_u64(0);
    [0u64; 196].map(|_| rng.random())
}

pub fn player_b_zobrist() -> [u64; 196] {
    let mut rng = StdRng::seed_from_u64(1);
    [0u64; 196].map(|_| rng.random())
}

// also somewhat functions as a stm, as for most moves null move count is 0 thus nmc[0] is xored every ply
pub fn null_move_count_zobrist() -> [u64; 3] {
    let mut rng = StdRng::seed_from_u64(2);
    // [u64; 3] has random defined!
    rng.random()
}
