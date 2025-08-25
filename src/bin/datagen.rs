use blok_rs::{
    board::{BoardState, GameResult, Player, StartPosition},
    mcts::MonteCarlo,
    movegen::generate_moves,
};

use rand::seq::IndexedRandom;

use rayon::prelude::*;
use std::time::Instant;

fn main() {
    let start = Instant::now();

    let total: i32 = (0..8)
        .into_par_iter()
        .map(|_| {
            let mut i = 0;
            let mut board = BoardState::new(StartPosition::Corner);
            let mut mcts = MonteCarlo::new();
            let mut rng = rand::rng();

            // opening: skip opening moves
            for _ in 0..6 {
                let moves = generate_moves(&board);
                let random_move = moves.choose(&mut rng).unwrap();
                board.do_move(*random_move);
            }

            while board.game_result() == GameResult::InProgress {
                mcts.run_search(&board, "eval");
                let evaluation = mcts.get_stats();

                let chosen_move = mcts.best_play().unwrap();

                mcts.clear();
                println!("Evaluation: {:?}", evaluation);

                let packed = pack(&board, evaluation.0);
                // println!("Packed: {:?}", packed);

                board.do_move(chosen_move);
                i += 1;
            }

            i
        })
        .sum();

    let duration = start.elapsed();
    println!("Total positions: {}", total);
    println!(
        "Positions per second: {}",
        total as f64 / duration.as_secs_f64()
    );
}

//TODO: make sure side to move is always in the top left
//If the player is player b, we need to flip the board
fn pack(board: &BoardState, n_wins: usize) -> [u32; 15] {
    let mut packed: [u32; 15] = [0; 15];

    #[allow(clippy::needless_range_loop)]
    for i in 0..14 {
        let actual_i = match board.player {
            Player::White => i,
            Player::Black => 13 - i,
        };

        let player_a_data = match board.player {
            Player::White => board.player_a_bit_board[actual_i],
            Player::Black => reverse_bits(board.player_b_bit_board[actual_i]),
        };

        let player_b_data = match board.player {
            Player::White => board.player_b_bit_board[actual_i],
            Player::Black => reverse_bits(board.player_a_bit_board[actual_i]),
        };

        packed[i] = player_a_data as u32 | (player_b_data as u32) << 16;
    }
    packed[14] = n_wins as u32;
    packed
}

// reverses the 14 least significant bits of a u16
fn reverse_bits(bits: u16) -> u16 {
    bits.reverse_bits() >> 2
}
