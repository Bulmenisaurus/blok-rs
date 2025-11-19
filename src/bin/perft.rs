use blok_rs::{
    board::{BoardState, GameResult, StartPosition},
    movegen::generate_moves,
};
use rand::{rng, seq::IndexedRandom};
use std::env;
use std::time::Instant;

#[allow(dead_code)]
pub fn perft(board: &BoardState, depth: usize, split: bool) -> u64 {
    let moves = generate_moves(board);

    if depth == 1 {
        return moves.len() as u64;
    }

    let mut nodes = 0;

    for m in moves {
        let mut new_board = board.clone();
        new_board.do_move(m);
        let moves = perft(&new_board, depth - 1, false);
        nodes += moves;

        if split {
            println!("depth: {}, move: {}, nodes: {}", depth, m, moves);
        }
    }

    nodes
}

#[allow(dead_code)]
fn playout(amount: usize) -> u64 {
    let mut rng = rng();
    let mut moves_amount: u64 = 0;
    for _ in 0..amount {
        let mut board = BoardState::new(StartPosition::Corner);
        while board.game_result() == GameResult::InProgress {
            let moves = generate_moves(&board);
            let m = moves.choose(&mut rng).unwrap();
            board.do_move(*m);
            moves_amount += 1;
        }
    }
    moves_amount
}

fn main() {
    let args: Vec<String> = env::args().collect();

    let depth = args
        .get(1)
        .unwrap_or(&String::from("5"))
        .parse::<usize>()
        .unwrap();

    let move_strs: Vec<&str> = args
        .get(2)
        .map(|s| s.as_str())
        .unwrap_or("")
        .split_whitespace()
        .collect();

    let mut board = BoardState::new(StartPosition::Corner);

    // Play the moves from the command line
    for mstr in move_strs {
        // Try to parse the move from string
        let legal_moves = generate_moves(&board);
        let parsed_move = legal_moves.iter().find(|&&m| m.to_string() == mstr);
        match parsed_move {
            Some(&m) => board.do_move(m),
            None => {
                eprintln!("Illegal or unrecognized move: {}", mstr);
                std::process::exit(1);
            }
        }
    }

    println!("Split perft, depth {depth}");
    let start = Instant::now();
    let nodes = perft(&board, depth, true);
    let duration = start.elapsed();
    let secs = duration.as_secs_f64();
    let nodes_per_sec = nodes as f64 / secs;
    println!(
        "Total nodes: {}\nElapsed: {:.3} seconds\nNodes/second: {:.2}",
        nodes, secs, nodes_per_sec
    );
}
