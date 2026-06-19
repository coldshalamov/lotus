//! Emits a JSON reference fixture for the interactive demo in `docs/index.html`.
//!
//! Run with:
//!
//! ```text
//! cargo run --example generate_demo_fixture
//! ```
//!
//! Paste the printed array into the `LOTUS_REFERENCE` constant in
//! `docs/index.html`. The page recomputes Lotus bit lengths for the same
//! sample values in JavaScript and asserts equality against this fixture on
//! load — that ties the demo's math to the Rust reference in `src/lib.rs`.
use lotus::{lotus_encode_u64_framed, lotus_encoded_bit_len};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Sample grid: boundary values where Lotus tiers/widths transition, plus a
    // spread of magnitudes. These must stay in sync with the JS sample list in
    // docs/index.html (the page asserts the match, so drift is caught at load).
    let values: &[u64] = &[
        0,
        1,
        2,
        3,
        5,
        13,
        14,
        29,
        30,
        61,
        62,
        125,
        126,
        253,
        254,
        509,
        510,
        1021,
        1022,
        2045,
        2046,
        4093,
        4094,
        8189,
        8190,
        16381,
        16382,
        65533,
        65534,
        1000000,
        1048573,
        1048574,
        16777213,
        16777214,
        4294967293,
        4294967294,
        (1u64 << 48),
    ];

    let configs: &[(usize, usize)] = &[(1, 2), (2, 1), (3, 1), (3, 2)];

    print!("[");
    let mut first = true;
    for &v in values {
        for &(j, d) in configs {
            // Some (value, config) pairs are out of range; record null there.
            let bits = lotus_encoded_bit_len(v, j, d).ok();
            // Cross-check the arithmetic-only helper against a real encode.
            if let Some(b) = bits {
                let framed = lotus_encode_u64_framed(v, j, d)?;
                assert_eq!(
                    framed.bit_len, b,
                    "bit_len mismatch for value={v} j={j} d={d}"
                );
            }
            if !first {
                print!(",");
            }
            first = false;
            match bits {
                Some(b) => print!("{{\"v\":{v},\"j\":{j},\"d\":{d},\"bits\":{b}}}"),
                None => print!("{{\"v\":{v},\"j\":{j},\"d\":{d},\"bits\":null}}"),
            }
        }
    }
    println!("]");

    Ok(())
}
