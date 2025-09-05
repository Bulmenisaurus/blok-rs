use blok_rs::board::{BoardState, GameResult, StartPosition};
use blok_rs::mcts::MonteCarlo;
use blok_rs::movegen::generate_moves;
use blok_rs::nn::Network;

static NNUE: Network = unsafe { std::mem::transmute(*include_bytes!("../../nn/quantised.bin")) };

fn main() {
    // Create a new board in the default start position
    let mut board = BoardState::new(StartPosition::Corner, NNUE);

    let mut mcts = MonteCarlo::new();

    let moves = generate_moves(&board);

    for m in moves {
        let mut new_board = board.clone();
        new_board.do_move(m);
        let eval = new_board.sample_nn();
        println!("Move: {}, Eval: {}", m, eval);
    }
}
