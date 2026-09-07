use blok_rs::board::BoardState;
#[cfg(any(
    feature = "until-end",
    not(any(feature = "nodes-1m", feature = "nodes-10k"))
))]
use blok_rs::board::GameResult;
use blok_rs::board::StartPosition;
use blok_rs::minimax;
use blok_rs::movegen::generate_moves;
use rand::SeedableRng;
use rand::prelude::*;
use rand::seq::IndexedRandom;

pub fn main() {
    let mut board = BoardState::new(StartPosition::Corner);

    const SEED: [u8; 32] = [0; 32];
    let mut rng = SmallRng::from_seed(SEED);
    let mut opening_moves: Vec<u32> = vec![];
    for _ in 0..10 {
        let moves = generate_moves(&board);
        let m = moves.choose(&mut rng).unwrap();
        opening_moves.push(*m);
        board.do_move(*m);
    }

    println!("Opening moves: {:?}", opening_moves);
    run_search(&mut board);
}

/// Constant time per move until the game ends (samply).
#[cfg(any(
    feature = "until-end",
    not(any(feature = "nodes-1m", feature = "nodes-10k"))
))]
fn run_search(board: &mut BoardState) {
    while board.game_result() == GameResult::InProgress {
        board.do_move(minimax::search(board, 2_000));
    }
}

/// Single search, 1M nodes (`time`).
#[cfg(feature = "nodes-1m")]
fn run_search(board: &mut BoardState) {
    let best_move = minimax::search_nodes(board, 1_000_000);
    println!("Best move: {best_move}");
}

/// Single search, 10k nodes (hyperfine).
#[cfg(feature = "nodes-10k")]
fn run_search(board: &mut BoardState) {
    let best_move = minimax::search_nodes(board, 10_000);
    println!("Best move: {best_move}");
}
