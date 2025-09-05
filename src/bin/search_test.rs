use blok_rs::board::BoardState;
use blok_rs::board::StartPosition;
use blok_rs::minimax::search;

pub fn main() {
    let board = BoardState::new(StartPosition::Corner);

    let best_move = search(&board, 1_000);

    println!("Best move: {}", best_move);
}
