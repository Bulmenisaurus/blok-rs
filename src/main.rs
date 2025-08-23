mod board;
mod mcts;
mod movegen;

use mcts::MonteCarlo;

fn main() {
    let mut board = board::BoardState::new(board::StartPosition::Corner);
    let mut eval: MonteCarlo = MonteCarlo::new();

    eval.run_search(board.clone());
    let best = eval.best_play().unwrap();
    println!("best: {}", best);
    println!("stat: {:?}", eval.get_stats());

    board.do_move(0);
    eval.clear();
}
