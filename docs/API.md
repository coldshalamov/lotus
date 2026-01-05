# Lotus Rust API

The public API intentionally mirrors the mathematical construction from the whitepaper while keeping Rust ergonomics front and center.

## Library layout

* `lotus_encode_u64(value: u64, j_bits: usize, tiers: usize) -> Result<Vec<u8>, LotusError>`
  * Encodes a single integer with the provided jumpstarter width and tier count.
  * Returns the minimal byte buffer containing the encoded payload.
* `lotus_decode_u64(bytes: &[u8], j_bits: usize, tiers: usize) -> Result<(u64, usize), LotusError>`
  * Decodes an integer and returns both the value and the number of bits consumed from `bytes`.
* `BitWriter` / `BitReader`
  * Streaming helpers for advanced scenarios such as incremental network framing.
* Presets
  * `LOTUS_J2D1`, `LOTUS_J1D2`, `LOTUS_J3D1` provide tuned defaults evaluated in the whitepaper.
* Feature flags
  * `small-int-fastpath` keeps single-byte encodings for very small integers before falling back to Lotus.

### Error handling

The `LotusError` enum models all error cases without panicking:

* `JumpstarterOverflow`: the requested payload width cannot be represented with the chosen jumpstarter.
* `UnexpectedEof`: the input ran out of bits mid-decode.
* `InvalidEncoding`: the bit pattern cannot be mapped to a valid Lotus value.

### Usage pattern

Most callers will wire the presets into higher-level protocols:

```rust
use lotus::{lotus_encode_u64, lotus_decode_u64, LOTUS_J2D1};

let encoded = lotus_encode_u64(42, LOTUS_J2D1.0, LOTUS_J2D1.1)?;
let (decoded, _bits) = lotus_decode_u64(&encoded, LOTUS_J2D1.0, LOTUS_J2D1.1)?;
assert_eq!(decoded, 42);
```

For streaming applications, reuse a `BitWriter` and feed chunks directly into sockets or files.
