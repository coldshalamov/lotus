use lotus::{LOTUS_J2D1, lotus_decode_u64, lotus_encode_u64};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let encoded = lotus_encode_u64(42, LOTUS_J2D1.0, LOTUS_J2D1.1)?;
    let (decoded, _bits) = lotus_decode_u64(&encoded, LOTUS_J2D1.0, LOTUS_J2D1.1)?;
    println!("42 -> {:?} -> {}", encoded, decoded);
    Ok(())
}
