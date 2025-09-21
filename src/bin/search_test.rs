use blok_rs::board::BoardState;
use blok_rs::board::GameResult;
use blok_rs::board::StartPosition;
use blok_rs::minimax;
use blok_rs::movegen::generate_moves;
use rand::rng;
use rand::seq::IndexedRandom;

pub fn main() {
    let mut board = BoardState::new(StartPosition::Corner);

    let mut rng = rng();
    let mut opening_moves: Vec<u32> = vec![];
    for _ in 0..10 {
        let moves = generate_moves(&board);
        let m = moves.choose(&mut rng).unwrap();
        opening_moves.push(*m);
        board.do_move(*m);
    }

    println!("Opening moves: {:?}", opening_moves);
    // while board.game_result() == GameResult::InProgress {
    //     // Option 1: search nodes (for perf)
    //     // board.do_move(minimax::search_nodes(&board, 5_000));

    //     // Option 2: search (for benchmarking)
    //     board.do_move(minimax::search(&board, 2_000));
    // }

    let best_move = minimax::search(&board, 1_000_000);

    // println!("Best move: {}", best_move);
}
