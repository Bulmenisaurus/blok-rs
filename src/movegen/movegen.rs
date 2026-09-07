use crate::board::{
    BoardState, Coord, CornerMovesInfo, Player, StartPosition, get_start_position_coord,
};

use crate::movegen::blok_move::{INVALID_MOVE, Move, NULL_MOVE};
use crate::movegen::movegen_data::{
    CORNER_ATTACHERS_DIR_DATA, CORNER_MOVES_DATA, CORNERS_DATA, ORIENTATION_DATA,
    ORIENTATIONS_BITBOARD_DATA, SHORT_BOUNDING_BOX_DATA,
};
use crate::movegen::zobrist::{NULL_MOVE_COUNT_ZOBRIST, PLAYER_A_ZOBRIST, PLAYER_B_ZOBRIST};

use once_cell::sync::Lazy;

/// Window geometry: every cached move's tiles lie within [-4, 4]^2 of the corner cell.
const WIN: i32 = 9;
const HALF: i32 = 4;

#[inline]
fn dir_offset(d: usize) -> (i32, i32) {
    match d {
        0 => (-1, -1),
        1 => (1, -1),
        2 => (1, 1),
        _ => (-1, 1),
    }
}

/// Footprint of each cached move relative to the corner cell, as a 9x9 window bitmask.
pub static CORNER_MOVE_FOOTPRINTS: Lazy<[[u128; 127]; 4]> = Lazy::new(|| {
    let mut out = [[0u128; 127]; 4];
    for d in 0..4 {
        let off = dir_offset(d);
        for i in 0..127 {
            let mov = Move::unpack(CORNER_MOVES_DATA[d][i]);
            let rel = (mov.x - (7 + off.0), mov.y - (7 + off.1));
            let mut fp = 0u128;
            for tile in &ORIENTATION_DATA[mov.movetype as usize][mov.orientation as usize] {
                let rx = rel.0 + tile.x;
                let ry = rel.1 + tile.y;
                assert!(rx.abs() <= HALF && ry.abs() <= HALF);
                fp |= 1u128 << ((ry + HALF) * WIN + (rx + HALF));
            }
            out[d][i] = fp;
        }
    }
    out
});

/// For each direction and piece type, bitmask of cached moves that use that type.
pub static CORNER_MOVE_TYPE_MASKS: Lazy<[[u128; 21]; 4]> = Lazy::new(|| {
    let mut out = [[0u128; 21]; 4];
    for d in 0..4 {
        for i in 0..127 {
            let t = Move::get_movetype(CORNER_MOVES_DATA[d][i]) as usize;
            out[d][t] |= 1u128 << i;
        }
    }
    out
});

/// Bitmask of cached moves whose piece type is still available, per direction.
#[inline]
fn available_masks(my_remaining: u32) -> [u128; 4] {
    let mut out = [0u128; 4];
    let mut r = my_remaining;
    while r != 0 {
        let t = r.trailing_zeros() as usize;
        r &= r - 1;
        for d in 0..4 {
            out[d] |= CORNER_MOVE_TYPE_MASKS[d][t];
        }
    }
    out
}

#[inline]
fn forbidden_row(my: &[u16; 16], their: &[u16; 16], idx: usize) -> u32 {
    (their[idx] | my[idx] | my[idx] << 1 | my[idx] >> 1 | my[idx - 1] | my[idx + 1]) as u32 & 0x3fff
}

/// 9x9 window of cells where the side to move may not place a tile, centred on (cx, cy).
/// Off-board cells count as forbidden, which subsumes the bounds check.
#[inline]
fn forbidden_window(my: &[u16; 16], their: &[u16; 16], cx: i32, cy: i32) -> u128 {
    let mut w = 0u128;
    for ry in -HALF..=HALF {
        let y = cy + ry;
        let bits: u32 = if !(0..14).contains(&y) {
            0x1ff
        } else {
            let wide = (forbidden_row(my, their, (y + 1) as usize) << HALF) | !(0x3fffu32 << HALF);
            (wide >> cx) & 0x1ff
        };
        w |= (bits as u128) << ((ry + HALF) * WIN);
    }
    w
}

/// Bounding box (x0, y0, x1, y1) inclusive of a placed piece, grown by `grow`.
#[inline]
fn move_region(m: u32, grow: i32) -> Option<(i32, i32, i32, i32)> {
    if m == INVALID_MOVE || m == NULL_MOVE {
        return None;
    }
    let mov = Move::unpack(m);
    let (bx, by) = SHORT_BOUNDING_BOX_DATA[mov.movetype as usize][mov.orientation as usize];
    Some((
        mov.x - grow,
        mov.y - grow,
        mov.x + bx as i32 + grow,
        mov.y + by as i32 + grow,
    ))
}

#[inline]
fn window_hits(c: &Coord, r: &Option<(i32, i32, i32, i32)>) -> bool {
    match r {
        None => false,
        Some((x0, y0, x1, y1)) => {
            c.x + HALF >= *x0 && c.x - HALF <= *x1 && c.y + HALF >= *y0 && c.y - HALF <= *y1
        }
    }
}

/// Remove the cached moves that intersect the forbidden window.
#[inline]
fn filter_by_window(moves: u128, d: usize, w: u128) -> u128 {
    let foot = &CORNER_MOVE_FOOTPRINTS[d];
    let mut out = moves;
    let mut bits = moves;
    while bits != 0 {
        let i = bits.trailing_zeros() as usize;
        bits &= bits - 1;
        if foot[i] & w != 0 {
            out &= !(1u128 << i);
        }
    }
    out
}

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

    while bitboard_copy != 0 {
        let index = bitboard_copy.trailing_zeros();

        bitboard_copy &= !(1 << index);

        // now we check if this move is still legal
        let cache_move = CORNER_MOVES_DATA[info.direction as usize][index as usize];
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

    let mut unique_moves_info: Vec<u32> = Vec::with_capacity(512);
    for (coord, info) in board.my_corner_moves_info().iter() {
        let moves = get_corner_moves_from_bitboard(board, *info, coord);
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
    my_bitboard: &[u16; 16],
    their_bitboard: &[u16; 16],
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
    let d = board.corner_direction[from.y as usize * 14 + from.x as usize] as usize;
    let avail = available_masks(board.my_remaining())[d];
    let w = forbidden_window(board.my_bitboard(), board.their_bitboard(), from.x, from.y);
    CornerMovesInfo {
        direction: d as u8,
        moves: filter_by_window(avail, d, w),
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
    info: &mut CornerMovesInfo,
    coord: &Coord,
    avail: &[u128; 4],
    my_bitboard: &[u16; 16],
    their_bitboard: &[u16; 16],
) {
    let d = info.direction as usize;
    info.moves &= avail[d];
    if info.moves == 0 {
        return;
    }
    let w = forbidden_window(my_bitboard, their_bitboard, coord.x, coord.y);
    info.moves = filter_by_window(info.moves, d, w);
}

pub fn update_move_cache(board: &mut BoardState, last_move: u32) {
    let mov = Move::unpack(last_move);
    board.last_move[mov.player as usize] = last_move;

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

        let offset = absolute_corner.y as usize + 1;
        let my_bitboard = board.my_bitboard()[offset]
            | board.their_bitboard()[offset]
            | board.my_bitboard()[offset] << 1
            | board.my_bitboard()[offset] >> 1
            | board.my_bitboard()[offset - 1]
            | board.my_bitboard()[offset + 1];

        let move_row = my_bitboard & (1 << absolute_corner.x);
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

    // Since this side's cache was last validated, the board changed by: its own last
    // placement (tiles + adjacency, so grow by 1) and the opponent's last placement (tiles).
    let (me, them) = match board.player {
        Player::White => (0usize, 1usize),
        Player::Black => (1, 0),
    };
    let own_region = move_region(board.last_move[me], 1);
    let opp_region = move_region(board.last_move[them], 0);

    if board.player == Player::White {
        let avail = available_masks(board.player_a_remaining);
        for (coord, info) in board.player_a_corner_moves_info.iter_mut() {
            info.moves &= avail[info.direction as usize];
            if info.moves != 0
                && (window_hits(coord, &own_region) || window_hits(coord, &opp_region))
            {
                update_corner_moves_info(
                    info,
                    coord,
                    &avail,
                    &board.player_a_bit_board,
                    &board.player_b_bit_board,
                );
            }
        }
    } else {
        let avail = available_masks(board.player_b_remaining);
        for (coord, info) in board.player_b_corner_moves_info.iter_mut() {
            info.moves &= avail[info.direction as usize];
            if info.moves != 0
                && (window_hits(coord, &own_region) || window_hits(coord, &opp_region))
            {
                update_corner_moves_info(
                    info,
                    coord,
                    &avail,
                    &board.player_b_bit_board,
                    &board.player_a_bit_board,
                );
            }
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

    let avail = available_masks(my_remaining);
    for (coord, info) in my_corner_moves_info.iter_mut() {
        update_corner_moves_info(info, coord, &avail, my_bitboard, their_bitboard);
    }
}
