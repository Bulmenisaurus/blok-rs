use blok_rs::{
    board::{BoardState, Coord, StartPosition},
    movegen::Move,
};

fn main() {
    let unpacked: Move = Move {
        orientation: 0,
        y: 7,
        x: 7,
        movetype: 16,
        player: 0,
    };
    let packed = unpacked.pack();
    println!("Packed: {}", packed);
    let mut board = BoardState::new(StartPosition::Corner);
    board.do_move(packed);
    // NW NE SE SW
    let corner_order = vec![
        Coord { x: 6, y: 6 },
        Coord { x: 8, y: 6 },
        Coord { x: 8, y: 8 },
        Coord { x: 6, y: 8 },
    ];
    let mut all_moves: Vec<Vec<u32>> = Vec::new();
    for corner in corner_order {
        let mut moves = board
            .player_a_corner_moves
            .get(&corner)
            .unwrap_or(&vec![])
            .clone();
        // add the single dot
        let single_dot = Move {
            orientation: 0,
            y: corner.y,
            x: corner.x,
            movetype: 16,
            player: 0,
        };
        moves.push(single_dot.pack());
        moves.sort();

        all_moves.push(moves);
    }
    println!("{:?}", all_moves);
}
