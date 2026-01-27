use lotus::{LOTUS_J1D2, LOTUS_J2D1, LOTUS_J3D1, LotusError, lotus_decode_u64, lotus_encode_u64};
#[cfg(feature = "bigint")]
use lotus::lotus_encode_biguint;

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
