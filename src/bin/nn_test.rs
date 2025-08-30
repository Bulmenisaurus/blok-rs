use blok_rs::board::{BoardState, Coord, GameResult, Player, StartPosition};
use blok_rs::mcts::MonteCarlo;
use blok_rs::movegen::{Move, NULL_MOVE, generate_moves};
use blok_rs::nn::{Accumulator, Network};

static NNUE: Network = unsafe { std::mem::transmute(*include_bytes!("../../nn/quantised.bin")) };

fn main() {
    // Create a new board in the default start position
    let mut board = BoardState::new(StartPosition::Corner, NNUE);

    let mut mcts = MonteCarlo::new(NNUE);

    let mut player_a_accumulator = Accumulator::new(&NNUE);
    let mut player_b_accumulator = Accumulator::new(&NNUE);

    // Play 6 random moves
    while board.game_result() == GameResult::InProgress {
        let moves = generate_moves(&board);
        if moves.is_empty() {
            break;
        }

        mcts.run_search(&board, "easy");
        let best_move = mcts.best_play().unwrap();
        let (n_plays, score) = mcts.get_stats();
        let approximate_wins = ((score + n_plays as f64) / 2.0) as usize;
        mcts.clear();

        board.do_move(best_move);

        // now we must update the accumulators
        let unpacked = Move::unpack(best_move);

        let move_tiles: &Vec<Coord> = if best_move == NULL_MOVE {
            &vec![]
        } else {
            &blok_rs::movegen::ORIENTATION_DATA[unpacked.movetype as usize]
                [unpacked.orientation as usize]
        };

        for tile in move_tiles {
            let x = tile.x + unpacked.x;
            let y = tile.y + unpacked.y;

            let player_a_offset = (x + 14 * y) as usize;
            let player_b_offset = ((13 - x) + 14 * (13 - y)) as usize;

            let stm_offset = 0;
            let ntm_offset = 196;

            player_a_accumulator.add_feature(
                if unpacked.player == 0 {
                    stm_offset + player_a_offset
                } else {
                    ntm_offset + player_b_offset
                },
                &NNUE,
            );

            player_b_accumulator.add_feature(
                if unpacked.player == 1 {
                    stm_offset + player_b_offset
                } else {
                    ntm_offset + player_a_offset
                },
                &NNUE,
            );
        }

        let eval = if board.player == Player::White {
            NNUE.evaluate(&board.player_a_accumulator, &board.player_b_accumulator)
        } else {
            NNUE.evaluate(&board.player_b_accumulator, &board.player_a_accumulator)
        };

        println!("Eval: {}", eval);
        println!("Board: {:?}", pack(&board, approximate_wins, n_plays));
        println!();
    }
}

fn pack(board: &BoardState, n_wins: usize, n_plays: usize) -> [u32; 15] {
    let mut packed: [u32; 15] = [0; 15];

    #[allow(clippy::needless_range_loop)]
    for i in 0..14 {
        let player_a_data = board.player_a_bit_board[i];
        let player_b_data = board.player_b_bit_board[i];

        packed[i] = player_a_data as u32 | (player_b_data as u32) << 16;
    }
    // n_wins, n_plays each take 14 bits (so max of 2^14 =)
    // use the top two bits to store result (00 = win, 01 = loss, 10 = tie)

    let side_to_move = match board.player {
        Player::White => 0,
        Player::Black => 1,
    };

    packed[14] = n_plays as u32 | (n_wins as u32) << 14 | side_to_move << 28;
    packed
}
