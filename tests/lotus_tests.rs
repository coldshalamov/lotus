use lotus::{LOTUS_J1D2, LOTUS_J2D1, LOTUS_J3D1, LotusError, lotus_decode_u64, lotus_encode_u64};
#[cfg(feature = "bigint")]
use lotus::lotus_encode_biguint;
use rand::{Rng, SeedableRng};
use rand::rngs::StdRng;

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

fn max_width_for_config(j_bits: usize, tiers: usize) -> u128 {
    let mut max_width = 1u128 << j_bits;
    for _ in 0..tiers {
        let shift = match max_width.checked_add(1).and_then(|v| u32::try_from(v).ok()) {
            Some(value) if value < 128 => value,
            _ => return u128::MAX,
        };
        let base = match 1u128.checked_shl(shift) {
            Some(value) => value,
            None => return u128::MAX,
        };
        max_width = base.saturating_sub(4);
    }
    max_width
}

fn round_trip(value: u64, cfg: (usize, usize)) {
    let encoded = lotus_encode_u64(value, cfg.0, cfg.1).expect("encode");
    let (decoded, _) = lotus_decode_u64(&encoded, cfg.0, cfg.1).expect("decode");
    assert_eq!(decoded, value);
}

#[test]
fn presets_roundtrip() {
    let scenarios = [
        (LOTUS_J1D2, vec![0, 1, 15]),
        (LOTUS_J2D1, vec![0, 1, 255, 1_024]),
        (LOTUS_J3D1, vec![0, 1, 255, 8_192]),
    ];

    for (cfg, values) in scenarios {
        for v in values {
            round_trip(v, cfg);
        }
    }
}

#[test]
fn maximal_edges() {
    round_trip(u32::MAX as u64, LOTUS_J3D1);
    round_trip((1u64 << 40) - 1, LOTUS_J3D1);
}

#[test]
fn leb128_comparison() {
    let sample = [0u64, 1, 2, 127, 128, 4096, 1_000_000];
    for value in sample {
        let lotus = lotus_encode_u64(value, LOTUS_J2D1.0, LOTUS_J2D1.1).unwrap();
        let leb = leb128_encode(value);
        assert!(
            lotus.len() <= leb.len() + 2,
            "lotus should be competitive enough for demo"
        );
    }
}

#[test]
fn lotus_j1d2_beats_leb128_uniform_u32() {
    let mut rng = StdRng::seed_from_u64(0x5a5a_1234_9876_4321);
    let mut lotus_total = 0u64;
    let mut leb_total = 0u64;
    let samples = 5_000;
    let max_width = max_width_for_config(LOTUS_J1D2.0, LOTUS_J1D2.1);
    let max_value = (1u128 << (max_width + 1)).saturating_sub(4);
    let max_value = u32::try_from(max_value).expect("max value fits in u32");

    for _ in 0..samples {
        let value = rng.gen_range(0..=max_value) as u64;
        let lotus = lotus_encode_u64(value, LOTUS_J1D2.0, LOTUS_J1D2.1).unwrap();
        let (decoded, lotus_bits) = lotus_decode_u64(&lotus, LOTUS_J1D2.0, LOTUS_J1D2.1).unwrap();
        assert_eq!(decoded, value);
        lotus_total += lotus_bits as u64;
        leb_total += (leb128_encode(value).len() * 8) as u64;
    }

    assert!(
        lotus_total < leb_total,
        "expected Lotus J=1,d=2 to beat LEB128 on uniform u32 samples"
    );
}

#[test]
fn lotus_j3d1_beats_leb128_uniform_u64() {
    let mut rng = StdRng::seed_from_u64(0x1234_5678_9abc_def0);
    let mut lotus_total = 0u64;
    let mut leb_total = 0u64;
    let samples = 5_000;

    for _ in 0..samples {
        let value = rng.r#gen::<u64>();
        let lotus = lotus_encode_u64(value, LOTUS_J3D1.0, LOTUS_J3D1.1).unwrap();
        let (decoded, lotus_bits) = lotus_decode_u64(&lotus, LOTUS_J3D1.0, LOTUS_J3D1.1).unwrap();
        assert_eq!(decoded, value);
        lotus_total += lotus_bits as u64;
        leb_total += (leb128_encode(value).len() * 8) as u64;
    }

    assert!(
        lotus_total < leb_total,
        "expected Lotus J=3,d=1 to beat LEB128 on uniform u64 samples"
    );
}

#[test]
fn invalid_inputs() {
    let err = lotus_decode_u64(&[], 2, 1).unwrap_err();
    assert!(matches!(err, LotusError::UnexpectedEof));
}

#[test]
fn u64_max_with_deeper_tiers() {
    let encoded = lotus_encode_u64(u64::MAX, 3, 2).expect("encode max");
    let (decoded, _) = lotus_decode_u64(&encoded, 3, 2).expect("decode max");
    assert_eq!(decoded, u64::MAX);
}

#[test]
fn value_too_large_for_small_config() {
    let err = lotus_encode_u64(60, 1, 1).unwrap_err();
    assert_eq!(err, LotusError::ValueTooLarge);
}

#[cfg(feature = "bigint")]
mod bigint_tests {
    use super::*;
    use num_bigint::BigUint;

    #[test]
    fn encode_100_digit_number() {
        let huge_val = BigUint::parse_bytes(
            b"1355737381323775828630676731039195664907583275030601675940045606875040670309706208564942376964601277566867233121",
            10,
        )
        .unwrap();

        let encoded = lotus_encode_biguint(&huge_val, 3, 2).expect("encode 100-digit");
        println!("100-digit number encoded to {} bytes", encoded.len());

        assert!(
            encoded.len() <= 48,
            "Lotus should be competitive with LEB128 byte count"
        );
    }
}
