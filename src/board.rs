use serde::Deserialize;
use std::collections::HashMap;

use crate::movegen::{
    CORNER_ATTACHERS_DATA, CORNERS_DATA, Move, NULL_MOVE, ORIENTATION_DATA,
    ORIENTATIONS_BITBOARD_DATA, get_legal_moves_from, is_move_legal,
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
    middle,
    corner,
    middleBlokee,
}

pub fn get_start_position_coord(start_position: StartPosition) -> (Coord, Coord) {
    match start_position {
        StartPosition::middle => (Coord { x: 4, y: 4 }, Coord { x: 9, y: 9 }),
        StartPosition::corner => (Coord { x: 0, y: 0 }, Coord { x: 13, y: 13 }),
        StartPosition::middleBlokee => (Coord { x: 6, y: 7 }, Coord { x: 7, y: 6 }),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Score {
    playerA: u32,
    playerB: u32,
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
    // Pieces on the board, packed into 32 bits
    pub pieces: Vec<u32>,
    // Player to move
    pub player: Player,

    // Remaining pieces for each player, as a bitmask
    pub playerARemaining: u32,
    pub playerBRemaining: u32,

    // Bitboards for tiles placed
    pub playerABitBoard: [u32; 14],
    pub playerBBitBoard: [u32; 14],

    pub startPosition: StartPosition,

    // How many null moves have been made (>= 2 in a row is game end)
    pub nullMoveCounter: u8,

    // Cached corner moves
    pub playerACornerMoves: HashMap<Coord, Vec<u32>>,
    pub playerBCornerMoves: HashMap<Coord, Vec<u32>>,
}

impl BoardState {
    pub fn new(start_position: StartPosition) -> Self {
        Self {
            pieces: Vec::new(),
            player: Player::White,
            playerARemaining: 0x1fffff,
            playerBRemaining: 0x1fffff,
            playerABitBoard: [0; 14],
            playerBBitBoard: [0; 14],
            nullMoveCounter: 0,
            startPosition: start_position,
            playerACornerMoves: HashMap::new(),
            playerBCornerMoves: HashMap::new(),
        }
    }

    pub fn is_game_over(&self) -> bool {
        self.nullMoveCounter >= 2
    }

    pub fn score(&self) -> Score {
        let playerAScore: usize = self
            .pieces
            .iter()
            .filter(|m| Move::get_player(**m) == 0)
            .map(|m| ORIENTATION_DATA[Move::get_movetype(*m) as usize][0].len())
            .sum();

        let playerBScore: usize = self
            .pieces
            .iter()
            .filter(|m| Move::get_player(**m) == 1)
            .map(|m| ORIENTATION_DATA[Move::get_movetype(*m) as usize][0].len())
            .sum();

        Score {
            playerA: playerAScore as u32,
            playerB: playerBScore as u32,
        }
    }

    pub fn game_result(&self) -> GameResult {
        if !self.is_game_over() {
            return GameResult::InProgress;
        }

        let score = self.score();
        if score.playerA > score.playerB {
            return GameResult::PlayerAWon;
        } else if score.playerA < score.playerB {
            return GameResult::PlayerBWon;
        } else {
            return GameResult::Draw;
        }
    }

    // change states, incrementally update move cache
    pub fn doMove(&mut self, boardMove: u32) {
        if boardMove == NULL_MOVE {
            self.nullMoveCounter += 1;
            self.skipTurn();

            // Take ownership of the cached moves, filter them, then reassign
            let cached_moves = if self.player == Player::White {
                std::mem::take(&mut self.playerACornerMoves)
            } else {
                std::mem::take(&mut self.playerBCornerMoves)
            };

            let filtered_moves: HashMap<Coord, Vec<u32>> = cached_moves
                .into_iter()
                .map(|(coord, moves)| {
                    let legal_moves: Vec<u32> = moves
                        .into_iter()
                        .filter(|m| is_move_legal(&self, *m))
                        .collect();
                    (coord, legal_moves)
                })
                .collect();

            if self.player == Player::White {
                self.playerACornerMoves = filtered_moves;
            } else {
                self.playerBCornerMoves = filtered_moves;
            }

            return;
        }

        self.nullMoveCounter = 0;
        self.pieces.push(boardMove);
        let mov = Move::unpack(boardMove);

        // remove this move from the pool
        if mov.player == (Player::White as u8) {
            self.playerARemaining &= !(1 << mov.movetype);
        } else {
            self.playerBRemaining &= !(1 << mov.movetype);
        }

        let my_bitboard = if mov.player == (Player::White as u8) {
            &mut self.playerABitBoard
        } else {
            &mut self.playerBBitBoard
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
            self.playerACornerMoves.remove(&absolute_corner);
            self.playerBCornerMoves.remove(&absolute_corner);
        }

        let my_remaining_pieces = if self.player == Player::White {
            self.playerARemaining
        } else {
            self.playerBRemaining
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
                if self.playerACornerMoves.contains_key(&absolute_corner) {
                    continue;
                }
            } else {
                if self.playerBCornerMoves.contains_key(&absolute_corner) {
                    continue;
                }
            }

            let mut legal_moves: Vec<u32> = Vec::new();

            for unplaced_piece in 0..21 {
                if my_remaining_pieces & (1 << unplaced_piece) == 0 {
                    continue;
                }

                let movetype = unplaced_piece as u8;
                let orientation = mov.orientation;

                legal_moves.extend(get_legal_moves_from(absolute_corner, movetype, self));
            }

            if self.player == Player::White {
                self.playerACornerMoves.insert(absolute_corner, legal_moves);
            } else {
                self.playerBCornerMoves.insert(absolute_corner, legal_moves);
            }
        }

        self.skipTurn();

        // filter opponent's moves (now the player to move)
        let opponent_cached_moves: Vec<Coord> = if self.player == Player::White {
            self.playerACornerMoves.keys().cloned().collect()
        } else {
            self.playerBCornerMoves.keys().cloned().collect()
        };

        for coord in opponent_cached_moves {
            let old_moves = if self.player == Player::White {
                self.playerACornerMoves.get_mut(&coord).unwrap().clone()
            } else {
                self.playerBCornerMoves.get_mut(&coord).unwrap().clone()
            };

            let new_moves: Vec<u32> = old_moves
                .into_iter()
                .filter(|m| is_move_legal(self, *m))
                .collect();

            if self.player == Player::White {
                self.playerACornerMoves.insert(coord, new_moves);
            } else {
                self.playerBCornerMoves.insert(coord, new_moves);
            }
        }
    }

    pub fn skipTurn(&mut self) {
        self.player = self.player.other();
    }

    pub fn hash(&self) -> String {
        let moves = self
            .pieces
            .iter()
            .map(|m| m.to_string())
            .collect::<Vec<_>>()
            .join("/");

        format!("{}+{}", moves, self.nullMoveCounter)
    }
}
