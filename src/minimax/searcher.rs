use std::time::{Duration, Instant};

use crate::{
    board::{BoardState, GameResult, Player},
    minimax::transposition_table::{TTFlag, TranspositionTable, TranspositionTableEntry},
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
    nodes: u32,
    max_nodes: u32,
}

impl Searcher {
    pub fn new() -> Self {
        Self {
            transposition_table: TranspositionTable::new(),
            history: [0; 14 * 14 * 21],
            nodes: 0,
            max_nodes: u32::MAX,
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

            eprintln!(
                "depth {} bestmove {} score {} nodes {}",
                current_depth, search_move, search_score, self.nodes,
            );

            assert_ne!(best_move, INVALID_MOVE, "Best move is invalid");

            best_move = search_move;
        }

        best_move
    }

    /// Search for the best move, but stop after a certain number of nodes
    /// Useful for testing the engine's performance at a certain number of nodes
    pub fn search_root_nodes(&mut self, state: &BoardState, nodes: u32) -> u32 {
        self.max_nodes = nodes;
        // an hour
        let timeout_ms = 3_600_000;
        self.search_root(state, timeout_ms)
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
        self.nodes += 1;

        if self.nodes > self.max_nodes {
            return Err(());
        }

        if Instant::now() > deadline {
            return Err(());
        }

        if state.is_game_over() {
            return Ok((self.game_over_eval(state), INVALID_MOVE));
        }

        if depth == 0 {
            return Ok((self.static_eval(state), INVALID_MOVE));
        }

        let root_node = depth == max_depth;

        // check tt for cutoffs
        // however, this should only be done for non-root nodes
        let mut tt_move = INVALID_MOVE;
        if !root_node {
            let tt_entry = self.transposition_table.get(state.hash);

            // This tt entry exists and is at least as deep as the current depth
            if let Some(tt_entry) = tt_entry {
                if tt_entry.depth >= depth {
                    // Make sure that this tt entry is actually useful
                    if tt_entry.flag == TTFlag::Exact
                        || tt_entry.flag == TTFlag::LowerBound && tt_entry.score >= beta
                        || tt_entry.flag == TTFlag::UpperBound && tt_entry.score < alpha
                    {
                        return Ok((tt_entry.score, INVALID_MOVE));
                    }
                }

                // otherwise, we can still use the tt move for ordering

                tt_move = tt_entry.best_move;
            }
        }

        // RFP: aggresively prunes moves that we predict will not be better than beta
        let static_eval = self.static_eval(state);

        let rfp_eval = static_eval - 100 * depth as i32;
        let rfp_depth = 3;
        if depth <= rfp_depth && rfp_eval > beta {
            return Ok((rfp_eval, INVALID_MOVE));
        }

        let mut alpha = alpha;

        let mut legal_moves = generate_moves(state);
        self.order_moves(&mut legal_moves, tt_move);

        let mut best_score = -SCORE_INFINITY;
        let mut hash_bound = TTFlag::UpperBound;
        let mut best_move = legal_moves[0];

        for m in legal_moves {
            let mut new_state = state.clone();
            new_state.do_move(m);

            // otherwise, do a full search and store the result in the TT
            let score = -self
                .alpha_beta(&new_state, -beta, -alpha, depth - 1, max_depth, deadline)?
                .0;

            if score > best_score {
                best_score = score;
                best_move = m;
            }
            if score > alpha {
                alpha = score;
                hash_bound = TTFlag::Exact;
            }

            // Fail high cutoff
            if alpha >= beta {
                hash_bound = TTFlag::LowerBound;
                if m != NULL_MOVE {
                    let mov = Move::unpack(m);
                    // the deeper we go, the more we increase the history
                    let increasing_depth = max_depth - depth;
                    self.history
                        [mov.y as usize * 14 + mov.x as usize + mov.movetype as usize * 14 * 14] +=
                        increasing_depth as u32;
                }
                break;
            }
        }

        // store the result in the TT
        self.transposition_table.insert(
            state.hash,
            TranspositionTableEntry {
                score: best_score,
                depth,
                flag: hash_bound,
                best_move,
            },
        );

        Ok((best_score, best_move))
    }

    fn order_moves(&self, moves: &mut [u32], tt_move: u32) {
        moves.sort_by_key(|m| {
            if *m == NULL_MOVE {
                return 0;
            }

            // always put the tt move first
            if *m == tt_move {
                return 999999;
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
