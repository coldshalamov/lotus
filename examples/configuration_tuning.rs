use lotus::{LOTUS_J1D2, LOTUS_J2D1, LOTUS_J3D1, lotus_encoded_bits};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let value = 1_000_000u64;
    for (name, cfg) in [
        ("J1D2", LOTUS_J1D2),
        ("J2D1", LOTUS_J2D1),
        ("J3D1", LOTUS_J3D1),
    ] {
        let bits = lotus_encoded_bits(value, cfg.0, cfg.1)?;
        println!("{name}: {bits} bits");
    }
    Ok(())
}
