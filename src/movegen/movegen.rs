use std::collections::HashMap;

use crate::board::{BoardState, Coord, Player, StartPosition, get_start_position_coord};

use crate::movegen::zobrist::{NULL_MOVE_COUNT_ZOBRIST, PLAYER_A_ZOBRIST, PLAYER_B_ZOBRIST};
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

// pub static RR_DATA: Lazy<Vec<Vec<u32>>> = Lazy::new(|| {
//     let json_str = include_str!("piece-rr.json");
//     serde_json::from_str(json_str).unwrap()
// });

pub static CORNERS_DATA: Lazy<Vec<Vec<Vec<Coord>>>> = Lazy::new(|| {
    let json_str = include_str!("piece-corners.json");
    serde_json::from_str(json_str).unwrap()
});

pub static CORNER_ATTACHERS_DATA: Lazy<Vec<Vec<Vec<Coord>>>> = Lazy::new(|| {
    let json_str = include_str!("piece-corner-attachers.json");
    serde_json::from_str(json_str).unwrap()
});

pub static SHORT_BOUNDING_BOX_DATA: Lazy<Vec<Vec<(u8, u8)>>> = Lazy::new(|| {
    let json_str = include_str!("piece-short-bounding-box.json");
    serde_json::from_str(json_str).unwrap()
});

static CORNER_MOVES_DATA: [[u32; 127]; 4] = [
    [
        426, 427, 431, 438, 664, 665, 668, 693, 797, 814, 2478, 2479, 2482, 2483, 2602, 2712, 2717,
        2721, 2731, 2732, 2841, 2844, 4523, 4527, 4530, 4534, 4654, 4760, 4764, 4773, 4777, 4778,
        4889, 4893, 6825, 6826, 6827, 6832, 6952, 8746, 8747, 8864, 8865, 8880, 9003, 10785, 10786,
        10787, 10800, 11040, 12833, 12834, 12843, 12848, 12960, 13091, 14888, 15008, 16928, 16929,
        16938, 16947, 17059, 17186, 18977, 18978, 18987, 18992, 19107, 19112, 19232, 21034, 21035,
        21039, 21046, 21152, 21153, 21156, 21165, 21166, 21285, 23072, 23079, 23081, 23082, 23083,
        23084, 23086, 23093, 23201, 23202, 23203, 23204, 23205, 23334, 25256, 27176, 27177, 27181,
        27188, 27298, 27302, 27303, 27315, 27427, 27436, 29224, 29235, 29345, 29354, 29355, 29474,
        31273, 31280, 31394, 31400, 31403, 31523, 33584, 35505, 35624, 37425, 37664, 39345, 39704,
        41265, 41744,
    ],
    [
        687, 924, 1048, 1049, 1053, 1066, 1067, 1070, 1076, 1079, 2858, 2968, 2973, 2987, 3097,
        3100, 3104, 3117, 3118, 3119, 3122, 3123, 4907, 5016, 5020, 5039, 5145, 5149, 5156, 5160,
        5162, 5166, 5171, 5175, 7081, 7208, 7210, 7211, 7217, 9003, 9248, 9249, 9258, 9259, 9265,
        11041, 11296, 11298, 11299, 11313, 13090, 13217, 13344, 13347, 13354, 13361, 15264, 15400,
        17185, 17315, 17440, 17442, 17449, 17459, 19233, 19362, 19369, 19488, 19491, 19498, 19505,
        21412, 21423, 21536, 21537, 21541, 21546, 21547, 21548, 21550, 21559, 23335, 23456, 23458,
        23459, 23460, 23461, 23585, 23590, 23592, 23594, 23595, 23597, 23599, 23604, 25640, 27437,
        27554, 27683, 27686, 27687, 27688, 27689, 27692, 27698, 27701, 29601, 29611, 29730, 29736,
        29737, 29747, 31650, 31657, 31779, 31784, 31786, 31793, 33840, 35880, 35889, 37920, 37937,
        39960, 39985, 42000, 42033,
    ],
    [
        706, 961, 1065, 1082, 1088, 1091, 1092, 1093, 1094, 1095, 2887, 3008, 3013, 3014, 3120,
        3133, 3134, 3135, 3137, 3138, 3139, 3140, 4934, 5057, 5058, 5061, 5173, 5177, 5178, 5182,
        5184, 5187, 5188, 5191, 7107, 7227, 7232, 7233, 7234, 9026, 9265, 9280, 9281, 9282, 9283,
        11075, 11315, 11328, 11329, 11330, 13123, 13248, 13360, 13371, 13377, 13378, 15296, 15416,
        17217, 17344, 17456, 17465, 17474, 17475, 19267, 19387, 19392, 19507, 19512, 19521, 19522,
        21441, 21442, 21561, 21562, 21568, 21571, 21572, 21573, 21574, 21575, 23362, 23488, 23489,
        23493, 23494, 23495, 23601, 23608, 23610, 23613, 23614, 23615, 23619, 23620, 25664, 27456,
        27591, 27703, 27704, 27713, 27714, 27715, 27716, 27717, 27718, 29632, 29633, 29752, 29753,
        29762, 29763, 31680, 31683, 31800, 31803, 31809, 31810, 33856, 35904, 35905, 37952, 37953,
        40000, 40001, 42048, 42049,
    ],
    [
        443, 450, 454, 455, 680, 705, 708, 709, 832, 835, 2494, 2495, 2498, 2499, 2631, 2737, 2748,
        2752, 2757, 2758, 2881, 2884, 4539, 4543, 4546, 4550, 4675, 4788, 4792, 4801, 4805, 4807,
        4928, 4932, 6842, 6848, 6849, 6851, 6978, 8770, 8771, 8880, 8896, 8897, 9026, 10802, 10816,
        10817, 10819, 11074, 12849, 12858, 12864, 12867, 12993, 13122, 14904, 15040, 16944, 16954,
        16961, 16963, 17088, 17218, 18994, 19001, 19008, 19011, 19130, 19137, 19266, 21051, 21058,
        21062, 21063, 21176, 21185, 21187, 21188, 21189, 21312, 23088, 23097, 23099, 23100, 23102,
        23103, 23106, 23109, 23232, 23233, 23236, 23238, 23239, 23363, 25280, 27193, 27200, 27204,
        27205, 27318, 27330, 27331, 27335, 27457, 27462, 29240, 29251, 29370, 29376, 29377, 29506,
        31289, 31296, 31418, 31425, 31427, 31554, 33600, 35521, 35648, 37441, 37696, 39361, 39744,
        41281, 41792,
    ],
];

pub const NULL_MOVE: u32 = 0xf800;
pub const INVALID_MOVE: u32 = 0xf801;

/// An unpacked move, with all the information
#[derive(Clone, Copy, Debug)]
pub struct Move {
    /// Orientation, 0-7
    pub orientation: u8,
    /// Y coordinate, 0-13
    pub y: i32,
    /// X coordinate, 0-13
    pub x: i32,
    /// Move type, 0-20 i think
    pub movetype: u8,
    /// Player, 0-1
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

pub enum CornerDirection {
    NW,
    NE,
    SE,
    SW,
}

/// Check if a move is legal
/// This method does not check if there is a corner!
/// It only makes sure that the move is in bounds and not intersecting with any other pieces or touching any of our pieces.
pub fn is_move_legal(board: &BoardState, m: u32) -> bool {
    let player = Move::get_player(m);
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
    is_move_legal_no_board(my_remaining, &my_bitboard, &their_bitboard, m)
}

/// Used to avoid a clone of the board when updating the move cache.
/// Since we need a mutable reference to the board, we can't use it immutably in `is_move_legal`
pub fn is_move_legal_no_board(
    my_remaining: u32,
    my_bitboard: &[u16; 16],
    their_bitboard: &[u16; 16],

    m: u32,
) -> bool {
    // Null moves are assumed to be legal if generated
    // However, null move cannot always be played, the burden is on the caller to check if it is legal
    if m == NULL_MOVE {
        return true;
    }

    let location = Move::get_location(m);
    let movetype = Move::get_movetype(m);
    let orientation = Move::get_orientation(m);

    // check if it is outside of the board
    let (bx, by) = SHORT_BOUNDING_BOX_DATA[movetype as usize][orientation as usize];
    if location.x + bx as i32 > 13 || location.y + by as i32 > 13 {
        return false;
    }

    // check if this move has already been placed
    if my_remaining & (1u32 << movetype) == 0 {
        return false;
    }

    let piece_bitboard = &ORIENTATIONS_BITBOARD_DATA[movetype as usize][orientation as usize];

    // check if the move intersects with any of their pieces or adjacent to any of our pieces
    for (bb_y, row) in piece_bitboard.iter().enumerate() {
        let bitboard_row = row << location.x;
        let idx = location.y as usize + bb_y + 1;
        let game_row = their_bitboard[idx]
            | my_bitboard[idx]
            | my_bitboard[idx] << 1
            | my_bitboard[idx] >> 1
            | my_bitboard[idx - 1]
            | my_bitboard[idx + 1];

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
/// Rules for the first move are different
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

                moves.push(mov.pack());
            }
        }
    }

    // Filter moves by legality
    moves
        .into_iter()
        .filter(|m| is_move_legal(board, *m))
        .collect()
}

/// Use the move cache to generate moves
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

    // Since the same move can be generated from multiple corners, we need to deduplicate them
    let mut unique_moves: Vec<u32> = my_corner_moves.values().flatten().cloned().collect();
    unique_moves.sort_unstable();
    unique_moves.dedup();

    if unique_moves.is_empty() {
        return vec![NULL_MOVE];
    }

    unique_moves
}

/// Used to generate moves from a corner when a new piece is placed
// TODO: hardcode some options for moves that are guaranteed to not intersect the piece we are placing
pub fn get_legal_moves_from(from: Coord, movetype: u8, board: &BoardState) -> Vec<u32> {
    let mut legal_moves: Vec<u32> = Vec::new();
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

    legal_moves
}

/// Used to hash a move for the Zobrist hash
pub fn move_zobrist_hash(mov: &Move, board: &BoardState) -> u64 {
    if mov.movetype == 31 {
        return NULL_MOVE_COUNT_ZOBRIST[board.null_move_counter as usize];
    }
    let my_zobrist = if mov.player == 0 {
        &PLAYER_A_ZOBRIST
    } else {
        &PLAYER_B_ZOBRIST
    };

    let mut hash = 0;

    let tiles = &ORIENTATION_DATA[mov.movetype as usize][mov.orientation as usize];
    for tile in tiles {
        let absolute_tile = Coord {
            x: tile.x + mov.x,
            y: tile.y + mov.y,
        };
        hash ^= my_zobrist[absolute_tile.y as usize * 14 + absolute_tile.x as usize];
    }

    hash ^= NULL_MOVE_COUNT_ZOBRIST[board.null_move_counter as usize];

    hash
}
fn update_hash(board: &mut BoardState, mov: &Move) {
    board.hash ^= move_zobrist_hash(mov, board);
}

fn update_null_hash(board: &mut BoardState) {
    board.hash ^= NULL_MOVE_COUNT_ZOBRIST[board.null_move_counter as usize];
}

pub fn update_move_cache(board: &mut BoardState, last_move: u32) {
    let mov = Move::unpack(last_move);

    update_hash(board, &mov);

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
        my_bitboard[mov.y as usize + bb_y + 1] |= piece_bitboard[bb_y] << mov.x;
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
        if (corner.x < 0 && -corner.x > mov.x) || (corner.y < 0 && -corner.y > mov.y) {
            continue;
        }

        let absolute_corner = Coord {
            x: (corner.x + mov.x),
            y: (corner.y + mov.y),
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

        for unplaced_piece in 0..21 {
            if my_remaining_pieces & (1 << unplaced_piece) == 0 {
                continue;
            }

            let movetype = unplaced_piece as u8;

            legal_moves.extend(get_legal_moves_from(absolute_corner, movetype, board));
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

    if board.player == Player::White {
        board
            .player_a_corner_moves
            .iter_mut()
            .for_each(|(_coord, moves)| {
                moves.retain(|m| {
                    is_move_legal_no_board(
                        board.player_a_remaining,
                        &board.player_a_bit_board,
                        &board.player_b_bit_board,
                        *m,
                    )
                })
            });
    } else {
        board
            .player_b_corner_moves
            .iter_mut()
            .for_each(|(_coord, moves)| {
                moves.retain(|m| {
                    is_move_legal_no_board(
                        board.player_b_remaining,
                        &board.player_b_bit_board,
                        &board.player_a_bit_board,
                        *m,
                    )
                })
            });
    }
}

pub fn update_move_cache_from_null_move(board: &mut BoardState) {
    update_null_hash(board);

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
