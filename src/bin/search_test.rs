use blok_rs::board::BoardState;
use blok_rs::board::StartPosition;
use blok_rs::mcts::MonteCarlo;
use blok_rs::nn::NNUE;

pub fn main() {
    let mut board = BoardState::new(StartPosition::Corner, NNUE);
    // board.do_move(0);
    let mut mcts = MonteCarlo::new(NNUE, false);

    for i in 0..20 {
        mcts.run_search(&board, "test");
        let (plays, score) = mcts.get_stats();
        let best_move = mcts.best_play().unwrap();
        mcts.clear();
        println!(
            "Result of search: score: {}, plays: {}",
            score / plays as f64,
            plays
        );
        mcts.clear();
        board.do_move_nonlazy(best_move);
    }
}
