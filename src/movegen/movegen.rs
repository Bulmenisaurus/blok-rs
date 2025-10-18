use crate::board::{
    BoardState, Coord, CornerMovesInfo, Player, StartPosition, get_start_position_coord,
};

use crate::movegen::blok_move::{Move, NULL_MOVE};
use crate::movegen::movegen_data::{
    CORNER_ATTACHERS_DIR_DATA, CORNER_MOVES_DATA, CORNER_MOVES_DATA_U64, CORNERS_DATA,
    ORIENTATION_DATA, ORIENTATIONS_BITBOARD_DATA, SHORT_BOUNDING_BOX_DATA,
};
use crate::movegen::zobrist::{NULL_MOVE_COUNT_ZOBRIST, PLAYER_A_ZOBRIST, PLAYER_B_ZOBRIST};

#[derive(serde::Deserialize, Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum CornerDirection {
    NW = 0,
    NE = 1,
    SE = 2,
    SW = 3,
}

fn corner_direction_to_enum(direction: u8) -> CornerDirection {
    match direction {
        0 => CornerDirection::NW,
        1 => CornerDirection::NE,
        2 => CornerDirection::SE,
        3 => CornerDirection::SW,
        _ => unreachable!("Invalid corner direction: {}", direction),
    }
}
#[derive(serde::Deserialize, Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct CoordWithDirection {
    x: i32,
    y: i32,
    d: u8,
}

/// Check if a move is legal
/// This method does not check if there is a corner!
/// It only makes sure that the move is in bounds and not intersecting with any other pieces or touching any of our pieces.
pub fn is_move_legal(board: &BoardState, m: u32) -> bool {
    is_move_legal_no_board(
        board.my_remaining(),
        board.my_bitboard(),
        board.their_bitboard(),
        m,
    )
}

pub fn is_move_legal_with_bitboard(
    board: &BoardState,
    m: u32,
    piece_bb: u64,
    window_bb: u64,
) -> bool {
    // check if it's been played before

    // (this condition is necessary for cache moves)
    if board.my_remaining() & (1 << Move::get_movetype(m)) == 0 {
        return false;
    }

    return piece_bb & window_bb == 0;
}

fn get_corner_window(board: &BoardState, corner: Coord, direction: u8) -> u64 {
    // how this corner attacher is offset from the corner itself
    let move_center_offset: (i32, i32) = match direction {
        0 => (0, 0),
        1 => (1, 0),
        2 => (1, 1),
        3 => (0, 1),
        _ => unreachable!(),
    };

    let window_top_left_coord = Coord {
        x: corner.x + move_center_offset.0,
        y: corner.y + move_center_offset.1,
    };

    let my_bitboard = board.my_bitboard();
    let their_bitboard = board.their_bitboard();

    // println!("My bitboard: {:?}", my_bitboard);
    // println!("Their bitboard: {:?}", their_bitboard);

    let mut bitboard: u64 = 0;
    for y in 0..8 {
        let idx = y + window_top_left_coord.y as usize + 1;
        let my_row = my_bitboard[idx];
        let my_row_above = my_bitboard[idx - 1];
        let my_row_below = my_bitboard[idx + 1];

        let their_row = their_bitboard[idx];

        let my_row_adjacency =
            my_row | my_row << 1 | my_row >> 1 | my_row_above | my_row_below | their_row;
        let my_row_adjacency = my_row_adjacency >> window_top_left_coord.x as usize;

        // println!("row {}: {:?}", y, my_row_adjacency);
        bitboard |= ((my_row_adjacency as u64) & 255) << (y * 8);
    }

    bitboard
}

/// Used to avoid a clone of the board when updating the move cache.
/// Since we need a mutable reference to the board, we can't use it immutably in `is_move_legal`
pub fn is_move_legal_no_board(
    my_remaining: u32,
    my_bitboard: &[u32; 24],
    their_bitboard: &[u32; 24],

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

    // check if this move has already been placed
    if my_remaining & (1u32 << movetype) == 0 {
        return false;
    }

    let piece_bitboard = &ORIENTATIONS_BITBOARD_DATA[movetype as usize][orientation as usize];

    // check if the move intersects with any of their pieces or adjacent to any of our pieces
    for (bb_y, row) in piece_bitboard.iter().enumerate() {
        let bitboard_row = (*row as u32) << location.x;
        let idx = location.y as usize + bb_y + 5;
        let game_row = their_bitboard[idx]
            | my_bitboard[idx]
            | my_bitboard[idx] << 1
            | my_bitboard[idx] >> 1
            | my_bitboard[idx - 1]
            | my_bitboard[idx + 1];

        let game_row = game_row >> 4;

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

                if !piece_middle.in_bounds() {
                    continue;
                }
                // check if it is outside of the board
                let (bx, by) =
                    SHORT_BOUNDING_BOX_DATA[mov.movetype as usize][mov.orientation as usize];
                if piece_middle.x + bx as i32 > 13 || piece_middle.y + by as i32 > 13 {
                    continue;
                }

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

pub fn get_corner_moves_from_bitboard(
    board: &BoardState,
    info: CornerMovesInfo,
    coord: &Coord,
) -> Vec<u32> {
    let mut moves: Vec<u32> = Vec::new();
    let mut bitboard_copy = info.moves;

    let direction_enum = corner_direction_to_enum(info.direction);

    let my_remaining = board.my_remaining();
    let my_bitboard = board.my_bitboard();
    let their_bitboard = board.their_bitboard();

    let window = get_corner_window(board, *coord, info.direction);

    while bitboard_copy != 0 {
        let index = bitboard_copy.trailing_zeros();

        bitboard_copy &= !(1 << index);

        // now we check if this move is still legal
        let cache_move = CORNER_MOVES_DATA[info.direction as usize][index as usize];
        let piece_bb = CORNER_MOVES_DATA_U64[info.direction as usize][index as usize];

        if piece_bb & window != 0 {
            continue;
        }

        let updated = update_cache_corner_move(
            cache_move,
            direction_enum,
            coord,
            board.player,
            my_remaining,
            my_bitboard,
            their_bitboard,
            false,
            false,
        );

        if let Some(updated) = updated {
            moves.push(updated);
        }
    }

    moves
}

/// Use the move cache to generate moves
pub fn generate_moves(board: &BoardState) -> Vec<u32> {
    if board.is_game_over() {
        return vec![];
    }

    if board.my_remaining() == 0x1fffff {
        return generate_first_moves(board);
    }

    let mut unique_moves_info: Vec<u32> = Vec::new();
    for (coord, info) in board.my_corner_moves_info().iter() {
        let moves = get_corner_moves_from_bitboard(board, *info, coord);
        // println!(
        //     "Generating moves for corner: {:?}, moves: {:?}",
        //     coord,
        //     moves.len()
        // );
        unique_moves_info.extend(moves);
    }
    unique_moves_info.sort_unstable();
    unique_moves_info.dedup();

    if unique_moves_info.is_empty() {
        return vec![NULL_MOVE];
    }

    unique_moves_info
}

// the cache moves are calculated from a fixed point (branching off of 7,7) so we need to translate the move to the correct position
pub fn update_cache_corner_move(
    cache_move: u32,
    direction: CornerDirection,
    position: &Coord,
    player: Player,
    my_remaining: u32,
    my_bitboard: &[u32; 24],
    their_bitboard: &[u32; 24],
    check_legal: bool,
    check_bounds: bool,
) -> Option<u32> {
    let mov = Move::unpack(cache_move);

    // how this corner attacher is offset from the corner itself
    let move_center_offset: (i32, i32) = match direction {
        CornerDirection::NW => (-1, -1),
        CornerDirection::NE => (1, -1),
        CornerDirection::SE => (1, 1),
        CornerDirection::SW => (-1, 1),
    };
    // since our cache is calculated from 7,7, we need to add the offset to the move
    let move_absolute_offset: (i32, i32) = (7 + move_center_offset.0, 7 + move_center_offset.1);

    let mov_coord = Coord {
        x: mov.x + position.x - move_absolute_offset.0,
        y: mov.y + position.y - move_absolute_offset.1,
    };

    // check if it is outside of the board
    if check_bounds {
        let (bx, by) = SHORT_BOUNDING_BOX_DATA[mov.movetype as usize][mov.orientation as usize];
        if mov_coord.x + bx as i32 > 13 || mov_coord.y + by as i32 > 13 {
            return None;
        }
    }

    let mov = Move {
        orientation: mov.orientation,
        y: mov_coord.y,
        x: mov_coord.x,
        player: player as u8,
        movetype: mov.movetype,
    };

    if !check_legal || is_move_legal_no_board(my_remaining, my_bitboard, their_bitboard, mov.pack())
    {
        return Some(mov.pack());
    }

    None
}
/// Used to generate moves from a corner when a new piece is placed
// TODO: hardcode some options for moves that are guaranteed to not intersect the piece we are placing
pub fn get_legal_moves_from(from: &Coord, board: &BoardState) -> CornerMovesInfo {
    let corner_direction =
        corner_direction_to_enum(board.corner_direction[from.y as usize * 14 + from.x as usize]);

    let cached_moves = CORNER_MOVES_DATA[corner_direction as usize];

    let mut bitboard: u128 = 0;
    let my_remaining = board.my_remaining();
    let my_bitboard = board.my_bitboard();
    let their_bitboard = board.their_bitboard();

    for (i, m) in cached_moves.into_iter().enumerate() {
        let updated = update_cache_corner_move(
            m,
            corner_direction,
            from,
            board.player,
            my_remaining,
            my_bitboard,
            their_bitboard,
            true,
            true,
        );
        if updated.is_some() {
            bitboard |= 1 << i;
        }
    }

    CornerMovesInfo {
        direction: corner_direction as u8,
        moves: bitboard,
    }
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

pub fn update_corner_moves_info(
    player: Player,
    info: &mut CornerMovesInfo,
    coord: &Coord,
    my_remaining: u32,
    my_bitboard: &[u32; 24],
    their_bitboard: &[u32; 24],
) {
    let mut bitboard_copy = info.moves;
    let direction_enum = corner_direction_to_enum(info.direction);

    while bitboard_copy != 0 {
        let index = bitboard_copy.trailing_zeros();

        bitboard_copy &= !(1 << index);

        // now we check if this move is still legal
        let cache_move = CORNER_MOVES_DATA[info.direction as usize][index as usize];
        let updated = update_cache_corner_move(
            cache_move,
            direction_enum,
            coord,
            player,
            my_remaining,
            my_bitboard,
            their_bitboard,
            true,
            false,
        );

        // it is illegal, so we remove it from the bitboard
        if updated.is_none() {
            info.moves &= !(1 << index);
        }
    }
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
        my_bitboard[mov.y as usize + bb_y + 5] |= (piece_bitboard[bb_y] as u32) << (mov.x + 4);
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
        board.player_a_corner_moves_info.remove(&absolute_corner);
        board.player_b_corner_moves_info.remove(&absolute_corner);
    }

    let corner_attachers =
        &CORNER_ATTACHERS_DIR_DATA[mov.movetype as usize][mov.orientation as usize];
    for corner in corner_attachers {
        // If this corner is not in bounds, skip
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

        let offset = absolute_corner.y as usize + 5;
        let my_bitboard = board.my_bitboard()[offset]
            | board.their_bitboard()[offset]
            | board.my_bitboard()[offset] << 1
            | board.my_bitboard()[offset] >> 1
            | board.my_bitboard()[offset - 1]
            | board.my_bitboard()[offset + 1];

        let move_row = my_bitboard & (1 << (absolute_corner.x + 4));
        if move_row != 0 {
            continue;
        }

        // now we update corner directions
        // I'm pretty sure we can just overwrite the direction for this corner, ignoring any other direction assigned here before
        // this is because the valid moves from this corner should be the intersection of the valid moves from the other corners, which is still a subset of the valid moves from this direction
        board.corner_direction[absolute_corner.y as usize * 14 + absolute_corner.x as usize] =
            corner.d;

        if board.my_corner_moves_info().contains_key(&absolute_corner) {
            continue;
        }

        let info = get_legal_moves_from(&absolute_corner, board);

        if board.player == Player::White {
            board
                .player_a_corner_moves_info
                .insert(absolute_corner, info);
        } else {
            board
                .player_b_corner_moves_info
                .insert(absolute_corner, info);
        }
    }

    board.skip_turn();

    if board.player == Player::White {
        for (coord, info) in board.player_a_corner_moves_info.iter_mut() {
            update_corner_moves_info(
                Player::White,
                info,
                coord,
                board.player_a_remaining,
                &board.player_a_bit_board,
                &board.player_b_bit_board,
            );
        }
    } else {
        for (coord, info) in board.player_b_corner_moves_info.iter_mut() {
            update_corner_moves_info(
                Player::Black,
                info,
                coord,
                board.player_b_remaining,
                &board.player_b_bit_board,
                &board.player_a_bit_board,
            );
        }
    }
}

pub fn update_move_cache_from_null_move(board: &mut BoardState) {
    update_null_hash(board);

    let my_remaining = board.my_remaining();
    let my_bitboard = if board.player == Player::White {
        &board.player_a_bit_board
    } else {
        &board.player_b_bit_board
    };
    let their_bitboard = if board.player == Player::White {
        &board.player_b_bit_board
    } else {
        &board.player_a_bit_board
    };
    let my_corner_moves_info = if board.player == Player::White {
        &mut board.player_a_corner_moves_info
    } else {
        &mut board.player_b_corner_moves_info
    };

    for (coord, info) in my_corner_moves_info.iter_mut() {
        update_corner_moves_info(
            board.player,
            info,
            coord,
            my_remaining,
            my_bitboard,
            their_bitboard,
        );
    }
}
