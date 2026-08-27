use lotus::{RECOMMENDED_PROFILES, lotus_encoded_bit_len};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let values = [42u64, 65_535, 1_000_000, u32::MAX as u64, u64::MAX];

    for profile in RECOMMENDED_PROFILES {
        let max = profile.config.max_u64_value()?;
        println!(
            "{} ({}) — max u64 value: {}",
            profile.label, profile.purpose, max
        );
        for value in values {
            match lotus_encoded_bit_len(
                value,
                profile.config.jumpstarter_bits,
                profile.config.tiers,
            ) {
                Ok(bits) => println!("  {value}: {bits} bits"),
                Err(_) => println!("  {value}: out of range"),
            }
        }
    }
    Ok(())
}
