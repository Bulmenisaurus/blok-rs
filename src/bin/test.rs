use blok_rs::{
    board::{BoardState, StartPosition},
    movegen,
};

fn main() {
    let mut board = BoardState::new(StartPosition::Corner);
    board.do_move(38912);
    board.do_move(73313);
    board.do_move(22685);
    let moves = movegen::generate_moves(&board);
    assert!(moves.contains(&66642));
    println!("{:?}", moves);
}
