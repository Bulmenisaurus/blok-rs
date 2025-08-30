// Simplest possible way to compare two networks. Requires them be to have the same hidden size.

use blok_rs::{
    board::{BoardState, GameResult, Player, StartPosition},
    mcts::MonteCarlo,
    movegen::generate_moves,
    nn::{NNUE, Network},
};
use rand::seq::IndexedRandom;

static NNUE1: Network =
    unsafe { std::mem::transmute(*include_bytes!("../../nn/quantised-nnue.bin")) };

static NNUE2: Network = unsafe { std::mem::transmute(*include_bytes!("../../nn/quantised.bin")) };

fn main() {
    let mut nn1 = 0;
    let mut nn2 = 0;
    let mut draws = 0;

    for i in 0..100 {
        let (nn1_score, nn2_score, draws_score) = compare_nn();
        nn1 += nn1_score;
        nn2 += nn2_score;
        draws += draws_score;
        println!(
            "Game {}: NN1: {}, NN2: {}, Draws: {}",
            i, nn1_score, nn2_score, draws_score
        );
    }

    println!("Final result: NN1: {}, NN2: {}, Draws: {}", nn1, nn2, draws);
}

fn compare_nn() -> (i32, i32, i32) {
    let mut b1_opening = BoardState::new(StartPosition::Corner, NNUE1);
    let mut mcts1 = MonteCarlo::new(NNUE1);
    let mut b2_opening = BoardState::new(StartPosition::Corner, NNUE2);
    let mut mcts2 = MonteCarlo::new(NNUE2);

    let mut rng = rand::rng();

    let mut nnue1_score = 0;
    let mut nnue2_score = 0;
    let mut draws = 0;

    // opening
    for _ in 0..6 {
        let moves = generate_moves(&b1_opening);
        let m = moves.choose(&mut rng).unwrap();
        b1_opening.do_move(*m);
        b2_opening.do_move(*m);
    }

    let mut board1 = b1_opening.clone();
    let mut board2 = b2_opening.clone();

    // now we play
    while board1.game_result() == GameResult::InProgress {
        let my_move = if board1.player == Player::White {
            mcts1.run_search(&board1, "eval");
            let best_move = mcts1.best_play().unwrap();
            mcts1.clear();
            best_move
        } else {
            mcts2.run_search(&board2, "eval");
            let best_move = mcts2.best_play().unwrap();
            mcts2.clear();
            best_move
        };

        board1.do_move(my_move);
        board2.do_move(my_move);
    }

    let result = board1.game_result();
    if result == GameResult::PlayerAWon {
        nnue1_score += 1;
    } else if result == GameResult::PlayerBWon {
        nnue2_score += 1;
    } else {
        draws += 1;
    }

    let mut board1 = b1_opening.clone();
    let mut board2 = b2_opening.clone();

    // play the same opening, with other color
    while board1.game_result() == GameResult::InProgress {
        // now play mcts1 if we're black
        let my_move = if board1.player == Player::Black {
            mcts1.run_search(&board1, "eval");
            let best_move = mcts1.best_play().unwrap();
            mcts1.clear();
            best_move
        } else {
            mcts2.run_search(&board2, "eval");
            let best_move = mcts2.best_play().unwrap();
            mcts2.clear();
            best_move
        };

        board1.do_move(my_move);
        board2.do_move(my_move);
    }

    let result = board1.game_result();
    if result == GameResult::PlayerAWon {
        nnue2_score += 1;
    } else if result == GameResult::PlayerBWon {
        nnue1_score += 1;
    } else {
        draws += 1;
    }

    return (nnue1_score, nnue2_score, draws);
}

fn pack(board: &BoardState, n_wins: usize, n_plays: usize) -> [u32; 15] {
    let mut packed: [u32; 15] = [0; 15];

    #[allow(clippy::needless_range_loop)]
    for i in 0..14 {
        let player_a_data = board.player_a_bit_board[i];
        let player_b_data = board.player_b_bit_board[i];

        packed[i] = player_a_data as u32 | (player_b_data as u32) << 16;
    }
    // n_wins, n_plays each take 14 bits (so max of 2^14 =)
    // use the top two bits to store result (00 = win, 01 = loss, 10 = tie)

    let side_to_move = match board.player {
        Player::White => 0,
        Player::Black => 1,
    };

    packed[14] = n_plays as u32 | (n_wins as u32) << 14 | side_to_move << 28;
    packed
}
