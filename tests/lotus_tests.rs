#[cfg(feature = "bigint")]
use lotus::lotus_encode_biguint;
use lotus::metrics::{
    elias_delta_decode, elias_delta_encode, elias_delta_len, elias_gamma_bits, elias_gamma_decode,
    elias_gamma_encode, leb128_bits, leb128_decode, leb128_encode, vlq_bits, vlq_decode,
    vlq_encode,
};
use lotus::{
    LOTUS_J1D2, LOTUS_J2D1, LOTUS_J3D1, LotusError, lotus_decode_u64, lotus_encode_u64,
    lotus_encode_u64_framed,
};
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};

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
fn framed_round_trip_reports_same_bit_length_as_decoder() {
    let framed = lotus_encode_u64_framed(1_000_000, LOTUS_J2D1.0, LOTUS_J2D1.1).unwrap();
    let (decoded, consumed) = lotus_decode_u64(&framed.bytes, LOTUS_J2D1.0, LOTUS_J2D1.1).unwrap();
    assert_eq!(decoded, 1_000_000);
    assert_eq!(consumed, framed.bit_len);
}

#[test]
fn decode_fails_when_input_truncated() {
    let encoded = lotus_encode_u64(u64::MAX, LOTUS_J3D1.0, LOTUS_J3D1.1).unwrap();
    let truncated = &encoded[..encoded.len() - 1];
    let err = lotus_decode_u64(truncated, LOTUS_J3D1.0, LOTUS_J3D1.1).unwrap_err();
    assert_eq!(err, LotusError::UnexpectedEof);
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

// ---------------------------------------------------------------------------
// Comparator codec correctness (LEB128, VLQ, Elias gamma, Elias delta)
// ---------------------------------------------------------------------------

/// Boundary + magnitude samples that exercise every transition a varint or
/// universal code makes: byte/7-bit boundaries for the byte varints, and
/// power-of-two + power-of-two-minus-one boundaries for the Elias codes.
///
/// `u64::MAX` is intentionally excluded: Elias γ/δ code the positive integer
/// `x = value + 1`, which does not fit u64 for `value = u64::MAX`. The byte
/// varints handle it, but the shared sample set keeps the codecs comparable.
fn codec_samples() -> Vec<u64> {
    vec![
        0,
        1,
        2,
        3,
        126,
        127,
        128,
        129,
        255,
        256,
        16383,
        16384,
        16385,
        65535,
        65536,
        2_097_151,
        2_097_152,
        1_000_000,
        4_294_967_295, // u32::MAX
        (1u64 << 40) - 1,
        u64::MAX - 2,
        u64::MAX - 1,
    ]
}

#[test]
fn leb128_round_trip_and_bits() {
    for v in codec_samples() {
        let enc = leb128_encode(v);
        let (dec, used) = leb128_decode(&enc).expect("decode");
        assert_eq!(dec, v, "leb128 value {v}");
        assert_eq!(used, enc.len(), "leb128 consumed {v}");
        assert_eq!(enc.len() * 8, leb128_bits(v), "leb128_bits {v}");
    }
}

#[test]
fn vlq_round_trip_and_bits() {
    for v in codec_samples() {
        let enc = vlq_encode(v);
        let (dec, used) = vlq_decode(&enc).expect("decode");
        assert_eq!(dec, v, "vlq value {v}");
        assert_eq!(used, enc.len(), "vlq consumed {v}");
        assert_eq!(enc.len() * 8, vlq_bits(v), "vlq_bits {v}");
    }
}

#[test]
fn elias_gamma_round_trip_and_bits() {
    for v in codec_samples() {
        let enc = elias_gamma_encode(v);
        let (dec, used) = elias_gamma_decode(&enc).expect("decode");
        assert_eq!(dec, v, "elias gamma value {v}");
        assert_eq!(
            used,
            elias_gamma_bits(v),
            "elias gamma consumed == bits fn {v}"
        );
    }
}

#[test]
fn elias_delta_round_trip_and_bits() {
    for v in codec_samples() {
        let enc = elias_delta_encode(v);
        let (dec, used) = elias_delta_decode(&enc).expect("decode");
        assert_eq!(dec, v, "elias delta value {v}");
        assert_eq!(
            used,
            elias_delta_len(v),
            "elias delta consumed == bits fn {v}"
        );
    }
}

#[test]
fn byte_varints_reject_truncated_input() {
    // A stream with only continuation bytes is incomplete.
    let incomplete = [0x80u8, 0x80, 0x80];
    assert_eq!(
        leb128_decode(&incomplete).unwrap_err(),
        LotusError::UnexpectedEof
    );
    assert_eq!(
        vlq_decode(&incomplete).unwrap_err(),
        LotusError::UnexpectedEof
    );
}

#[test]
fn elias_codes_round_trip_random() {
    let mut rng = StdRng::seed_from_u64(0xc0de_c0de);
    // Elias γ/δ code x = value + 1, so the max representable value is u64::MAX - 1.
    for _ in 0..2000 {
        let v = rng.r#gen::<u64>() % u64::MAX;
        let g_enc = elias_gamma_encode(v);
        assert_eq!(elias_gamma_decode(&g_enc).unwrap().0, v);
        let d_enc = elias_delta_encode(v);
        assert_eq!(elias_delta_decode(&d_enc).unwrap().0, v);
    }
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
