use lotus::{
    BitReader, BitWriter, LOTUS_DENSE_U64, lotus_decode_from_reader,
    lotus_encode_into_writer,
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let values = [3u64, 42, 127, 128, 65_535, u32::MAX as u64];
    let config = LOTUS_DENSE_U64;

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
    println!(
        "packed {} values into {} meaningful bits ({} backing bytes)",
        values.len(),
        meaningful_bits,
        bytes.len()
    );
    Ok(())
}
