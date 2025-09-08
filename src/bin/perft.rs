use blok_rs::{
    board::{BoardState, StartPosition},
    movegen::generate_moves,
};

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

fn main() {
    let board = BoardState::new(StartPosition::Corner);
    let nodes = perft(&board, 3);
    println!("{}", nodes);
}
