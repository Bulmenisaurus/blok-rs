use blok_rs::{
    board::{BoardState, StartPosition},
    minimax,
    movegen::generate_moves,
};
use std::env;

const THINK_DURATION_MS: usize = 1000;

fn main() {
    // Read moves from command line argument
    let args: Vec<String> = env::args().collect();
    if args.len() != 2 {
        eprintln!("Usage: {} \"<move1> <move2> ...\"", args[0]);
        std::process::exit(1);
    }
    let move_strs: Vec<&str> = args[1].split_whitespace().collect();

    let mut board = BoardState::new(StartPosition::Corner);

    // Play the moves from the command line
    for mstr in move_strs {
        // Try to parse the move from string
        let legal_moves = generate_moves(&board);
        let parsed_move = legal_moves.iter().find(|&&m| m.to_string() == mstr);
        match parsed_move {
            Some(&m) => board.do_move(m),
            None => {
                eprintln!("Illegal or unrecognized move: {}", mstr);
                std::process::exit(1);
            }
        }
    }

    // Now think and print the best response move

    let best_move = minimax::search(&board, THINK_DURATION_MS);
    println!("{}", best_move);
}
