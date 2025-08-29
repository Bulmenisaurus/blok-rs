use std::collections::HashMap;

use crate::{
    movegen::{
        Move, NULL_MOVE, ORIENTATION_DATA, PIECE_DATA, update_move_cache,
        update_move_cache_from_null_move,
    },
    nn::{Accumulator, NNUE, Network},
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

#[derive(Clone)]
pub struct BoardState {
    // Player to move
    pub player: Player,

    // Remaining pieces for each player, as a bitmask
    pub player_a_remaining: u32,
    pub player_b_remaining: u32,

    // Bitboards for tiles placed
    pub player_a_bit_board: [u16; 14],
    pub player_b_bit_board: [u16; 14],

    pub start_position: StartPosition,

    // How many null moves have been made (>= 2 in a row is game end)
    pub null_move_counter: u8,

    // Cached corner moves
    pub player_a_corner_moves: HashMap<Coord, Vec<u32>>,
    pub player_b_corner_moves: HashMap<Coord, Vec<u32>>,

    pub player_a_accumulator: Accumulator,
    pub player_b_accumulator: Accumulator,
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
            player_a_accumulator: Accumulator::new(&NNUE),
            player_b_accumulator: Accumulator::new(&NNUE),
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

            update_move_cache_from_null_move(self);

            return;
        }

        let unpacked = Move::unpack(board_move);
        // update the accumulators
        let move_tiles =
            &ORIENTATION_DATA[unpacked.movetype as usize][unpacked.orientation as usize];

        for tile in move_tiles {
            let x = tile.x + unpacked.x;
            let y = tile.y + unpacked.y;

            let player_a_offset = (x + 14 * y) as usize;
            let player_b_offset = ((13 - x) + 14 * (13 - y)) as usize;

            let stm_offset = 0;
            let ntm_offset = 196;

            self.player_a_accumulator.add_feature(
                if unpacked.player == 0 {
                    stm_offset + player_a_offset
                } else {
                    ntm_offset + player_b_offset
                },
                &NNUE,
            );

            self.player_b_accumulator.add_feature(
                if unpacked.player == 1 {
                    stm_offset + player_b_offset
                } else {
                    ntm_offset + player_a_offset
                },
                &NNUE,
            );
        }

        self.null_move_counter = 0;
        // note: update move cache calls skip_turn
        update_move_cache(self, board_move);
    }

    pub fn skip_turn(&mut self) {
        self.player = self.player.other();
    }
}
