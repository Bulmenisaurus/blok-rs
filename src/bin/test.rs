use blok_rs::{
    board::{BoardState, StartPosition},
    movegen,
};

fn main() {
    let mut board = BoardState::new(StartPosition::Corner);
    board.do_move(0);
    board.do_move(67152);
    let moves = movegen::generate_moves(&board);
    println!("{:?}", moves);
}
