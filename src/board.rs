use std::collections::HashMap;

use crate::movegen::{NULL_MOVE, PIECE_DATA, update_move_cache, update_move_cache_from_null_move};

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
    pub x: i32,
    pub y: i32,
}

impl Coord {
    pub fn in_bounds(&self) -> bool {
        self.x < 14 && self.y < 14 && self.x >= 0 && self.y >= 0
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
    Win(Player),
    Draw,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CornerMovesInfo {
    pub direction: u8,
    pub moves: u128,
}

#[derive(Debug, Clone)]
pub struct BoardState {
    /// Player to move
    pub player: Player,

    /// Remaining pieces for each player, as a bitmask
    pub player_a_remaining: u32,
    pub player_b_remaining: u32,

    /// Bitboards for tiles placed
    pub player_a_bit_board: [u16; 16],
    pub player_b_bit_board: [u16; 16],

    pub start_position: StartPosition,

    /// How many null moves have been made (>= 2 in a row is game end)
    pub null_move_counter: u8,

    /// Cached corner moves
    pub player_a_corner_moves_info: HashMap<Coord, CornerMovesInfo>,
    pub player_b_corner_moves_info: HashMap<Coord, CornerMovesInfo>,

    pub corner_direction: [u8; 196],
    pub history: Vec<u32>,

    pub hash: u64,
}

impl BoardState {
    pub fn new(start_position: StartPosition) -> Self {
        Self {
            player: Player::White,
            player_a_remaining: 0x1fffff,
            player_b_remaining: 0x1fffff,
            player_a_bit_board: [0; 16],
            player_b_bit_board: [0; 16],
            null_move_counter: 0,
            start_position,
            player_a_corner_moves_info: HashMap::new(),
            player_b_corner_moves_info: HashMap::new(),
            corner_direction: [0; 196],
            history: Vec::new(),
            hash: 0,
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
            std::cmp::Ordering::Greater => GameResult::Win(Player::White),
            std::cmp::Ordering::Less => GameResult::Win(Player::Black),
            std::cmp::Ordering::Equal => GameResult::Draw,
        }
    }

    // change states, incrementally update move cache
    pub fn do_move(&mut self, board_move: u32) {
        self.history.push(board_move);
        if board_move == NULL_MOVE {
            self.null_move_counter += 1;
            self.skip_turn();

            update_move_cache_from_null_move(self);

            return;
        }

        self.null_move_counter = 0;
        // note: update move cache calls skip_turn
        update_move_cache(self, board_move);
    }

    pub fn skip_turn(&mut self) {
        self.player = self.player.other();
    }

    pub fn serialize(&self) -> String {
        let serialized = (1..15)
            .map(|y| self.player_a_bit_board[y] as u32 | (self.player_b_bit_board[y] as u32) << 16)
            .map(|x| format!("{}", x))
            .collect::<Vec<String>>()
            .join(", ");

        format!("[{}]", serialized)
    }
}
