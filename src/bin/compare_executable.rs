use blok_rs::board::{BoardState, GameResult, Player, StartPosition};
use blok_rs::movegen::generate_moves;
use rand::rng;
use rand::seq::IndexedRandom;
use rayon::prelude::*;
use std::process::{Command, Stdio};
use std::sync::{Arc, Mutex};

/// Path to the two engine executables to compare.
/// You may want to change these to the correct paths for your system.
const ENGINE1_PATH: &str = "./executables/ab-test";
const ENGINE2_PATH: &str = "./executables/ab-latest";

const OPENING_PLIES: usize = 6;
const PARALLEL_GAMES: usize = 4;

/// SPRT stuff - these will be set based on command line arguments

const ALPHA: f64 = 0.05;
const BETA: f64 = 0.05;

const DRAW_RATE: f64 = 0.05;

fn elo_to_prob(elo: f64) -> f64 {
    1.0 / (1.0 + 10.0f64.powf(-elo / 400.0))
}

#[derive(PartialEq)]
enum Outcome {
    Win,
    Draw,
    Lose,
}

#[derive(PartialEq)]
enum SPRTResult {
    SignificantNull,
    SignificantAlt,
    NotSignificant,
}

#[derive(Clone)]
struct GameTask<'a> {
    engine_white: &'a str,
    engine_black: &'a str,
    opening: Vec<u32>,
    perspective: GamePerspective,
}

#[derive(Clone, Copy)]
enum GamePerspective {
    Engine1,
    Engine2,
}

#[allow(clippy::upper_case_acronyms)]
struct SPRT {
    llr: f64,
    total_wins: usize,
    total_losses: usize,
    total_draws: usize,
    elo_0: f64,
    elo_1: f64,
}

impl SPRT {
    pub fn new(elo_0: f64, elo_1: f64) -> Self {
        Self {
            llr: 0.0,
            total_wins: 0,
            total_losses: 0,
            total_draws: 0,
            elo_0,
            elo_1,
        }
    }

    fn get_p1_win(&self) -> f64 {
        (1.0 - DRAW_RATE) * elo_to_prob(self.elo_1)
    }

    fn get_p1_draw() -> f64 {
        DRAW_RATE
    }

    fn get_p1_lose(&self) -> f64 {
        (1.0 - DRAW_RATE) * (1.0 - elo_to_prob(self.elo_1))
    }

    fn get_p0_win(&self) -> f64 {
        (1.0 - DRAW_RATE) * elo_to_prob(self.elo_0)
    }

    fn get_p0_draw() -> f64 {
        DRAW_RATE
    }

    fn get_p0_lose(&self) -> f64 {
        (1.0 - DRAW_RATE) * (1.0 - elo_to_prob(self.elo_0))
    }

    // The lower bound of the SPRT, indicates that the null hypothesis is true
    pub fn get_sprt_a() -> f64 {
        f64::ln(BETA / (1.0 - ALPHA))
    }

    // The upper bound of the SPRT, indicates that the alternative hypothesis is true
    pub fn get_sprt_b() -> f64 {
        f64::ln((1.0 - BETA) / ALPHA)
    }

    pub fn update(&mut self, result: Outcome) {
        self.total_wins += if result == Outcome::Win { 1 } else { 0 };
        self.total_losses += if result == Outcome::Lose { 1 } else { 0 };
        self.total_draws += if result == Outcome::Draw { 1 } else { 0 };

        match result {
            Outcome::Win => self.llr += f64::ln(self.get_p1_win() / self.get_p0_win()),
            Outcome::Draw => self.llr += f64::ln(Self::get_p1_draw() / Self::get_p0_draw()),
            Outcome::Lose => self.llr += f64::ln(self.get_p1_lose() / self.get_p0_lose()),
        }
    }

    pub fn result(&self) -> SPRTResult {
        if self.llr < Self::get_sprt_a() {
            return SPRTResult::SignificantNull;
        }
        if self.llr > Self::get_sprt_b() {
            return SPRTResult::SignificantAlt;
        }

        SPRTResult::NotSignificant
    }
}
fn main() {
    let args: Vec<String> = std::env::args().collect();

    if args.len() != 2 {
        eprintln!("Usage: {} <gain|nonregr>", args[0]);
        std::process::exit(1);
    }

    let (elo_0, elo_1) = match args[1].as_str() {
        "gain" => (0.0, 5.0),
        "nonregr" => (-5.0, 0.0),
        _ => {
            eprintln!("Invalid mode: {}. Use 'gain' or 'nonregr'", args[1]);
            std::process::exit(1);
        }
    };

    println!(
        "Starting comparison between {} and {}",
        ENGINE1_PATH, ENGINE2_PATH
    );
    println!("SPRT elo bounds: {} - {}", elo_0, elo_1);
    let sprt = Arc::new(Mutex::new(SPRT::new(elo_0, elo_1)));

    while {
        let sprt_guard = sprt.lock().unwrap();
        sprt_guard.result() == SPRTResult::NotSignificant
    } {
        // Generate openings for parallel games
        let openings: Vec<Vec<u32>> = (0..PARALLEL_GAMES).map(|_| generate_opening()).collect();

        // Create game tasks for parallel execution
        let game_tasks: Vec<_> = openings
            .into_iter()
            .flat_map(|opening| {
                vec![
                    GameTask {
                        engine_white: ENGINE1_PATH,
                        engine_black: ENGINE2_PATH,
                        opening: opening.clone(),
                        perspective: GamePerspective::Engine1,
                    },
                    GameTask {
                        engine_white: ENGINE2_PATH,
                        engine_black: ENGINE1_PATH,
                        opening,
                        perspective: GamePerspective::Engine2,
                    },
                ]
            })
            .collect();

        // Run games in parallel
        let results: Vec<Outcome> = game_tasks
            .par_iter()
            .map(|task| {
                let result = play_game(task.engine_white, task.engine_black, &task.opening);
                match (result, task.perspective) {
                    (GameResult::Win(Player::White), GamePerspective::Engine1) => Outcome::Win,
                    (GameResult::Win(Player::Black), GamePerspective::Engine1) => Outcome::Lose,
                    (GameResult::Win(Player::White), GamePerspective::Engine2) => Outcome::Lose,
                    (GameResult::Win(Player::Black), GamePerspective::Engine2) => Outcome::Win,
                    (GameResult::Draw, _) => Outcome::Draw,
                    (GameResult::InProgress, _) => unreachable!(),
                }
            })
            .collect();

        // Update SPRT with all results
        {
            let mut sprt_guard = sprt.lock().unwrap();
            for result in results {
                sprt_guard.update(result);
            }

            println!(
                "WDL {} {} {} -> LLR {:.2} ({:.2}, {:.2})",
                sprt_guard.total_wins,
                sprt_guard.total_draws,
                sprt_guard.total_losses,
                sprt_guard.llr,
                SPRT::get_sprt_a(),
                SPRT::get_sprt_b()
            );
        }
    }

    let sprt_guard = sprt.lock().unwrap();
    match sprt_guard.result() {
        SPRTResult::SignificantNull => println!("Significant Null"),
        SPRTResult::SignificantAlt => println!("Significant Alt"),
        SPRTResult::NotSignificant => println!("Not Significant"),
    }
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

    board.game_result()
}

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
