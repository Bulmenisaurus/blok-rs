use std::collections::HashMap;

use crate::board::{
    BoardState, Coord, CoordOffset, Player, StartPosition, get_start_position_coord,
};

use once_cell::sync::Lazy;

pub static PIECE_DATA: Lazy<Vec<Vec<Coord>>> = Lazy::new(|| {
    let json_str = include_str!("pieces.json");
    serde_json::from_str(json_str).unwrap()
});

pub static ORIENTATION_DATA: Lazy<Vec<Vec<Vec<Coord>>>> = Lazy::new(|| {
    let json_str = include_str!("piece-orientations.json");
    serde_json::from_str(json_str).unwrap()
});

pub static ORIENTATIONS_BITBOARD_DATA: Lazy<Vec<Vec<Vec<u16>>>> = Lazy::new(|| {
    let json_str = include_str!("piece-orientations-bitboard.json");
    serde_json::from_str(json_str).unwrap()
});

pub static ORIENTATIONS_BITBOARD_HALO_DATA: Lazy<Vec<Vec<Vec<u16>>>> = Lazy::new(|| {
    let json_str = include_str!("piece-orientations-bitboard-halo.json");
    serde_json::from_str(json_str).unwrap()
});

// pub static RR_DATA: Lazy<Vec<Vec<u32>>> = Lazy::new(|| {
//     let json_str = include_str!("piece-rr.json");
//     serde_json::from_str(json_str).unwrap()
// });

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

pub const NULL_MOVE: u32 = 0xf800;

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

    // check if it is outside of the board
    let (bx, by) = SHORT_BOUNDING_BOX_DATA[movetype as usize][orientation as usize];
    if location.x + bx > 13 || location.y + by > 13 {
        return false;
    }

    let my_remaining = if player == 0 {
        board.player_a_remaining
    } else {
        board.player_b_remaining
    };

    // check if this move has already been placed
    if my_remaining & (1u32 << movetype) == 0 {
        return false;
    }

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

    let piece_bitboard = &ORIENTATIONS_BITBOARD_DATA[movetype as usize][orientation as usize];

    // check for intersection or adjacency with my pieces
    let halo_data = &ORIENTATIONS_BITBOARD_HALO_DATA[movetype as usize][orientation as usize];

    for bb_y in 0..piece_bitboard.len() + 2 {
        if location.y as usize + bb_y == 0 || location.y as usize + bb_y > my_bitboard.len() {
            continue;
        }
        let cached_halo = halo_data[bb_y] << location.x;
        // shift by 1 to match the halo data
        let game_row = my_bitboard[location.y as usize + bb_y - 1] << 1;
        if (cached_halo & game_row) != 0 {
            return false;
        }
    }

    // check if there's an intersection with opponent

    for bb_y in 0..piece_bitboard.len() {
        let bitboard_row = piece_bitboard[bb_y] << location.x;
        let game_row = their_bitboard[location.y as usize + bb_y];

        if bitboard_row & game_row != 0 {
            return false;
        }
    }

    true
}

pub fn is_move_blokee_legal(m: &Move) -> bool {
    let move_tiles = &ORIENTATION_DATA[m.movetype as usize][m.orientation as usize];

    move_tiles.iter().all(|c| {
        let absolute: Coord = Coord {
            x: c.x + m.x,
            y: c.y + m.y,
        };

        if m.player == 0 {
            absolute.x <= 6 && absolute.y > 6
        } else {
            absolute.x > 6 && absolute.y <= 6
        }
    })
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

                // Special rules for "middleBlokee"
                if StartPosition::MiddleBlokee == board.start_position
                    && !is_move_blokee_legal(&mov)
                {
                    continue;
                }

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

    let mut unique_moves: Vec<u32> = my_corner_moves.values().flatten().cloned().collect();
    unique_moves.sort_unstable();
    unique_moves.dedup();

    if unique_moves.is_empty() {
        return vec![NULL_MOVE];
    }

    unique_moves
}

pub fn get_legal_moves_from(
    from: Coord,
    movetype: u8,
    board: &BoardState,
    legal_moves: &mut Vec<u32>,
) {
    let orientation_data = &ORIENTATION_DATA[movetype as usize];

    for i in 0..orientation_data.len() {
        let corners = &CORNERS_DATA[movetype as usize][i];
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
                movetype,
            };

            if is_move_legal(board, mov.pack()) {
                legal_moves.push(mov.pack());
            }
        }
    }
}

pub fn update_move_cache(board: &mut BoardState, last_move: u32) {
    let mov = Move::unpack(last_move);

    // remove this move from the pool
    if mov.player == (Player::White as u8) {
        board.player_a_remaining &= !(1 << mov.movetype);
    } else {
        board.player_b_remaining &= !(1 << mov.movetype);
    }

    let my_bitboard = if mov.player == (Player::White as u8) {
        &mut board.player_a_bit_board
    } else {
        &mut board.player_b_bit_board
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
        board.player_a_corner_moves.remove(&absolute_corner);
        board.player_b_corner_moves.remove(&absolute_corner);
    }

    let my_remaining_pieces = if board.player == Player::White {
        board.player_a_remaining
    } else {
        board.player_b_remaining
    };

    let corner_attachers = &CORNER_ATTACHERS_DATA[mov.movetype as usize][mov.orientation as usize];
    for corner in corner_attachers {
        if (corner.x < 0 && -corner.x > mov.x as i8) || (corner.y < 0 && -corner.y > mov.y as i8) {
            continue;
        }

        let absolute_corner = Coord {
            x: (corner.x + mov.x as i8) as u8,
            y: (corner.y + mov.y as i8) as u8,
        };

        if !absolute_corner.in_bounds() {
            continue;
        }

        if board.player == Player::White {
            if board.player_a_corner_moves.contains_key(&absolute_corner) {
                continue;
            }
        } else if board.player_b_corner_moves.contains_key(&absolute_corner) {
            continue;
        }

        let mut legal_moves: Vec<u32> = Vec::new();

        for unplaced_piece in 0..21u8 {
            if my_remaining_pieces & (1 << unplaced_piece) == 0 {
                continue;
            }

            get_legal_moves_from(absolute_corner, unplaced_piece, board, &mut legal_moves);
        }

        if board.player == Player::White {
            board
                .player_a_corner_moves
                .insert(absolute_corner, legal_moves);
        } else {
            board
                .player_b_corner_moves
                .insert(absolute_corner, legal_moves);
        }
    }

    board.skip_turn();

    // filter opponent's moves (now the player to move)
    let opponent_cached_moves: Vec<Coord> = if board.player == Player::White {
        board.player_a_corner_moves.keys().cloned().collect()
    } else {
        board.player_b_corner_moves.keys().cloned().collect()
    };

    for coord in opponent_cached_moves {
        let old_moves = if board.player == Player::White {
            board.player_a_corner_moves.get_mut(&coord).unwrap().clone()
        } else {
            board.player_b_corner_moves.get_mut(&coord).unwrap().clone()
        };

        let new_moves: Vec<u32> = old_moves
            .into_iter()
            .filter(|m| is_move_legal(board, *m))
            .collect();

        if board.player == Player::White {
            board.player_a_corner_moves.insert(coord, new_moves);
        } else {
            board.player_b_corner_moves.insert(coord, new_moves);
        }
    }
}

pub fn update_move_cache_from_null_move(board: &mut BoardState) {
    // Take ownership of the cached moves, filter them, then reassign
    let cached_moves = if board.player == Player::White {
        std::mem::take(&mut board.player_a_corner_moves)
    } else {
        std::mem::take(&mut board.player_b_corner_moves)
    };

    let filtered_moves: HashMap<Coord, Vec<u32>> = cached_moves
        .into_iter()
        .map(|(coord, moves)| {
            let legal_moves: Vec<u32> = moves
                .into_iter()
                .filter(|m| is_move_legal(board, *m))
                .collect();
            (coord, legal_moves)
        })
        .collect();

    if board.player == Player::White {
        board.player_a_corner_moves = filtered_moves;
    } else {
        board.player_b_corner_moves = filtered_moves;
    }
}
