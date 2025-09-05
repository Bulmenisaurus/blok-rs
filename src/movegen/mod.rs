#![allow(clippy::module_inception)]
mod movegen;

pub use movegen::{
    Move, NULL_MOVE, PIECE_DATA, generate_moves, update_move_cache,
    update_move_cache_from_null_move,
};
