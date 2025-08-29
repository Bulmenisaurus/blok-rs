use blok_rs::board::BoardState;
use blok_rs::board::StartPosition;
use blok_rs::mcts::MonteCarlo;

pub fn main() {
    let mut board = BoardState::new(StartPosition::Corner);
    board.do_move(0);
    let mut mcts = MonteCarlo::new();

    mcts.run_search(&board, "hard");
    let best_move = mcts.best_play().unwrap();
    let stats = mcts.get_stats();
    println!(
        "Result of search: {} (eval ~ {}/{}) score: {}",
        best_move, stats.0, stats.1, stats.2
    );
}
