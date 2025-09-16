use blok_rs::{
    board::{BoardState, GameResult, StartPosition},
    movegen::generate_moves,
};
use rand::{rng, seq::IndexedRandom};

#[allow(dead_code)]
fn perft(board: &BoardState, depth: usize) -> u64 {
    let moves = generate_moves(board);

    if depth == 1 {
        return moves.len() as u64;
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
fn playout(amount: usize) {
    let mut rng = rng();
    for _ in 0..amount {
        let mut board = BoardState::new(StartPosition::Corner);
        while board.game_result() == GameResult::InProgress {
            let moves = generate_moves(&board);
            let m = moves.choose(&mut rng).unwrap();
            board.do_move(*m);
        }
    }
}

fn main() {
    playout(1_000);
    // let board = BoardState::new(StartPosition::Corner);
    // let nodes = perft(&board, 4);
    // println!("{}", nodes);
}
