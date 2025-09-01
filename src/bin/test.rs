use blok_rs::board::{BoardState, Player, StartPosition};

use blok_rs::movegen::generate_moves;
use blok_rs::nn::NNUE;

fn main() {
    // Create a new board in the default start position
    let mut board = BoardState::new(StartPosition::Corner, NNUE);

    let moves = [
        8195, 67156, 2452, 93531, 20530, 87118, 515, 80728, 4113, 85427, 41985, 107920, 14656,
        70538, 10329, 88593, 13003, 78297, 19507, 82841, 23340, 73920, 40584, 103041, 27738,
        100784, 31289, 99264, 36392, 72369, 7499, 94361, 30169, 63488, 63488,
    ];

    for m in moves {
        board.do_move(m);
    }

    println!("{:?}", board.score());
    println!("{:?}", board.game_result());
}
