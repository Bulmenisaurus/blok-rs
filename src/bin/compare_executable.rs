use blok_rs::board::{BoardState, GameResult, StartPosition};
use blok_rs::movegen::generate_moves;
use rand::rng;
use rand::seq::IndexedRandom;
use std::process::{Command, Stdio};

/// Path to the two engine executables to compare.
/// You may want to change these to the correct paths for your system.
const ENGINE1_PATH: &str = "./executables/ab-tt";
const ENGINE2_PATH: &str = "./executables/ab-latest";
const NUM_GAME_PAIRS: usize = 50;
const OPENING_PLIES: usize = 6;

fn generate_opening() -> Vec<u32> {
    let mut board = BoardState::new(StartPosition::Corner);
    let mut moves: Vec<u32> = Vec::new();
    let mut rng = rng();

    for _ in 0..OPENING_PLIES {
        let legal_moves = generate_moves(&board);
        if legal_moves.is_empty() {
            break;
        }
        let &chosen_move = legal_moves.choose(&mut rng).unwrap();
        board.do_move(chosen_move);
        moves.push(chosen_move);
    }
    moves
}

fn main() {
    let mut total_engine1 = 0;
    let mut total_engine2 = 0;
    let mut total_draws = 0;

    for pair in 0..NUM_GAME_PAIRS {
        // Generate a random opening for this pair
        let opening = generate_opening();

        // Game 1: Engine1 as White, Engine2 as Black
        let result1 = play_game(ENGINE1_PATH, ENGINE2_PATH, &opening);
        match result1 {
            GameResult::PlayerAWon => total_engine1 += 1,
            GameResult::PlayerBWon => total_engine2 += 1,
            GameResult::Draw => total_draws += 1,
            GameResult::InProgress => unreachable!(),
        }
        println!(
            "Pair {} Game 1 result: {:?} (Engine1 as White, Engine2 as Black)",
            pair + 1,
            result1
        );

        // Game 2: Engine2 as White, Engine1 as Black
        let result2 = play_game(ENGINE2_PATH, ENGINE1_PATH, &opening);
        match result2 {
            GameResult::PlayerAWon => total_engine2 += 1,
            GameResult::PlayerBWon => total_engine1 += 1,
            GameResult::Draw => total_draws += 1,
            GameResult::InProgress => unreachable!(),
        }
        println!(
            "Pair {} Game 2 result: {:?} (Engine2 as White, Engine1 as Black)",
            pair + 1,
            result2
        );

        println!(
            "Cumulative: Engine1: {}, Engine2: {}, Draws: {}",
            total_engine1, total_engine2, total_draws
        );
    }

    println!(
        "Final result after {} pairs ({} games):",
        NUM_GAME_PAIRS,
        NUM_GAME_PAIRS * 2
    );
    println!("Engine1: {}", total_engine1);
    println!("Engine2: {}", total_engine2);
    println!("Draws: {}", total_draws);
}

/// Plays a single game between two engines, returning the result from the perspective of the first engine (as White).
/// The game starts from the given opening moves.
fn play_game(engine_white: &str, engine_black: &str, opening_moves: &[u32]) -> GameResult {
    let mut board = BoardState::new(StartPosition::Corner);
    let mut moves: Vec<u32> = Vec::new();
    let mut move_strings: Vec<String> = Vec::new();

    // Play the opening moves
    for &m in opening_moves {
        board.do_move(m);
        moves.push(m);
        move_strings.push(m.to_string());
    }

    // Determine which engine is to move next
    // If opening_moves.len() is even, it's White's turn (engine_white)
    // If odd, it's Black's turn (engine_black)
    let mut current_engine = if opening_moves.len() % 2 == 0 {
        engine_white
    } else {
        engine_black
    };

    while board.game_result() == GameResult::InProgress {
        // Prepare the move list as a space-separated string
        let moves_arg = move_strings.join(" ");

        // Call the engine executable
        let output = Command::new(current_engine)
            .arg(&moves_arg)
            .stdout(Stdio::piped())
            .output()
            .expect("Failed to run engine executable");

        if !output.status.success() {
            eprintln!(
                "Engine {} failed to run or returned error. Moves: {}",
                current_engine, moves_arg
            );
            std::process::exit(1);
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let best_move_str = stdout.trim();

        // Find the move in the legal moves
        let legal_moves = generate_moves(&board);
        let parsed_move = legal_moves
            .iter()
            .find(|&&m| m.to_string() == best_move_str);

        let chosen_move = match parsed_move {
            Some(&m) => m,
            None => {
                eprintln!(
                    "Engine {} returned illegal or unrecognized move: '{}'. Legal moves: {:?}",
                    current_engine,
                    best_move_str,
                    legal_moves
                        .iter()
                        .map(|m| m.to_string())
                        .collect::<Vec<_>>()
                );
                std::process::exit(1);
            }
        };

        board.do_move(chosen_move);
        moves.push(chosen_move);
        move_strings.push(chosen_move.to_string());

        // Alternate engines
        current_engine = if current_engine == engine_white {
            engine_black
        } else {
            engine_white
        };
    }

    println!(
        "Moves: {:?} [player a: {}, player b: {}]",
        moves, engine_white, engine_black
    );

    board.game_result()
}
