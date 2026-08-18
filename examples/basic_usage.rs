use lotus::{LOTUS_DENSE_U64, lotus_decode_u64, lotus_encode_u64_framed};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = LOTUS_DENSE_U64;
    let encoded = lotus_encode_u64_framed(42, config.jumpstarter_bits, config.tiers)?;
    let (decoded, consumed) = lotus_decode_u64(
        &encoded.bytes,
        config.jumpstarter_bits,
        config.tiers,
    )?;

    assert_eq!(decoded, 42);
    assert_eq!(consumed, encoded.bit_len);
    println!("42 -> {} meaningful bits", encoded.bit_len);
    Ok(())
}
