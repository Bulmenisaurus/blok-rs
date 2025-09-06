use std::{
    collections::HashMap,
    time::{Duration, Instant},
};

use crate::{
    board::{BoardState, Player},
    movegen::{Move, NULL_MOVE, PIECE_DATA, generate_moves},
};

#[derive(Clone)]
pub struct TranspositionTableEntry {
    pub score: i32,
    pub depth: usize,
    pub best_move: u32,
}

#[derive(Clone)]
pub struct TranspositionTable {
    entries: HashMap<u64, TranspositionTableEntry>,
}

impl TranspositionTable {
    pub fn new() -> Self {
        Self {
            entries: HashMap::new(),
        }
    }

    pub fn insert(&mut self, hash: u64, entry: TranspositionTableEntry) {
        self.entries.insert(hash, entry);
    }

    pub fn get(&self, hash: u64) -> Option<&TranspositionTableEntry> {
        self.entries.get(&hash)
    }
}

const SCORE_MIN: i32 = -1_000_000;
const SCORE_MAX: i32 = 1_000_000;

pub fn search(state: &BoardState, timeout_ms: usize) -> u32 {
    let start_time = Instant::now();
    let end_time = start_time + Duration::from_millis(timeout_ms as u64);
    let mut current_depth = 1;

    let mut transposition_table = TranspositionTable::new();

    // the current best move, as found by the last full search
    let mut best_move = generate_moves(state)[0];

    loop {
        eprintln!("Searching at depth: {}", current_depth);
        let legal_moves = generate_moves(state);

        let mut current_depth_best_score = SCORE_MIN;
        let mut current_depth_best_move = legal_moves[0];

        for m in legal_moves {
            let mut new_state = state.clone();
            new_state.do_move(m);
            let score = match alpha_beta(
                &new_state,
                SCORE_MIN,
                SCORE_MAX,
                current_depth,
                &mut transposition_table,
                end_time,
            ) {
                Ok(x) => -x,
                Err(()) => return best_move,
            };

            if score > current_depth_best_score {
                current_depth_best_score = score;
                current_depth_best_move = m;
            }
        }

        // only update the outer best score and depth move now, to make sure if the search is interrupted, we return the best move found at the last full depth searched

        best_move = current_depth_best_move;

        current_depth += 1;
    }
}

fn order_moves(moves: &mut Vec<u32>) {
    moves.sort_by_key(|m| {
        if *m == NULL_MOVE {
            return 0;
        }
        PIECE_DATA[Move::get_movetype(*m) as usize].len() as i32
    });
    moves.reverse();
}

fn alpha_beta(
    state: &BoardState,
    alpha: i32,
    beta: i32,
    depth: usize,
    tt: &mut TranspositionTable,
    deadline: Instant,
) -> Result<i32, ()> {
    if Instant::now() > deadline {
        return Err(());
    }

    if depth == 0 {
        return Ok(static_eval(state));
    }

    let mut alpha = alpha;

    let mut legal_moves = generate_moves(state);
    order_moves(&mut legal_moves);

    let mut best_score = SCORE_MIN;

    for m in legal_moves {
        let mut new_state = state.clone();
        new_state.do_move(m);

        // Only use the TT if it's at least as deep as the current depth
        let tt_entry = tt.get(new_state.hash).and_then(|entry| {
            if entry.depth >= depth {
                // if depth > 2 {
                //     eprintln!("TT hit at depth: {}", entry.depth);
                // }
                return Some(entry.score);
            } else {
                return None;
            }
        });

        let score: i32;

        if let Some(tt_score) = tt_entry {
            score = tt_score;
        } else {
            // otherwise, do a full search and store the result in the TT
            score = -alpha_beta(&new_state, -beta, -alpha, depth - 1, tt, deadline)?;

            tt.insert(
                new_state.hash,
                TranspositionTableEntry {
                    score,
                    depth: depth,
                    best_move: m,
                },
            );
        }

        if score > best_score {
            best_score = score;
        }
        if score > alpha {
            alpha = score;
        }

        if score >= beta {
            return Ok(score);
        }
    }

    Ok(best_score)
}

// from the persepective of the player to move
fn static_eval(state: &BoardState) -> i32 {
    let score = state.score();

    let person_to_move = match state.player {
        Player::White => 1,
        Player::Black => -1,
    };

    return person_to_move * (score.player_a as i32 - score.player_b as i32);
}
