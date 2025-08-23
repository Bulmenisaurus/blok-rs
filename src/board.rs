use std::collections::HashMap;

use crate::movegen::{
    CORNER_ATTACHERS_DATA, CORNERS_DATA, Move, NULL_MOVE, ORIENTATIONS_BITBOARD_DATA, PIECE_DATA,
    get_legal_moves_from, is_move_legal,
};

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Player {
    White = 0,
    Black = 1,
}

impl Player {
    pub fn other(&self) -> Player {
        match self {
            Player::White => Player::Black,
            Player::Black => Player::White,
        }
    }
}

#[derive(serde::Deserialize, Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Coord {
    pub x: u8,
    pub y: u8,
}

#[derive(serde::Deserialize, Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct CoordOffset {
    pub x: i8,
    pub y: i8,
}

impl Coord {
    pub fn in_bounds(&self) -> bool {
        self.x < 14 && self.y < 14
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StartPosition {
    Middle,
    Corner,
    MiddleBlokee,
}

pub fn get_start_position_coord(start_position: StartPosition) -> (Coord, Coord) {
    match start_position {
        StartPosition::Middle => (Coord { x: 4, y: 4 }, Coord { x: 9, y: 9 }),
        StartPosition::Corner => (Coord { x: 0, y: 0 }, Coord { x: 13, y: 13 }),
        StartPosition::MiddleBlokee => (Coord { x: 6, y: 7 }, Coord { x: 7, y: 6 }),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Score {
    pub player_a: u32,
    pub player_b: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GameResult {
    InProgress,
    PlayerAWon,
    PlayerBWon,
    Draw,
}

#[derive(Debug, Clone)]
pub struct BoardState {
    // Player to move
    pub player: Player,

    // Remaining pieces for each player, as a bitmask
    pub player_a_remaining: u32,
    pub player_b_remaining: u32,

    // Bitboards for tiles placed
    pub player_a_bit_board: [u32; 14],
    pub player_b_bit_board: [u32; 14],

    pub start_position: StartPosition,

    // How many null moves have been made (>= 2 in a row is game end)
    pub null_move_counter: u8,

    // Cached corner moves
    pub player_a_corner_moves: HashMap<Coord, Vec<u32>>,
    pub player_b_corner_moves: HashMap<Coord, Vec<u32>>,
}

impl BoardState {
    pub fn new(start_position: StartPosition) -> Self {
        Self {
            player: Player::White,
            player_a_remaining: 0x1fffff,
            player_b_remaining: 0x1fffff,
            player_a_bit_board: [0; 14],
            player_b_bit_board: [0; 14],
            null_move_counter: 0,
            start_position,
            player_a_corner_moves: HashMap::new(),
            player_b_corner_moves: HashMap::new(),
        }
    }

    pub fn is_game_over(&self) -> bool {
        self.null_move_counter >= 2
    }

    pub fn score(&self) -> Score {
        let player_a_remaining = self.player_a_remaining;
        let mut player_a_score = 0;
        for i in 0..21 {
            if player_a_remaining & (1 << i) == 0 {
                player_a_score += PIECE_DATA[i as usize].len();
            }
        }

        let player_b_remaining = self.player_b_remaining;
        let mut player_b_score = 0;
        for i in 0..21 {
            if player_b_remaining & (1 << i) == 0 {
                player_b_score += PIECE_DATA[i as usize].len();
            }
        }

        Score {
            player_a: player_a_score as u32,
            player_b: player_b_score as u32,
        }
    }

    pub fn game_result(&self) -> GameResult {
        if !self.is_game_over() {
            return GameResult::InProgress;
        }

        let score = self.score();
        match score.player_a.cmp(&score.player_b) {
            std::cmp::Ordering::Greater => GameResult::PlayerAWon,
            std::cmp::Ordering::Less => GameResult::PlayerBWon,
            std::cmp::Ordering::Equal => GameResult::Draw,
        }
    }

    // change states, incrementally update move cache
    pub fn do_move(&mut self, board_move: u32) {
        if board_move == NULL_MOVE {
            self.null_move_counter += 1;
            self.skip_turn();

            // Take ownership of the cached moves, filter them, then reassign
            let cached_moves = if self.player == Player::White {
                std::mem::take(&mut self.player_a_corner_moves)
            } else {
                std::mem::take(&mut self.player_b_corner_moves)
            };

            let filtered_moves: HashMap<Coord, Vec<u32>> = cached_moves
                .into_iter()
                .map(|(coord, moves)| {
                    let legal_moves: Vec<u32> = moves
                        .into_iter()
                        .filter(|m| is_move_legal(self, *m))
                        .collect();
                    (coord, legal_moves)
                })
                .collect();

            if self.player == Player::White {
                self.player_a_corner_moves = filtered_moves;
            } else {
                self.player_b_corner_moves = filtered_moves;
            }

            return;
        }

        self.null_move_counter = 0;
        let mov = Move::unpack(board_move);

        // remove this move from the pool
        if mov.player == (Player::White as u8) {
            self.player_a_remaining &= !(1 << mov.movetype);
        } else {
            self.player_b_remaining &= !(1 << mov.movetype);
        }

        let my_bitboard = if mov.player == (Player::White as u8) {
            &mut self.player_a_bit_board
        } else {
            &mut self.player_b_bit_board
        };

        // update bitboards
        let piece_bitboard =
            &ORIENTATIONS_BITBOARD_DATA[mov.movetype as usize][mov.orientation as usize];

        for bb_y in 0..piece_bitboard.len() {
            my_bitboard[mov.y as usize + bb_y] |= piece_bitboard[bb_y] << mov.x;
        }

        // Update the corner data.
        // 1. For each of the corners of the placed piece, clear the corner moves for that corner (because there cannot be any moves there anymore)
        // 2. Filter out the moves that are no longer valid
        // 3. Add the new moves to the corner moves

        let corners = &CORNERS_DATA[mov.movetype as usize][mov.orientation as usize];
        for corner in corners {
            let absolute_corner = Coord {
                x: corner.x + mov.x,
                y: corner.y + mov.y,
            };

            // delete all the moves for this corner
            self.player_a_corner_moves.remove(&absolute_corner);
            self.player_b_corner_moves.remove(&absolute_corner);
        }

        let my_remaining_pieces = if self.player == Player::White {
            self.player_a_remaining
        } else {
            self.player_b_remaining
        };

        let corner_attachers =
            &CORNER_ATTACHERS_DATA[mov.movetype as usize][mov.orientation as usize];
        for corner in corner_attachers {
            if (corner.x < 0 && -corner.x > mov.x as i8)
                || (corner.y < 0 && -corner.y > mov.y as i8)
            {
                continue;
            }

            let absolute_corner = Coord {
                x: (corner.x + mov.x as i8) as u8,
                y: (corner.y + mov.y as i8) as u8,
            };

            if !absolute_corner.in_bounds() {
                continue;
            }

            if self.player == Player::White {
                if self.player_a_corner_moves.contains_key(&absolute_corner) {
                    continue;
                }
            } else if self.player_b_corner_moves.contains_key(&absolute_corner) {
                continue;
            }

            let mut legal_moves: Vec<u32> = Vec::new();

            for unplaced_piece in 0..21 {
                if my_remaining_pieces & (1 << unplaced_piece) == 0 {
                    continue;
                }

                let movetype = unplaced_piece as u8;

                legal_moves.extend(get_legal_moves_from(absolute_corner, movetype, self));
            }

            if self.player == Player::White {
                self.player_a_corner_moves
                    .insert(absolute_corner, legal_moves);
            } else {
                self.player_b_corner_moves
                    .insert(absolute_corner, legal_moves);
            }
        }

        self.skip_turn();

        // filter opponent's moves (now the player to move)
        let opponent_cached_moves: Vec<Coord> = if self.player == Player::White {
            self.player_a_corner_moves.keys().cloned().collect()
        } else {
            self.player_b_corner_moves.keys().cloned().collect()
        };

        for coord in opponent_cached_moves {
            let old_moves = if self.player == Player::White {
                self.player_a_corner_moves.get_mut(&coord).unwrap().clone()
            } else {
                self.player_b_corner_moves.get_mut(&coord).unwrap().clone()
            };

            let new_moves: Vec<u32> = old_moves
                .into_iter()
                .filter(|m| is_move_legal(self, *m))
                .collect();

            if self.player == Player::White {
                self.player_a_corner_moves.insert(coord, new_moves);
            } else {
                self.player_b_corner_moves.insert(coord, new_moves);
            }
        }
    }

    pub fn skip_turn(&mut self) {
        self.player = self.player.other();
    }
}
