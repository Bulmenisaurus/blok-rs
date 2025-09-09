use std::time::{Duration, Instant};

use crate::{
    board::{BoardState, GameResult, Player},
    minimax::transposition_table::{TranspositionTable, TranspositionTableEntry},
    movegen::{INVALID_MOVE, Move, NULL_MOVE, PIECE_DATA, generate_moves},
};

/// Used for the bounds of alpha-beta pruning
const SCORE_INFINITY: i32 = 1_000_000;

/// End of game score, if winning +max, if losing -max
const SCORE_MAX: i32 = 999_999;

const MAX_DEPTH: usize = 100;

pub struct Searcher {
    transposition_table: TranspositionTable,
    history: [u32; 14 * 14 * 21],
}

impl Searcher {
    pub fn new() -> Self {
        Self {
            transposition_table: TranspositionTable::new(),
            history: [0; 14 * 14 * 21],
        }
    }

    pub fn search_root(&mut self, state: &BoardState, timeout_ms: usize) -> u32 {
        let start_time = Instant::now();
        let end_time = start_time + Duration::from_millis(timeout_ms as u64);

        // the current best move, as found by the last full search
        let mut best_move = generate_moves(state)[0];

        for current_depth in 1..=MAX_DEPTH {
            let search = self.alpha_beta(
                state,
                -SCORE_INFINITY,
                SCORE_INFINITY,
                current_depth,
                current_depth,
                end_time,
            );

            let (search_score, search_move) = match search {
                Ok((score, m)) => (score, m),
                Err(()) => break,
            };

            eprintln!("depth {} score {}", current_depth, search_score);

            assert_ne!(best_move, INVALID_MOVE, "Best move is invalid");

            best_move = search_move;
        }

        return best_move;
    }

    fn alpha_beta(
        &mut self,
        state: &BoardState,
        alpha: i32,
        beta: i32,
        depth: usize,
        max_depth: usize,
        deadline: Instant,
    ) -> Result<(i32, u32), ()> {
        if Instant::now() > deadline {
            return Err(());
        }

        if state.is_game_over() {
            return Ok((self.game_over_eval(state), INVALID_MOVE));
        }

        if depth == 0 {
            return Ok((self.static_eval(state), INVALID_MOVE));
        }

        let mut alpha = alpha;

        let mut legal_moves = generate_moves(state);
        self.order_moves(&mut legal_moves);

        let mut best_score = -SCORE_INFINITY;
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
                    .alpha_beta(&new_state, -beta, -alpha, depth - 1, max_depth, deadline)?
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
                if m != NULL_MOVE {
                    let mov = Move::unpack(m);
                    // the deeper we go, the more we increase the history
                    let increasing_depth = max_depth - depth;
                    self.history
                        [mov.y as usize * 14 + mov.x as usize + mov.movetype as usize * 14 * 14] +=
                        increasing_depth as u32;
                }
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
            // order by history first, then by move type
            let history_score = self.history[self.move_history_idx(Move::unpack(*m))];

            let move_type_score = PIECE_DATA[Move::get_movetype(*m) as usize].len() as u32;

            history_score * 5 + move_type_score
        });

        // Descending order
        moves.reverse();
    }

    fn game_over_eval(&self, state: &BoardState) -> i32 {
        match state.game_result() {
            GameResult::Win(p) => {
                if p == state.player {
                    SCORE_MAX
                } else {
                    -SCORE_MAX
                }
            }
            GameResult::Draw => 0,
            GameResult::InProgress => unreachable!(),
        }
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
        let move_count = state.player_a_corner_moves.values().flatten().count() as i32;
        score * 100 + move_count
    }

    fn black_eval(&self, state: &BoardState) -> i32 {
        let score = state.score().player_b as i32;
        let move_count = state.player_b_corner_moves.values().flatten().count() as i32;

        score * 100 + move_count
    }

    fn move_history_idx(&self, mov: Move) -> usize {
        mov.movetype as usize * 14 * 14 + mov.y as usize * 14 + mov.x as usize
    }
}
