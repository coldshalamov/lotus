use lotus::{BitReader, BitWriter, LOTUS_J3D1, lotus_decode_u64, lotus_encode_u64_framed};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut writer = BitWriter::new();
    for value in [1u64, 5, 9] {
        let encoded = lotus_encode_u64_framed(value, LOTUS_J3D1.0, LOTUS_J3D1.1)?;
        for byte in encoded.bytes {
            writer.write_bits(byte as u64, 8)?;
        }
    }
    let bytes = writer.into_bytes();

    // Demonstrate value-wise decoding from known frame boundaries.
    let mut cursor_bits = 0usize;
    for _ in 0..3 {
        let byte_offset = cursor_bits / 8;
        let (value, consumed) =
            lotus_decode_u64(&bytes[byte_offset..], LOTUS_J3D1.0, LOTUS_J3D1.1)?;
        println!("decoded {value} (consumed {consumed} bits)");
        cursor_bits += consumed;
    }

    // BitReader can be used directly when integrating with custom framing.
    let mut reader = BitReader::new(&bytes);
    let _first_16_bits = reader.read_bits(16)?;
    Ok(())
}
