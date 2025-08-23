use blok_rs::board::BoardState;
use blok_rs::board::StartPosition;
use blok_rs::mcts::MonteCarlo;

pub fn main() {
    let board = BoardState::new(StartPosition::Corner);
    let mut mcts = MonteCarlo::new();

    mcts.run_search(&board, "easy");
    let best_move = mcts.best_play().unwrap();
    let stats = mcts.get_stats();
    println!(
        "Result of search: {} (eval ~ {}/{})",
        best_move, stats.0, stats.1
    );
}
