use blok_rs::board::BoardState;
use blok_rs::board::StartPosition;
use blok_rs::mcts::MonteCarlo;
use blok_rs::nn::NNUE;

pub fn main() {
    let mut board = BoardState::new(StartPosition::Corner, NNUE);
    // board.do_move(0);
    let mut mcts = MonteCarlo::new(NNUE);

    mcts.run_search(&board, "test");
    let best_move = mcts.best_play().unwrap();
    let (plays, score) = mcts.get_stats();
    println!("Result of search: score: {}, plays: {}", score, plays);
}
