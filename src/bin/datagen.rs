use blok_rs::{
    board::{BoardState, GameResult, Player, StartPosition},
    mcts::MonteCarlo,
    movegen::{NULL_MOVE, generate_moves},
    nn::NNUE,
};

use rand::seq::IndexedRandom;

use rayon::prelude::*;
use std::{
    fs::File,
    io::{BufWriter, Write},
    sync::atomic::{AtomicU64, Ordering},
    time::Instant,
};

const TOTAL_GAMES: usize = 10;
const BATCH_SIZE: usize = 10;
const START_POSITION: StartPosition = StartPosition::Corner;
const OPENING_PLIES: usize = 6;

fn main() {
    let start = Instant::now();
    let mut file = BufWriter::new(File::create("data.bin").unwrap());
    let total_written = AtomicU64::new(0);

    // Process in batches to allow periodic writes
    let batch_size = BATCH_SIZE; // Adjust based on your needs
    let total_games = TOTAL_GAMES;

    for batch_start in (0..total_games).step_by(batch_size) {
        let batch_end = (batch_start + batch_size).min(total_games);

        // Collect batch results
        let results: Vec<Vec<[u32; 15]>> = (batch_start..batch_end)
            .into_par_iter()
            .map(|_| playout())
            .collect();

        // Write batch results
        for packed_positions in results {
            let bytes = packed_positions
                .iter()
                .flat_map(|p| serialize(*p))
                .collect::<Vec<u8>>();

            file.write_all(&bytes).unwrap();
            total_written.fetch_add(packed_positions.len() as u64, Ordering::Relaxed);
        }

        // Flush to disk periodically
        file.flush().unwrap();
        println!(
            "Written {} positions so far",
            total_written.load(Ordering::Relaxed)
        );
    }

    let duration = start.elapsed();
    println!("Total positions: {}", total_written.load(Ordering::Relaxed));
    println!(
        "Positions per second: {}",
        total_written.load(Ordering::Relaxed) as f64 / duration.as_secs_f64()
    );
}

fn playout() -> Vec<[u32; 15]> {
    let mut board = BoardState::new(START_POSITION, NNUE);
    let mut mcts = MonteCarlo::new(NNUE, false);
    let mut rng = rand::rng();

    let mut packed_positions: Vec<[u32; 15]> = Vec::new();

    // opening: skip opening moves
    for _ in 0..OPENING_PLIES {
        let moves = generate_moves(&board);
        let random_move = moves.choose(&mut rng).unwrap();
        board.do_move_nonlazy(*random_move);
    }

    while board.game_result() == GameResult::InProgress {
        mcts.run_search(&board, "eval");
        let (plays, score) = mcts.get_stats();

        let approximate_wins = ((score + plays as f64) / 2.0) as usize;

        let chosen_move = mcts.best_play().unwrap();

        mcts.clear();

        let packed = pack(&board, approximate_wins, plays);

        // stop recording positions after the game is close to decided

        let probability = (score / plays as f64 + 1.0) / 2.0;

        let should_stop = chosen_move == NULL_MOVE || !(0.1..=0.9).contains(&probability);
        if !should_stop {
            packed_positions.push(packed);
        }

        board.do_move_nonlazy(chosen_move);
    }

    let result = board.game_result();

    // annotate all of the packed positions with the result (absolute result)
    for packed in packed_positions.iter_mut() {
        // 0 -> A, 1 -> B, 2 -> T
        let result_bits = match result {
            GameResult::PlayerAWon => 0,
            GameResult::PlayerBWon => 1,
            GameResult::Draw => 2,
            GameResult::InProgress => unreachable!(),
        };
        packed[14] |= result_bits << 30;
    }

    packed_positions
}

//Note: nevermind do not flip the board!!!!! (store stm in the last row)
fn pack(board: &BoardState, n_wins: usize, n_plays: usize) -> [u32; 15] {
    let mut packed: [u32; 15] = [0; 15];

    #[allow(clippy::needless_range_loop)]
    for i in 0..14 {
        let player_a_data = board.player_a_bit_board[i];
        let player_b_data = board.player_b_bit_board[i];

        packed[i] = player_a_data as u32 | (player_b_data as u32) << 16;
    }
    // n_wins, n_plays each take 14 bits (so max of 2^14 =)
    // use the top two bits to store result (00 = win, 01 = loss, 10 = tie)

    let side_to_move = match board.player {
        Player::White => 0,
        Player::Black => 1,
    };

    packed[14] = n_plays as u32 | (n_wins as u32) << 14 | side_to_move << 28;
    packed
}

fn serialize(packed: [u32; 15]) -> Vec<u8> {
    packed.into_iter().flat_map(|p| p.to_le_bytes()).collect()
}
