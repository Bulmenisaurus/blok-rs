use blok_rs::{
    board::{BoardState, GameResult, StartPosition},
    movegen::generate_moves,
};
use rand::{rng, seq::IndexedRandom};
use std::time::Instant;

#[allow(dead_code)]
fn perft(board: &BoardState, depth: usize) -> u64 {
    let moves = generate_moves(board);

    if depth == 0 {
        return 1;
    }

    let mut nodes = 0;

    for m in moves {
        let mut new_board = board.clone();
        new_board.do_move(m);
        nodes += perft(&new_board, depth - 1);
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
    let start = Instant::now();
    let moves_amount = playout(10_000);
    let duration = start.elapsed();
    let secs = duration.as_secs_f64();
    let moves_per_sec = moves_amount as f64 / secs;
    println!(
        "Total moves: {}\nElapsed: {:.3} seconds\nMoves/second: {:.2}",
        moves_amount, secs, moves_per_sec
    );
    // let board = BoardState::new(StartPosition::Corner);
    // let nodes = perft(&board, 4);
    // println!("{}", nodes);
}
