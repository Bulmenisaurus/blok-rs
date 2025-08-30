use std::{
    fs::File,
    io::{BufReader, BufWriter, Read, Write},
    mem,
};

// Reading in a file of positions, write out a file with them all mirrored
#[repr(C)]
struct Packed {
    data: [u32; 15],
}
const PACKED_SIZE: usize = mem::size_of::<Packed>();

fn transform(packed: [u8; 60]) -> [u8; 60] {
    let packed: Packed = unsafe { mem::transmute::<[u8; PACKED_SIZE], Packed>(packed) };

    let mut output: [u32; 15] = [0; 15];

    // to map across the diagonal, swap x and y

    for y in 0..14 {
        for x in 0..14 {
            let a_bit = packed.data[x] >> (y) & 1;
            let b_bit = packed.data[x] >> (y + 16) & 1;

            output[y] |= (a_bit << x) | (b_bit << (16 + x));
        }
    }
    // keep the metadata the same (eval should remain equal too)
    output[14] = packed.data[14];

    let out_bytes: [u8; PACKED_SIZE] =
        unsafe { mem::transmute::<Packed, [u8; PACKED_SIZE]>(Packed { data: output }) };

    return out_bytes;
}

fn main() {
    let file = File::open("data.bin").unwrap();
    let mut reader = BufReader::new(file);
    let mut writer = BufWriter::new(File::create("data_mirrored.bin").unwrap());

    // Each Packed is 15 u32s = 60 bytes
    let mut buffer = [0u8; PACKED_SIZE];

    while let Ok(()) = reader.read_exact(&mut buffer) {
        writer.write_all(&transform(buffer)).unwrap();
    }

    writer.flush().unwrap();
}
