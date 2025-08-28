use blok_rs::board::{BoardState, GameResult, StartPosition};
use blok_rs::mcts::MonteCarlo;
use blok_rs::movegen::generate_moves;
use blok_rs::nn::{Accumulator, Network};
use rand::rng;
use rand::seq::IndexedRandom;

fn main() {
    // Create a new board in the default start position
    let mut board = BoardState::new(StartPosition::Corner);
    let mut rng = rand::rng();

    // Play 6 random moves
    for _ in 0..6 {
        let moves = generate_moves(&board);
        if moves.is_empty() {
            break;
        }
        let random_move = moves.choose(&mut rng).unwrap();
        board.do_move(*random_move);
    }

    // Now, 10 times, run MCTS and print the best move
    for i in 0..10 {
        let mut mcts = MonteCarlo::new();
        mcts.run_search(&board, "eval");
        let best_move = mcts.best_play().unwrap();
        println!("Best move {}: {}", i + 1, best_move);
        // Optionally, play the move to see the sequence
        // board.do_move(best_move);
    }
}
