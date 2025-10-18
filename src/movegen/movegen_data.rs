use once_cell::sync::Lazy;

use crate::{
    board::Coord,
    movegen::{Move, movegen::CoordWithDirection},
};

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

pub static CORNERS_DATA: Lazy<Vec<Vec<Vec<Coord>>>> = Lazy::new(|| {
    let json_str = include_str!("piece-corners.json");
    serde_json::from_str(json_str).unwrap()
});

pub static CORNER_ATTACHERS_DIR_DATA: Lazy<Vec<Vec<Vec<CoordWithDirection>>>> = Lazy::new(|| {
    let json_str = include_str!("piece-corner-attachers-dir.json");
    serde_json::from_str(json_str).unwrap()
});

pub static SHORT_BOUNDING_BOX_DATA: Lazy<Vec<Vec<(u8, u8)>>> = Lazy::new(|| {
    let json_str = include_str!("piece-short-bounding-box.json");
    serde_json::from_str(json_str).unwrap()
});

// Note: center is 7, 7
// This was achieved by placing a 1x1 piece at 7, 7 and then getting the moves from the corners
pub static CORNER_MOVES_DATA: [[u32; 127]; 4] = [
    // Direction::NW (6, 6)
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
    // Direction::NE (8, 6)
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
    // Direction::SE (8, 8)
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
    // Direction::SW (6, 8)
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

pub static CORNER_MOVES_DATA_U64: Lazy<[[u64; 127]; 4]> = Lazy::new(|| {
    let mut corner_moves_data_u64: [[u64; 127]; 4] = [[0; 127]; 4];
    for direction in 0..4 {
        let mut data: [u64; 127] = [0; 127];
        for i in 0..127 {
            let move_unpacked = Move::unpack(CORNER_MOVES_DATA[direction][i]);
            // now we need move it over to the correct location
            let original_center: (i32, i32) = match direction {
                0 => (6, 6),
                1 => (8, 6),
                2 => (8, 8),
                3 => (6, 8),
                _ => unreachable!(),
            };

            let new_center: (i32, i32) = match direction {
                0 => (4, 4),
                1 => (3, 4),
                2 => (3, 3),
                3 => (4, 3),
                _ => unreachable!(),
            };

            let new_coord: Coord = Coord {
                x: move_unpacked.x + new_center.0 - original_center.0,
                y: move_unpacked.y + new_center.1 - original_center.1,
            };

            let mut bitboard: u64 = 0;

            let piece_data = &ORIENTATION_DATA[move_unpacked.movetype as usize]
                [move_unpacked.orientation as usize];
            for coord in piece_data {
                let absolute_coord = Coord {
                    x: coord.x + new_coord.x,
                    y: coord.y + new_coord.y,
                };

                bitboard |= 1 << (absolute_coord.y as usize * 8 + absolute_coord.x as usize);
            }

            data[i] = bitboard;
        }
        corner_moves_data_u64[direction] = data;
    }

    corner_moves_data_u64
});
