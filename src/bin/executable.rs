use blok_rs::{
    board::{BoardState, StartPosition},
    mcts::MonteCarlo,
    movegen::generate_moves,
    nn::NNUE,
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

    let mut board = BoardState::new(StartPosition::Corner, NNUE);

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
    let mut mcts = MonteCarlo::new(NNUE, false);

    mcts.run_search_timeout(&board, THINK_DURATION_MS);
    let best_move = mcts.best_play().unwrap();
    println!("{}", best_move);
}
