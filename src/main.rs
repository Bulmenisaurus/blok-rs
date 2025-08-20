mod board;
mod mcts;
mod movegen;

use rand::rng;
use rand::seq::IndexedRandom;

fn main() {
    let mut rng = rng();
    let mut playerAwins = 0;
    let mut playerBwins = 0;
    let mut draws = 0;

    for _ in 0..1000 {
        let mut board = board::BoardState::new(board::StartPosition::corner);

        while !board.is_game_over() {
            let moves = movegen::generate_moves(&board);
            let random_move = moves.choose(&mut rng).unwrap();
            board.doMove(*random_move);
        }

        match board.game_result() {
            board::GameResult::PlayerAWon => playerAwins += 1,
            board::GameResult::PlayerBWon => playerBwins += 1,
            board::GameResult::Draw => draws += 1,
            board::GameResult::InProgress => unreachable!(),
        }
    }

    println!("Stats: {:?}", (playerAwins, playerBwins, draws));
}
