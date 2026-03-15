# Lotus Rust API

## Core encode/decode

- `lotus_encode_u64(value, j_bits, tiers) -> Result<Vec<u8>, LotusError>`
  - Backward-compatible convenience API.
  - Returns bytes only.

- `lotus_encode_u64_framed(value, j_bits, tiers) -> Result<EncodedLotus, LotusError>`
  - Preferred API for stream/framing code.
  - Returns:
    - `bytes`: packed MSB-first bitstream
    - `bit_len`: exact number of meaningful bits

- `lotus_decode_u64(bytes, j_bits, tiers) -> Result<(u64, usize), LotusError>`
  - Returns decoded value plus consumed bits.

## Bit-level framing semantics

Lotus is bit-oriented, not byte-oriented:

- Final encoded byte may include trailing zero padding bits.
- `EncodedLotus.bit_len` (or decode's consumed bits) is authoritative for framing.
- Extra trailing bytes are ignored by single-value decode unless your protocol forbids them.

For multi-value streams, delimit using the returned/recorded bit lengths.

## Errors

`LotusError` variants:

- `JumpstarterOverflow`
- `UnexpectedEof`
- `InvalidEncoding`
- `ValueTooLarge`

## Features

- `small-int-fastpath`: internal optimization surface (non-default).
- `bigint`: enables `lotus_encode_biguint`.
- `cli`: enables the `lotus` binary and CLI-only dependencies (`clap`, `hex`, `serde`, `serde_json`).

## Presets

- `LOTUS_J2D1`
- `LOTUS_J1D2`
- `LOTUS_J3D1`
