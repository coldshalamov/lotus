# API guide

## Recommended profile

`LOTUS_DENSE_U64` (`J1D2`) is the default minimum-bit profile for arbitrary `u64` values.

```rust
use lotus::{LOTUS_DENSE_U64, lotus_decode_u64, lotus_encode_u64_framed};

let config = LOTUS_DENSE_U64;
let encoded = lotus_encode_u64_framed(
    42,
    config.jumpstarter_bits,
    config.tiers,
)?;

let (value, consumed_bits) = lotus_decode_u64(
    &encoded.bytes,
    config.jumpstarter_bits,
    config.tiers,
)?;

assert_eq!(value, 42);
assert_eq!(consumed_bits, encoded.bit_len);
# Ok::<(), lotus::LotusError>(())
```

## Packed streams

Lotus is a bitstream codec. Do not concatenate the padded `Vec<u8>` returned for independent values.

```rust
use lotus::{
    BitReader, BitWriter, LOTUS_DENSE_U64,
    lotus_decode_from_reader, lotus_encode_into_writer,
};

let config = LOTUS_DENSE_U64;
let values = [3u64, 42, 127, 128];

let mut writer = BitWriter::new();
for value in values {
    lotus_encode_into_writer(
        value,
        config.jumpstarter_bits,
        config.tiers,
        &mut writer,
    )?;
}
let meaningful_bits = writer.bits_written();
let bytes = writer.into_bytes();

let mut reader = BitReader::new(&bytes);
for expected in values {
    let (decoded, _) = lotus_decode_from_reader(
        &mut reader,
        config.jumpstarter_bits,
        config.tiers,
    )?;
    assert_eq!(decoded, expected);
}
assert_eq!(reader.bits_consumed(), meaningful_bits);
# Ok::<(), lotus::LotusError>(())
```

## Profile selection

- `LOTUS_TINY` / J1D1: values through 125.
- `LOTUS_COMPACT_31` / J2D1: values through `2^31 - 3`.
- `LOTUS_DENSE_U64` / J1D2: minimum-bit full-`u64` profile.
- `LOTUS_FAST_U64` / J3D1: one-tier full-`u64` profile.

`RECOMMENDED_PROFILES` contains this frontier and is shared by metrics, benchmarks, examples, and the generated demo.

## Exact sizing

`lotus_encoded_bit_len` computes meaningful bits without allocating. `EncodedLotus.bit_len` is authoritative for a real standalone encode.

The length of the backing byte vector is not a compression statistic unless the protocol intentionally byte-aligns every value.
