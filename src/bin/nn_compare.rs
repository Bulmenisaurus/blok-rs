// Simplest possible way to compare two networks. Requires them be to have the same hidden size.

use blok_rs::{
    board::{BoardState, GameResult, Player, StartPosition},
    mcts::MonteCarlo,
    movegen::generate_moves,
    nn::Network,
};
use rand::seq::IndexedRandom;
use rayon::prelude::*;
use std::io::{self, Write};

static NNUE1: Network = unsafe { std::mem::transmute(*include_bytes!("../../nn/quantised.bin")) };

static NNUE2: Network =
    unsafe { std::mem::transmute(*include_bytes!("../../nn/quantised-old.bin")) };

fn main() {
    let num_games = 200;
    let mut total_nn1 = 0;
    let mut total_nn2 = 0;
    let mut total_draws = 0;
    let mut run_count = 0;

    loop {
        run_count += 1;
        println!("Starting run #{} ({} games)...", run_count, num_games);

        // Run all games in parallel
        let results: Vec<(i32, i32, i32)> = (0..num_games)
            .into_par_iter()
            .map(|_| compare_nn())
            .collect();

        // Aggregate results for this run
        let mut nn1 = 0;
        let mut nn2 = 0;
        let mut draws = 0;

        for (nn1_score, nn2_score, draws_score) in results.iter() {
            nn1 += nn1_score;
            nn2 += nn2_score;
            draws += draws_score;
        }

        total_nn1 += nn1;
        total_nn2 += nn2;
        total_draws += draws;

        println!(
            "Run #{} result: NN1: {}, NN2: {}, Draws: {}",
            run_count, nn1, nn2, draws
        );
        println!(
            "Cumulative result after {} runs ({} games): NN1: {}, NN2: {}, Draws: {}",
            run_count,
            run_count * num_games,
            total_nn1,
            total_nn2,
            total_draws
        );

        print!(
            "Do you want to run another set of {} games? (y/n): ",
            num_games
        );
        io::stdout().flush().unwrap();

        let mut input = String::new();
        if io::stdin().read_line(&mut input).is_err() {
            println!("Failed to read input, exiting.");
            break;
        }
        let input = input.trim().to_lowercase();
        if input != "y" && input != "yes" {
            println!("Exiting.");
            break;
        }
    }
}

fn compare_nn() -> (i32, i32, i32) {
    let mut b1_opening = BoardState::new(StartPosition::Corner, NNUE1);
    let mut mcts1 = MonteCarlo::new(NNUE1, false);
    let mut b2_opening = BoardState::new(StartPosition::Corner, NNUE2);
    let mut mcts2 = MonteCarlo::new(NNUE2, true);

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

    // Optionally print per-game stats here if desired
    println!(
        "Game: NN1: {}, NN2: {}, Draws: {}",
        nnue1_score, nnue2_score, draws
    );

    (nnue1_score, nnue2_score, draws)
}
