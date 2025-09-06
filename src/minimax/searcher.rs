use std::time::{Duration, Instant};

use crate::{
    board::{BoardState, GameResult, Player},
    minimax::transposition_table::{TranspositionTable, TranspositionTableEntry},
    movegen::{INVALID_MOVE, Move, NULL_MOVE, PIECE_DATA, generate_moves},
};

const SCORE_MIN: i32 = -1_000_000;
const SCORE_MAX: i32 = 1_000_000;

pub struct Searcher {
    transposition_table: TranspositionTable,
}

impl Searcher {
    pub fn new() -> Self {
        Self {
            transposition_table: TranspositionTable::new(),
        }
    }

    pub fn search_root(&mut self, state: &BoardState, timeout_ms: usize) -> u32 {
        let start_time = Instant::now();
        let end_time = start_time + Duration::from_millis(timeout_ms as u64);

        // the current best move, as found by the last full search
        let mut best_move = generate_moves(state)[0];
        let mut current_depth = 1;

        loop {
            eprintln!("Searching at depth: {}", current_depth);

            let search = self.alpha_beta(state, SCORE_MIN, SCORE_MAX, current_depth, end_time);

            best_move = match search {
                Ok((_, m)) => m,
                Err(()) => return best_move,
            };

            assert_ne!(best_move, INVALID_MOVE, "Best move is invalid");

            current_depth += 1;
        }
    }

    fn alpha_beta(
        &mut self,
        state: &BoardState,
        alpha: i32,
        beta: i32,
        depth: usize,
        deadline: Instant,
    ) -> Result<(i32, u32), ()> {
        if Instant::now() > deadline {
            return Err(());
        }

        if state.is_game_over() {
            let score = match (state.game_result(), state.player) {
                (GameResult::PlayerAWon, Player::White)
                | (GameResult::PlayerBWon, Player::Black) => 999_999,
                (GameResult::PlayerBWon, Player::White)
                | (GameResult::PlayerAWon, Player::Black) => -999_999,
                (GameResult::Draw, _) => 0,
                (GameResult::InProgress, _) => unreachable!(),
            };
            return Ok((score, INVALID_MOVE));
        }

        if depth == 0 {
            return Ok((self.static_eval(state), INVALID_MOVE));
        }

        let mut alpha = alpha;

        let mut legal_moves = generate_moves(state);
        self.order_moves(&mut legal_moves);

        let mut best_score = SCORE_MIN;
        let mut best_move = legal_moves[0];

        for m in legal_moves {
            let mut new_state = state.clone();
            new_state.do_move(m);

            // Only use the TT if it's at least as deep as the current depth
            let tt_entry = self
                .transposition_table
                .get(new_state.hash)
                .and_then(|entry| {
                    if entry.depth >= depth {
                        Some(entry.score)
                    } else {
                        None
                    }
                });

            let score: i32;

            if let Some(tt_score) = tt_entry {
                score = tt_score;
            } else {
                // otherwise, do a full search and store the result in the TT
                score = -self
                    .alpha_beta(&new_state, -beta, -alpha, depth - 1, deadline)?
                    .0;

                self.transposition_table
                    .insert(new_state.hash, TranspositionTableEntry { score, depth });
            }

            if score > best_score {
                best_score = score;
                best_move = m;
            }
            if score > alpha {
                alpha = score;
            }

            // beta cutoff shouldn't ever be used?
            if score >= beta {
                return Ok((score, best_move));
            }
        }

        Ok((best_score, best_move))
    }

    fn order_moves(&self, moves: &mut [u32]) {
        moves.sort_by_key(|m| {
            if *m == NULL_MOVE {
                return 0;
            }
            PIECE_DATA[Move::get_movetype(*m) as usize].len() as i32
        });
        moves.reverse();
    }

    // from the persepective of the player to move
    fn static_eval(&self, state: &BoardState) -> i32 {
        let person_to_move = match state.player {
            Player::White => 1,
            Player::Black => -1,
        };

        person_to_move * (self.white_eval(state) - self.black_eval(state))
    }

    fn white_eval(&self, state: &BoardState) -> i32 {
        let score = state.score().player_a as i32;
        let corner_bonus = state.player_a_corner_moves.values().flatten().count() as i32;
        score * 100 + corner_bonus

        // return score;
    }

    fn black_eval(&self, state: &BoardState) -> i32 {
        let score = state.score().player_b as i32;
        let corner_bonus = state.player_b_corner_moves.values().flatten().count() as i32;
        score * 100 + corner_bonus

        // return score;
    }
}
