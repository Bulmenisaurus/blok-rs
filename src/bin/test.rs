use blok_rs::board::{BoardState, Player, StartPosition};

use blok_rs::movegen::generate_moves;

fn main() {
    // Create a new board in the default start position
    let mut board = BoardState::new(StartPosition::Corner);

    let moves = generate_moves(&board);

    let mut moves_with_evals: Vec<(u32, i32)> = moves
        .iter()
        .map(|m| {
            let mut new_board = board.clone();
            new_board.do_move(*m);
            let eval = if new_board.player == Player::White {
                blok_rs::nn::NNUE.evaluate(
                    &new_board.player_a_accumulator,
                    &new_board.player_b_accumulator,
                )
            } else {
                blok_rs::nn::NNUE.evaluate(
                    &new_board.player_b_accumulator,
                    &new_board.player_a_accumulator,
                )
            };
            (*m, eval)
        })
        .collect();

    moves_with_evals.sort_by_key(|(_, eval)| *eval);

    for (m, eval) in moves_with_evals {
        println!("Move: {}, Eval: {}", m, eval);
    }
}
