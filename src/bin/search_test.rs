use blok_rs::board::BoardState;
use blok_rs::board::StartPosition;
use blok_rs::minimax::search;

pub fn main() {
    let mut board = BoardState::new(StartPosition::Corner);

    for _ in 0..10 {
        board.do_move(search(&board, 100));
    }

    let best_move = search(&board, 1_000_000);

    println!("Best move: {}", best_move);
}
