use lotus::{BitReader, BitWriter, LOTUS_J3D1, lotus_decode_u64, lotus_encode_u64};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut writer = BitWriter::new();
    for value in [1u64, 5, 9] {
        let encoded = lotus_encode_u64(value, LOTUS_J3D1.0, LOTUS_J3D1.1)?;
        for byte in encoded {
            writer.write_bits(byte as u64, 8)?;
        }
    }
    let bytes = writer.into_bytes();

    let mut reader = BitReader::new(&bytes);
    for _ in 0..3 {
        let start_pos = reader.clone();
        let (value, consumed) = lotus_decode_u64(&bytes[0..], LOTUS_J3D1.0, LOTUS_J3D1.1)?;
        println!("decoded {value} (consumed ~{consumed} bits) from stream");
        drop(start_pos);
        reader.read_bits(consumed)?;
    }
    Ok(())
}
