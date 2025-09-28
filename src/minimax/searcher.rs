use std::time::{Duration, Instant};

use crate::{
    board::BoardState,
    minimax::transposition_table::{TTFlag, TranspositionTable, TranspositionTableEntry},
    movegen::{INVALID_MOVE, Move, NULL_MOVE, PIECE_DATA, generate_moves},
};

use crate::minimax::eval::eval;

/// Used for the bounds of alpha-beta pruning
const SCORE_INFINITY: i32 = 1_000_000;

const MAX_DEPTH: usize = 100;

pub struct Searcher {
    transposition_table: TranspositionTable,
    history: [u32; 2 * 8 * 14 * 14 * 21],
    nodes: u32,
    max_nodes: u32,
}

impl Searcher {
    pub fn new() -> Self {
        Self {
            transposition_table: TranspositionTable::new(),
            history: [0; 2 * 8 * 14 * 14 * 21],
            nodes: 0,
            max_nodes: u32::MAX,
        }
    }

    pub fn search_root(&mut self, state: &BoardState, timeout_ms: usize) -> u32 {
        let start_time = Instant::now();
        let end_time = start_time + Duration::from_millis(timeout_ms as u64);

        // the current best move, as found by the last full search
        let mut best_move = generate_moves(state)[0];
        let mut previous_score = 0;
        let mut previous_previous_score = 0;

        for current_depth in 1..=MAX_DEPTH {
            let search = self.aspiration_window(
                state,
                previous_previous_score,
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
            previous_previous_score = previous_score;
            previous_score = search_score;
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

    pub fn aspiration_window(
        &mut self,
        state: &BoardState,
        previous_score: i32,
        depth: usize,
        max_depth: usize,
        deadline: Instant,
    ) -> Result<(i32, u32), ()> {
        // println!("================");
        // println!("Previous score: {}", previous_score);
        // println!("Depth: {}", depth);
        // println!("================");

        let mut beta_margin = 20;
        let mut alpha_margin = 20;

        while depth > 3 && alpha_margin <= 500 && beta_margin <= 500 {
            let alpha = previous_score - alpha_margin;
            let beta = previous_score + beta_margin;

            let search_result = self.alpha_beta(state, alpha, beta, depth, max_depth, deadline)?;

            let score = search_result.0;
            let best_move = search_result.1;

            // if the score is exact, return
            if score > alpha && score < beta {
                // println!("Exact score: {} [{} {}]", score, alpha, beta);
                return Ok((score, best_move));
            }
            if score <= alpha {
                // println!("Score <= alpha: {} [{} {}]", score, alpha, beta);
                alpha_margin *= 2;
            }
            if score >= beta {
                // println!("Score >= beta: {} [{} {}]", score, alpha, beta);
                beta_margin *= 2;
            }
        }
        //?
        // println!("Gradually widening failed, doing full search");
        // otherwise, just do a full search if gradually widening failed
        let search_result = self.alpha_beta(
            state,
            -SCORE_INFINITY,
            SCORE_INFINITY,
            depth,
            max_depth,
            deadline,
        )?;
        //?
        // println!("Full search result: {:?}", search_result.0);

        let score = search_result.0;
        let best_move = search_result.1;

        Ok((score, best_move))
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

        let static_eval = eval(state);

        if state.is_game_over() || depth == 0 {
            return Ok((static_eval, INVALID_MOVE));
        }

        let root_node = depth == max_depth;
        let pv_node = alpha != beta - 1;

        // check tt for cutoffs
        // however, this should only be done for non-root nodes
        let mut tt_move = INVALID_MOVE;

        let tt_entry = self.transposition_table.get(state.hash);

        // This tt entry exists and is at least as deep as the current depth
        if let Some(tt_entry) = tt_entry {
            if !pv_node && !root_node && tt_entry.depth >= depth {
                // Make sure that this tt entry is actually useful
                if tt_entry.flag == TTFlag::Exact
                    || tt_entry.flag == TTFlag::LowerBound && tt_entry.score >= beta
                    || tt_entry.flag == TTFlag::UpperBound && tt_entry.score < alpha
                {
                    return Ok((tt_entry.score, tt_entry.best_move));
                }
            }
            //?
            // if root_node {
            //     println!("TT move: {}", tt_entry.best_move);
            // }

            // otherwise, we can still use the tt move for ordering

            tt_move = tt_entry.best_move;
        }

        // RFP: aggresively prunes moves that we predict will not be better than beta
        if !pv_node && max_depth > 2 {
            let rfp_eval = static_eval - 100 * depth as i32;
            let rfp_depth = 3;
            if depth <= rfp_depth && rfp_eval > beta {
                return Ok((rfp_eval, INVALID_MOVE));
            }
        }

        let mut alpha = alpha;

        let mut legal_moves = generate_moves(state);
        // let amount = legal_moves.len();
        self.order_moves(&mut legal_moves, tt_move);

        let mut best_score = -SCORE_INFINITY;
        let mut hash_bound = TTFlag::UpperBound;
        let mut best_move = legal_moves[0];

        let mut moves_played = 0;

        for m in legal_moves {
            // Late move pruning - skip moves that are too late in move ordering
            // https://www.chessprogramming.org/Futility_Pruning#Move_Count_Based_Pruning
            // Much more aggresive version of LMR
            let lmp_moves_threshold = 10 + 5 * depth * depth;
            if !root_node && moves_played >= lmp_moves_threshold && max_depth > 2 {
                break;
            }

            let mut new_state = state.clone();
            new_state.do_move(m);

            moves_played += 1;
            //?
            // if root_node {
            //     println!("Playing move: {} {}/{}", m, moves_played, amount);
            // }

            let score;

            if moves_played == 1 {
                // search at full depth and window
                score = -self
                    .alpha_beta(&new_state, -beta, -alpha, depth - 1, max_depth, deadline)?
                    .0;
            } else {
                // Super simple LMR, search later moves at shallower depth
                let mut lmr = 0;
                if depth >= 3 && moves_played >= 16 {
                    lmr = 2;
                }

                // null search window
                let null_window_score = -self
                    .alpha_beta(
                        &new_state,
                        -alpha - 1,
                        -alpha,
                        depth - 1 - lmr,
                        max_depth,
                        deadline,
                    )?
                    .0;
                if null_window_score > alpha {
                    score = -self
                        .alpha_beta(&new_state, -beta, -alpha, depth - 1, max_depth, deadline)?
                        .0;
                } else {
                    score = null_window_score;
                }
            }

            if score > best_score {
                best_score = score;
                best_move = m;
            }
            if score > alpha {
                // if root_node {
                //     println!("Raised alpha {} -> {}", alpha, score);
                // }
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
                    self.history[self.move_history_idx(mov)] += increasing_depth as u32;
                }
                break;
            }
        }

        let tt_new_entry = TranspositionTableEntry {
            score: best_score,
            depth,
            flag: hash_bound,
            best_move,
        };

        //?
        // if root_node {
        //     let tt_entry = self.transposition_table.get(state.hash);
        //     if let Some(tt_entry) = tt_entry {
        //         println!("My entry: {:?}", tt_new_entry);
        //         println!("TT entry: {:?}", tt_entry);
        //     }
        // }

        // store the result in the TT
        self.transposition_table.insert(state.hash, tt_new_entry);

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

    fn move_history_idx(&self, mov: Move) -> usize {
        mov.player as usize * 8 * 14 * 14 * 21
            + mov.orientation as usize * 14 * 14 * 21
            + mov.movetype as usize * 14 * 14
            + mov.y as usize * 14
            + mov.x as usize
    }
}
