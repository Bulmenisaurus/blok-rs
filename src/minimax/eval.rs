use crate::board::{BoardState, GameResult, Player};

/// End of game score, if winning +max, if losing -max
pub const SCORE_MAX: i32 = 999_999;

/// Side-to-move bonus. Odd/even search scores swing ~700 without this;
/// half of that puts even and odd depths on a comparable scale so RFP
/// and aspiration can use sane margins.
const TEMPO: i32 = 350;

pub fn eval(state: &BoardState) -> i32 {
    match state.game_result() {
        GameResult::Win(p) => {
            if p == state.player {
                SCORE_MAX
            } else {
                -SCORE_MAX
            }
        }
        GameResult::Draw => 0,
        GameResult::InProgress => static_eval(state),
    }
}

// from the persepective of the player to move
fn static_eval(state: &BoardState) -> i32 {
    let person_to_move = match state.player {
        Player::White => 1,
        Player::Black => -1,
    };

    person_to_move * (white_eval(state) - black_eval(state)) + TEMPO
}

fn white_eval(state: &BoardState) -> i32 {
    let score = state.score().player_a as i32;

    let move_count_2 = state
        .player_a_corner_moves_info
        .values()
        .map(|info| info.moves.count_ones() as i32)
        .sum::<i32>();

    score * 100 + move_count_2
}

fn black_eval(state: &BoardState) -> i32 {
    let score = state.score().player_b as i32;
    let move_count_2 = state
        .player_b_corner_moves_info
        .values()
        .map(|info| info.moves.count_ones() as i32)
        .sum::<i32>();

    score * 100 + move_count_2
}
