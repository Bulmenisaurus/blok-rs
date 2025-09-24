use crate::board::Coord;

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
