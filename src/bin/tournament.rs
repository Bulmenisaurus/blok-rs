use blok_rs::{
    board::{BoardState, GameResult, Player, StartPosition},
    movegen::generate_moves,
};
use rand::{rng, seq::IndexedRandom};
use skillratings::{
    Outcomes,
    glicko2::{Glicko2Config, Glicko2Rating, glicko2},
};

use itertools::Itertools;
use std::process::{Command, Stdio};

const OPENING_PLIES: usize = 6;
const NUM_GAMES: usize = 15 * 200;
struct TournamentPlayer {
    rating: Glicko2Rating,
    name: &'static str,
    engine_path: &'static str,
}
fn main() {
    let mut players: Vec<TournamentPlayer> = vec![
        TournamentPlayer {
            name: "AB",
            rating: Glicko2Rating::new(),
            engine_path: "./executables/ab-latest",
        },
        TournamentPlayer {
            name: "MCTS",
            rating: Glicko2Rating::new(),
            engine_path: "./executables/mcts-classic-latest",
        },
        TournamentPlayer {
            name: "MCTS (NN)",
            rating: Glicko2Rating::new(),
            engine_path: "./executables/mcts-nn-latest",
        },
        TournamentPlayer {
            name: "MCTS (PUCT)",
            rating: Glicko2Rating::new(),
            engine_path: "./executables/mcts-puct-latest",
        },
    ];

    let config = Glicko2Config::default();

    let mut matches = get_matches(players.len());
    for i in 0..NUM_GAMES {
        let (a, b) = matches.next().unwrap();

        let (left, right) = players.split_at_mut(b);
        let player_a = &mut left[a];
        let player_b = &mut right[0];

        println!(
            "[{}/{}] Playing game pair between {} and {}",
            i + 1,
            NUM_GAMES,
            player_a.name,
            player_b.name
        );

        play_game_pair(player_a, player_b, &config);
        pretty_print_ratings(&players);
    }
}

fn get_matches(players: usize) -> impl Iterator<Item = (usize, usize)> {
    (0..players).combinations(2).map(|m| (m[0], m[1])).cycle()
}

/// Given two engines, run a full game pair with a random opening, and update the ratings
fn play_game_pair(
    player_a: &mut TournamentPlayer,
    player_b: &mut TournamentPlayer,
    config: &Glicko2Config,
) {
    let opening = generate_opening();

    // the first game pair is with player_a as white, player_b as black
    let game_pair_one_result = play_game(player_a.engine_path, player_b.engine_path, &opening);
    let game_pair_one_result = match game_pair_one_result {
        GameResult::Win(Player::White) => Outcomes::WIN,
        GameResult::Win(Player::Black) => Outcomes::LOSS,
        GameResult::Draw => Outcomes::DRAW,

        GameResult::InProgress => unreachable!(),
    };

    // update the ratings
    let updates_players = glicko2(
        &player_a.rating,
        &player_b.rating,
        &game_pair_one_result,
        config,
    );

    player_a.rating = updates_players.0;
    player_b.rating = updates_players.1;

    // now, play the opposite game pair, with player_b as white, player_a as black
    let game_pair_two_result = play_game(player_b.engine_path, player_a.engine_path, &opening);
    let game_pair_two_result = match game_pair_two_result {
        GameResult::Win(Player::White) => Outcomes::WIN,
        GameResult::Win(Player::Black) => Outcomes::LOSS,
        GameResult::Draw => Outcomes::DRAW,
        GameResult::InProgress => unreachable!(),
    };

    // update the ratings
    let updates_players = glicko2(
        &player_b.rating,
        &player_a.rating,
        &game_pair_two_result,
        config,
    );

    player_b.rating = updates_players.0;
    player_a.rating = updates_players.1;
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

fn pretty_print_ratings(players: &[TournamentPlayer]) {
    for player in players
        .iter()
        .sorted_by(|a, b| a.rating.rating.partial_cmp(&b.rating.rating).unwrap())
        .rev()
    {
        println!("{}: {}", player.name, player.rating.rating.round());
    }
}
