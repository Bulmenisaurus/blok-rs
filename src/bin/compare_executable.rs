use blok_rs::board::{BoardState, GameResult, Player, StartPosition};
use blok_rs::movegen::generate_moves;
use rand::rng;
use rand::seq::IndexedRandom;
use std::process::{Command, Stdio, exit};

/// Path to the two engine executables to compare.
/// You may want to change these to the correct paths for your system.
const ENGINE1_PATH: &str = "./executables/mcts-puct-latest";
const ENGINE2_PATH: &str = "./executables/ab-latest";

const OPENING_PLIES: usize = 6;

/// SPRT stuff
const ELO_0: f64 = 0.0;
const ELO_1: f64 = 10.0;

const ALPHA: f64 = 0.05;
const BETA: f64 = 0.05;

const DRAW_RATE: f64 = 0.01;

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

struct SPRT {
    LLR: f64,
    total_wins: usize,
    total_losses: usize,
    total_draws: usize,
}

impl SPRT {
    pub fn new() -> Self {
        Self {
            LLR: 0.0,
            total_wins: 0,
            total_losses: 0,
            total_draws: 0,
        }
    }

    fn get_p1_win() -> f64 {
        (1.0 - DRAW_RATE) * elo_to_prob(ELO_1)
    }

    fn get_p1_draw() -> f64 {
        DRAW_RATE
    }

    fn get_p1_lose() -> f64 {
        (1.0 - DRAW_RATE) * (1.0 - elo_to_prob(ELO_1))
    }

    fn get_p0_win() -> f64 {
        (1.0 - DRAW_RATE) * elo_to_prob(ELO_0)
    }

    fn get_p0_draw() -> f64 {
        DRAW_RATE
    }

    fn get_p0_lose() -> f64 {
        (1.0 - DRAW_RATE) * (1.0 - elo_to_prob(ELO_0))
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
            Outcome::Win => self.LLR += f64::ln(Self::get_p1_win() / Self::get_p0_win()),
            Outcome::Draw => self.LLR += f64::ln(Self::get_p1_draw() / Self::get_p0_draw()),
            Outcome::Lose => self.LLR += f64::ln(Self::get_p1_lose() / Self::get_p0_lose()),
        }
    }

    pub fn result(&self) -> SPRTResult {
        if self.LLR < Self::get_sprt_a() {
            return SPRTResult::SignificantNull;
        }
        if self.LLR > Self::get_sprt_b() {
            return SPRTResult::SignificantAlt;
        }
        return SPRTResult::NotSignificant;
    }
}
fn main() {
    let mut sprt = SPRT::new();

    while sprt.result() == SPRTResult::NotSignificant {
        // Generate a random opening for this pair
        let opening = generate_opening();

        // Game 1: Engine1 as White, Engine2 as Black
        let result1 = play_game(ENGINE1_PATH, ENGINE2_PATH, &opening);
        match result1 {
            GameResult::Win(Player::White) => sprt.update(Outcome::Win),
            GameResult::Win(Player::Black) => sprt.update(Outcome::Lose),
            GameResult::Draw => sprt.update(Outcome::Draw),
            GameResult::InProgress => unreachable!(),
        }

        // Game 2: Engine2 as White, Engine1 as Black
        let result2 = play_game(ENGINE2_PATH, ENGINE1_PATH, &opening);
        match result2 {
            GameResult::Win(Player::White) => sprt.update(Outcome::Lose),
            GameResult::Win(Player::Black) => sprt.update(Outcome::Win),
            GameResult::Draw => sprt.update(Outcome::Draw),
            GameResult::InProgress => unreachable!(),
        }

        println!(
            "WDL {} {} {} -> LLR {:.2} ({:.2}, {:.2})",
            sprt.total_wins,
            sprt.total_draws,
            sprt.total_losses,
            sprt.LLR,
            SPRT::get_sprt_a(),
            SPRT::get_sprt_b()
        );
    }

    match sprt.result() {
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
