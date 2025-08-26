use blok_rs::{
    board::{BoardState, GameResult, Player, StartPosition},
    mcts::MonteCarlo,
    movegen::{NULL_MOVE, generate_moves},
};

use rand::seq::IndexedRandom;

use rayon::prelude::*;
use std::{
    fs::File,
    io::Write,
    sync::{Arc, Mutex},
    time::Instant,
};

fn main() {
    let start = Instant::now();
    let file = Arc::new(Mutex::new(File::create("data.bin").unwrap()));

    let total: u32 = (0..1).into_par_iter().map(|_| playout(&file)).sum();

    let duration = start.elapsed();
    println!("Total positions: {}", total);
    println!(
        "Positions per second: {}",
        total as f64 / duration.as_secs_f64()
    );
}

fn playout(file: &Arc<Mutex<File>>) -> u32 {
    let mut i = 0;
    let mut board = BoardState::new(StartPosition::Corner);
    let mut mcts = MonteCarlo::new();
    let mut rng = rand::rng();

    let mut packed_positions: Vec<[u32; 15]> = Vec::new();

    // opening: skip opening moves
    // for _ in 0..6 {
    //     let moves = generate_moves(&board);
    //     let random_move = moves.choose(&mut rng).unwrap();
    //     board.do_move(*random_move);
    // }

    let bad_opening = vec![32768, 67152, 34953, 69168, 6162, 70730];
    for m in bad_opening {
        board.do_move(m);
    }

    while board.game_result() == GameResult::InProgress {
        mcts.run_search(&board, "eval");
        let (wins, plays) = mcts.get_stats();

        let chosen_move = mcts.best_play().unwrap();

        mcts.clear();

        let packed = pack(&board, wins, plays);

        // stop recording positions after the game is close to decided

        let probability = wins as f64 / plays as f64;

        let should_stop = chosen_move == NULL_MOVE || probability < 0.1 || probability > 0.9;
        if !should_stop {
            packed_positions.push(packed);
            i += 1;
        }

        board.do_move(chosen_move);
    }

    let result = board.game_result();
    println!("Result: {:?}", result);

    // annotate all of the packed positions with the result (make sure it's the side to move)
    for (i, packed) in packed_positions.iter_mut().enumerate() {
        let side_to_move = i % 2;
        let result_bits = match result {
            GameResult::PlayerAWon => {
                if side_to_move == 0 {
                    0
                } else {
                    1
                }
            }
            GameResult::PlayerBWon => {
                if side_to_move == 1 {
                    0
                } else {
                    1
                }
            }
            GameResult::Draw => 2,
            GameResult::InProgress => unreachable!(),
        };
        packed[14] = packed[14] | result_bits << 30;
        println!(
            "[Winner: {:?}, i: {}, side_to_move: {}] -> {} (packed: {:?})",
            result, i, side_to_move, result_bits, packed
        );
    }

    file.lock()
        .unwrap()
        .write(
            &packed_positions
                .into_iter()
                .flat_map(|p| serialize(p))
                .collect::<Vec<u8>>(),
        )
        .unwrap();

    i
}

//TODO: make sure side to move is always in the top left
//If the player is player b, we need to flip the board
fn pack(board: &BoardState, n_wins: usize, n_plays: usize) -> [u32; 15] {
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
    // n_wins, n_plays each take 15 bits
    // use the top two bits to store result (00 = win, 01 = loss, 10 = tie)
    packed[14] = n_plays as u32 | (n_wins as u32) << 15;
    packed
}

fn serialize(packed: [u32; 15]) -> Vec<u8> {
    packed.into_iter().flat_map(|p| p.to_le_bytes()).collect()
}

// reverses the 14 least significant bits of a u16
fn reverse_bits(bits: u16) -> u16 {
    bits.reverse_bits() >> 2
}
