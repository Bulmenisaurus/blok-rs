mod board;
mod mcts;
mod movegen;

use rand::rng;
use rand::seq::IndexedRandom;

fn main() {
    let mut rng = rng();
    let mut player_a_wins = 0;
    let mut player_b_wins = 0;
    let mut draws = 0;

    for _ in 0..1000 {
        let mut board = board::BoardState::new(board::StartPosition::Corner);

        while !board.is_game_over() {
            let moves = movegen::generate_moves(&board);
            let random_move = moves.choose(&mut rng).unwrap();
            board.do_move(*random_move);
        }

        match board.game_result() {
            board::GameResult::PlayerAWon => player_a_wins += 1,
            board::GameResult::PlayerBWon => player_b_wins += 1,
            board::GameResult::Draw => draws += 1,
            board::GameResult::InProgress => unreachable!(),
        }
    }

    println!("Stats: {:?}", (player_a_wins, player_b_wins, draws));
}
