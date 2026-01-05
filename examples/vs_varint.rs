fn leb128_encode(mut value: u64) -> Vec<u8> {
    let mut out = Vec::new();
    loop {
        let mut byte = (value & 0x7f) as u8;
        value >>= 7;
        if value != 0 {
            byte |= 0x80;
            out.push(byte);
        } else {
            out.push(byte);
            break;
        }
    }
    out
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let value = 1337u64;
    let lotus = lotus::lotus_encode_u64(value, lotus::LOTUS_J2D1.0, lotus::LOTUS_J2D1.1)?;
    let leb = leb128_encode(value);
    println!(
        "lotus: {} bits | leb128: {} bits",
        lotus.len() * 8,
        leb.len() * 8
    );
    Ok(())
}
