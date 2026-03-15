use lotus::{LOTUS_J2D1, lotus_decode_u64, lotus_encode_u64_framed};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let encoded = lotus_encode_u64_framed(42, LOTUS_J2D1.0, LOTUS_J2D1.1)?;
    let (decoded, bits) = lotus_decode_u64(&encoded.bytes, LOTUS_J2D1.0, LOTUS_J2D1.1)?;
    println!("42 -> {:?} ({} bits) -> {}", encoded.bytes, bits, decoded);
    Ok(())
}
