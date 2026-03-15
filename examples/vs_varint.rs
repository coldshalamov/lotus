use lotus::{LOTUS_J2D1, lotus_decode_u64, lotus_encode_u64};

fn leb128_encode(mut value: u64) -> Vec<u8> {
    let mut out = Vec::new();
    loop {
        let mut byte = (value & 0x7f) as u8;
        value >>= 7;
        if value != 0 {
            byte |= 0x80;
        }
        out.push(byte);
        if value == 0 {
            break;
        }
    }
    out
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let value = 1337u64;
    let lotus = lotus_encode_u64(value, LOTUS_J2D1.0, LOTUS_J2D1.1)?;
    let (_, lotus_bits) = lotus_decode_u64(&lotus, LOTUS_J2D1.0, LOTUS_J2D1.1)?;
    let leb = leb128_encode(value);
    println!("lotus: {lotus_bits} bits | leb128: {} bits", leb.len() * 8);
    Ok(())
}
