use blok_rs::board::BoardState;
use blok_rs::board::StartPosition;
use blok_rs::mcts::MonteCarlo;

pub fn main() {
    let mut board = BoardState::new(StartPosition::Corner);
    // board.do_move(0);
    let mut mcts = MonteCarlo::new();

    mcts.run_search(&board, "test");
    let best_move = mcts.best_play().unwrap();
    let (plays, score) = mcts.get_stats();
    println!("Result of search: score: {}, plays: {}", score, plays);
}
