use std::collections::HashSet;

use crate::board::{
    BoardState, Coord, CoordOffset, Player, StartPosition, get_start_position_coord,
};
use serde::Deserialize;
use serde_json;

use once_cell::sync::Lazy;

pub static PIECE_DATA: Lazy<Vec<Vec<Coord>>> = Lazy::new(|| {
    let json_str = include_str!("pieces.json");
    serde_json::from_str(json_str).unwrap()
});

pub static ORIENTATION_DATA: Lazy<Vec<Vec<Vec<Coord>>>> = Lazy::new(|| {
    let json_str = include_str!("piece-orientations.json");
    serde_json::from_str(json_str).unwrap()
});

pub static ORIENTATIONS_BITBOARD_DATA: Lazy<Vec<Vec<Vec<u32>>>> = Lazy::new(|| {
    let json_str = include_str!("piece-orientations-bitboard.json");
    serde_json::from_str(json_str).unwrap()
});

pub static ORIENTATIONS_BITBOARD_HALO_DATA: Lazy<Vec<Vec<Vec<u32>>>> = Lazy::new(|| {
    let json_str = include_str!("piece-orientations-bitboard-halo.json");
    serde_json::from_str(json_str).unwrap()
});

pub static RR_DATA: Lazy<Vec<Vec<u32>>> = Lazy::new(|| {
    let json_str = include_str!("piece-rr.json");
    serde_json::from_str(json_str).unwrap()
});

pub static CORNERS_DATA: Lazy<Vec<Vec<Vec<Coord>>>> = Lazy::new(|| {
    let json_str = include_str!("piece-corners.json");
    serde_json::from_str(json_str).unwrap()
});

pub static CORNER_ATTACHERS_DATA: Lazy<Vec<Vec<Vec<CoordOffset>>>> = Lazy::new(|| {
    let json_str = include_str!("piece-corner-attachers.json");
    serde_json::from_str(json_str).unwrap()
});

pub static SHORT_BOUNDING_BOX_DATA: Lazy<Vec<Vec<(u8, u8)>>> = Lazy::new(|| {
    let json_str = include_str!("piece-short-bounding-box.json");
    serde_json::from_str(json_str).unwrap()
});

pub const NULL_MOVE: u32 = 0x7800;

// An unpacked move, with all the information
#[derive(Clone, Copy, Debug)]
pub struct Move {
    // Orientation, 0-7
    pub orientation: u8,
    // Y coordinate, 0-13
    pub y: u8,
    // X coordinate, 0-13
    pub x: u8,
    // Move type, 0-20 i think
    pub movetype: u8,
    // Player, 0-1
    pub player: u8,
}

impl Move {
    pub fn new(orientation: u8, y: u8, x: u8, movetype: u8, player: u8) -> Move {
        Move {
            orientation,
            y,
            x,
            movetype,
            player,
        }
    }

    pub fn pack(self) -> u32 {
        (self.orientation as u32)
            | ((self.y as u32) << 3)
            | ((self.x as u32) << 7)
            | ((self.movetype as u32) << 11)
            | ((self.player as u32) << 16)
    }

    pub fn get_orientation(packed: u32) -> u8 {
        (packed & 0x7) as u8
    }

    pub fn get_location(packed: u32) -> Coord {
        let x = (packed & 0x780) >> 7;
        let y = (packed & 0x78) >> 3;
        Coord {
            x: x.try_into().unwrap(),
            y: y.try_into().unwrap(),
        }
    }

    pub fn get_movetype(packed: u32) -> u8 {
        ((packed & 0xf800) >> 11) as u8
    }

    pub fn get_player(packed: u32) -> u8 {
        ((packed & 0x10000) >> 16) as u8
    }

    pub fn unpack(packed: u32) -> Move {
        Move {
            orientation: Self::get_orientation(packed),
            y: Self::get_location(packed).y,
            x: Self::get_location(packed).x,
            movetype: Self::get_movetype(packed),
            player: Self::get_player(packed),
        }
    }
}

pub fn is_move_legal(board: &BoardState, m: u32) -> bool {
    if m == NULL_MOVE {
        return true;
    }
    let player = Move::get_player(m);
    let location = Move::get_location(m);
    let movetype = Move::get_movetype(m);
    let orientation = Move::get_orientation(m);

    let my_remaining = if player == 0 {
        board.player_a_remaining
    } else {
        board.player_b_remaining
    };

    let my_bitboard = if player == 0 {
        board.player_a_bit_board
    } else {
        board.player_b_bit_board
    };

    let their_bitboard = if player == 0 {
        board.player_b_bit_board
    } else {
        board.player_a_bit_board
    };

    // check if this move has already been placed
    if my_remaining & (1u32 << movetype) == 0 {
        return false;
    }

    if movetype == 16 && orientation == 1 {
        println!(
            "VAL weird move: movetype: {}, orientation: {}",
            movetype, orientation
        );
    }

    // check if it is outside of the board
    let (bx, by) = SHORT_BOUNDING_BOX_DATA[movetype as usize][orientation as usize];
    let bottom_right = Coord {
        x: location.x + bx,
        y: location.y + by,
    };

    if !bottom_right.in_bounds() || !location.in_bounds() {
        return false;
    }

    // check if there's an intersection with opponent
    let piece_bitboard = &ORIENTATIONS_BITBOARD_DATA[movetype as usize][orientation as usize];

    for bb_y in 0..piece_bitboard.len() {
        let bitboard_row = piece_bitboard[bb_y] << location.x;
        let game_row = their_bitboard[location.y as usize + bb_y];

        if bitboard_row & game_row != 0 {
            return false;
        }
    }

    // check for intersection or adjacency with my pieces
    let halo_data = &ORIENTATIONS_BITBOARD_HALO_DATA[movetype as usize][orientation as usize];

    for bb_y in 0..piece_bitboard.len() + 2 {
        if location.y as usize + bb_y == 0 || location.y as usize + bb_y - 1 >= my_bitboard.len() {
            continue;
        }
        let cached_halo = halo_data[bb_y as usize] << location.x;
        // shift by 1 to match the halo data
        let game_row = my_bitboard[location.y as usize + bb_y - 1] << 1;
        if (cached_halo & game_row) != 0 {
            return false;
        }
    }

    true
}

// Rules for the first move are different

pub fn generate_first_moves(board: &BoardState) -> Vec<u32> {
    // Get the starting position for the current player
    let (start_a, start_b) = get_start_position_coord(board.start_position);
    let start_pos = match board.player {
        Player::White => start_a,
        Player::Black => start_b,
    };

    if board.null_move_counter != 0 {
        panic!("NMC not 0 at the beginning of the game");
    }

    let mut moves: Vec<u32> = Vec::new();

    // each piece type
    for piece in 0..21 {
        // each (unique) orientation
        let orientations = &ORIENTATION_DATA[piece];
        // each location it can be placed
        for (i, piece_tiles) in orientations.iter().enumerate() {
            for tile in piece_tiles {
                if tile.x > start_pos.x || tile.y > start_pos.y {
                    continue;
                }
                let piece_middle = Coord {
                    x: start_pos.x - (tile.x),
                    y: start_pos.y - (tile.y),
                };

                // Build the move representation (assuming Move is u32, otherwise adjust)
                let mov = Move {
                    y: piece_middle.y,
                    x: piece_middle.x,
                    movetype: piece as u8,
                    player: board.player as u8,
                    orientation: i as u8,
                };

                if mov.movetype == 16 && mov.orientation == 1 {
                    println!(
                        "GEN weird move: movetype: {}, orientation: {}",
                        mov.movetype, mov.orientation
                    );
                }

                // Special rules for "middleBlokee"
                // if let StartPosition::middleBlokee = board.startPosition {
                //     if !is_move_blokee_legal(&mov, piece_tiles) {
                //         continue;
                //     }
                // }

                // Serialize the move to u32 or whatever is needed
                let packed = mov.pack();
                if Move::get_movetype(packed) == 16 && Move::get_orientation(packed) == 1 {
                    println!(
                        "GEN weird move: movetype: {}, orientation: {}, from move: {:?} -> {}",
                        mov.movetype, mov.orientation, mov, packed
                    );
                }
                moves.push(packed);
            }
        }
    }

    // Filter moves by legality
    moves
        .into_iter()
        .filter(|m| is_move_legal(board, *m))
        .collect()
}

pub fn generate_moves(board: &BoardState) -> Vec<u32> {
    if board.is_game_over() {
        return vec![];
    }

    let my_remaining = if board.player == Player::White {
        board.player_a_remaining
    } else {
        board.player_b_remaining
    };

    if my_remaining == 0x1fffff {
        return generate_first_moves(board);
    }

    // otherwise, use the cached moves
    let my_corner_moves = if board.player == Player::White {
        &board.player_a_corner_moves
    } else {
        &board.player_b_corner_moves
    };

    let unique_moves: HashSet<u32> = my_corner_moves.values().flatten().cloned().collect();
    let unique_moves: Vec<u32> = unique_moves.into_iter().collect();

    if unique_moves.len() == 0 {
        return vec![NULL_MOVE];
    }

    unique_moves
}

pub fn get_legal_moves_from(from: Coord, movetype: u8, board: &BoardState) -> Vec<u32> {
    let mut legal_moves: Vec<u32> = Vec::new();
    let orientation_data = &ORIENTATION_DATA[movetype as usize];

    for i in 0..orientation_data.len() {
        let corners = &CORNERS_DATA[movetype as usize][i as usize];
        for corner in corners {
            if from.x < corner.x || from.y < corner.y {
                continue;
            }

            let coord = Coord {
                x: from.x - corner.x,
                y: from.y - corner.y,
            };

            if !coord.in_bounds() {
                continue;
            }

            let mov = Move {
                orientation: i as u8,
                y: coord.y,
                x: coord.x,
                player: board.player as u8,
                movetype: movetype,
            };

            legal_moves.push(mov.pack());
        }
    }

    return legal_moves
        .into_iter()
        .filter(|m| is_move_legal(board, *m))
        .collect();
}
