use lotus::metrics::leb128_bits;
use lotus::{LOTUS_DENSE_U64, lotus_encoded_bit_len};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let value = 1_337u64;
    let config = LOTUS_DENSE_U64;
    let lotus_bits = lotus_encoded_bit_len(
        value,
        config.jumpstarter_bits,
        config.tiers,
    )?;

    println!(
        "Lotus J1D2: {lotus_bits} meaningful bits | LEB128: {} bits",
        leb128_bits(value)
    );
    Ok(())
}
