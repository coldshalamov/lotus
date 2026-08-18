#[cfg(feature = "bigint")]
use lotus::lotus_encode_biguint;
use lotus::metrics::{
    UniformDomain, elias_delta_decode, elias_delta_encode, elias_delta_len, elias_gamma_bits,
    elias_gamma_decode, elias_gamma_encode, leb128_bits, leb128_decode, leb128_encode,
    summarize_uniform_domain, vlq_bits, vlq_decode, vlq_encode,
};
use lotus::{
    BitReader, BitWriter, LOTUS_COMPACT_31, LOTUS_DENSE_U64, LOTUS_FAST_U64, LOTUS_TINY,
    LotusError, lotus_decode_from_reader, lotus_decode_u64, lotus_encode_into_writer,
    lotus_encode_u64, lotus_encode_u64_framed, lotus_encoded_bit_len,
};
use proptest::prelude::*;

fn round_trip(value: u64, j: usize, d: usize) {
    let framed = lotus_encode_u64_framed(value, j, d).expect("encode");
    let (decoded, consumed) = lotus_decode_u64(&framed.bytes, j, d).expect("decode");
    assert_eq!(decoded, value);
    assert_eq!(consumed, framed.bit_len);
    assert_eq!(
        lotus_encoded_bit_len(value, j, d).unwrap(),
        framed.bit_len
    );
}

#[test]
fn recommended_profile_boundaries_round_trip() {
    let tiny_max = LOTUS_TINY.max_u64_value().unwrap();
    for value in [0, 1, 2, 124, 125, tiny_max] {
        round_trip(value, LOTUS_TINY.jumpstarter_bits, LOTUS_TINY.tiers);
    }
    assert_eq!(
        lotus_encode_u64(
            tiny_max + 1,
            LOTUS_TINY.jumpstarter_bits,
            LOTUS_TINY.tiers,
        ),
        Err(LotusError::JumpstarterOverflow)
    );

    let compact_max = LOTUS_COMPACT_31.max_u64_value().unwrap();
    for value in [0, 1, 255, 1_000_000, compact_max - 1, compact_max] {
        round_trip(
            value,
            LOTUS_COMPACT_31.jumpstarter_bits,
            LOTUS_COMPACT_31.tiers,
        );
    }
    assert_eq!(
        lotus_encode_u64(
            compact_max + 1,
            LOTUS_COMPACT_31.jumpstarter_bits,
            LOTUS_COMPACT_31.tiers,
        ),
        Err(LotusError::JumpstarterOverflow)
    );

    for config in [LOTUS_DENSE_U64, LOTUS_FAST_U64] {
        for value in [
            0,
            1,
            2,
            127,
            128,
            u32::MAX as u64,
            (1u64 << 63) - 1,
            u64::MAX,
        ] {
            round_trip(value, config.jumpstarter_bits, config.tiers);
        }
    }
}

proptest! {
    #[test]
    fn dense_u64_round_trips_everywhere(value in any::<u64>()) {
        let encoded = lotus_encode_u64(
            value,
            LOTUS_DENSE_U64.jumpstarter_bits,
            LOTUS_DENSE_U64.tiers,
        ).unwrap();
        let decoded = lotus_decode_u64(
            &encoded,
            LOTUS_DENSE_U64.jumpstarter_bits,
            LOTUS_DENSE_U64.tiers,
        ).unwrap().0;
        prop_assert_eq!(decoded, value);
    }

    #[test]
    fn fast_u64_round_trips_everywhere(value in any::<u64>()) {
        let encoded = lotus_encode_u64(
            value,
            LOTUS_FAST_U64.jumpstarter_bits,
            LOTUS_FAST_U64.tiers,
        ).unwrap();
        let decoded = lotus_decode_u64(
            &encoded,
            LOTUS_FAST_U64.jumpstarter_bits,
            LOTUS_FAST_U64.tiers,
        ).unwrap().0;
        prop_assert_eq!(decoded, value);
    }
}

#[test]
fn canonical_bit_lengths_are_stable() {
    let dense = LOTUS_DENSE_U64;
    let fast = LOTUS_FAST_U64;

    assert_eq!(
        lotus_encoded_bit_len(0, dense.jumpstarter_bits, dense.tiers),
        Ok(4)
    );
    assert_eq!(
        lotus_encoded_bit_len(42, dense.jumpstarter_bits, dense.tiers),
        Ok(9)
    );
    assert_eq!(
        lotus_encoded_bit_len(42, fast.jumpstarter_bits, fast.tiers),
        Ok(10)
    );
    assert_eq!(
        lotus_encoded_bit_len(u64::MAX, dense.jumpstarter_bits, dense.tiers),
        Ok(73)
    );
    assert_eq!(
        lotus_encoded_bit_len(u64::MAX, fast.jumpstarter_bits, fast.tiers),
        Ok(73)
    );
}

#[test]
fn exact_uniform_u32_regression_proves_original_claim() {
    let summary = summarize_uniform_domain(UniformDomain {
        name: "uniform_u32_test",
        start: 0,
        end: u32::MAX as u64,
    });

    let dense = summary
        .lotus
        .iter()
        .find(|result| result.label == "J1D2")
        .unwrap();
    assert_eq!(dense.total_bits, Some(161_061_240_644));
    assert_eq!(summary.leb128_total_bits, 169_634_298_880);
    let comparison = dense.versus_leb128.unwrap();
    assert_eq!(comparison.wins, 4_058_104_710);
    assert_eq!(comparison.ties, 33_686_546);
    assert_eq!(comparison.losses, 203_176_040);
    assert_eq!(
        comparison.wins + comparison.ties + comparison.losses,
        1u128 << 32
    );

    let fast = summary
        .lotus
        .iter()
        .find(|result| result.label == "J3D1")
        .unwrap();
    assert_eq!(fast.total_bits, Some(161_061_240_770));
    let comparison = fast.versus_leb128.unwrap();
    assert_eq!(comparison.wins, 4_058_104_702);
    assert_eq!(comparison.ties, 33_686_538);
    assert_eq!(comparison.losses, 203_176_056);
}

#[test]
fn exact_uniform_u64_regression_is_not_sampled() {
    let summary = summarize_uniform_domain(UniformDomain {
        name: "uniform_u64_test",
        start: 0,
        end: u64::MAX,
    });
    let dense = summary
        .lotus
        .iter()
        .find(|result| result.label == "J1D2")
        .unwrap();
    assert_eq!(dense.total_bits, Some(1_300_495_457_194_375_872_390));
    assert_eq!(
        summary.leb128_total_bits,
        1_401_371_549_788_580_740_096
    );
    let counts = dense.versus_leb128.unwrap();
    assert_eq!(counts.wins, 18_446_459_269_896_323_966);
    assert_eq!(counts.ties, 282_578_816_992_276);
    assert_eq!(counts.losses, 2_224_996_235_374);
    assert_eq!(counts.wins + counts.ties + counts.losses, 1u128 << 64);
}

#[test]
fn packed_stream_round_trip_has_no_per_value_padding() {
    let values = [0u64, 42, 127, 128, 65_535, u32::MAX as u64, u64::MAX];
    let config = LOTUS_DENSE_U64;
    let mut writer = BitWriter::new();
    let mut expected_bits = 0usize;

    for &value in &values {
        expected_bits += lotus_encode_into_writer(
            value,
            config.jumpstarter_bits,
            config.tiers,
            &mut writer,
        )
        .unwrap();
    }
    assert_eq!(writer.bits_written(), expected_bits);

    let bytes = writer.into_bytes();
    let mut reader = BitReader::new(&bytes);
    for &expected in &values {
        let (decoded, _) = lotus_decode_from_reader(
            &mut reader,
            config.jumpstarter_bits,
            config.tiers,
        )
        .unwrap();
        assert_eq!(decoded, expected);
    }
    assert_eq!(reader.bits_consumed(), expected_bits);
}

#[test]
fn standalone_padding_does_not_change_consumed_bits() {
    let config = LOTUS_DENSE_U64;
    let framed = lotus_encode_u64_framed(
        1_000_000,
        config.jumpstarter_bits,
        config.tiers,
    )
    .unwrap();
    let mut extended = framed.bytes.clone();
    extended.extend_from_slice(&[0xff, 0xff]);
    let (decoded, consumed) = lotus_decode_u64(
        &extended,
        config.jumpstarter_bits,
        config.tiers,
    )
    .unwrap();
    assert_eq!(decoded, 1_000_000);
    assert_eq!(consumed, framed.bit_len);
}

#[test]
fn malformed_and_truncated_inputs_fail_without_looping() {
    assert_eq!(
        lotus_decode_u64(&[], 1, 2),
        Err(LotusError::UnexpectedEof)
    );
    assert_eq!(
        lotus_decode_u64(&[0xff; 64], 8, 2),
        Err(LotusError::ValueTooLarge)
    );

    let config = LOTUS_FAST_U64;
    let encoded = lotus_encode_u64(
        u64::MAX,
        config.jumpstarter_bits,
        config.tiers,
    )
    .unwrap();
    for end in 0..encoded.len() {
        assert_eq!(
            lotus_decode_u64(
                &encoded[..end],
                config.jumpstarter_bits,
                config.tiers,
            ),
            Err(LotusError::UnexpectedEof)
        );
    }
}

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
        16_383,
        16_384,
        16_385,
        65_535,
        65_536,
        2_097_151,
        2_097_152,
        1_000_000,
        u32::MAX as u64,
        (1u64 << 40) - 1,
        u64::MAX - 1,
        u64::MAX,
    ]
}

#[test]
fn byte_varint_round_trips_and_lengths_match() {
    for value in codec_samples() {
        let encoded = leb128_encode(value);
        let (decoded, bytes) = leb128_decode(&encoded).unwrap();
        assert_eq!(decoded, value);
        assert_eq!(bytes, encoded.len());
        assert_eq!(encoded.len() * 8, leb128_bits(value));

        let encoded = vlq_encode(value);
        let (decoded, bytes) = vlq_decode(&encoded).unwrap();
        assert_eq!(decoded, value);
        assert_eq!(bytes, encoded.len());
        assert_eq!(encoded.len() * 8, vlq_bits(value));
    }
}

#[test]
fn elias_codecs_cover_the_full_u64_domain() {
    for value in codec_samples() {
        let encoded = elias_gamma_encode(value);
        let (decoded, bits) = elias_gamma_decode(&encoded).unwrap();
        assert_eq!(decoded, value);
        assert_eq!(bits, elias_gamma_bits(value));

        let encoded = elias_delta_encode(value);
        let (decoded, bits) = elias_delta_decode(&encoded).unwrap();
        assert_eq!(decoded, value);
        assert_eq!(bits, elias_delta_len(value));
    }
}

#[test]
fn byte_varints_reject_truncated_input() {
    let incomplete = [0x80u8, 0x80, 0x80];
    assert_eq!(leb128_decode(&incomplete), Err(LotusError::UnexpectedEof));
    assert_eq!(vlq_decode(&incomplete), Err(LotusError::UnexpectedEof));
}

#[test]
fn bit_writer_handles_full_width_fields_after_partial_fields() {
    let mut writer = BitWriter::new();
    writer.write_bits(0b101, 3).unwrap();
    writer.write_bits(u64::MAX, 64).unwrap();
    let bytes = writer.into_bytes();

    let mut reader = BitReader::new(&bytes);
    assert_eq!(reader.read_bits(3).unwrap(), 0b101);
    assert_eq!(reader.read_bits(64).unwrap(), u64::MAX);
}

#[cfg(feature = "bigint")]
#[test]
fn bigint_uses_the_same_canonical_descriptor_mapping() {
    use num_bigint::BigUint;

    let value = BigUint::parse_bytes(
        b"1355737381323775828630676731039195664907583275030601675940045606875040670309706208564942376964601277566867233121",
        10,
    )
    .unwrap();

    let encoded = lotus_encode_biguint(&value, 3, 2).expect("encode bigint");
    assert!(!encoded.is_empty());
}
